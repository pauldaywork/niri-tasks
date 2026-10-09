//! Which herdr session a workspace's project terminal attaches to, and
//! opening that session (in the folder [`Dirs::start_dir`] picks) and its
//! agents.
//!
//! Two rules, and the inverse of each, because `niritasks tag --session` has
//! to get from a terminal back to the workspace it was opened for. The
//! lookup itself is `Workspace::of_session`'s, in the `workspace` module;
//! the inverses below are what it is made of:
//!
//! * **Folder**, defined in `dirs`: a workspace named "hansard-votes" works in
//!   `~/Projects/hansard-votes` ([`Dirs::start_dir`]). Inverse:
//!   [`Dirs::project_from_cwd`].
//! * **Session**: its project terminal runs `herdr --session <name>`, with the
//!   name made safe for herdr ([`herdr_session_name`]). Inverse:
//!   [`workspace_for_session`], fed by [`session_from_env`].
//!
//! The folder uses the *raw* workspace name and the session the sanitised one.
//! A workspace called "my project" looks in `~/Projects/my project` while
//! running in session `my_project`.
//!
//! Opening a session and its agents are steps on a [`Session`], which go
//! through `Port`, everything the module does outside itself, so the steps
//! can be tested against a fake as well as run for real through
//! `herdr::Process`.
//!
//! [`Dirs::start_dir`]: crate::dirs::Dirs::start_dir
//! [`Dirs::project_from_cwd`]: crate::dirs::Dirs::project_from_cwd

mod herdr;
#[cfg(test)]
pub(crate) mod fake;

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// herdr refuses session names longer than this (herdr `src/session.rs`,
/// `MAX_SESSION_NAME_LEN`).
pub const SESSION_NAME_MAX: usize = 64;

/// The herdr session a workspace's project terminal attaches to.
///
/// herdr only accepts `[A-Za-z0-9._-]`, at most 64 bytes, and not `.` or `..`;
/// anything else makes `herdr --session` exit 2 before it draws a thing. And
/// `default` is not a name at all to herdr — it means the unnamed default
/// session. So each disallowed character becomes `_` (per character, not
/// run-collapsing, case kept), the result is cut to 64, and the three names
/// herdr treats specially get a `ws-` prefix.
pub fn herdr_session_name(workspace: &str) -> String {
    let mut name: String = workspace
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Every char is ASCII by now, so truncating at a byte count cannot split one.
    name.truncate(SESSION_NAME_MAX);
    if matches!(name.as_str(), "" | "." | ".." | "default") {
        name = format!("ws-{name}");
    }
    name
}

/// Which of `workspaces` this herdr session was opened for.
///
/// The session name is a *lossy* rendering of the workspace name — spaces and
/// slashes both became underscores — so it cannot simply be unfolded. Matching
/// each live workspace name through the same rule instead recovers the
/// original exactly, and answers `None` when the workspace it named is gone or
/// has since been renamed.
pub fn workspace_for_session<'a>(session: &str, workspaces: &'a [String]) -> Option<&'a String> {
    workspaces
        .iter()
        .find(|name| herdr_session_name(name) == session)
}

/// The herdr session this process is running in, from the environment herdr
/// gives its panes.
///
/// `HERDR_SESSION` carries the name. herdr's client sets it and its server,
/// and so every pane, inherits it — but only for a named session: the default
/// session removes it, and there is no name to recover then. `HERDR_SOCKET_PATH`
/// is the backup, `<config>/sessions/<name>/herdr.sock` for a named session and
/// `<config>/herdr.sock` for the default one. Passed in rather than read here
/// so the tests need not touch the real environment.
pub fn session_from_env(herdr_session: Option<&str>, socket_path: Option<&str>) -> Option<String> {
    if let Some(name) = herdr_session.filter(|s| !s.is_empty()) {
        return Some(name.to_string());
    }
    let dir = Path::new(socket_path?).parent()?;
    let is_named = dir.parent()?.file_name()? == "sessions";
    is_named.then(|| dir.file_name()?.to_str().map(str::to_string)).flatten()
}

/// A failure herdr itself reported: its JSON error's code and message. The
/// message is what the user sees; the code is what a caller acts on
/// (`agent_not_found`, `agent_prompt_stalled`, `timeout`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HerdrError {
    /// herdr's error code, when its stderr was its JSON and carried one.
    pub code: Option<String>,
    /// herdr's own message, or its raw stderr when that was not its JSON.
    pub message: String,
}

impl std::fmt::Display for HerdrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HerdrError {}

/// What a herdr call comes back with: herdr's own refusal as `Err(HerdrError)`
/// inside an `Ok`, and "could not run herdr at all", or a herdr answer that
/// lacks an id the caller needs, as the outer `Err`, so neither is ever
/// mistaken for herdr saying "no".
pub(crate) type HerdrResult<T> = Result<std::result::Result<T, HerdrError>>;

/// One of a session's herdr workspaces, as `workspace list` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// The id other herdr calls take, such as `tab create --workspace`.
    pub id: String,
    /// The label the user sees, which a terminal's window title shows.
    pub label: String,
}

/// What `tab create` or `workspace create` made: the pane to start Claude in,
/// and the tab, when herdr said which, to close again if that fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    /// The new tab's root pane.
    pub pane: String,
    /// The new tab, when herdr's answer named it.
    pub tab: Option<String>,
}

/// What `worktree open` opened, or found open: the workspace, and its root
/// pane, when herdr reports one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// The worktree's herdr workspace.
    pub workspace: String,
    /// That workspace's root pane. herdr's answer for a worktree that was
    /// already open need not carry one, so its absence is no failure here;
    /// the caller that needs the pane says so.
    pub pane: Option<String>,
}

