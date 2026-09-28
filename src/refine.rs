//! Handing a task to Claude in its workspace's herdr session, to be worked up
//! into a plan by the `refine-task` skill.
//!
//! Only the task's uuid crosses over: the skill reads the task itself, so
//! nothing needs quoting through two CLIs and it always sees the current
//! version rather than the one the menu was opened on.

use crate::{herdr, niri, notify, project, session, text};
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
/// exist here: the sockets that could run something outside the sandbox, and
/// the credentials.
fn hidden_paths(home: &Path) -> Vec<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    runtime
        .into_iter()
        .chain([PathBuf::from("/run/docker.sock"), home.join(".config/herdr")])
        .filter(|p| p.exists())
        .chain(credential_paths(home))
        .collect()
}

/// A niri window's id, app_id and title — the columns [`find_session_window`]
/// needs, kept as a tuple rather than `niri_ipc::Window` so a test can build
/// one without niri_ipc's layout fields.
type WindowInfo<'a> = (u64, Option<&'a str>, Option<&'a str>);

/// The ghostty window already showing `label`'s session, if niri has one open.
///
/// herdr sets the outer terminal's title to its `window_title`, default
/// `"{hostname}: {workspace}"` — so a project session's ghostty window's title
/// ends with `": <label>"`, where `label` is that herdr workspace's own label.
/// This depends on that default: a user who changes `window_title` just costs
/// themselves an extra attached terminal window rather than a focus, which is
/// harmless.
fn find_session_window(windows: &[WindowInfo], label: &str) -> Option<u64> {
    let suffix = format!(": {label}");
    windows
        .iter()
        .find(|(_, app_id, title)| {
            *app_id == Some("com.mitchellh.ghostty") && title.is_some_and(|t| t.ends_with(&suffix))
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
    let Some(label) = herdr::first_workspace_label(list) else {
        return Ok(());
    };
    let windows = niri::windows()?;
    let info: Vec<WindowInfo> = windows
        .iter()
        .map(|w| (w.id, w.app_id.as_deref(), w.title.as_deref()))
        .collect();
    match find_session_window(&info, &label) {
        Some(id) => niri::focus_window(id)?,
        None => niri::spawn(project::project_terminal_command(dir, s, true))?,
    }
    Ok(())
}

/// Open Claude on a task in `workspace`'s herdr session.
///
/// Opens the project terminal first if the session is not running, and goes
/// back to the task's existing tab if it is already being refined.
pub fn launch(workspace: &str, uuid: &str, description: &str, mode: Mode) -> Result<()> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let dir = session::start_dir(Path::new(&home), workspace);
    let s = session::herdr_session_name(workspace);
    let name = agent_name(uuid);

    // Whether the session was already running when we asked. A session we
    // spawn ourselves below already has a window in front of the user; one
    // that was already running might have had its window closed while the
    // herdr server it belongs to kept going.
    let mut already_running = true;
    let list = match herdr::run(&herdr::workspace_list(&s)) {
        Ok(list) => list,
        Err(_) => {
            anyhow::ensure!(
                project::on_path(project::SESSION_MANAGER),
                "herdr is not installed, so there is no session to refine in."
            );
            // Through niri, so the window lands on the focused workspace —
            // the one the task belongs to — as the project picker's does.
            niri::spawn(project::project_terminal_command(&dir, &s, true))?;
            already_running = false;
            wait_for_session(&s)?
        }
    };

    if already_running {
        show_session_window(&dir, &s, &list)?;
    }

    if herdr::run(&herdr::agent_get(&s, &name)).is_ok() {
        herdr::run(&herdr::agent_focus(&s, &name))?;
        notify::tasks("Already being refined — switched to its tab.");
        return Ok(());
    }

    let label = tab_label(mode, description);
    let created = match herdr::first_workspace_id(&list) {
        Some(id) => herdr::run(&herdr::tab_create(&s, &id, &dir, &label))?,
        // The workspace itself is named after the project, not this tab.
        None => herdr::run(&herdr::workspace_create(&s, &dir, workspace))?,
    };
    let pane = herdr::root_pane_id(&created).context("herdr did not say which pane it made")?;

    let home = Path::new(&home);
    let settings = session_settings(&dir, &crate::task::data_location()?, &hidden_paths(home));
    if let Err(e) = herdr::run(&herdr::agent_start_claude_refiner(&s, &name, &pane, &settings)) {
        // Best-effort: a retry should not find a pile of bare-shell tabs from
        // every failed attempt, but a failure here must not hide the real error.
        if let Some(tab_id) = herdr::created_tab_id(&created) {
            let _ = herdr::run(&herdr::tab_close(&s, &tab_id));
        }
        return Err(e);
    }
    herdr::run(&herdr::agent_prompt(&s, &name, &prompt(uuid, mode)))
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn finds_the_ghostty_window_whose_title_ends_with_the_label() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, "hansard"), Some(1));
    }

    #[test]
    fn no_window_matches_a_different_label() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: other"))];
        assert_eq!(find_session_window(&windows, "hansard"), None);
    }

    #[test]
    fn a_different_app_id_with_the_matching_title_does_not_match() {
        let windows = [(1, Some("org.wezfurlong.wezterm"), Some("paul-msi-ubuntu: hansard"))];
        assert_eq!(find_session_window(&windows, "hansard"), None);
    }

    /// "tasks" must not match a title ending "niri-tasks" — a label that is a
    /// suffix of another workspace's label is not the same workspace.
    #[test]
    fn a_label_that_is_a_suffix_of_another_label_does_not_match() {
        let windows = [(1, Some(GHOSTTY), Some("paul-msi-ubuntu: niri-tasks"))];
        assert_eq!(find_session_window(&windows, "tasks"), None);
    }
}
