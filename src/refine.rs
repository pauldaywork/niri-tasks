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

/// Which of a card's Refine button or its Go to list's Grill me or Generate
/// report opened Claude. They differ in the prompt [`prompt`] sends, and so
/// in the skill on the other end: the two refines share `refine-task`, which
/// reads the mode to decide whether to interview before drafting, while a
/// report runs `report-task` on the plan the task already holds. A report
/// session also tells the mod so, through [`session_settings`].
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
/// [`credential_paths`] — leaves nothing of that kind to send. Beyond the
/// classic credentials it covers app tokens (Codex, Vercel, t3) and the
/// browsers' profiles, which hold saved logins and session cookies.
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
    "~/.codex/**",
    "~/.local/share/com.vercel.cli/**",
    "~/.t3/**",
    "~/.mozilla/**",
    "~/snap/firefox/common/.mozilla/**",
    "~/.config/google-chrome/**",
    "~/.config/chromium/**",
];

/// Secrets found by pattern, not by a fixed path: the `.env` files of
/// every project under `~/Projects`, which hold API keys and database
/// URLs. Hidden from the Read tool by a deny rule and from sandboxed
/// Bash by `denyRead`, which takes globs on Linux (it does not for
/// writes). A refine loses nothing it needs: a project's `.env.example`
/// goes too, and the code that reads the variables still shows their names.
pub const SECRET_GLOBS: &[&str] = &["~/Projects/**/.env*"];

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

/// The file a `Report: <path>` note names. The mod always writes an absolute
/// path, which is used as written; a hand-written `~` or `~/...` is under the
/// home folder, and a relative path is in [`reports_dir`].
pub fn report_file(path: &str, dirs: &Dirs) -> PathBuf {
    if path == "~" {
        return dirs.home().to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return dirs.home().join(rest);
    }
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        reports_dir(dirs).join(p)
    }
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
/// Not the hooks under `task_data`, though: Taskwarrior runs them, hooks
/// are on, and the daemon's own `task` calls are not sandboxed, so a hook
/// written there would run as the user. And `$TMPDIR` is the fence's own
/// folder ([`session_tmp`]), not the temp folder the user's unsandboxed
/// Claude sessions share. It is not set here: the settings' `env` does not
/// reach Claude Code's temp folder (measured 2026-10-12), so
/// `CLAUDE_CODE_TMPDIR` is exported in the tab's shell before Claude starts
/// ([`Claude::Refiner`]'s `tmp`) and set on an unattended run's process.
///
/// Unix sockets are allowed because the sandbox's socket filter cannot load
/// under Ubuntu's bwrap AppArmor profile, which denies it the capability it
/// needs. `fence.hidden` makes up for it: the sockets that could run something
/// outside the sandbox — niri's spawn, the session bus, docker, herdr's panes —
/// are hidden from it instead, along with the credentials.
///
/// Allowed unasked: every Bash command (`Bash`, bare), since the sandbox
/// is the fence and the rules only say so. Auto-allow alone leaves
/// Claude Code's own command-shape checks in place (a variable, quoted
/// text, a `;` chain), and with no one to ask, as in an unattended run,
/// they deny (ADR 0002, 2026-10-12). Bare `Bash` opens no way out of the
/// sandbox: `allowUnsandboxedCommands` is false and there are no
/// `excludedCommands`. Also web search and fetch.
/// The Read tool may read anywhere (`Read(//**)`: `//` is Claude Code's
/// absolute form, one `/` would be the project's), as sandboxed Bash
/// already can. Deny rules win over it: the [`CREDENTIALS`], the
/// [`SECRET_GLOBS`], and every path `hidden` hides from sandboxed Bash,
/// so the tool opens nothing Bash could not read already.
/// The write itself is the refine mod's tool, which `pluginConfigs` tells
/// which task it may write: `uuid`, never anything the model says.
/// It is told `reports` too, the folder its report tool writes to, and
/// `report`: true in a Report session, where it offers only the report tool
/// and links the report from the task itself, and `unattended`: true in an
/// unattended run, where the write tool writes with no question and the
/// report tool opens nothing (see [`run_unattended`]).
/// `status_line`, when given, is the command for the task's status line
/// (`status_line::command`); a tab's refine shows it, an unattended run has
/// no screen to.
#[allow(clippy::too_many_arguments)]
pub fn session_settings(project: &Path, task_data: &Path, fence: &Fence, uuid: &str, reports: &Path, mode: Mode, unattended: bool, status_line: Option<&str>) -> String {
    let mut deny: Vec<String> = CREDENTIALS.iter().map(|c| format!("Read({c})")).collect();
    deny.extend(fence.hidden.iter().map(|p| format!("Read(//{}/**)", p.display().to_string().trim_start_matches('/'))));
    deny.extend(SECRET_GLOBS.iter().map(|g| format!("Read({g})")));
    let mut deny_read: Vec<String> = fence.hidden.iter().map(|p| p.display().to_string()).collect();
    deny_read.extend(SECRET_GLOBS.iter().map(|g| fence.home.join(g.trim_start_matches("~/")).display().to_string()));
    let allow = ["WebSearch", "WebFetch", "Bash", "Read(//**)"];
    let mut settings = serde_json::json!({
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
                "denyWrite": [project, task_data.join("hooks")],
                "allowWrite": [task_data],
                "denyRead": deny_read,
            },
        },
        "pluginConfigs": {
            (REFINE_MOD): { "options": { "uuid": uuid, "reports": reports, "report": (mode == Mode::Report), "unattended": unattended } },
        },
    });
    if let Some(command) = status_line {
        settings["statusLine"] = crate::status_line::setting(command);
    }
    settings.to_string()
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
/// sockets — the session's own temp folder is under the cache instead
/// ([`session_tmp`]) — `/var/snap` the snaps', and herdr keeps its own
/// under its config. `~/.cache/ibus` holds the input method's private bus,
/// which can type into other windows.
/// [`exposed_sockets`] checks the result against the live socket table.
fn hidden_paths(home: &Path) -> Vec<PathBuf> {
    [
        PathBuf::from("/run"),
        PathBuf::from("/tmp"),
        PathBuf::from("/var/snap"),
        home.join(".config/herdr"),
        home.join(".cache/ibus"),
    ]
    .into_iter()
    .filter(|p| p.exists())
    .chain(credential_paths(home))
    .collect()
}