/// An agent as `agent get` reports it: its name, None for one nobody named,
/// and its status (`idle`, `working`, `blocked`, `done`, `unknown`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    /// The agent's name, None for a Claude started by hand.
    pub name: Option<String>,
    /// The agent's lifecycle state, as herdr words it.
    pub status: Option<String>,
}

/// A niri window's id, app id and title: what finding the session's terminal
/// window needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// niri's id for the window, which focusing it takes.
    pub id: u64,
    /// The window's app id, when it set one.
    pub app_id: Option<String>,
    /// The window's title, when it set one.
    pub title: Option<String>,
}

/// Which Claude `agent start` runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claude {
    /// Refine's: fenced by the settings JSON and the refine mod it loads.
    Refiner {
        /// The settings JSON passed to `claude --settings`.
        settings: String,
        /// The folder of the refine mod Claude loads.
        mod_dir: PathBuf,
    },
    /// Start working's: the user's own defaults, since it is meant to do the work.
    Worker,
}

/// A hold on a session's launches, taken by [`Session::claim`], or on a
/// task's setup, taken by [`Session::hold_setup`]; given up when dropped.
/// Whoever holds a session's claim is the one launch checking the session
/// for its agent and starting it, so a second press of Refine or Start
/// working waits, then finds the first one's agent instead of starting
/// another.
#[must_use = "the claim is given up as soon as it is dropped"]
pub struct Claim {
    // Whatever the adapter holds the claim by: an open, locked file for the
    // process adapter, a logger of the release for the fake. Dropping it is
    // what gives the claim up.
    _held: Box<dyn std::any::Any>,
}

impl Claim {
    /// A claim held for as long as `held` lives.
    pub(crate) fn new(held: impl std::any::Any) -> Claim {
        Claim { _held: Box::new(held) }
    }
}

/// Everything the session module does outside itself: herdr, every call
/// naming the session; niri's window list, focus and spawn; time; the user's
/// notifications; and the claim that keeps two launches in one session from
/// both starting the same agent. Two adapters make the seam real: [`herdr::Process`]
/// and, in tests, the fake.
pub(crate) trait Port {
    fn workspace_list(&self, session: &str) -> HerdrResult<Vec<Workspace>>;
    fn workspace_create(&self, session: &str, dir: &Path, label: &str) -> HerdrResult<Created>;
    fn tab_create(&self, session: &str, workspace: &str, dir: &Path, label: &str) -> HerdrResult<Created>;
    fn tab_close(&self, session: &str, tab: &str) -> HerdrResult<()>;
    fn worktree_open(&self, session: &str, repo: &Path, path: &Path, label: &str) -> HerdrResult<Opened>;
    fn pane_run(&self, session: &str, pane: &str, command: &str) -> HerdrResult<()>;
    fn agent_get(&self, session: &str, target: &str) -> HerdrResult<Agent>;
    fn agent_list(&self, session: &str) -> HerdrResult<Vec<String>>;
    fn agent_focus(&self, session: &str, name: &str) -> HerdrResult<()>;
    fn agent_rename(&self, session: &str, target: &str, name: &str) -> HerdrResult<()>;
    fn agent_start(&self, session: &str, name: &str, pane: &str, claude: &Claude) -> HerdrResult<()>;
    fn agent_wait_ready(&self, session: &str, name: &str) -> HerdrResult<()>;
    /// Send a prompt and wait for herdr to see Claude start on it, failing
    /// with `agent_prompt_stalled` when it does not.
    fn agent_prompt(&self, session: &str, name: &str, text: &str) -> HerdrResult<()>;
    fn windows(&self) -> Result<Vec<WindowInfo>>;
    fn focus_window(&self, id: u64) -> Result<()>;
    fn spawn(&self, command: Vec<String>) -> Result<()>;
    fn sleep(&self, d: Duration);
    fn notify(&self, text: &str);
    /// Whether herdr is on `$PATH` at all, for the one place that starts a
    /// terminal to run it.
    fn herdr_installed(&self) -> bool;
    /// Hold the session against every other launch in it until the returned
    /// claim is dropped, waiting for one already held; an error when it is
    /// still held after a bounded wait.
    fn claim(&self, session: &str) -> Result<Claim>;
    /// Try `task`'s setup lock, which a setup tab holds for its whole life,
    /// for up to `wait`: the lock, or None when another holds it still. A
    /// zero `wait` tries once.
    fn setup_lock(&self, task: &str, wait: Duration) -> Result<Option<Claim>>;
}

/// How long a just-opened project terminal gets to bring its herdr session up.
const SESSION_WAIT: Duration = Duration::from_secs(10);
/// How often the session is asked whether it is up yet.
const SESSION_POLL: Duration = Duration::from_millis(250);
/// How many times a stalled prompt is sent before giving up.
const PROMPT_TRIES: u32 = 4;
/// How long a setup tab waits for its task's setup lock: a launch looking to
/// see whether it is held takes it for a moment, so a tab starting just then
/// must not fail. Far longer than a look, far shorter than any real setup.
const SETUP_TAKE_WAIT: Duration = Duration::from_secs(2);
/// The pause before a stalled prompt is sent again.
const PROMPT_RETRY: Duration = Duration::from_secs(2);

/// What a confirmed prompt's result means for sending it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptOutcome {
    /// Claude took it.
    Delivered,
    /// Claude never started on it, so sending it again is safe.
    Resend,
    /// herdr refused it for some other reason.
    Failed,
}

