# Refine a Task with Claude — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two new rows in a task's action menu — **Refine** and **Grill me** — open Claude in plan mode in a new tab of the workspace's herdr session; a `refine-task` skill works the task up and writes back a sharper description, replaced notes and a `+planned` tag, which the task panel and picker mark with a clipboard-check icon.

**Architecture:** A pure herdr CLI wrapper (`src/herdr.rs`) builds argv and parses herdr's JSON; `src/refine.rs` orchestrates the launch (ensure session → reuse or create tab → start agent → prompt) and is reached from a new `niritasks task refine <uuid> [--grill]` subcommand the menu calls. The skill is a Markdown file in `.claude/skills/refine-task/`, linked into `~/.claude/skills` by `install.sh`. `Task` learns its `tags`; the panel model and picker rows gain a Planned state.

**Tech Stack:** Rust 2021 (anyhow, clap derive, serde_json), herdr 0.9.1 CLI, Taskwarrior 2.6.2, Claude Code skills (Markdown + YAML frontmatter), fuzzel.

**Spec:** `docs/superpowers/specs/2026-09-28-refine-task-design.md`

## Global Constraints

- The tag is exactly `planned` (never `ready` — Taskwarrior has a virtual `+READY`).
- The planned icon is exactly `"\u{f014e}"` (Material Design clipboard-check in JetBrainsMono Nerd Font).
- Precedence on a card: Active, then Blocked, then Planned, then Pending. In a picker row: `▶ ` (active) wins over the planned mark.
- Urgency and sort order are unchanged by `+planned`.
- The herdr agent name is `task-` + the first 8 characters of the task uuid, lowercased.
- Claude is started with exactly `--permission-mode plan`.
- The prompt is exactly `/refine-task <uuid>` (Refine) or `/refine-task <uuid> grill` (Grill me).
- Tasks are addressed by uuid, never by numeric id.
- Every herdr call names its session with `herdr --session <session>`; `--session` wins over inherited `HERDR_*` variables (verified).
- Errors from `niritasks` reach the user through `main()`'s existing `notify::error` — do not add another reporting path.
- Match the surrounding code's comment style: a doc comment on every public item saying *why*, not just what.

## Precondition

`src/main.rs`, `src/task.rs` and `tests/write_path.rs` carry the user's uncommitted work (the **Update status** picker, `task::stop`, `task::wait`). This plan builds on it. Before Task 1, that work must be committed on its own so later commits do not swallow it — ask the user to commit it, or get their explicit go-ahead to commit it as-is with a message they approve. Do not proceed past this point with it uncommitted.

Baseline: `cargo test` passes (120 unit tests + integration tests) at the start.

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `src/task.rs` | Modify | `Task.tags`, `PLANNED_TAG`, `Task::is_planned()` |
| `src/panel/model.rs` | Modify | `Status::Planned`, its icon, precedence in `cards` |
| `src/panel/surface.rs` | Modify | `planned` CSS class on the card button |
| `src/rows.rs` | Modify | planned marker in picker rows |
| `src/herdr.rs` | Create | herdr argv builders, `run`, JSON field readers |
| `src/refine.rs` | Create | `Mode`, `agent_name`, `prompt`, `tab_label`, `launch` |
| `src/lib.rs` | Modify | `pub mod herdr; pub mod refine;` |
| `src/main.rs` | Modify | `TaskCommand::Refine`, the two menu rows |
| `.claude/skills/refine-task/SKILL.md` | Create | the skill |
| `install.sh` | Modify | link the skill |
| `README.md`, `CONTEXT.md` | Modify | document the rows, the command, the Planned state |

---

### Task 1: `Task` knows its tags

**Files:**
- Modify: `src/task.rs` (struct at lines 16–28, `impl Task` at 36–56, tests module at ~338)
- Modify: `src/rows.rs:58-72` (test fixture), `src/panel/model.rs:98-106` (test fixture)

**Interfaces:**
- Produces: `pub const PLANNED_TAG: &str = "planned";` in `niri_tasks::task`; `Task.tags: Vec<String>`; `Task::is_planned(&self) -> bool`.

