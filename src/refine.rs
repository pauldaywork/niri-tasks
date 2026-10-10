//! Handing a task to Claude in its workspace's herdr session, to be worked up
//! into a plan by the `refine-task` skill, or, once it has one, reported on
//! by `report-task`.
//!
//! Only the task's uuid crosses over: the skill reads the task itself, so
//! nothing needs quoting through two CLIs and it always sees the current
//! version rather than the one the panel showed.

use crate::dirs::Dirs;
use crate::session::{Claude, Session};
use crate::workspace::Workspace;
use crate::{names, niri, notify, programs, task};
use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Which of a card's buttons, Refine, Grill me or Report, opened Claude. They
/// differ in the prompt [`prompt`] sends, and so in the skill on the other
/// end: the two refines share `refine-task`, which reads the mode to decide
/// whether to interview before drafting, while a report runs `report-task`
/// on the plan the task already holds. A report session also tells the mod
/// so, through [`session_settings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Draft straight away; ask only what the code cannot answer.
    Quick,
    /// A full interview, via the `grilling` skill, before the draft.
    Grill,
    /// No refine: the report of a planned task's plan, opened in the browser
    /// and linked from the task.
    Report,
}

/// Only the uuid crosses to the skill — the description and everything else
/// about the task is read by the skill itself, so nothing here needs quoting
/// through two CLIs (this one, then herdr's `agent prompt`).
pub fn prompt(uuid: &str, mode: Mode) -> String {
    match mode {
        Mode::Quick => format!("/refine-task {uuid}"),
        Mode::Grill => format!("/refine-task {uuid} grill"),
        Mode::Report => format!("/report-task {uuid}"),
    }
}