/// Read a `--wait` prompt's outcome from herdr's error code (`None` when it
/// succeeded): still working past the timeout means it arrived; a stall means
/// Claude never started on it, which is safe to resend; anything else fails.
fn prompt_outcome(code: Option<&str>) -> PromptOutcome {
    match code {
        None | Some("timeout") => PromptOutcome::Delivered,
        Some("agent_prompt_stalled") => PromptOutcome::Resend,
        Some(_) => PromptOutcome::Failed,
    }
}

/// The herdr pane this process runs in, from what herdr hands its panes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentPane {
    /// The named session, from `HERDR_SESSION`, else the session directory of
    /// `HERDR_SOCKET_PATH` (see [`session_from_env`]).
    pub session: String,
    /// The pane, from `HERDR_PANE_ID`, when herdr set it.
    pub pane: Option<String>,
    /// The pane's tab, from `HERDR_TAB_ID`, when herdr set it.
    pub tab: Option<String>,
}

/// The pane this process runs in, or None outside herdr and in its unnamed
/// default session, which has no name to address it by. The one reader of
/// herdr's pane variables, so every caller agrees on what an empty one means.
pub fn current_pane() -> Option<CurrentPane> {
    let session = session_from_env(
        std::env::var("HERDR_SESSION").ok().as_deref(),
        std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
    )?;
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    Some(CurrentPane { session, pane: var("HERDR_PANE_ID"), tab: var("HERDR_TAB_ID") })
}

/// A workspace's herdr session: its name, the folder its terminals start in,
/// and the way out to herdr and niri.
pub struct Session {
    name: String,
    dir: PathBuf,
    port: Box<dyn Port>,
}

impl Session {
    /// A session by name, with the folder its terminals start in, through the
    /// process adapter: what a workspace's session is, built by
    /// `Workspace::session`.
    pub fn at(name: String, dir: PathBuf) -> Session {
        Session { name, dir, port: Box::new(herdr::Process) }
    }

    /// A session known by name alone, through the process adapter: as a pane
    /// knows its own, or as the panel asks about a workspace's agents. Its
    /// folder is unknown, and no HOME is read, so `open` must not be called
    /// on it.
    pub fn named(session: &str) -> Session {
        Session::at(session.to_string(), PathBuf::new())
    }

