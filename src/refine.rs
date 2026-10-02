//! Handing a task to Claude in its workspace's herdr session, to be worked up
//! into a plan by the `refine-task` skill.
//!
//! Only the task's uuid crosses over: the skill reads the task itself, so
//! nothing needs quoting through two CLIs and it always sees the current
//! version rather than the one the menu was opened on.

use crate::{herdr, niri, notify, project, session, task, text};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Which of the menu's two entries opened Claude. The two differ only in the
/// prompt [`prompt`] sends — the skill on the other end reads it to decide
/// whether to interview before drafting or draft straight away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Draft straight away; ask only what the code cannot answer.
    Quick,
    /// A full interview, via the `grilling` skill, before the draft.
    Grill,
}

/// How long a just-opened project terminal gets to bring its herdr session up.
const SESSION_WAIT: Duration = Duration::from_secs(10);
const SESSION_POLL: Duration = Duration::from_millis(250);

/// Longest description, in characters, a tab label carries before eliding.
const LABEL_DESCRIPTION_MAX: usize = 30;

/// The herdr agent name for a task's refine session. One name per task is
/// what lets a second Refine find the first instead of opening another.
pub fn agent_name(uuid: &str) -> String {
    let short: String = uuid.chars().take(8).collect();
    format!("task-{}", short.to_ascii_lowercase())
}

/// Only the uuid crosses to the skill — the description and everything else
/// about the task is read by the skill itself, so nothing here needs quoting
/// through two CLIs (this one, then herdr's `agent prompt`).
pub fn prompt(uuid: &str, mode: Mode) -> String {
    match mode {
        Mode::Quick => format!("/refine-task {uuid}"),
        Mode::Grill => format!("/refine-task {uuid} grill"),
    }
}

/// The verb says which mode the tab is in; the description is cut short
/// because the label shares herdr's sidebar with every other tab's label.
pub fn tab_label(mode: Mode, description: &str) -> String {
    let verb = match mode {
        Mode::Quick => "Refine",
        Mode::Grill => "Grill",
    };
    let d = text::collapse_whitespace(description);
    let short = if d.chars().count() > LABEL_DESCRIPTION_MAX {
        let cut: String = d.chars().take(LABEL_DESCRIPTION_MAX - 1).collect();
        format!("{cut}…")
    } else {
        d
    };
    format!("{verb}: {short}")
}

/// Credential files and folders, as `Read` rule patterns relative to home.
///
/// The session searches and fetches from the web unasked, so a page it reads
/// could try to talk it into sending a secret out in a URL. Hiding these from
/// every reader it has — the `Read` tool by deny rule, sandboxed Bash by
/// [`credential_paths`] — leaves nothing of that kind to send.
pub const CREDENTIALS: &[&str] = &[
    "~/.ssh/**",
    "~/.gnupg/**",
    "~/.aws/**",
    "~/.config/gh/**",
    "~/.docker/**",
    "~/.kube/**",
    "~/.netrc",
    "~/.git-credentials",
    "~/.claude/.credentials.json",
    "~/.local/share/keyrings/**",
];

/// The Claude Code settings that fence a refine session in, as the JSON
/// `--settings` takes.
///
/// A Bash sandbox, so the session can work without a prompt per command yet
/// cannot start on the task: sandboxed commands run unasked, the OS keeps them
/// out of `project` (writes only — reading it is the point), and the one place
/// they may write is `task_data`, for the skill's final import. No retrying a
/// blocked command outside the sandbox, and no starting at all without one.
///
/// Unix sockets are allowed because the sandbox's socket filter cannot load
/// under Ubuntu's bwrap AppArmor profile, which denies it the capability it
/// needs. `hidden` makes up for it: the sockets that could run something
/// outside the sandbox — niri's spawn, the session bus, docker, herdr's panes —
/// are hidden from it instead, along with the credentials.
///
/// Allowed unasked on top: web search and fetch, which write nothing; and
/// `task` and `python3`, which Claude Code asks about even inside the sandbox
/// (`task` for reasons it does not log, `python3 -c` because inline
/// interpreter code always asks) — the skill's write is exactly those two, and
/// they still run sandboxed.
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf]) -> String {
    let deny: Vec<String> = CREDENTIALS.iter().map(|c| format!("Read({c})")).collect();
    serde_json::json!({
        "permissions": {
            "allow": ["WebSearch", "WebFetch", "Bash(task *)", "Bash(python3 *)"],
            "deny": deny,
        },
        "sandbox": {
            "enabled": true,
            "autoAllowBashIfSandboxed": true,
            "allowUnsandboxedCommands": false,
            "failIfUnavailable": true,
            "network": { "allowAllUnixSockets": true },
            "filesystem": {
                "denyWrite": [project],
                "allowWrite": [task_data],
                "denyRead": hidden,
            },
        }
    })
    .to_string()
}

