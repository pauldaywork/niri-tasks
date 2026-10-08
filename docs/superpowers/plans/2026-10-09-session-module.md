# Session Module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One deep `session` module owns opening a workspace's herdr session and putting or finding a Claude in it, behind a typed port with two adapters (the process, and an in-crate fake), so Refine, Start working and Go to session stop copying the herdr sequences and the module is tested without herdr.

**Architecture:** `src/herdr.rs` moves to `src/session/herdr.rs` as the process adapter implementing `session::Port` (its argv builders and JSON readers stay, with their tests). `session.rs` gains the port's types, a `Session { name, dir, port }` value with step methods (`open`, `focus_agent`, `new_tab`, `close_tab`, `open_worktree`, `run_in_pane`, `start_claude`, `prompt`, `agent_names`, `agent`, `rename_agent`) and `current_pane()`. `session/fake.rs` is the stateful test adapter. Then `refine.rs`, `work.rs`, `link.rs` and `lib.rs` call the Session and lose their copies. Taskwarrior task `0cb3900d`.

**Tech Stack:** Rust (anyhow, serde_json, niri_ipc), bash PATH-shim tests.

**Spec:** `docs/superpowers/specs/2026-10-09-session-module-design.md`, which supplements `docs/superpowers/specs/2026-10-03-session-module-design.md` (both committed with this plan).

## Global Constraints

- Conventional Commits with an attribution trailer (`Co-Authored-By: <your model> <noreply@anthropic.com>`); subject ≤ 72, body wrapped at 72 saying what and why.
- `cargo test` passes and `cargo build` has no warnings after every task. `cargo test` includes `tests/typed_task_number.rs` and `tests/link_pane.rs`, which run the real binary against a fake `herdr` on PATH and assert on the exact argv; **the process adapter must emit byte-for-byte today's argv**, and those two suites are the check.
- Work in the worktree `/home/paul/.worktrees/niri-tasks/task-session-module-0cb3900d` (branch `task/session-module-0cb3900d`); `finish-worktree` lands it and marks the task completed.
- **Never run `install.sh`, `cargo install` or `systemctl --user restart niri-tasks`.** Do not run `tests/e2e-tag.sh` (it needs a live niri with two workspaces); do not start herdr sessions.
- One intended behaviour change (spec, Q21): Refine's prompt is confirmed and resent on a stall, and a blocked refiner start is waited out, as Start working's already are. Nothing else the user sees changes. `task start --here --workspace <name>` keeps its shape.
- `refine::launch` keeps its fence (sockets, mod, settings) before anything opens (ADR 0002).
- Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Docs on every pub item; comments are full sentences that say why. Line numbers are anchors from `main` at 9b6e741; match on quoted text.

---

### Task 1: The port and the process adapter

**Files:**
- Move: `src/herdr.rs` → `src/session/herdr.rs` (`git mv`), then edit
- Modify: `src/session.rs` (add the port types and `pub(crate) mod herdr;`), `src/lib.rs` (remove `pub mod herdr;`), `src/refine.rs`, `src/work.rs`, `src/link.rs` (their `use crate::{herdr, …}` lines become `use crate::session::herdr;` plus the rest)

**Interfaces:**
- Produces, in `src/session.rs`: `HerdrError`, `HerdrResult<T>`, `Workspace`, `Created`, `Opened`, `Agent`, `WindowInfo`, `Claude`, `pub(crate) trait Port`; in `src/session/herdr.rs`: `pub(crate) struct Process;` implementing `Port`, with today's `pub fn` builders and readers kept (they are what the adapter and, until Task 3, the callers use).

- [ ] **Step 1: Move the file and fix the paths**

```bash
mkdir -p src/session && git mv src/herdr.rs src/session/herdr.rs
```

In `src/lib.rs` delete `pub mod herdr;`. In `src/session.rs` add, after the module doc, `pub(crate) mod herdr;`. In `src/refine.rs`, `src/work.rs` and `src/link.rs` change the `use crate::{herdr, …}` imports to import `herdr` from `crate::session::herdr` (e.g. `use crate::session::herdr;` as its own line and drop `herdr` from the braces). `cargo build` must pass before going on.

- [ ] **Step 2: The port's types and trait, in `session.rs`**

Add to `src/session.rs` (after the existing naming rules, before the tests):

```rust
use anyhow::Result;
use std::time::Duration;

/// A failure herdr itself reported: its JSON error's code and message. The
/// message is what the user sees; the code is what a caller acts on
/// (`agent_not_found`, `agent_prompt_stalled`, `timeout`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HerdrError {
    pub code: Option<String>,
    pub message: String,
}

impl std::fmt::Display for HerdrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HerdrError {}

/// What a herdr call comes back with: herdr's own refusal as `Err(HerdrError)`
/// inside an `Ok`, and "could not run herdr at all" as the outer `Err`, so a
/// missing herdr is never mistaken for "no".
pub type HerdrResult<T> = Result<std::result::Result<T, HerdrError>>;

/// One of a session's herdr workspaces, as `workspace list` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub label: String,
}

/// What `tab create` or `workspace create` made: the pane to start Claude in,
/// and the tab, when herdr said which, to close again if that fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub pane: String,
    pub tab: Option<String>,
}

/// What `worktree open` opened, or found open: the workspace, and its root pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub workspace: String,
    pub pane: String,
}

/// An agent as `agent get` reports it: its name, None for one nobody named,
/// and its status (`idle`, `working`, `blocked`, `done`, `unknown`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub name: Option<String>,
    pub status: Option<String>,
}

/// A niri window's id, app id and title: what finding the session's terminal
/// window needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: u64,
    pub app_id: Option<String>,
    pub title: Option<String>,
}

/// Which Claude `agent start` runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claude {
    /// Refine's: fenced by the settings JSON and the refine mod it loads.
    Refiner { settings: String, mod_dir: std::path::PathBuf },
    /// Start working's: the user's own defaults, since it is meant to do the work.
    Worker,
}

/// Everything the session module does outside itself: herdr, every call
/// naming the session; niri's window list, focus and spawn; time; and the
/// user's notifications. Two adapters make the seam real: [`herdr::Process`]
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
    /// `confirm` waits for herdr to see Claude start on the prompt, failing
    /// with `agent_prompt_stalled` when it does not.
    fn agent_prompt(&self, session: &str, name: &str, text: &str, confirm: bool) -> HerdrResult<()>;
    fn windows(&self) -> Result<Vec<WindowInfo>>;
    fn focus_window(&self, id: u64) -> Result<()>;
    fn spawn(&self, command: Vec<String>) -> Result<()>;
    fn sleep(&self, d: Duration);
    fn notify(&self, text: &str);
    /// Whether herdr is on `$PATH` at all, for the one place that starts a
    /// terminal to run it.
    fn herdr_installed(&self) -> bool;
}
```