/// The verb says which mode the tab is in; the description is cut short
/// because the label shares herdr's sidebar with every other tab's label.
pub fn tab_label(mode: Mode, description: &str) -> String {
    let verb = match mode {
        Mode::Quick => "Refine",
        Mode::Grill => "Grill",
        Mode::Report => "Report",
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

/// Where an unattended run's log goes: `$XDG_DATA_HOME/niri-tasks/unattended`,
/// beside the reviews, so a failed run can be read later.
pub fn unattended_dir(dirs: &Dirs) -> PathBuf {
    dirs.data().join("niri-tasks/unattended")
}

/// The log of an unattended run on `uuid` started at `now` (Unix seconds):
/// `<uuid8>-<stamp>.log` under [`unattended_dir`], one file for both of its
/// Claude runs, named by time so a second run on the task keeps the first's.
pub fn log_path(dirs: &Dirs, uuid: &str, now: i64) -> PathBuf {
    unattended_dir(dirs).join(format!("{}-{}.log", names::uuid8(uuid), task::stamp(now)))
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
/// The Read tool may read the installed skills too, and the real folders
/// the links point into ([`skill_dirs`]): the report catalogue lives there,
/// outside the project, where Read would otherwise ask, and sandboxed Bash
/// can read it regardless. The rule names ~/.claude, Claude Code's default; a
/// CLAUDE_CONFIG_DIR elsewhere would bring the prompt back.
/// The write itself is the refine mod's tool, which `pluginConfigs` tells
/// which task it may write: `uuid`, never anything the model says.
/// It is told `reports` too, the folder its report tool writes to, and
/// `report`: true in a Report session, where it offers only the report tool
/// and links the report from the task itself, and `unattended`: true in an
/// unattended run, where the write tool writes with no question and the
/// report tool opens nothing (see [`run_unattended`]).
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path, skill_dirs: &[PathBuf], mode: Mode, unattended: bool) -> String {
    let deny: Vec<String> = CREDENTIALS.iter().map(|c| format!("Read({c})")).collect();
    let mut allow: Vec<String> =
        ["WebSearch", "WebFetch", "Bash(task *)", "Read(~/.claude/skills/**)"].iter().map(|s| s.to_string()).collect();
    // `//` is Claude Code's absolute form; one `/` would be the project's.
    allow.extend(skill_dirs.iter().map(|dir| format!("Read(//{}/**)", dir.display().to_string().trim_start_matches('/'))));
    serde_json::json!({
        "permissions": {
            "allow": allow,
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
            (REFINE_MOD): { "options": { "uuid": uuid, "reports": reports, "report": (mode == Mode::Report), "unattended": unattended } },
        },
    })
    .to_string()
}

/// The real folders of the installed skill files the session's Read tool
/// has to read: today the refine-task catalogue's, which both the report
/// skill and a refine's "Show me a report first" read. `install.sh` links
/// the file into the repo, and Claude Code lets the Read tool through a
/// link only when the link and its target are both allowed, so
/// [`session_settings`] allows these beside `~/.claude/skills`. Empty when
/// the catalogue is not installed: the skill then stops and says so.
pub fn skill_dirs(home: &Path) -> Vec<PathBuf> {
    let catalogue = home.join(".claude/skills/refine-task/report-catalogue.md");
    std::fs::canonicalize(catalogue)
        .ok()
        .and_then(|file| file.parent().map(Path::to_path_buf))
        .into_iter()
        .collect()
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
    let settings = session_settings(
        session.dir(),
        &task::data_location()?,
        &hidden,
        &t.uuid,
        &reports_dir(&dirs),
        &skill_dirs(home),
        mode,
        false,
    );

    let claude = Claude::Refiner { settings, mod_dir };
    let label = tab_label(mode, &t.description);
    if launch_in(&session, &name, &label, ws.name(), &claude, &prompt(&t.uuid, mode))? == Launched::AlreadyRunning {
        // The same agent name for all three modes, so a press of any lands in the tab already open on the task.
        session.notify("Already open on this task — switched to its tab.");
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

/// The flag that makes `task refine` run with no one watching.
pub const UNATTENDED_FLAG: &str = "--unattended";

/// Add & all's command on `uuid`: the Refine button's ([`quick_command`])
/// with [`UNATTENDED_FLAG`] after it, so it cannot drift from Refine either.
pub fn unattended_command(exe: &str, uuid: &str) -> Vec<String> {
    let mut command = quick_command(exe, uuid);
    command.push(UNATTENDED_FLAG.to_string());
    command
}

/// Hand a task just added to an unattended refine and report — Add & all.
///
/// In a `niritasks task refine --unattended` process of its own, spawned by
/// niri as [`spawn_quick`]'s is: the daemon's GTK loop must never wait on a
/// Claude run that takes minutes. That process reports its own failures;
/// this only reports failing to start it. Either way the task is added.
pub fn spawn_unattended(uuid: &str) {
    let result = std::env::current_exe()
        .map_err(anyhow::Error::from)
        .and_then(|exe| niri::spawn(unattended_command(&exe.to_string_lossy(), uuid)));
    if let Err(e) = result {
        notify::tasks(&format!("Added, but could not start planning it: {e}"));
    }
}

/// Claude Code's binary, which an unattended run executes itself.
pub const CLAUDE: &str = "claude";

/// The two prompts an unattended run sends, in order: `refine-task` in its
/// `auto` mode, which decides every open question itself and writes the
/// plan with no question asked, then `report-task` on the plan it wrote.
pub fn unattended_prompts(uuid: &str) -> [String; 2] {
    [format!("/refine-task {uuid} auto"), prompt(uuid, Mode::Report)]
}

/// What an unattended run executes: `claude -p <prompt>` with the tab's own
/// flags ([`crate::session::refiner_flags`]: default permission mode, the
/// editing tools removed, the standing instruction, the settings fence and
/// the mod) and `--permission-prompts none`, so anything that would ask — a
/// permission, AskUserQuestion — is refused rather than waited on. Not plan
/// mode and not auto mode, for the reasons `agent_start_claude_refiner`
/// gives; the sandbox already runs every command unasked.
pub fn claude_unattended_argv(prompt: &str, settings: &str, mod_dir: &Path) -> Vec<String> {
    let mut argv = vec![CLAUDE.to_string(), "-p".to_string(), prompt.to_string()];
    argv.extend(crate::session::refiner_flags(settings, mod_dir));
    argv.extend(["--permission-prompts", "none"].map(String::from));
    argv
}

/// What the notification says when both runs are done, before the task's
/// description.
pub const PLANNED_AND_REPORTED: &str = "Planned and reported";

/// Refine a task and build its report with no one watching: Add & all's
/// second half, and `niritasks task refine <uuid> --unattended`.
///
/// Two `claude -p` runs in this process, in the project folder, behind the
/// fence a refine tab gets ([`session_settings`] with `unattended` set,
/// [`claude_unattended_argv`]): `/refine-task <uuid> auto`, which writes
/// the plan with no question, then, once the task is planned,
/// `/report-task <uuid>`, which writes and links the report without opening
/// it. No herdr and no window.
///
/// `+processing` goes on the task before anything runs and comes off on
/// every way out of here — done, a run that failed, a run that wrote
/// nothing, `claude` or the mod missing — so its card shows it is being
/// planned and offers nothing meanwhile, and nothing else writes it. A run
/// this process does not control, killed with it, leaves the tag; that is
/// `task <uuid8> modify -processing`. Both runs' output goes to one log
/// ([`log_path`]), which a failure's error names; the error is `main`'s to
/// notify, so a failure is said once.
pub fn run_unattended(ws: &Workspace, t: &task::Task) -> Result<()> {
    let dirs = Dirs::from_env()?;
    let log = log_path(&dirs, &t.uuid, task::now_secs());
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("could not make {}", dir.display()))?;
    }
    task::set_processing(&t.uuid, true)?;
    let outcome = unattended_steps(ws, t, &dirs, &log);
    // Cleared whatever happened above; a tag left behind would lock the
    // card for good.
    if let Err(e) = task::set_processing(&t.uuid, false) {
        eprintln!("could not clear +{}: {e:#}", task::PROCESSING_TAG);
    }
    match outcome {
        Ok(()) => {
            notify::tasks(&format!("{PLANNED_AND_REPORTED}: {}", t.description));
            Ok(())
        }
        Err(e) => {
            append_line(&log, &format!("== failed: {e:#}"));
            bail!("{e:#} Log: {}", log.display())
        }
    }
}

/// Which of the two runs a failure is reported against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Refine,
    Report,
}

impl Step {
    fn name(self) -> &'static str {
        match self {
            Step::Refine => "refine",
            Step::Report => "report",
        }
    }
}