- [ ] **Step 1: Write the failing tests**

Add to the existing `#[cfg(test)] mod tests` in `src/task.rs`:

```rust
    /// `task export` leaves `tags` out entirely on an untagged task, so the
    /// field must default rather than fail to parse.
    #[test]
    fn tags_absent_from_export_are_empty() {
        let t: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert!(t.tags.is_empty());
        assert!(!t.is_planned());
    }

    #[test]
    fn the_planned_tag_marks_a_task_planned() {
        let t: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["proj","planned"]}"#).unwrap();
        assert!(t.is_planned());
    }

    /// Tags are case-sensitive, and `+PLANNED` is not ours.
    #[test]
    fn planned_is_matched_exactly() {
        let t: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["PLANNED","planned_x"]}"#).unwrap();
        assert!(!t.is_planned());
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --lib task::tests::tags_absent_from_export_are_empty`
Expected: compile error — no field `tags` / no method `is_planned`.

- [ ] **Step 3: Implement**

In `src/task.rs`, add above `pub struct Task`:

```rust
/// The tag the `refine-task` skill puts on a task once it has been worked up
/// into a plan. Not `ready`: Taskwarrior already has a virtual `+READY`
/// meaning something else, one Shift key away.
pub const PLANNED_TAG: &str = "planned";
```

Add to `pub struct Task`, after `annotations`:

```rust
    /// Absent rather than empty on a task with no tags, hence the default.
    #[serde(default)]
    pub tags: Vec<String>,
```

Add to `impl Task`:

```rust
    /// Worked up into a plan by the `refine-task` skill.
    pub fn is_planned(&self) -> bool {
        self.tags.iter().any(|t| t == PLANNED_TAG)
    }
```

In the fixtures `fn task(...)` in `src/rows.rs` tests and `src/panel/model.rs` tests, add `tags: Vec::new(),` to the `Task { ... }` literal.

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all pass, including the three new ones.

- [ ] **Step 5: Commit**

```bash
git add src/task.rs src/rows.rs src/panel/model.rs
git commit -m "Read a task's tags, and know a planned one"
```

---

### Task 2: Planned tasks on the panel and in the picker

**Files:**
- Modify: `src/panel/model.rs` (enum `Status`, `Card::icon`, `cards`, tests)
- Modify: `src/panel/surface.rs:365-371` (the `match card.status`)
- Modify: `src/rows.rs` (`build`, tests)
- Modify: `CONTEXT.md`, `README.md` (Task panel section)

**Interfaces:**
- Consumes: `Task::is_planned()` from Task 1.
- Produces: `Status::Planned`; picker rows for planned tasks start with `"\u{f014e} "`.

- [ ] **Step 1: Write the failing tests**

In `src/panel/model.rs` tests, add:

```rust
    fn planned(uuid: &str, active: bool) -> Task {
        let mut t = task(uuid, 1.0, active);
        t.tags = vec![crate::task::PLANNED_TAG.into()];
        t
    }

    #[test]
    fn planned_tasks_get_the_clipboard() {
        let got = cards(&[planned("p", false)], &[]);
        assert_eq!(got[0].status, Status::Planned);
        assert_eq!(got[0].icon(), "\u{f014e}");
    }

    #[test]
    fn started_outranks_planned() {
        let got = cards(&[planned("p", true)], &[]);
        assert_eq!(got[0].status, Status::Active);
    }

    /// A planned task that is waiting on another still cannot be started, and
    /// the lock is what says so.
    #[test]
    fn blocked_outranks_planned() {
        let got = cards(&[planned("p", false)], &["p".into()]);
        assert_eq!(got[0].status, Status::Blocked);
    }

    #[test]
    fn planned_does_not_change_the_order() {
        let mut low = planned("low", false);
        low.urgency = 1.0;
        let got = cards(&[low, task("high", 9.0, false)], &[]);
        assert_eq!(texts(&got), vec!["high", "low"]);
    }
```

In `src/rows.rs` tests, add:

```rust
    #[test]
    fn planned_task_gets_the_clipboard_marker() {
        let mut t = task("a", "planned", 1.0, false, 0);
        t.tags = vec![crate::task::PLANNED_TAG.into()];
        let rows = build(&[t]);
        assert_eq!(rows[0].1, "\u{f014e} planned");
        assert_eq!(
            rows[0].1.chars().count(),
            "  planned".chars().count(),
            "the marker keeps the padding's width"
        );
    }

    #[test]
    fn the_active_marker_wins_over_planned() {
        let mut t = task("a", "both", 1.0, true, 0);
        t.tags = vec![crate::task::PLANNED_TAG.into()];
        assert_eq!(build(&[t])[0].1, "▶ both");
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --lib planned`
Expected: compile error — no variant `Status::Planned`; after that, row assertions fail.

- [ ] **Step 3: Implement**

`src/panel/model.rs` — add the variant after `Pending`:

```rust
    /// Worked up into a plan by the `refine-task` skill.
    Planned,
```

In `Card::icon`, after the `Pending` arm:

```rust
            // Material Design's clipboard-check, from the same Nerd Font as the
            // lock: flat and one colour, so it reads as a sibling of it.
            Status::Planned => "\u{f014e}",
```

In `cards`, replace the status expression with:

```rust
            status: if t.is_active() {
                Status::Active
            } else if blocked.contains(&t.uuid) {
                Status::Blocked
            } else if t.is_planned() {
                Status::Planned
            } else {
                Status::Pending
            },
```

and extend the doc comment on `cards` with: `Blocked outranks planned: a plan does not make a task you cannot start yet startable.`

`src/panel/surface.rs` — in `card_button`'s match, add:

```rust
            Status::Planned => button.add_css_class("planned"),
```

(no style rule yet: a planned card is coloured like a pending one; the class is there so it can be styled later.)

`src/rows.rs` — in `build`, replace the `mark` line with:

```rust
            let mark = if t.is_active() {
                "▶ "
            } else if t.is_planned() {
                "\u{f014e} "
            } else {
                "  "
            };
```

and update the doc comment on `build`: `The leading marker flags the active task, or failing that a planned one;`.

`CONTEXT.md` — under `### Scoping`, after **Active task**, add:

```markdown
**Planned task**:
A pending task the `refine-task` skill has worked up into a plan — a sharper
description and a consolidated set of notes — marked with the `+planned` tag
and a clipboard-check icon on its task card and picker row.
_Avoid_: ready (Taskwarrior's `+READY` means something else), refined, groomed
```

`README.md` — in `## Task panel`, after the paragraph that starts "Click a card", add:

```markdown
A card's icon says where the task is: `▶` for the active task, a lock for one
blocked by another, a clipboard-check for a planned one — worked up with Claude
from the menu's **Refine** or **Grill me** — and `○` for the rest.
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/panel/model.rs src/panel/surface.rs src/rows.rs CONTEXT.md README.md
git commit -m "Mark planned tasks with a clipboard-check on their cards and rows"
```

---

### Task 3: herdr CLI wrapper

**Files:**
- Create: `src/herdr.rs`
- Modify: `src/lib.rs` (add `pub mod herdr;` in alphabetical position, after `github`)

**Interfaces:**
- Produces (all in `niri_tasks::herdr`):
  - `pub fn workspace_list(session: &str) -> Vec<String>`
  - `pub fn workspace_create(session: &str, dir: &Path, label: &str) -> Vec<String>`
  - `pub fn tab_create(session: &str, workspace_id: &str, dir: &Path, label: &str) -> Vec<String>`
  - `pub fn agent_get(session: &str, name: &str) -> Vec<String>`
  - `pub fn agent_focus(session: &str, name: &str) -> Vec<String>`
  - `pub fn agent_start_claude_plan(session: &str, name: &str, pane: &str) -> Vec<String>`
  - `pub fn agent_prompt(session: &str, name: &str, text: &str) -> Vec<String>`
  - `pub fn run(argv: &[String]) -> Result<serde_json::Value>`
  - `pub fn error_message(stderr: &[u8]) -> String`
  - `pub fn first_workspace_id(list: &Value) -> Option<String>`
  - `pub fn root_pane_id(created: &Value) -> Option<String>`