/// [`CREDENTIALS`] as paths under `home`, of those that exist: the sandbox
/// refuses to start on a path it cannot find.
fn credential_paths(home: &Path) -> Vec<PathBuf> {
    CREDENTIALS
        .iter()
        .map(|c| home.join(c.trim_start_matches("~/").trim_end_matches("/**")))
        .filter(|p| p.exists())
        .collect()
}

/// Every path [`session_settings`] hides from sandboxed Bash, of those that
/// exist here: the folders the machine's sockets live in, and the credentials.
///
/// Whole folders, not a list of known sockets: a socket is a way to make some
/// other, unsandboxed process act — niri spawns, D-Bus starts units, Docker is
/// root, VS Code runs commands, Xwayland takes keystrokes — and a hand-picked
/// list misses whichever one nobody thought of. `/run` holds the system's and
/// the session's (`/var/run` is the same folder), `/tmp` the X11 and app
/// sockets — the sandbox keeps its own temp folder under it regardless —
/// `/var/snap` the snaps', and herdr keeps its own under its config.
/// [`exposed_sockets`] checks the result against the live socket table.
fn hidden_paths(home: &Path) -> Vec<PathBuf> {
    [
        PathBuf::from("/run"),
        PathBuf::from("/tmp"),
        PathBuf::from("/var/snap"),
        home.join(".config/herdr"),
    ]
    .into_iter()
    .filter(|p| p.exists())
    .chain(credential_paths(home))
    .collect()
}

/// The pathname sockets in a `/proc/net/unix` table. Abstract ones (`@…`)
/// are left out: the sandbox gives commands their own network namespace, and
/// abstract sockets do not cross it.
fn socket_paths(table: &str) -> Vec<PathBuf> {
    table
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().nth(7))
        .filter(|p| p.starts_with('/'))
        .map(PathBuf::from)
        .collect()
}

/// The sockets a refine session could still reach: those under none of
/// `hidden`, or under one of `visible` — folders the sandbox shows inside a
/// hidden one. By path component, so hiding `/run` does not hide `/runner`.
fn exposed_sockets(sockets: &[PathBuf], hidden: &[PathBuf], visible: &[PathBuf]) -> Vec<PathBuf> {
    sockets
        .iter()
        .filter(|s| !hidden.iter().any(|h| s.starts_with(h)) || visible.iter().any(|v| s.starts_with(v)))
        .cloned()
        .collect()
}

/// The sandbox's own temp folder, which it keeps visible inside a hidden
/// `/tmp`: `CLAUDE_CODE_TMPDIR` if set, else `/tmp/claude-<uid>`.
fn sandbox_temp() -> Result<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    if let Some(dir) = std::env::var_os("CLAUDE_CODE_TMPDIR") {
        return Ok(PathBuf::from(dir));
    }
    let uid = std::fs::metadata("/proc/self").context("could not read /proc/self")?.uid();
    Ok(PathBuf::from(format!("/tmp/claude-{uid}")))
}

/// Refuse to start a refine while any socket on the machine would be left in
/// its sandbox's reach — so a new app's socket somewhere unexpected stops the
/// feature loudly instead of quietly becoming a way out.
///
/// Paths are resolved first, since a socket bound under `/var/run` is really
/// under `/run`; one that no longer exists cannot be connected to and is
/// skipped.
fn ensure_no_exposed_sockets(hidden: &[PathBuf]) -> Result<()> {
    let table = std::fs::read_to_string("/proc/net/unix").context("could not read /proc/net/unix")?;
    let sockets: Vec<PathBuf> = socket_paths(&table)
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .collect();
    let exposed = exposed_sockets(&sockets, hidden, &[sandbox_temp()?]);
    if !exposed.is_empty() {
        let list: Vec<String> = exposed.iter().map(|p| p.display().to_string()).collect();
        bail!(
            "Refine would leave these sockets in reach of its sandbox, so it did not start: {}. \
             Hide their folder in refine::hidden_paths.",
            list.join(", ")
        );
    }
    Ok(())
}

