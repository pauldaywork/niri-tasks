# Finished Tab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The task panel gets a Finished tab listing the workspace's last 12 completed tasks, most recently finished first, from which Back to list reopens a task closed too soon.

**Architecture:** `src/task.rs` reads `end`, gains `completed_for_tag` (the last 12 by `end`) and teaches `set_status(Stopped)` to reopen a completed task with `modify status:pending`. `TaskState` gains `finished`, which gives a task Back to list, Edit, Note, Speak and Remove, as `waiting` does. `model.rs` gets `Status::Finished` and `Filter::Finished`, kept off every other tab as waiting cards are, and sorts finished cards last, newest `end` first, ageing them from `end`. The keys move Ideas to 7. The daemon loads the completed tasks with the rest, and also watches `completed.data`, which is the only file taskwarrior writes when a finished task is edited, noted or removed.

**Tech Stack:** Rust 2021, GTK4 (gtk4-rs) and its CSS, Taskwarrior 2.6.2 CLI. No new crates.

**Spec:** Taskwarrior task `b6f3bc1f-2a99-49b6-acac-2f7704b0ee24`. Read it with `task rc.json.array=on b6f3bc1f-2a99-49b6-acac-2f7704b0ee24 export`. Its description and notes are the spec.

## Global Constraints

- A Finished filter tab after Waiting and before Ideas, shown only while it has a task; key 6 picks it and Ideas moves to 7.
- It lists the workspace tag's last 12 completed tasks, most recently finished (`end`) first; deleted tasks are left out.
- Finished cards are kept off All, the other tabs, the hover and the peek, as waiting cards are.
- A finished card's age counts from when it was finished (`end`), not from when it was added.
- A finished card's actions are Back to list (reopen), Edit, Note, Speak and Remove; no Clear all.
- Back to list on a finished task reopens it as pending, through the same `niritasks task status <uuid> stopped` command it already runs.
- Out of scope: finished tasks in the picker and the action menu; finished tasks beyond the last 12.
- Done when: finishing a task puts it at the top of Finished, Finished never shows more than 12, Back to list puts a task back on All, and the panel state tests cover the new tab.
- House style: every new item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commits use Conventional Commits (`feat(panel): …`, imperative, lowercase, ≤72 chars) and end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Facts checked while planning (taskwarrior 2.6.2, sandboxed TASKDATA)

- `task <uuid> stop` on a completed task fails ("not started") and `task <uuid> modify wait:` succeeds but leaves it `completed`. So today `task status <uuid> stopped` does nothing to a finished task, and Back to list needs a new path.
- `task <uuid> modify status:pending` reopens a completed task, and taskwarrior drops its `end` by itself. The task gets an id again on the next export.
- `task export` of a completed task carries `end` (`20261007T040233Z`) and `"id":0`; a uuid still addresses it.
- `task <uuid> annotate`, `task import` (what `task::replace_text` uses) and `task <uuid> delete` on a completed task each write `completed.data` and leave `pending.data`'s mtime alone. A deleted completed task keeps its `end` but its status becomes `deleted`, so `status:completed` drops it.

## Decisions made while planning (flag any you disagree with)

- **The daemon watches `completed.data` too.** It skips `task export` unless `pending.data`'s mtime moved. Without the second file, Remove, Edit or Note on a finished card would not show until something else changed.
- **Reopening lives in `task::set_status`, not in `stop`.** `set_status` already reads the task to decide. `Stopped` on a completed task reopens it, so the Back button's command, the menu's Update status → Stopped and `niritasks task status <uuid> stopped` all reopen. The notification still reads "Stopped: …", as it does for a waiting task brought back.
- **`Card.entry` is renamed `Card.since`.** It is the stamp the age counts from: `entry` on a task to do, `end` on a finished one. Keeping the name `entry` would read wrong on every finished card.
- **`cards()` sorts finished cards last, newest `end` first**, and `completed_for_tag` sorts and cuts to 12 too. The cut has to be in `task.rs`, so it sorts there. `cards()` sorts again so the model does not rely on the order it is handed.
- **`TaskState::of` reads `finished` from the task's status.** Enter on a finished card's body opens the action menu (`niritasks task menu <uuid>`), and it should offer what the row does, not Start working. The menu then reads Edit, Note, Speak, Update status, Move to workspace, the same as a waiting task's. Finished tasks are still not listed in the picker or reachable from it, so the out-of-scope line holds.
- **The finished card is dimmed** like waiting and blocked cards, with Font Awesome's check (`\u{f00c}`) as its icon. A finished task that still carries `+next` is not drawn yellow.
- **A workspace with only finished tasks** hides on the edge, and its keyboard panel opens on All reading "No tasks" beside the Finished tab, as one with only waiting tasks does today. It no longer falls back to the fuzzel list's "Add task" row there.
- **The e2e panel test presses 7 for Ideas.** It completes every task midway, so by its Ideas-after-restart step a Finished tab exists and 6 would pick it.

## File Structure

- `src/task.rs`: `Task.end`, `FINISHED_CAP`, `completed_for_tag`, private `latest_finished` and `reopen`, and `set_status` reopening. Unit tests.
- `tests/write_path.rs`: the Finished list and the reopen against a sandboxed database.
- `src/rows.rs`, `src/panel/model.rs`: test helpers gain `end`.
- `src/actions.rs`: `TaskState.finished`, `TaskState::off_list`, `applies`, `of`.
- `src/panel/actions.rs`: `advance` on a finished task, and row tests.
- `src/menu.rs`: a test for a finished task's menu.
- `src/panel/model.rs`: `Status::Finished`, `Filter::Finished`, `Card.since`, the sort.
- `src/panel/surface.rs`: the `finished` CSS class, `since`, and the module doc's tab keys.
- `src/panel/style.rs`: dim `.finished`.
- `src/panel/state.rs`: test helper `since`, doc comments, and the Finished tab tests.
- `src/panel/keys.rs`: 6 is Finished, 7 is Ideas.
- `src/daemon.rs`: load the completed tasks, and watch `completed.data`.
- `tests/e2e-panel.sh`: 7 for Ideas.
- `CONTEXT.md`, `README.md`, `llms.txt`: the tab, its keys, and `stopped` reopening.

---

### Task 1: Read `end`, list the last 12 finished, and reopen on Stopped