/// What a refine session is fenced in by beyond its project: the home
/// its secrets are found under, the paths hidden from it ([`hidden_paths`])
/// and its own temp folder ([`session_tmp`]), made ready by
/// [`prepare_fence`] and written into the settings by [`session_settings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    pub home: PathBuf,
    pub hidden: Vec<PathBuf>,
    pub tmp: PathBuf,
}

/// The temp folder a refine of `uuid` gets as `$TMPDIR`, in place of the
/// `/tmp/claude-<uid>` the user's own, unsandboxed Claude sessions share:
/// a sandboxed command may write its temp folder, and one it shares is a
/// place to leave a trap for a process that is not sandboxed. Under the
/// cache, named by the task's first eight characters, kept short because
/// Claude Code falls back to the shared folder for a long one.
pub fn session_tmp(dirs: &Dirs, uuid: &str) -> PathBuf {
    dirs.cache().join("niri-tasks/refine-tmp").join(names::uuid8(uuid))
}

/// The fence for a refine of `uuid`, ready to start a session behind:
/// its folders made ([`make_fence_dirs`]), then refused if any socket is
/// left in reach.
fn prepare_fence(dirs: &Dirs, task_data: &Path, uuid: &str) -> Result<Fence> {
    let tmp = session_tmp(dirs, uuid);
    make_fence_dirs(task_data, &tmp)?;
    let hidden = hidden_paths(dirs.home());
    ensure_no_exposed_sockets(&hidden, &tmp)?;
    Ok(Fence { home: dirs.home().to_path_buf(), hidden, tmp })
}