/// A niri window's id, app_id and title — the columns [`find_session_window`]
/// needs, kept as a tuple rather than `niri_ipc::Window` so a test can build
/// one without niri_ipc's layout fields.
type WindowInfo<'a> = (u64, Option<&'a str>, Option<&'a str>);

/// The ghostty window already showing session `labels` belongs to, if niri
/// has one open.
///
/// herdr sets the outer terminal's title to its `window_title`, default
/// `"{hostname}: {workspace}"`, where `{workspace}` is the label of whichever
/// herdr workspace that client has focused: the project's, or a task's
/// worktree after Start working. So a session's ghostty window's title ends
/// with `": <label>"` for one of `labels`, the session's workspace labels.
/// The process tree can't say which window a client is in, because ghostty
/// runs every window from one process. This depends on herdr's default title:
/// a user who changes `window_title` just costs themselves an extra attached
/// terminal window rather than a focus, which is harmless.
fn find_session_window(windows: &[WindowInfo], labels: &[String]) -> Option<u64> {
    let suffixes: Vec<String> = labels.iter().map(|l| format!(": {l}")).collect();
    windows
        .iter()
        .find(|(_, app_id, title)| {
            *app_id == Some("com.mitchellh.ghostty")
                && title.is_some_and(|t| suffixes.iter().any(|s| t.ends_with(s)))
        })
        .map(|(id, _, _)| *id)
}

/// Bring the window showing `s`'s session into view: focus it if niri still
/// has one open, or attach another client to the running session if the user
/// closed it — herdr allows more than one client on a session, so a second
/// attach is harmless. Without this, closing the project terminal window
/// leaves the herdr server running and any refine opened afterwards invisible.
///
/// A session that answers `workspace list` but has no herdr workspace yet has
/// no title to look for; `launch` creates one right after this call, and that
/// create takes `--focus` itself.
fn show_session_window(dir: &Path, s: &str, list: &Value) -> Result<()> {
    let labels = herdr::workspace_labels(list);
    if labels.is_empty() {
        return Ok(());
    }
    let windows = niri::windows()?;
    let info: Vec<WindowInfo> = windows
        .iter()
        .map(|w| (w.id, w.app_id.as_deref(), w.title.as_deref()))
        .collect();
    match find_session_window(&info, &labels) {
        Some(id) => niri::focus_window(id)?,
        None => niri::spawn(project::project_terminal_command(dir, s, true))?,
    }
    Ok(())
}

/// Make `s`'s herdr session running and in front of the user, and return its
/// workspace list: start the project terminal if the session is stopped (a
/// window then comes with it), otherwise focus the window showing it or attach
/// another — a running session may have had its window closed while its herdr
/// server kept going. Shared by every launcher that opens something in a
/// project's session.
pub(crate) fn open_session(dir: &Path, s: &str) -> Result<Value> {
    match herdr::run(&herdr::workspace_list(s)) {
        Ok(list) => {
            show_session_window(dir, s, &list)?;
            Ok(list)
        }
        Err(_) => {
            anyhow::ensure!(project::on_path(project::SESSION_MANAGER), "herdr is not installed.");
            // Through niri, so the window lands on the focused workspace —
            // the one the task belongs to — as the project picker's does.
            niri::spawn(project::project_terminal_command(dir, s, true))?;
            wait_for_session(s)
        }
    }
}

