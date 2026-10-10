# Open an Existing Report From the Card's Report Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On a task whose notes hold a `Report: <path>` line, the card's Report button opens that file in the browser instead of starting a Claude, its tooltip and hint read **Open report**, and `niritasks task report <uuid> --fresh` builds a new report anyway.

**Architecture:** `Task::report_path()` in `src/task.rs` reads the newest `Report: <path>` note. `TaskState` gains `has_report`, set by `TaskState::of` from the task and by `Card::state` from a new `Card.has_report`, which `cards()` sets from the task's notes. `Action::label` takes the whole `TaskState` instead of the lone `up_next` bool, so Report reads **Open report** when `has_report`; `applies` and `args` are unchanged. `niritasks task report <uuid>` gains `--fresh`: without it, a task with a report path whose file exists spawns `xdg-open` on it through niri (as the refine mod does) and notifies **Opened the report**; a path whose file is gone falls through to building a fresh one and the notification says so. Opening needs no pending or planned check; building keeps both.

**Tech Stack:** Rust (clap, serde, anyhow; `cargo test`), Taskwarrior 3.5, niri's IPC (`niri::spawn`), `xdg-open`.

**Spec:** Taskwarrior task `68618466-b589-479e-b856-edccdd4ad98f`, "feat: Open an existing report from the card's Report button". Read it with `task rc.json.array=on 68618466-b589-479e-b856-edccdd4ad98f export`; its notes are the spec and are quoted in **Global Constraints**. It builds on `docs/superpowers/plans/2026-10-10-report-a-planned-task.md`, fully landed on this branch, whose "Out of scope" named exactly this feature.

## Global Constraints

- **The report path:** `Task::report_path()` in `src/task.rs` is the newest `Report: ` note's path; `Report (earlier draft): ` notes never count. "Newest" is the last matching annotation in export order, which is the order the notes were added (the mod appends report notes in the order the reports were made).
- **The state:** `TaskState` gains `pub has_report: bool`. `TaskState::of` sets it from `task.report_path().is_some()`. The card sets `has_report` from its notes: `Card` gains `pub has_report: bool`, set in `cards()` by `t.report_path().is_some()`, false on the "+N more" card, and `Card::state` copies it in.
- **The label:** `Action::label` takes the `TaskState` (not the lone `up_next` bool). `Report.label(state)` reads `Open report` when `state.has_report`, else `Report`. `UpNext` keeps reading `Not up next` when `state.up_next`. No other label changes. `Action::applies` and `Action::args` are unchanged: Report still needs `planned && !off_list()`, and the button still runs `task report <uuid>`.
- **The command:** `niritasks task report <uuid> [--fresh]`. Without `--fresh`, a task with a report path whose file exists spawns `xdg-open` on it through niri and notifies `Opened the report: <description>`; a path whose file is gone falls through to building a fresh one and the notification reads `Report file gone, building a fresh one: <description>`. With `--fresh`, or on a task with no report path, it builds one as now. Opening needs no pending or planned check; building keeps both (`Only a pending task can be reported on.`, `Only a planned task has a plan to report on. Refine it first.`).
- **How it opens:** through niri, `niri::spawn(["sh", "-c", "xdg-open \"$1\" >/dev/null 2>&1 </dev/null &", "sh", <path>])`, the mod's own `openArgv` in `claude/refine-mod/hooks/report.ts`, so the browser is niri's child and outlives this process.
- **Docs:** README button row and commands block, llms.txt row (the command tests need `--fresh` shown being run), CONTEXT.md Task action.
- **Done when:** a card with a `Report:` note shows Open report and pressing `p` opens the file; a card without builds one as now; `task report --fresh` builds one on a task that has a report; `cargo test` passes, the README and llms.txt command tests included.
- **Out of scope:** a second button or armed second press for a fresh report; removing or relabelling old `Report:` notes.
- **Commits:** Conventional Commits, subject at most 72 characters, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. The task's type is `feat`.
- **Tests stay green at every commit.** `cargo test` checks that README's Commands block and llms.txt run every subcommand and flag, so `--fresh`'s doc lines land in the same commit as the flag.

## File Structure

- `src/task.rs` (modify): `Task::report_path()` and its tests.
- `src/actions.rs` (modify): `TaskState.has_report`, `TaskState::of`, `Action::label(TaskState)`, and the label tests.
- `src/panel/actions.rs` (modify): `hint` passes the state to `label`; hint tests for Open report.
- `src/panel/model.rs` (modify): `Card.has_report`, set in `cards()` and `cap()`, read in `Card::state`; tests.
- `src/panel/state.rs` (modify): the `card` test fixture's literal; `advance`'s notify label.
- `src/panel/surface.rs` (modify): the tooltip call.
- `src/panel/projects.rs`, `src/main.rs` (modify): the other `label(false)` callers.
- `src/refine.rs` (modify): `open_report_command(path)` and `open_report(path)`, with a test.
- `src/main.rs` (modify): `TaskCommand::Report { uuid, fresh }` and its handler; a parse test.
- `README.md`, `llms.txt`, `CONTEXT.md` (modify): the `--fresh` lines with the command, the button row and Task action with the docs.

