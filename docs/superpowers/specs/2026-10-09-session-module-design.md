# Session module design (D1): the port, the adapter and the fake

Supplements the 2026-10-03 design (`2026-10-03-session-module-design.md`),
whose decisions Q1-Q18 and Q20-Q28 stand. This document settles Q19 and
fixes the scope of task `0cb3900d`: parts (a) and (b) of that design, the
move into one module and the test seam. Parts (c) (Start working rules 1-4,
`--here` without `--workspace`, `--close-tab`, the clipboard; Q20, Q25-Q28)
and (d) (the double-refine race, task `2ee54dab`) are later tasks and change
nothing here: `task start --here --workspace <name>` keeps its shape.

## Q19: step methods over a typed port

The Session's interface is a handful of step methods (D), not one deep
`claude(...)` call (C): Refine and Start working share the steps but not the
sequence (Start working opens a worktree and waits out a blocked start;
Refine checks a sandbox and a mod first), and a one-call interface would
carry every knob of both. The port under the Session is typed operations, not
raw argv: the fake then models herdr's state instead of parsing argv, and the
argv builders and JSON readers stay where they are today, as the process
adapter's implementation with their 22 tests.

## The module

`src/session.rs` grows into the deep module; `src/herdr.rs` moves to
`src/session/herdr.rs` as the process adapter and stops being `pub`;
`src/session/fake.rs` is the test adapter under `#[cfg(test)]`. Today's
naming rules (`herdr_session_name`, `workspace_for_session`,
`session_from_env`, `start_dir`, `project_from_cwd`) stay as they are, with
their eleven tests.

### Errors (Q13, Q23)

```rust
/// A failure herdr itself reported: its JSON error's code and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HerdrError { pub code: Option<String>, pub message: String }
impl std::fmt::Display for HerdrError  // the message
impl std::error::Error for HerdrError
/// What a herdr call comes back with: herdr's own refusal as `Err(HerdrError)`,
/// wrapped in the crate's `Result` for "could not run herdr at all".
pub type HerdrResult<T> = anyhow::Result<std::result::Result<T, HerdrError>>;
```

A `HerdrError` reaching the user keeps its message, as today's `run` does;
callers that act on the code (`agent_not_found`, `agent_prompt_stalled`,
`timeout`) read it off the value.

### The port (Q8, Q9, Q16, Q22)

```rust
/// Everything the session module does outside itself. Two adapters make the
/// seam real: `herdr::Process` (std::process, niri_ipc, thread::sleep,
/// notify-send) and, in tests, `fake::Fake`.
pub(crate) trait Port {
    // herdr, every call naming the session
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
    fn agent_prompt(&self, session: &str, name: &str, text: &str, confirm: bool) -> HerdrResult<()>;
    // niri
    fn windows(&self) -> anyhow::Result<Vec<WindowInfo>>;
    fn focus_window(&self, id: u64) -> anyhow::Result<()>;
    fn spawn(&self, command: Vec<String>) -> anyhow::Result<()>;
    // time and the user
    fn sleep(&self, d: Duration);
    fn notify(&self, text: &str);
    fn herdr_installed(&self) -> bool;
}

pub struct Workspace { pub id: String, pub label: String }
pub struct Created { pub pane: String, pub tab: Option<String> }     // tab_create / workspace_create
pub struct Opened { pub workspace: String, pub pane: String }        // worktree_open
pub struct Agent { pub name: Option<String>, pub status: Option<String> }
pub struct WindowInfo { pub id: u64, pub app_id: Option<String>, pub title: Option<String> }
/// Which Claude `agent start` runs: the refiner, fenced by its settings and
/// mod, or the worker with the user's own defaults.
pub enum Claude { Refiner { settings: String, mod_dir: PathBuf }, Worker }
```

`herdr::Process` implements it with today's argv builders and JSON readers
(`workspace_labels` and `first_workspace_id` become the `Workspace` list;
`root_pane_id` and `created_tab_id` become `Created`; `opened_workspace_id`
and `root_pane_id` become `Opened`; `agent_name_of` and `agent_status` become
`Agent`; `agent_names` the list; `run_coded` is the one runner, `run` is
gone; `REFINER_SYSTEM_PROMPT` and the refiner flags stay in the adapter).
The argv it produces is byte-for-byte today's, which the PATH-shim tests
(`tests/typed_task_number.rs`, `tests/link_pane.rs`, `tests/e2e-tag.sh`)
assert on and keep asserting on (Q11).

### The value (Q3, Q5)

