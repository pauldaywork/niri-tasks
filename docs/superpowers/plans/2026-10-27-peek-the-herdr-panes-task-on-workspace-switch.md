# Peek the herdr Pane's Task on Workspace Switch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Going to a niri workspace whose herdr session has a task's pane focused shows that task's card as the peek on the panel's edge, even after a daemon restart and without selecting a pane in herdr first.

**Architecture:** The lookup the herdr plugin does in Python (focused pane → its agent's `work-`/`task-`/`report-<uuid8>` name, else its folder's `task/<slug>-<uuid8>` branch) moves into Rust as `link::focused_task(&Session)`, built on a new `pane list` call in the session port. `niritasks task peek --focused` runs it from a herdr pane, and the plugin's focus hook shrinks to calling that. The daemon's tick, whenever the outputs it sees change (a workspace switch, its first tick, a hotplug), spawns a thread per shown workspace that runs the same function against that workspace's herdr session and hands the answer back to the GTK main thread, which sets the peek exactly as a `peek` IPC request does.

**Tech Stack:** Rust (gtk4/glib, clap, anyhow, serde_json), Python 3 (the herdr plugin, `unittest`), bash (the nested-niri e2e test), herdr 0.9.3, Taskwarrior 3.5.

**Spec:** Taskwarrior task `194352a9-a24c-4e76-9deb-9c62c7ab727b`: its description and notes (`task rc.json.array=on 194352a9 export`). No separate spec file. It builds on the peek from `docs/superpowers/plans/2026-10-26-peek-the-focused-herdr-panes-task.md`.

## Global Constraints