(`session.rs` already imports `std::path::Path`; add `PathBuf` where the enum needs it.)

- [ ] **Step 3: `Process` in `session/herdr.rs`**

Add to `src/session/herdr.rs`, after the readers:

```rust
use super::{Agent, Claude, Created, HerdrError, HerdrResult, Opened, Port, WindowInfo, Workspace};

/// Every workspace in a `workspace list` response, id and label, in herdr's order.
pub fn workspaces(list: &Value) -> Vec<Workspace> {
    list["result"]["workspaces"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|w| {
            Some(Workspace { id: w["workspace_id"].as_str()?.to_string(), label: w["label"].as_str()?.to_string() })
        })
        .collect()
}

/// The adapter that runs herdr, asks niri, sleeps and notifies for real.
pub(crate) struct Process;

impl Process {
    /// Run `argv` and read its JSON with `read`; herdr's refusal comes back as
    /// the inner `Err`, and only failing to run herdr is the outer one.
    fn call<T>(&self, argv: &[String], read: impl FnOnce(&Value) -> Result<T>) -> HerdrResult<T> {
        match run_coded(argv)? {
            Ok(value) => Ok(Ok(read(&value)?)),
            Err((code, message)) => Ok(Err(HerdrError { code, message })),
        }
    }
}

impl Port for Process {
    fn workspace_list(&self, s: &str) -> HerdrResult<Vec<Workspace>> {
        self.call(&workspace_list(s), |v| Ok(workspaces(v)))
    }
    fn workspace_create(&self, s: &str, dir: &Path, label: &str) -> HerdrResult<Created> {
        self.call(&workspace_create(s, dir, label), created)
    }
    fn tab_create(&self, s: &str, workspace: &str, dir: &Path, label: &str) -> HerdrResult<Created> {
        self.call(&tab_create(s, workspace, dir, label), created)
    }
    fn tab_close(&self, s: &str, tab: &str) -> HerdrResult<()> {
        self.call(&tab_close(s, tab), |_| Ok(()))
    }
    fn worktree_open(&self, s: &str, repo: &Path, path: &Path, label: &str) -> HerdrResult<Opened> {
        self.call(&worktree_open(s, repo, path, label), |v| {
            Ok(Opened {
                workspace: opened_workspace_id(v).context("herdr did not say which workspace it opened")?,
                pane: root_pane_id(v).context("herdr did not say which pane it opened")?,
            })
        })
    }
    fn pane_run(&self, s: &str, pane: &str, command: &str) -> HerdrResult<()> {
        self.call(&pane_run(s, pane, command), |_| Ok(()))
    }
    fn agent_get(&self, s: &str, target: &str) -> HerdrResult<Agent> {
        self.call(&agent_get(s, target), |v| Ok(Agent { name: agent_name_of(v), status: agent_status(v) }))
    }
    fn agent_list(&self, s: &str) -> HerdrResult<Vec<String>> {
        self.call(&agent_list(s), |v| Ok(agent_names(v)))
    }
    fn agent_focus(&self, s: &str, name: &str) -> HerdrResult<()> {
        self.call(&agent_focus(s, name), |_| Ok(()))
    }
    fn agent_rename(&self, s: &str, target: &str, name: &str) -> HerdrResult<()> {
        self.call(&agent_rename(s, target, name), |_| Ok(()))
    }
    fn agent_start(&self, s: &str, name: &str, pane: &str, claude: &Claude) -> HerdrResult<()> {
        let argv = match claude {
            Claude::Refiner { settings, mod_dir } => agent_start_claude_refiner(s, name, pane, settings, mod_dir),
            Claude::Worker => agent_start_claude(s, name, pane),
        };
        self.call(&argv, |_| Ok(()))
    }
    fn agent_wait_ready(&self, s: &str, name: &str) -> HerdrResult<()> {
        self.call(&agent_wait_ready(s, name), |_| Ok(()))
    }
    fn agent_prompt(&self, s: &str, name: &str, text: &str, confirm: bool) -> HerdrResult<()> {
        let argv = if confirm { agent_prompt_confirmed(s, name, text) } else { agent_prompt(s, name, text) };
        self.call(&argv, |_| Ok(()))
    }
    fn windows(&self) -> Result<Vec<WindowInfo>> {
        Ok(crate::niri::windows()?
            .into_iter()
            .map(|w| WindowInfo { id: w.id, app_id: w.app_id, title: w.title })
            .collect())
    }
    fn focus_window(&self, id: u64) -> Result<()> {
        crate::niri::focus_window(id)
    }
    fn spawn(&self, command: Vec<String>) -> Result<()> {
        crate::niri::spawn(command)
    }
    fn sleep(&self, d: std::time::Duration) {
        std::thread::sleep(d)
    }
    fn notify(&self, text: &str) {
        crate::notify::tasks(text)
    }
    fn herdr_installed(&self) -> bool {
        crate::project::on_path(crate::project::SESSION_MANAGER)
    }
}

/// `tab create` and `workspace create` both answer with the pane they made
/// and the tab it is in.
fn created(v: &Value) -> Result<Created> {
    Ok(Created { pane: root_pane_id(v).context("herdr did not say which pane it made")?, tab: created_tab_id(v) })
}
```

Check `niri::windows()` returns `Vec<niri_ipc::Window>` with `id`, `app_id: Option<String>`, `title: Option<String>` (read `src/niri.rs`; adapt the field moves if the types differ). The existing `pub fn run` stays for now (Task 3 removes it when the last caller goes). Keep every existing test. Add one test for `workspaces`:

```rust
    #[test]
    fn workspaces_read_id_and_label_in_order() {
        let list: Value = serde_json::from_str(
            r#"{"result":{"workspaces":[{"workspace_id":"w1","label":"alpha"},{"workspace_id":"w2","label":"task/x"}]}}"#,
        )
        .unwrap();
        assert_eq!(
            workspaces(&list),
            vec![
                Workspace { id: "w1".into(), label: "alpha".into() },
                Workspace { id: "w2".into(), label: "task/x".into() },
            ]
        );
        assert!(workspaces(&Value::Null).is_empty());
    }
```

Update `session/herdr.rs`'s module doc: it is the process adapter behind `session::Port`; the builders are pure for the tests; `run_coded` is the one runner. Update `session.rs`'s module doc to say the module also owns opening a session and its agents, behind `Port` (the steps arrive in Task 2).

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4`
Expected: no warnings; all pass, the PATH-shim suites included (nothing they run has changed yet).

- [ ] **Step 4: Commit**

```bash
git add -A src
git commit -m "refactor(session): put herdr behind a typed port with a process adapter