---

### Task 1: `Task::report_path()`

**Files:**
- Modify: `src/task.rs` (the `impl Task` block after `is_up_next`, around line 100; the tests module)

**Interfaces:**
- Produces: `pub fn report_path(&self) -> Option<&str>` on `Task`. Later tasks call `t.report_path().is_some()` and `t.report_path()` for the path to open.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/task.rs`, after `planned_is_matched_exactly`:

```rust
    /// The report the mod linked from the task: the last `Report: <path>`
    /// note, which is the newest, the mod appending them in the order the
    /// reports were made.
    #[test]
    fn the_newest_report_note_is_the_tasks_report() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Goal: x"},
                {"entry":"20261006T020000Z","description":"Report: /r/first.html"},
                {"entry":"20261006T030000Z","description":"Report: /r/second.html"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), Some("/r/second.html"));
    }

    /// A task with no report note, or no notes at all, has no report.
    #[test]
    fn a_task_without_a_report_note_has_no_report() {
        let noted: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[{"entry":"20261006T010000Z","description":"Goal: x"}]}"#,
        )
        .unwrap();
        assert_eq!(noted.report_path(), None);
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(bare.report_path(), None);
    }

    /// A report of an earlier draft of the plan is not the plan's report,
    /// and a note that merely mentions a report is not a link to one.
    #[test]
    fn an_earlier_drafts_report_and_a_mention_are_not_the_report() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Report (earlier draft): /r/old.html"},
                {"entry":"20261006T020000Z","description":"See the Report: /r/not-this.html"},
                {"entry":"20261006T030000Z","description":"Report:/r/no-space.html"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), None);
    }

    /// The path is the rest of the line, trimmed, so a note written with a
    /// trailing space still opens.
    #[test]
    fn the_report_path_is_trimmed() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[{"entry":"20261006T010000Z","description":"Report: /r/a b.html "}]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), Some("/r/a b.html"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib task::tests::the_newest_report_note_is_the_tasks_report task::tests::a_task_without_a_report_note_has_no_report task::tests::an_earlier_drafts_report_and_a_mention_are_not_the_report task::tests::the_report_path_is_trimmed`
Expected: compile error, `no method named report_path found`.

- [ ] **Step 3: Implement `report_path`**

In `src/task.rs`, add a constant beside `UP_NEXT_TAG` (after its doc comment and definition):

```rust
/// What a note linking the task's refine report starts with, as the refine
/// mod writes it: `Report: <path>`. `Report (earlier draft): <path>` marks a
/// report of a plan since changed, and is not the task's report.
pub const REPORT_NOTE: &str = "Report: ";
```

And in `impl Task`, after `is_up_next`:

```rust
    /// The refine report linked from the task's notes: the path of the newest
    /// `Report: <path>` note, the last one, the mod appending them in the
    /// order the reports were made. None on a task with no such note; a
    /// `Report (earlier draft):` note is a report of a plan since changed,
    /// and never counts.
    pub fn report_path(&self) -> Option<&str> {
        self.annotations
            .iter()
            .rev()
            .find_map(|a| a.description.strip_prefix(REPORT_NOTE))
            .map(str::trim)
            .filter(|p| !p.is_empty())
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib task::tests`
Expected: all pass, the four new ones included.

- [ ] **Step 5: Commit**

```bash
git add src/task.rs
git commit -m "feat(task): read the report a task's notes link" -m "The newest Report: <path> note is the task's report, so the card and
the CLI can open it instead of building another. A Report (earlier
draft): note is a report of a plan since changed, and never counts.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: `TaskState.has_report` and `Action::label(TaskState)`

**Files:**
- Modify: `src/actions.rs` (the `TaskState` struct around line 58, `TaskState::of` around line 85, `Action::label` around line 236, the tests)
- Modify: `src/panel/actions.rs:65,76` (`hint`'s two `label` calls)
- Modify: `src/panel/surface.rs:1071` (the tooltip)
- Modify: `src/panel/state.rs:906` (`advance`'s notification)
- Modify: `src/panel/projects.rs:224`, `src/panel/state.rs:2093` (test), `src/main.rs:353,528` (the `label(false)` callers)
- Modify: `src/panel/model.rs:837` (the exhaustive `TaskState` literal in `a_cards_state_is_its_tasks`)

**Interfaces:**
- Consumes: `Task::report_path()` from Task 1.
- Produces: `TaskState { …, pub has_report: bool }`; `pub fn label(self, state: TaskState) -> &'static str`. Task 3 sets `has_report` from the card; Task 4's notification uses the words `Opened the report`, not this label.

- [ ] **Step 1: Write the failing tests in `src/actions.rs`**

Replace the three label tests `labels_read_as_the_tooltips_do`, `no_two_actions_share_a_label` and `up_next_reads_as_the_step_it_takes` with these, and add the `of` test:

```rust
    #[test]
    fn labels_read_as_the_tooltips_do() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label(TaskState::default())).collect();
        assert_eq!(
            labels,
            vec![
                "Go to session", "Back to list", "Start working", "Refine", "Grill me", "Report", "Edit",
                "Speak", "Up next", "Complete", "Move to workspace", "Stop", "Waiting", "Remove",
            ]
        );
    }

    /// Each is its button's tooltip, so no two may share one, whichever way
    /// Up next and Report read.
    #[test]
    fn no_two_actions_share_a_label() {
        for up_next in [false, true] {
            for has_report in [false, true] {
                let state = TaskState { up_next, has_report, ..TaskState::default() };
                let mut labels: Vec<&str> = Action::ALL.iter().map(|a| a.label(state)).collect();
                labels.sort();
                labels.dedup();
                assert_eq!(labels.len(), Action::ALL.len(), "{state:?}");
            }
        }
    }

    /// Up next reads as the step it takes: Not up next on a task already up
    /// next. That is also the word `task up-next` notifies with, for where
    /// the task is once the step is taken. Report reads as what a press does:
    /// Open report on a task whose notes link one, which the press opens
    /// instead of building another. No other action changes its words.
    #[test]
    fn up_next_and_report_read_as_the_step_they_take() {
        let plain = TaskState::default();
        let up_next = TaskState { up_next: true, ..plain };
        let reported = TaskState { has_report: true, ..plain };
        assert_eq!(UpNext.label(plain), "Up next");
        assert_eq!(UpNext.label(up_next), "Not up next");
        assert_eq!(UpNext.label(reported), "Up next");
        assert_eq!(Report.label(plain), "Report");
        assert_eq!(Report.label(reported), "Open report");
        assert_eq!(Report.label(up_next), "Report");
        let every_way = TaskState { active: true, planned: true, has_session: true, up_next: true, has_report: true, ..plain };
        for action in Action::ALL.into_iter().filter(|a| !matches!(a, UpNext | Report)) {
            assert_eq!(action.label(every_way), action.label(plain), "{action:?}");
        }
    }

    /// A task's state reads whether its notes link a report off the task
    /// itself, as it reads planned and up next.
    #[test]
    fn of_reads_has_report_from_the_tasks_notes() {
        let with: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","status":"pending","tags":["planned"],
                "annotations":[{"entry":"20261006T010000Z","description":"Report: /r/a.html"}]}"#,
        )
        .unwrap();
        let without: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","status":"pending","tags":["planned"]}"#).unwrap();
        assert_eq!(
            TaskState::of(&with, false, false),
            TaskState { planned: true, has_report: true, ..TaskState::default() }
        );
        assert_eq!(TaskState::of(&without, false, false), TaskState { planned: true, ..TaskState::default() });
    }
```

Also, in `report_needs_a_plan_and_a_task_on_the_list`, add one line at the end, so the constraint "applies unchanged" is pinned:

```rust
        // A linked report changes the words, never who gets the button.
        assert!(!Report.applies(TaskState { has_report: true, ..TaskState::default() }));
        assert!(!Report.applies(TaskState { planned: true, has_report: true, waiting: true, ..TaskState::default() }));
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib actions::tests`
Expected: compile error, `no field has_report` and `expected bool, found TaskState`.

- [ ] **Step 3: Add `has_report` to `TaskState` and `of`**

In `src/actions.rs`, in `pub struct TaskState`, after `has_session`:

```rust
    /// Its notes link a refine report, `Report: <path>`: Report opens that
    /// rather than building another.
    pub has_report: bool,
```

In `TaskState::of`, after `has_session,`:

```rust
            has_report: task.report_path().is_some(),
```

- [ ] **Step 4: Change `Action::label` to take the state**

Replace `label` in `src/actions.rs`:

```rust
    /// The action's words, on its button's tooltip and in the card's hint.
    /// Two read as the step they take on this task: Up next reads Not up
    /// next on a task already up next, and Report reads Open report on a
    /// task whose notes link a report, which a press opens instead of
    /// building another.
    pub fn label(self, state: TaskState) -> &'static str {
        match self {
            Session => "Go to session",
            Back => "Back to list",
            Start => "Start working",
            Refine => "Refine",
            Grill => "Grill me",
            Report if state.has_report => "Open report",
            Report => "Report",
            Edit => "Edit",
            Speak => "Speak",
            UpNext if state.up_next => "Not up next",
            UpNext => "Up next",
            Complete => "Complete",
            Move => "Move to workspace",
            Stop => "Stop",
            Wait => "Waiting",
            Remove => "Remove",
        }
    }
```

Also update the `Report` variant's doc comment at the top of the enum to:

```rust
    /// The refine report of a planned task's plan. On a task whose notes
    /// link one, opens that file in the browser; otherwise Claude builds it
    /// in the workspace's herdr session, opens it and links it from the
    /// task's notes, without refining again.
    Report,
```

- [ ] **Step 5: Fix every caller**

`src/panel/actions.rs`, in `hint`: line 65 `action.label(state.up_next)` becomes `action.label(state)`; line 76 `step.label(false)` becomes `step.label(state)`. (Ctrl+Enter never presses Report or Up next, so the state changes nothing there; passing it keeps one shape.)

`src/panel/surface.rs:1071`: `action.label(state.up_next)` becomes `action.label(state)`. Update the comment above it to: `// The icon's name in words, under the pointer and for a screen reader. Up next reads Not up next on a task already up next, and Report reads Open report on a task whose notes link one.`

`src/panel/state.rs:906`, in `advance`: `action.label(false)` becomes `action.label(card.state.unwrap_or_default())`. (`card.state` is `Option<TaskState>` and is `Some` here, the `and_then` above having used it; `unwrap_or_default` avoids a second `let Some`.)

`src/panel/projects.rs:224`: `Action::Move.label(false)` becomes `Action::Move.label(TaskState::default())`. Line 8's `use crate::actions::Action;` becomes `use crate::actions::{Action, TaskState};`.

`src/panel/state.rs:2093` (test): `Action::Move.label(false)` becomes `Action::Move.label(TaskState::default())`.

`src/main.rs:353`: `Action::UpNext.label(t.is_up_next())` becomes `Action::UpNext.label(TaskState::of(&t, false, false))`. In the `use niri_tasks::{ … }` block at line 11, `actions::Action,` becomes `actions::{Action, TaskState},`.

`src/main.rs:528` (test): `action.label(false)` becomes `action.label(TaskState::default())`.

`src/panel/model.rs:837` (test `a_cards_state_is_its_tasks`): the full literal gains `has_report: false` after `has_session: true`. (Task 3 changes this test again; for now it only has to compile.)

- [ ] **Step 6: Build and run every test**

Run: `cargo build 2>&1 | grep -E "^(error|warning)" ; cargo test 2>&1 | tail -5`
Expected: no errors or warnings; every test passes. If `grep` finds another `label(` caller the list above missed, fix it the same way: a `TaskState` where a `bool` was.

- [ ] **Step 7: Commit**

```bash
git add src/actions.rs src/panel/actions.rs src/panel/surface.rs src/panel/state.rs src/panel/projects.rs src/panel/model.rs src/main.rs
git commit -m "feat(actions): read Open report on a task with a linked report" -m "TaskState gains has_report, read off the task's Report: notes, and
Action::label takes the state rather than the lone up_next bool, so
Report reads Open report when a press will open the linked file instead
of building another. Who gets the button and what it runs are unchanged.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: The card carries `has_report`, and the hint reads Open report

**Files:**
- Modify: `src/panel/model.rs` (the `Card` struct around line 30, `cards()` around line 288, `cap()` around line 333, `Card::state` around line 107, tests)
- Modify: `src/panel/state.rs:940` (the `card` test fixture)
- Modify: `src/panel/actions.rs` (hint tests)

**Interfaces:**
- Consumes: `Task::report_path()` (Task 1); `TaskState.has_report` and `Action::label(TaskState)` (Task 2).
- Produces: `Card { …, pub has_report: bool }`.

- [ ] **Step 1: Write the failing tests in `src/panel/model.rs`**

Add after `a_card_carries_its_tasks_notes`:

```rust
    /// A card knows whether its task's notes link a report, so its Report
    /// button can read Open report; "+N more" stands for no one task.
    #[test]
    fn a_card_knows_its_task_has_a_report() {
        let mut reported = planned("reported", false);
        reported.annotations = vec![
            crate::task::Annotation { entry: "20261006T010000Z".into(), description: "Goal: x".into() },
            crate::task::Annotation { entry: "20261006T020000Z".into(), description: "Report: /r/a.html".into() },
        ];
        let mut drafted = planned("drafted", false);
        drafted.annotations = vec![crate::task::Annotation {
            entry: "20261006T010000Z".into(),
            description: "Report (earlier draft): /r/old.html".into(),
        }];
        let got = cards(&todo(vec![reported, drafted, planned("plain", false)]), &[]);
        let by = |text: &str| got.iter().find(|c| c.text == text).unwrap();
        assert!(by("reported").has_report);
        assert!(!by("drafted").has_report, "an earlier draft's report is not the plan's");
        assert!(!by("plain").has_report);
        let many: Vec<Task> = (0..CAP + 1).map(|i| planned(&format!("t{i}"), false)).collect();
        assert!(!cap(&cards(&todo(many), &[]), CAP).last().unwrap().has_report);
    }
```

Replace `a_cards_state_is_its_tasks` with:

```rust
    /// A card's status, tags and report are its task's state; "+N more" has none.
    #[test]
    fn a_cards_state_is_its_tasks() {
        let card = |status, planned, up_next, has_report| Card {
            status,
            text: "t".into(),
            uuid: Some("u".into()),
            id: 0,
            planned,
            up_next,
            since: String::new(),
            notes: Vec::new(),
            has_report,
        };
        assert_eq!(
            card(Status::Active, true, true, true).state(true),
            Some(TaskState { active: true, waiting: false, finished: false, planned: true, up_next: true, has_session: true, has_report: true })
        );
        assert_eq!(
            card(Status::Waiting, false, false, false).state(false),
            Some(TaskState { waiting: true, ..TaskState::default() })
        );
        assert_eq!(card(Status::Blocked, false, false, false).state(false), Some(TaskState::default()));
        assert_eq!(
            card(Status::Planned, true, false, true).state(false),
            Some(TaskState { planned: true, has_report: true, ..TaskState::default() })
        );
        let more = cap(&[card(Status::Pending, false, false, false), card(Status::Pending, false, false, false)], 1).pop().unwrap();
        assert_eq!(more.status, Status::More);
        assert_eq!(more.state(true), None);
    }
```

Check the `Annotation` struct's fields are `pub` (they are: `pub entry`, `pub description` in `src/task.rs`), and that the test module's `planned` and `todo` helpers exist (they do, around line 389 and above).

- [ ] **Step 2: Write the failing hint tests in `src/panel/actions.rs`**

In the tests module, after the `with_claude` helper, add:

```rust
    fn with_report(state: TaskState) -> TaskState {
        TaskState { has_report: true, ..state }
    }
```

Add after `a_lettered_button_hints_its_letter_and_name`:

```rust
    /// On a task whose notes link a report, Report's hint reads Open report,
    /// as its tooltip does: the press opens that file rather than building
    /// another. Ctrl+Enter's words are untouched.
    #[test]
    fn report_hints_open_report_on_a_task_with_one() {
        assert_eq!(hint(with_report(on_list(true)), Some(Report)), "p: Open report · Ctrl+Enter: Start working");
        assert_eq!(hint(with_report(active(true)), Some(Report)), "p: Open report");
        assert_eq!(hint(with_report(with_claude(active(true))), Some(Report)), "p: Open report · Ctrl+Enter: Go to session");
        // A report changes Report's words alone.
        assert_eq!(hint(with_report(on_list(true)), Some(Start)), "s: Start working · Ctrl+Enter: Start working");
        assert_eq!(hint(with_report(on_list(true)), None), "Space: view notes · Ctrl+Enter: Start working");
    }
```

And in `every_state()`, so the exhaustive hint checks cover a reported card too, replace the body:

```rust
    /// Every state a task card can be in, with a Claude on it and without,
    /// and with a report linked and without.
    fn every_state() -> Vec<TaskState> {
        let mut all = Vec::new();
        for planned in [false, true] {
            for state in [on_list(planned), active(planned), waiting(planned), finished(planned)] {
                for state in [state, with_claude(state)] {
                    all.push(state);
                    all.push(with_report(state));
                }
            }
        }
        all
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib panel::model::tests panel::actions::tests`
Expected: compile error, `struct Card has no field named has_report`.

- [ ] **Step 4: Add `has_report` to `Card`**

In `src/panel/model.rs`, in `pub struct Card`, after `notes`:

```rust
    /// The task's notes link a refine report, `Report: <path>`: its Report
    /// button reads Open report and opens that. False on "+N more".
    pub has_report: bool,
```

In `cards()`, in the `Card { … }` literal, after `notes: …,`:

```rust
            has_report: t.report_path().is_some(),
```

In `cap()`, in the "+N more" literal, after `notes: Vec::new(),`:

```rust
            has_report: false,
```

In `Card::state`, after `has_session,`:

```rust
            has_report: self.has_report,
```

In `src/panel/state.rs:940`, the test fixture's literal gains `has_report: false` after `notes: Vec::new()`.

- [ ] **Step 5: Run every test**

Run: `cargo test 2>&1 | tail -5`
Expected: every test passes, the new model and hint tests included. If `the_hint_leaves_enter_and_ctrl_delete_to_the_footer` or any other exhaustive hint test fails on a reported state, read its message: "Open report" contains neither "Enter" nor "Ctrl+Del", so it should not.

- [ ] **Step 6: Commit**

```bash
git add src/panel/model.rs src/panel/state.rs src/panel/actions.rs
git commit -m "feat(panel): show Open report on a card whose notes link one" -m "The card reads has_report off its task's notes and carries it into its
state, so the Report button's tooltip and hint read Open report when
pressing it will open the linked file.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: `niritasks task report <uuid> [--fresh]` opens the linked report

**Files:**
- Modify: `src/refine.rs` (after `spawn_quick`, around line 375; tests)
- Modify: `src/main.rs` (`TaskCommand::Report` around line 174; its handler around line 383; CLI tests around line 540)
- Modify: `README.md:273-274` (the commands block), `llms.txt:44` (the commands table)

**Interfaces:**
- Consumes: `Task::report_path()` (Task 1); `niri::spawn(Vec<String>) -> Result<()>`; `notify::tasks(&str)`; `refine::launch(&Workspace, &Task, Mode)`.
- Produces: `pub fn open_report_command(path: &str) -> Vec<String>` and `pub fn open_report(path: &str) -> Result<()>` in `src/refine.rs`; the `--fresh` flag.

- [ ] **Step 1: Write the failing test in `src/refine.rs`**

Add to the tests module, after `quick_command_is_the_refine_buttons_command`:

```rust
    /// Open report runs what the mod runs when it opens a report it just
    /// wrote (`openArgv` in `hooks/report.ts`): `xdg-open` detached under a
    /// shell, the path passed as an argument and never quoted into the
    /// script, so a path with a space or a quote opens the same file.
    #[test]
    fn open_report_runs_xdg_open_on_the_path_as_an_argument() {
        let command = open_report_command("/home/x/.local/share/niri-tasks/reviews/a report's.html");
        assert_eq!(command[0], "sh");
        assert_eq!(command[1], "-c");
        assert_eq!(command[2], "xdg-open \"$1\" >/dev/null 2>&1 </dev/null &");
        assert_eq!(command[3], "sh");
        assert_eq!(command[4], "/home/x/.local/share/niri-tasks/reviews/a report's.html");
        assert_eq!(command.len(), 5);
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --lib refine::tests::open_report_runs_xdg_open_on_the_path_as_an_argument`
Expected: compile error, `cannot find function open_report_command`.

- [ ] **Step 3: Implement `open_report_command` and `open_report`**

In `src/refine.rs`, after `spawn_quick`:

```rust
/// The command that opens a report already written, `path`, in the browser:
/// what the mod runs when it opens one it has just written (`openArgv` in
/// `hooks/report.ts`). `xdg-open` detached under a shell, so the browser
/// is not waited on, with the path as the shell's `$1`, never quoted into
/// the script.
pub fn open_report_command(path: &str) -> Vec<String> {
    vec![
        "sh".to_string(),
        "-c".to_string(),
        "xdg-open \"$1\" >/dev/null 2>&1 </dev/null &".to_string(),
        "sh".to_string(),
        path.to_string(),
    ]
}

/// Open the report at `path` in the browser, through niri, so the browser
/// is niri's child and outlives the `niritasks` process a button press
/// spawned. The caller has checked the file is there.
pub fn open_report(path: &str) -> Result<()> {
    niri::spawn(open_report_command(path)).with_context(|| format!("could not open the report {path}"))
}
```

`src/refine.rs` already imports `anyhow::{bail, Context, Result}` and `niri`, so nothing else is needed.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib refine::tests`
Expected: all pass.

- [ ] **Step 5: Write the failing CLI parse test in `src/main.rs`**

Add to the tests module after `going_to_a_tasks_session_is_a_command`:

```rust
    /// A card's Report runs `task report <uuid>`; `--fresh` is how a script,
    /// or a person, gets a second report on a task that already has one.
    #[test]
    fn a_fresh_report_is_a_flag_on_task_report() {
        assert!(Cli::try_parse_from(["niritasks", "task", "report", "c53b6e3d"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "report", "c53b6e3d", "--fresh"]).is_ok());
    }
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cargo test --bin niritasks a_fresh_report_is_a_flag_on_task_report`
Expected: FAIL, the second assertion (`unexpected argument '--fresh'`). (If the binary target has another name, `grep -n '^name\|\[\[bin\]\]' Cargo.toml` says; `cargo test a_fresh_report` with no target flag also works.)

- [ ] **Step 7: Add the flag and the handler**

In `src/main.rs`, replace the `Report` variant of `TaskCommand`:

```rust
    /// Open the report of a planned task's plan, or have Claude build it in a new tab of the workspace's herdr session
    ///
    /// On a task whose notes link a report as `Report: <path>`, opens that
    /// file in the browser, whatever the task's status. Otherwise, or with
    /// --fresh, Claude builds the HTML report of the plan the task already
    /// holds, as a refine's "Show me a report first" does, opens it in the
    /// browser and links it from the task's notes, without refining again;
    /// that is refused for a task that is not pending or not tagged planned:
    /// refine it first. A linked file that is gone is built afresh, and the
    /// notification says so. The workspace is this herdr session's in a
    /// herdr pane, else the focused one.
    Report {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// Build a new report even when the task's notes link one
        #[arg(long)]
        fresh: bool,
    },
```

Replace the handler arm:

```rust
        TaskCommand::Report { uuid, fresh } => {
            let t = task::get(&uuid)?.context("task not found")?;
            // Opening what is already written needs no session and no
            // pending or planned check: the report is a file, and reading it
            // is harmless on any task. Building one needs both.
            let mut gone = None;
            if !fresh {
                if let Some(path) = t.report_path() {
                    if Path::new(path).is_file() {
                        refine::open_report(path)?;
                        notify::tasks(&format!("Opened the report: {}", t.description));
                        return Ok(());
                    }
                    gone = Some(path.to_string());
                }
            }
            let workspace = Workspace::of_caller()?;
            anyhow::ensure!(t.status == "pending", "Only a pending task can be reported on.");
            // An unplanned task has no plan to report on: the card offers
            // Refine or Grill me instead, and a script gets the same refusal.
            anyhow::ensure!(t.is_planned(), "Only a planned task has a plan to report on. Refine it first.");
            if gone.is_some() {
                notify::tasks(&format!("Report file gone, building a fresh one: {}", t.description));
            }
            refine::launch(&workspace, &t, refine::Mode::Report)?;
        }
```

`src/main.rs` does not import `Path` yet: add `use std::path::Path;` after line 10's `use clap::{Parser, Subcommand};`. Note the order: the workspace is resolved after the open path, so a report is opened even from a pane whose herdr session matches no workspace, which would refuse a build.

- [ ] **Step 8: Add `--fresh` to the docs the command tests read**

`cargo test`'s `readme_commands_block_runs_every_subcommand_and_flag` and `llms_txt_runs_every_subcommand_and_flag` fail until both files show `--fresh` being run, so these edits go in this commit.

In `README.md`, replace the two `task report` lines in the `## Commands` fenced block (currently lines 273–274):

```
niritasks task report <uuid>       # open the report of its plan linked from its notes; without one, a report
                                   #   built by Claude in the workspace's herdr session, opened in the browser
                                   #   and linked from the task's notes (a planned task only)
niritasks task report <uuid> --fresh  #   build a new report even when its notes link one
```

In `llms.txt`, replace the `task report` row (line 44) with two rows:

```
| `niritasks task report <uuid>` | On a task whose notes link a report as `Report: <path>`, open that file in the browser and notify `Opened the report`, whatever the task's status. Otherwise open a tab in the workspace's herdr session with Claude building the HTML report of the task's plan (the `report-task` skill), opening it in the browser and adding a `Report: <path>` note to the task; that is refused for a task not pending or not tagged `planned`: refine it first. A linked file that is gone is built afresh, and the notification says so | GUI (browser), or herdr tab, follows the herdr session, else focus |
| `niritasks task report <uuid> --fresh` | Build a new report even when the task's notes link one; the same refusals as a build | herdr tab, follows the herdr session, else focus |
```

- [ ] **Step 9: Run every test**

Run: `cargo test 2>&1 | tail -5`
Expected: every test passes, `a_fresh_report_is_a_flag_on_task_report`, `every_task_action_is_a_command_the_cli_accepts`, `readme_commands_block_runs_every_subcommand_and_flag` and `llms_txt_runs_every_subcommand_and_flag` included.

- [ ] **Step 10: Check it by hand**

Build and run against a real task with a `Report:` note, in a terminal on this workspace (the daemon's panel is not needed):

```bash
cargo build
u=$(task +planned status:pending export rc.json.array=on | python3 -c 'import json,sys; print(next(t["uuid"] for t in json.load(sys.stdin) if any(a["description"].startswith("Report: ") for a in t.get("annotations",[]))))')
./target/debug/niritasks task report "$u"
```

Expected: the browser opens the report, and a "Tasks" notification reads `Opened the report: <description>`. No herdr tab opens. If no such task exists, add a `Report:` note to a planned test task pointing at any existing `.html` under `~/.local/share/niri-tasks/reviews/` with `niritasks task note <uuid> "Report: <path>"`, then remove the note afterwards with `niritasks task edit <uuid>`.

Then the gone file:

```bash
niritasks task note "$u" "Report: /nonexistent/gone.html"
./target/debug/niritasks task report "$u"
```

Expected: a notification `Report file gone, building a fresh one: <description>`, then a "Report: …" herdr tab opens as before (close it; it is a real Claude). Remove the test note afterwards with `niritasks task edit "$u"` (delete the `Report: /nonexistent/gone.html` row and save), so the task's real report is the newest again.

Then `--fresh`: `./target/debug/niritasks task report "$u" --fresh` opens a "Report: …" tab straight away, with no notification about the file. Close it.

- [ ] **Step 11: Commit**

```bash
git add src/refine.rs src/main.rs README.md llms.txt
git commit -m "feat(task): open a task's linked report, or build one with --fresh" -m "niritasks task report opens the report a task's notes link, through
niri with xdg-open as the mod does, instead of starting a Claude to
build another; a linked file that is gone falls through to a build and
the notification says so. --fresh builds one regardless. Opening needs
no pending or planned check; building keeps both.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: Docs and the full test run

**Files:**
- Modify: `README.md:21` (the keybind row), `README.md:54` (the button row)
- Modify: `CONTEXT.md` (**Task action**, around line 98; **Refine report**, around line 55)

**Interfaces:**
- Consumes: nothing new. The words it documents are Task 2's `Open report`, Task 4's `--fresh` and `Opened the report`.

- [ ] **Step 1: README's button row**

Replace line 54 of `README.md`:

```
| **Report** (mauve) | `p` | Claude builds the HTML report of the task's plan — what changes, what needs your eye, diagrams, files and checks — opens it in the browser and adds a `Report: <path>` note to the task, without refining again — only on a planned task on the list. Once the task has that note the button reads **Open report** and opens the file instead; `niritasks task report <uuid> --fresh` builds a new one |
```

- [ ] **Step 2: README's keybind row**

In line 21 of `README.md`, change the words `see a report of that plan once it has one,` to `see a report of that plan once it has one (or open the report it already has),`. Nothing else on the line changes.

- [ ] **Step 3: CONTEXT.md**

In **Task action** (around line 98), change `Which ones a task gets goes by its state: only a` so the sentence before it reads the words too. Replace:

```
workspace, Stop, Waiting or Remove — with its words, its icon and the
`niritasks` command it runs. Which ones a task gets goes by its state: only a
```

with:

```
workspace, Stop, Waiting or Remove — with its words, its icon and the
`niritasks` command it runs. Two read as the step they take: Up next reads
Not up next on a task already up next, and Report reads Open report on a task
whose notes link a refine report, which it opens instead of building another.
Which ones a task gets goes by its state: only a
```

In **Refine report** (around line 55), replace:

```
task card's Report, on the plan the task already holds. It is saved under the
reviews folder and linked from the task's notes as `Report: <path>`.
```

with:

```
task card's Report, on the plan the task already holds. It is saved under the
reviews folder and linked from the task's notes as `Report: <path>`; once
linked, the card's Report opens it, and `niritasks task report <uuid> --fresh`
builds another.
```

- [ ] **Step 4: Run the whole suite**

Run: `cargo test 2>&1 | tail -5`
Expected: every test passes. Then `tests/all.sh`; expected: every suite that can run here passes, and one that cannot says SKIP. `e2e-panel.sh` and `e2e-box.sh` take the screen and keyboard, so leave the machine alone while they run; if they cannot run, say so in the task report rather than skipping silently.

- [ ] **Step 5: Check the card by hand**

With the daemon running the new binary (`install.sh` or however this machine installs it, then restart the `niri-tasks` user unit), press `Mod+Alt+Ctrl+T` on a workspace with a planned task whose notes hold a `Report:` line. Expected: the focused card's Report button's tooltip and hint read `p: Open report`, and pressing `p` opens the file in the browser with the `Opened the report` notification. A planned card without the note still reads `p: Report` and opens a "Report: …" tab.

- [ ] **Step 6: Commit**

```bash
git add README.md CONTEXT.md
git commit -m "docs: describe Open report on a card with a linked report" -m "The button row, keybind row, Task action and Refine report entries say
that Report opens the report a task's notes already link, and that
task report --fresh builds another.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

## Self-review

- **Spec coverage.** `Task::report_path()`, newest `Report:` note, earlier drafts never counting: Task 1. `TaskState.has_report`, `Action::label(TaskState)`, Open report, applies and args unchanged: Task 2. Card sets `has_report` from its notes: Task 3. `task report <uuid> [--fresh]`, xdg-open through niri, `Opened the report`, the gone file falling through with its own notification, no pending or planned check on open, both kept on build: Task 4. README button row and commands block, llms.txt row with `--fresh` run, CONTEXT.md Task action: Tasks 4 and 5. Done-when: the card check is Task 5 step 5, `--fresh` on a reported task is Task 4 step 10, `cargo test` with the command tests is Task 4 step 9 and Task 5 step 4. Out of scope is left alone: no second button, no armed press, no note edits.
- **Placeholders.** None: every code step has its code and every doc step its text.
- **Type consistency.** `report_path(&self) -> Option<&str>` is used as `.is_some()` in Tasks 2 and 3 and as `Some(path)` with `Path::new(path)` in Task 4. `has_report: bool` has the same name on `TaskState` and `Card`. `label(self, state: TaskState)` is called with a `TaskState` everywhere, `TaskState::default()` where no task is at hand. `open_report_command(&str) -> Vec<String>` and `open_report(&str) -> Result<()>` are defined in Task 4 and used only there.