- [ ] **Step 1: Write the module with its tests, implementation stubbed**

Create `src/herdr.rs` with the tests below and every function body `todo!()` so it compiles:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn every_call_names_its_session() {
        assert_eq!(workspace_list("alpha"), vec!["herdr", "--session", "alpha", "workspace", "list"]);
    }

    #[test]
    fn a_tab_opens_in_the_project_folder_and_takes_focus() {
        assert_eq!(
            tab_create("alpha", "w1", Path::new("/home/x/Projects/alpha"), "Refine: fix it"),
            vec![
                "herdr", "--session", "alpha", "tab", "create", "--workspace", "w1",
                "--cwd", "/home/x/Projects/alpha", "--label", "Refine: fix it", "--focus",
            ]
        );
    }

    #[test]
    fn a_workspace_is_created_when_the_session_has_none() {
        assert_eq!(
            workspace_create("alpha", Path::new("/p"), "Refine: x"),
            vec!["herdr", "--session", "alpha", "workspace", "create", "--cwd", "/p", "--label", "Refine: x", "--focus"]
        );
    }

    /// Claude's own arguments go after `--`, which is how herdr tells them
    /// from its own.
    #[test]
    fn claude_starts_in_plan_mode() {
        assert_eq!(
            agent_start_claude_plan("alpha", "task-0123abcd", "w1:p3"),
            vec![
                "herdr", "--session", "alpha", "agent", "start", "task-0123abcd",
                "--kind", "claude", "--pane", "w1:p3", "--timeout", "60000",
                "--", "--permission-mode", "plan",
            ]
        );
    }

    #[test]
    fn agent_lookups_and_prompts_go_by_name() {
        assert_eq!(agent_get("a", "task-1"), vec!["herdr", "--session", "a", "agent", "get", "task-1"]);
        assert_eq!(agent_focus("a", "task-1"), vec!["herdr", "--session", "a", "agent", "focus", "task-1"]);
        assert_eq!(
            agent_prompt("a", "task-1", "/refine-task u"),
            vec!["herdr", "--session", "a", "agent", "prompt", "task-1", "/refine-task u"]
        );
    }

    /// Shapes copied from herdr 0.9.1's real responses.
    #[test]
    fn ids_are_read_from_herdr_responses() {
        let list: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"workspace_id":"w1","label":"hansard"}]}}"#,
        ).unwrap();
        assert_eq!(first_workspace_id(&list).as_deref(), Some("w1"));

        let empty: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[]}}"#,
        ).unwrap();
        assert_eq!(first_workspace_id(&empty), None);

        let created: Value = serde_json::from_str(
            r#"{"id":"cli:tab:create","result":{"root_pane":{"pane_id":"w1:p2","tab_id":"w1:t2"},"tab":{"tab_id":"w1:t2"},"type":"tab_created"}}"#,
        ).unwrap();
        assert_eq!(root_pane_id(&created).as_deref(), Some("w1:p2"));
    }

    #[test]
    fn server_errors_are_read_from_the_json_herdr_writes_to_stderr() {
        let stderr = br#"{"id":"cli:agent:get","error":{"code":"agent_not_found","message":"agent target task-nope not found"}}"#;
        assert_eq!(error_message(stderr), "agent target task-nope not found");
        assert_eq!(error_message(b"usage: herdr ...\n"), "usage: herdr ...", "non-JSON passes through");
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --lib herdr::`
Expected: FAIL — panics with `not yet implemented`.

- [ ] **Step 3: Implement**

Replace the stubs in `src/herdr.rs` with:

```rust
//! The herdr CLI, driven from outside any herdr pane.
//!
//! Every call names its session with `--session`, which wins over the
//! `HERDR_*` variables a pane inherits — so this behaves the same from a
//! fuzzel pick as from a terminal inside some other herdr session. The argv
//! builders are pure so they can be tested without a herdr server; [`run`] is
//! the only part that talks to one.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

fn cmd(session: &str, rest: &[&str]) -> Vec<String> {
    ["herdr", "--session", session]
        .iter()
        .chain(rest)
        .map(|s| s.to_string())
        .collect()
}

/// The session's herdr workspaces. Also the liveness check: a stopped session
/// answers `server_not_running`.
pub fn workspace_list(session: &str) -> Vec<String> {
    cmd(session, &["workspace", "list"])
}

/// A first herdr workspace, for a session that has none yet — a fresh server
/// can answer before its client has made one, and `tab create` needs one.
pub fn workspace_create(session: &str, dir: &Path, label: &str) -> Vec<String> {
    let dir = dir.display().to_string();
    cmd(session, &["workspace", "create", "--cwd", &dir, "--label", label, "--focus"])
}

pub fn tab_create(session: &str, workspace_id: &str, dir: &Path, label: &str) -> Vec<String> {
    let dir = dir.display().to_string();
    cmd(
        session,
        &["tab", "create", "--workspace", workspace_id, "--cwd", &dir, "--label", label, "--focus"],
    )
}

pub fn agent_get(session: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "get", name])
}