herdr.rs moves under session as the adapter that runs herdr, asks niri,
sleeps and notifies for real, behind one Port trait of typed operations.
Its argv builders and readers stay as the implementation, so the argv is
the same byte for byte; nothing else changes yet. A second adapter, a
fake for the tests, is what makes the seam real and comes next.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

### Task 2: The `Session` value, its steps, `current_pane()` and the fake

**Files:**
- Modify: `src/session.rs` (add `Session`, `CurrentPane`, `current_pane`, `prompt_outcome`, the steps, tests)
- Create: `src/session/fake.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: Task 1's `Port`, types, `herdr::Process`; `crate::project::project_terminal_command(dir, s, true)` (the terminal argv), `start_dir`, `herdr_session_name`, `session_from_env`.
- Produces: `pub struct Session` with `for_workspace`, `with_port` (pub(crate)), `name`, `dir`, `open`, `focus_agent`, `new_tab`, `close_tab`, `open_worktree`, `run_in_pane`, `start_claude`, `prompt`, `agent_names`, `agent`, `rename_agent`; `pub struct CurrentPane { session, pane, tab }`, `pub fn current_pane() -> Option<CurrentPane>`; `pub enum PromptOutcome`, `pub fn prompt_outcome(code: Option<&str>) -> PromptOutcome`; `pub(crate) mod fake` with `Fake`. Callers are not switched until Task 3.

- [ ] **Step 1: Write the fake**

Create `src/session/fake.rs`:

```rust
//! A herdr, a niri, a clock and a notifier that answer from a table: the
//! adapter the session module's tests run against, so every step is checked
//! without a herdr server or a compositor. It models what the module reads:
//! a session running or not, its workspaces, tabs and panes, its agents, the
//! worktrees it has opened, niri's windows, and it logs every call it gets.

use super::{Agent, Claude, Created, HerdrError, HerdrResult, Opened, Port, WindowInfo, Workspace};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How a scripted `agent start` goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartOutcome {
    Ok,
    /// Fails with no agent to show for it.
    Fails,
    /// Fails, but the agent is there and `blocked`: Claude asking a start-up
    /// question. `agent wait` then finds it idle.
    FailsBlockedThenIdle,
}

#[derive(Debug, Clone)]
pub struct FakeAgent {
    pub name: Option<String>,
    pub pane: String,
    pub status: String,
}

#[derive(Debug, Default)]
struct State {
    installed: bool,
    running: bool,
    /// `workspace_list` calls still to come before a stopped session is up;
    /// 0 with `running` false means it never starts.
    boot_polls: u32,
    workspaces: Vec<Workspace>,
    next_id: u32,
    worktrees: HashMap<PathBuf, Opened>,
    agents: Vec<FakeAgent>,
    start: StartOutcome,
    /// One per confirmed `agent prompt` attempt: None delivered, Some(code) failed with it.
    prompt_codes: Vec<Option<String>>,
    windows: Vec<WindowInfo>,
    log: Vec<String>,
}

impl Default for StartOutcome {
    fn default() -> Self {
        StartOutcome::Ok
    }
}

/// The fake port. Build one with [`Fake::running`] or [`Fake::stopped`] and
/// the `with_*` methods, hand it to `Session::with_port`, act, then read
/// [`Fake::log`].
#[derive(Debug, Default)]
pub struct Fake {
    state: RefCell<State>,
}

impl Fake {
    /// herdr installed, the session running with these workspaces.
    pub fn running(workspaces: &[(&str, &str)]) -> Fake {
        let fake = Fake::default();
        {
            let mut s = fake.state.borrow_mut();
            s.installed = true;
            s.running = true;
            s.workspaces = workspaces.iter().map(|(id, label)| Workspace { id: id.to_string(), label: label.to_string() }).collect();
        }
        fake
    }

    /// herdr installed, the session stopped; it starts after `boot_polls`
    /// `workspace_list` calls (0 means never), then has one workspace `w1`
    /// labelled `label`.
    pub fn stopped(boot_polls: u32, label: &str) -> Fake {
        let fake = Fake::default();
        {
            let mut s = fake.state.borrow_mut();
            s.installed = true;
            s.boot_polls = boot_polls;
            s.workspaces = vec![Workspace { id: "w1".into(), label: label.into() }];
        }
        fake
    }

    pub fn not_installed(self) -> Fake {
        self.state.borrow_mut().installed = false;
        self
    }
    pub fn with_agent(self, name: Option<&str>, pane: &str, status: &str) -> Fake {
        self.state.borrow_mut().agents.push(FakeAgent { name: name.map(String::from), pane: pane.into(), status: status.into() });
        self
    }
    pub fn with_window(self, id: u64, app_id: &str, title: &str) -> Fake {
        self.state.borrow_mut().windows.push(WindowInfo { id, app_id: Some(app_id.into()), title: Some(title.into()) });
        self
    }
    pub fn with_start(self, outcome: StartOutcome) -> Fake {
        self.state.borrow_mut().start = outcome;
        self
    }
    pub fn with_prompt_codes(self, codes: &[Option<&str>]) -> Fake {
        self.state.borrow_mut().prompt_codes = codes.iter().map(|c| c.map(String::from)).collect();
        self
    }
    pub fn with_worktree(self, path: &str, workspace: &str, pane: &str) -> Fake {
        self.state.borrow_mut().worktrees.insert(PathBuf::from(path), Opened { workspace: workspace.into(), pane: pane.into() });
        self
    }

    /// Every call so far, one readable line each.
    pub fn log(&self) -> Vec<String> {
        self.state.borrow().log.clone()
    }
    pub fn agents(&self) -> Vec<FakeAgent> {
        self.state.borrow().agents.clone()
    }

    fn note(&self, line: String) {
        self.state.borrow_mut().log.push(line);
    }

    fn refuse(code: &str, message: &str) -> HerdrError {
        HerdrError { code: Some(code.into()), message: message.into() }
    }

    /// Not installed is the outer error; a stopped server is herdr's own refusal.
    fn check(&self) -> HerdrResult<()> {
        let s = self.state.borrow();
        anyhow::ensure!(s.installed, "could not run `herdr` — is herdr installed?");
        if !s.running {
            return Ok(Err(Self::refuse("server_not_running", "server not running")));
        }
        Ok(Ok(()))
    }

    fn mint(&self, prefix: &str) -> String {
        let mut s = self.state.borrow_mut();
        s.next_id += 1;
        format!("{prefix}{}", s.next_id)
    }
}

