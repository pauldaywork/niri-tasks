//! Handing a task to Claude in its workspace's herdr session, to be worked up
//! into a plan by the `refine-task` skill.
//!
//! Only the task's uuid crosses over: the skill reads the task itself, so
//! nothing needs quoting through two CLIs and it always sees the current
//! version rather than the one the panel showed.

use crate::dirs::Dirs;
use crate::session::{Claude, Session};
use crate::workspace::Workspace;
use crate::{names, niri, notify, task};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Which of a card's two buttons, Refine or Grill me, opened Claude. The two differ only in the
/// prompt [`prompt`] sends — the skill on the other end reads it to decide
/// whether to interview before drafting or draft straight away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Draft straight away; ask only what the code cannot answer.
    Quick,
    /// A full interview, via the `grilling` skill, before the draft.
    Grill,
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
    format!("{verb}: {}", names::elide(description))
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

/// The refine mod's plugin name, as `claude/refine-mod/.claude-plugin/plugin.json`
/// declares it: the key its options travel under in `--settings`.
pub const REFINE_MOD: &str = "niri-tasks-refine";

/// Where `install.sh` links the refine mod's folder:
/// `$XDG_DATA_HOME/niri-tasks/refine-mod`, else under `~/.local/share`.
///
/// A fixed path because `niritasks` is `cargo install`ed and does not know
/// where the repo is. Never under `~/.claude/skills`: a plugin there loads in
/// every Claude session, not just a refine. A relative `XDG_DATA_HOME` is
/// ignored, as the XDG spec says.
pub fn refine_mod_dir(dirs: &Dirs) -> PathBuf {
    dirs.data().join("niri-tasks/refine-mod")
}