- herdr is the truth: the daemon re-resolves **every** time the outputs change, not only when it remembers no peek for that tag, and a `None` answer clears that tag's peek.
- A focused pane with no task, a uuid8 Taskwarrior does not know (a branch that outlived its task), no focused pane, a session that is not running, or no herdr at all all mean "peek at nothing". None of them is an error or a notification.
- The herdr calls never run on the GTK main thread from the tick: spawn a `std::thread`, and come back with `gtk4::glib::idle_add_once`, as `ipc::listen` already does.
- `niritasks task peek` (all three forms) works only from a pane of a named herdr session (`session::current_pane()` is `Some`); outside one it fails with the existing message. It never falls back to the folder or the focused workspace.
- Out of scope: herdr sessions that are no niri-tasks workspace's (an unnamed workspace or one with no usable tag is skipped); the Mod+Alt+Ctrl+T keyboard panel (unchanged).
- The plugin's actions `open` and `panel` keep their Python lookup (`find_task`); only the focus hook moves to Rust.
- The plugin's focus hook never notifies; failures go to stderr (`herdr plugin log list --plugin task-docs`).
- Vocabulary is CONTEXT.md's: **Peek**, **Task panel**, **Task card**. Avoid "sliver", "handle", "hover card".
- Comments and doc comments follow the surrounding code: full sentences, why over what, the same density.
- Commits are Conventional Commits, subject ≤ 72 characters, body wrapped at 72, ending with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. This task is a `fix`, so its code commits are `fix(…)`; test- or doc-only commits use `test`/`docs`.
- Tests: `cargo test`; `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs`; `NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh` (without `NIRITASKS` it tests the installed binary, not this worktree's).
- Never send a request to the real daemon from this worktree's build: `target/debug/niritasks task peek …` in a real herdr pane reaches the user's running daemon. Check the CLI's refusal with `env -u HERDR_SESSION -u HERDR_SOCKET_PATH`, and everything else in the nested niri of `tests/e2e-panel.sh`.

---

## File Structure

- `src/names.rs` — `uuid8_of_agent`, `uuid8_of_branch`: the two name parsers, beside the builders they invert.
- `src/work.rs` — `branch_in`, moved from `src/status_line.rs` so the resolver can read a pane folder's branch too.
- `src/session.rs` — `Pane` type, `Port::pane_list`, `Session::focused_pane`.
- `src/session/herdr.rs` — `pane_list` argv, `panes` JSON reader, `Process::pane_list`.
- `src/session/fake.rs` — `with_pane`, `pane_list`.
- `src/link.rs` — `focused_uuid8` (tested against the fake) and `focused_task` (adds the git and Taskwarrior lookups).
- `src/main.rs` — `niritasks task peek --focused`.
- `herdr/task-docs/task_docs.py`, `herdr/task-docs/herdr-plugin.toml`, `herdr/task-docs/tests/test_task_docs.py` — the hook runs `--focused`; the Python focused-pane code goes.
- `src/daemon.rs` — `set_peek` (shared by the IPC request and the tick), `workspaces_shown`, `refresh_peeks`, the tick's call.
- `tests/e2e-panel.sh` — a herdr stub and a section that restarts the daemon, switches workspaces and runs `--focused`.
- `README.md`, `CONTEXT.md`, `llms.txt` — the switch, and `--focused`.

---

### Task 1: Confirm the cause

No code and no commit: this checks the diagnosis in the task's notes before anything is built on it.

**Files:** none.

- [ ] **Step 1: Note the time and restart the user's daemon**

```bash
date -Is
systemctl --user restart niri-tasks
```

- [ ] **Step 2: Ask the user to switch away and back**

Ask the user (AskUserQuestion, or a plain question) to go to another niri workspace and back to one whose herdr session has a task's pane focused (a `work-<uuid8>` Claude or a shell on a `task/…` branch), touching nothing inside herdr, and to say whether a card peeks on the panel's right edge.

- [ ] **Step 3: Check that no focus hook ran**

Run, with `<session>` that workspace's herdr session (its name as niri shows it, with anything but `[A-Za-z0-9._-]` as `_`):

```bash
herdr --session <session> plugin log list --plugin task-docs | tail -n 20
```

Expected: no entry after the time from Step 1, and the user saw no card. If a card did show, or the log has a `focused` run after the restart, stop and report to the user: the cause is not what the task says, and the plan below would not fix it.

---

### Task 2: Find the focused pane's task in Rust

**Files:**
- Modify: `src/names.rs` (two parsers and their tests)
- Modify: `src/work.rs` (gains `branch_in`), `src/status_line.rs` (loses it, calls `work::branch_in`)
- Modify: `src/session.rs` (`Pane`, `Port::pane_list`, `Session::focused_pane`)
- Modify: `src/session/herdr.rs` (`pane_list`, `panes`, `Process::pane_list`, tests)
- Modify: `src/session/fake.rs` (`panes` state, `with_pane`, `pane_list`)
- Modify: `src/link.rs` (`focused_uuid8`, `focused_task`, tests)

**Interfaces:**
- Consumes: `Session::agent(&self, target: &str) -> Result<Option<Agent>>`, `task::get(&str) -> Result<Option<Task>>` (both existing).
- Produces:
  - `crate::names::uuid8_of_agent(name: &str) -> Option<String>`
  - `crate::names::uuid8_of_branch(branch: &str) -> Option<String>`
  - `crate::work::branch_in(dir: &Path) -> Option<String>`
  - `crate::session::Pane { pub id: String, pub focused: bool, pub cwd: Option<PathBuf> }`
  - `Session::focused_pane(&self) -> Option<Pane>`
  - `crate::link::focused_uuid8(session: &Session, branch_in: impl Fn(&Path) -> Option<String>) -> Option<String>`
  - `crate::link::focused_task(session: &Session) -> Option<String>` — the full uuid.

- [ ] **Step 1: Write the failing parser tests**

Add to `src/names.rs`'s `mod tests` (it has one; if not, add `#[cfg(test)] mod tests { use super::*; … }` at the end):

```rust
    /// The three agent names a task's Claude can carry give its uuid8;
    /// anything else, or a prefix with no uuid8 after it, gives none.
    #[test]
    fn an_agent_named_for_a_task_gives_its_uuid8() {
        assert_eq!(uuid8_of_agent("work-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_agent("task-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_agent("report-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_agent(&work_agent("1234ABCD-5678")).as_deref(), Some("1234abcd"), "the builder's own");
        for name in ["reviewer", "work-1234abc", "work-1234abcde", "work-1234ABCD", "work-", "xwork-1234abcd"] {
            assert_eq!(uuid8_of_agent(name), None, "{name}");
        }
    }

    /// A `task/<slug>-<uuid8>` branch gives its uuid8; the slug may hold
    /// dashes of its own.
    #[test]
    fn a_task_branch_gives_its_uuid8() {
        assert_eq!(uuid8_of_branch("task/fix-1234abcd").as_deref(), Some("1234abcd"));
        assert_eq!(uuid8_of_branch("task/fix-the-peek-on-1234abcd").as_deref(), Some("1234abcd"));
        for branch in ["main", "task/1234abcd", "task/-1234abcd", "feature/fix-1234abcd", "task/fix-1234abc", "task/fix-1234abcz"] {
            assert_eq!(uuid8_of_branch(branch), None, "{branch}");
        }
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib names::`
Expected: compile error, `cannot find function uuid8_of_agent`.

- [ ] **Step 3: Write the parsers**

In `src/names.rs`, after `is_task_agent`:

```rust
/// Whether `s` is a uuid8 as [`uuid8`] spells one: eight lowercase hex
/// digits.
fn is_uuid8(s: &str) -> bool {
    s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The uuid8 of the task a `task-`, `work-` or `report-<uuid8>` agent is
/// named for, or None for any other name: how the focused pane's task is
/// found from its Claude.
pub fn uuid8_of_agent(name: &str) -> Option<String> {
    let rest = ["task-", "work-", "report-"].iter().find_map(|p| name.strip_prefix(p))?;
    is_uuid8(rest).then(|| rest.to_string())
}

/// The uuid8 of the task a `task/<slug>-<uuid8>` branch is for, as
/// `work::branch_name` spells it, or None for any other branch: how a
/// shell's pane is found to be on a task.
pub fn uuid8_of_branch(branch: &str) -> Option<String> {
    let (slug, uuid8) = branch.strip_prefix("task/")?.rsplit_once('-')?;
    (!slug.is_empty() && is_uuid8(uuid8)).then(|| uuid8.to_string())
}
```

- [ ] **Step 4: Run them to see them pass**

Run: `cargo test --lib names::`
Expected: PASS.

- [ ] **Step 5: Move `branch_in` to `src/work.rs`**

Cut `branch_in` (its doc comment and body) from `src/status_line.rs` and paste it into `src/work.rs` right after `branch_name`, made `pub`:

```rust
/// The branch checked out in `dir`, or None: detached, no repository, or no
/// git.
pub fn branch_in(dir: &Path) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(["branch", "--show-current"]).output().ok()?;
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !branch.is_empty()).then_some(branch)
}
```

In `src/status_line.rs`, change `and_then(branch_in)` to `and_then(work::branch_in)` (it already imports `work`), and drop `use std::process::Command;` if nothing else there uses it (`cargo build` warns if so).

- [ ] **Step 6: Write the failing herdr adapter tests**

In `src/session/herdr.rs`'s `mod tests`, at the end:

```rust
    #[test]
    fn panes_are_listed_per_session() {
        assert_eq!(pane_list("alpha"), vec!["herdr", "--session", "alpha", "pane", "list"]);
    }

    /// Shape from herdr 0.9.3's `pane list`. The folder is the foreground
    /// process's, where a shell has cd'd to, else the one the pane started
    /// in; a pane with no id is left out.
    #[test]
    fn panes_are_read_from_pane_list() {
        let list: Value = serde_json::from_str(
            r#"{"id":"cli:pane:list","result":{"panes":[
                {"agent":"claude","cwd":"/a","focused":false,"foreground_cwd":"/a/sub","pane_id":"w1:p2A","tab_id":"w1:t1H","workspace_id":"w1"},
                {"agent_status":"unknown","cwd":"/b","focused":true,"pane_id":"w2B:p2","tab_id":"w2B:t1","workspace_id":"w2B"},
                {"cwd":"/c","focused":false}
            ]}}"#,
        )
        .unwrap();
        assert_eq!(
            panes(&list),
            vec![
                Pane { id: "w1:p2A".into(), focused: false, cwd: Some("/a/sub".into()) },
                Pane { id: "w2B:p2".into(), focused: true, cwd: Some("/b".into()) },
            ]
        );
        assert!(panes(&Value::Null).is_empty());
    }
```

- [ ] **Step 7: Add `Pane` and `Port::pane_list` in `src/session.rs`**

After the `Agent` struct:

```rust
/// A pane as `pane list` reports it: its id, whether it has the session's
/// focus, and the folder its foreground process is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    /// The id other herdr calls take, such as `agent get`.
    pub id: String,
    /// Whether it is the session's focused pane; one at most is.
    pub focused: bool,
    /// Where a shell in it has cd'd to, else where it started; None when
    /// herdr said neither.
    pub cwd: Option<PathBuf>,
}
```

In `trait Port`, after `pane_report_metadata`:

```rust
    fn pane_list(&self, session: &str) -> HerdrResult<Vec<Pane>>;
```

In `impl Session`, after `agent`:

```rust
    /// The pane with the session's focus, if any; none when the session is
    /// stopped or herdr is not there, since the daemon asks on every
    /// workspace switch and must not fail for it.
    pub fn focused_pane(&self) -> Option<Pane> {
        match self.port.pane_list(&self.name) {
            Ok(Ok(panes)) => panes.into_iter().find(|p| p.focused),
            _ => None,
        }
    }
```

- [ ] **Step 8: Implement the adapter in `src/session/herdr.rs`**

Add `Pane` to the `use super::{…}` list. After `agent_list`:

```rust
/// Every pane in the session, with which one has the focus: how the daemon
/// and `task peek --focused` find the pane the user is on.
pub fn pane_list(session: &str) -> Vec<String> {
    cmd(session, &["pane", "list"])
}
```

After `agent_names`:

```rust
/// Every pane in a `pane list` response, in herdr's order. A pane with no
/// id is left out: nothing else can be asked about it.
pub fn panes(list: &Value) -> Vec<Pane> {
    list["result"]["panes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some(Pane {
                id: p["pane_id"].as_str()?.to_string(),
                focused: p["focused"].as_bool().unwrap_or(false),
                cwd: p["foreground_cwd"].as_str().or(p["cwd"].as_str()).map(PathBuf::from),
            })
        })
        .collect()
}
```

In `impl Port for Process`, after `pane_report_metadata`:

```rust
    fn pane_list(&self, s: &str) -> HerdrResult<Vec<Pane>> {
        self.call(&pane_list(s), |v| Ok(panes(v)))
    }
```

- [ ] **Step 9: Implement the fake in `src/session/fake.rs`**

Add `Pane` to the `use super::{…}` list. In `struct State`, after `agents`:

```rust
    /// The panes `pane list` reports, in order.
    panes: Vec<Pane>,
```

After `with_agent`:

```rust
    /// A pane in the session, focused or not, its foreground in `cwd`.
    pub fn with_pane(self, id: &str, focused: bool, cwd: &str) -> Fake {
        self.state.borrow_mut().panes.push(Pane { id: id.into(), focused, cwd: Some(cwd.into()) });
        self
    }
```

In `impl Port for Fake`, after `pane_report_metadata`:

```rust
    fn pane_list(&self, session: &str) -> HerdrResult<Vec<Pane>> {
        self.note(format!("pane_list {session}"));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        Ok(Ok(self.state.borrow().panes.clone()))
    }
```

Also add "its panes" is already in the module doc ("its workspaces, tabs and panes"); leave it.

- [ ] **Step 10: Run the adapter tests**

Run: `cargo test --lib session::`
Expected: PASS, including the two new herdr tests.

- [ ] **Step 11: Write the failing resolver tests**

In `src/link.rs`'s `mod tests`, at the end:

```rust
    /// The branch reader the tests hand `focused_uuid8`: `branch` for any
    /// folder, recording that it was asked.
    fn on_branch(branch: Option<&str>, asked: &std::cell::Cell<bool>) -> impl Fn(&std::path::Path) -> Option<String> + '_ {
        let branch = branch.map(String::from);
        move |_| {
            asked.set(true);
            branch.clone()
        }
    }

    /// The focused pane's agent, named for a task, wins over its folder's
    /// branch; another pane's agent is not asked.
    #[test]
    fn the_focused_panes_agent_name_gives_its_task() {
        let fake = Fake::running(&[("w1", "alpha")])
            .with_pane("w1:p1", false, "/elsewhere")
            .with_pane("w1:p2", true, "/work")
            .with_agent(Some("work-11111111"), "w1:p1", "idle")
            .with_agent(Some("task-1234abcd"), "w1:p2", "idle");
        let s = Session::with_port("alpha", PathBuf::new(), Box::new(fake.clone()));
        let asked = std::cell::Cell::new(false);
        assert_eq!(focused_uuid8(&s, on_branch(Some("task/x-99999999"), &asked)).as_deref(), Some("1234abcd"));
        assert!(!asked.get(), "the branch is not read when the agent names the task");
        assert_eq!(fake.log(), vec!["pane_list alpha", "agent_get alpha w1:p2"]);
    }

    /// A shell, or a Claude nobody named for a task, is on the task its
    /// folder's branch is for.
    #[test]
    fn a_focused_pane_on_no_tasks_agent_falls_back_to_its_branch() {
        let fake = Fake::running(&[("w1", "alpha")])
            .with_pane("w1:p2", true, "/work")
            .with_agent(Some("reviewer"), "w1:p2", "idle");
        let s = Session::with_port("alpha", PathBuf::new(), Box::new(fake));
        let asked = std::cell::Cell::new(false);
        assert_eq!(focused_uuid8(&s, on_branch(Some("task/fix-1234abcd"), &asked)).as_deref(), Some("1234abcd"));
        let shell = Session::with_port(
            "alpha",
            PathBuf::new(),
            Box::new(Fake::running(&[("w1", "alpha")]).with_pane("w1:p2", true, "/work")),
        );
        assert_eq!(focused_uuid8(&shell, on_branch(Some("task/fix-1234abcd"), &asked)).as_deref(), Some("1234abcd"));
        assert_eq!(focused_uuid8(&shell, on_branch(Some("main"), &asked)), None);
        assert_eq!(focused_uuid8(&shell, on_branch(None, &asked)), None, "no repository");
    }

    /// No focused pane, a stopped session, or no herdr: no task, and no
    /// branch is read.
    #[test]
    fn no_focused_pane_or_no_session_gives_no_task() {
        let asked = std::cell::Cell::new(false);
        let unfocused = Fake::running(&[("w1", "alpha")]).with_pane("w1:p2", false, "/work");
        for fake in [unfocused, Fake::stopped(0, "alpha"), Fake::running(&[("w1", "alpha")]).not_installed()] {
            let s = Session::with_port("alpha", PathBuf::new(), Box::new(fake));
            assert_eq!(focused_uuid8(&s, on_branch(Some("task/fix-1234abcd"), &asked)), None);
        }
        assert!(!asked.get());
    }
```

(`Fake::not_installed` exists and takes `self`; `Fake::stopped(0, …)` is a session that never starts. If `not_installed` turns out to need a different receiver, build it the way the existing fake tests in `src/session.rs` do.)

- [ ] **Step 12: Run them to see them fail**

Run: `cargo test --lib link::`
Expected: compile error, `cannot find function focused_uuid8`.

- [ ] **Step 13: Write the resolver**

In `src/link.rs`, add `use std::path::Path;` to the imports, and after `live_agents_named`:

```rust
/// The uuid8 of the task `session`'s focused pane is on: its agent's name
/// when that is named for a task (`work-`, `task-` or `report-<uuid8>`),
/// else the branch checked out in its folder (`task/<slug>-<uuid8>`), read
/// with `branch_in`. None when the session is not running, no pane has the
/// focus, or the pane is on no task. The daemon and `task peek --focused`
/// both ask this, so herdr's focus hooks and a workspace switch cannot
/// disagree on a pane's task.
pub fn focused_uuid8(session: &Session, branch_in: impl Fn(&Path) -> Option<String>) -> Option<String> {
    let pane = session.focused_pane()?;
    let by_agent = session.agent(&pane.id).ok().flatten().and_then(|a| a.name);
    by_agent
        .and_then(|name| names::uuid8_of_agent(&name))
        .or_else(|| pane.cwd.as_deref().and_then(branch_in).and_then(|b| names::uuid8_of_branch(&b)))
}

/// The full uuid of the task `session`'s focused pane is on, as
/// [`focused_uuid8`] finds it, asking git for the folder's branch. None too
/// for a uuid8 Taskwarrior does not know: a branch that outlived its task.
pub fn focused_task(session: &Session) -> Option<String> {
    let uuid8 = focused_uuid8(session, crate::work::branch_in)?;
    crate::task::get(&uuid8).ok().flatten().map(|t| t.uuid)
}
```

- [ ] **Step 14: Run every test**

Run: `cargo test`
Expected: PASS, with no new warnings from `cargo build` (`focused_task` is unused until Task 3; if the build warns `dead_code`, leave it: Task 3 calls it).

- [ ] **Step 15: Commit**

```bash
git add src/names.rs src/work.rs src/status_line.rs src/session.rs src/session/herdr.rs src/session/fake.rs src/link.rs
git commit -m "$(cat <<'EOF'
fix(link): find the focused herdr pane's task in Rust

The task of a session's focused pane was found only by the task-docs
plugin, in Python, from herdr's focus hooks. The daemon needs the same
answer on a niri workspace switch, so the lookup moves here: the
focused pane from herdr's pane list, then its agent's task name, else
its folder's task branch.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: `task peek --focused`, and the plugin's hook calls it

**Files:**
- Modify: `src/main.rs` (`TaskCommand::Peek`, its handler, `peeking_takes_a_task_or_clear`)
- Modify: `herdr/task-docs/task_docs.py` (module docstring, `focused_action`; `focused_pane` removed)
- Modify: `herdr/task-docs/herdr-plugin.toml` (the `[[events]]` comment)
- Modify: `herdr/task-docs/tests/test_task_docs.py` (`FAKE_HERDR`, `FAKE_NIRITASKS_PEEK`, `TheFocusHook`)
- Modify: `llms.txt` (the `task peek` row and the task-docs line)

**Interfaces:**
- Consumes: `crate::link::focused_task(&Session) -> Option<String>`, `Session::named(&str) -> Session` (Task 2 and existing).
- Produces: the CLI form `niritasks task peek --focused`, which Task 4's e2e section runs.

- [ ] **Step 1: Write the failing CLI parse test**

In `src/main.rs`, replace the body of `peeking_takes_a_task_or_clear` and its doc comment with:

```rust
    /// herdr's focus hooks run `task peek --focused`; a task or `--clear`
    /// peeks by hand. Exactly one of the three.
    #[test]
    fn peeking_takes_a_task_clear_or_focused() {
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "c53b6e3d"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "--clear"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "--focused"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "c53b6e3d", "--clear"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "c53b6e3d", "--focused"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "--clear", "--focused"]).is_err());
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --bin niritasks peeking_takes`
Expected: FAIL on the `--focused` line (unexpected argument).

- [ ] **Step 3: Add `--focused`**

Replace the `Peek` variant in `enum TaskCommand` with:

```rust
    /// Peek at a task's card on the edge of its workspace's task panel, as herdr's focused pane does
    ///
    /// What herdr's task-docs plugin runs, as --focused, when a pane, tab or
    /// workspace is selected, so only from a pane of a named herdr session:
    /// the peek is that session's workspace's. Tucked away, the panel shows
    /// that card alone, whole, with its notes and buttons, sliding out under
    /// the pointer without taking the keyboard. A task not pending on that
    /// workspace shows nothing, as --clear does. Exit 1 with no daemon, an
    /// unknown task, or outside a named herdr session.
    Peek {
        /// The task's uuid, or its first 8 characters, as in a
        /// `task/<slug>-<uuid8>` branch
        #[arg(required_unless_present_any = ["clear", "focused"], conflicts_with_all = ["clear", "focused"])]
        uuid: Option<String>,
        /// Peek at nothing: the focused pane runs no task
        #[arg(long, conflicts_with = "focused")]
        clear: bool,
        /// Peek at the task of the session's focused pane, found from its
        /// agent's name or its folder's branch, as the daemon does on a
        /// workspace switch; at nothing when it is on no task
        #[arg(long)]
        focused: bool,
    },
