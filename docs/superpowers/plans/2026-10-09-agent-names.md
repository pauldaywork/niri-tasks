# Agent Names Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The `task-`/`work-` agent names, the uuid cut and the tab-label elision are defined once, in `src/names.rs`; refine.rs, work.rs and link.rs call it. No behaviour change.

**Architecture:** see `docs/superpowers/specs/2026-10-09-agent-names-design.md`. Taskwarrior task `da9a4cba`.

## Global Constraints

- Conventional Commits with an attribution trailer (`Co-Authored-By: <your model> <noreply@anthropic.com>`); subject ≤72, body wrapped at 72.
- `cargo build 2>&1 | grep -E "^(warning|error)"` prints nothing; `cargo test` passes in full including `tests/typed_task_number.rs` and `tests/link_pane.rs` (not edited: they pin the names).
- Work in `/home/paul/.worktrees/niri-tasks/task-agent-names-da9a4cba` (branch `task/agent-names-da9a4cba`). **Never run `install.sh`, `cargo install`, `systemctl --user restart niri-tasks`, any `tests/e2e-*.sh`**, or start herdr.
- Docs on every pub item; comments are full sentences that say why. Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Line numbers from main @ 9c5c6cf.

---

### Task 1: `names.rs` and its callers

**Files:** Create `src/names.rs`; modify `src/lib.rs` (`pub mod names;`), `src/refine.rs`, `src/work.rs`, `src/link.rs`, `src/panel/state.rs` (one test line ~:1133).

- [ ] **Write `src/names.rs`** (tests first if you like; they are listed in the spec):

```rust
//! The names a task carries into herdr: its agents, by the first eight
//! characters of its uuid, and the label its tabs show. Refine names its
//! agent `task-<uuid8>` and Start working names its `work-<uuid8>`, so the
//! panel finds a task's agent by asking herdr for those names and nothing is
//! stored on the task. They are spelled here once, so the three callers
//! cannot drift apart.

use crate::text;

/// The first eight characters of a uuid, lowercased — enough to tell one
/// task's branch and agent from another's.
pub fn uuid8(uuid: &str) -> String { uuid.chars().take(8).collect::<String>().to_ascii_lowercase() }

/// The herdr agent name for a task's refine session. One name per task is
/// what lets a second Refine find the first instead of opening another.
pub fn refine_agent(uuid: &str) -> String { format!("task-{}", uuid8(uuid)) }

/// The herdr agent name for a task's working Claude. `work-`, not Refine's
/// `task-`, so a refine still open on the task is never mistaken for it.
pub fn work_agent(uuid: &str) -> String { format!("work-{}", uuid8(uuid)) }

/// Both of a task's agent names, the working Claude first: the one you most
/// likely want back, with a refine still open beside it second.
pub fn both_agents(uuid: &str) -> [String; 2] { [work_agent(uuid), refine_agent(uuid)] }

/// Whether `name` was given by Refine or Start working (or by linking a pane),
/// and so names a task already.
pub fn is_task_agent(name: &str) -> bool { name.starts_with("task-") || name.starts_with("work-") }

/// Longest description, in characters, a tab label carries before eliding:
/// the label shares herdr's sidebar with every other tab's label.
pub const LABEL_MAX: usize = 30;

/// A description cut to fit a tab label, by characters, with an ellipsis;
/// whitespace collapsed first, so a two-line description reads as one.
pub fn elide(description: &str) -> String {
    let d = text::collapse_whitespace(description);
    if d.chars().count() > LABEL_MAX {
        let cut: String = d.chars().take(LABEL_MAX - 1).collect();
        format!("{cut}…")
    } else {
        d
    }
}
```
(Format the one-line fns as the repo does, multi-line bodies.) Copy the existing docs' wording where it exists (refine.rs:28-30, work.rs:57-58, :71-72, link.rs:14-18, :52-53) rather than the sketches above.

Tests in `names.rs`: `uuid8_is_the_first_eight_lowercased`, `the_agents_are_named_after_their_task` (`refine_agent("00DEEEE1-3cbd-…") == "task-00deeee1"`, `work_agent("7CD9FD3A-d27b-…") == "work-7cd9fd3a"`), `both_agents_prefer_the_working_claude`, `is_task_agent_knows_both_prefixes` (`task-x`, `work-x` true; `reviewer`, `` false), `long_descriptions_are_elided` (`elide(&"é".repeat(40))` has 30 chars and ends in `…`; `elide("two\n lines") == "two lines"`; a 30-char description is unchanged).

- [ ] **Switch the callers.** `refine.rs`: delete `agent_name` and `LABEL_DESCRIPTION_MAX`; `launch` → `names::refine_agent(&t.uuid)`; `tab_label` body → `format!("{verb}: {}", names::elide(description))`; delete the moved tests (`the_agent_is_named_after_its_task`, `long_descriptions_are_elided_in_the_label`), keep `the_tab_says_what_it_is_for`. `work.rs`: delete `uuid8`, `work_agent_name`, `short`, `LABEL_DESCRIPTION_MAX`; `branch_name` → `names::uuid8`; call sites → `names::work_agent`, `names::elide`; in `the_branch_and_agent_are_named_after_the_task` drop the agent assertion (rename the test `the_branch_is_named_after_the_task`). `link.rs`: `pane_link_name` → `if current.is_some_and(names::is_task_agent) { return None; } Some(names::work_agent(uuid))`; `session_agent` → `names::both_agents(uuid).into_iter().find(..)`; drop `use crate::{refine, work}` if unused; module doc: keep the explanation, add "spelled in `names`". `panel/state.rs:1133` → `crate::names::work_agent("b")`.
- [ ] Checks: `grep -rn "agent_name\|work_agent_name\|LABEL_DESCRIPTION_MAX\|fn short\b\|fn uuid8" src` prints nothing; `grep -rn '"task-\|"work-' src` prints only `names.rs` (and test literals). `cargo build` warning grep empty; `cargo test` all green including the two PATH-shim suites; `cargo clippy --all-targets` adds nothing.
- [ ] Commit: `refactor(names): spell a task's agent names and tab labels once` with a body: three files spelled the `task-`/`work-` prefixes and two wrote the same 30-character elision; names.rs owns them so the pane link, Refine, Start working and the panel's lookup cannot drift; nothing visible changes.

## Self-review notes

One task: the module and its callers must land together (deleting the old functions and adding the new in one commit keeps the tree building). Decision 3's tests all land here. PATH-shim suites are the end-to-end pin.