```rust
/// A workspace's herdr session: its name, the folder its terminals start in,
/// and the way out to herdr and niri.
pub struct Session { name: String, dir: PathBuf, port: Box<dyn Port> }

impl Session {
    /// The session for `workspace`, through the process adapter.
    pub fn for_workspace(workspace: &str) -> anyhow::Result<Session>;   // HOME → start_dir; herdr_session_name
    /// For tests: the same with any adapter.
    pub(crate) fn with_port(name: &str, dir: PathBuf, port: Box<dyn Port>) -> Session;
    pub fn name(&self) -> &str;
    pub fn dir(&self) -> &Path;
}

/// The herdr pane this process runs in, from what herdr hands its panes:
/// `HERDR_SESSION` (else the session dir of `HERDR_SOCKET_PATH`),
/// `HERDR_PANE_ID` and `HERDR_TAB_ID`. None outside herdr or in its unnamed
/// default session. Replaces the env reads in `lib.rs::herdr_session`,
/// `link.rs::link_current_pane` and `work.rs::set_up_here`.
pub struct CurrentPane { pub session: String, pub pane: Option<String>, pub tab: Option<String> }
pub fn current_pane() -> Option<CurrentPane>;
```

### The steps

Each is today's code moved, named for what it does; the lines it comes from
are in the plan.

```rust
impl Session {
    /// Make the session running and in front of the user; its herdr workspaces.
    /// Stopped: start the project terminal and poll until it answers (10s, every
    /// 250ms, through the port's sleep). Running: focus the ghostty window whose
    /// title ends in one of its workspace labels, else attach another terminal.
    pub fn open(&self) -> anyhow::Result<Vec<Workspace>>;               // refine.rs open_session + show_session_window + find_session_window + wait_for_session
    /// Whether `name` is a live agent here; if so, focus it. Herdr not
    /// runnable is an error, not false.
    pub fn focus_agent(&self, name: &str) -> anyhow::Result<bool>;        // the agent_get-then-agent_focus pair in refine.rs:350 and work.rs:229
    /// A new tab in the session's first workspace, or, with none yet, a first
    /// workspace labelled `workspace_label`; the pane to start Claude in.
    pub fn new_tab(&self, workspaces: &[Workspace], dir: &Path, label: &str, workspace_label: &str) -> anyhow::Result<Created>;   // refine.rs:357-362, work.rs:243-247
    /// Best-effort: a tab left bare by a failed start, closed so a retry does not pile up.
    pub fn close_tab(&self, tab: &str);
    /// Open a worktree as its own workspace, grouped under the repo's.
    pub fn open_worktree(&self, repo: &Path, path: &Path, label: &str) -> anyhow::Result<Opened>;
    pub fn run_in_pane(&self, pane: &str, command: &str) -> anyhow::Result<()>;
    /// Start Claude as `name` in `pane`. A start that fails while the agent is
    /// `blocked` is Claude asking something first (folder trust on a new
    /// worktree): the user is told, and the start waits for the answer.
    pub fn start_claude(&self, name: &str, pane: &str, claude: &Claude) -> anyhow::Result<()>;   // work.rs start_claude's blocked-wait, minus set_active
    /// Send `text` until herdr sees Claude start on it: confirmed, resent on a
    /// stall up to four times two seconds apart.
    pub fn prompt(&self, name: &str, text: &str) -> anyhow::Result<()>;   // work.rs deliver_prompt + prompt_outcome
    /// The named live agents; none when the session is stopped or herdr is not there.
    pub fn agent_names(&self) -> Vec<String>;                            // link.rs live_agent_names
    /// The agent in `target` (a pane or a name), if any.
    pub fn agent(&self, target: &str) -> anyhow::Result<Option<Agent>>;   // link.rs link_current_pane's agent_get
    pub fn rename_agent(&self, target: &str, name: &str) -> anyhow::Result<()>;
}
```

One behaviour change, Q21: Refine's prompt goes through `prompt` (confirmed,
resent on a stall) and its start through `start_claude` (a blocked start is
waited out), where today it fires `agent prompt` once and fails on a blocked
start. `prompt_outcome` stays a pure function in the module with its tests.

### Callers after the move

- `refine::launch` keeps the fence (sockets, mod, settings; ADR 0002), then
  `let s = Session::for_workspace(ws)?; let ws_list = s.open()?; if s.focus_agent(&name)? { notify; return } let tab = s.new_tab(&ws_list, s.dir(), &label, ws)?; if let Err(e) = s.start_claude(&name, &tab.pane, &Claude::Refiner{..}) { tab.tab.map(|t| s.close_tab(&t)); return Err(e) } s.prompt(&name, &prompt(..))`.