    /// The same session over any adapter: the tests' way in.
    // The tests' constructor: a plain build has no caller for it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_port(name: &str, dir: PathBuf, port: Box<dyn Port>) -> Session {
        Session { name: name.to_string(), dir, port }
    }

    /// The herdr session's name, as `--session` takes it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The folder the session's terminals start in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Hold this session against every other launch in it, waiting for one
    /// under way, until the claim is dropped. A launch takes it before
    /// [`open`](Session::open) and keeps it until Claude has started, so
    /// that two quick presses cannot both find no agent and both start one,
    /// nor both start a project terminal for a stopped session. Keyed by
    /// the session rather than the agent for that reason; two different
    /// tasks' launches in one session simply take turns.
    pub fn claim(&self) -> Result<Claim> {
        self.port.claim(&self.name)
    }

    /// Hold `task`'s setup lock until the returned claim is dropped: what a
    /// setup tab does for its whole life, so that a press of Start working
    /// meanwhile finds it held and points at that tab rather than opening
    /// another. Waits briefly ([`SETUP_TAKE_WAIT`]) since a launch's look
    /// takes it for a moment; held past that, another tab is setting the
    /// task up, and this one says so.
    pub fn hold_setup(&self, task: &str) -> Result<Claim> {
        self.port
            .setup_lock(task, SETUP_TAKE_WAIT)?
            .context("This task is already being set up in another tab.")
    }

    /// Make the session running and in front of the user, and return its
    /// herdr workspaces: start the project terminal if the session is
    /// stopped (a window then comes with it) and wait for it to answer;
    /// otherwise focus the window showing it, or attach another, since a
    /// running session may have had its window closed while its herdr server
    /// kept going. Shared by everything that opens something in a session.
    pub fn open(&self) -> Result<Vec<Workspace>> {
        // Any failure to list, herdr's refusal or herdr not running at all,
        // counts as stopped: the check below then says which it was.
        match self.port.workspace_list(&self.name) {
            Ok(Ok(workspaces)) => {
                self.show_window(&workspaces)?;
                Ok(workspaces)
            }
            _ => {
                anyhow::ensure!(self.port.herdr_installed(), "herdr is not installed.");
                // Through niri, so the window lands on the focused workspace,
                // the one the task belongs to, as the project list's does.
                self.port.spawn(crate::programs::project_terminal_command(&self.dir, &self.name, true))?;
                self.wait_for_start()
            }
        }
    }

    /// Bring the window showing the session into view: focus it if niri
    /// still has one open, or attach another client if the user closed it.
    /// herdr allows more than one client on a session, so a second attach is
    /// harmless, and without it any tab opened afterwards would be invisible.
    ///
    /// A session with no herdr workspace yet has no title to look for; the
    /// caller's `new_tab` creates one right after, with `--focus` itself.
    fn show_window(&self, workspaces: &[Workspace]) -> Result<()> {
        if workspaces.is_empty() {
            return Ok(());
        }
        let labels: Vec<String> = workspaces.iter().map(|w| w.label.clone()).collect();
        let windows = self.port.windows()?;
        match find_session_window(&windows, &labels) {
            Some(id) => self.port.focus_window(id),
            None => self.port.spawn(crate::programs::project_terminal_command(&self.dir, &self.name, true)),
        }
    }

    /// Poll until the session answers with a workspace in it. A server that
    /// answers but never gets one is returned as it is at the deadline, and
    /// the caller creates the workspace itself.
    ///
    /// The deadline is counted in polls through the port's sleep rather than
    /// read off a clock, so the fake runs it in no time; the polls are the
    /// clock's: one at the start and one after each pause, the last at 10s.
    fn wait_for_start(&self) -> Result<Vec<Workspace>> {
        let pauses = (SESSION_WAIT.as_millis() / SESSION_POLL.as_millis()) as u32;
        let mut answered = None;
        for pause in 0..=pauses {
            if let Ok(Ok(workspaces)) = self.port.workspace_list(&self.name) {
                if !workspaces.is_empty() {
                    return Ok(workspaces);
                }
                answered = Some(workspaces);
            }
            if pause < pauses {
                self.port.sleep(SESSION_POLL);
            }
        }
        match answered {
            Some(workspaces) => Ok(workspaces),
            None => bail!("herdr session {} did not start within {}s.", self.name, SESSION_WAIT.as_secs()),
        }
    }

    /// Whether `name` is a live agent here, and if so bring the user to it.
    /// herdr not runnable is an error, not false, so a missing herdr never
    /// passes for "no agent" and opens a duplicate.
    pub fn focus_agent(&self, name: &str) -> Result<bool> {
        if self.port.agent_get(&self.name, name)?.is_err() {
            return Ok(false);
        }
        self.port.agent_focus(&self.name, name)??;
        Ok(true)
    }

    /// A new tab in the session's first workspace, in `dir`, or, with no
    /// workspace yet, a first workspace labelled `workspace_label`: the
    /// workspace is named after the project, not after this tab. The pane to
    /// start Claude in, and the tab to close if that fails.
    pub fn new_tab(&self, workspaces: &[Workspace], dir: &Path, label: &str, workspace_label: &str) -> Result<Created> {
        match workspaces.first() {
            Some(w) => self.tab_in(&w.id, dir, label),
            None => Ok(self.port.workspace_create(&self.name, dir, workspace_label)??),
        }
    }

    /// A new tab in the herdr workspace `workspace`, in `dir`: for a caller
    /// that already knows which workspace, such as a worktree's. The pane to
    /// start Claude in, and the tab to close if that fails.
    pub fn tab_in(&self, workspace: &str, dir: &Path, label: &str) -> Result<Created> {
        Ok(self.port.tab_create(&self.name, workspace, dir, label)??)
    }

    /// Close a tab left bare by a failed start, so a retry does not pile
    /// another up beside it. Best effort: a failure here must not hide the
    /// error that led here.
    pub fn close_tab(&self, tab: &str) {
        let _ = self.port.tab_close(&self.name, tab);
    }

    /// Open an existing worktree as its own herdr workspace, grouped under
    /// the repo's, as the herdr-worktrunk plugin does; or find it open.
    pub fn open_worktree(&self, repo: &Path, path: &Path, label: &str) -> Result<Opened> {
        Ok(self.port.worktree_open(&self.name, repo, path, label)??)
    }

    /// Type `command` into a pane's shell and run it, for a step that needs
    /// a terminal to ask the user something on.
    pub fn run_in_pane(&self, pane: &str, command: &str) -> Result<()> {
        Ok(self.port.pane_run(&self.name, pane, command)??)
    }

    /// Start Claude as `name` in `pane`. A start that fails while the agent
    /// is `blocked` is Claude asking something first, such as whether to
    /// trust a folder it has not seen; that answer is the user's, so they are
    /// told and the start waits for it. Any other failure is a failure, and
    /// a "No" exits Claude, so the wait then fails with herdr's reason.
    ///
    /// `claim` is the launch's hold on the session, given up as soon as
    /// `agent start` returns, blocked or not: herdr has the agent under its
    /// name by then, so a second press finds it and lands in the tab that
    /// wants the answer, and other tasks' launches here need not wait on a
    /// question the user has not answered yet.
    pub fn start_claude(&self, claim: Claim, name: &str, pane: &str, claude: &Claude) -> Result<()> {
        let started = self.port.agent_start(&self.name, name, pane, claude);
        drop(claim);
        if let Err(e) = started? {
            // Any failure to ask is "not blocked": the start's own error is
            // the one worth showing.
            let blocked = matches!(
                self.port.agent_get(&self.name, name),
                Ok(Ok(Agent { status: Some(ref status), .. })) if status == "blocked"
            );
            if !blocked {
                return Err(e.into());
            }
            eprintln!(
                "Claude is asking something before it starts — most likely whether to trust this \
                 folder. Answer it in its tab; this carries on once Claude is ready."
            );
            self.port.notify("Claude needs an answer before it can start — see its tab.");
            // The context goes on both layers, herdr failing to run as well as
            // herdr refusing, so either says what was being waited for.
            self.port
                .agent_wait_ready(&self.name, name)
                .and_then(|answer| Ok(answer?))
                .context("Claude did not become ready")?;
        }
        Ok(())
    }

    /// Send `text` until herdr sees Claude start on it. Right after a start-up
    /// question is answered, herdr can report Claude idle a moment before it
    /// takes input, and a prompt sent then vanishes without an error; so it is
    /// confirmed, and resent on a stall.
    pub fn prompt(&self, name: &str, text: &str) -> Result<()> {
        for attempt in 1..=PROMPT_TRIES {
            let (code, message) = match self.port.agent_prompt(&self.name, name, text)? {
                Ok(()) => (None, String::new()),
                Err(e) => (e.code, e.message),
            };
            match prompt_outcome(code.as_deref()) {
                PromptOutcome::Delivered => return Ok(()),
                PromptOutcome::Resend if attempt < PROMPT_TRIES => self.port.sleep(PROMPT_RETRY),
                _ => bail!("Claude started, but the prompt was not delivered: {message}"),
            }
        }
        unreachable!("the last attempt returns or bails")
    }

    /// The named live agents; none when the session is stopped or herdr is
    /// not there, since the panel asks on every open and must not fail for it.
    pub fn agent_names(&self) -> Vec<String> {
        match self.port.agent_list(&self.name) {
            Ok(Ok(names)) => names,
            _ => Vec::new(),
        }
    }

    /// The agent in `target`, a pane id or a name, if there is one. herdr
    /// not runnable is an error, not None.
    pub fn agent(&self, target: &str) -> Result<Option<Agent>> {
        Ok(self.port.agent_get(&self.name, target)?.ok())
    }

    /// Give the agent in `target`, a pane id or a name, the name `name`: how
    /// a Claude started by hand gets the name its task's card looks for.
    pub fn rename_agent(&self, target: &str, name: &str) -> Result<()> {
        Ok(self.port.agent_rename(&self.name, target, name)??)
    }
}