/// Everything [`run_unattended`] does between tagging the task and
/// untagging it: the fence checks a tab makes, then the two runs, each
/// followed by reading the task back to see it did its job.
fn unattended_steps(ws: &Workspace, t: &task::Task, dirs: &Dirs, log: &Path) -> Result<()> {
    let home = dirs.home();
    let hidden = hidden_paths(home);
    ensure_no_exposed_sockets(&hidden)?;
    let mod_dir = refine_mod_dir(dirs);
    anyhow::ensure!(
        mod_dir.join(".claude-plugin/plugin.json").is_file(),
        "The refine mod is missing, or its link is broken, at {}. Run install.sh from the niri-tasks repo.",
        mod_dir.display()
    );
    anyhow::ensure!(programs::on_path(CLAUDE), "Claude Code (`{CLAUDE}`) is not on PATH, so nothing can plan the task.");
    let session = ws.session()?;
    let project = session.dir();
    let task_data = task::data_location()?;
    let reports = reports_dir(dirs);
    let skills = skill_dirs(home);
    let [refine_prompt, report_prompt] = unattended_prompts(&t.uuid);

    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, &skills, Mode::Quick, true);
    run_claude(project, log, Step::Refine, &claude_unattended_argv(&refine_prompt, &settings, &mod_dir))?;
    let planned = task::get(&t.uuid)?.context("The task is gone after the refine step.")?;
    anyhow::ensure!(planned.is_planned(), "The refine step wrote no plan: the task is not tagged planned.");

    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, &skills, Mode::Report, true);
    run_claude(project, log, Step::Report, &claude_unattended_argv(&report_prompt, &settings, &mod_dir))?;
    let reported = task::get(&t.uuid)?.context("The task is gone after the report step.")?;
    let path = reported.report_path().context("The report step linked no report: the task has no Report: note.")?;
    anyhow::ensure!(Path::new(path).is_file(), "The report step linked {path}, which is not there.");
    Ok(())
}