```

Replace the `TaskCommand::Peek` arm in `task_command` with:

```rust
        // The session's workspace, never the focused one or the folder's:
        // herdr runs the hook in its server, in the plugin's folder, and the
        // user may be looking anywhere. A uuid8 is resolved here, as for the
        // panel; --focused asks the session for its focused pane's task,
        // the same lookup the daemon makes on a workspace switch.
        TaskCommand::Peek { uuid, clear: _, focused } => {
            let pane = session::current_pane().context(
                "task peek is for a pane of a named herdr session, whose workspace it peeks on; this is not one.",
            )?;
            let tag = Workspace::of_session()?.tag().to_string();
            let uuid = if focused {
                link::focused_task(&session::Session::named(&pane.session))
            } else {
                match uuid {
                    Some(u) => Some(task::get(&u)?.context("task not found")?.uuid),
                    None => None,
                }
            };
            ipc::send(&ipc::Request::Peek { tag, uuid })?
        }
```

(`main.rs` is the binary crate and imports `link` and `session` from `niri_tasks`, so no `crate::` prefix.)

- [ ] **Step 4: Run the Rust tests, and check the refusal**

Run: `cargo test && env -u HERDR_SESSION -u HERDR_SOCKET_PATH target/debug/niritasks task peek --focused; echo "exit $?"`
Expected: tests PASS; the command prints `Error: task peek is for a pane of a named herdr session, whose workspace it peeks on; this is not one.` and `exit 1`. Do **not** run it with `HERDR_SESSION` set: that would reach the real daemon.

- [ ] **Step 5: Write the failing plugin tests**

In `herdr/task-docs/tests/test_task_docs.py`:

1. In `FAKE_HERDR`, delete the `pane list` branch (the last two lines of the script) and change its comment to `# herdr: logs each call as a JSON list, and answers \`agent get <target>\` from the FAKE_AGENTS map, exiting 1 for a target it does not hold, as herdr does.`
2. Replace `FAKE_NIRITASKS_PEEK` and its comment with:

```python
# niritasks: appends its arguments as one line per call, and exits
# $FAKE_PEEK_EXIT with an error, as with no daemon.
FAKE_NIRITASKS_PEEK = """#!/bin/sh
echo "$*" >> "$FAKE_NIRITASKS_LOG"
if [ -n "$FAKE_PEEK_EXIT" ]; then
    echo "Error: The niri-tasks daemon is not running" >&2
    exit "$FAKE_PEEK_EXIT"
fi
"""
```

3. In `PluginTest.setUp`, delete `self.panes = …`, `self.with_panes([])` and the `"FAKE_PANES"` env entry; delete the `with_panes` method. (Run `grep -n "with_panes\|FAKE_PANES" herdr/task-docs/tests/test_task_docs.py` afterwards: only `TheFocusHook` used them, and it is replaced next.)
4. Replace the whole `class TheFocusHook` with:

```python
class TheFocusHook(PluginTest):
    """The hook hands the lookup to niritasks, which finds the focused pane's
    task as the daemon does on a workspace switch: the lookup's cases are
    src/link.rs's tests."""

    def setUp(self):
        super().setUp()
        fake(self.bin, "niritasks", FAKE_NIRITASKS_PEEK)
        self.niritasks_log = self.tmp / "niritasks.log"
        patcher = mock.patch.dict(os.environ, {"FAKE_NIRITASKS_LOG": str(self.niritasks_log)})
        patcher.start()
        self.addCleanup(patcher.stop)

    def asked(self):
        return self.niritasks_log.read_text().splitlines() if self.niritasks_log.exists() else []

    def test_it_peeks_at_the_focused_panes_task_through_niritasks(self):
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek --focused"])
        self.assertEqual(self.calls(), [], "niritasks asks herdr itself")
        self.assertEqual(self.notified(), "")

    def test_every_focus_event_peeks_again(self):
        task_docs.focused_action()
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek --focused"] * 2)

    def test_a_failing_niritasks_notifies_nothing(self):
        with mock.patch.dict(os.environ, {"FAKE_PEEK_EXIT": "1"}):
            task_docs.focused_action()
        self.assertEqual(self.notified(), "")
        self.assertEqual(self.asked(), ["task peek --focused"])

    def test_without_niritasks_it_notifies_nothing(self):
        (self.bin / "niritasks").unlink()
        task_docs.focused_action()
        self.assertEqual(self.notified(), "")
```