/// Open Claude on a task in `workspace`'s herdr session.
///
/// Opens the project terminal first if the session is not running, and goes
/// back to the task's existing tab if it is already being refined.
///
/// The task as found, not a uuid as typed: the session is found again by the
/// uuid's first eight characters, and a typed task number has none of them.
pub fn launch(workspace: &str, t: &task::Task, mode: Mode) -> Result<()> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let home = Path::new(&home);
    let dir = session::start_dir(home, workspace);
    let s = session::herdr_session_name(workspace);
    let name = agent_name(&t.uuid);

    // Before anything opens: a refused refine should leave nothing behind.
    let hidden = hidden_paths(home);
    ensure_no_exposed_sockets(&hidden)?;
    let settings = session_settings(&dir, &crate::task::data_location()?, &hidden);

    let list = open_session(&dir, &s)?;

    if herdr::run(&herdr::agent_get(&s, &name)).is_ok() {
        herdr::run(&herdr::agent_focus(&s, &name))?;
        notify::tasks("Already being refined — switched to its tab.");
        return Ok(());
    }

    let label = tab_label(mode, &t.description);
    let created = match herdr::first_workspace_id(&list) {
        Some(id) => herdr::run(&herdr::tab_create(&s, &id, &dir, &label))?,
        // The workspace itself is named after the project, not this tab.
        None => herdr::run(&herdr::workspace_create(&s, &dir, workspace))?,
    };
    let pane = herdr::root_pane_id(&created).context("herdr did not say which pane it made")?;

    if let Err(e) = herdr::run(&herdr::agent_start_claude_refiner(&s, &name, &pane, &settings)) {
        // Best-effort: a retry should not find a pile of bare-shell tabs from
        // every failed attempt, but a failure here must not hide the real error.
        if let Some(tab_id) = herdr::created_tab_id(&created) {
            let _ = herdr::run(&herdr::tab_close(&s, &tab_id));
        }
        return Err(e);
    }
    herdr::run(&herdr::agent_prompt(&s, &name, &prompt(&t.uuid, mode)))
        .context("Claude started, but the prompt was not delivered.")?;
    Ok(())
}

/// Poll until the session answers with a workspace in it. A server that
/// answers but never gets one is returned as it is at the deadline — `launch`
/// then creates the workspace itself.
fn wait_for_session(s: &str) -> Result<Value> {
    let deadline = Instant::now() + SESSION_WAIT;
    let mut answered = None;
    loop {
        if let Ok(list) = herdr::run(&herdr::workspace_list(s)) {
            if herdr::first_workspace_id(&list).is_some() {
                return Ok(list);
            }
            answered = Some(list);
        }
        if Instant::now() >= deadline {
            return match answered {
                Some(list) => Ok(list),
                None => bail!("herdr session {s} did not start within {}s.", SESSION_WAIT.as_secs()),
            };
        }
        std::thread::sleep(SESSION_POLL);
    }
}

/// The command the panel's Refine button runs on `uuid`, with `exe` as the
/// `niritasks` binary — built from the button's own arguments, so the two
/// cannot drift apart.
pub fn quick_command(exe: &str, uuid: &str) -> Vec<String> {
    let mut command = vec![exe.to_string()];
    command.extend(crate::panel::actions::Action::Refine.args(uuid));
    command
}