pub fn agent_focus(session: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "focus", name])
}

/// Claude, in plan mode, as a named agent in `pane`. The timeout is doubled
/// from herdr's 30s default: a cold Claude Code start with hooks and plugins
/// has been seen near 4s, and a slow disk should not turn that into an error.
pub fn agent_start_claude_plan(session: &str, name: &str, pane: &str) -> Vec<String> {
    cmd(
        session,
        &[
            "agent", "start", name, "--kind", "claude", "--pane", pane, "--timeout", "60000",
            "--", "--permission-mode", "plan",
        ],
    )
}

pub fn agent_prompt(session: &str, name: &str, text: &str) -> Vec<String> {
    cmd(session, &["agent", "prompt", name, text])
}

/// Run one herdr command and return its JSON.
///
/// herdr writes server errors as JSON on stderr with exit status 1, so a
/// failure carries herdr's own message rather than a bare status. A success
/// with no JSON on stdout is `Null`, not an error: only the callers that read
/// ids need a body, and they say so when it is missing.
pub fn run(argv: &[String]) -> Result<Value> {
    let out = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .with_context(|| format!("could not run `{}` — is herdr installed?", argv[0]))?;
    if !out.status.success() {
        bail!("{}", error_message(&out.stderr));
    }
    Ok(serde_json::from_slice(&out.stdout).unwrap_or(Value::Null))
}

/// herdr's error message out of its stderr, or the stderr itself when it is
/// not herdr's JSON (a usage error, say).
pub fn error_message(stderr: &[u8]) -> String {
    serde_json::from_slice::<Value>(stderr)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| String::from_utf8_lossy(stderr).trim().to_string())
}

/// The first herdr workspace in a `workspace list` response. A project's
/// session has one, named after the project.
pub fn first_workspace_id(list: &Value) -> Option<String> {
    list["result"]["workspaces"].get(0)?["workspace_id"]
        .as_str()
        .map(str::to_string)
}

/// The new pane in a `tab create` or `workspace create` response.
pub fn root_pane_id(created: &Value) -> Option<String> {
    created["result"]["root_pane"]["pane_id"]
        .as_str()
        .map(str::to_string)
}
```

Add `pub mod herdr;` to `src/lib.rs` after `pub mod github;`.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/herdr.rs src/lib.rs
git commit -m "Wrap the herdr CLI calls a refine needs, testable without a server"
```

---

### Task 4: `niritasks task refine` and the menu rows

**Files:**
- Create: `src/refine.rs`
- Modify: `src/lib.rs` (add `pub mod refine;` after `pub mod project;`)
- Modify: `src/main.rs` (`TaskCommand` enum ~line 66–91, `task_command` match, `task_menu` ~line 336)
- Modify: `README.md` (`## Commands` block)

**Interfaces:**
- Consumes: everything in `niri_tasks::herdr` (Task 3); `session::herdr_session_name`, `session::start_dir`; `project::project_terminal_command`, `project::on_path`, `project::SESSION_MANAGER`; `niri::spawn`; `text::collapse_whitespace`.
- Produces: `niri_tasks::refine::{Mode, agent_name, prompt, tab_label, launch}`:
  - `pub enum Mode { Quick, Grill }`
  - `pub fn agent_name(uuid: &str) -> String`
  - `pub fn prompt(uuid: &str, mode: Mode) -> String`
  - `pub fn tab_label(mode: Mode, description: &str) -> String`
  - `pub fn launch(workspace: &str, uuid: &str, description: &str, mode: Mode) -> anyhow::Result<()>`
  - CLI: `niritasks task refine <uuid> [--grill]`