- [ ] **Step 6: Run them to see them fail**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs`
Expected: FAIL in `TheFocusHook`: the hook still calls `herdr pane list` (which the fake no longer answers), so it peeks at nothing or asks herdr.

- [ ] **Step 7: Reduce the hook to `--focused`**

In `herdr/task-docs/task_docs.py`:

1. Delete `focused_pane()` entirely.
2. Replace `focused_action()` with:

```python
def focused_action():
    """The pane, tab and workspace focus hooks: peek at the session's focused
    pane's task, or at nothing when it runs none. niritasks finds it, from
    the pane's agent name or its folder's branch as `find_task` does, with
    the very lookup the daemon makes on a niri workspace switch, so the two
    cannot disagree. Nothing is remembered here: the daemon ignores a peek
    it already shows."""
    peek("--focused")
```

3. In the module docstring, replace the paragraph that begins `` `task_docs.py focused` is the focus hook`` with:

```
`task_docs.py focused` is the focus hook herdr runs as a pane, a tab or a
workspace is selected: it runs `niritasks task peek --focused`, which finds
the session's focused pane's task the same way and peeks at it, or at
nothing, so that task's card alone peeks out of its workspace's task panel.
The daemon makes the same lookup itself on a niri workspace switch. Every
focus event peeks: the daemon ignores a peek it already shows. A failure goes
to stderr, herdr's plugin log, never to a notification.
```

4. `peek()`'s docstring is unchanged; `sys` is still used by it and `main`.

- [ ] **Step 8: Update the manifest comment**

In `herdr/task-docs/herdr-plugin.toml`, replace the comment above the first `[[events]]` with:

```toml
# Run as a pane, a tab or a workspace is selected: runs `niritasks task peek
# --focused`, which finds the now-focused pane's task as `open` does and peeks
# at it, or at nothing for a pane on no task, so only that task's card peeks
# out of its workspace's task panel. The daemon does the same on a niri
# workspace switch. Failures go to `herdr plugin log list --plugin task-docs`,
# never a notification.
```

- [ ] **Step 9: Run the plugin tests**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs`
Expected: PASS.

