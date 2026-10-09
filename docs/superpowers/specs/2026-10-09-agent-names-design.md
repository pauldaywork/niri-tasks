# names.rs: a task's agent names and tab labels in one module

Date: 2026-10-09. Taskwarrior task `da9a4cba`. Closes review card D5 (v3/v4).

## Problem

The names a task gets in herdr are spelled in three files: `refine::agent_name` (`task-<uuid8>`, refine.rs:31), `work::work_agent_name` (`work-<uuid8>`, work.rs:73) over `work::uuid8` (work.rs:60), and `link::pane_link_name` testing both prefixes with `starts_with` (link.rs:19) while `link::session_agent` builds both names (link.rs:55). The 30-character tab-label elision is written twice, as `refine::tab_label`'s body (refine.rs:46-58) and `work::short` (work.rs:264-271), each with its own `LABEL_DESCRIPTION_MAX`.

## Decisions

1. **`src/names.rs`** owns them: `uuid8(uuid) -> String` (first eight chars, lowercased), `refine_agent(uuid) -> String` (`task-…`), `work_agent(uuid) -> String` (`work-…`), `both_agents(uuid) -> [String; 2]` (work first, then refine: the order `session_agent` prefers), `is_task_agent(name: &str) -> bool` (named by Refine or Start working, i.e. either prefix), `const LABEL_MAX: usize = 30` and `elide(description) -> String` (collapse whitespace; over the limit, cut to 29 chars plus `…`). Every output byte-identical to today's.
2. **Callers.** `refine.rs` deletes `agent_name` and `LABEL_DESCRIPTION_MAX`; `launch` uses `names::refine_agent`; `tab_label(mode, description)` stays and formats `"{verb}: {}"` with `names::elide`. `work.rs` deletes `uuid8`, `work_agent_name`, `short`, `LABEL_DESCRIPTION_MAX`; `branch_name` uses `names::uuid8`; `launch`/`set_up` use `names::work_agent`; the "Start: …" label uses `names::elide`. `link.rs`: `pane_link_name` uses `names::is_task_agent` and `names::work_agent`; `session_agent` uses `names::both_agents`; the module doc keeps its explanation and points at `names`. `src/panel/state.rs:1133` test uses `crate::names::work_agent`.
3. **Tests.** Move to `names.rs`: `the_agent_is_named_after_its_task` (refine), the agent assertion of `the_branch_and_agent_are_named_after_the_task` (work; the branch assertion stays in work.rs), `long_descriptions_are_elided_in_the_label` becomes an `elide` test (the `é`×40 and `two\n lines` cases), plus new `is_task_agent_knows_both_prefixes` and `both_agents_prefer_the_working_claude`. `link.rs`'s five tests stay (they test the link rules). The PATH-shim suites `tests/typed_task_number.rs` and `tests/link_pane.rs` pin the names end to end and are not edited.
4. **No behaviour change.** Names, labels and the pane-link rule are unchanged.

## Out of scope

`branch_name`/`slug` stay in `work.rs` (they name a worktree, not an agent); the pane-link rule itself; the 30-character limit's value.