- [ ] **Step 1: Write the failing tests**

Create `src/refine.rs` containing only the tests below plus `todo!()` stubs for `Mode`, `agent_name`, `prompt`, `tab_label` (signatures above) so it compiles; add `pub mod refine;` to `src/lib.rs`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

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
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --lib refine::`
Expected: FAIL — `not yet implemented`.

- [ ] **Step 3: Implement `src/refine.rs`**

```rust
//! Handing a task to Claude in its workspace's herdr session, to be worked up
//! into a plan by the `refine-task` skill.
//!
//! Only the task's uuid crosses over: the skill reads the task itself, so
//! nothing needs quoting through two CLIs and it always sees the current
//! version rather than the one the menu was opened on.

use crate::{herdr, niri, project, session, text};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Draft straight away; ask only what the code cannot answer.
    Quick,
    /// A full `/grill-me` interview before the draft.
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

pub fn prompt(uuid: &str, mode: Mode) -> String {
    match mode {
        Mode::Quick => format!("/refine-task {uuid}"),
        Mode::Grill => format!("/refine-task {uuid} grill"),
    }
}

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

/// Open Claude on a task in `workspace`'s herdr session.
///
/// Opens the project terminal first if the session is not running, and goes
/// back to the task's existing tab if it is already being refined.
pub fn launch(workspace: &str, uuid: &str, description: &str, mode: Mode) -> Result<()> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let dir = session::start_dir(Path::new(&home), workspace);
    let s = session::herdr_session_name(workspace);
    let name = agent_name(uuid);

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
            wait_for_session(&s)?
        }
    };

    if herdr::run(&herdr::agent_get(&s, &name)).is_ok() {
        herdr::run(&herdr::agent_focus(&s, &name))?;
        return Ok(());
    }

    let label = tab_label(mode, description);
    let created = match herdr::first_workspace_id(&list) {
        Some(id) => herdr::run(&herdr::tab_create(&s, &id, &dir, &label))?,
        None => herdr::run(&herdr::workspace_create(&s, &dir, &label))?,
    };
    let pane = herdr::root_pane_id(&created).context("herdr did not say which pane it made")?;

    herdr::run(&herdr::agent_start_claude_plan(&s, &name, &pane))?;
    herdr::run(&herdr::agent_prompt(&s, &name, &prompt(uuid, mode)))?;
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
```

- [ ] **Step 4: Run the unit tests**

Run: `cargo test --lib refine::`
Expected: PASS.

- [ ] **Step 5: Wire the subcommand and menu into `src/main.rs`**

Add `refine` to the `use niri_tasks::{...}` list.

In `enum TaskCommand`, after `Note`:

```rust
    /// Work a task up into a plan with Claude, in a new tab of the workspace's herdr session
    Refine {
        uuid: String,
        /// Interview first (/grill-me) rather than drafting straight away
        #[arg(long)]
        grill: bool,
    },
```

In `task_command`'s match, before the closing brace:

```rust
        TaskCommand::Refine { uuid, grill } => {
            // The same refusal every entry point makes: a task refined on an
            // unnamed workspace would have no session to open in.
            require_workspace_tag()?;
            let workspace = niri::focused_workspace_name()?.unwrap_or_default();
            let t = task::get(&uuid)?.context("task not found")?;
            let mode = if grill { refine::Mode::Grill } else { refine::Mode::Quick };
            refine::launch(&workspace, &uuid, &t.description, mode)?;
        }
```

In `task_menu`, change `.lines(4)` to `.lines(6)` and the entries to:

```rust
        .run(&[
            "Edit".into(),
            "Note".into(),
            "Refine".into(),
            "Grill me".into(),
            "Update status".into(),
            "Move to workspace".into(),
        ])?;
