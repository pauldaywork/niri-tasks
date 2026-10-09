# Two Exports Per Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One `task export` of the tag, split in Rust into pending, waiting and completed, plus the `+BLOCKED` export; the waiting sentinel deleted; an integration oracle proving the Rust split equals Taskwarrior's own filters.

**Architecture:** see `docs/superpowers/specs/2026-10-09-two-exports-design.md`. Taskwarrior task `f5845a99`.

## Global Constraints

- Conventional Commits with an attribution trailer; subject ≤72, body wrapped at 72.
- `cargo build 2>&1 | grep -E "^(warning|error)"` prints nothing; `cargo test` passes in full. `tests/write_path.rs` IS edited in Task 1 (the oracle block); `tests/typed_task_number.rs` and `tests/link_pane.rs` are not.
- Work in `/home/paul/.worktrees/niri-tasks/task-two-exports-f5845a99` (branch `task/two-exports-f5845a99`). **Never run `install.sh`, `cargo install`, `systemctl --user restart niri-tasks`, any `tests/e2e-*.sh`**, or start herdr. Read-only `task … export` commands against the real database are allowed for Task 3's measurement only; never write to the real database (the write-path suite uses its own TASKDATA sandbox).
- Docs on every pub item; comments are full sentences that say why. Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Line numbers from main @ 207c318.

---

### Task 1: `Task.wait`, `Listing`, `partition`, `listing`, the oracle, and the daemon on it

**Files:** `src/task.rs`, `src/daemon.rs` (`cards_for_workspace`), `tests/write_path.rs`

- [ ] **`Task.wait`** (task.rs, beside `start` ~:36): `#[serde(default)] pub wait: Option<String>` with the doc from the spec (parked-until stamp; `99991229T130000Z` for `wait:someday`; a passed date stays on the task and means nothing).
- [ ] **Tests first** (task.rs tests module): a helper `fn t(uuid, status, wait: Option<&str>, urgency: f64, end: &str) -> Task` and the cases in the spec's Tests section (use a fixed `now`, e.g. `stamp_secs("20261009T120000Z").unwrap()`, and stamps around it). Run `cargo test --lib task::` and see them fail to compile.
- [ ] **Implement** `Listing`, `impl Listing { is_empty, into_tasks }`, `fn partition(tasks: Vec<Task>, now: i64) -> Listing`, `pub fn listing(tag: &str) -> Result<Listing>` as the explainer sketches (reuse `latest_finished` for completed; sort pending by urgency desc exactly as `pending_for_tag` does; `partition` is private, `listing` pub). Docs: `Listing` ("A workspace tag's tasks, split the way the task panel's tabs want them, from one `task export`. Deleted tasks are not here: thrown away, not finished."), `partition` ("Taskwarrior 2.6's own rule: a pending task is waiting while its `wait` is after `now`; the stored status stays `pending` either way. A `wait` that does not parse counts as none, so a task is shown rather than hidden on a stamp this tool cannot read."), `into_tasks` ("…marks the waiting part's status `waiting`, which is what `model::cards` reads today; step 2 hands it the Listing instead.").
- [ ] **Daemon:** `cards_for_workspace` → `let Ok(listing) = task::listing(&t) else { return Vec::new() }; if listing.is_empty() { return Vec::new(); } let blocked = task::blocked_uuids_for_tag(&t).unwrap_or_default(); model::cards(&listing.into_tasks(), &blocked)` with the explainer's comments (one export split here; blocked still Taskwarrior's question because a dependency on another tag is invisible to this tag's export). Update the fn doc.
- [ ] **Oracle:** in `tests/write_path.rs::write_path_lifecycle`, find the point after the sandbox holds a parked task (`wait:someday`), a passed-wait task, more than `FINISHED_CAP` completed tasks and a deleted one (read the test: around the `completed_for_tag` assertions ~:517-535 is after the cap check; make sure a parked task still exists there, else place the block where both hold, or park one more task for it). Append the block from the spec (decision 5) with its comment. It must pass against the real `task` binary.
- [ ] Checks: `cargo build` warning grep empty; `cargo test` all green (`write_path` included); `grep -n "pending_for_tag\|waiting_for_tag\|completed_for_tag" src` shows no callers in `src/` (tests only) — note this in the report, it is expected; the functions stay as the oracle.
- [ ] Commit: `perf(task): read a tag's tasks with one export, split in Rust` — body: four exports per refresh became two; the split follows Taskwarrior's own +WAITING rule and is unit-tested; the write-path suite compares it against Taskwarrior's three filters on a real database; blocked stays a Taskwarrior question because a dependency may live on another tag.