/// The ghostty window already showing one of the session's `labels`, if niri
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
        .find(|w| {
            w.app_id.as_deref() == Some("com.mitchellh.ghostty")
                && w.title.as_deref().is_some_and(|t| suffixes.iter().any(|s| t.ends_with(s)))
        })
        .map(|w| w.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_workspace_name_is_its_own_session_name() {
        assert_eq!(herdr_session_name("niri-tasks"), "niri-tasks");
        assert_eq!(herdr_session_name("my_project"), "my_project");
        assert_eq!(herdr_session_name("v1.2"), "v1.2", "herdr allows dots");
        assert_eq!(herdr_session_name("MyProject"), "MyProject", "case is kept");
    }

    #[test]
    fn characters_herdr_rejects_become_underscores_one_for_one() {
        assert_eq!(herdr_session_name("my project"), "my_project");
        assert_eq!(herdr_session_name("a  b"), "a__b", "runs are not collapsed");
        assert_eq!(herdr_session_name("a/b:c"), "a_b_c");
        assert_eq!(herdr_session_name("café"), "caf_", "non-ASCII is one char, one underscore");
    }

    #[test]
    fn long_names_are_cut_to_what_herdr_accepts() {
        let long = "x".repeat(100);
        assert_eq!(herdr_session_name(&long).len(), SESSION_NAME_MAX);
        let unicode = "é".repeat(100);
        assert_eq!(herdr_session_name(&unicode).len(), SESSION_NAME_MAX);
    }

    #[test]
    fn names_herdr_treats_specially_are_prefixed() {
        assert_eq!(herdr_session_name("default"), "ws-default");
        assert_eq!(herdr_session_name("."), "ws-.");
        assert_eq!(herdr_session_name(".."), "ws-..");
        assert_eq!(herdr_session_name(""), "ws-");
        // Only the exact names: these are ordinary.
        assert_eq!(herdr_session_name("Default"), "Default");
        assert_eq!(herdr_session_name("defaults"), "defaults");
    }

    /// The session name cannot be unfolded — "a  b" and "a__b" both become
    /// "a__b" — so the live workspace names are run through the same rule and
    /// compared, which recovers the original.
    #[test]
    fn a_session_finds_the_workspace_that_named_it() {
        let workspaces: Vec<String> = ["niri-tasks", "a  b", "My Project", "default"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        assert_eq!(workspace_for_session("niri-tasks", &workspaces), Some(&workspaces[0]));
        assert_eq!(workspace_for_session("a__b", &workspaces), Some(&workspaces[1]));
        assert_eq!(workspace_for_session("My_Project", &workspaces), Some(&workspaces[2]));
        assert_eq!(workspace_for_session("ws-default", &workspaces), Some(&workspaces[3]));
    }

    /// A session whose workspace was renamed or closed matches nothing, rather
    /// than resolving to a plausible-looking wrong one.
    #[test]
    fn a_session_with_no_live_workspace_matches_nothing() {
        let workspaces = vec!["niri-tasks".to_string()];
        assert_eq!(workspace_for_session("old-name", &workspaces), None);
        assert_eq!(workspace_for_session("niri", &workspaces), None, "no prefix matching");
    }

    #[test]
    fn herdr_session_comes_from_herdr_session_first() {
        assert_eq!(
            session_from_env(Some("alpha"), Some("/h/.config/herdr/sessions/beta/herdr.sock")),
            Some("alpha".to_string())
        );
    }

    #[test]
    fn socket_path_is_the_backup_for_a_named_session() {
        assert_eq!(
            session_from_env(None, Some("/h/.config/herdr/sessions/beta/herdr.sock")),
            Some("beta".to_string())
        );
        assert_eq!(
            session_from_env(Some(""), Some("/h/.config/herdr/sessions/beta/herdr.sock")),
            Some("beta".to_string()),
            "an empty HERDR_SESSION is no name"
        );
    }

    #[test]
    fn the_default_session_and_no_herdr_have_no_session() {
        assert_eq!(session_from_env(None, Some("/h/.config/herdr/herdr.sock")), None);
        assert_eq!(session_from_env(None, None), None);
    }

    use super::fake::{Fake, StartOutcome};

    /// A session over `fake`, and the fake again to read after acting: its
    /// clones share one state.
    fn session(fake: Fake) -> (Session, Fake) {
        let s = Session::with_port("alpha", PathBuf::from("/home/x/Projects/alpha"), Box::new(fake.clone()));
        (s, fake)
    }

    fn logged(f: &Fake, line: &str) -> bool {
        f.log().iter().any(|l| l == line)
    }

    fn logged_start(f: &Fake, prefix: &str) -> bool {
        f.log().iter().any(|l| l.starts_with(prefix))
    }

    /// A setup tab holds its task's setup lock, waiting briefly for it,
    /// until the claim it was given is dropped.
    #[test]
    fn a_setup_tab_holds_its_tasks_lock_until_it_goes() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        let held = s.hold_setup("7cd9fd3a-0000").unwrap();
        assert!(logged(&f, "setup_lock 7cd9fd3a-0000 2000ms"), "{:?}", f.log());
        assert!(!logged(&f, "setup_release 7cd9fd3a-0000"));
        drop(held);
        assert!(logged(&f, "setup_release 7cd9fd3a-0000"), "{:?}", f.log());
    }

    /// The first `workspace_list` finds it stopped; the terminal is spawned
    /// right after, and the wait sleeps between the two refused polls that
    /// remain and the one that answers.
    #[test]
    fn opening_a_stopped_session_starts_the_terminal_and_waits() {
        let (s, f) = session(Fake::stopped(3, "alpha"));
        let ws = s.open().unwrap();
        assert_eq!(ws, vec![Workspace { id: "w1".into(), label: "alpha".into() }]);
        let log = f.log();
        assert_eq!(log[0], "workspace_list alpha", "{log:?}");
        assert!(log[1].starts_with("spawn ") && log[1].ends_with("-e herdr --session alpha"), "{log:?}");
        assert_eq!(log.iter().filter(|l| *l == "sleep 250ms").count(), 2, "{log:?}");
    }

    /// Ten seconds at a quarter-second apart, counted through the port's
    /// sleep: the same deadline as a clock would give.
    #[test]
    fn a_session_that_never_starts_is_an_error_after_the_deadline() {
        let (s, f) = session(Fake::stopped(0, "alpha"));
        let err = s.open().unwrap_err().to_string();
        assert!(err.contains("did not start within 10s"), "{err}");
        assert_eq!(f.log().iter().filter(|l| *l == "sleep 250ms").count(), 40);
    }

    /// Refine's fresh server: it answers, but its client has made no
    /// workspace by the deadline. The empty list comes back, not an error,
    /// and the caller makes the workspace itself.
    #[test]
    fn a_session_that_answers_with_no_workspace_is_returned_empty_at_the_deadline() {
        let (s, f) = session(Fake::stopped(1, "alpha").without_workspaces());
        assert_eq!(s.open().unwrap(), vec![]);
        let log = f.log();
        assert_eq!(log.iter().filter(|l| *l == "sleep 250ms").count(), 40, "{log:?}");
        assert_eq!(log.iter().filter(|l| l.starts_with("spawn")).count(), 1, "{log:?}");
        assert!(!log.contains(&"windows".to_string()), "{log:?}");
    }

    #[test]
    fn a_stopped_session_without_herdr_says_so() {
        let (s, f) = session(Fake::stopped(3, "alpha").not_installed());
        let err = s.open().unwrap_err().to_string();
        assert_eq!(err, "herdr is not installed.");
        assert!(!logged_start(&f, "spawn"));
    }

    #[test]
    fn a_running_session_with_a_window_is_focused_not_reopened() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_window(7, "com.mitchellh.ghostty", "host: alpha"));
        s.open().unwrap();
        assert!(logged(&f, "focus_window 7"), "{:?}", f.log());
        assert!(!logged_start(&f, "spawn"));
    }

    #[test]
    fn a_running_session_with_no_window_gets_another_terminal() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_window(7, "com.mitchellh.ghostty", "host: beta"));
        s.open().unwrap();
        assert!(logged_start(&f, "spawn "), "{:?}", f.log());
        assert!(!logged_start(&f, "focus_window"));
    }

    #[test]
    fn a_running_session_with_no_workspace_looks_for_no_window() {
        let (s, f) = session(Fake::running(&[]));
        assert!(s.open().unwrap().is_empty());
        assert!(!logged(&f, "windows"));
        assert!(!logged_start(&f, "spawn"));
    }

    #[test]
    fn focusing_an_agent_says_whether_there_was_one() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_agent(Some("task-abc"), "p1", "idle"));
        assert!(s.focus_agent("task-abc").unwrap());
        assert!(logged(&f, "agent_focus alpha task-abc"));
        assert!(!s.focus_agent("work-abc").unwrap());
        assert!(!logged(&f, "agent_focus alpha work-abc"));
        let (s, _) = session(Fake::running(&[]).not_installed());
        assert!(s.focus_agent("task-abc").is_err(), "no herdr is an error, not no");
    }

    #[test]
    fn a_new_tab_goes_in_the_first_workspace_or_makes_one() {
        let (s, f) = session(Fake::running(&[("w1", "alpha"), ("w2", "task/x")]));
        let ws = s.open().unwrap();
        let tab = s.new_tab(&ws, Path::new("/d"), "Refine: x", "alpha").unwrap();
        assert!(logged(&f, "tab_create alpha w1 /d Refine: x"), "{:?}", f.log());
        assert!(tab.tab.is_some());
        let (s, f) = session(Fake::running(&[]));
        let made = s.new_tab(&[], Path::new("/d"), "Refine: x", "alpha").unwrap();
        assert!(logged(&f, "workspace_create alpha /d alpha"), "{:?}", f.log());
        assert!(!made.pane.is_empty());
    }

    #[test]
    fn a_tab_opens_in_the_workspace_named() {
        let (s, f) = session(Fake::running(&[("w1", "alpha"), ("w2", "task/x")]));
        let tab = s.tab_in("w2", Path::new("/w/x"), "Claude").unwrap();
        assert_eq!(f.log(), vec!["tab_create alpha w2 /w/x Claude"]);
        assert!(tab.tab.is_some());
    }

    #[test]
    fn closing_a_tab_is_best_effort() {
        let (s, f) = session(Fake::running(&[]));
        s.close_tab("t3");
        assert!(logged(&f, "tab_close alpha t3"));
        // No herdr at all: still tried, and the failure swallowed rather
        // than raised over the error that led here.
        let (s, f) = session(Fake::running(&[]).not_installed());
        s.close_tab("t3");
        assert!(logged(&f, "tab_close alpha t3"));
    }

    #[test]
    fn an_open_worktree_is_found_again_and_a_new_one_opened() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_worktree("/w/x", "w5", "p9"));
        let opened = s.open_worktree(Path::new("/p"), Path::new("/w/x"), "task/x").unwrap();
        assert_eq!(opened, Opened { workspace: "w5".into(), pane: Some("p9".into()) });
        assert!(logged(&f, "worktree_open alpha /p /w/x task/x"));
        let fresh = s.open_worktree(Path::new("/p"), Path::new("/w/y"), "task/y").unwrap();
        assert_ne!(fresh.workspace, "w5");
        assert!(fresh.pane.is_some());
        assert!(logged(&f, "worktree_open alpha /p /w/y task/y"));
    }

    #[test]
    fn a_command_runs_in_the_named_pane() {
        let (s, f) = session(Fake::running(&[]));
        s.run_in_pane("p4", "wt switch x").unwrap();
        assert!(logged(&f, "pane_run alpha p4 wt switch x"));
    }

    #[test]
    fn a_clean_start_starts_once_and_waits_for_nothing() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        s.start_claude(s.claim().unwrap(), "task-abc", "p1", &Claude::Refiner { settings: "{}".into(), mod_dir: PathBuf::from("/m") })
            .unwrap();
        assert!(logged(&f, "agent_start alpha task-abc p1 refiner"));
        assert!(!logged_start(&f, "agent_wait_ready"));
    }

    #[test]
    fn a_blocked_start_is_waited_out_and_a_failed_one_is_an_error() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::FailsBlockedThenIdle));
        s.start_claude(s.claim().unwrap(), "work-abc", "p1", &Claude::Worker).unwrap();
        let log = f.log();
        assert!(log.iter().any(|l| l.starts_with("notify ")), "{log:?}");
        assert!(log.contains(&"agent_wait_ready alpha work-abc".to_string()), "{log:?}");
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::Fails));
        assert_eq!(
            s.start_claude(s.claim().unwrap(), "work-abc", "p1", &Claude::Worker).unwrap_err().to_string(),
            "claude exited"
        );
        assert!(!logged_start(&f, "agent_wait_ready"));
        assert!(!logged_start(&f, "notify"));
    }

    #[test]
    fn a_prompt_is_resent_on_a_stall_and_given_up_after_four() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        s.prompt("work-abc", "/plan").unwrap();
        assert_eq!(f.log(), vec!["agent_prompt alpha work-abc /plan"]);

        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("agent_prompt_stalled"), None]));
        s.prompt("work-abc", "/plan").unwrap();
        let log = f.log();
        assert_eq!(log.iter().filter(|l| l.starts_with("agent_prompt ")).count(), 2, "{log:?}");
        assert_eq!(log.iter().filter(|l| *l == "sleep 2000ms").count(), 1, "{log:?}");

        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("agent_prompt_stalled"); 4]));
        let err = s.prompt("work-abc", "/plan").unwrap_err().to_string();
        assert_eq!(err, "Claude started, but the prompt was not delivered: prompt not taken");
        assert_eq!(f.log().iter().filter(|l| l.starts_with("agent_prompt ")).count(), 4);
        assert_eq!(f.log().iter().filter(|l| *l == "sleep 2000ms").count(), 3);

        let (s, _) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("timeout")]));
        s.prompt("work-abc", "/plan").unwrap();
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("agent_not_found")]));
        assert!(s.prompt("work-abc", "/plan").is_err());
        assert_eq!(f.log().len(), 1, "a failure is not resent");
    }

    #[test]
    fn agent_names_are_empty_for_a_stopped_session_or_no_herdr() {
        let (s, _) = session(Fake::stopped(0, "alpha"));
        assert!(s.agent_names().is_empty());
        let (s, _) = session(Fake::running(&[]).not_installed());
        assert!(s.agent_names().is_empty());
        let (s, _) = session(Fake::running(&[]).with_agent(Some("task-a"), "p1", "idle").with_agent(None, "p2", "idle"));
        assert_eq!(s.agent_names(), vec!["task-a".to_string()]);
    }

    #[test]
    fn a_panes_agent_is_found_and_renamed() {
        let (s, f) = session(Fake::running(&[]).with_agent(None, "p2", "idle"));
        assert_eq!(s.agent("p2").unwrap(), Some(Agent { name: None, status: Some("idle".into()) }));
        assert_eq!(s.agent("p9").unwrap(), None);
        s.rename_agent("p2", "work-abc").unwrap();
        assert!(logged(&f, "agent_rename alpha p2 work-abc"));
        assert_eq!(f.agents()[0].name.as_deref(), Some("work-abc"));
    }

    #[test]
    fn a_name_already_taken_is_not_renamed_to() {
        let (s, _) = session(Fake::running(&[]).with_agent(Some("work-abc"), "p1", "idle").with_agent(None, "p2", "idle"));
        assert!(s.rename_agent("p2", "work-abc").is_err());
    }

    /// The claim goes through the port by the session's name, and is given
    /// up when dropped, not before: what the launches' ordering rests on.
    #[test]
    fn a_claim_is_held_until_dropped() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        let claim = s.claim().unwrap();
        s.open().unwrap();
        assert_eq!(f.log().first().map(String::as_str), Some("claim alpha"));
        assert!(!logged(&f, "release alpha"), "{:?}", f.log());
        drop(claim);
        assert_eq!(f.log().last().map(String::as_str), Some("release alpha"));
    }

    /// The claim goes the moment `agent start` returns: before a blocked
    /// start's question is waited out, which can take the user minutes, and
    /// before anything else is asked of herdr, whatever the outcome.
    #[test]
    fn a_start_lets_the_claim_go_before_waiting_on_a_question() {
        for outcome in [StartOutcome::Ok, StartOutcome::Fails, StartOutcome::FailsBlockedThenIdle] {
            let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_start(outcome));
            let _ = s.start_claude(s.claim().unwrap(), "work-abc", "p1", &Claude::Worker);
            let log = f.log();
            let start = log.iter().position(|l| l.starts_with("agent_start ")).unwrap();
            assert_eq!(log[start + 1], "release alpha", "{outcome:?}: {log:?}");
        }
    }

    #[test]
    fn a_session_knows_its_name_and_folder() {
        let (s, _) = session(Fake::running(&[]));
        assert_eq!(s.name(), "alpha");
        assert_eq!(s.dir(), Path::new("/home/x/Projects/alpha"));
    }

    /// A session known by name only has no folder, and finding it needs no
    /// HOME: the pane link and the panel's agent list need neither.
    #[test]
    fn a_session_by_name_alone_has_no_folder() {
        let s = Session::named("alpha");
        assert_eq!(s.name(), "alpha");
        assert_eq!(s.dir(), Path::new(""));
    }

    /// What a `--wait` prompt's outcome means: `timeout` is Claude still busy
    /// with it (delivered), a stall is Claude never starting (resend), anything
    /// else is a failure.
    #[test]
    fn a_stalled_prompt_is_resent_and_a_timed_out_one_counts_as_delivered() {
        assert_eq!(prompt_outcome(None), PromptOutcome::Delivered);
        assert_eq!(prompt_outcome(Some("timeout")), PromptOutcome::Delivered);
        assert_eq!(prompt_outcome(Some("agent_prompt_stalled")), PromptOutcome::Resend);
        assert_eq!(prompt_outcome(Some("agent_blocked")), PromptOutcome::Failed);
    }

    const GHOSTTY: &str = "com.mitchellh.ghostty";

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn window(id: u64, app_id: &str, title: &str) -> WindowInfo {
        WindowInfo { id, app_id: Some(app_id.into()), title: Some(title.into()) }
    }

    #[test]
    fn finds_the_ghostty_window_whose_title_ends_with_the_label() {
        let windows = [window(1, GHOSTTY, "paul-msi-ubuntu: hansard")];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), Some(1));
    }

    /// After Start working, herdr's title names the task's worktree
    /// workspace, not the project's: the window is still the session's.
    #[test]
    fn finds_the_window_showing_a_worktree_workspace() {
        let windows = [window(4, GHOSTTY, "paul-msi-ubuntu: task/fix-it-6a970973")];
        let session = labels(&["niri-tasks", "task/fix-it-6a970973"]);
        assert_eq!(find_session_window(&windows, &session), Some(4));
    }

    /// Another session's worktree is not this session's window, even though
    /// both titles start with `task/`.
    #[test]
    fn a_worktree_of_another_session_does_not_match() {
        let windows = [window(4, GHOSTTY, "paul-msi-ubuntu: task/other-1234abcd")];
        let session = labels(&["niri-tasks", "task/fix-it-6a970973"]);
        assert_eq!(find_session_window(&windows, &session), None);
    }

    #[test]
    fn no_window_matches_a_different_label() {
        let windows = [window(1, GHOSTTY, "paul-msi-ubuntu: other")];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), None);
    }

    #[test]
    fn a_session_with_no_workspaces_matches_no_window() {
        let windows = [window(1, GHOSTTY, "paul-msi-ubuntu: hansard")];
        assert_eq!(find_session_window(&windows, &[]), None);
    }

    #[test]
    fn a_different_app_id_with_the_matching_title_does_not_match() {
        let windows = [window(1, "org.wezfurlong.wezterm", "paul-msi-ubuntu: hansard")];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), None);
    }

    /// "tasks" must not match a title ending "niri-tasks" — a label that is a
    /// suffix of another workspace's label is not the same workspace.
    #[test]
    fn a_label_that_is_a_suffix_of_another_label_does_not_match() {
        let windows = [window(1, GHOSTTY, "paul-msi-ubuntu: niri-tasks")];
        assert_eq!(find_session_window(&windows, &labels(&["tasks"])), None);
    }

    /// A window with no title or no app id is nobody's session window.
    #[test]
    fn a_window_without_a_title_or_app_id_does_not_match() {
        let windows = [
            WindowInfo { id: 1, app_id: Some(GHOSTTY.into()), title: None },
            WindowInfo { id: 2, app_id: None, title: Some("paul-msi-ubuntu: hansard".into()) },
        ];
        assert_eq!(find_session_window(&windows, &labels(&["hansard"])), None);
    }
}