- `work::launch` and `work::set_up`: the same shape with `open_worktree`,
  `new_tab`, `start_claude(.., &Claude::Worker)`, `prompt`, then
  `task::set_active`; `run_in_pane` for the setup command; `set_up_here`
  closes its own tab through `current_pane().and_then(|p| p.tab)`.
- `link.rs` keeps the naming rules (`pane_link_name`, `session_agent`) and
  `go_to`/`live_agent`/`live_agent_names`/`link_current_pane` become thin
  over `Session`; its env reads go through `current_pane()`.
- `lib.rs::herdr_session` becomes `session::current_pane().map(|p| p.session)`.
- `refine.rs` loses `open_session`, `show_session_window`,
  `find_session_window`, `wait_for_session`, `SESSION_WAIT`, `SESSION_POLL`;
  `work.rs` loses `start_claude`, `deliver_prompt`, `PROMPT_TRIES`,
  `PromptOutcome`, `prompt_outcome` (all move).
- `pub mod herdr` leaves `lib.rs`; nothing outside `session` names herdr.

### The fake (Q10)

`session/fake.rs`, `#[cfg(test)]`: `Fake` holds a `RefCell<State>` and
implements `Port`. Its state per session: `running: bool` with
`boot_polls: u32` (each `workspace_list` on a not-yet-running session counts
one poll down; at zero the session becomes running, so `open` on a stopped
session is tested without time); `workspaces: Vec<Workspace>`; tabs with
root panes (`tab_create`/`workspace_create` mint `t<n>`/`p<n>` ids);
`worktrees: HashMap<PathBuf, Opened>`; `agents: Vec<FakeAgent { name,
pane, status }>`; `start_outcome: StartOutcome { Ok, Fails, FailsBlockedThenIdle }`;
`prompt_codes: Vec<Option<String>>` (one per `agent_prompt --wait`
attempt, `None` = delivered); `windows: Vec<WindowInfo>`; `installed: bool`;
and a `log: Vec<String>` of every call in a readable form (`"spawn ghostty …"`,
`"focus_window 7"`, `"agent_focus task-abc"`, `"notify …"`, `"sleep 250ms"`,
`"tab_close t1"`). Builder methods make the arrangements the tests need:
`Fake::stopped(boot_polls)`, `Fake::running(workspaces)`, `.with_agent(name,
pane, status)`, `.with_window(id, app_id, title)`, `.with_start(outcome)`,
`.with_prompt_codes(..)`, `.not_installed()`.

## Tests

At the Session interface, against the fake (in `session.rs`'s test module):

- `open`: stopped → spawns the project terminal and polls until running
  (the log shows the spawn, then `boot_polls` sleeps); stopped and never
  answering → `Err("did not start")`; running with a window whose title ends
  in a label → `focus_window`; running with no such window → spawns another
  terminal; running with no workspace yet → no window looked for, the empty
  list returned.
- `focus_agent`: a live agent → true and `agent_focus` logged; none → false
  and no focus; `not_installed` → Err.
- `new_tab`: with a workspace → `tab_create` in the first; with none →
  `workspace_create` labelled with the workspace name; both return the pane.
- `start_claude`: Ok → one start; Fails → Err and no wait; FailsBlockedThenIdle
  → `notify` logged, `agent_wait_ready` logged, Ok.
- `prompt`: `[None]` → one attempt; `[stalled, None]` → two attempts and one
  2s sleep; four stalls → Err naming the message; `[timeout]` → delivered;
  `[other]` → Err.
- `agent_names`: stopped → empty; not installed → empty (never an error);
  running → the named agents only.
- `agent`/`rename_agent`: a pane with an unnamed agent → `Agent{name: None}`;
  rename logged.
- `prompt_outcome`'s three existing tests move with it.

The flows (`refine::launch`, `work::launch`) also touch Taskwarrior, the
filesystem and `wt`, so they are not driven against the fake here; the
PATH-shim suites keep covering them end to end, with the same argv. Flow
tests against the fake are part (c)'s work, when the flows get a `launch_in`
seam.

## Out of scope

Parts (c) and (d) of the 2026-10-03 design; the agent-name prefixes stay in
`refine::agent_name` and `work::work_agent_name` (Q6 moves them to `link.rs`
in part (c)); CONTEXT.md's **Session** entry already names the concept and
needs no change.