### Task 2: `model::cards` takes the `Listing`; the sentinel goes

**Files:** `src/panel/model.rs` (`cards` ~:238-290 and its tests ~:471, :484, :669 that build `"waiting"` tasks), `src/daemon.rs`, `src/task.rs` (delete `into_tasks`)

- [ ] Read `model::cards` and every test that calls it. Change the signature to `pub fn cards(listing: &task::Listing, blocked: &[String]) -> Vec<Card>` (or three slices if that reads better in the tests; say which and why). `Status::Waiting` is decided by membership in `listing.waiting`, `Status::Finished` by `listing.completed` (keep the `finished` sort rule on `end`; it can test `t.status == "completed"` still, or membership). Keep the sort order and every other rule byte-for-byte.
- [ ] Adapt the model tests: where they built a task with `status: "waiting"`, put it in `Listing.waiting` instead; assert the same cards.
- [ ] `daemon.rs`: `model::cards(&listing, &blocked)`. `task.rs`: delete `into_tasks` and its test. `grep -rn '"waiting"' src` → only `task.rs` (the `waiting_for_tag` rewrite, kept for the oracle; add a sentence to its doc saying the panel no longer reads it) and `actions.rs` (the CLI argument).
- [ ] Checks: warning grep empty; `cargo test` green; `cargo clippy --all-targets` adds nothing.
- [ ] Commit: `refactor(panel): build cards from the listing, not a status sentinel`.

### Task 3: measure the export overrides, apply only if justified

**Files:** possibly `src/task.rs` (`base()` or `export_values`), else nothing

- [ ] Measure, read-only, against the real database (tag `niri_tasks`): median of 7 wall-clock runs of `task rc.confirmation=no rc.verbose=nothing rc.json.array=on +niri_tasks export >/dev/null` with and without `rc.hooks=off rc.gc=off`. Also: `stat -c %Y ~/.task/pending.data` before and after five plain exports (and after five with `rc.gc=off`): does a read ever bump the mtime? Record the commands and numbers.
- [ ] Decide: if the overrides save ≥1 ms or a plain export ever bumped the mtime (a read that writes would make the daemon refresh itself), add `rc.gc=off` and/or `rc.hooks=off` to the export path ONLY (not to writes: GC on writes is Taskwarrior's housekeeping), with a comment stating the measured reason. Otherwise change nothing.
- [ ] If changed: `cargo test` green; commit `perf(task): skip hooks and gc on the daemon's reads` with the numbers in the body. If not changed: no commit; the report carries the numbers.

### Task 4: docs

- [ ] `CONTEXT.md`: if the **Task panel** or **Tab** entry mentions how tasks are read, keep it true; else nothing. `docs/adr/0001-*` (the task-data drift ADR): read it; if it lists the per-status exports, add a dated line that the panel now reads one listing and the per-status functions remain as the test oracle. Update the P5 row's wording nowhere else (the HTML reports are ledgers, left alone).
- [ ] Commit `docs: note the one-export listing` only if a file changed.

## Self-review notes

Decisions 1, 2, 3, 5 → Task 1; 4 → Task 2; 6 → Task 3; docs → Task 4. The oracle lands with step 1 so the Rust rule is proven before the sentinel is removed. Task 2's signature choice is left to the implementer with a reason.