impl Port for Fake {
    fn workspace_list(&self, session: &str) -> HerdrResult<Vec<Workspace>> {
        self.note(format!("workspace_list {session}"));
        {
            let mut s = self.state.borrow_mut();
            anyhow::ensure!(s.installed, "could not run `herdr` — is herdr installed?");
            if !s.running && s.boot_polls > 0 {
                s.boot_polls -= 1;
                if s.boot_polls == 0 {
                    s.running = true;
                }
            }
        }
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        Ok(Ok(self.state.borrow().workspaces.clone()))
    }
    fn workspace_create(&self, session: &str, dir: &Path, label: &str) -> HerdrResult<Created> {
        self.note(format!("workspace_create {session} {} {label}", dir.display()));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        let id = self.mint("w");
        let pane = self.mint("p");
        let tab = self.mint("t");
        self.state.borrow_mut().workspaces.push(Workspace { id, label: label.into() });
        Ok(Ok(Created { pane, tab: Some(tab) }))
    }
    fn tab_create(&self, session: &str, workspace: &str, dir: &Path, label: &str) -> HerdrResult<Created> {
        self.note(format!("tab_create {session} {workspace} {} {label}", dir.display()));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        Ok(Ok(Created { pane: self.mint("p"), tab: Some(self.mint("t")) }))
    }
    fn tab_close(&self, session: &str, tab: &str) -> HerdrResult<()> {
        self.note(format!("tab_close {session} {tab}"));
        self.check()
    }
    fn worktree_open(&self, session: &str, repo: &Path, path: &Path, label: &str) -> HerdrResult<Opened> {
        self.note(format!("worktree_open {session} {} {} {label}", repo.display(), path.display()));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        let opened = self.state.borrow().worktrees.get(path).cloned();
        Ok(Ok(match opened {
            Some(o) => o,
            None => {
                let o = Opened { workspace: self.mint("w"), pane: self.mint("p") };
                self.state.borrow_mut().worktrees.insert(path.to_path_buf(), o.clone());
                o
            }
        }))
    }
    fn pane_run(&self, session: &str, pane: &str, command: &str) -> HerdrResult<()> {
        self.note(format!("pane_run {session} {pane} {command}"));
        self.check()
    }
    fn agent_get(&self, session: &str, target: &str) -> HerdrResult<Agent> {
        self.note(format!("agent_get {session} {target}"));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        let found = self.state.borrow().agents.iter().find(|a| a.name.as_deref() == Some(target) || a.pane == target).cloned();
        Ok(match found {
            Some(a) => Ok(Agent { name: a.name, status: Some(a.status) }),
            None => Err(Self::refuse("agent_not_found", "agent target not found")),
        })
    }
    fn agent_list(&self, session: &str) -> HerdrResult<Vec<String>> {
        self.note(format!("agent_list {session}"));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        Ok(Ok(self.state.borrow().agents.iter().filter_map(|a| a.name.clone()).collect()))
    }
    fn agent_focus(&self, session: &str, name: &str) -> HerdrResult<()> {
        self.note(format!("agent_focus {session} {name}"));
        self.check()
    }
    fn agent_rename(&self, session: &str, target: &str, name: &str) -> HerdrResult<()> {
        self.note(format!("agent_rename {session} {target} {name}"));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        let mut s = self.state.borrow_mut();
        if s.agents.iter().any(|a| a.name.as_deref() == Some(name)) {
            return Ok(Err(Self::refuse("agent_name_taken", "an agent already has that name")));
        }
        if let Some(a) = s.agents.iter_mut().find(|a| a.pane == target || a.name.as_deref() == Some(target)) {
            a.name = Some(name.into());
        }
        Ok(Ok(()))
    }
    fn agent_start(&self, session: &str, name: &str, pane: &str, claude: &Claude) -> HerdrResult<()> {
        let kind = match claude { Claude::Refiner { .. } => "refiner", Claude::Worker => "worker" };
        self.note(format!("agent_start {session} {name} {pane} {kind}"));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        let outcome = self.state.borrow().start;
        match outcome {
            StartOutcome::Ok => {
                self.state.borrow_mut().agents.push(FakeAgent { name: Some(name.into()), pane: pane.into(), status: "idle".into() });
                Ok(Ok(()))
            }
            StartOutcome::Fails => Ok(Err(Self::refuse("agent_start_failed", "claude exited"))),
            StartOutcome::FailsBlockedThenIdle => {
                self.state.borrow_mut().agents.push(FakeAgent { name: Some(name.into()), pane: pane.into(), status: "blocked".into() });
                Ok(Err(Self::refuse("timeout", "agent did not become ready")))
            }
        }
    }
    fn agent_wait_ready(&self, session: &str, name: &str) -> HerdrResult<()> {
        self.note(format!("agent_wait_ready {session} {name}"));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        if let Some(a) = self.state.borrow_mut().agents.iter_mut().find(|a| a.name.as_deref() == Some(name)) {
            a.status = "idle".into();
        }
        Ok(Ok(()))
    }
    fn agent_prompt(&self, session: &str, name: &str, text: &str, confirm: bool) -> HerdrResult<()> {
        self.note(format!("agent_prompt {session} {name} {text} confirm={confirm}"));
        if let Err(e) = self.check()? { return Ok(Err(e)); }
        if !confirm {
            return Ok(Ok(()));
        }
        let code = {
            let mut s = self.state.borrow_mut();
            if s.prompt_codes.is_empty() { None } else { s.prompt_codes.remove(0) }
        };
        Ok(match code {
            None => Ok(()),
            Some(code) => Err(Self::refuse(&code, "prompt not taken")),
        })
    }
    fn windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        self.note("windows".into());
        Ok(self.state.borrow().windows.clone())
    }
    fn focus_window(&self, id: u64) -> anyhow::Result<()> {
        self.note(format!("focus_window {id}"));
        Ok(())
    }
    fn spawn(&self, command: Vec<String>) -> anyhow::Result<()> {
        self.note(format!("spawn {}", command.join(" ")));
        Ok(())
    }
    fn sleep(&self, d: Duration) {
        self.note(format!("sleep {}ms", d.as_millis()));
    }
    fn notify(&self, text: &str) {
        self.note(format!("notify {text}"));
    }
    fn herdr_installed(&self) -> bool {
        self.state.borrow().installed
    }
}
```

In `src/session.rs` add `#[cfg(test)] pub(crate) mod fake;`.

- [ ] **Step 2: Write the failing Session tests**

Add to `src/session.rs`'s tests module (which exists, with the naming-rule tests):

