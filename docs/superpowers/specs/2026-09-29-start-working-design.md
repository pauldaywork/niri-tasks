# Start working on a task in its own worktree — design

Date: 2026-09-29

## Goal

From a task's action menu, **Start working** gives the task its own git
worktree (through worktrunk, so the repo's setup hooks run), opens that
worktree as its own workspace in the project's herdr session, and starts Claude
there with `/superpowers:writing-plans` and the task. Every task gets its own
worktree; picking it again goes back to it.

## Decisions

| Question | Decision |
|---|---|
| Menu row | **Start working**, after Grill me |
| Skill | `/superpowers:writing-plans` — the task's description and notes are the spec |
| Branch | `task/<slug>-<uuid8>`; worktree where worktrunk's `worktree-path` puts it (`~/.worktrees/<repo>/task-<slug>-<uuid8>`) |
| Task status | Marked active (stopping any other active task on the workspace), as Update status → Active does |
| Second pick | Back to the task's worktree and its Claude; a fresh Claude there if the old one is gone. Never a second worktree |
| Claude's mode | The user's own default, no sandbox and no extra flags: this session is meant to do the work |
| Cleanup | Out of scope — the worktrunk plugin's merge and remove keys handle the worktree |

## Why the worktree step runs in a herdr tab

ADR 0002: worktrunk owns worktrees and runs each repo's `.config/wt.toml`
hooks, whose text is approved once per repo. Verified with worktrunk 0.79:
without a terminal and without `--yes`, `wt switch --create` refuses to run
unapproved hooks and creates nothing ("Cannot prompt for approval in
non-interactive environment"). `--yes` would defeat the approval. So the step
runs where the user can answer — a short-lived herdr tab, as the
herdr-worktrunk plugin's own picker does — which also shows hook output such as
a database clone.

## Flow

**`niritasks task start <uuid>`** (from the menu):

1. The task must exist and be pending; `~/Projects/<workspace>` must be a git
   repository.
2. Find the task's worktree: `wt -C <repo> list --format=json`, an item whose
   `branch` starts `task/` and ends `-<uuid8>` and has a `worktree.path`. By
   uuid, so a description changed since (by Refine) cannot fork a second one.
3. Make the project's herdr session visible (shared with Refine: start it if
   stopped, focus or reattach its window).
4. **Found:** `herdr worktree open --cwd <repo> --path <path> --label <branch>
   --focus`. If agent `work-<uuid8>` is live, `agent focus` it. Otherwise open a
   tab in that worktree workspace and start Claude there (step 7 onward).
5. **Not found:** open a tab labelled `Start: <description>` in the project
   workspace and `herdr pane run` in it: `niritasks task start --here <uuid>`.

**`niritasks task start --here <uuid>`** (inside that tab):

6. `wt -C <repo> switch --create task/<slug>-<uuid8> --no-cd --format=json` —
   approval prompts and hook output land in this tab. On failure, say so and
   wait for a key, leaving the tab open.
7. `herdr worktree open --cwd <repo> --path <path> --label <branch> --focus
   --json`; its `result.root_pane.pane_id` is where Claude goes.
8. `herdr agent start work-<uuid8> --kind claude --pane <pane>`.
9. `herdr agent prompt work-<uuid8> "/superpowers:writing-plans Plan
   Taskwarrior task <uuid>. Read it with \`task rc.json.array=on <uuid>
   export\`; its description and notes are the spec."`
10. Mark the task active; close the setup tab (its own `HERDR_TAB_ID`).

## Found in the live run

- **Claude asks to trust a project it has never been trusted in.** Trust is
  keyed on the main checkout's root, so a worktree of a trusted project starts
  without asking (verified), and trusting `~/.worktrees` would not help — a
  parent folder's trust never extends into a nested git repository. The probe
  asked only because its repo was brand new. When it does ask, `herdr agent
  start` returns `agent_not_ready`; that answer is the user's, so the setup
  tab says so, a notification points to the worktree, and `herdr agent wait
  --until idle` (10 min) carries on once it is answered. A "No" exits Claude
  and the wait fails with herdr's reason.
- **A prompt sent just after that answer can vanish.** herdr reports Claude
  idle a moment before it takes input; a plain `agent prompt` then returns
  success and nothing arrives. So the prompt goes with `--wait --timeout
  15000`: `agent_prompt_stalled` (no activity seen) is resent, up to four
  times; `timeout` (still working) counts as delivered.

## Slug

From the description: lowercased, every run of characters outside `[a-z0-9]`
becomes one `-`, trimmed of `-`, cut to 40 characters at a `-` boundary where
possible; empty becomes `task`. Branch `task/<slug>-<uuid8>`.

## Code

- `src/work.rs`: `branch_name`, `slug`, `find_task_worktree` (over `wt list`
  JSON), `work_agent_name`, `plan_prompt` — pure and unit-tested — and
  `launch` / `set_up_here` doing the I/O.
- `src/refine.rs`: the session-opening part of `launch` becomes
  `open_session`, shared by both.
- `src/herdr.rs`: `worktree_open`, `agent_start_claude` (no extra flags),
  `pane_run`.
- `src/main.rs`: `TaskCommand::Start { uuid, here }` and the menu row.

## Testing

- Unit: slug (punctuation, unicode, length cap, empty), branch name,
  `find_task_worktree` against a real `wt list` shape (match by uuid only,
  ignore items without a worktree), the herdr argv, the prompt.
- Live: a throwaway repo and a throwaway herdr session through `--here`;
  the menu path by the user.