/// Hand a task just added to the refine-task skill — Add & refine.
///
/// In a `niritasks task refine` process of its own, spawned by niri as the
/// panel's Refine button is: the daemon's GTK loop must never wait on herdr,
/// and a child of `niritasks task add` would inherit the `flock` the
/// Mod+Alt+T binds hold and keep it for as long as Claude's terminal stayed
/// open. That process reports its own failures; this only reports failing to
/// start it. Either way the task is already added.
pub fn spawn_quick(uuid: &str) {
    let result = std::env::current_exe()
        .map_err(anyhow::Error::from)
        .and_then(|exe| niri::spawn(quick_command(&exe.to_string_lossy(), uuid)));
    if let Err(e) = result {
        notify::tasks(&format!("Added, but could not start refining it: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Add & refine runs exactly what the panel's Refine button runs, so the
    /// two cannot drift apart.
    #[test]
    fn quick_command_is_the_refine_buttons_command() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        assert_eq!(
            quick_command("/usr/bin/niritasks", u),
            ["/usr/bin/niritasks", "task", "refine", u]
        );
    }

    /// The fence the refine session runs in: every command sandboxed with no
    /// way out, the project read-only, the task database writable, and the
    /// sockets that could start a process outside the sandbox hidden.
    #[test]
    fn the_sandbox_fences_the_session_to_the_task_database() {
        let hidden = [PathBuf::from("/run/user/1000"), PathBuf::from("/run/docker.sock")];
        let json = session_settings(Path::new("/home/x/Projects/alpha"), Path::new("/home/x/.task"), &hidden);
        let v: Value = serde_json::from_str(&json).unwrap();
        let sb = &v["sandbox"];
        assert_eq!(sb["enabled"], true);
        assert_eq!(sb["autoAllowBashIfSandboxed"], true);
        assert_eq!(sb["allowUnsandboxedCommands"], false, "no retrying outside the sandbox");
        assert_eq!(sb["failIfUnavailable"], true, "never run unfenced");
        assert_eq!(sb["filesystem"]["denyWrite"], serde_json::json!(["/home/x/Projects/alpha"]));
        assert_eq!(sb["filesystem"]["allowWrite"], serde_json::json!(["/home/x/.task"]));
        assert_eq!(sb["filesystem"]["denyRead"], serde_json::json!(["/run/user/1000", "/run/docker.sock"]));
        assert_eq!(sb["network"]["allowAllUnixSockets"], true);
    }

    /// The web and the skill's write run unasked; credentials cannot be read
    /// through the Read tool, the one reader the Bash sandbox does not cover.
    #[test]
    fn the_session_may_search_the_web_but_not_read_credentials() {
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[]);
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = &v["permissions"]["allow"];
        for tool in ["WebSearch", "WebFetch", "Bash(task *)", "Bash(python3 *)"] {
            assert!(allow.as_array().unwrap().iter().any(|a| a == tool), "{tool} allowed");
        }
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.iter().any(|d| d == "Read(~/.ssh/**)"));
        assert!(deny.iter().any(|d| d == "Read(~/.claude/.credentials.json)"));
        assert_eq!(deny.len(), CREDENTIALS.len());
    }

    /// Listening sockets come from `/proc/net/unix`: the path is the last
    /// column when there is one, abstract names start with `@` and are left
    /// out — the sandbox's own network namespace already cuts those off.
    #[test]
    fn socket_paths_are_read_from_proc_net_unix() {
        let table = "\
Num       RefCount Protocol Flags    Type St Inode Path
0000000000000000: 00000002 00000000 00010000 0001 01 20779 /run/user/1000/bus
0000000000000000: 00000002 00000000 00010000 0001 01 20780 @/tmp/.X11-unix/X0
0000000000000000: 00000003 00000000 00000000 0001 03 20781
0000000000000000: 00000002 00000000 00010000 0001 01 20782 /tmp/vscode-ipc.sock
";
        assert_eq!(
            socket_paths(table),
            vec![PathBuf::from("/run/user/1000/bus"), PathBuf::from("/tmp/vscode-ipc.sock")]
        );
    }

    /// The check that stops a refine starting next to a socket the sandbox
    /// would leave in reach: anything not under a hidden path is exposed.
    #[test]
    fn a_socket_outside_every_hidden_path_is_exposed() {
        let hidden = [PathBuf::from("/run"), PathBuf::from("/tmp"), PathBuf::from("/home/x/.config/herdr")];
        let sockets = [
            PathBuf::from("/run/docker.sock"),
            PathBuf::from("/tmp/.X11-unix/X0"),
            PathBuf::from("/home/x/.config/herdr/sessions/a/herdr.sock"),
            PathBuf::from("/var/snap/cups/common/run/cups.sock"),
            PathBuf::from("/runner/other.sock"),
        ];
        assert_eq!(
            exposed_sockets(&sockets, &hidden, &[]),
            vec![PathBuf::from("/var/snap/cups/common/run/cups.sock"), PathBuf::from("/runner/other.sock")],
            "hidden by path component, not by string prefix"
        );
    }

    /// The sandbox keeps its own temp folder visible inside a hidden `/tmp`,
    /// so a socket there is exposed even though `/tmp` is hidden.
    #[test]
    fn a_socket_in_the_sandboxes_own_temp_folder_is_exposed() {
        let hidden = [PathBuf::from("/tmp")];
        let visible = [PathBuf::from("/tmp/claude-1000")];
        let sockets = [PathBuf::from("/tmp/claude-1000/x.sock"), PathBuf::from("/tmp/.X11-unix/X0")];
        assert_eq!(
            exposed_sockets(&sockets, &hidden, &visible),
            vec![PathBuf::from("/tmp/claude-1000/x.sock")]
        );
    }

    /// Credentials are hidden from sandboxed Bash by the same list, as paths
    /// under home — but only those that exist, since the sandbox will not
    /// start on a path it cannot find.
    #[test]
    fn credential_paths_resolve_under_home_and_skip_what_is_absent() {
        let home = std::env::temp_dir().join(format!("niritasks-cred-{}", std::process::id()));
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::write(home.join(".claude/.credentials.json"), "{}").unwrap();

        let got = credential_paths(&home);
        assert_eq!(got, vec![home.join(".ssh"), home.join(".claude/.credentials.json")]);

        std::fs::remove_dir_all(&home).ok();
    }

    /// herdr names must match `[a-z][a-z0-9_-]{0,31}`; a uuid's first eight
    /// characters are hex, and enough to tell one task's session from another.
    #[test]
    fn the_agent_is_named_after_its_task() {
        assert_eq!(agent_name("00DEEEE1-3cbd-465d-8c85-c4c4d643b1d0"), "task-00deeee1");
    }

    #[test]
    fn the_prompt_invokes_the_skill_with_the_uuid() {
        assert_eq!(prompt("u-1", Mode::Quick), "/refine-task u-1");
        assert_eq!(prompt("u-1", Mode::Grill), "/refine-task u-1 grill");
    }

    #[test]
    fn the_tab_says_what_it_is_for() {
        assert_eq!(tab_label(Mode::Quick, "fix the peek"), "Refine: fix the peek");
        assert_eq!(tab_label(Mode::Grill, "fix the peek"), "Grill: fix the peek");
    }

    /// A tab label shares herdr's sidebar with every other tab; a long
    /// description is cut, by characters, with an ellipsis.
    #[test]
    fn long_descriptions_are_elided_in_the_label() {
        let label = tab_label(Mode::Quick, &"é".repeat(40));
        assert_eq!(label, format!("Refine: {}…", "é".repeat(29)));
        assert_eq!(tab_label(Mode::Quick, "two\n lines"), "Refine: two lines");
    }

    const GHOSTTY: &str = "com.mitchellh.ghostty";

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn finds_the_ghostty_window_whose_title_ends_with_the_label() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), Some(1));
    }

    /// After Start working, herdr's title names the task's worktree
    /// workspace, not the project's: the window is still the session's.
    #[test]
    fn finds_the_window_showing_a_worktree_workspace() {
        let windows = [(4, Some(GHOSTTY), Some("paul-msi-ubuntu: task/fix-it-6a970973"))];
        let session = labels(&["niri-tasks", "task/fix-it-6a970973"]);
        assert_eq!(find_session_window(&windows, &session), Some(4));
    }

    /// Another session's worktree is not this session's window, even though
    /// both titles start with `task/`.
    #[test]
    fn a_worktree_of_another_session_does_not_match() {
        let windows = [(4, Some(GHOSTTY), Some("paul-msi-ubuntu: task/other-1234abcd"))];
        let session = labels(&["niri-tasks", "task/fix-it-6a970973"]);
        assert_eq!(find_session_window(&windows, &session), None);
    }

    #[test]
    fn no_window_matches_a_different_label() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: other"))];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), None);
    }

    #[test]
    fn a_session_with_no_workspaces_matches_no_window() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, &[]), None);
    }

    #[test]
    fn a_different_app_id_with_the_matching_title_does_not_match() {
        let windows = [(1, Some("org.wezfurlong.wezterm"), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), None);
    }

    /// "tasks" must not match a title ending "niri-tasks" — a label that is a
    /// suffix of another workspace's label is not the same workspace.
    #[test]
    fn a_label_that_is_a_suffix_of_another_label_does_not_match() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: niri-tasks"))];
        assert_eq!(find_session_window(&windows, &labels(&["tasks"])), None);
    }
}