```rust
    use super::fake::{Fake, StartOutcome};
    use std::path::PathBuf;

    fn session(fake: Fake) -> (Session, std::rc::Rc<Fake>) {
        let fake = std::rc::Rc::new(fake);
        let s = Session::with_port("alpha", PathBuf::from("/home/x/Projects/alpha"), Box::new(ShareFake(fake.clone())));
        (s, fake)
    }

    /// The fake behind an Rc, so the test keeps a handle to read its log after
    /// the Session has taken the Box.
    struct ShareFake(std::rc::Rc<Fake>);
    impl Port for ShareFake {
        // delegate every method to self.0 — one line each
    }

    #[test]
    fn opening_a_stopped_session_starts_the_terminal_and_waits() {
        let (s, f) = session(Fake::stopped(3, "alpha"));
        let ws = s.open().unwrap();
        assert_eq!(ws.len(), 1);
        let log = f.log();
        assert!(log[1].starts_with("spawn "), "{log:?}");  // after the first workspace_list said stopped
        assert_eq!(log.iter().filter(|l| l.starts_with("sleep")).count(), 2, "{log:?}");
    }

    #[test]
    fn a_session_that_never_starts_is_an_error_after_the_deadline() {
        let (s, _f) = session(Fake::stopped(0, "alpha"));
        let err = s.open().unwrap_err().to_string();
        assert!(err.contains("did not start"), "{err}");
    }

    #[test]
    fn a_running_session_with_a_window_is_focused_not_reopened() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_window(7, "com.mitchellh.ghostty", "host: alpha"));
        s.open().unwrap();
        assert!(f.log().contains(&"focus_window 7".to_string()), "{:?}", f.log());
        assert!(!f.log().iter().any(|l| l.starts_with("spawn")));
    }

    #[test]
    fn a_running_session_with_no_window_gets_another_terminal() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        s.open().unwrap();
        assert!(f.log().iter().any(|l| l.starts_with("spawn")), "{:?}", f.log());
    }

    #[test]
    fn a_running_session_with_no_workspace_looks_for_no_window() {
        let (s, f) = session(Fake::running(&[]));
        assert!(s.open().unwrap().is_empty());
        assert!(!f.log().contains(&"windows".to_string()));
    }

    #[test]
    fn focusing_an_agent_says_whether_there_was_one() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_agent(Some("task-abc"), "p1", "idle"));
        assert!(s.focus_agent("task-abc").unwrap());
        assert!(f.log().contains(&"agent_focus alpha task-abc".to_string()));
        assert!(!s.focus_agent("work-abc").unwrap());
        let (s, _) = session(Fake::running(&[]).not_installed());
        assert!(s.focus_agent("task-abc").is_err(), "no herdr is an error, not no");
    }

    #[test]
    fn a_new_tab_goes_in_the_first_workspace_or_makes_one() {
        let (s, f) = session(Fake::running(&[("w1", "alpha"), ("w2", "task/x")]));
        let ws = s.open().unwrap();
        let tab = s.new_tab(&ws, Path::new("/d"), "Refine: x", "alpha").unwrap();
        assert!(f.log().iter().any(|l| l.starts_with("tab_create alpha w1 /d Refine: x")), "{:?}", f.log());
        assert!(tab.tab.is_some());
        let (s, f) = session(Fake::running(&[]));
        s.new_tab(&[], Path::new("/d"), "Refine: x", "alpha").unwrap();
        assert!(f.log().iter().any(|l| l.starts_with("workspace_create alpha /d alpha")), "{:?}", f.log());
    }

    #[test]
    fn a_blocked_start_is_waited_out_and_a_failed_one_is_an_error() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::FailsBlockedThenIdle));
        s.start_claude("work-abc", "p1", &Claude::Worker).unwrap();
        let log = f.log();
        assert!(log.iter().any(|l| l.starts_with("notify ")), "{log:?}");
        assert!(log.contains(&"agent_wait_ready alpha work-abc".to_string()), "{log:?}");
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::Fails));
        assert!(s.start_claude("work-abc", "p1", &Claude::Worker).is_err());
        assert!(!f.log().iter().any(|l| l.starts_with("agent_wait_ready")));
    }

    #[test]
    fn a_prompt_is_resent_on_a_stall_and_given_up_after_four() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("agent_prompt_stalled"), None]));
        s.prompt("work-abc", "/plan").unwrap();
        let log = f.log();
        assert_eq!(log.iter().filter(|l| l.starts_with("agent_prompt ")).count(), 2, "{log:?}");
        assert!(log.contains(&"sleep 2000ms".to_string()));
        let (s, _) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("agent_prompt_stalled"); 4]));
        assert!(s.prompt("work-abc", "/plan").unwrap_err().to_string().contains("not delivered"));
        let (s, _) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("timeout")]));
        s.prompt("work-abc", "/plan").unwrap();
        let (s, _) = session(Fake::running(&[("w1", "alpha")]).with_prompt_codes(&[Some("agent_not_found")]));
        assert!(s.prompt("work-abc", "/plan").is_err());
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
        assert!(f.log().contains(&"agent_rename alpha p2 work-abc".to_string()));
        assert_eq!(f.agents()[0].name.as_deref(), Some("work-abc"));
    }
```

Also move `work.rs`'s three `prompt_outcome` tests here unchanged (they are in `work.rs`'s test module; look for `prompt_outcome`). Fill in `ShareFake`'s delegation (one line per `Port` method, `self.0.method(..)`).

Run: `cargo test --lib session::`
Expected: FAIL to compile (`Session`, `with_port`, `Claude` in scope etc.).

- [ ] **Step 3: Implement the Session**

Add to `src/session.rs`:

```rust
/// How long a just-opened project terminal gets to bring its herdr session up.
const SESSION_WAIT: Duration = Duration::from_secs(10);
const SESSION_POLL: Duration = Duration::from_millis(250);
/// How many times a stalled prompt is sent before giving up, and the pause between.
const PROMPT_TRIES: u32 = 4;
const PROMPT_RETRY: Duration = Duration::from_secs(2);

/// What a confirmed prompt's result means for sending it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptOutcome { Delivered, Resend, Failed }

/// Read a `--wait` prompt's outcome from herdr's error code (`None` when it
/// succeeded): still working past the timeout means it arrived; a stall means
/// Claude never started on it, which is safe to resend; anything else fails.
pub fn prompt_outcome(code: Option<&str>) -> PromptOutcome {
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
    /// `HERDR_SOCKET_PATH`.
    pub session: String,
    pub pane: Option<String>,
    pub tab: Option<String>,
}

/// The pane this process runs in, or None outside herdr and in its unnamed
/// default session. The one reader of herdr's pane variables.
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
    /// The session for `workspace`, through the process adapter.
    pub fn for_workspace(workspace: &str) -> Result<Session> {
        let home = std::env::var("HOME").context("HOME is unset")?;
        Ok(Session {
            name: herdr_session_name(workspace),
            dir: start_dir(Path::new(&home), workspace),
            port: Box::new(herdr::Process),
        })
    }

    /// The same session over any adapter: the tests' way in.
    pub(crate) fn with_port(name: &str, dir: PathBuf, port: Box<dyn Port>) -> Session {
        Session { name: name.to_string(), dir, port }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn dir(&self) -> &Path { &self.dir }

    /// Make the session running and in front of the user, and return its
    /// herdr workspaces: start the project terminal if the session is
    /// stopped (a window then comes with it) and wait for it to answer;
    /// otherwise focus the window showing it, or attach another, since a
    /// running session may have had its window closed while its server kept
    /// going. Shared by everything that opens something in a session.
    pub fn open(&self) -> Result<Vec<Workspace>> {
        match self.port.workspace_list(&self.name)? {
            Ok(workspaces) => {
                self.show_window(&workspaces)?;
                Ok(workspaces)
            }
            Err(_) => {
                anyhow::ensure!(self.port.herdr_installed(), "herdr is not installed.");
                // Through niri, so the window lands on the focused workspace,
                // the one the task belongs to.
                self.port.spawn(crate::project::project_terminal_command(&self.dir, &self.name, true))?;
                self.wait_for_start()
            }
        }
    }

    /// Focus the ghostty window whose title ends in one of the session's
    /// workspace labels, or attach another terminal when niri has none. A
    /// session with no workspace yet has no title to look for; the caller's
    /// `new_tab` creates one with `--focus` itself.
    fn show_window(&self, workspaces: &[Workspace]) -> Result<()> {
        if workspaces.is_empty() {
            return Ok(());
        }
        let labels: Vec<String> = workspaces.iter().map(|w| w.label.clone()).collect();
        let windows = self.port.windows()?;
        match find_session_window(&windows, &labels) {
            Some(id) => self.port.focus_window(id),
            None => self.port.spawn(crate::project::project_terminal_command(&self.dir, &self.name, true)),
        }
    }

    /// Poll until the session answers with a workspace in it. A server that
    /// answers but never gets one is returned as it is at the deadline, and
    /// the caller creates the workspace itself.
    fn wait_for_start(&self) -> Result<Vec<Workspace>> {
        let polls = (SESSION_WAIT.as_millis() / SESSION_POLL.as_millis()) as u32;
        let mut answered = None;
        for _ in 0..polls {
            if let Ok(list) = self.port.workspace_list(&self.name)? {
                if !list.is_empty() {
                    return Ok(list);
                }
                answered = Some(list);
            }
            self.port.sleep(SESSION_POLL);
        }
        match answered {
            Some(list) => Ok(list),
            None => bail!("herdr session {} did not start within {}s.", self.name, SESSION_WAIT.as_secs()),
        }
    }

    /// Whether `name` is a live agent here, and if so bring the user to it.
    /// herdr not runnable is an error, not false.
    pub fn focus_agent(&self, name: &str) -> Result<bool> {
        if self.port.agent_get(&self.name, name)?.is_err() {
            return Ok(false);
        }
        self.port.agent_focus(&self.name, name)?.map_err(anyhow::Error::from)?;
        Ok(true)
    }

    /// A new tab in the session's first workspace, in `dir`, or, with no
    /// workspace yet, a first workspace labelled `workspace_label` (the
    /// project's name, not the tab's). The pane to start Claude in.
    pub fn new_tab(&self, workspaces: &[Workspace], dir: &Path, label: &str, workspace_label: &str) -> Result<Created> {
        let created = match workspaces.first() {
            Some(w) => self.port.tab_create(&self.name, &w.id, dir, label)?,
            None => self.port.workspace_create(&self.name, dir, workspace_label)?,
        };
        created.map_err(anyhow::Error::from)
    }

    /// Close a tab left bare by a failed start, so a retry does not pile
    /// another up beside it. Best effort: a failure here must not hide the
    /// error that led here.
    pub fn close_tab(&self, tab: &str) {
        let _ = self.port.tab_close(&self.name, tab);
    }

    /// Open a worktree as its own herdr workspace, grouped under the repo's.
    pub fn open_worktree(&self, repo: &Path, path: &Path, label: &str) -> Result<Opened> {
        self.port.worktree_open(&self.name, repo, path, label)?.map_err(anyhow::Error::from)
    }

    /// Type `command` into a pane's shell and run it.
    pub fn run_in_pane(&self, pane: &str, command: &str) -> Result<()> {
        self.port.pane_run(&self.name, pane, command)?.map_err(anyhow::Error::from)
    }

    /// Start Claude as `name` in `pane`. A start that fails while the agent
    /// is `blocked` is Claude asking something first, on a new worktree
    /// whether to trust the folder; that answer is the user's, so they are
    /// told and the start waits for it. Any other failure is a failure.
    pub fn start_claude(&self, name: &str, pane: &str, claude: &Claude) -> Result<()> {
        if let Err(e) = self.port.agent_start(&self.name, name, pane, claude)? {
            let blocked = self
                .port
                .agent_get(&self.name, name)?
                .ok()
                .and_then(|a| a.status)
                .is_some_and(|s| s == "blocked");
            if !blocked {
                return Err(e.into());
            }
            eprintln!(
                "Claude is asking something before it starts — most likely whether to trust this \
                 new worktree. Answer it in its tab; this carries on once Claude is ready."
            );
            self.port.notify("Claude needs an answer before it can start — see its tab.");
            self.port.agent_wait_ready(&self.name, name)?.map_err(anyhow::Error::from).context("Claude did not become ready")?;
        }
        Ok(())
    }

    /// Send `text` until herdr sees Claude start on it. Right after a start-up
    /// question is answered, herdr can report Claude idle a moment before it
    /// takes input, and a prompt sent then vanishes without an error; so it is
    /// confirmed, and resent on a stall.
    pub fn prompt(&self, name: &str, text: &str) -> Result<()> {
        for attempt in 1..=PROMPT_TRIES {
            let (code, message) = match self.port.agent_prompt(&self.name, name, text, true)? {
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

    /// The agent in `target`, a pane id or a name, if there is one.
    pub fn agent(&self, target: &str) -> Result<Option<Agent>> {
        Ok(self.port.agent_get(&self.name, target)?.ok())
    }

    pub fn rename_agent(&self, target: &str, name: &str) -> Result<()> {
        self.port.agent_rename(&self.name, target, name)?.map_err(anyhow::Error::from)
    }
}

/// The ghostty window already showing one of the session's `labels`, if niri
/// has one. herdr sets the outer terminal's title to `"{hostname}: {workspace}"`,
/// the label of whichever herdr workspace that client has focused, so the
/// window's title ends in `": <label>"` for one of them. The process tree
/// cannot say, because ghostty runs every window from one process. A user
/// who changes herdr's `window_title` costs themselves an extra attached
/// terminal rather than a focus, which is harmless.
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
```