```

and add arms after `Some("Note")`:

```rust
        // Both open Claude in the workspace's herdr session; they differ only
        // in whether it interviews you before drafting.
        Some("Refine") => return task_command(TaskCommand::Refine { uuid: selected, grill: false }),
        Some("Grill me") => return task_command(TaskCommand::Refine { uuid: selected, grill: true }),
```

In `README.md`'s `## Commands` block, after the `niritasks task note` line:

```
niritasks task refine <uuid>       # work it up into a plan with Claude, in the workspace's herdr session
niritasks task refine <uuid> --grill  #   the same, interviewing you first
```

- [ ] **Step 6: Build and run all tests**

Run: `cargo build && cargo test`
Expected: builds with no warnings; all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/refine.rs src/lib.rs src/main.rs README.md
git commit -m "Open a task in Claude from its menu, as Refine or Grill me"
```

---

### Task 5: The `refine-task` skill

**Files:**
- Create: `.claude/skills/refine-task/SKILL.md`
- Modify: `install.sh` (section 4, after the `workspace-tasks` link)

**Interfaces:**
- Consumes: invoked as `/refine-task <uuid> [grill]` by Task 4's prompt; the `grilling` skill (mattpocock-skills, installed); `PLANNED_TAG` value `planned`.

- [ ] **Step 1: Write the skill**

Create `.claude/skills/refine-task/SKILL.md`:

````markdown
---
name: refine-task
description: >
  Work one Taskwarrior task up into a plan — a sharper one-line description and a
  consolidated set of notes — then write it back and tag it +planned. Invoked by
  niri-tasks' Refine and Grill me menu rows as `/refine-task <uuid> [grill]`.
disable-model-invocation: true
argument-hint: <uuid> [grill]
---

# refine-task: turn a terse task into a plan

Arguments: `$ARGUMENTS` — a task uuid, then optionally `grill`.

Tasks are typed into a one-line box, so they are terse. Your job is to turn one
into something an agent or the user could pick up cold: a description that
still fits a task card, and notes that carry the goal, the decisions and what
"done" means. You are in plan mode; the plan you present **is** the proposed
task. Approving it is what lets you write.

## 1. Read the task

```bash
task rc.json.array=on <uuid> export
```

Stop and say so if it returns `[]` or its `status` is not `pending`. Keep the
`description` and `annotations` you read — step 5 compares against them.
Show the user the description and every note before going further.

## 2. Ground it

Read enough of the project to understand what the task touches: `CONTEXT.md`,
`AGENTS.md`/`CLAUDE.md`, the docs and the code the task names. Facts you can look
up are yours to find; never ask the user for one.

## 3. Work it up

- **No `grill` argument (quick):** draft straight away. Ask only questions the
  code cannot answer and a wrong guess would make the plan wrong — at most
  three, in one round, each with your recommended answer. None is fine.
- **`grill`:** invoke the `grilling` skill with the task (description and notes)
  as the plan to grill, and follow it until its frontier is empty.

## 4. Propose

Present the proposal with ExitPlanMode. The plan contains:

1. **New description** — one line, about 50 characters or fewer: the card and
   picker row show one line at that width. Imperative, specific.
2. **New notes** — each one line, each becoming one annotation, in this order
   where they apply: `Goal: …`, `Context: …`, `Decided: …` (one per decision),
   `Steps: …`, `Done when: …`, `Out of scope: …`. They **replace** the existing
   notes, so fold in everything the old notes said that still holds.
3. **Before** — the current description and notes, so the user can see nothing
   was dropped.

If the user rejects it with feedback, revise and propose again.

## 5. Write

Re-read the task (`task rc.json.array=on <uuid> export`). If its description or
notes differ from what you read in step 1, show the difference and ask before
writing — someone else changed it meanwhile.

Otherwise write the description, the notes and the tag in **one** import, so the
task is never half-updated. Put the approved text in the heredoc — JSON-escaped —
and run:

```bash
task rc.json.array=on <uuid> export | python3 -c '
import datetime, json, sys
task = json.load(sys.stdin)[0]
new = json.loads(sys.argv[1])
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
task["description"] = new["description"]
task["annotations"] = [{"entry": now, "description": n} for n in new["notes"]]
task["tags"] = sorted(set(task.get("tags", [])) | {"planned"})
print(json.dumps(task))
' "$(cat <<'EOF'
{"description": "…", "notes": ["Goal: …", "Done when: …"]}
EOF
)" | task rc.verbose=nothing import
```

Taskwarrior bumps identical note timestamps a second apart itself. Do not touch
the status, start, other tags or any other field.

## 6. Verify and report

Export once more and check: the description matches, the notes are exactly the
new list, `planned` is in `tags`, and the other tags are unchanged. Tell the user
what was written. Leave the session open — they may want to carry on.

Always address the task by uuid, never its numeric id: ids are renumbered as
tasks complete.
````

- [ ] **Step 2: Link it from `install.sh`**

In section 4, after the `workspace-tasks` `link` line, add:

```bash
# refine-task: what the menu's Refine and Grill me rows open Claude with.
link "$REPO/.claude/skills/refine-task/SKILL.md" "$CLAUDE_SKILLS/refine-task/SKILL.md"
```

and change the section header comment to `# ─── 4. the Claude Code skills ─────…` (keep its width).