/// Where a refine's HTML report is written: beside the other reviews, under
/// the XDG data folder. Outside the session's sandbox, so only the mod writes
/// there.
pub fn reports_dir(dirs: &Dirs) -> PathBuf {
    dirs.data().join("niri-tasks/reviews")
}

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
/// `task`, which Claude Code asks about even inside the sandbox, for reasons
/// it does not log — the skill reads with it, and it still runs sandboxed.
/// The write itself is the refine mod's tool, which `pluginConfigs` tells
/// which task it may write: `uuid`, never anything the model says.
/// It is told `reports` too, the folder its report tool writes to.
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path) -> String {
    let deny: Vec<String> = CREDENTIALS.iter().map(|c| format!("Read({c})")).collect();
    serde_json::json!({
        "permissions": {
            "allow": ["WebSearch", "WebFetch", "Bash(task *)"],
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
        },
        "pluginConfigs": {
            (REFINE_MOD): { "options": { "uuid": uuid, "reports": reports } },
        },
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

/// Open Claude on a task in `ws`'s herdr session.
///
/// Opens the project terminal first if the session is not running, and goes
/// back to the task's existing tab if it is already being refined.
///
/// The prompt is confirmed, and resent if Claude was still starting, as
/// Start working's is; a start Claude blocks on a question is waited out the
/// same way.
///
/// The task as found, not a uuid as typed: the session is found again by the
/// uuid's first eight characters, and a typed task number has none of them.
pub fn launch(ws: &Workspace, t: &task::Task, mode: Mode) -> Result<()> {
    let dirs = Dirs::from_env()?;
    let home = dirs.home();
    let name = names::refine_agent(&t.uuid);

    // Before anything opens: a refused refine should leave nothing behind.
    let hidden = hidden_paths(home);
    ensure_no_exposed_sockets(&hidden)?;
    let mod_dir = refine_mod_dir(&dirs);
    // Without the mod the session would have no way to write its plan, and
    // the user would find that out only at the end of the interview.
    anyhow::ensure!(
        mod_dir.join(".claude-plugin/plugin.json").is_file(),
        "The refine mod is missing, or its link is broken, at {}. Run install.sh from the niri-tasks repo.",
        mod_dir.display()
    );
    let session = ws.session()?;
    let settings = session_settings(session.dir(), &task::data_location()?, &hidden, &t.uuid, &reports_dir(&dirs));

    let claude = Claude::Refiner { settings, mod_dir };
    let label = tab_label(mode, &t.description);
    if launch_in(&session, &name, &label, ws.name(), &claude, &prompt(&t.uuid, mode))? == Launched::AlreadyRunning {
        session.notify("Already being refined — switched to its tab.");
    }
    Ok(())
}

/// Whether [`launch_in`] started the refine session or found it running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Launched {
    /// A new Claude, prompted.
    Started,
    /// The task's refiner was live already, and is now focused.
    AlreadyRunning,
}

/// Refine's steps in the session, apart from what reads the machine: open
/// the session, go back to `name` if it is live, else start it in a new tab
/// labelled `label` and hand it `text`. Kept apart so the order of the steps
/// can be checked against the fake.
///
/// All of it up to `agent start` runs under the session's claim, so a
/// second press of Refine waits here and then finds this one's Claude,
/// rather than finding none and starting another beside it. The claim is let
/// go as soon as herdr has the agent, before a blocked start's question and
/// the prompt's retries, either of which can take a while: by then the
/// agent exists for that second press to find.
fn launch_in(session: &Session, name: &str, label: &str, workspace_label: &str, claude: &Claude, text: &str) -> Result<Launched> {
    let claim = session.claim()?;
    let workspaces = session.open()?;
    if session.focus_agent(name)? {
        return Ok(Launched::AlreadyRunning);
    }

    let tab = session.new_tab(&workspaces, session.dir(), label, workspace_label)?;
    // The claim goes inside, once herdr has the agent under its name.
    if let Err(e) = session.start_claude(claim, name, &tab.pane, claude) {
        // A retry should not find a pile of bare-shell tabs from every
        // failed attempt; closing is best effort, so the real error stands.
        if let Some(tab) = &tab.tab {
            session.close_tab(tab);
        }
        return Err(e);
    }
    session.prompt(name, text)?;
    Ok(Launched::Started)
}

/// Refine's own command on `uuid`, with `exe` as the `niritasks` binary:
/// built from the task action's arguments, so Add & refine runs what the
/// panel's Refine button runs.
pub fn quick_command(exe: &str, uuid: &str) -> Vec<String> {
    let mut command = vec![exe.to_string()];
    command.extend(crate::actions::Action::Refine.args(uuid));
    command
}

/// Hand a task just added to the refine-task skill — Add & refine.
///
/// In a `niritasks task refine` process of its own, spawned by niri as the
/// panel's Refine button is: the daemon's GTK loop must never wait on herdr.
/// That process reports its own failures; this only reports failing to
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
    use serde_json::Value;

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
        let json = session_settings(Path::new("/home/x/Projects/alpha"), Path::new("/home/x/.task"), &hidden, "u", Path::new("/r"));
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

    /// The web and the skill's reads run unasked; credentials cannot be read
    /// through the Read tool, the one reader the Bash sandbox does not cover.
    /// No `python3`: the write is the mod's tool now.
    #[test]
    fn the_session_may_search_the_web_but_not_read_credentials() {
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], "u", Path::new("/r"));
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        for tool in ["WebSearch", "WebFetch", "Bash(task *)"] {
            assert!(allow.iter().any(|a| a == tool), "{tool} allowed");
        }
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("python3"))));
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.iter().any(|d| d == "Read(~/.ssh/**)"));
        assert!(deny.iter().any(|d| d == "Read(~/.claude/.credentials.json)"));
        assert_eq!(deny.len(), CREDENTIALS.len());
    }

    /// The task the mod may write, and where it may write a report, come from
    /// these settings, never from the model.
    #[test]
    fn the_mod_is_told_which_task_it_may_write_and_where_reports_go() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/home/x/.local/share/niri-tasks/reviews"));
        let v: Value = serde_json::from_str(&json).unwrap();
        let options = &v["pluginConfigs"][REFINE_MOD]["options"];
        assert_eq!(options["uuid"], u);
        assert_eq!(options["reports"], "/home/x/.local/share/niri-tasks/reviews");
        assert_eq!(REFINE_MOD, "niri-tasks-refine", "the name in claude/refine-mod's plugin.json");
    }

    /// Reports go beside the other reviews, under the XDG data folder: the
    /// session's sandbox cannot write there, so only the mod can.
    #[test]
    fn reports_go_in_the_reviews_folder() {
        let dirs = Dirs::at(Path::new("/home/x"));
        assert_eq!(reports_dir(&dirs), PathBuf::from("/home/x/.local/share/niri-tasks/reviews"));
        assert_eq!(
            reports_dir(&dirs.with_data(Path::new("/data"))),
            PathBuf::from("/data/niri-tasks/reviews")
        );
    }

    /// Where install.sh links the mod: under the XDG data folder, never under
    /// ~/.claude/skills, where a plugin would load in every session.
    #[test]
    fn the_mod_lives_under_the_xdg_data_folder() {
        let dirs = Dirs::at(Path::new("/home/x"));
        assert_eq!(refine_mod_dir(&dirs), PathBuf::from("/home/x/.local/share/niri-tasks/refine-mod"));
        assert_eq!(
            refine_mod_dir(&dirs.clone().with_data(Path::new("/data"))),
            PathBuf::from("/data/niri-tasks/refine-mod")
        );
        // The XDG spec says a relative path is to be ignored.
        assert_eq!(
            refine_mod_dir(&dirs.with_data(Path::new("rel"))),
            PathBuf::from("/home/x/.local/share/niri-tasks/refine-mod")
        );
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

    use crate::session::fake::{Fake, StartOutcome};

    fn refiner() -> Claude {
        Claude::Refiner { settings: "{}".into(), mod_dir: PathBuf::from("/m") }
    }

    /// Refine's session steps over `fake`, as `launch` runs them.
    fn refine(fake: &Fake) -> Result<Launched> {
        let session = Session::with_port("alpha", PathBuf::from("/p/alpha"), Box::new(fake.clone()));
        launch_in(&session, "task-abc", "Refine: x", "alpha", &refiner(), "/refine-task abc")
    }

    /// Where `line` first appears in the log, failing the test when it is
    /// not there at all.
    fn at(log: &[String], line: &str) -> usize {
        log.iter().position(|l| l.starts_with(line)).unwrap_or_else(|| panic!("no {line:?} in {log:?}"))
    }

    /// The race of two quick Refine presses: each would find no agent and
    /// start one. The claim is what stops it, so it has to be taken before
    /// the session is even opened (a stopped session would otherwise get two
    /// project terminals) and held until Claude has started, then given up
    /// before the prompt, whose retries can take seconds while the second
    /// press could already find and focus the agent.
    #[test]
    fn refine_holds_the_session_from_opening_it_until_claude_has_started() {
        let fake = Fake::running(&[("w1", "alpha")]);
        assert_eq!(refine(&fake).unwrap(), Launched::Started);
        let log = fake.log();
        assert_eq!(log[0], "claim alpha", "{log:?}");
        assert!(at(&log, "release alpha") > at(&log, "agent_start alpha task-abc"), "{log:?}");
        assert!(at(&log, "release alpha") < at(&log, "agent_prompt alpha task-abc"), "{log:?}");
    }

    /// The second of two presses, once the first has let go: it opens the
    /// session, finds the first one's Claude and focuses it, starting
    /// nothing; one agent in all.
    #[test]
    fn refine_twice_on_one_task_ends_with_one_agent() {
        let fake = Fake::running(&[("w1", "alpha")]);
        assert_eq!(refine(&fake).unwrap(), Launched::Started);
        assert_eq!(refine(&fake).unwrap(), Launched::AlreadyRunning);
        let log = fake.log();
        assert_eq!(log.iter().filter(|l| l.starts_with("agent_start ")).count(), 1, "{log:?}");
        assert_eq!(fake.agents().len(), 1);
        let second: Vec<&String> = log.iter().skip(at(&log, "agent_prompt ") + 1).collect();
        assert_eq!(second.first().map(|l| l.as_str()), Some("claim alpha"), "{log:?}");
        assert_eq!(second.last().map(|l| l.as_str()), Some("release alpha"), "{log:?}");
        assert!(second.iter().any(|l| *l == "agent_focus alpha task-abc"), "{log:?}");
    }

    /// A failed start lets go as soon as `agent start` returns, and still
    /// closes its bare tab after.
    #[test]
    fn a_failed_refine_lets_go_and_closes_its_tab() {
        let fake = Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::Fails);
        assert!(refine(&fake).is_err());
        let log = fake.log();
        assert_eq!(log[0], "claim alpha", "{log:?}");
        assert!(at(&log, "agent_start alpha task-abc") < at(&log, "release alpha"), "{log:?}");
        assert!(at(&log, "tab_close alpha") > at(&log, "release alpha"), "{log:?}");
        assert!(!log.iter().any(|l| l.starts_with("agent_prompt")), "{log:?}");
    }

    /// A start blocked on a question lets go before waiting for the answer,
    /// so a second press can land in the tab that wants it.
    #[test]
    fn a_blocked_refine_lets_go_before_waiting_for_the_answer() {
        let fake = Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::FailsBlockedThenIdle);
        assert_eq!(refine(&fake).unwrap(), Launched::Started);
        let log = fake.log();
        assert!(at(&log, "agent_start alpha task-abc") < at(&log, "release alpha"), "{log:?}");
        assert!(at(&log, "release alpha") < at(&log, "agent_wait_ready alpha task-abc"), "{log:?}");
        assert!(at(&log, "release alpha") < at(&log, "agent_prompt alpha task-abc"), "{log:?}");
    }
}