Note `wait_for_start` polls a fixed `polls` count (40) through the port's `sleep`, so the fake tests it in no time; the deadline behaviour is the same (10s at 250ms). The two existing `find_session_window` tests in `refine.rs` (look for them; they build `WindowInfo` tuples) move here and adapt to the struct. Add `use anyhow::{bail, Context}` as needed.

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test --lib session:: 2>&1 | tail -3`
Expected: no warnings (everything new is pub or used by the fake under cfg(test); if `with_port`/`fake` show as unused outside tests, they are `pub(crate)` and used by the tests, which is fine for `cargo build` only if nothing warns: `cargo build` does not compile tests, so `with_port` would warn as dead code; mark it `#[cfg_attr(not(test), allow(dead_code))]` with a comment saying it is the tests' constructor). All pass. The `prompt_outcome` tests still exist in `work.rs` too at this point; delete them there in Task 3.

- [ ] **Step 4: Commit**

```bash
git add src/session.rs src/session/fake.rs
git commit -m "feat(session): add the Session, its steps and a fake herdr to test them

Opening a session, finding or focusing its agents, opening tabs and
worktrees, starting Claude and delivering a prompt are now steps on one
Session value over the port. The fake adapter models a session's state
so each step is tested without herdr or a compositor, including the
blocked start, the stalled prompt and the stopped session's wait.
Callers switch in the next commit.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

### Task 3: Refine, Start working, Go to session and the pane link on the Session

**Files:**
- Modify: `src/refine.rs` (`launch` :327-375; delete `show_session_window` :281-297, `open_session` :304-317, `wait_for_session` :378-398, `find_session_window` :252-269, `WindowInfo` :247, `SESSION_WAIT`/`SESSION_POLL` :26-27, their tests; imports), `src/work.rs` (`launch` :219-256, `set_up_here` :263-278, `set_up` :280-300, `start_claude` :185-206, `deliver_prompt` :166-181, `prompt_outcome` :154-160, `PROMPT_TRIES`, `PromptOutcome`, their tests; imports), `src/link.rs` (`link_current_pane` :33-52, `live_agent_names` :66-71, `go_to` :83-92; imports), `src/lib.rs` (`herdr_session` :62-68), `src/session/herdr.rs` (delete `run`; `mod herdr` becomes private in `session.rs`)

**Interfaces:**
- Consumes: Task 2's `Session`, `current_pane`, `Claude`.
- Produces: no `herdr::` use outside `src/session/`; `refine.rs`, `work.rs`, `link.rs` import `crate::session::{Session, Claude, current_pane}` as needed.

- [ ] **Step 1: refine.rs**

`launch` becomes (keeping its fence unchanged above the open):

```rust
pub fn launch(workspace: &str, t: &task::Task, mode: Mode) -> Result<()> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let home = Path::new(&home);
    let name = agent_name(&t.uuid);

    // Before anything opens: a refused refine should leave nothing behind.
    let hidden = hidden_paths(home);
    ensure_no_exposed_sockets(&hidden)?;
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let mod_dir = refine_mod_dir(home, xdg.as_deref());
    anyhow::ensure!(
        mod_dir.join(".claude-plugin/plugin.json").is_file(),
        "The refine mod is missing, or its link is broken, at {}. Run install.sh from the niri-tasks repo.",
        mod_dir.display()
    );
    let session = Session::for_workspace(workspace)?;
    let settings = session_settings(session.dir(), &task::data_location()?, &hidden, &t.uuid);

    let workspaces = session.open()?;
    if session.focus_agent(&name)? {
        notify::tasks("Already being refined — switched to its tab.");
        return Ok(());
    }
    let tab = session.new_tab(&workspaces, session.dir(), &tab_label(mode, &t.description), workspace)?;
    let claude = Claude::Refiner { settings, mod_dir };
    if let Err(e) = session.start_claude(&name, &tab.pane, &claude) {
        if let Some(tab) = &tab.tab {
            session.close_tab(tab);
        }
        return Err(e);
    }
    session.prompt(&name, &prompt(&t.uuid, mode))
}
```

Keep the existing doc comments on `launch` (and note in it that the prompt is confirmed as Start working's is). Delete the moved functions, constants and the `WindowInfo` alias; move their tests (the `find_session_window` ones went in Task 2; `tab_label`, `prompt`, `agent_name`, `session_settings` tests stay). Fix imports (`herdr`, `Value`, `Duration`, `Instant` likely unused now).

- [ ] **Step 2: work.rs**

`launch`:

```rust
pub fn launch(workspace: &str, t: &task::Task) -> Result<()> {
    let repo = repo_for(workspace)?;
    anyhow::ensure!(project::on_path("wt"), "worktrunk (wt) is not installed.");
    let session = Session::for_workspace(workspace)?;
    let name = work_agent_name(&t.uuid);

    let workspaces = session.open()?;

    if let Some(wt) = find_task_worktree(&wt_list(&repo)?, &t.uuid) {
        let opened = session.open_worktree(&repo, &wt.path, &wt.branch)?;
        if session.focus_agent(&name)? {
            notify::tasks("Back to its worktree.");
            return Ok(());
        }
        // The worktree outlived its Claude: a fresh one, in a tab of its own
        // so whatever the workspace's first pane is doing is left alone.
        let tab = session.new_tab(&[Workspace { id: opened.workspace, label: wt.branch.clone() }], &wt.path, "Claude", workspace)?;
        return start_working(&session, &name, &tab.pane, &t.uuid);
    }

    let label = format!("Start: {}", short(&t.description));
    let tab = session.new_tab(&workspaces, &repo, &label, workspace)?;
    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    let command = format!(
        "{} task start --here --workspace {} {}",
        sh_quote(&exe.display().to_string()),
        sh_quote(workspace),
        sh_quote(&t.uuid)
    );
    session.run_in_pane(&tab.pane, &command)
}