- [ ] **Step 3: Verify the write command against a sandbox database**

Run the step-5 pipeline with a scratch database (never `~/.task`):

```bash
S=$(mktemp -d) && mkdir "$S/data" && echo "data.location=$S/data" > "$S/rc"
export TASKRC="$S/rc" TASKDATA="$S/data"
task rc.verbose=nothing add terse thing +proj
U=$(task rc.verbose=nothing _uuids)
task rc.verbose=nothing "$U" annotate "old note"
# …paste the step-5 pipeline with <uuid>=$U and notes ["Goal: a","Done when: b"]…
task rc.json.array=on "$U" export
rm -rf "$S"; unset TASKRC TASKDATA
```

Expected: `description` is the new one, `annotations` is exactly the two new notes (no "old note"), `tags` is `["planned","proj"]`.

- [ ] **Step 4: Install and confirm the skill loads**

Run: `./install.sh` then `ls -l ~/.claude/skills/refine-task/SKILL.md`
Expected: a symlink into this repo. In a new `claude` session, `/refine-task` autocompletes.

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md install.sh
git commit -m "Add the refine-task skill the menu opens Claude with"
```

---

### Task 6: End-to-end check

No code, unless a check fails — then fix it in the task that owns the code, with a regression test where one fits.

- [ ] **Step 1:** `./install.sh` (rebuilds and installs `niritasks`), then restart the daemon: `systemctl --user restart niri-tasks`.
- [ ] **Step 2: Refine, session running.** On a named project workspace whose herdr session is open, add a throwaway task, open the picker, pick it, pick **Refine**. Expected: a `Refine: …` tab opens and is focused in the herdr window; Claude shows `plan mode on`; it has read the task and drafts. Approve. Expected: `task <uuid> export` shows the new description and notes and `+planned`; the task card shows the clipboard-check; the picker row starts with it.
- [ ] **Step 3: Permission after approval.** Note whether Claude asked permission to run the `task … import` pipeline. If it did, tell the user and suggest adding `Bash(task:*)` to their Claude Code allow list (do not add it yourself).
- [ ] **Step 4: Refine twice.** Pick **Refine** again on a task whose refine tab is still open. Expected: that tab is focused; no second tab.
- [ ] **Step 5: Grill me.** Pick **Grill me** on another throwaway task. Expected: `Grill: …` tab; Claude interviews in numbered rounds before proposing.
- [ ] **Step 6: Session not running.** `herdr session stop <session>` for a project, close its window, then pick **Refine** from that workspace. Expected: a project terminal opens on the workspace, the session starts, and the refine tab opens in it within ~10s.
- [ ] **Step 7: Clean up** the throwaway tasks (`task <uuid> delete`) and tabs.
- [ ] **Step 8: Run the suites:** `bash tests/all.sh`. Expected: no suite that ran fails.
- [ ] **Step 9:** Report to the user what was verified, what was skipped and why, and the answer from Step 3.