/// Make the hooks folder under `task_data` and the session's temp folder
/// `tmp`, because the sandbox drops a path that does not exist: a dropped
/// `denyWrite` on the hooks would let the session make the folder and
/// write a hook into it.
fn make_fence_dirs(task_data: &Path, tmp: &Path) -> Result<()> {
    let hooks = task_data.join("hooks");
    std::fs::create_dir_all(&hooks).with_context(|| format!("could not make {}", hooks.display()))?;
    std::fs::create_dir_all(tmp).with_context(|| format!("could not make {}", tmp.display()))?;
    Ok(())
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

/// Refuse to start a refine while any socket on the machine would be left in
/// its sandbox's reach — so a new app's socket somewhere unexpected stops the
/// feature loudly instead of quietly becoming a way out.
///
/// Paths are resolved first, since a socket bound under `/var/run` is really
/// under `/run`; one that no longer exists cannot be connected to and is
/// skipped. `tmp` is the session's own temp folder, which the sandbox
/// keeps visible even under a hidden path.
fn ensure_no_exposed_sockets(hidden: &[PathBuf], tmp: &Path) -> Result<()> {
    let table = std::fs::read_to_string("/proc/net/unix").context("could not read /proc/net/unix")?;
    let sockets: Vec<PathBuf> = socket_paths(&table)
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .collect();
    let exposed = exposed_sockets(&sockets, hidden, &[tmp.to_path_buf()]);
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
/// back to the task's existing tab for `mode` if there is one: a refine for
/// Refine and Grill me, a report for Generate report.
///
/// The prompt is confirmed, and resent if Claude was still starting, as
/// Start working's is; a start Claude blocks on a question is waited out the
/// same way.
///
/// The task as found, not a uuid as typed: the session is found again by the
/// uuid's first eight characters, and a typed task number has none of them.
pub fn launch(ws: &Workspace, t: &task::Task, mode: Mode) -> Result<()> {
    let dirs = Dirs::from_env()?;

    // Before anything opens: a refused refine should leave nothing behind.
    let task_data = task::data_location()?;
    let fence = prepare_fence(&dirs, &task_data, &t.uuid)?;
    let mod_dir = refine_mod_dir(&dirs);
    // Without the mod the session would have no way to write its plan, and
    // the user would find that out only at the end of the interview.
    anyhow::ensure!(
        mod_dir.join(".claude-plugin/plugin.json").is_file(),
        "The refine mod is missing, or its link is broken, at {}. Run install.sh from the niri-tasks repo.",
        mod_dir.display()
    );
    let session = ws.session()?;
    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    let settings = session_settings(
        session.dir(),
        &task_data,
        &fence,
        &t.uuid,
        &reports_dir(&dirs),
        mode,
        false,
        Some(&crate::status_line::command(&exe, &t.uuid)),
    );

    let claude = Claude::Refiner { settings, mod_dir, tmp: fence.tmp.clone() };
    open_in(&session, t, ws.name(), &claude, mode)
}

/// The herdr agent `mode`'s session runs under: Refine and Grill me share
/// the task's refine agent, so either press lands in a refine already open;
/// a report has its own, since a refine tab cannot run one.
fn agent_name(mode: Mode, uuid: &str) -> String {
    match mode {
        Mode::Quick | Mode::Grill => names::refine_agent(uuid),
        Mode::Report => names::report_agent(uuid),
    }
}

/// What a press says when `mode`'s session was open on the task already.
fn already_open(mode: Mode) -> &'static str {
    match mode {
        Mode::Quick | Mode::Grill => "Already refining this task — switched to its tab.",
        Mode::Report => "Already reporting on this task — switched to its tab.",
    }
}

/// [`launch`]'s steps in `session` once `claude` is built: the agent, tab,
/// pane name and prompt for `mode` on `t`, handed to [`launch_in`], and a
/// notice when its session was open already. Kept apart from what reads the
/// machine so the fake can check that each mode lands in its own tab.
fn open_in(session: &Session, t: &task::Task, workspace_label: &str, claude: &Claude, mode: Mode) -> Result<()> {
    let name = agent_name(mode, &t.uuid);
    let label = tab_label(mode, &t.description);
    let role = match mode {
        Mode::Quick | Mode::Grill => names::Role::Refine,
        Mode::Report => names::Role::Report,
    };
    let pane = names::PaneName::for_task(role, t.id, &t.uuid, &t.description);
    if launch_in(session, &name, &label, workspace_label, claude, &pane, &prompt(&t.uuid, mode))? == Launched::AlreadyRunning {
        session.notify(already_open(mode));
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
///
/// Once Claude has started, its pane is labelled with the task (`pane_name`);
/// a Claude found running is left as it is.
fn launch_in(session: &Session, name: &str, label: &str, workspace_label: &str, claude: &Claude, pane_name: &names::PaneName, text: &str) -> Result<Launched> {
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
    session.label_pane(&tab.pane, pane_name);
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
/// gives; the settings already allow every sandboxed command and read.
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
    // card for good. Retried once, and a failure that stays is the run's
    // to report, not a line on stderr nobody reads.
    let cleared = task::set_processing(&t.uuid, false).or_else(|_| task::set_processing(&t.uuid, false));
    if let Err(e) = &cleared {
        append_line(&log, &format!("== could not clear +{}: {e:#}", task::PROCESSING_TAG));
    }
    match settle(outcome, cleared, t) {
        Ok(message) => {
            notify::tasks(&message);
            Ok(())
        }
        Err(e) => {
            append_line(&log, &format!("== failed: {e:#}"));
            bail!("{}", with_log(&format!("{e:#}"), &log))
        }
    }
}

/// `message` followed by the log's path as a sentence of its own, whether or
/// not `message` ends with a full stop.
fn with_log(message: &str, log: &Path) -> String {
    let stop = if message.ends_with('.') { "" } else { "." };
    format!("{message}{stop} Log: {}", log.display())
}

/// What [`run_unattended`] tells the user, from how the steps ended and
/// whether `+processing` came off: the notification text when both went
/// well, otherwise the error. A tag that stayed on is an error even after
/// two good runs, for the card is locked until it is cleared by hand.
fn settle(steps: Result<()>, cleared: Result<()>, t: &task::Task) -> Result<String> {
    let Err(clear) = cleared else {
        return steps.map(|()| format!("{PLANNED_AND_REPORTED}: {}", t.description));
    };
    let ran = match steps {
        Ok(()) => "Both runs finished".to_string(),
        Err(e) => format!("{e:#}"),
    };
    let ran = ran.trim_end_matches('.');
    bail!(
        "{ran}, and +{} could not be cleared ({clear:#}): clear it with `task {} modify -{}`.",
        task::PROCESSING_TAG,
        names::uuid8(&t.uuid),
        task::PROCESSING_TAG,
    )
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
    let task_data = task::data_location()?;
    let fence = prepare_fence(dirs, &task_data, &t.uuid)?;
    let mod_dir = refine_mod_dir(dirs);
    anyhow::ensure!(
        mod_dir.join(".claude-plugin/plugin.json").is_file(),
        "The refine mod is missing, or its link is broken, at {}. Run install.sh from the niri-tasks repo.",
        mod_dir.display()
    );
    anyhow::ensure!(programs::on_path(CLAUDE), "Claude Code (`{CLAUDE}`) is not on PATH, so nothing can plan the task.");
    let session = ws.session()?;
    let project = session.dir();
    let reports = reports_dir(dirs);
    let [refine_prompt, report_prompt] = unattended_prompts(&t.uuid);

    // What the task held before the runs, so a read-back that only finds
    // what was already there does not pass for a run that wrote it.
    let before = task::get(&t.uuid)?.context("The task is gone before the refine step.")?;
    let was_planned = before.is_planned();
    let notes_before = note_texts(&before);
    let report_before = before.report_path().map(String::from);

    let settings = session_settings(project, &task_data, &fence, &t.uuid, &reports, Mode::Quick, true, None);
    run_claude(project, log, Step::Refine, &claude_unattended_argv(&refine_prompt, &settings, &mod_dir), &fence.tmp)?;
    let planned = task::get(&t.uuid)?.context("The task is gone after the refine step.")?;
    refine_wrote(was_planned, &notes_before, planned.is_planned(), &note_texts(&planned))?;

    let settings = session_settings(project, &task_data, &fence, &t.uuid, &reports, Mode::Report, true, None);
    run_claude(project, log, Step::Report, &claude_unattended_argv(&report_prompt, &settings, &mod_dir), &fence.tmp)?;
    let reported = task::get(&t.uuid)?.context("The task is gone after the report step.")?;
    let path = report_wrote(report_before.as_deref(), reported.report_path())?;
    anyhow::ensure!(Path::new(path).is_file(), "The report step linked {path}, which is not there.");
    Ok(())
}

/// The text of each of the task's notes, in order.
fn note_texts(t: &task::Task) -> Vec<String> {
    t.annotations.iter().map(|a| a.description.clone()).collect()
}

/// Whether the refine step left evidence of its own write: the task is
/// planned, and if it already was before the run, its notes changed too,
/// since the plan is written as notes.
fn refine_wrote(was_planned: bool, notes_before: &[String], planned: bool, notes_after: &[String]) -> Result<()> {
    anyhow::ensure!(planned, "The refine step wrote no plan: the task is not tagged planned.");
    anyhow::ensure!(
        !was_planned || notes_before != notes_after,
        "The refine step wrote no plan: the task was planned already and its notes are unchanged."
    );
    Ok(())
}

/// The report path the report step linked, if it linked one: a `Report:`
/// note whose path differs from the one the task had before the run.
fn report_wrote<'a>(before: Option<&str>, after: Option<&'a str>) -> Result<&'a str> {
    let path = after.context("The report step linked no report: the task has no Report: note.")?;
    anyhow::ensure!(
        before != Some(path),
        "The report step linked no new report: the task's Report: note is still {path}."
    );
    Ok(path)
}

/// One `claude -p` run in `project`, both its streams appended to `log`
/// under a line naming the step and the prompt (not the settings, which are
/// long and the same for both). Stdin is closed: there is no one typing.
/// `tmp` is the session's own temp folder ([`session_tmp`]), set as
/// `CLAUDE_CODE_TMPDIR`, which is the only way it reaches Claude Code.
fn run_claude(project: &Path, log: &Path, step: Step, argv: &[String], tmp: &Path) -> Result<()> {
    append_line(log, &format!("== {}: {}", step.name(), argv[..3].join(" ")));
    let out = File::options().append(true).create(true).open(log).with_context(|| format!("could not open {}", log.display()))?;
    let err = out.try_clone().context("could not share the log between stdout and stderr")?;
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(project)
        .env("CLAUDE_CODE_TMPDIR", tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .with_context(|| format!("could not run {} for the {} step", argv[0], step.name()))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn tk() -> task::Task {
        serde_json::from_str(r#"{"uuid":"7cd9fd3a-d27b-4387-8249-aaf0d6785f90","description":"fix: a thing"}"#).unwrap()
    }

    fn failing<T>(msg: &str) -> Result<T> {
        Err(anyhow::anyhow!("{msg}"))
    }

    /// The notification comes only when both runs went well and the tag
    /// came off; a tag that stayed is an error naming how to clear it and
    /// the step that failed, if one did.
    #[test]
    fn settle_announces_success_only_when_the_tag_came_off() {
        let t = tk();
        assert_eq!(settle(Ok(()), Ok(()), &t).unwrap(), "Planned and reported: fix: a thing");
        let e = settle(failing("The report step failed: claude exited with 1."), Ok(()), &t).unwrap_err();
        assert_eq!(format!("{e:#}"), "The report step failed: claude exited with 1.");
        let e = format!("{:#}", settle(Ok(()), failing("db locked"), &t).unwrap_err());
        assert!(e.starts_with("Both runs finished, and +processing could not be cleared"), "{e}");
        assert!(e.contains("task 7cd9fd3a modify -processing"), "{e}");
        let e = format!("{:#}", settle(failing("The refine step failed: x."), failing("db locked"), &t).unwrap_err());
        assert!(e.starts_with("The refine step failed: x, and +processing"), "{e}");
        assert!(!e.contains(".,"), "{e}");
    }

    /// The log is a sentence of its own after any message.
    #[test]
    fn with_log_ends_the_message_before_the_log() {
        let log = Path::new("/l/x.log");
        assert_eq!(with_log("It failed.", log), "It failed. Log: /l/x.log");
        assert_eq!(with_log("could not run claude: no such file", log), "could not run claude: no such file. Log: /l/x.log");
    }

    /// A plan already there before the run does not count as the run's.
    #[test]
    fn refine_must_have_written_something() {
        let a = vec!["plan v1".to_string()];
        let b = vec!["plan v2".to_string()];
        assert!(refine_wrote(false, &[], true, &a).is_ok());
        assert!(refine_wrote(true, &a, true, &b).is_ok());
        assert!(refine_wrote(false, &[], false, &[]).is_err());
        assert!(refine_wrote(true, &a, true, &a).is_err(), "planned before, nothing changed");
    }

    /// A report note already there before the run does not count as the run's.
    #[test]
    fn report_must_be_a_new_path() {
        assert_eq!(report_wrote(None, Some("/r/a.html")).unwrap(), "/r/a.html");
        assert_eq!(report_wrote(Some("/r/a.html"), Some("/r/b.html")).unwrap(), "/r/b.html");
        assert!(report_wrote(None, None).is_err());
        assert!(report_wrote(Some("/r/a.html"), None).is_err());
        assert!(report_wrote(Some("/r/a.html"), Some("/r/a.html")).is_err());
    }

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

    /// A tab's refine shows the task in its status line; an unattended run
    /// has no screen to show it on.
    #[test]
    fn the_status_line_rides_in_the_settings_only_when_given() {
        let with = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), "u", Path::new("/r"), Mode::Quick, false, Some("niritasks task status-line u"));
        let v: Value = serde_json::from_str(&with).unwrap();
        assert_eq!(v["statusLine"]["command"], "niritasks task status-line u");
        assert_eq!(v["sandbox"]["enabled"], true, "the fence is still there");
        let without = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), "u", Path::new("/r"), Mode::Quick, true, None);
        let v: Value = serde_json::from_str(&without).unwrap();
        assert!(v.get("statusLine").is_none(), "{v}");
    }

    /// A fence for the settings tests: these hidden paths and a fixed
    /// private temp folder.
    fn fence(hidden: &[PathBuf]) -> Fence {
        Fence { home: PathBuf::from("/home/x"), hidden: hidden.to_vec(), tmp: PathBuf::from("/home/x/.cache/niri-tasks/refine-tmp/d9f76b94") }
    }

    /// Each task's session gets its own temp folder under the cache, short
    /// and named by the task, so nothing unsandboxed shares it.
    #[test]
    fn a_session_gets_a_temp_folder_of_its_own() {
        let dirs = Dirs::at(Path::new("/home/x"));
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        assert_eq!(session_tmp(&dirs, u), PathBuf::from("/home/x/.cache/niri-tasks/refine-tmp/d9f76b94"));
        assert_eq!(
            session_tmp(&dirs.with_cache(Path::new("/c")), u),
            PathBuf::from("/c/niri-tasks/refine-tmp/d9f76b94")
        );
    }

    /// The fence makes the folders it denies or hands out, since the
    /// sandbox drops an entry whose path does not exist, and a `denyWrite`
    /// that was dropped would let the session create the hooks folder and
    /// write a hook into it. The socket check `prepare_fence` runs after
    /// has tests of its own, and would read this machine's sockets here.
    #[test]
    fn preparing_the_fence_makes_the_hooks_and_temp_folders() {
        let root = std::env::temp_dir().join(format!("niritasks-fence-{}", std::process::id()));
        let task_data = root.join("task");
        std::fs::create_dir_all(&task_data).unwrap();
        let dirs = Dirs::at(&root.join("home"));
        let tmp = session_tmp(&dirs, "d9f76b94-e0ff-44df-85b4-060be4219169");
        make_fence_dirs(&task_data, &tmp).unwrap();
        assert!(task_data.join("hooks").is_dir());
        assert!(tmp.is_dir());
        make_fence_dirs(&task_data, &tmp).expect("ready already is fine");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// The fence the refine session runs in: every command sandboxed with no
    /// way out, the project read-only, the task database writable, and the
    /// sockets that could start a process outside the sandbox hidden.
    #[test]
    fn the_sandbox_fences_the_session_to_the_task_database() {
        let hidden = [PathBuf::from("/run/user/1000"), PathBuf::from("/run/docker.sock")];
        let json = session_settings(Path::new("/home/x/Projects/alpha"), Path::new("/home/x/.task"), &fence(&hidden), "u", Path::new("/r"), Mode::Quick, false, None);
        let v: Value = serde_json::from_str(&json).unwrap();
        let sb = &v["sandbox"];
        assert_eq!(sb["enabled"], true);
        assert_eq!(sb["autoAllowBashIfSandboxed"], true);
        assert_eq!(sb["allowUnsandboxedCommands"], false, "no retrying outside the sandbox");
        assert_eq!(sb["failIfUnavailable"], true, "never run unfenced");
        assert_eq!(
            sb["filesystem"]["denyWrite"],
            serde_json::json!(["/home/x/Projects/alpha", "/home/x/.task/hooks"]),
            "the project, and the hooks the user's own task commands would run"
        );
        assert_eq!(sb["filesystem"]["allowWrite"], serde_json::json!(["/home/x/.task"]));
        let deny_read = sb["filesystem"]["denyRead"].as_array().unwrap();
        assert!(deny_read.starts_with(&[serde_json::json!("/run/user/1000"), serde_json::json!("/run/docker.sock")]));
        assert_eq!(deny_read.len(), 2 + SECRET_GLOBS.len(), "the hidden paths, then the secret globs");
        assert_eq!(sb["network"]["allowAllUnixSockets"], true);
    }

    /// Every shell command runs unasked, of any shape: a bare `Bash` rule,
    /// because the sandbox is the fence, not the rules. Claude Code's
    /// command-shape checks otherwise deny loops and `$( )` when there is
    /// no one to ask, even with auto-allow (ADR 0002, 2026-10-12). No
    /// scoped `Bash(…)` rule is left beside it. Web search and fetch, and
    /// the Read tool anywhere (`//` is the absolute form), as sandboxed
    /// Bash can. Deny rules win over the allows: secrets, and every path
    /// the sandbox hides from Bash.
    #[test]
    fn the_session_may_run_any_sandboxed_command_but_not_read_secrets() {
        let hidden = [PathBuf::from("/run"), PathBuf::from("/home/x/.config/herdr")];
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&hidden), "u", Path::new("/r"), Mode::Quick, false, None);
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        for tool in ["WebSearch", "WebFetch", "Bash", "Read(//**)"] {
            assert!(allow.iter().any(|a| a == tool), "{tool} allowed");
        }
        assert_eq!(allow.len(), 4, "nothing else runs unasked");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.starts_with("Bash("))), "no scoped Bash rule beside the bare one");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.starts_with("WebFetch(domain:"))), "hosts come from the sandbox's own list");
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("python3"))));
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("skills"))), "one rule for every folder, not a list");
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.iter().any(|d| d == "Read(~/.ssh/**)"));
        assert!(deny.iter().any(|d| d == "Read(~/.claude/.credentials.json)"));
        assert!(deny.iter().any(|d| d == "Read(//run/**)"));
        assert!(deny.iter().any(|d| d == "Read(//home/x/.config/herdr/**)"));
        assert_eq!(deny.len(), CREDENTIALS.len() + SECRET_GLOBS.len() + hidden.len());
    }

    /// Secret stores beyond the classic credentials are hidden too: the
    /// Codex and Vercel CLIs' tokens, t3's keys, the browsers' profiles
    /// (saved logins and cookies), and project `.env` files. Reviews on
    /// 2026-10-12 found each readable from a refine.
    #[test]
    fn app_tokens_browser_profiles_and_env_files_are_secrets() {
        for c in [
            "~/.codex/**",
            "~/.local/share/com.vercel.cli/**",
            "~/.t3/**",
            "~/.mozilla/**",
            "~/snap/firefox/common/.mozilla/**",
            "~/.config/google-chrome/**",
            "~/.config/chromium/**",
        ] {
            assert!(CREDENTIALS.contains(&c), "{c}");
        }
        assert_eq!(SECRET_GLOBS, ["~/Projects/**/.env*"]);
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), "u", Path::new("/r"), Mode::Quick, false, None);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert!(v["permissions"]["deny"].as_array().unwrap().iter().any(|d| d == "Read(~/Projects/**/.env*)"));
        let deny_read = v["sandbox"]["filesystem"]["denyRead"].as_array().unwrap();
        assert!(deny_read.iter().any(|d| d.as_str().is_some_and(|s| s.ends_with("/Projects/**/.env*") && s.starts_with('/'))), "{deny_read:?}");
    }

    /// The task the mod may write, and where it may write a report, come from
    /// these settings, never from the model.
    #[test]
    fn the_mod_is_told_which_task_it_may_write_and_where_reports_go() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), u, Path::new("/home/x/.local/share/niri-tasks/reviews"), Mode::Quick, false, None);
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
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), u, Path::new("/r"), Mode::Report, false, None);
        let v: Value = serde_json::from_str(&json).unwrap();
        let options = &v["pluginConfigs"][REFINE_MOD]["options"];
        assert_eq!(options["report"], true);
        assert_eq!(options["uuid"], u, "the same fence and task as a refine");
        assert_eq!(options["reports"], "/r");
        for mode in [Mode::Quick, Mode::Grill] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), u, Path::new("/r"), mode, false, None);
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
            let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), u, Path::new("/r"), mode, true, None);
            let v: Value = serde_json::from_str(&json).unwrap();
            let options = &v["pluginConfigs"][REFINE_MOD]["options"];
            assert_eq!(options["unattended"], true, "{mode:?}");
            assert_eq!(options["report"], mode == Mode::Report, "{mode:?}");
            assert_eq!(options["uuid"], u, "the same fence and task");
        }
        for mode in [Mode::Quick, Mode::Grill, Mode::Report] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), u, Path::new("/r"), mode, false, None);
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
        let tmp = Path::new("/home/x/.cache/niri-tasks/refine-tmp/d9f76b94");
        let ok = ["sh", "-c", "echo out; echo err >&2"].map(String::from);
        run_claude(&dir, &log, Step::Refine, &ok, tmp).unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        assert_eq!(text, "== refine: sh -c echo out; echo err >&2\nout\nerr\n");

        let bad = ["sh", "-c", "exit 3"].map(String::from);
        let err = run_claude(&dir, &log, Step::Report, &bad, tmp).unwrap_err().to_string();
        assert!(err.starts_with("The report step failed: claude exited with exit status: 3"), "{err}");
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("== report: sh -c exit 3\n"), "{text}");

        // The session's own temp folder reaches Claude through its
        // environment, since the settings' `env` does not move it.
        let echo = ["sh", "-c", "echo \"$CLAUDE_CODE_TMPDIR\""].map(String::from);
        run_claude(&dir, &log, Step::Refine, &echo, tmp).unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("\n/home/x/.cache/niri-tasks/refine-tmp/d9f76b94\n"), "{text}");

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

    /// A report note's path as typed: `~` is the home folder, a relative
    /// path is in the reviews folder, and an absolute one is as written.
    #[test]
    fn a_report_note_is_found_from_home_the_reviews_folder_or_as_written() {
        let dirs = Dirs::at(Path::new("/home/x"));
        assert_eq!(report_file("~", &dirs), PathBuf::from("/home/x"));
        assert_eq!(
            report_file("~/.local/share/niri-tasks/reviews/r.html", &dirs),
            PathBuf::from("/home/x/.local/share/niri-tasks/reviews/r.html")
        );
        assert_eq!(report_file("r.html", &dirs), PathBuf::from("/home/x/.local/share/niri-tasks/reviews/r.html"));
        assert_eq!(report_file("old/r.html", &dirs), PathBuf::from("/home/x/.local/share/niri-tasks/reviews/old/r.html"));
        assert_eq!(
            report_file("r.html", &dirs.clone().with_data(Path::new("/data"))),
            PathBuf::from("/data/niri-tasks/reviews/r.html")
        );
        assert_eq!(report_file("/r/r.html", &dirs), PathBuf::from("/r/r.html"));
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
        Claude::Refiner { settings: "{}".into(), mod_dir: PathBuf::from("/m"), tmp: PathBuf::from("/c/tmp") }
    }

    /// Refine's session steps over `fake`, as `launch` runs them.
    fn refine(fake: &Fake) -> Result<Launched> {
        let session = Session::with_port("alpha", PathBuf::from("/p/alpha"), Box::new(fake.clone()));
        let pane = crate::names::PaneName::for_task(crate::names::Role::Refine, 61, "abc", "x");
        launch_in(&session, "task-abc", "Refine: x", "alpha", &refiner(), &pane, "/refine-task abc")
    }

    /// Where `line` first appears in the log, failing the test when it is
    /// not there at all.
    fn at(log: &[String], line: &str) -> usize {
        log.iter().position(|l| l.starts_with(line)).unwrap_or_else(|| panic!("no {line:?} in {log:?}"))
    }

    /// The new Claude's pane says which task it is on, once Claude has
    /// started in it and before the prompt; one found running is left as
    /// it is.
    #[test]
    fn refine_labels_its_new_pane_and_only_that() {
        let fake = Fake::running(&[("w1", "alpha")]);
        refine(&fake).unwrap();
        let log = fake.log();
        assert!(at(&log, "agent_start alpha task-abc") < at(&log, "pane_rename alpha"), "{log:?}");
        assert!(at(&log, "pane_rename alpha") < at(&log, "agent_prompt alpha task-abc"), "{log:?}");
        assert!(log.iter().any(|l| l.ends_with(" Refine #61 · x")), "{log:?}");
        refine(&fake).unwrap();
        assert_eq!(fake.log().iter().filter(|l| l.starts_with("pane_rename")).count(), 1);
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
        assert_eq!(
            at(&log, "pane_run alpha p1 export CLAUDE_CODE_TMPDIR='/c/tmp'") + 1,
            at(&log, "agent_start alpha task-abc"),
            "the temp folder is set in the pane's shell just before Claude starts in it: {log:?}"
        );
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

    /// `launch`'s session steps for `mode` on [`tk`] over `fake`.
    fn open(fake: &Fake, mode: Mode) -> Result<()> {
        let session = Session::with_port("alpha", PathBuf::from("/p/alpha"), Box::new(fake.clone()));
        open_in(&session, &tk(), "alpha", &refiner(), mode)
    }

    #[test]
    fn each_mode_has_its_agent() {
        assert_eq!(agent_name(Mode::Quick, "7CD9FD3A-d27b"), "task-7cd9fd3a");
        assert_eq!(agent_name(Mode::Grill, "7CD9FD3A-d27b"), "task-7cd9fd3a");
        assert_eq!(agent_name(Mode::Report, "7CD9FD3A-d27b"), "report-7cd9fd3a");
    }

    /// Generate report with a refine tab still open on the task: the refine
    /// tab was started without the report option and may be mid-interview,
    /// so the report starts a Claude of its own, prompted, and the refine is
    /// left alone.
    #[test]
    fn a_report_opens_its_own_tab_beside_a_refine() {
        let fake = Fake::running(&[("w1", "alpha")]);
        open(&fake, Mode::Quick).unwrap();
        open(&fake, Mode::Report).unwrap();
        let log = fake.log();
        assert!(!log.iter().any(|l| l.starts_with("agent_focus alpha task-7cd9fd3a")), "{log:?}");
        assert!(log.iter().any(|l| l.starts_with("agent_start alpha report-7cd9fd3a ")), "{log:?}");
        assert!(
            log.iter().any(|l| l == "agent_prompt alpha report-7cd9fd3a /report-task 7cd9fd3a-d27b-4387-8249-aaf0d6785f90"),
            "{log:?}"
        );
        assert!(log.iter().any(|l| l.starts_with("pane_rename ") && l.ends_with(" Report · fix: a thing")), "{log:?}");
        assert!(!log.iter().any(|l| l.starts_with("notify ")), "{log:?}");
        assert_eq!(fake.agents().len(), 2);
    }

    /// A second press of each goes back to its own tab, and says so in its
    /// own words; nothing new starts.
    #[test]
    fn a_second_press_goes_back_to_its_own_tab() {
        let fake = Fake::running(&[("w1", "alpha")]);
        open(&fake, Mode::Quick).unwrap();
        open(&fake, Mode::Report).unwrap();
        let before = fake.log().len();
        open(&fake, Mode::Report).unwrap();
        open(&fake, Mode::Grill).unwrap();
        let log: Vec<String> = fake.log().into_iter().skip(before).collect();
        assert!(!log.iter().any(|l| l.starts_with("agent_start ")), "{log:?}");
        assert!(at(&log, "agent_focus alpha report-7cd9fd3a") < at(&log, "agent_focus alpha task-7cd9fd3a"), "{log:?}");
        assert!(log.iter().any(|l| *l == format!("notify {}", already_open(Mode::Report))), "{log:?}");
        assert!(log.iter().any(|l| *l == format!("notify {}", already_open(Mode::Grill))), "{log:?}");
        assert_ne!(already_open(Mode::Report), already_open(Mode::Quick));
        assert_eq!(fake.agents().len(), 2);
    }
}