/// Start the working Claude in `pane`, hand it the task to plan, and mark the
/// task active, as Update status → Active would. Other active tasks on the
/// workspace stay active: they are other worktrees' agents at work.
fn start_working(session: &Session, name: &str, pane: &str, uuid: &str) -> Result<()> {
    session.start_claude(name, pane, &Claude::Worker)?;
    session.prompt(name, &plan_prompt(uuid))?;
    task::set_active(uuid)
}
```

(`new_tab` with a one-element `Workspace` list is how the worktree's own workspace gets the tab; `Workspace` comes from `crate::session`.)

`set_up_here`: the tab close becomes
```rust
            if let Some(tab) = crate::session::current_pane().and_then(|p| p.tab) {
                if let Ok(session) = Session::for_workspace(workspace) {
                    session.close_tab(&tab);
                }
            }
```
`set_up`: `let session = Session::for_workspace(workspace)?; … let opened = session.open_worktree(&repo, &wt.path, &wt.branch)?; start_working(&session, &work_agent_name(uuid), &opened.pane, uuid)`.

Delete `start_claude`, `deliver_prompt`, `prompt_outcome`, `PROMPT_TRIES`, `PromptOutcome` and the `prompt_outcome` tests (moved in Task 2). Fix imports.

- [ ] **Step 3: link.rs and lib.rs**

```rust
pub fn link_current_pane(uuid: &str) -> Result<()> {
    let Some(CurrentPane { session, pane: Some(pane), .. }) = crate::session::current_pane() else {
        return Ok(());
    };
    let session = Session::with_port(&session, PathBuf::new(), Box::new(crate::session::herdr::Process));
```
Stop: `with_port` is `pub(crate)` for tests and `herdr` is private. Add instead to `session.rs` a `pub fn named(session: &str) -> Session` constructor (process adapter, `dir` empty, for a session known by name rather than by workspace, as the pane link has) and use it here:
```rust
    let session = Session::named(&session);
    let Ok(Some(agent)) = session.agent(&pane) else { return Ok(()) };
    let Some(name) = pane_link_name(agent.name.as_deref(), uuid) else { return Ok(()) };
    session
        .rename_agent(&pane, &name)
        .with_context(|| format!("The task is active, but this pane's agent could not be named {name}"))?;
    Ok(())
}

pub fn live_agent_names(workspace: &str) -> Vec<String> {
    Session::for_workspace(workspace).map(|s| s.agent_names()).unwrap_or_default()
}

pub fn go_to(workspace: &str, uuid: &str) -> Result<()> {
    let name = live_agent(workspace, uuid)
        .context("No Claude is working on this task in this workspace's herdr session.")?;
    let session = Session::for_workspace(workspace)?;
    session.open()?;
    anyhow::ensure!(session.focus_agent(&name)?, "The Claude working on this task has gone.");
    Ok(())
}
```
(Today `go_to` runs `agent_focus` straight; `focus_agent` checks first, which only adds the message for an agent that vanished between the list and the focus.) `lib.rs`: `fn herdr_session() -> Option<String> { session::current_pane().map(|p| p.session) }`.

- [ ] **Step 4: Close the module**

In `src/session.rs` make `mod herdr;` private (no `pub(crate)`), delete `herdr::run` and anything in `session/herdr.rs` now unused (`cargo build` says), keep `run_coded`, the builders, the readers and their tests. `grep -rn "herdr::" src | grep -v "^src/session/"` must print nothing but `session/herdr.rs`-internal uses.

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -6`
Expected: no warnings; every suite passes, `typed_task_number` and `link_pane` included (they assert on the argv the adapter emits: `--session alpha agent focus work-…`, `pane run w1:p1 `, `--here --workspace 'alpha' '<uuid>'`, `agent start … --kind claude`, `agent prompt task-… /refine-task <uuid>`). Note: the refine prompt in `typed_task_number.rs:252` is asserted as `agent prompt {refiner} /refine-task {uuid}`; with Q21 the adapter now sends it with `--wait --timeout 15000` after the text, which `contains` still matches. The fake herdr script there answers every unknown call with the same JSON and exit 0, so a confirmed prompt is "delivered" on the first try.

- [ ] **Step 5: Commit**

```bash
git add -A src
git commit -m "refactor: open sessions and their claudes through Session

Refine, Start working, Go to session and the pane link each ran their
own herdr sequence: open or find the session's window, find or focus the
agent, open a tab or create the workspace, start Claude, deliver the
prompt. Those are Session's steps now, written once and tested against
the fake; refine.rs keeps its fence and work.rs its worktrees. Refine's
prompt is confirmed and a blocked refiner start waited out, as Start
working's already were.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

### Task 4: Docs

**Files:**
- Modify: `docs/superpowers/specs/2026-10-03-session-module-design.md` (a note under the title), `CONTEXT.md` (the **Session** entry gains one sentence), `llms.txt` (the `task refine` row: its prompt is now confirmed; check wording), `README.md` (the Refine paragraph under Requirements if it describes the prompt)

- [ ] **Step 1: Edits**

- At the top of `docs/superpowers/specs/2026-10-03-session-module-design.md`, after the date line, add: "Q19 is settled and parts (a) and (b) done by `2026-10-09-session-module-design.md` (task `0cb3900d`). Parts (c) and (d) are still to do."
- CONTEXT.md **Session** entry: append "In the code, `session::Session` opens it and finds or starts the Claude in it; nothing else talks to herdr."
- Read llms.txt's `task refine` and `task start` rows and README's Refine/Start working paragraphs; if either says the prompt is sent once or that a blocked start fails, make it true (both are now confirmed and waited out). If neither mentions it, change nothing and say so.

- [ ] **Step 2: Verify and commit**

Run: `cargo test --bin niritasks 2>&1 | tail -3` (the README/llms tests).

```bash
git add docs/superpowers/specs/2026-10-03-session-module-design.md CONTEXT.md llms.txt README.md
git commit -m "docs: record the session module and what Refine now shares with Start working

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

## Self-review notes

- Spec coverage: the port and adapter → Task 1; the value, the steps, `current_pane`, the fake and the interface tests → Task 2; the callers and closing the module → Task 3; docs → Task 4. Q21 lands in Task 3 (refine uses `start_claude` and `prompt`).
- Type consistency: `Port`'s methods (Task 1) are what `Process` and `Fake` implement and `Session` calls; `Created { pane, tab }` and `Opened { workspace, pane }` are read by Task 3's callers as named; `Session::named` is introduced in Task 3 (add it in `session.rs` with a doc: "A session known by name alone, as a pane knows its own; the folder is unknown and `open` must not be called on it.").
- Intermediate states: after Task 1 callers use `crate::session::herdr::…` (the module is `pub(crate)` then); after Task 2 the Session exists unused by callers (pub, no warning; `with_port` carries the cfg_attr); Task 3 closes the module.
- PATH-shim argv: unchanged by construction (same builders). The refine prompt gains `--wait --timeout 15000`; the one assertion on it uses `contains` and still matches.