/// One `claude -p` run in `project`, both its streams appended to `log`
/// under a line naming the step and the prompt (not the settings, which are
/// long and the same for both). Stdin is closed: there is no one typing.
fn run_claude(project: &Path, log: &Path, step: Step, argv: &[String]) -> Result<()> {
    append_line(log, &format!("== {}: {}", step.name(), argv[..3].join(" ")));
    let out = File::options().append(true).create(true).open(log).with_context(|| format!("could not open {}", log.display()))?;
    let err = out.try_clone().context("could not share the log between stdout and stderr")?;
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .with_context(|| format!("could not run {}", argv[0]))?;
    anyhow::ensure!(status.success(), "The {} step failed: claude exited with {status}.", step.name());
    Ok(())
}

/// Add `line` to the log. Best effort: the log is for reading a failure
/// later, and a log that cannot be written must not hide the failure itself.
fn append_line(log: &Path, line: &str) {
    if let Ok(mut file) = File::options().append(true).create(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
}

/// The command that opens a report already written, `path`, in the browser:
/// what the mod runs when it opens one it has just written (`openArgv` in
/// `hooks/report.ts`). `xdg-open` detached under a shell, so the browser
/// is not waited on, with the path as the shell's `$1`, never quoted into
/// the script.
pub fn open_report_command(path: &str) -> Vec<String> {
    vec![
        "sh".to_string(),
        "-c".to_string(),
        "xdg-open \"$1\" >/dev/null 2>&1 </dev/null &".to_string(),
        "sh".to_string(),
        path.to_string(),
    ]
}

/// Open the report at `path` in the browser, through niri, so the browser
/// is niri's child and outlives the `niritasks` process a button press
/// spawned. The caller has checked the file is there.
pub fn open_report(path: &str) -> Result<()> {
    niri::spawn(open_report_command(path)).with_context(|| format!("could not open the report {path}"))
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

    /// Open report runs what the mod runs when it opens a report it just
    /// wrote (`openArgv` in `hooks/report.ts`): `xdg-open` detached under a
    /// shell, the path passed as an argument and never quoted into the
    /// script, so a path with a space or a quote opens the same file.
    #[test]
    fn open_report_runs_xdg_open_on_the_path_as_an_argument() {
        let command = open_report_command("/home/x/.local/share/niri-tasks/reviews/a report's.html");
        assert_eq!(command[0], "sh");
        assert_eq!(command[1], "-c");
        assert_eq!(command[2], "xdg-open \"$1\" >/dev/null 2>&1 </dev/null &");
        assert_eq!(command[3], "sh");
        assert_eq!(command[4], "/home/x/.local/share/niri-tasks/reviews/a report's.html");
        assert_eq!(command.len(), 5);
    }

    /// The fence the refine session runs in: every command sandboxed with no
    /// way out, the project read-only, the task database writable, and the
    /// sockets that could start a process outside the sandbox hidden.
    #[test]
    fn the_sandbox_fences_the_session_to_the_task_database() {
        let hidden = [PathBuf::from("/run/user/1000"), PathBuf::from("/run/docker.sock")];
        let json = session_settings(Path::new("/home/x/Projects/alpha"), Path::new("/home/x/.task"), &hidden, "u", Path::new("/r"), &[], Mode::Quick, false);
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

    /// The web and the skill's reads run unasked, and so does the Read tool
    /// on the installed skills, where the report catalogue lives: outside
    /// the project, so the Read tool would otherwise ask, while sandboxed
    /// Bash can read it anyway. Credentials cannot be read through the Read
    /// tool, the one reader the Bash sandbox does not cover. No `python3`:
    /// the write is the mod's tool now.
    #[test]
    fn the_session_may_search_the_web_but_not_read_credentials() {
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], "u", Path::new("/r"), &[], Mode::Quick, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        for tool in ["WebSearch", "WebFetch", "Bash(task *)", "Read(~/.claude/skills/**)"] {
            assert!(allow.iter().any(|a| a == tool), "{tool} allowed");
        }
        assert_eq!(allow.len(), 4, "nothing else runs unasked");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("python3"))));
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.iter().any(|d| d == "Read(~/.ssh/**)"));
        assert!(deny.iter().any(|d| d == "Read(~/.claude/.credentials.json)"));
        assert_eq!(deny.len(), CREDENTIALS.len());
    }

    /// install.sh links the skills into the repo, and Claude Code lets the
    /// Read tool through a link only when the link and its target are both
    /// allowed, so each resolved skill folder gets a rule of its own, in the
    /// absolute `//` form. The order is the rules' own, so a test can pin it.
    #[test]
    fn each_resolved_skill_folder_is_allowed_beside_the_links() {
        let dirs = [PathBuf::from("/home/x/Projects/niri-tasks/.claude/skills/refine-task")];
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], "u", Path::new("/r"), &dirs, Mode::Report, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow[3], "Read(~/.claude/skills/**)");
        assert_eq!(allow[4], "Read(//home/x/Projects/niri-tasks/.claude/skills/refine-task/**)");
        assert_eq!(allow.len(), 5);
    }

    /// The catalogue's real folder, found through its installed link; none
    /// when it is not installed, since the skill then stops and says so.
    #[test]
    fn skill_dirs_resolve_the_installed_catalogue_and_skip_what_is_absent() {
        let home = std::env::temp_dir().join(format!("niritasks-skills-{}", std::process::id()));
        let repo = home.join("repo/.claude/skills/refine-task");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("report-catalogue.md"), "# Refine report catalogue\n").unwrap();
        let installed = home.join(".claude/skills/refine-task");
        std::fs::create_dir_all(&installed).unwrap();
        std::os::unix::fs::symlink(repo.join("report-catalogue.md"), installed.join("report-catalogue.md")).unwrap();

        assert_eq!(skill_dirs(&home), vec![std::fs::canonicalize(&repo).unwrap()]);

        std::fs::remove_dir_all(&home).ok();
        assert_eq!(skill_dirs(&home), Vec::<PathBuf>::new());
    }

    /// The task the mod may write, and where it may write a report, come from
    /// these settings, never from the model.
    #[test]
    fn the_mod_is_told_which_task_it_may_write_and_where_reports_go() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/home/x/.local/share/niri-tasks/reviews"), &[], Mode::Quick, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        let options = &v["pluginConfigs"][REFINE_MOD]["options"];
        assert_eq!(options["uuid"], u);
        assert_eq!(options["reports"], "/home/x/.local/share/niri-tasks/reviews");
        assert_eq!(options["report"], false, "a refine is not a report session");
        assert_eq!(REFINE_MOD, "niri-tasks-refine", "the name in claude/refine-mod's plugin.json");
    }

    /// A Report session tells the mod so: it offers only the report tool,
    /// armed from the start, and links the report from the task itself.
    #[test]
    fn a_report_session_tells_the_mod_so() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), &[], Mode::Report, false);
        let v: Value = serde_json::from_str(&json).unwrap();
        let options = &v["pluginConfigs"][REFINE_MOD]["options"];
        assert_eq!(options["report"], true);
        assert_eq!(options["uuid"], u, "the same fence and task as a refine");
        assert_eq!(options["reports"], "/r");
        for mode in [Mode::Quick, Mode::Grill] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), &[], mode, false);
            let v: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["report"], false, "{mode:?}");
        }
    }

    /// An unattended run tells the mod so, in both of its modes: the write
    /// tool writes with no question and the report tool opens nothing. A tab
    /// never does.
    #[test]
    fn an_unattended_run_tells_the_mod_so() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        for mode in [Mode::Quick, Mode::Report] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), &[], mode, true);
            let v: Value = serde_json::from_str(&json).unwrap();
            let options = &v["pluginConfigs"][REFINE_MOD]["options"];
            assert_eq!(options["unattended"], true, "{mode:?}");
            assert_eq!(options["report"], mode == Mode::Report, "{mode:?}");
            assert_eq!(options["uuid"], u, "the same fence and task");
        }
        for mode in [Mode::Quick, Mode::Grill, Mode::Report] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), &[], mode, false);
            let v: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["unattended"], false, "{mode:?}");
        }
    }

    /// Add & all runs the Refine button's command with --unattended after
    /// it, so it cannot drift from Refine either.
    #[test]
    fn unattended_command_is_the_refine_command_plus_the_flag() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        assert_eq!(
            unattended_command("/usr/bin/niritasks", u),
            ["/usr/bin/niritasks", "task", "refine", u, "--unattended"]
        );
    }

    /// The two prompts, in the order the runs go: refine-task's auto mode,
    /// which asks nothing, then report-task.
    #[test]
    fn unattended_prompts_refine_automatically_then_report() {
        assert_eq!(unattended_prompts("u-1"), ["/refine-task u-1 auto", "/report-task u-1"]);
        assert_eq!(unattended_prompts("u-1")[1], prompt("u-1", Mode::Report), "the tab's own report prompt");
    }

    /// `claude -p` with the tab's own flags between the prompt and
    /// `--permission-prompts none`, which makes anything that would ask
    /// refuse instead. Default permission mode, as the tab: not plan mode,
    /// whose approval means implement, and not auto mode.
    #[test]
    fn an_unattended_run_is_the_tabs_claude_with_no_one_to_ask() {
        let argv = claude_unattended_argv("/refine-task u-1 auto", "{\"sandbox\":{}}", Path::new("/m/refine-mod"));
        assert_eq!(argv[..3], ["claude", "-p", "/refine-task u-1 auto"]);
        let flags = crate::session::refiner_flags("{\"sandbox\":{}}", Path::new("/m/refine-mod"));
        assert_eq!(argv[3..3 + flags.len()], flags[..]);
        assert_eq!(argv[3 + flags.len()..], ["--permission-prompts", "none"]);
        assert_eq!(argv.iter().filter(|a| *a == "--permission-mode").count(), 1);
        assert_eq!(CLAUDE, "claude");
    }

    /// The log goes beside the reviews, under the XDG data folder, named by
    /// the task and when the run started, so two runs on one task keep both.
    #[test]
    fn the_log_goes_beside_the_reviews_named_by_task_and_time() {
        let dirs = Dirs::at(Path::new("/home/x"));
        assert_eq!(unattended_dir(&dirs), PathBuf::from("/home/x/.local/share/niri-tasks/unattended"));
        assert_eq!(
            log_path(&dirs, "d9f76b94-e0ff-44df-85b4-060be4219169", 1_791_248_523),
            PathBuf::from("/home/x/.local/share/niri-tasks/unattended/d9f76b94-20261006T010203Z.log")
        );
        assert_eq!(
            unattended_dir(&dirs.with_data(Path::new("/data"))),
            PathBuf::from("/data/niri-tasks/unattended")
        );
    }

    /// A run's output lands in the log under its step's heading, and a
    /// non-zero exit is the step's failure, named. `sh` stands in for
    /// claude: `argv[..3]` is what the heading shows.
    #[test]
    fn a_run_logs_its_output_under_its_step_and_fails_on_a_bad_exit() {
        let dir = std::env::temp_dir().join(format!("niritasks-unattended-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("x.log");
        let ok = ["sh", "-c", "echo out; echo err >&2"].map(String::from);
        run_claude(&dir, &log, Step::Refine, &ok).unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        assert_eq!(text, "== refine: sh -c echo out; echo err >&2\nout\nerr\n");

        let bad = ["sh", "-c", "exit 3"].map(String::from);
        let err = run_claude(&dir, &log, Step::Report, &bad).unwrap_err().to_string();
        assert!(err.starts_with("The report step failed: claude exited with exit status: 3"), "{err}");
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("== report: sh -c exit 3\n"), "{text}");

        append_line(&log, "== failed: because");
        assert!(std::fs::read_to_string(&log).unwrap().ends_with("== failed: because\n"));
        std::fs::remove_dir_all(&dir).unwrap();
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
        // Its own skill: it reports on the plan, never refines.
        assert_eq!(prompt("u-1", Mode::Report), "/report-task u-1");
    }

    #[test]
    fn the_tab_says_what_it_is_for() {
        assert_eq!(tab_label(Mode::Quick, "fix the peek"), "Refine: fix the peek");
        assert_eq!(tab_label(Mode::Grill, "fix the peek"), "Grill: fix the peek");
        assert_eq!(tab_label(Mode::Report, "fix the peek"), "Report: fix the peek");
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
