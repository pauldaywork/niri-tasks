# Two exports per refresh: one listing split in Rust, blocked still asked of Taskwarrior

Date: 2026-10-09. Taskwarrior task `f5845a99`. Closes review card P5 (v3/v4), approach A in two steps, as laid out in `~/.local/share/niri-tasks/reviews/p5-one-export-explainer-20261009.html` (measured there against the real database: four exports in sequence 23 ms, two 14 ms, each export ~8 ms of process start-up).

## Problem

`daemon::cards_for_workspace` (daemon.rs:69-88) runs four `task export` subprocesses per refresh: `pending_for_tag`, `waiting_for_tag`, `completed_for_tag`, `blocked_uuids_for_tag` (task.rs:237-297). `waiting_for_tag` rewrites each task's status to `waiting` (task.rs:252-254) because Taskwarrior 2.6.2 matches `status:waiting` but exports the status as `pending`; `model::cards` reads that sentinel (panel/model.rs:267). Four exports can straddle a write and show one task in two tabs or none.

## Decisions

1. **`Task.wait: Option<String>`** (serde default), beside `start`: the parked-until stamp, `99991229T130000Z` for `wait:someday`.
2. **`task::Listing { pending, waiting, completed }`** and **`fn partition(tasks: Vec<Task>, now: i64) -> Listing`**, pure: status `pending` with `wait` parsing to a stamp strictly after `now` → waiting; other `pending` → pending, sorted most urgent first as `pending_for_tag` does; `completed` → through `latest_finished` (newest `end` first, cut to `FINISHED_CAP`); `deleted` and anything else dropped. A `wait` that does not parse counts as none (shown, not hidden). **`pub fn listing(tag) -> Result<Listing>`** = `partition(export(&[&format!("+{tag}")])?, now_secs())`.
3. **Step 1 keeps `model::cards` untouched.** `Listing::into_tasks()` concatenates the three parts and marks the waiting part's status `waiting`, so the sentinel moves rather than vanishes. `cards_for_workspace` calls `listing` then `blocked_uuids_for_tag` (still Taskwarrior's question: a dependency may live on another tag) then `model::cards(&listing.into_tasks(), &blocked)`. Two subprocesses.
4. **Step 2 deletes the sentinel.** `model::cards` takes the `Listing` (or its three slices) and the blocked uuids, deciding `Status::Waiting` by which part a task came from; `into_tasks` goes; `panel/model.rs` tests that build tasks with status `"waiting"` build a Listing instead. Nothing outside `task.rs` reads `"waiting"` afterwards (`actions.rs:262` is the CLI argument `task status <uuid> waiting`, which stays).
5. **The oracle.** `tests/write_path.rs::write_path_lifecycle` gains a block, at the point where the sandbox holds pending, parked, passed-wait, completed past the cap and deleted tasks, asserting `listing(TAG)`'s pending and waiting uuid sets equal `pending_for_tag`/`waiting_for_tag`'s, its completed uuids equal `completed_for_tag`'s in order, its waiting part is non-empty, its completed part has `FINISHED_CAP` entries, and its waiting tasks still carry status `pending` as exported. This is what keeps the Rust rule honest against the real binary. The per-status functions stay for that reason and for `is_waiting`/`active_for_tag`'s callers.
6. **A measurement, not a decision yet:** time `task … export` with and without `rc.hooks=off rc.gc=off` and check whether a plain export ever bumps `pending.data`'s mtime (a read that writes would make the daemon refresh itself). Apply the overrides to the export path only if the measurement or the mtime check justifies it, with a comment saying which.
7. **User-visible behaviour unchanged** except the race in the Problem, which can no longer split one snapshot.

## Out of scope

Reading Taskwarrior's data files; incremental per-uuid refresh (ruled out: one process spawn is the floor); concurrency (approach B); changing `FINISHED_CAP`, the urgency order or the panel's tabs.

## Tests

Unit (task.rs, `partition` with hand-built tasks and a fixed `now`): pending without wait; wait in the future → waiting; wait in the past → pending; wait equal to now → pending (strict, documented choice); wait unparsable → pending; completed sorted newest first and cut to the cap; deleted dropped; unknown status dropped; empty input; waiting tasks keep status `pending`; pending sorted by urgency desc; `into_tasks` marks waiting (step 1 only) ; `is_empty`. Integration: the oracle block (decision 5). Model (step 2): the existing `cards` tests adapted to a Listing.