**Files:**
- Modify: `src/task.rs` (struct `Task` at :27-58; after `blocked_uuids_for_tag` at ~:270; `set_status` at :683-696; `mod tests` from :698)
- Modify: `src/rows.rs:64-81` (test helper `task()`)
- Modify: `src/panel/model.rs:282-295` (test helper `task()`)
- Test: `tests/write_path.rs` (inside `write_path_lifecycle`, after the `set_status(&del, Status::Deleted)` block at ~:510)

**Interfaces:**
- Produces:
  - `Task.end: String` (taskwarrior's stamp, empty when absent)
  - `pub const FINISHED_CAP: usize = 12` in `crate::task`
  - `pub fn completed_for_tag(tag: &str) -> Result<Vec<Task>>` in `crate::task`: completed tasks carrying `+tag`, newest `end` first, at most 12. Each has `status == "completed"`.
  - `task::set_status(uuid, Status::Stopped)` on a completed task makes it pending.

- [ ] **Step 1: Write the failing unit tests**

Add to `mod tests` in `src/task.rs`:

```rust
    /// `end` is when a task was finished: present on a completed task,
    /// absent on one still to do.
    #[test]
    fn parses_end() {
        let json = r#"[{"uuid":"a","description":"d","status":"completed","end":"20261007T040233Z"},
                       {"uuid":"b","description":"d"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].end, "20261007T040233Z");
        assert_eq!(tasks[1].end, "");
    }

    fn finished(uuid: &str, day: u32) -> Task {
        serde_json::from_value(serde_json::json!({
            "uuid": uuid,
            "description": uuid,
            "status": "completed",
            "end": format!("202610{day:02}T120000Z"),
        }))
        .unwrap()
    }

    /// The Finished tab's list: the most recently finished first, and never
    /// more than twelve, however many there are.
    #[test]
    fn the_latest_finished_come_first_and_stop_at_twelve() {
        let got = latest_finished((1..=15).map(|d| finished(&format!("t{d}"), d)).collect());
        assert_eq!(got.len(), FINISHED_CAP);
        assert_eq!(FINISHED_CAP, 12);
        assert_eq!(got[0].uuid, "t15");
        assert_eq!(got[11].uuid, "t4");
    }

    #[test]
    fn fewer_than_twelve_finished_are_all_kept_newest_first() {
        let got = latest_finished(vec![finished("old", 1), finished("new", 9), finished("mid", 5)]);
        let uuids: Vec<&str> = got.iter().map(|t| t.uuid.as_str()).collect();
        assert_eq!(uuids, vec!["new", "mid", "old"]);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib task::tests 2>&1 | tail -20`
Expected: compile errors: no field `end` on `Task`, cannot find `latest_finished` and `FINISHED_CAP`.

- [ ] **Step 3: Add `end`, `FINISHED_CAP`, `completed_for_tag` and `latest_finished`**

In `struct Task`, after the `priority` field:

```rust
    /// When the task was finished, as taskwarrior stamps it, on a completed
    /// or deleted task alone. The Finished tab lists the newest first and
    /// ages its cards from it. Empty on a task still to do.
    #[serde(default)]
    pub end: String,
```

After `blocked_uuids_for_tag`:

```rust
/// How many finished tasks the task panel's Finished tab lists: enough to
/// find one closed too soon, few enough that it never needs "+N more".
pub const FINISHED_CAP: usize = 12;

/// The last [`FINISHED_CAP`] tasks finished on `tag`, the most recently
/// finished first, for the task panel's Finished tab. Completed only: a
/// deleted task was thrown away, not finished, and its status says
/// `deleted` even though it keeps its `end`.
pub fn completed_for_tag(tag: &str) -> Result<Vec<Task>> {
    Ok(latest_finished(export(&[&format!("+{tag}"), "status:completed"])?))
}

/// Newest `end` first, cut to [`FINISHED_CAP`]. Apart from
/// [`completed_for_tag`] so the order and the cut are tested without a
/// task database. The stamp sorts as text the way it does as a time.
fn latest_finished(mut tasks: Vec<Task>) -> Vec<Task> {
    tasks.sort_by(|a, b| b.end.cmp(&a.end));
    tasks.truncate(FINISHED_CAP);
    tasks
}
```

In `src/rows.rs` test helper `task()`, after `priority: None,` add `end: String::new(),`. Do the same in `src/panel/model.rs` test helper `task()`.

- [ ] **Step 4: Run the unit tests**

Run: `cargo test --lib task::tests 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 5: Write the failing integration test**

In `tests/write_path.rs`, right after `task::set_status(&del, Status::Deleted).expect("deleting twice is a no-op");`:

```rust
    // ---- a finished task: the Finished tab, and Back to list -------------
    // `st` was completed above and `del` deleted. Stopped is what Back to
    // list runs, and on a finished task it has to reopen it: stop alone
    // leaves a completed task completed.
    let finished = task::completed_for_tag(TAG).expect("finished");
    assert!(finished.iter().any(|t| t.uuid == st), "a completed task is on the Finished list");
    assert!(finished.iter().all(|t| t.status == "completed" && !t.end.is_empty()));
    assert!(!finished.iter().any(|t| t.uuid == del), "a deleted task is not finished");

    task::set_status(st8, Status::Stopped).expect("back to list");
    assert_eq!(raw(&st, "status"), "pending", "stopped reopens a finished task");
    assert!(raw(&st, "end").is_empty(), "a reopened task is not finished");
    assert!(task::pending_for_tag(TAG).expect("list").iter().any(|t| t.uuid == st));
    assert!(!task::completed_for_tag(TAG).expect("finished").iter().any(|t| t.uuid == st));

    // Never more than twelve, however many are finished.
    for n in 0..14 {
        let uuid = task::add(TAG, &text::add_args(&format!("finish me {n}")))
            .expect("add")
            .expect("a uuid");
        task::complete(&uuid).expect("complete");
    }
    assert_eq!(task::completed_for_tag(TAG).expect("finished").len(), task::FINISHED_CAP);
```

- [ ] **Step 6: Run it to see it fail**

Run: `cargo test --test write_path 2>&1 | tail -15`
Expected: FAIL at `stopped reopens a finished task` (left `"completed"`).

- [ ] **Step 7: Reopen in `set_status`**

In `src/task.rs`, after `fn stop`, add:

```rust
/// Put a finished task back on the list, pending again: Back to list on
/// the Finished tab. Taskwarrior drops its `end` with the status. `stop`
/// cannot do it, since neither stopping nor clearing a wait touches a
/// completed task's status.
fn reopen(uuid: &str) -> Result<()> {
    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg("status:pending")
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify status:pending` failed");
    Ok(())
}
```

In `set_status`, change the second `match` to:

```rust
    match status {
        Status::Active => set_active(uuid),
        // Back to list on a finished card runs Stopped, as it does on a
        // waiting one: one command for both ways back.
        Status::Stopped if current.status == "completed" => reopen(uuid),
        Status::Stopped => stop(uuid),
        Status::Waiting => wait(uuid),
        Status::Completed => complete(uuid),
        Status::Deleted => delete(uuid),
    }
```

and add to `set_status`'s doc comment, after its first line:

```rust
///
/// Stopped on a completed task reopens it, which is how Back to list on
/// the Finished tab puts a task back.
```

- [ ] **Step 8: Run every test**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: every `test result: ok`.

- [ ] **Step 9: Commit**

```bash
git add src/task.rs src/rows.rs src/panel/model.rs tests/write_path.rs
git commit -m "feat(task): list the last finished tasks and reopen one on stopped

The task panel's Finished tab needs a workspace's last 12 completed
tasks, newest end first, so Task reads end and completed_for_tag lists
them. Back to list runs task status <uuid> stopped, which left a
completed task completed; it now reopens it as pending.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: A finished task's actions

**Files:**
- Modify: `src/actions.rs` (`Back` doc at :22-24, `TaskState` at :50-80, `applies` at :92-106, tests from :178)
- Modify: `src/panel/actions.rs` (`advance` at :38-45, tests from :170)
- Modify: `src/menu.rs` (tests, after `a_waiting_task_gets_no_start_working_or_refine` at :107-114)
- Modify: `src/panel/model.rs:85-91` (`Card::state`'s literal, only so it compiles; Task 3 sets it properly)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces:
  - `TaskState.finished: bool`
  - `TaskState::off_list(self) -> bool` (waiting or finished)
  - `Action::applies` on a finished state gives exactly `[Back, Edit, Note, Speak, Remove]` from `Action::ALL`.
  - `Action::row` on a finished state gives `[Back, Edit, Speak, Remove]`.
  - `Action::advance` on a finished state is `None`.

- [ ] **Step 1: Write the failing tests**

In `src/actions.rs` `mod tests`, add:

```rust
    /// A finished task is off the list, as a waiting one is: the way back,
    /// and what still works on any task. Never Start working or Up next.
    #[test]
    fn a_finished_task_gets_back_to_list_and_what_still_works_on_it() {
        for has_session in [false, true] {
            for up_next in [false, true] {
                for planned in [false, true] {
                    let state = TaskState { finished: true, has_session, up_next, planned, ..TaskState::default() };
                    assert_eq!(offered(state), vec![Back, Edit, Note, Speak, Remove], "{state:?}");
                }
            }
        }
    }
```

In `a_tasks_state_is_read_from_the_task`, change the full literal to include `finished: false`:

```rust
            TaskState { active: true, waiting: false, finished: false, planned: true, up_next: true, has_session: true }
```

and add at the end of that test:

```rust
        let done: Task = serde_json::from_str(r#"{"uuid":"u","description":"d","status":"completed"}"#).unwrap();
        assert_eq!(TaskState::of(&done, false, false), TaskState { finished: true, ..TaskState::default() });
```

In `src/panel/actions.rs` `mod tests`, add a helper after `fn waiting`:

```rust
    fn finished(planned: bool) -> TaskState {
        TaskState { finished: true, planned, ..TaskState::default() }
    }
```

change `every_state()`'s inner array to `[on_list(planned), active(planned), waiting(planned), finished(planned)]`, and add:

```rust
    /// A finished task gets the way back, and what still works on it, as a
    /// waiting one does: Note is the menu's, as on every card.
    #[test]
    fn a_finished_task_gets_back_to_list_edit_speak_and_remove() {
        for state in [finished(false), with_claude(finished(true))] {
            assert_eq!(Action::row(state), vec![Back, Edit, Speak, Remove], "{state:?}");
        }
    }

    /// Nothing to refine or start on a task already done.
    #[test]
    fn ctrl_enter_leaves_a_finished_task_alone() {
        assert_eq!(Action::advance(finished(false)), None);
        assert_eq!(Action::advance(finished(true)), None);
        assert_eq!(hint(finished(false), None), "");
        assert_eq!(hint(finished(false), Some(Back)), "b: Back to list");
    }
```

Rename `every_card_but_a_waiting_one_gets_up_next` to `every_card_on_the_list_gets_up_next`, change its doc to "Every card on the list gets Up next, active ones too; a waiting or finished task is off the list." and its assertion to:

```rust
            assert_eq!(Action::row(state).contains(&UpNext), !state.off_list(), "{state:?}");
```

In `src/menu.rs` `mod tests`, after `a_waiting_task_gets_no_start_working_or_refine`:

```rust
    /// Enter on a finished card opens this menu, which offers what its row
    /// does, as a waiting task's does.
    #[test]
    fn a_finished_task_gets_no_start_working_or_refine() {
        for has_session in [false, true] {
            let state = TaskState { finished: true, has_session, ..TaskState::default() };
            assert_eq!(labels(state), ["Edit", "Note", "Speak", "Update status", "Move to workspace"], "{state:?}");
        }
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib actions menu 2>&1 | tail -10`
Expected: compile error: no field `finished` on `TaskState`.

- [ ] **Step 3: Add `finished` and `off_list`, and use them**

In `src/actions.rs`, `struct TaskState`, after `waiting`:

```rust
    /// Completed: on the Finished tab, off the list as a waiting task is.
    pub finished: bool,
```

In `TaskState::of`, after `waiting,` add:

```rust
            // A completed task's export says so, unlike a waiting one's.
            finished: task.status == "completed",
```

and update its doc's first sentence to "`task`'s state, finished included, read from its status."

Add to `impl TaskState`:

```rust
    /// Off the list: parked as waiting, or finished. Either way it gets the
    /// way back and what works on any task, and nothing that moves a task
    /// along the list.
    pub fn off_list(self) -> bool {
        self.waiting || self.finished
    }
```

Change `applies` to:

```rust
    pub fn applies(self, state: TaskState) -> bool {
        match self {
            Session => state.has_session && !state.off_list(),
            Back => state.off_list(),
            Stop => state.active,
            Start | Refine | Grill | UpNext | Wait => !state.off_list(),
            Edit | Note | Speak | Remove => true,
        }
    }
```

and in its doc comment change "A waiting task is parked: it gets the way back to the list" to "A waiting or finished task is off the list: it gets the way back to it".

Change `Back`'s doc on the enum to:

```rust
    /// Back on the list: Update status → Stopped, which clears a waiting
    /// task's wait date and reopens a finished one.
```

In `src/panel/actions.rs` `advance`, add a first arm:

```rust
            TaskState { finished: true, .. } => None,
```

and change its doc's last sentence to "Nothing once a planned task is being worked, nor on a waiting or finished task, which have no step to take."

In `src/panel/model.rs` `Card::state`, add `finished: false,` after `waiting: …,` in the literal (Task 3 replaces it).

- [ ] **Step 4: Run every test**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: every `test result: ok`. `every_hint_fits_beside_the_widest_row` now covers finished cards too.

- [ ] **Step 5: Commit**

```bash
git add src/actions.rs src/panel/actions.rs src/menu.rs src/panel/model.rs
git commit -m "feat(actions): give a finished task back to list, edit, note, speak

A finished task is off the list as a waiting one is, so it gets the same
actions: Back to list, which now reopens it, Edit, Note, Speak and
Remove. Its menu, which Enter on its card opens, offers the same.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Finished cards and the Finished filter

**Files:**
- Modify: `src/panel/model.rs` (`Status` :15-26, `Card` :28-49, `impl Card` :51-93, `Filter` :95-178, `cards` :200-247, `cap` :249-266, tests)
- Modify: `src/panel/surface.rs:818-829` (card class match) and `:866` (`card.entry`)
- Modify: `src/panel/style.rs` (the dimmed rule at ~:223-226, its doc at :9-11, a test)
- Modify: `src/panel/state.rs:617` (test helper `card()`)

**Interfaces:**
- Consumes: `Task.end` (Task 1); `TaskState.finished` (Task 2).
- Produces:
  - `Status::Finished`, icon `"\u{f00c}"`
  - `Card.since: String` (replaces `Card.entry`)
  - `Filter::Finished`, label `"Finished"`, empty text `"No finished tasks"`
  - `Filter::TABS: [Filter; 6]` = All, Active, Planned, ToRefine, Waiting, Finished
  - `model::cards` puts finished tasks (`status == "completed"`) last, newest `end` first, as `Status::Finished` cards.

- [ ] **Step 1: Write the failing tests**

In `src/panel/model.rs` `mod tests`, add a helper after `fn waiting`:

```rust
    /// A task added on 1 October and finished on this day.
    fn finished(uuid: &str, day: u32) -> Task {
        let mut t = task(uuid, 1, false);
        t.status = "completed".into();
        t.end = format!("202610{day:02}T180000Z");
        t
    }
```

and these tests:

```rust
    #[test]
    fn a_finished_task_gets_the_check() {
        let got = cards(&[finished("done", 2)], &[]);
        assert_eq!(got[0].status, Status::Finished);
        assert_eq!(got[0].icon(), "\u{f00c}");
    }

    /// Finished tasks go under every task still to do, the one finished
    /// last on top, whatever was added when.
    #[test]
    fn finished_tasks_go_last_most_recently_finished_first() {
        let got = cards(&[finished("old", 2), task("todo", 1, false), finished("new", 5), task("started", 1, true)], &[]);
        assert_eq!(texts(&got), vec!["started", "todo", "new", "old"]);
    }

    /// Finished is finished: under the Finished tab and no other, so All
    /// stays what the hover shows.
    #[test]
    fn a_finished_task_is_only_under_finished() {
        let mut planned_done = finished("planned-done", 3);
        planned_done.tags = vec![crate::task::PLANNED_TAG.into()];
        let all = cards(&[task("plain", 9, false), waiting("parked"), finished("done", 4), planned_done], &[]);
        assert_eq!(texts(&Filter::All.pick(&all)), vec!["plain"]);
        assert!(Filter::Planned.pick(&all).is_empty());
        assert_eq!(texts(&Filter::ToRefine.pick(&all)), vec!["plain"]);
        assert_eq!(texts(&Filter::Waiting.pick(&all)), vec!["parked"]);
        assert_eq!(texts(&Filter::Finished.pick(&all)), vec!["done", "planned-done"]);
    }

    /// The age on a finished card is how long ago it was finished.
    #[test]
    fn a_finished_card_ages_from_when_it_was_finished() {
        let got = cards(&[finished("done", 6)], &[]);
        assert_eq!(got[0].since, "20261006T180000Z");
        // 2026-10-06 12:00:00Z is 1_791_288_000, so 18:00 is 6h on.
        let later = 1_791_288_000 + 6 * 3_600 + 2 * 3_600;
        assert_eq!(got[0].age(later).as_deref(), Some("2h"));
    }

    /// A finished card's state is finished, and it is not drawn yellow even
    /// when the task still carries `+next`.
    #[test]
    fn a_finished_card_is_finished_and_never_yellow() {
        let mut next_done = finished("done", 2);
        next_done.tags = vec![crate::task::UP_NEXT_TAG.into()];
        let got = cards(&[next_done], &[]);
        assert_eq!(got[0].state(false), Some(TaskState { finished: true, up_next: true, ..TaskState::default() }));
        assert!(!got[0].shows_up_next());
    }

    #[test]
    fn the_finished_tab_comes_after_waiting_and_before_ideas() {
        let all = cards(&[task("plain", 9, false), waiting("parked"), finished("done", 2)], &[]);
        assert_eq!(
            Tab::shown(&all),
            vec![
                Tab::Filter(Filter::All),
                Tab::Filter(Filter::ToRefine),
                Tab::Filter(Filter::Waiting),
                Tab::Filter(Filter::Finished),
                Tab::Ideas,
            ]
        );
        assert_eq!(Filter::shown(&cards(&[finished("done", 2)], &[])), vec![Filter::All, Filter::Finished]);
    }
```

Update the existing tests:
- `the_tabs_run_all_active_planned_to_refine_waiting`: rename to `the_tabs_run_all_active_planned_to_refine_waiting_finished` and expect `vec!["All", "Active", "Planned", "To refine", "Waiting", "Finished"]`.
- `an_empty_tab_says_what_it_has_none_of`: add `assert_eq!(Filter::Finished.empty_text(), "No finished tasks");`.
- `a_cards_state_is_its_tasks`: in the `card` closure rename `entry: String::new()` to `since: String::new()`, and add `finished: false,` to the first full `TaskState { … }` literal.
- `a_card_ages_from_its_tasks_stamp`: `got[0].entry` → `got[0].since`.
- `the_more_card_has_no_age`: `more.entry` → `more.since`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib panel::model 2>&1 | tail -10`
Expected: compile errors: no variant `Status::Finished`, `Filter::Finished`, no field `since`.

- [ ] **Step 3: Implement in `model.rs`**

In `enum Status`, after `Waiting`:

```rust
    /// Completed, on the keyboard's Finished tab only, as a waiting task is
    /// on the Waiting tab only.
    Finished,
```

In `Card::icon`, after the `Waiting` arm:

```rust
            // Font Awesome's check: done.
            Status::Finished => "\u{f00c}",
```

Replace the `entry` field of `Card` with:

```rust
    /// The stamp the age at the card's right end counts from: when the
    /// task was added, or, on a finished card, when it was finished. The
    /// stamp, not the age: the age changes every minute, and a changed card
    /// re-renders the panel, disarming a half-pressed Remove. Empty on
    /// "+N more".
    pub since: String,
```

`Card::shows_up_next`: change the match to `!matches!(self.status, Status::Active | Status::Waiting | Status::Finished)` and its doc's first sentence to "Whether the card is drawn yellow, as up next. Not an active card, which stays green, nor a waiting or finished one, which keep their dimmed look:".

`Card::age`: `crate::task::age(&self.since, now)`, doc: "How long ago the task was added, or finished on a finished card, `now` in Unix seconds. None on "+N more", and on a stamp taskwarrior wrote in some other shape."

`Card::state`: replace the Task 2 placeholder with `finished: self.status == Status::Finished,`.

In `enum Filter`, after `Waiting`:

```rust
    /// Tasks completed lately, the last [`crate::task::FINISHED_CAP`].
    Finished,
```

and change the enum's doc's last sentence to "A waiting task is under Waiting and no other tab, and a finished one under Finished: it is parked or done, and All is what the hover shows."

`Filter::TABS`:

```rust
    /// The tabs left to right, which is also the order 1 to 6 pick them in.
    pub const TABS: [Filter; 6] = [
        Filter::All,
        Filter::Active,
        Filter::Planned,
        Filter::ToRefine,
        Filter::Waiting,
        Filter::Finished,
    ];
```

`label`: add `Filter::Finished => "Finished",`. `empty_text`: add `Filter::Finished => "No finished tasks",` and change its doc's "on a workspace whose tasks are all waiting" to "on a workspace whose tasks are all waiting or finished".

`matches`:

```rust
        match self {
            Filter::Waiting => card.status == Status::Waiting,
            Filter::Finished => card.status == Status::Finished,
            _ if matches!(card.status, Status::Waiting | Status::Finished) => false,
            Filter::All => true,
            Filter::Active => card.status == Status::Active,
            Filter::Planned => card.planned && card.status != Status::Active,
            Filter::ToRefine => !card.planned,
        }
```

`cards`: add to its doc, after the first paragraph's first sentence: "Finished tasks, which only the Finished tab shows, go last, the one finished most recently first." Replace the sort and the map:

```rust
    // Only below up next: an active or up next task stays on top blocked.
    let sinks = |t: &Task| !t.is_active() && !t.is_up_next() && blocked.contains(&t.uuid);
    let finished = |t: &Task| t.status == "completed";
    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by(|a, b| {
        finished(a).cmp(&finished(b)).then_with(|| {
            if finished(a) {
                // Both finished: the one finished last on top.
                return b.end.cmp(&a.end);
            }
            b.is_active()
                .cmp(&a.is_active())
                .then(b.is_up_next().cmp(&a.is_up_next()))
                .then(sinks(a).cmp(&sinks(b)))
                .then(b.priority_rank().cmp(&a.priority_rank()))
                // The stamp sorts as text the way it does as a time.
                .then(b.entry.cmp(&a.entry))
        })
    });

    sorted
        .iter()
        .map(|t| Card {
            // Finished first, then waiting: parking a task stops it, so a
            // waiting task is not the work in progress whatever else it
            // carries.
            status: if finished(t) {
                Status::Finished
            } else if t.status == "waiting" {
                Status::Waiting
            } else if t.is_active() {
                Status::Active
            } else if blocked.contains(&t.uuid) {
                Status::Blocked
            } else if t.is_planned() {
                Status::Planned
            } else {
                Status::Pending
            },
            text: crate::text::collapse_whitespace(&t.description),
            uuid: Some(t.uuid.clone()),
            planned: t.is_planned(),
            up_next: t.is_up_next(),
            since: if finished(t) { t.end.clone() } else { t.entry.clone() },
        })
        .collect()
```

`cap`: `entry: String::new(),` → `since: String::new(),`.

- [ ] **Step 4: Follow the rename and the new status elsewhere**

`src/panel/state.rs:617`, test helper `card()`: `entry: String::new()` → `since: String::new()`.

`src/panel/surface.rs`, in `card_widget`'s `match card.status`, after the `Waiting` arm:

```rust
            Status::Finished => root.add_css_class("finished"),
```

and change the comment under it to "whatever its icon, unless it is active, waiting or finished." At `:866`: `card.entry.clone()` → `card.since.clone()`.

`src/panel/style.rs`: the dimmed rule becomes

```css
.task-panel .task-card.blocked,
.task-panel .task-card.waiting,
.task-panel .task-card.finished,
.task-panel .task-card.more {{ color: alpha({TEXT}, 0.55); }}
```

and the comment after it: "Active, waiting and finished cards never get the class (Card::shows_up_next)." Add to the module doc's second paragraph: "Blocked, waiting and finished cards are dimmed." Add a test to its `mod tests`:

```rust
    /// A finished card is dimmed, as a waiting one is: done, not to do.
    #[test]
    fn a_finished_card_is_dimmed() {
        assert!(css().contains(
            ".task-panel .task-card.waiting,\n.task-panel .task-card.finished,\n.task-panel .task-card.more"
        ));
    }
```

(Check the exact whitespace of the generated rule with `cargo test --lib panel::style` once; the CSS is one `format!` string, so the lines are joined by `\n` with no indent.)

- [ ] **Step 5: Run every test**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: every `test result: ok`.

- [ ] **Step 6: Commit**

```bash
git add src/panel/model.rs src/panel/surface.rs src/panel/style.rs src/panel/state.rs
git commit -m "feat(panel): add finished cards and the Finished filter

Completed tasks become finished cards, dimmed with a check, on a
Finished filter after Waiting and on no other tab, the hover and the
peek included. They sort last, the one finished most recently first,
and age from when they were finished.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Key 6 for Finished, 7 for Ideas, and the panel state tests

**Files:**
- Modify: `src/panel/keys.rs` (`KeyAction` docs at :25-30, `key_action` at :76-81, tests)
- Modify: `src/panel/state.rs` (`hidden` doc at :146-149, `filter_key` doc at :461-462, tests after `the_brackets_skip_hidden_tabs_and_stop_at_the_ends` at :740-751)
- Modify: `src/panel/surface.rs` (module doc at :59-93, `empty_line` doc at :1279-1282, the `empty` comment at ~:787)
- Modify: `tests/e2e-panel.sh` (:495, :503, :602)

**Interfaces:**
- Consumes: `Filter::Finished`, `Filter::TABS[5]`, `Status::Finished` (Task 3); `Action::row` on a finished card is `[Back, Edit, Speak, Remove]` (Task 2).
- Produces: `key_action(gdk::Key::_6, false, false) == KeyAction::Filter(Filter::Finished)`; `key_action(gdk::Key::_7, false, false) == KeyAction::Ideas`.

- [ ] **Step 1: Write the failing key tests**

In `src/panel/keys.rs` `mod tests`:
- `numbers_pick_the_tabs_in_order`: doc "1 to 6 are the tabs left to right." and `let numbers = [gdk::Key::_1, gdk::Key::_2, gdk::Key::_3, gdk::Key::_4, gdk::Key::_5, gdk::Key::_6];`
- Replace `six_picks_ideas` with:

```rust
    /// 7 is the Ideas tab, after the six filter tabs.
    #[test]
    fn seven_picks_ideas() {
        assert_eq!(key_action(gdk::Key::_7, false, false), KeyAction::Ideas);
    }
```

- `other_numbers_pass_through`: doc "Past the seven tabs a number is nothing, not an eighth tab." and keys `[gdk::Key::_0, gdk::Key::_8, gdk::Key::_9]`.
- `on_ideas_every_other_key_is_typing`: add `gdk::Key::_7,` after `gdk::Key::_6,`.

- [ ] **Step 2: Write the failing state tests**

In `src/panel/state.rs` `mod tests`, after `the_brackets_skip_hidden_tabs_and_stop_at_the_ends`:

```rust
    // ─── the Finished tab ────────────────────────────────────────────────

    /// Finished tasks are off the hover and the peek, as waiting ones are,
    /// and a workspace with only finished tasks shows nothing on its edge.
    #[test]
    fn finished_tasks_are_off_the_hover() {
        let mut state = PanelState::default();
        state.set_cards(&[card("a", Status::Pending), card("f", Status::Finished)]);
        assert_eq!(uuids(&state), vec!["a"]);
        state.set_cards(&[card("f", Status::Finished)]);
        assert!(state.hidden());
    }

    /// Only finished tasks: the keyboard's panel opens on All saying it has
    /// none, beside the Finished tab, as with only waiting tasks.
    #[test]
    fn only_finished_tasks_open_on_all_beside_the_finished_tab() {
        let state = keyboard(vec![card("f", Status::Finished)]);
        assert!(!state.hidden());
        assert_eq!(state.empty_text(), Some("No tasks"));
        assert_eq!(state.tabs(), vec![Filter::All, Filter::Finished]);
        assert_eq!(state.focus(), None);
    }

    /// After Waiting, and only while it has a task.
    #[test]
    fn the_finished_tab_shows_after_waiting_while_it_has_a_task() {
        let state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting), card("f", Status::Finished)]);
        assert_eq!(state.tabs(), vec![Filter::All, Filter::ToRefine, Filter::Waiting, Filter::Finished]);
        assert!(!keyboard(pending(&["p"])).tabs().contains(&Filter::Finished));
    }

    /// Its cards alone, each with Back to list, Edit, Speak and Remove, and
    /// no Clear all.
    #[test]
    fn the_finished_tab_lists_its_cards_with_their_buttons_and_no_clear_all() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("f1", Status::Finished), card("f2", Status::Finished)]);
        assert_eq!(uuids(&state), vec!["p"], "All leaves them out");
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::Finished)), vec![Effect::Render]);
        assert_eq!(uuids(&state), vec!["f1", "f2"]);
        assert_eq!(state.visible()[0].actions, vec![Action::Back, Action::Edit, Action::Speak, Action::Remove]);
        assert_eq!(state.focus(), focused("f1", Slot::Body).as_ref());
        assert!(!state.shows_clear_all());
        assert_eq!(key(&mut state, KeyAction::ClearAll), Vec::new());
    }

    /// Back to list reopens the task with `task status <uuid> stopped`: it
    /// leaves the tab, the focus moves on and the keyboard stays. Once the
    /// daemon's next cards have it pending, it is back on All.
    #[test]
    fn back_to_list_on_a_finished_card_puts_it_back_on_all() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("f1", Status::Finished), card("f2", Status::Finished)]);
        key(&mut state, KeyAction::Filter(Filter::Finished));
        assert_eq!(
            key(&mut state, KeyAction::Run(Action::Back)),
            vec![Effect::Focus(focused("f2", Slot::Body)), Effect::Spawn(vec![
                "task".into(), "status".into(), "f1".into(), "stopped".into(),
            ])],
        );
        assert!(state.keyboard());
        state.set_cards(&[card("p", Status::Pending), card("f1", Status::Pending), card("f2", Status::Finished)]);
        assert_eq!(state.filter(), Filter::Finished);
        assert_eq!(uuids(&state), vec!["f2"]);
        key(&mut state, KeyAction::Filter(Filter::All));
        assert_eq!(uuids(&state), vec!["p", "f1"]);
    }

    /// Finishing a task while the panel is up takes it off All and onto
    /// the Finished tab, which comes up with it.
    #[test]
    fn finishing_a_task_moves_it_to_the_finished_tab() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert!(!state.tabs().contains(&Filter::Finished));
        state.set_cards(&[card("b", Status::Pending), card("a", Status::Finished)]);
        assert_eq!(uuids(&state), vec!["b"]);
        assert!(state.tabs().contains(&Filter::Finished));
        key(&mut state, KeyAction::Filter(Filter::Finished));
        assert_eq!(uuids(&state), vec!["a"]);
    }

    /// [ and ] step from Waiting to Finished, then Ideas.
    #[test]
    fn the_brackets_step_from_waiting_to_finished_to_ideas() {
        let mut state = keyboard(vec![card("w", Status::Waiting), card("f", Status::Finished)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        key(&mut state, KeyAction::NextFilter);
        assert_eq!(state.tab(), Tab::Filter(Filter::Finished));
        key(&mut state, KeyAction::NextFilter);
        assert_eq!(state.tab(), Tab::Ideas);
        key(&mut state, KeyAction::PrevFilter);
        assert_eq!(state.tab(), Tab::Filter(Filter::Finished));
    }
```

- [ ] **Step 3: Run them to see which fail**

Run: `cargo test --lib panel:: 2>&1 | grep -E "FAILED|panicked|test result"`
Expected: `numbers_pick_the_tabs_in_order` and `seven_picks_ideas` fail (6 is still Ideas, 7 nothing). The state tests should already pass on Task 3's model; they are the spec's "panel state tests cover the new tab". If any state test fails, the model is wrong: fix it, do not bend the test.

- [ ] **Step 4: Move the keys**

In `src/panel/keys.rs` `key_action`:

```rust
        gdk::Key::_5 => KeyAction::Filter(Filter::TABS[4]),
        gdk::Key::_6 => KeyAction::Filter(Filter::TABS[5]),
        gdk::Key::_7 => KeyAction::Ideas,
```

(replacing the old `_6 => KeyAction::Ideas` line). In `enum KeyAction`: `/// 1 to 6: show this filter tab's cards, when it is shown.` and `/// 7: the Ideas tab, after the filter tabs.`

- [ ] **Step 5: Update the doc comments that name the tabs and keys**

- `src/panel/state.rs` `filter_key`: "1 to 6 pick a filter tab and 7 Ideas; [ and ] step along the tabs on show, Ideas the last, stopping at the ends."
- `src/panel/state.rs` `hidden`: "The hover and the peek show no waiting or finished task, so they hide with nothing else; the keyboard's panel still has the Waiting and Finished tabs, and hides only with no task at all."
- `src/panel/surface.rs` module doc: "a bar of filter tabs, All, Active, Planned, To refine and Waiting," → "a bar of filter tabs, All, Active, Planned, To refine, Waiting and Finished,"; "1 to 5 pick one" → "1 to 6 pick one"; "6 picks it, and ] from the last filter tab." → "7 picks it, and ] from the last filter tab."; and replace the "Waiting tasks are on the Waiting tab and nowhere else…" paragraph with:

```rust
//! Waiting tasks are on the Waiting tab and nowhere else, and the last
//! twelve finished ones on the Finished tab and nowhere else: not on All,
//! not on the hover or the peek, so a workspace whose tasks are all waiting
//! or finished shows nothing on its edge. With the keyboard it opens on
//! All, which then says it has no tasks, beside those tabs. A finished
//! card's Back to list reopens its task.
```

- `src/panel/surface.rs` `empty_line` doc: "(only All, when every task is waiting or finished)". The comment in `render` at ~:787: "Only All, with every task waiting or finished: it says so, and the tabs beside it have them."

- [ ] **Step 6: Point the e2e test at 7 for Ideas**

`tests/e2e-panel.sh`: line 495's comment "# 6 opens Ideas:" → "# 7 opens Ideas:"; line 503 and line 602 `"${NENV[@]}" wtype 6` → `"${NENV[@]}" wtype 7`. Line 602 matters: by then every task has been completed, so the Finished tab exists and 6 would pick it.

- [ ] **Step 7: Run every test**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: every `test result: ok`.

- [ ] **Step 8: Commit**

```bash
git add src/panel/keys.rs src/panel/state.rs src/panel/surface.rs tests/e2e-panel.sh
git commit -m "feat(panel): pick the Finished tab with 6 and move Ideas to 7

The tab keys follow the tabs left to right, so the Finished tab after
Waiting takes 6 and Ideas, still last, 7. State tests cover the tab:
off the hover, its buttons, no Clear all, and Back to list.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The daemon loads finished tasks and watches `completed.data`

**Files:**
- Modify: `src/daemon.rs` (module doc :6-18, `State` :42-49, `pending_data_path` and `task_db_mtime` :51-61, `cards_for_workspace` :63-80, `tick` :258-262 and :280-282, tests :347-371)

**Interfaces:**
- Consumes: `task::completed_for_tag` (Task 1); `model::cards` taking completed tasks (Task 3).

- [ ] **Step 1: Write the failing test**

Replace `pending_data_path_follows_taskdata` in `src/daemon.rs` `mod tests` with:

```rust
    /// Both files the tick watches follow TASKDATA, so a sandboxed run
    /// watches its own.
    #[test]
    fn data_paths_follow_taskdata() {
        std::env::set_var("TASKDATA", "/tmp/somewhere");
        assert_eq!(data_path("pending.data").unwrap(), std::path::PathBuf::from("/tmp/somewhere/pending.data"));
        assert_eq!(data_path("completed.data").unwrap(), std::path::PathBuf::from("/tmp/somewhere/completed.data"));
        std::env::remove_var("TASKDATA");
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --lib daemon 2>&1 | tail -5`
Expected: compile error: cannot find function `data_path`.

- [ ] **Step 3: Watch both files**

Replace `pending_data_path` and `task_db_mtime` with:

```rust
/// One of taskwarrior's data files. Honour TASKDATA so a sandboxed run
/// watches the right one.
fn data_path(file: &str) -> Option<std::path::PathBuf> {
    if let Some(d) = std::env::var_os("TASKDATA") {
        return Some(std::path::PathBuf::from(d).join(file));
    }
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".task").join(file))
}

/// The mtimes of `pending.data` and `completed.data`. Both: editing, noting
/// or removing a finished task writes `completed.data` alone, and its card
/// on the Finished tab has to follow.
fn task_db_mtimes() -> [Option<SystemTime>; 2] {
    ["pending.data", "completed.data"]
        .map(|file| data_path(file).and_then(|p| std::fs::metadata(p).ok()?.modified().ok()))
}
```

In `struct State`, replace `task_mtime: Option<SystemTime>,` with:

```rust
    /// [`task_db_mtimes`] at the last tick.
    task_mtimes: [Option<SystemTime>; 2],
```

In `tick`: `let mtime = task_db_mtime();` → `let mtimes = task_db_mtimes();`; `s.task_mtime == mtime` → `s.task_mtimes == mtimes`; `s.task_mtime = mtime;` → `s.task_mtimes = mtimes;`.

Module doc, point 2: "the mtimes of taskwarrior's `pending.data` and `completed.data` (two stats)." and its last line: "Idle cost is a socket round-trip and two stats."

- [ ] **Step 4: Load the finished tasks**

In `cards_for_workspace`, after the `tasks.extend(task::waiting_for_tag(…))` line:

```rust
    // The last few finished, for the Finished tab; the panel keeps them off
    // every other tab, the hover and the peek, as it does waiting ones.
    tasks.extend(task::completed_for_tag(&t).unwrap_or_default());
```

- [ ] **Step 5: Run every test, and lint**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked" && cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | sort | uniq -c`
Expected: every `test result: ok`, and no clippy warning in a line this branch changed (`git diff main` shows which).

- [ ] **Step 6: See it on the real panel**

Run: `cargo build --release 2>&1 | tail -1`, then follow `README.md`'s "Testing" section for running a dev daemon (or `bash tests/e2e-panel.sh` if wtype and niri are present). Check by hand on a workspace with completed tasks: Mod+Alt+Ctrl+T shows a Finished tab after Waiting; 6 opens it, newest finished on top, at most 12 cards, each aged from when it was finished; `niritasks task status <uuid> completed` on a pending task puts it at the top of Finished within a tick; `b` on a finished card takes it off Finished and it is back on All; Edit on a finished card shows the new text without any other change. Report what was and was not checked.

- [ ] **Step 7: Commit**

```bash
git add src/daemon.rs
git commit -m "feat(daemon): load the last finished tasks and watch completed.data

The panel's Finished tab needs a workspace's last 12 completed tasks.
Editing, noting or removing a finished task writes completed.data and
not pending.data, so the tick watches both files' mtimes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Docs

**Files:**
- Modify: `CONTEXT.md` (**Task action** :73-79, **Action row** :81-89, **Filter tab** :94-103, **Ideas tab** :105-114, **Task card** :65-71)
- Modify: `README.md` (key table row at :20, button table :56-60, the paragraphs at :93-121, the CLI block at :224-225)
- Modify: `llms.txt` (:12, :31)

- [ ] **Step 1: CONTEXT.md**

- **Task card**: after "with blocked tasks under the rest." add "A finished task's card, on the Finished tab alone, is dimmed, carries a check, and counts its age from when the task was finished."
- **Task action**: "a waiting task gets only Back to list, Edit, Note, Speak and Remove." → "a waiting or finished task gets only Back to list, Edit, Note, Speak and Remove, and Back to list reopens a finished one."
- **Action row**: "and a waiting task getting just Back to list, Edit, Speak and Remove" → "and a waiting or finished task getting just Back to list, Edit, Speak and Remove".
- **Filter tab**: "All, Active, Planned, To refine and Waiting" → "All, Active, Planned, To refine, Waiting and Finished"; after "and a waiting task is under Waiting alone." add "Finished lists the workspace's last 12 completed tasks, the most recently finished first, and a finished task is under it alone."
- **Ideas tab**: "after Waiting and always shown" → "after Finished and always shown"; "6 picks it" → "7 picks it".
- **Action menu**: after "A waiting task's is just Edit, Note, Speak, Update status and Move to workspace." add "So is a finished task's, from Enter on its card."

- [ ] **Step 2: README.md**

- Row `Mod+Alt+Ctrl+T` (:20): "picked with 1–5 or [ and ], narrow them to All, Active, Planned, To refine or Waiting" → "picked with 1–6 or [ and ], narrow them to All, Active, Planned, To refine, Waiting or Finished"; "The last tab, Ideas (6)," → "The last tab, Ideas (7),". Add after the Clear all clause: "Finished lists the last 12 finished tasks, and Back to list (`b`) reopens one."
- **Speak** row: "on every task, waiting ones too" → "on every task, waiting and finished ones too".
- **Up next** row: "not on a waiting task" → "not on a waiting or finished task".
- **Back to list** row: "Off waiting and back on the list — only on a waiting task, which gets just this, Edit, Speak and Remove" → "Off waiting, or reopened from finished, and back on the list — only on a waiting or finished task, which gets just this, Edit, Speak and Remove".
- Paragraph at :70-76: "and on a waiting task just Edit, Note, Speak, Update status and Move to workspace" → "and on a waiting or finished task just Edit, Note, Speak, Update status and Move to workspace".
- Paragraph at :82-85: "or on a waiting task." → "or on a waiting or finished task."
- Tabs paragraph (:93-…): "**To refine** (not planned yet) and **Waiting** (parked)" → "**To refine** (not planned yet), **Waiting** (parked) and **Finished** (the last 12 completed, the most recently finished first, each aged from when it was finished)"; "`1` to `5` pick one" → "`1` to `6` pick one"; after "Waiting tasks are on the Waiting tab only: not on All, the hover or the peek." add "Finished tasks are on the Finished tab only, the same way, and Back to list on one reopens it, back on All."
- Ideas paragraph: "It is always there, after Waiting; `6` picks it" → "It is always there, after Finished; `7` picks it".
- CLI block (:225): "#   (stopped also brings back a waiting task)" → "#   (stopped also brings back a waiting task, and reopens a completed one)".

- [ ] **Step 3: llms.txt**

- :12: "`stopped` (which also brings back a waiting task)" → "`stopped` (which also brings back a waiting task and reopens a completed one)".
- :31: "Move it to `active`, `stopped`, `waiting` or `completed`" → "Move it to `active`, `stopped` (also reopens a completed task), `waiting` or `completed`".

- [ ] **Step 4: Check nothing still says 1–5 or 6 for the tabs**

Run: `grep -n "1–5\|1 to 5\|\`6\` picks\|6 picks\|Ideas (6)" README.md CONTEXT.md llms.txt src/panel/*.rs`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add CONTEXT.md README.md llms.txt
git commit -m "docs: describe the Finished tab and stopped reopening a task

The panel's Finished tab, its key, its cards' actions and their age,
Ideas moving to 7, and task status stopped reopening a completed task.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