- [ ] **Step 10: Update `llms.txt`**

Replace the `task peek` table row (line 27) with:

```
| `niritasks task peek <uuid>` / `--clear` / `--focused` | Peek at that task's card on the edge of its workspace's task panel, at nothing, or at the task of the session's focused pane (its agent's `work-`/`task-`/`report-<uuid8>` name, else its folder's `task/<slug>-<uuid8>` branch; nothing for a pane on no task): tucked away the panel shows that card alone, whole, with notes and buttons, without taking the keyboard. Only from a pane of a named herdr session, whose workspace it peeks on; a task not pending there shows nothing. `--focused` is what herdr's task-docs focus hooks run; the daemon makes the same lookup on a niri workspace switch. Exit 1 with no daemon, an unknown task, or outside a named herdr session | GUI, follows the herdr session |
```

In the task-docs line (line 88), replace `its pane, tab and workspace focus hooks run \`niritasks task peek\` so the focused pane's task is the panel's peek` with `its pane, tab and workspace focus hooks run \`niritasks task peek --focused\` so the focused pane's task is the panel's peek`.

- [ ] **Step 11: Commit**

```bash
git add src/main.rs herdr/task-docs/task_docs.py herdr/task-docs/herdr-plugin.toml herdr/task-docs/tests/test_task_docs.py llms.txt
git commit -m "$(cat <<'EOF'
fix(task-docs): peek through niritasks task peek --focused

The focus hook found the focused pane's task in Python. It now runs
niritasks task peek --focused, which makes the Rust lookup the daemon
will make on a workspace switch, so the two cannot drift.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: The daemon peeks on every workspace switch

**Files:**
- Modify: `src/daemon.rs` (`set_peek`, `workspaces_shown`, `refresh_peeks`, `tick`, the `Request::Peek` arm, `PEEKS` doc, tests)
- Modify: `tests/e2e-panel.sh` (header comment; a new section after "peeking again, and --clear")
- Modify: `README.md` (Peeking section, Commands block, Testing paragraph), `CONTEXT.md` (Peek)

**Interfaces:**
- Consumes: `crate::link::focused_task(&Session) -> Option<String>` (Task 2); `niritasks task peek --focused` (Task 3, in the e2e).
- Produces: nothing later tasks use.

- [ ] **Step 1: Write the failing unit test**

In `src/daemon.rs`'s `mod tests`, after `a_peek_is_kept_per_workspace_tag`:

```rust
    /// A switch re-asks each workspace on a monitor once: two monitors on
    /// one workspace share the lookup, and a monitor on an unnamed one has
    /// no session to ask.
    #[test]
    fn a_switch_asks_each_shown_workspace_once() {
        let outputs: BTreeMap<String, Option<String>> = [
            ("DP-1".to_string(), Some("web".to_string())),
            ("DP-2".to_string(), Some("web".to_string())),
            ("eDP-1".to_string(), Some("api".to_string())),
            ("HDMI-A-1".to_string(), None),
        ]
        .into_iter()
        .collect();
        assert_eq!(workspaces_shown(&outputs), vec!["api", "web"]);
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --lib a_switch_asks`
Expected: compile error, `cannot find function workspaces_shown`.

- [ ] **Step 3: Share the peek setter**

In `src/daemon.rs`, after `remember_peek`, add:

```rust
/// Peek at `uuid` on the workspace with this tag, or at nothing: kept for
/// the tag, and shown at once on every monitor showing it. A monitor that
/// shows it later is handed it by the tick. What a `peek` request and a
/// workspace switch both end in.
fn set_peek(tag: &str, uuid: Option<&str>) {
    PEEKS.with(|p| remember_peek(&mut p.borrow_mut(), tag, uuid));
    let showing: Vec<Rc<Panel>> =
        PANELS.with(|p| p.borrow().values().filter(|panel| panel.shows(tag)).cloned().collect());
    for panel in showing {
        panel.peek(uuid);
    }
}
```

and replace the whole `Request::Peek { tag, uuid } => { … }` arm in `serve_request` with:

```rust
        Request::Peek { tag, uuid } => set_peek(&tag, uuid.as_deref()),
```

- [ ] **Step 4: Ask herdr on a switch**

After `set_peek`, add:

```rust
/// The named workspaces the monitors show, each once, in name order.
fn workspaces_shown(outputs: &BTreeMap<String, Option<String>>) -> Vec<String> {
    let names: std::collections::BTreeSet<&String> = outputs.values().flatten().collect();
    names.into_iter().cloned().collect()
}

/// Ask each shown workspace's herdr session which task its focused pane is
/// on, and peek at that, or at nothing. herdr's focus hooks fire only on a
/// focus change inside herdr, so without this a restarted daemon, a session
/// never re-focused, or a pane that became a task's after it got the focus
/// would peek at nothing; and asked every time, a stale peek never lingers.
/// herdr, git and Taskwarrior are subprocesses, so each workspace is asked
/// on a thread of its own and the answer set back on the main thread.
fn refresh_peeks(outputs: &BTreeMap<String, Option<String>>) {
    for name in workspaces_shown(outputs) {
        // A workspace with no usable tag has no session of ours to ask.
        let Ok(ws) = Workspace::named(&name) else { continue };
        std::thread::spawn(move || {
            let uuid = crate::link::focused_task(&crate::session::Session::named(&ws.session_name()));
            let tag = ws.tag().to_string();
            debug(&format!("focused pane of +{tag}: {uuid:?}"));
            gtk4::glib::idle_add_once(move || set_peek(&tag, uuid.as_deref()));
        });
    }
}
```

In `tick`, replace the block

```rust
    {
        let s = state.borrow();
        if s.outputs.as_ref() == Some(&outputs) && s.task_mtimes == mtimes {
            return; // nothing moved; skip the subprocess
        }
    }
```

with

```rust
    // A workspace switch, the first tick, or a hotplug (which forgets the
    // outputs): what herdr's focused panes say is asked again.
    let switched = state.borrow().outputs.as_ref() != Some(&outputs);
    if !switched && state.borrow().task_mtimes == mtimes {
        return; // nothing moved; skip the subprocess
    }
```

and, just before `let mut s = state.borrow_mut();` at the end of `tick`, add:

```rust
    if switched {
        refresh_peeks(&outputs);
    }
```

Update `PEEKS`' doc comment to:

```rust
    /// The task each workspace tag peeks at, by uuid: herdr's focused pane's,
    /// from `niritasks task peek` and from each workspace switch. Kept per
    /// tag rather than per panel, so a workspace on no monitor yet peeks at
    /// it once a monitor shows it.
```

- [ ] **Step 5: Run every Rust test**

Run: `cargo test && cargo build`
Expected: PASS, no warnings.

- [ ] **Step 6: Add the e2e section**

In `tests/e2e-panel.sh`, in the header comment, replace `The peek itself it sets with` / `` `task peek`, as herdr's task-docs plugin does from a focused pane.`` with:

```
# out, the slide back, clicks on the peek's buttons and clicks passing beside
# it are checked by hand (README, "Testing"). The peek itself it sets with
# `task peek`, as herdr's task-docs plugin does from a focused pane, and,
# against a herdr stub, by restarting the daemon and switching workspaces.
```

(keep the line before it as is; only the last sentence changes.)

Then insert this section right after the `--clear` check that ends `bad "after --clear the edge still draws columns …"` / `fi`, before `# ─── the keyboard: every card wrapped, …`:

```bash
# ─── a workspace switch peeks at herdr's focused pane ────────────────────────
# herdr's focus hooks fire only on a focus change inside herdr, so the daemon
# asks each shown workspace's session itself, on its first tick and on every
# switch. A herdr stub, on this daemon's PATH alone, plays the e2e session:
# one focused pane, whose agent is named in $SB/herdr-focus, no agent when
# that file is empty, and no running server when it is missing. The nested
# niri's spawns never see it, and nothing reaches your real herdr.
mkdir -p "$SB/herdr-stub"
cat > "$SB/herdr-stub/herdr" <<STUB
#!/bin/sh
echo "\$*" >> "$SB/herdr-stub.log"
focus=\$(cat "$SB/herdr-focus" 2>/dev/null) || {
    echo '{"error":{"code":"server_not_running","message":"server not running"}}' >&2; exit 1; }
[ "\$1" = --session ] && shift 2
case "\$1 \$2" in
    "pane list") echo '{"result":{"panes":[{"pane_id":"w1:p1","focused":true,"cwd":"/","foreground_cwd":"/"}]}}' ;;
    "agent get")
        if [ -n "\$focus" ] && [ "\$3" = w1:p1 ]; then
            echo "{\"result\":{\"agent\":{\"name\":\"\$focus\",\"pane_id\":\"w1:p1\"}}}"
        else
            echo '{"error":{"code":"agent_not_found","message":"agent target not found"}}' >&2; exit 1
        fi ;;
    "agent list") echo '{"result":{"agents":[]}}' ;;
    *) echo '{"error":{"code":"unknown","message":"not in the stub"}}' >&2; exit 1 ;;
esac
STUB
chmod +x "$SB/herdr-stub/herdr"
STUB_PATH="$SB/herdr-stub:$("${NENV[@]}" printenv PATH)"
ship=$(uuid_of 'ship it')
focus() { printf '%s' "$1" > "$SB/herdr-focus"; }  # agent name, or '' for none
switch() {  # away to the empty workspace below, and back
    "${NENV[@]}" niri msg action focus-workspace-down >/dev/null 2>&1
    settle
    "${NENV[@]}" niri msg action focus-workspace e2e >/dev/null 2>&1
    settle
}
check() {  # label frame-to-match message
    shot "$1" || { summary; exit 1; }
    if same "$2" "$1"; then
        ok "$3"
    else
        read -r x0 x1 y0 y1 < <(measure "$1" "$2")
        bad "$3 — not so: columns ${x0}-${x1}, rows ${y0}-${y1} differ from $2"
    fi
}

focus "work-${ship:0:8}"
nested_daemon_stop
"${NENV[@]}" PATH="$STUB_PATH" "$NIRITASKS" daemon >"$SB/daemon-herdr.err" 2>&1 &
DAEMON=$!
sleep 3
kill -0 "$DAEMON" 2>/dev/null || die "the daemon with the herdr stub exited at once:
$(tail -n 20 "$SB/daemon-herdr.err")"
settle
check restarted one "a restarted daemon peeks at the task of herdr's focused pane"
if grep -qx -- "--session e2e pane list" "$SB/herdr-stub.log"; then
    ok "the daemon asks the workspace's own herdr session for its focused pane"
else
    bad "the daemon never ran herdr --session e2e pane list: $(tr '\n' ';' < "$SB/herdr-stub.log")"
fi

focus ""
switch
check switched_none baseline "after a switch, a focused pane on no task leaves the edge empty"

focus "work-${ship:0:8}"
switch
check switched_back one "a switch peeks again at the focused pane's task"

rm -f "$SB/herdr-focus"
switch
check switched_dead baseline "a herdr session that is not running leaves the edge empty"

# --focused is what herdr's focus hooks run: the same lookup, from the pane.
focus "work-${ship:0:8}"
"${NENV[@]}" PATH="$STUB_PATH" HERDR_SESSION=e2e "$NIRITASKS" task peek --focused >/dev/null 2>&1 ||
    bad "task peek --focused failed"
settle
check focused_cli one "task peek --focused peeks at the focused pane's task"
focus ""
"${NENV[@]}" PATH="$STUB_PATH" HERDR_SESSION=e2e "$NIRITASKS" task peek --focused >/dev/null 2>&1 ||
    bad "task peek --focused on no task failed"
settle
check focused_none baseline "task peek --focused on a pane with no task leaves the edge empty"

# Back to a daemon with no herdr at all, its edge empty, for the sections
# below.
nested_daemon_stop
nested_daemon_start "$SB/daemon-back.err"
settle
```

Before running, check two things the section relies on and fix the section if either is off:
- `measure <label> [against]` compares against `baseline` by default, so `measure "$1" "$2"` is right.
- `"${NENV[@]}" niri msg action focus-workspace-down` reaches an empty workspace (niri always keeps one below the last). If `switch` turns out not to change the outputs (the daemon's debug line in `$SB/daemon-herdr.err` with `NIRITASKS_DEBUG=1` added to that daemon's env shows `changed: outputs=…`), use `focus-workspace 2` / `focus-workspace e2e` instead.

- [ ] **Step 7: Build and run the e2e test**

Run: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`
Expected: every `ok`, including the seven new ones; no `bad`. Keep off the nested niri's workspace while it runs (the header says why). If `a restarted daemon peeks…` fails, look in `$SB/herdr-stub.log` for what the daemon asked and in `$SB/daemon-herdr.err` for errors (the script prints where `$SB` is).

- [ ] **Step 8: Update the docs**

`CONTEXT.md`, the **Peek** entry: replace `herdr's task-docs plugin sets it with \`niritasks task peek\` as` / `a pane, a tab or a workspace is selected, and the daemon keeps it per` / `workspace tag.` with:

```
herdr's task-docs plugin sets it with `niritasks task peek --focused` as a
pane, a tab or a workspace is selected, and the daemon asks the same of
each shown workspace's herdr session itself whenever a monitor switches
workspace, and when it starts; it keeps it per workspace tag.
```

and replace `Nothing shows when that pane runs no task, the task is not` / `pending on that workspace, or nothing has been peeked.` with `Nothing shows when that pane runs no task, the session is not running, or the task is not pending on that workspace.` (re-wrap the paragraph at the file's width).

`README.md`, "Peeking at the focused pane's task": replace the paragraph from `The plugin also hooks herdr's` to `why.` with:

```
The plugin also hooks herdr's `pane.focused`, `tab.focused` and
`workspace.focused` events; no key to bind. Selecting a pane, a tab or a
workspace finds the now-focused pane's task, as Task docs does, and that
task's card becomes the peek on its workspace's task panel: the one card the
edge shows. Going to a niri workspace does the same without touching herdr:
the daemon asks that workspace's herdr session for its focused pane each time
a monitor switches to it, and when the daemon starts, so the peek is right
after a restart too. A pane with no task, or a session that is not running,
empties the edge, and so does the task leaving the list (completed, waiting
or removed). Selecting the same pane again changes nothing on the panel. The
hooks run `niritasks task peek --focused` and need herdr 0.9.3. They send no
notification; if the peek does not follow the focus inside herdr,
`herdr plugin log list --plugin task-docs` says why.
```

In the Commands block, after the `niritasks task peek --clear` line, add:

```
niritasks task peek --focused      # peek at the task of the session's focused pane (herdr's focus hooks)
```

In the Testing paragraph about `tests/e2e-panel.sh`, after `` `--clear`, `` add `a restarted daemon and a workspace switch peeking at a stub herdr's focused pane, and \`--focused\`,` (re-wrap the lines).

- [ ] **Step 9: Run every test once more**

Run: `cargo test && python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs`
Expected: PASS. (The e2e ran in Step 7; docs changed nothing it checks.)

- [ ] **Step 10: Commit**

```bash
git add src/daemon.rs tests/e2e-panel.sh README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
fix(daemon): peek at herdr's focused pane on a workspace switch

The peek came only from herdr's focus hooks, which fire on a focus
change inside herdr. A restarted daemon, or a session never re-focused,
peeked at nothing. Each time the monitors' workspaces change, the
daemon now asks every shown workspace's herdr session for its focused
pane's task, off the main thread, and peeks at that or at nothing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 11: Check it on the real desktop after landing**

This worktree's daemon cannot be run against the user's herdr. After the branch is finished and installed (`finish-worktree`), ask the user to restart the daemon (`systemctl --user restart niri-tasks`), go to a workspace whose herdr session has a task's pane focused, and confirm its card peeks within a tick; then select a shell on no task inside herdr and confirm the edge empties.
