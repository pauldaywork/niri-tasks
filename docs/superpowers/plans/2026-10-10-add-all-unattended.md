# Add & All: Add, Refine and Report Unattended Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A third button on the add task box, **Add & all**, adds the task as Add does, then refines it and builds its report with Claude in the background, asking nothing, so the task comes back planned with a `Report:` note while its card shows it is processing and offers no actions until then.

**Architecture:** The box's `Submission.refine` becomes a `Then` enum (`Nothing`, `Refine`, `All`); `All` makes the daemon spawn `niritasks task refine <uuid> --unattended` through niri, as Add & refine spawns the attended refine. That command runs `claude -p` twice itself, in the project folder, behind the refine tab's own fence (the flags shared through `session::refiner_flags`, the `--settings` from `refine::session_settings` with a new `unattended` mod option) plus `--permission-prompts none`: `/refine-task <uuid> auto`, then, once the task is `+planned`, `/report-task <uuid>`. The mod's `unattended` option makes `write_task_plan` write without `$.ui.ask` and `show_task_report` not open the browser. While the run lasts the task carries `+processing`: `TaskState.processing` makes `Action::applies` offer nothing, the card is drawn dimmed with an hourglass, and `task refine`, `task report` and `task start` refuse it.

**Tech Stack:** Rust (clap, serde_json, anyhow; `cargo test`), the TypeScript Claude Code mod in `claude/refine-mod` (`claude plugin test`, `claude plugin validate`), Claude Code 2.1.296 (`claude -p --permission-prompts none`), Taskwarrior 3.5, bash (`tests/all.sh`, `try.sh`).

**Spec:** Taskwarrior task `aa677524-02f9-4187-b0ce-e7af156576a7`, "feat: Add & all: add, refine and report unattended". Read it with `task rc.json.array=on aa677524-02f9-4187-b0ce-e7af156576a7 export`; its notes are the spec and are quoted in **Global Constraints**.

## Global Constraints

- **The button:** **Add & all**, after **Add & refine** on the add box's footer, add mode only, with no keyboard shortcut. The hint, `keys.rs`, the IPC request and the keybinds are unchanged. The pointer path is checked by hand (`try.sh`), as the README says for the box.
- **Headless, not a tab:** `niritasks task refine <uuid> --unattended` runs `claude -p` itself in the project folder (the dir a refine's `Session` uses), with the same `--settings` fence (`refine::session_settings`), `--disallowedTools`, `--append-system-prompt` and `--plugin-dir` as `agent_start_claude_refiner`, plus `--permission-prompts none` and `--permission-mode default`. No herdr, no window. Not plan mode, not auto mode. The flag list is shared with the tab's argv builder (`session::refiner_flags`) so the two cannot drift. `--unattended` conflicts with `--grill`: grill is never unattended.
- **Two runs in one process:** `claude -p "/refine-task <uuid> auto"`, then, if the task is now `+planned`, `claude -p "/report-task <uuid>"`. Each run's stdout and stderr go to `$XDG_DATA_HOME/niri-tasks/unattended/<uuid8>-<stamp>.log` (beside `reviews`), one file per process, so a failure can be read later.
- **The mod's option:** `unattended` boolean in `plugin.json`'s `userConfig`, set by `session_settings` for this mode. In refine mode `write_task_plan` logs the plan lines and writes with no `$.ui.ask`, answering `Wrote the plan …`; in report mode `show_task_report` writes and links the report but does not `xdg-open` it. **Attended sessions are unchanged.**
- **The skill:** `refine-task` gets an `auto` argument: ask no questions, decide each open question with the recommended answer and record it as a `Decided:` note, offer no report, call `write_task_plan` once. `report-task` notes that an unattended run does not open the browser. `install.sh` is unchanged.
- **Processing:** a Taskwarrior tag, `+processing` (`task::PROCESSING_TAG`, `Task::is_processing`), set by `task refine --unattended` before Claude starts and cleared on every exit path it controls (success, a failed or non-zero run, a missing `claude` or mod). `Card.processing` and `Status::Processing` (Font Awesome's hourglass `\u{f252}`, dimmed like a blocked card); `TaskState.processing`; `Action::applies` offers nothing on a processing task: empty action row, hint `Planning in the background`, Ctrl+Enter and Ctrl+Delete do nothing. `task refine`, `task refine --unattended`, `task report` and `task start` refuse a processing task.
- **The notification:** the report is not opened. When both runs finish, `notify::tasks` says `Planned and reported: <description>`; when a run fails, the error says which step failed and the log path, the task stays added (planned if the refine succeeded), and the tag is cleared.
- **Docs:** README (task box section, Commands block line for `--unattended`), llms.txt (table row; rule 6's Runs column gains the background kind), CONTEXT.md (Task box, Task card, Task action, a Processing task term), ADR 0002 dated addendum on unattended mode.
- **Done when:** Add & all on a new task adds it at once and its card shows the hourglass, dimmed, with no buttons and Ctrl+Enter and Ctrl+Delete doing nothing; some minutes later, with nothing having opened on screen, the task is `+planned` with Goal/Decided/Steps/Done when notes and a `Report: <path>` note whose file exists under reviews, the processing tag is gone, and a notification says so. `niritasks task refine <uuid> --unattended` does the same from a shell. With `claude` or the mod missing, the task stays added, the tag is cleared and the notification names the log. Add, Add & refine, Refine, Grill me and Report behave as before. `cargo test`, `claude plugin test` and `tests/all.sh` pass.
- **Out of scope:** a keyboard shortcut or a `task add --all` default for the button; grilling unattended; cancelling or watching a run from the card; opening the report from the card; a Start working step after the report; recovering a `+processing` tag left by a killed runner (clear it with `task <uuid8> modify -processing`); a timeout on the `claude -p` runs.
- **Commits:** Conventional Commits, subject at most 72 characters, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- **Tests stay green at every commit.** `cargo test` checks that README's Commands block and llms.txt run every subcommand and flag, so `--unattended`'s doc lines land in the same commit as the flag (Task 3).

## File Structure

- `src/task.rs` (modify): `PROCESSING_TAG`, `PROCESSING_REFUSAL`, `Task::is_processing`, `set_processing` (with `set_up_next` over one private `set_tag`), `stamp` (the inverse of `stamp_secs`) and `civil_from_days`.
- `src/actions.rs` (modify): `TaskState.processing`, read by `TaskState::of`; `Action::applies` offers nothing on it.
- `src/panel/actions.rs` (modify): `Action::PROCESSING_HINT`; `advance` and `hint` on a processing task; tests.
- `src/panel/model.rs` (modify): `Status::Processing`, `Card.processing`, the icon, `cards()` and `cap()`.
- `src/panel/style.rs` (modify): the `.processing` card in the dimmed rule. `src/panel/surface.rs` (modify): the `processing` CSS class. `src/panel/state.rs` (modify): tests for the key no-ops.
- `src/session.rs` (modify): `pub use herdr::refiner_flags`. `src/session/herdr.rs` (modify): `refiner_flags`, used by `agent_start_claude_refiner`.
- `src/refine.rs` (modify): `session_settings` gets `unattended: bool`; `unattended_prompts`, `claude_unattended_argv`, `unattended_dir`, `log_path`, `unattended_command`, `spawn_unattended`, `run_unattended`.
- `src/main.rs` (modify): `TaskCommand::Refine { unattended }`, processing refusals on Refine, Report and Start; tests.
- `README.md`, `llms.txt` (modify, in Tasks 3 and 7).
- `claude/refine-mod/.claude-plugin/plugin.json`, `claude/refine-mod/hooks/refine.ts`, `claude/refine-mod/tests/refine.test.ts` (modify): the `unattended` option.
- `.claude/skills/refine-task/SKILL.md`, `.claude/skills/report-task/SKILL.md` (modify).
- `src/taskbox/form.rs` (modify): `Then`, `Submission.then`. `src/taskbox.rs` (modify): the Add & all button, `default_then`, `shortcut_then`. `src/daemon.rs` (modify): the spawn on `Then::All`.
- `CONTEXT.md`, `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` (modify).

---

### Task 1: The `+processing` tag, and a task state that offers nothing

**Files:**
- Modify: `src/task.rs:16-30` (constants), `:88-116` (`impl Task`), `:195-235` (`stamp_secs`, `days_from_civil`), `:783-798` (`set_up_next`), tests at `:995-1050`
- Modify: `src/actions.rs:57-104` (`TaskState`), `:197-216` (`applies`), tests
- Modify: `src/panel/actions.rs:13-85` (`impl Action`), tests at `:155-210`, `:430-445`

**Interfaces:**
- Consumes: `Task.tags`, `TaskState`, `Action::ALL`.
- Produces: `task::PROCESSING_TAG: &str = "processing"`, `task::PROCESSING_REFUSAL: &str`, `Task::is_processing(&self) -> bool`, `task::set_processing(uuid: &str, on: bool) -> Result<()>`, `task::stamp(secs: i64) -> String` (`20261006T010203Z`), `TaskState.processing: bool`, `Action::PROCESSING_HINT: &str = "Planning in the background"`. Task 2 reads `TaskState.processing` off a `Card`; Task 3 sets and clears the tag and names the log by `stamp`.

- [ ] **Step 1: Write the failing tests in `src/task.rs`**

In `mod tests`, after `planned_is_matched_exactly`:

```rust
    /// `task refine --unattended` tags the task while its two Claude runs
    /// last; this is how the card and the CLI know to leave it alone.
    #[test]
    fn the_processing_tag_marks_a_task_being_planned() {
        let t: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["proj","processing"]}"#).unwrap();
        assert!(t.is_processing());
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert!(!bare.is_processing());
        let other: Task =
            serde_json::from_str(r#"{"uuid":"u","description":"d","tags":["PROCESSING","processing_x"]}"#).unwrap();
        assert!(!other.is_processing(), "tags are case-sensitive and matched whole");
        assert_eq!(PROCESSING_TAG, "processing");
        assert!(PROCESSING_REFUSAL.contains("background"));
    }

    /// A stamp for now, in taskwarrior's own shape, so [`stamp_secs`] reads
    /// it back: what an unattended run's log is named by.
    #[test]
    fn stamp_writes_what_stamp_secs_reads() {
        assert_eq!(stamp(0), "19700101T000000Z");
        assert_eq!(stamp(1_791_248_523), "20261006T010203Z");
        for secs in [0, 951_782_400, 1_709_164_800, 1_791_248_523, 4_102_444_799] {
            assert_eq!(stamp_secs(&stamp(secs)), Some(secs), "{secs}");
        }
        // The last day of February in a leap year, and the first of March.
        assert_eq!(stamp(1_709_164_800), "20240229T000000Z");
        assert_eq!(stamp(1_709_251_200), "20240301T000000Z");
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib task::tests::the_processing_tag task::tests::stamp_writes 2>&1 | tail -5`
Expected: compile errors, `PROCESSING_TAG`, `is_processing` and `stamp` not found.

- [ ] **Step 3: Implement the tag, the refusal and the stamp in `src/task.rs`**

After `REPORT_NOTE` (line 30):

```rust
/// The tag `niritasks task refine --unattended` puts on a task while its
/// two Claude runs last, and takes off when they end, however they end. A
/// tag, not a file or a pid: the panel already redraws on a database change,
/// and `task <uuid8> modify -processing` clears one a killed run left.
pub const PROCESSING_TAG: &str = "processing";

/// What `task refine`, `task report` and `task start` say about a task
/// carrying [`PROCESSING_TAG`]: a Claude is writing it, and a second one, or
/// a worktree, would race that write.
pub const PROCESSING_REFUSAL: &str =
    "This task is being planned in the background; wait for its notification, or clear +processing if the run died.";
```

In `impl Task`, after `is_up_next`:

```rust
    /// Being planned in the background by `task refine --unattended`.
    pub fn is_processing(&self) -> bool {
        self.tags.iter().any(|t| t == PROCESSING_TAG)
    }
```

Replace `set_up_next` with `set_tag`, and the two callers of it:

```rust
/// Put `tag` on a task, or take it off. Only the tag changes: `+tag` or
/// `-tag` goes as one argument of its own, so taskwarrior reads it as a tag
/// and not as words for the description, and the task's status, start, wait,
/// other tags and notes stay as they were. A started task stays started.
fn set_tag(uuid: &str, tag: &str, on: bool) -> Result<()> {
    let sign = if on { '+' } else { '-' };
    let status = base()
        .arg(uuid)
        .arg("modify")
        .arg(format!("{sign}{tag}"))
        .status()
        .context("could not run `task modify`")?;
    anyhow::ensure!(status.success(), "`task modify {sign}{tag}` failed");
    Ok(())
}

/// Mark a task up next, or clear the mark. See [`set_tag`].
pub fn set_up_next(uuid: &str, on: bool) -> Result<()> {
    set_tag(uuid, UP_NEXT_TAG, on)
}

/// Mark a task as being planned in the background, or clear the mark. See
/// [`set_tag`].
pub fn set_processing(uuid: &str, on: bool) -> Result<()> {
    set_tag(uuid, PROCESSING_TAG, on)
}
```

After `days_from_civil`:

```rust
/// The date a day count from 1970-01-01 falls on, as (year, month, day):
/// [`days_from_civil`] run backwards, from the same page of Hinnant's.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `secs` since the epoch as a stamp of taskwarrior's shape,
/// `20261006T010203Z`, UTC: the one shape this module reads
/// ([`stamp_secs`]), so a file named by it sorts by time and reads back.
pub fn stamp(secs: i64) -> String {
    let (y, m, d) = civil_from_days(secs.div_euclid(DAY));
    let rest = secs.rem_euclid(DAY);
    format!("{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z", rest / HOUR, (rest % HOUR) / MINUTE, rest % MINUTE)
}
```

- [ ] **Step 4: Run the task tests**

Run: `cargo test --lib task:: 2>&1 | tail -3`
Expected: all pass, the two new ones included.

- [ ] **Step 5: Write the failing tests in `src/actions.rs`**

In `mod tests`, after `report_needs_a_plan_and_a_task_on_the_list`:

```rust
    /// A task being planned in the background gets nothing at all, whatever
    /// else is true of it: a Claude is writing it, and anything done to it
    /// meanwhile, a second Claude, an edit, a status change, would race that
    /// write. Off the list too: nothing brings it back until the run ends.
    #[test]
    fn a_processing_task_gets_nothing() {
        let processing = TaskState { processing: true, ..TaskState::default() };
        assert_eq!(offered(processing), Vec::<Action>::new());
        let every_way = TaskState {
            processing: true, active: true, planned: true, up_next: true, has_session: true, has_report: true,
            ..TaskState::default()
        };
        assert_eq!(offered(every_way), Vec::<Action>::new());
        assert_eq!(offered(TaskState { processing: true, waiting: true, ..TaskState::default() }), Vec::<Action>::new());
        assert_eq!(offered(TaskState { processing: true, finished: true, ..TaskState::default() }), Vec::<Action>::new());
    }

    /// Read off the task's tags, as planned and up next are.
    #[test]
    fn of_reads_processing_from_the_tasks_tags() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","status":"pending","tags":["processing"]}"#,
        )
        .unwrap();
        assert_eq!(TaskState::of(&t, false, false), TaskState { processing: true, ..TaskState::default() });
    }
```

In `a_tasks_state_is_read_from_the_task`, the full literal becomes:

```rust
            TaskState { active: true, waiting: false, finished: false, planned: true, up_next: true, has_session: true, has_report: false, processing: false }
```

- [ ] **Step 6: Implement `TaskState.processing` and the `applies` override**

In `TaskState`, after `has_report`:

```rust
    /// Being planned in the background by `niritasks task refine
    /// --unattended`, which tags it `+processing` while it runs: nothing may
    /// be done to it until the tag comes off.
    pub processing: bool,
```

In `TaskState::of`, after `has_report: task.report_path().is_some(),`:

```rust
            processing: task.is_processing(),
```

Two other literals build a `TaskState` field by field and must name the new one to compile; Task 2 wires them to the card. In `src/panel/model.rs`, `Card::state` (line 111) gets `processing: false,` after `has_report: self.has_report,`, and the full `TaskState` literal in its tests (line 876, the one listing every field) gets `, processing: false` at its end.

In `Action::applies`, add to the doc comment's end: `A processing task gets nothing at all: a Claude is writing it, and anything done to it meanwhile would race that write.` Then make the body:

```rust
    pub fn applies(self, state: TaskState) -> bool {
        if state.processing {
            return false;
        }
        match self {
            Session => state.has_session && !state.off_list(),
            Back => state.off_list(),
            Stop => state.active,
            // A plan to report on, and a task still on the list.
            Report => state.planned && !state.off_list(),
            Start | Refine | Grill | UpNext | Wait => !state.off_list(),
            // A finished task is done already, and stays where it was done.
            Complete | Move => !state.finished,
            Edit | Speak | Remove => true,
        }
    }
```

- [ ] **Step 7: Run the actions tests**

Run: `cargo test --lib actions:: 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 8: Write the failing tests in `src/panel/actions.rs`**

In `mod tests`, after `with_report`:

```rust
    /// A task being planned in the background, from any other state.
    fn processing(state: TaskState) -> TaskState {
        TaskState { processing: true, ..state }
    }
```

After `ctrl_enter_only_presses_a_button_the_card_has`:

```rust
    /// A processing card gets no buttons, whatever else is true of its task,
    /// so there is no button for Ctrl+Enter, a letter or Delete to press;
    /// its hint says what is happening instead, on the body and whether or
    /// not its notes show, and fits a card with no buttons.
    #[test]
    fn a_processing_card_gets_no_buttons_and_says_why() {
        for state in every_state().into_iter().map(processing) {
            assert_eq!(Action::row(state), Vec::<Action>::new(), "{state:?}");
            assert_eq!(Action::advance(state), None, "{state:?}");
            for notes in [false, true] {
                assert_eq!(Action::hint(state, &[], None, notes), "Planning in the background", "{state:?}");
            }
        }
        assert_eq!(Action::PROCESSING_HINT, "Planning in the background");
        assert!(Action::PROCESSING_HINT.chars().count() <= hint_room(0));
    }
```

Change `hint_room` so a row of no buttons does not underflow:

```rust
    fn hint_room(buttons: usize) -> usize {
        (760 - buttons * 32 - buttons.saturating_sub(1) - 24) / 7
    }
```

- [ ] **Step 9: Run them to verify they fail**

Run: `cargo test --lib panel::actions::tests::a_processing_card 2>&1 | tail -5`
Expected: compile error, `PROCESSING_HINT` not found.

- [ ] **Step 10: Implement the hint and the advance arm**

In `impl Action` in `src/panel/actions.rs`, after `SPINNER`:

```rust
    /// What the hint reads on a card being planned in the background, where
    /// there are no buttons to name and nothing for Ctrl+Enter to do.
    pub const PROCESSING_HINT: &'static str = "Planning in the background";
```

In `advance`, first arm:

```rust
        match state {
            TaskState { processing: true, .. } => None,
            TaskState { finished: true, .. } => None,
```

and add to its doc: `Nothing on a processing task either, which has no button to press.`

In `hint`, first lines of the body:

```rust
    pub fn hint(state: TaskState, row: &[Action], focused: Option<Action>, notes: bool) -> String {
        if state.processing {
            return Self::PROCESSING_HINT.to_string();
        }
        let mut parts = Vec::new();
```

and add to its doc: `On a processing card, which has no buttons, it reads [`Action::PROCESSING_HINT`] alone.`

- [ ] **Step 11: Run every unit test**

Run: `cargo test 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 12: Commit**

```bash
git add src/task.rs src/actions.rs src/panel/actions.rs
git commit -F- <<'EOF'
feat(actions): offer nothing on a task tagged processing

An unattended refine tags its task +processing while its two Claude runs
last. A task carrying it gets no action at all, whatever else is true of
it: a Claude is writing it, and a second one, an edit or a status change
would race that write. The hint says Planning in the background in place
of the buttons. task::stamp names the run's log in taskwarrior's own
stamp shape, the one the module already reads.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 2: The processing card: hourglass, dimmed, no row, keys do nothing

**Files:**
- Modify: `src/panel/model.rs:13-28` (`Status`), `:30-62` (`Card`), `:65-82` (`icon`), `:108-120` (`state`), `:290-320` (`cards`), `:335-348` (`cap`), tests
- Modify: `src/panel/style.rs:238-241` (the dimmed rule), tests at `:476-500`
- Modify: `src/panel/surface.rs:992-1000` (the status classes)
- Modify: `src/panel/state.rs:938-941` (the test `card` helper), tests after `:1690`

**Interfaces:**
- Consumes: `Task::is_processing`, `TaskState.processing`, `Action::PROCESSING_HINT` (Task 1).
- Produces: `Status::Processing`, `Card.processing: bool`, `Card::icon() == "\u{f252}"` on it, the `processing` CSS class dimmed like `blocked`.

- [ ] **Step 1: Write the failing tests in `src/panel/model.rs`**

In `mod tests`, after `up_next` (the helper):

```rust
    fn processing(uuid: &str) -> Task {
        let mut t = task(uuid, 1, false);
        t.tags = vec![crate::task::PROCESSING_TAG.into()];
        t
    }
```

After `blocked_outranks_planned`:

```rust
    /// A task an unattended refine is planning shows the hourglass, and its
    /// card knows it, so its state offers nothing. Above blocked and planned:
    /// you cannot act on it either way, and that is the fact to show; the
    /// Planned tab still goes by the tag.
    #[test]
    fn a_processing_task_gets_the_hourglass_and_offers_nothing() {
        let got = cards(&todo(vec![processing("p")]), &["p".into()]);
        assert_eq!(got[0].status, Status::Processing);
        assert_eq!(got[0].icon(), "\u{f252}");
        assert!(got[0].processing);
        assert_eq!(got[0].state(false), Some(TaskState { processing: true, ..TaskState::default() }));

        let mut planned_too = processing("q");
        planned_too.tags.push(crate::task::PLANNED_TAG.into());
        let got = cards(&todo(vec![planned_too]), &[]);
        assert_eq!(got[0].status, Status::Processing);
        assert!(got[0].planned, "still planned, for the Planned tab");
    }

    /// Started outranks processing, as it outranks everything on the list;
    /// the card still knows, so an active card being planned offers nothing.
    #[test]
    fn started_outranks_processing() {
        let got = cards(&todo(vec![processing("s")]), &[]);
        assert_eq!(got[0].status, Status::Processing);
        let mut started = processing("s");
        started.start = Some("20260927T080000Z".into());
        let got = cards(&todo(vec![started]), &[]);
        assert_eq!(got[0].status, Status::Active);
        assert!(got[0].processing);
        assert_eq!(got[0].state(false).map(crate::actions::Action::row), Some(Vec::new()));
    }

    /// "+N more" is being planned by no one.
    #[test]
    fn the_more_card_is_not_processing() {
        let many: Vec<Task> = (0..CAP + 1).map(|i| processing(&format!("t{i}"))).collect();
        let last = cap(&cards(&todo(many), &[]), CAP).last().unwrap().clone();
        assert_eq!(last.status, Status::More);
        assert!(!last.processing);
    }
```

In the test near line 863 whose closure builds a `Card` by `|status, planned, up_next, has_report| Card { … has_report, }`, add `processing: false,` after `has_report,`. (Its full `TaskState` literal already names `processing: false`, from Task 1.)

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib panel::model 2>&1 | tail -5`
Expected: compile errors, `Status::Processing` and `processing` not found.

- [ ] **Step 3: Implement the status, the field and the icon**

In `Status`, after `Blocked`:

```rust
    /// Being planned in the background by `task refine --unattended`:
    /// tagged `+processing` until its two Claude runs end.
    Processing,
```

In `Card`, after `has_report`:

```rust
    /// The task carries `+processing`, whatever its status: an active task
    /// being planned shows Active, and still offers nothing. False on
    /// "+N more".
    pub processing: bool,
```

In `Card::icon`, after the `Blocked` arm:

```rust
            // Font Awesome's hourglass, half run: being planned, wait.
            Status::Processing => "\u{f252}",
```

In `Card::state`, Task 1's `processing: false,` becomes:

```rust
            processing: self.processing,
```

In `cards()`, the status chain becomes:

```rust
            } else if t.is_active() {
                Status::Active
            } else if t.is_processing() {
                Status::Processing
            } else if blocked.contains(&t.uuid) {
                Status::Blocked
            } else if t.is_planned() {
                Status::Planned
            } else {
                Status::Pending
            },
```

and after `has_report: t.report_path().is_some(),` add `processing: t.is_processing(),`. Add to the doc comment of `cards` (after the paragraph on waiting and finished): `Processing comes above blocked and planned: a task being planned in the background cannot be acted on, which matters more than why it waits or whether it has a plan already; a started one stays Active, its card knowing all the same.`

In `cap()`'s "+N more" literal, after `has_report: false,` add `processing: false,`.

- [ ] **Step 4: Run the model tests**

Run: `cargo test --lib panel::model 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 5: Write the failing style test**

In `src/panel/style.rs`'s `mod tests`, after `a_finished_card_is_dimmed`:

```rust
    /// A processing card is dimmed like a blocked one: nothing can be done
    /// to it yet.
    #[test]
    fn a_processing_card_is_dimmed() {
        assert!(css().contains(
            ".task-panel .task-card.blocked,\n.task-panel .task-card.processing,\n.task-panel .task-card.waiting,"
        ));
    }
```

Run: `cargo test --lib panel::style::tests::a_processing_card_is_dimmed 2>&1 | tail -3`
Expected: FAIL, the rule is not there.

- [ ] **Step 6: Implement the rule and the class**

In `src/panel/style.rs`, the dimmed rule becomes:

```
.task-panel .task-card.blocked,
.task-panel .task-card.processing,
.task-panel .task-card.waiting,
.task-panel .task-card.finished,
.task-panel .task-card.more {{ color: alpha({TEXT}, 0.55); }}
```

and the module doc's line 12 becomes `//! active or waiting. Blocked, processing, waiting and finished cards are dimmed.`

In `src/panel/surface.rs`'s `card_widget`, after the `Blocked` arm:

```rust
            Status::Processing => root.add_css_class("processing"),
```

- [ ] **Step 7: Write the failing key tests in `src/panel/state.rs`**

Add `processing: false` to the `card` helper's literal (line 940), then a helper after `pending`:

```rust
    /// A card being planned in the background.
    fn processing(uuid: &str) -> Card {
        Card { processing: true, ..card(uuid, Status::Processing) }
    }
```

After `ctrl_delete_with_no_card_does_nothing`:

```rust
    // ─── processing ──────────────────────────────────────────────────────

    /// A card being planned in the background has no buttons, its hint says
    /// so, and the keys that act on a task do nothing to it: not Ctrl+Enter,
    /// not Ctrl+Delete, not a letter or Delete. The list stays up and the
    /// focus stays on its body; the next card is untouched.
    #[test]
    fn a_processing_card_has_no_buttons_and_the_keys_leave_it_alone() {
        let mut state = keyboard(vec![processing("a"), card("b", Status::Pending)]);
        assert_eq!(state.visible()[0].actions, Vec::<Action>::new());
        assert_eq!(state.card_hint("a"), Action::PROCESSING_HINT);
        assert_eq!(key(&mut state, KeyAction::Advance), Vec::new(), "Ctrl+Enter");
        assert_eq!(key(&mut state, KeyAction::Delete), Vec::new(), "Ctrl+Delete");
        assert_eq!(key(&mut state, KeyAction::Run(Action::Remove)), Vec::new(), "Delete");
        assert_eq!(key(&mut state, KeyAction::Run(Action::Refine)), Vec::new(), "r");
        assert_eq!(key(&mut state, KeyAction::NextSlot), Vec::new(), "no button to move to");
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        key(&mut state, KeyAction::NextCard);
        assert_eq!(
            key(&mut state, KeyAction::Advance),
            vec![Effect::Notify("Refine: b".into()), Effect::Spawn(Action::Refine.args("b"))],
        );
    }
```

Run: `cargo test --lib panel::state::tests::a_processing_card 2>&1 | tail -8`
Expected: PASS already, from Task 1's `applies` and this task's `Card.processing`, apart from a possible `NextSlot` difference: if `move_slot` on an empty row answers `vec![Effect::Focus(..)]` rather than nothing, read `move_slot` and assert what it does on a row with no buttons, keeping the focus on the body. Fix the assertion, not the code.

- [ ] **Step 8: Run every unit test**

Run: `cargo test 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git add src/panel/model.rs src/panel/style.rs src/panel/surface.rs src/panel/state.rs
git commit -F- <<'EOF'
feat(panel): dim a processing card and show it is being planned

A task tagged +processing draws with Font Awesome's hourglass, dimmed
like a blocked card, above blocked and planned: it cannot be acted on,
and that is the fact to show. Its card carries the tag whatever its
status, so an active task being planned offers nothing either, and the
panel's keys leave it alone.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 3: `niritasks task refine --unattended`: two `claude -p` runs behind the tab's fence

**Files:**
- Modify: `src/session.rs:29` (`mod herdr;`)
- Modify: `src/session/herdr.rs:86-120` (`agent_start_claude_refiner`), tests at `:639-656`
- Modify: `src/refine.rs:9-14` (imports), `:95-152` (`session_settings`), `:264-309` (`launch`), `:352-374` (`quick_command`, `spawn_quick`), tests
- Modify: `src/main.rs:162-172` (`Refine`), `:184-190` (`Report`), `:197-208` (`Start`), `:363-414` (handlers), tests
- Modify: `README.md:271-272` (Commands block), `llms.txt:43` (the table)

**Interfaces:**
- Consumes: `task::set_processing`, `task::stamp`, `Task::is_processing`, `task::PROCESSING_REFUSAL` (Task 1); `Workspace::session`, `Session::dir`, `Dirs`, `programs::on_path`, `names::uuid8`, `niri::spawn`, `notify::tasks`.
- Produces: `session::refiner_flags(settings: &str, mod_dir: &Path) -> Vec<String>`; `refine::session_settings(project, task_data, hidden, uuid, reports, skill_dirs, mode, unattended: bool) -> String`; `refine::unattended_prompts(uuid) -> [String; 2]`; `refine::claude_unattended_argv(prompt, settings, mod_dir) -> Vec<String>`; `refine::unattended_dir(&Dirs) -> PathBuf`; `refine::log_path(&Dirs, uuid, now_secs) -> PathBuf`; `refine::unattended_command(exe, uuid) -> Vec<String>`; `refine::spawn_unattended(uuid)`; `refine::run_unattended(&Workspace, &Task) -> Result<()>`; `refine::PLANNED_AND_REPORTED: &str`; `refine::CLAUDE: &str = "claude"`. Task 4's mod reads `options.unattended`; Task 6's daemon calls `spawn_unattended`.

- [ ] **Step 1: Write the failing flag test in `src/session/herdr.rs`**

In `mod tests`, after `claude_starts_unable_to_edit_files_or_plan`:

```rust
    /// The tab's Claude arguments are one list, the ones after `--`, which
    /// an unattended run passes to `claude -p` itself, so the two fences
    /// cannot drift apart.
    #[test]
    fn the_refiner_flags_are_the_tabs_arguments_after_the_dashes() {
        let flags = refiner_flags("{\"sandbox\":{}}", Path::new("/m/refine-mod"));
        let argv = agent_start_claude_refiner("alpha", "task-0123abcd", "w1:p3", "{\"sandbox\":{}}", Path::new("/m/refine-mod"));
        let at = argv.iter().position(|a| a == "--").expect("herdr's --");
        assert_eq!(argv[at + 1..], flags[..]);
        assert_eq!(flags[..2], ["--permission-mode", "default"]);
        assert!(!flags.contains(&"plan".to_string()) && !flags.contains(&"auto".to_string()));
    }
```

Run: `cargo test --lib session::herdr::tests::the_refiner_flags 2>&1 | tail -3`
Expected: compile error, `refiner_flags` not found.

- [ ] **Step 2: Implement `refiner_flags`**

In `src/session/herdr.rs`, before `agent_start_claude_refiner`:

```rust
/// Claude's own arguments for a refine: the ones after `--` in
/// [`agent_start_claude_refiner`], and the ones an unattended run passes to
/// `claude -p` itself (`refine::claude_unattended_argv`). Spelled once so the
/// tab and the headless run cannot drift apart; why each is here is on
/// [`agent_start_claude_refiner`].
pub fn refiner_flags(settings: &str, mod_dir: &Path) -> Vec<String> {
    let mod_dir = mod_dir.to_string_lossy();
    [
        "--permission-mode", "default",
        "--disallowedTools", "Edit", "Write", "NotebookEdit", "EnterPlanMode", "ExitPlanMode",
        "--append-system-prompt", REFINER_SYSTEM_PROMPT,
        "--settings", settings,
        "--plugin-dir", &mod_dir,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
```

and make `agent_start_claude_refiner`'s body:

```rust
pub fn agent_start_claude_refiner(session: &str, name: &str, pane: &str, settings: &str, mod_dir: &Path) -> Vec<String> {
    let mut argv = cmd(
        session,
        &["agent", "start", name, "--kind", "claude", "--pane", pane, "--timeout", "60000", "--"],
    );
    argv.extend(refiner_flags(settings, mod_dir));
    argv
}
```

In `src/session.rs`, after `mod herdr;` (line 29):

```rust
pub use herdr::refiner_flags;
```

Run: `cargo test --lib session:: 2>&1 | tail -3`
Expected: all pass, `claude_starts_unable_to_edit_files_or_plan` unchanged.

- [ ] **Step 3: Write the failing tests in `src/refine.rs`**

Every existing call to `session_settings` in `mod tests` gains `, false` as its last argument (six calls: one each in `the_sandbox_fences_…`, `the_session_may_search_…`, `each_resolved_skill_folder_…` and `the_mod_is_told_…`, and two in `a_report_session_tells_the_mod_so`, counting the one in its loop). Then add, after `a_report_session_tells_the_mod_so`:

```rust
    /// An unattended run tells the mod so, in both of its modes: the write
    /// tool writes with no question and the report tool opens nothing. A tab
    /// never does.
    #[test]
    fn an_unattended_run_tells_the_mod_so() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        for mode in [Mode::Quick, Mode::Report] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), &[], mode, true);
            let v: Value = serde_json::from_str(&json).unwrap();
            let options = &v["pluginConfigs"][REFINE_MOD]["options"];
            assert_eq!(options["unattended"], true, "{mode:?}");
            assert_eq!(options["report"], mode == Mode::Report, "{mode:?}");
            assert_eq!(options["uuid"], u, "the same fence and task");
        }
        for mode in [Mode::Quick, Mode::Grill, Mode::Report] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), &[], mode, false);
            let v: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["unattended"], false, "{mode:?}");
        }
    }

    /// Add & all runs the Refine button's command with --unattended after
    /// it, so it cannot drift from Refine either.
    #[test]
    fn unattended_command_is_the_refine_command_plus_the_flag() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        assert_eq!(
            unattended_command("/usr/bin/niritasks", u),
            ["/usr/bin/niritasks", "task", "refine", u, "--unattended"]
        );
    }

    /// The two prompts, in the order the runs go: refine-task's auto mode,
    /// which asks nothing, then report-task.
    #[test]
    fn unattended_prompts_refine_automatically_then_report() {
        assert_eq!(unattended_prompts("u-1"), ["/refine-task u-1 auto", "/report-task u-1"]);
        assert_eq!(unattended_prompts("u-1")[1], prompt("u-1", Mode::Report), "the tab's own report prompt");
    }

    /// `claude -p` with the tab's own flags between the prompt and
    /// `--permission-prompts none`, which makes anything that would ask
    /// refuse instead. Default permission mode, as the tab: not plan mode,
    /// whose approval means implement, and not auto mode.
    #[test]
    fn an_unattended_run_is_the_tabs_claude_with_no_one_to_ask() {
        let argv = claude_unattended_argv("/refine-task u-1 auto", "{\"sandbox\":{}}", Path::new("/m/refine-mod"));
        assert_eq!(argv[..3], ["claude", "-p", "/refine-task u-1 auto"]);
        let flags = crate::session::refiner_flags("{\"sandbox\":{}}", Path::new("/m/refine-mod"));
        assert_eq!(argv[3..3 + flags.len()], flags[..]);
        assert_eq!(argv[3 + flags.len()..], ["--permission-prompts", "none"]);
        assert_eq!(argv.iter().filter(|a| *a == "--permission-mode").count(), 1);
        assert_eq!(CLAUDE, "claude");
    }

    /// The log goes beside the reviews, under the XDG data folder, named by
    /// the task and when the run started, so two runs on one task keep both.
    #[test]
    fn the_log_goes_beside_the_reviews_named_by_task_and_time() {
        let dirs = Dirs::at(Path::new("/home/x"));
        assert_eq!(unattended_dir(&dirs), PathBuf::from("/home/x/.local/share/niri-tasks/unattended"));
        assert_eq!(
            log_path(&dirs, "d9f76b94-e0ff-44df-85b4-060be4219169", 1_791_248_523),
            PathBuf::from("/home/x/.local/share/niri-tasks/unattended/d9f76b94-20261006T010203Z.log")
        );
        assert_eq!(
            unattended_dir(&dirs.with_data(Path::new("/data"))),
            PathBuf::from("/data/niri-tasks/unattended")
        );
    }
```

Run: `cargo test --lib refine:: 2>&1 | tail -5`
Expected: compile errors, the new names not found and `session_settings` taking seven arguments.

- [ ] **Step 4: Implement the settings flag and the pure builders in `src/refine.rs`**

Imports at the top become:

```rust
use crate::dirs::Dirs;
use crate::session::{Claude, Session};
use crate::workspace::Workspace;
use crate::{names, niri, notify, programs, task};
use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
```

`session_settings`: add to its doc, after the sentence on `report`: `and `unattended`: true in an unattended run, where the write tool writes with no question and the report tool opens nothing (see [`run_unattended`]).` Its signature and `pluginConfigs` line become:

```rust
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path, skill_dirs: &[PathBuf], mode: Mode, unattended: bool) -> String {
```

```rust
        "pluginConfigs": {
            (REFINE_MOD): { "options": { "uuid": uuid, "reports": reports, "report": (mode == Mode::Report), "unattended": unattended } },
        },
```

In `launch`, the call gains `, false` after `mode`.

After `reports_dir`:

```rust
/// Where an unattended run's log goes: `$XDG_DATA_HOME/niri-tasks/unattended`,
/// beside the reviews, so a failed run can be read later.
pub fn unattended_dir(dirs: &Dirs) -> PathBuf {
    dirs.data().join("niri-tasks/unattended")
}

/// The log of an unattended run on `uuid` started at `now` (Unix seconds):
/// `<uuid8>-<stamp>.log` under [`unattended_dir`], one file for both of its
/// Claude runs, named by time so a second run on the task keeps the first's.
pub fn log_path(dirs: &Dirs, uuid: &str, now: i64) -> PathBuf {
    unattended_dir(dirs).join(format!("{}-{}.log", names::uuid8(uuid), task::stamp(now)))
}
```

After `spawn_quick`:

```rust
/// The flag that makes `task refine` run with no one watching.
pub const UNATTENDED_FLAG: &str = "--unattended";

/// Add & all's command on `uuid`: the Refine button's ([`quick_command`])
/// with [`UNATTENDED_FLAG`] after it, so it cannot drift from Refine either.
pub fn unattended_command(exe: &str, uuid: &str) -> Vec<String> {
    let mut command = quick_command(exe, uuid);
    command.push(UNATTENDED_FLAG.to_string());
    command
}

/// Hand a task just added to an unattended refine and report — Add & all.
///
/// In a `niritasks task refine --unattended` process of its own, spawned by
/// niri as [`spawn_quick`]'s is: the daemon's GTK loop must never wait on a
/// Claude run that takes minutes. That process reports its own failures;
/// this only reports failing to start it. Either way the task is added.
pub fn spawn_unattended(uuid: &str) {
    let result = std::env::current_exe()
        .map_err(anyhow::Error::from)
        .and_then(|exe| niri::spawn(unattended_command(&exe.to_string_lossy(), uuid)));
    if let Err(e) = result {
        notify::tasks(&format!("Added, but could not start planning it: {e}"));
    }
}

/// Claude Code's binary, which an unattended run executes itself.
pub const CLAUDE: &str = "claude";

/// The two prompts an unattended run sends, in order: `refine-task` in its
/// `auto` mode, which decides every open question itself and writes the
/// plan with no question asked, then `report-task` on the plan it wrote.
pub fn unattended_prompts(uuid: &str) -> [String; 2] {
    [format!("/refine-task {uuid} auto"), prompt(uuid, Mode::Report)]
}

/// What an unattended run executes: `claude -p <prompt>` with the tab's own
/// flags ([`crate::session::refiner_flags`]: default permission mode, the
/// editing tools removed, the standing instruction, the settings fence and
/// the mod) and `--permission-prompts none`, so anything that would ask — a
/// permission, AskUserQuestion — is refused rather than waited on. Not plan
/// mode and not auto mode, for the reasons `agent_start_claude_refiner`
/// gives; the sandbox already runs every command unasked.
pub fn claude_unattended_argv(prompt: &str, settings: &str, mod_dir: &Path) -> Vec<String> {
    let mut argv = vec![CLAUDE.to_string(), "-p".to_string(), prompt.to_string()];
    argv.extend(crate::session::refiner_flags(settings, mod_dir));
    argv.extend(["--permission-prompts", "none"].map(String::from));
    argv
}
```

Run: `cargo test --lib refine:: 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 5: Implement `run_unattended`**

After `claude_unattended_argv`:

```rust
/// What the notification says when both runs are done, before the task's
/// description.
pub const PLANNED_AND_REPORTED: &str = "Planned and reported";

/// Refine a task and build its report with no one watching: Add & all's
/// second half, and `niritasks task refine <uuid> --unattended`.
///
/// Two `claude -p` runs in this process, in the project folder, behind the
/// fence a refine tab gets ([`session_settings`] with `unattended` set,
/// [`claude_unattended_argv`]): `/refine-task <uuid> auto`, which writes
/// the plan with no question, then, once the task is planned,
/// `/report-task <uuid>`, which writes and links the report without opening
/// it. No herdr and no window.
///
/// `+processing` goes on the task before anything runs and comes off on
/// every way out of here — done, a run that failed, a run that wrote
/// nothing, `claude` or the mod missing — so its card shows it is being
/// planned and offers nothing meanwhile, and nothing else writes it. A run
/// this process does not control, killed with it, leaves the tag; that is
/// `task <uuid8> modify -processing`. Both runs' output goes to one log
/// ([`log_path`]), which a failure's error names; the error is `main`'s to
/// notify, so a failure is said once.
pub fn run_unattended(ws: &Workspace, t: &task::Task) -> Result<()> {
    let dirs = Dirs::from_env()?;
    let log = log_path(&dirs, &t.uuid, task::now_secs());
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("could not make {}", dir.display()))?;
    }
    task::set_processing(&t.uuid, true)?;
    let outcome = unattended_steps(ws, t, &dirs, &log);
    // Cleared whatever happened above; a tag left behind would lock the
    // card for good.
    if let Err(e) = task::set_processing(&t.uuid, false) {
        eprintln!("could not clear +{}: {e:#}", task::PROCESSING_TAG);
    }
    match outcome {
        Ok(()) => {
            notify::tasks(&format!("{PLANNED_AND_REPORTED}: {}", t.description));
            Ok(())
        }
        Err(e) => {
            append_line(&log, &format!("== failed: {e:#}"));
            bail!("{e:#} Log: {}", log.display())
        }
    }
}

/// Which of the two runs a failure is reported against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Refine,
    Report,
}

impl Step {
    fn name(self) -> &'static str {
        match self {
            Step::Refine => "refine",
            Step::Report => "report",
        }
    }
}

/// Everything [`run_unattended`] does between tagging the task and
/// untagging it: the fence checks a tab makes, then the two runs, each
/// followed by reading the task back to see it did its job.
fn unattended_steps(ws: &Workspace, t: &task::Task, dirs: &Dirs, log: &Path) -> Result<()> {
    let home = dirs.home();
    let hidden = hidden_paths(home);
    ensure_no_exposed_sockets(&hidden)?;
    let mod_dir = refine_mod_dir(dirs);
    anyhow::ensure!(
        mod_dir.join(".claude-plugin/plugin.json").is_file(),
        "The refine mod is missing, or its link is broken, at {}. Run install.sh from the niri-tasks repo.",
        mod_dir.display()
    );
    anyhow::ensure!(programs::on_path(CLAUDE), "Claude Code (`{CLAUDE}`) is not on PATH, so nothing can plan the task.");
    let session = ws.session()?;
    let project = session.dir();
    let task_data = task::data_location()?;
    let reports = reports_dir(dirs);
    let skills = skill_dirs(home);
    let [refine_prompt, report_prompt] = unattended_prompts(&t.uuid);

    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, &skills, Mode::Quick, true);
    run_claude(project, log, Step::Refine, &claude_unattended_argv(&refine_prompt, &settings, &mod_dir))?;
    let planned = task::get(&t.uuid)?.context("The task is gone after the refine step.")?;
    anyhow::ensure!(planned.is_planned(), "The refine step wrote no plan: the task is not tagged planned.");

    let settings = session_settings(project, &task_data, &hidden, &t.uuid, &reports, &skills, Mode::Report, true);
    run_claude(project, log, Step::Report, &claude_unattended_argv(&report_prompt, &settings, &mod_dir))?;
    let reported = task::get(&t.uuid)?.context("The task is gone after the report step.")?;
    let path = reported.report_path().context("The report step linked no report: the task has no Report: note.")?;
    anyhow::ensure!(Path::new(path).is_file(), "The report step linked {path}, which is not there.");
    Ok(())
}

/// One `claude -p` run in `project`, both its streams appended to `log`
/// under a line naming the step and the prompt (not the settings, which are
/// long and the same for both). Stdin is closed: there is no one typing.
fn run_claude(project: &Path, log: &Path, step: Step, argv: &[String]) -> Result<()> {
    append_line(log, &format!("== {}: {}", step.name(), argv[..3].join(" ")));
    let out = File::options().append(true).create(true).open(log).with_context(|| format!("could not open {}", log.display()))?;
    let err = out.try_clone().context("could not share the log between stdout and stderr")?;
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .with_context(|| format!("could not run {}", argv[0]))?;
    anyhow::ensure!(status.success(), "The {} step failed: claude exited with {status}.", step.name());
    Ok(())
}

/// Add `line` to the log. Best effort: the log is for reading a failure
/// later, and a log that cannot be written must not hide the failure itself.
fn append_line(log: &Path, line: &str) {
    if let Ok(mut file) = File::options().append(true).create(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
}
```

Add a test for `run_claude` and `append_line` with a stand-in for `claude`, after `the_log_goes_beside_the_reviews_named_by_task_and_time`:

```rust
    /// A run's output lands in the log under its step's heading, and a
    /// non-zero exit is the step's failure, named. `sh` stands in for
    /// claude: `argv[..3]` is what the heading shows.
    #[test]
    fn a_run_logs_its_output_under_its_step_and_fails_on_a_bad_exit() {
        let dir = std::env::temp_dir().join(format!("niritasks-unattended-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("x.log");
        let ok = ["sh", "-c", "echo out; echo err >&2"].map(String::from);
        run_claude(&dir, &log, Step::Refine, &ok).unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        assert_eq!(text, "== refine: sh -c echo out; echo err >&2\nout\nerr\n");

        let bad = ["sh", "-c", "exit 3"].map(String::from);
        let err = run_claude(&dir, &log, Step::Report, &bad).unwrap_err().to_string();
        assert!(err.starts_with("The report step failed: claude exited with exit status: 3"), "{err}");
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("== report: sh -c exit 3\n"), "{text}");

        append_line(&log, "== failed: because");
        assert!(std::fs::read_to_string(&log).unwrap().ends_with("== failed: because\n"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
```

Run: `cargo test --lib refine:: 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 6: Write the failing CLI tests in `src/main.rs`**

In `mod tests`, after `a_fresh_report_is_a_flag_on_task_report`:

```rust
    /// Add & all starts this in the background, so it has to be a command
    /// the CLI accepts; and a run with no one to answer cannot grill.
    #[test]
    fn the_unattended_refine_is_a_command_the_cli_accepts_and_never_grills() {
        let argv = refine::unattended_command("niritasks", "c53b6e3d");
        assert!(Cli::try_parse_from(&argv).is_ok(), "{argv:?}");
        assert!(Cli::try_parse_from(["niritasks", "task", "refine", "c53b6e3d", "--grill", "--unattended"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "refine", "c53b6e3d", "--unattended", "--grill"]).is_err());
    }
```

Run: `cargo test --bin niritasks the_unattended_refine 2>&1 | tail -5`
Expected: FAIL, `--unattended` is not a flag yet. (`readme_commands_block_…` and `llms_txt_…` will fail too until Step 8.)

- [ ] **Step 7: Implement the flag and the refusals**

`TaskCommand::Refine` becomes:

```rust
    /// Work a task up into a plan with Claude, in a new tab of the workspace's herdr session
    ///
    /// The workspace is this herdr session's in a herdr pane, else the
    /// focused one, as for task add, task start and task session. With
    /// --unattended there is no tab: Claude runs headless in the project
    /// folder, plans the task asking nothing, then builds and links its
    /// report, as the task box's Add & all does; the task carries
    /// +processing meanwhile and a notification says when it is done, or
    /// which step failed and where its log is.
    Refine {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// Interview first, via the `grilling` skill, rather than drafting straight away
        #[arg(long, conflicts_with = "unattended")]
        grill: bool,
        /// Plan and report in the background with no questions, no tab and no browser:
        /// what the task box's Add & all runs
        #[arg(long)]
        unattended: bool,
    },
```

The handler:

```rust
        TaskCommand::Refine { uuid, grill, unattended } => {
            // The same refusal every entry point makes: a task refined on an
            // unnamed workspace would have no session, or project folder, to
            // run in.
            let workspace = Workspace::of_caller()?;
            let t = task::get(&uuid)?.context("task not found")?;
            // task::get is unfiltered by status; a completed or deleted task
            // has nothing left to work up into a plan.
            anyhow::ensure!(t.status == "pending", "Only a pending task can be refined.");
            // A second run on the task, or a tab opened over one, would race
            // the first for the write.
            anyhow::ensure!(!t.is_processing(), "{}", task::PROCESSING_REFUSAL);
            if unattended {
                return refine::run_unattended(&workspace, &t);
            }
            let mode = if grill { refine::Mode::Grill } else { refine::Mode::Quick };
            refine::launch(&workspace, &t, mode)?;
        }
```

In `TaskCommand::Report`'s handler, right after `let t = task::get(&uuid)?.context("task not found")?;`:

```rust
            // Its plan is still being written, and the second run will link
            // a report itself.
            anyhow::ensure!(!t.is_processing(), "{}", task::PROCESSING_REFUSAL);
```

In `TaskCommand::Start`'s handler, right after its `let t = task::get(&uuid)?.context("task not found")?;`:

```rust
            // Its plan is still being written: the worktree's Claude would
            // read half a plan.
            anyhow::ensure!(!t.is_processing(), "{}", task::PROCESSING_REFUSAL);
```

Add to `Report`'s and `Start`'s doc comments one sentence: `Refused while the task is being planned in the background (+processing).`

- [ ] **Step 8: Document the flag where the tests look**

In `README.md`'s Commands block, after the `--grill` line:

```
niritasks task refine <uuid> --unattended  # headless: Claude plans it asking nothing, then builds and links its
                                   #   report, in the background; the card shows an hourglass meanwhile
```

In `llms.txt`'s table, after the `--grill` row:

```
| `niritasks task refine <uuid> --unattended` | Plan the task with no one watching: two `claude -p` runs in the project folder behind the refine fence, first `refine-task` in `auto` mode, which decides each open question with its recommended answer and writes the plan, then `report-task`, which writes the report and links it as `Report: <path>` without opening it. The task carries `+processing` meanwhile, its card dimmed with an hourglass and offering nothing, and `task refine`, `task report` and `task start` refuse it; it returns when both runs end, minutes later, with a notification `Planned and reported: …`, or which step failed and where its log is (`$XDG_DATA_HOME/niri-tasks/unattended/<uuid8>-<stamp>.log`). What the task box's Add & all runs. Never grills | scriptable, background (minutes, no window), follows the herdr session, else focus |
```

- [ ] **Step 9: Run every unit test**

Run: `cargo test 2>&1 | tail -3`
Expected: all pass, the README and llms.txt checks included.

- [ ] **Step 10: Commit**

```bash
git add src/session.rs src/session/herdr.rs src/refine.rs src/main.rs README.md llms.txt
git commit -F- <<'EOF'
feat(task): add task refine --unattended, a headless refine and report

Add & all needs a refine that asks no one. The flag runs claude -p twice
in the project folder, behind the tab's own fence, with
--permission-prompts none so anything that would ask is refused: the
refine-task skill in its auto mode, then report-task once the task is
planned. The tab's Claude arguments move into session::refiner_flags so
the two cannot drift. The task carries +processing from before the first
run until this process ends, however it ends, and refine, report and
start refuse a task carrying it. Both runs log to one file beside the
reviews, which a failure names.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 4: The mod's `unattended` option

**Files:**
- Modify: `claude/refine-mod/.claude-plugin/plugin.json`
- Modify: `claude/refine-mod/hooks/refine.ts:245-396` (`register`)
- Modify: `claude/refine-mod/tests/refine.test.ts` (new tests at the end)

**Interfaces:**
- Consumes: `options.unattended` from `pluginConfigs` (Task 3's `session_settings`).
- Produces: in refine mode, `write_task_plan` writes without `$.ui.ask`, answering `Wrote the plan to task <uuid>: N note(s), tagged planned.`; in report mode, `show_task_report` writes and links without `xdg-open`, answering `Wrote the report to <path> and linked it from the task as the note "Report: <path>".` Task 5's skills quote both.

- [ ] **Step 1: Write the failing tests**

At the end of `claude/refine-mod/tests/refine.test.ts`:

```ts
// An unattended run: `niritasks task refine --unattended`'s `claude -p`,
// where no one is at the keyboard. The write tool writes without asking and
// the report tool opens nothing.
const UNATTENDED = { uuid: UUID, reports: '/r', unattended: true }
const UNATTENDED_REPORT = { uuid: UUID, reports: '/r', report: true, unattended: true }

test('unattended: shows the plan and writes it, asking nothing', { options: UNATTENDED }, async ($, on) => {
  const { registered, runs, seen } = world(on, SANDBOX, { deny: 'nobody should be asked' })
  await $.session.start(START)
  expect(registered).toEqual([TOOL, REPORT_TOOL])

  const answer = await $.tool.call(CALL)

  expect(answer.deny).toBeUndefined()
  expect(answer.result).toBe(`Wrote the plan to task ${UUID}: 2 note(s), tagged planned.`)
  expect(seen).toEqual(['log: Description: feat: New words', 'log: Note 1: step one', 'log: Note 2: step two'])
  expect(verbs(runs)).toEqual(['export', 'export', 'import'])
  expect(JSON.parse(runs[2]?.init?.stdin ?? 'null')[0].tags).toEqual(['planned', 'zeta'])
})

test('unattended: the task changing under the write still refuses it', { options: UNATTENDED }, async ($, on) => {
  const edited = { ...TASK, annotations: [...TASK.annotations, { entry: '20260101T000002Z', description: 'added' }] }
  const { runs } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [TASK, edited])
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toContain('Nothing written: the task changed since it was read')
  expect(verbs(runs)).toEqual(['export', 'export'])
})

test('unattended: a line too long is still refused before anything', { options: UNATTENDED }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, { deny: 'nobody should be asked' })
  await $.session.start(START)
  const answer = await $.tool.call({ ...CALL, notes: ['x'.repeat(2000)] })
  expect(answer.deny).toStartWith('Nothing written: note 1 would show as a line of 2008 characters')
  expect(seen).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
})

test('unattended: a report no one asked for is refused, as nobody can ask', { options: UNATTENDED }, async ($, on) => {
  const { writes } = world(on, SANDBOX, { deny: 'nobody should be asked' })
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toStartWith('show_task_report: the person has not asked for a report.')
  expect(writes).toEqual([])
})

test('unattended report mode: writes the page and links it, opening nothing', { options: UNATTENDED_REPORT }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })

  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH} and linked it from the task as the note "Report: ${REPORT_PATH}".`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH])
  expect(seen).toEqual([`log: Report: ${REPORT_PATH}`])
  expect(runs.map(run => run.argv)).toEqual([
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    ['task', 'rc.hooks=off', 'rc.verbose=nothing', 'import'],
  ])
  const [imported] = JSON.parse(runs[1]?.init?.stdin ?? 'null')
  expect(imported.annotations.at(-1).description).toBe(`Report: ${REPORT_PATH}`)
})

test('unattended report mode: a failed import says nothing was opened', { options: UNATTENDED_REPORT }, async ($, on) => {
  engine(on, SANDBOX)
  on('process.run', ($, e) =>
    e.argv.includes('export')
      ? ran(JSON.stringify([PLANNED]))
      : { value: { ...ran('', 2).value, stderr: 'Not a valid JSON value.' } },
  )
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe(
    `Wrote the report to ${REPORT_PATH}, but task import failed (2): Not a valid JSON value. The task does not link it.`,
  )
})

test('attended sessions are unchanged by the option being false', { options: { ...REPORTS, unattended: false } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, [REPORT, WRITE], [TASK])
  await $.session.start(START)
  expect((await $.tool.call(CALL)).deny).toContain(`the person chose "${REPORT}"`)
  expect(seen.at(-1)).toBe(`ask: Write this to the task? [${WRITE} | ${REPORT} | Change something] (Task plan)`)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(runs.at(-1)?.argv[0]).toBe('sh')
})
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: the `unattended` tests fail (the write tool answers `Nothing written: the person could not be asked`, the report tool answers with `opened it in the browser` and runs `sh`); every earlier test passes. The `unattended: false` test may also fail with an unknown-option warning until Step 3 declares it.

- [ ] **Step 3: Declare the option**

In `claude/refine-mod/.claude-plugin/plugin.json`, change `description` to:

```
"Gives a refine session two tools: write_task_plan, which rewrites one Taskwarrior task's description and notes, and show_task_report, which writes and opens an HTML report of the plan when the person asks for one. With `report` set, a Report session gets show_task_report alone, on the task's own plan, and the mod links the report from the task. With `unattended` set, for a `claude -p` run with no one at the keyboard, write_task_plan writes without asking and show_task_report does not open the browser."
```

and after the `report` entry in `userConfig`:

```json
    "unattended": {
      "type": "boolean",
      "title": "Unattended",
      "description": "The session is a `claude -p` run with no one at the keyboard (`niritasks task refine --unattended`): write_task_plan writes without asking, its plan lines still logged and its stale-read check still made, and show_task_report does not open the browser. Unset, the session asks and opens as usual.",
      "required": false,
      "default": false
    }
```

- [ ] **Step 4: Implement both branches in `hooks/refine.ts`**

In `register`, after `const reportOnly = options.report === true`:

```ts
  // Unattended: a `claude -p` run with no one at the keyboard
  // (`niritasks task refine --unattended`, the task box's Add & all). The
  // command was the ask, as the button press is in report mode: the write
  // tool writes with no question, its lines still logged and the task still
  // read again first, and the report tool opens nothing. $.ui.ask rejects in
  // a -p run anyway; this is what makes the run do its job rather than fail
  // at the question.
  const unattended = options.unattended === true
```

Replace the block in the write tool's hook from `// A new question supersedes any earlier REPORT choice.` down to `if (answer !== WRITE) return { deny: notApproved(answer) }` with:

```ts
    const answer = await approval($)
    if (typeof answer !== 'string') return answer
    if (answer === REPORT && reports !== undefined) {
      askedFor = { description: input.description, notes: input.notes }
      return { deny: REPORT_FIRST }
    }
    if (answer !== WRITE) return { deny: notApproved(answer) }
```

and add, before `on('session.start', …)` and after the `fonts` declaration:

```ts
  // The person's answer to "Write this to the task?", or the refusal to
  // answer the call with when they could not be asked. Unattended, there is
  // no one to ask and the command was the ask: WRITE, with nothing shown but
  // the lines already logged.
  const approval = async ($: EngineInterface): Promise<string | { deny: string }> => {
    if (unattended) return WRITE
    // A new question supersedes any earlier REPORT choice.
    askedFor = undefined
    const choices = reports === undefined ? [WRITE, CHANGE] : [WRITE, REPORT, CHANGE]
    try {
      return await $.ui.ask(QUESTION, { options: choices, header: HEADER })
    } catch (error) {
      return { deny: notAsked(error instanceof Error ? error.message : String(error)) }
    }
  }
```

In the report tool's `reportOnly` branch, replace from `$.ui.log(\`Report: ${path}\`)` through the final `return { result: … }` of that branch with:

```ts
      $.ui.log(`Report: ${path}`)
      // Unattended, nothing opens: no one is at the screen, and the task's
      // Report: note is how the report is found later.
      if (!unattended) await $.process.run(openArgv(path))
      const opened = unattended ? '' : ' and opened it'

      // The session's one write to the task, and the mod's, not the model's:
      // the note's path is the one just written.
      const imported = await $.process.run([...TASK, 'rc.verbose=nothing', 'import'], {
        stdin: JSON.stringify([withReportNote(task, path, await $.clock.now())]),
      })
      if (imported.exitCode !== 0) {
        return {
          deny:
            `Wrote the report to ${path}${opened}, but task import failed (${imported.exitCode}): ` +
            `${imported.stderr.trim()} The task does not link it.`,
        }
      }
      linked = path
      return {
        result: unattended
          ? `Wrote the report to ${path} and linked it from the task as the note "Report: ${path}".`
          : `Wrote the report to ${path}, opened it in the browser, and linked it from the task ` +
            `as the note "Report: ${path}".`,
      }
```

The refine-mode report branch (after `if (askedFor === undefined) return { deny: NOT_ASKED_FOR }`) is unchanged: an unattended refine never answers REPORT, so it is never reached.

- [ ] **Step 5: Validate and run the mod's tests**

Run: `cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -6`
Expected: validates; every test passes, the seven new ones included, the attended strings unchanged character for character.

- [ ] **Step 6: Commit**

```bash
git add claude/refine-mod/.claude-plugin/plugin.json claude/refine-mod/hooks/refine.ts claude/refine-mod/tests/refine.test.ts
git commit -F- <<'EOF'
feat(refine): let the mod write and report with no one to ask

An unattended run's claude -p has no one at the keyboard, and $.ui.ask
rejects there. With the new unattended option the write tool takes the
command as the approval, as report mode takes the button press: it still
logs the plan and still refuses a task that changed since it was read,
but writes with no question. The report tool writes and links the report
without opening a browser nobody is watching. Attended sessions are
unchanged.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 5: `refine-task auto`, and the report skill's note

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md` (frontmatter, the intro paragraph, §3, §4, §5)
- Modify: `.claude/skills/report-task/SKILL.md` (§4)

**Interfaces:**
- Consumes: the prompts Task 3 sends (`/refine-task <uuid> auto`, `/report-task <uuid>`) and the tool answers Task 4 produces.
- Produces: skill text an unattended `claude -p` follows end to end with no question.

- [ ] **Step 1: `refine-task`'s frontmatter and intro**

Frontmatter becomes:

```yaml
---
name: refine-task
description: >
  Work one Taskwarrior task up into a plan — a sharper one-line description and a
  consolidated set of notes — then write it back and tag it +planned. Invoked by
  niri-tasks' task card's Refine and Grill me buttons as `/refine-task <uuid> [grill]`,
  and by `niritasks task refine <uuid> --unattended` as `/refine-task <uuid> auto`.
disable-model-invocation: true
argument-hint: <uuid> [grill|auto]
---
```

The line `Arguments: `$ARGUMENTS` — a task uuid, then optionally `grill`.` becomes `Arguments: `$ARGUMENTS` — a task uuid, then optionally `grill` or `auto`.`

In the bold intro paragraph, `Your one write is the Taskwarrior update in step 5, and the write tool asks the user itself before it writes.` becomes `Your one write is the Taskwarrior update in step 5, and the write tool asks the user itself before it writes (in `auto` mode it writes at once: the command that started you was the approval).`

- [ ] **Step 2: §3's `auto` bullet**

In `## 3. Work it up`, after the `grill` bullet (which ends `don't ask for it in a separate dialog.`), add:

```markdown
- **`auto`:** an unattended run — `niritasks task refine <uuid> --unattended`,
  or the task box's Add & all — in a `claude -p` process with no one at the
  keyboard. Ask nothing: AskUserQuestion is not available, and a question
  would end the run with the task unplanned. Where quick mode would have
  asked, take the answer you would have recommended and record it as its own
  `Decided: …` note, saying in it that it was decided for the user, so they
  can see what was chosen and change it. Decide at most three such questions;
  past that, the task is too open for an unattended plan: write the plan with
  what you have and say so in a `Decided:` note. Never offer or build a
  report: the run builds one itself afterwards, with `report-task`. Grilling
  is never unattended.
```

- [ ] **Step 3: §4 and §5 under `auto`**

At the end of `## 4. Propose`, before `Then go straight to step 5.`, add a paragraph:

```markdown
In `auto` mode, show the proposal and print the block all the same: the
transcript is the run's log, and the block is what the user reads later to
see what was written and why. Nothing waits on them.
```

In `## 5. Write`, after the paragraph beginning `The tool shows the user the description and notes it was given, asks` (which ends `touches nothing else. Its answer says what happened:`), insert before the bullet list:

```markdown
In `auto` mode the tool asks no one: it logs the plan, reads the task again,
writes and answers **Wrote the plan …**. Call it once. Of the answers below,
only the ones that name something to fix in the plan itself (a line too
long, a hidden character, a note on two lines) are worth a second call; on
any other **Nothing written** or failure, stop and say what it answered —
there is no one to ask, and the run reports the task as unplanned.
```

- [ ] **Step 4: `report-task`'s note**

In `.claude/skills/report-task/SKILL.md`'s `## 4. Show it`, after `It asks the user nothing: pressing Report was the ask.` and before `Its answer says what happened:`, add:

```markdown
In an unattended run (`niritasks task refine <uuid> --unattended`'s second
step, after `refine-task` in `auto` mode) the tool does not open the
browser: nobody is at the screen, and the `Report: <path>` note is how the
report is found later. Its answer then reads **Wrote the report to … and
linked it from the task**, without *opened it*; everything else is the
same, and you still end at step 5.
```

- [ ] **Step 5: Check the installed links still read the repo's files**

Run: `readlink -f ~/.claude/skills/refine-task/SKILL.md ~/.claude/skills/report-task/SKILL.md && grep -c "auto" .claude/skills/refine-task/SKILL.md`
Expected: both links resolve into a niri-tasks checkout (`install.sh` is unchanged; if they point at another worktree's copy, the hand check in Task 7 runs `bash install.sh` from `main` after landing), and the count is at least 6.

- [ ] **Step 6: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md .claude/skills/report-task/SKILL.md
git commit -F- <<'EOF'
feat(refine): give refine-task an auto mode for unattended runs

/refine-task <uuid> auto asks nothing: where quick mode would have put a
question, it takes its own recommended answer and records it as a
Decided: note the user can see and change later, offers no report, and
calls the write tool once, which writes without asking in that mode.
report-task notes that the same run's report is linked but not opened.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 6: The Add & all button

**Files:**
- Modify: `src/taskbox/form.rs:21-37` (`Submission`), `:71` (`submission`), tests at `:128-133`
- Modify: `src/taskbox.rs:33` (`pub use`), `:238-253` (the footer), `:262-318` (`do_submit`, the handlers, the keys), tests
- Modify: `src/daemon.rs:280-313` (`Request::Add`)

**Interfaces:**
- Consumes: `refine::spawn_quick`, `refine::spawn_unattended` (Task 3).
- Produces: `taskbox::Then { Nothing, Refine, All }` (`Default` is `Nothing`), `Submission.then: Then`, re-exported as `taskbox::Then`; the daemon's add closure matches on it.

- [ ] **Step 1: Write the failing tests in `src/taskbox/form.rs`**

Replace the last assertion of `note_texts_are_the_notes_in_order` with:

```rust
        assert_eq!(s.then, Then::Nothing, "what is on screen says nothing about which button was pressed");
        assert_eq!(Then::default(), Then::Nothing);
```

- [ ] **Step 2: Implement `Then`**

In `src/taskbox/form.rs`, replace `Submission`'s `refine` field with:

```rust
    /// What follows the add, by which button was pressed: nothing (Add), a
    /// refine in a herdr tab (Add & refine) or an unattended refine and
    /// report in the background (Add & all). Set by the button, not by what
    /// is on screen, so [`submission`] always leaves it `Nothing`.
    pub then: Then,
```

and before `Submission`:

```rust
/// What the daemon does once an added task has its uuid. One value per
/// button on the add box's footer; an existing task's box has only Save, so
/// its submissions are always `Nothing`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Then {
    /// Add, or Save: the task is added or saved, and that is all.
    #[default]
    Nothing,
    /// Add & refine: `niritasks task refine <uuid>`, a tab in the
    /// workspace's herdr session.
    Refine,
    /// Add & all: `niritasks task refine <uuid> --unattended`, a headless
    /// refine and report in the background.
    All,
}
```

In `submission`, the return becomes `Some(Submission { description, notes, then: Then::Nothing })`.

Run: `cargo test --lib taskbox::form 2>&1 | tail -3`
Expected: pass (the crate does not compile yet elsewhere; run Step 4 first if `cargo test` refuses, then come back).

- [ ] **Step 3: Write the failing tests in `src/taskbox.rs`**

In `mod tests`, after `the_hint_names_the_default_button`:

```rust
    /// Ctrl+Enter presses the default button and Ctrl+Shift+Enter Add &
    /// refine; Add & all is pressed by the pointer alone, so no key maps to
    /// it, and an existing task's box, with only Save, maps both to nothing.
    #[test]
    fn the_keys_press_add_or_add_and_refine_never_add_and_all() {
        assert_eq!(default_then(&BoxConfig::add("proj", false)), Then::Nothing);
        assert_eq!(default_then(&BoxConfig::add("proj", true)), Then::Refine);
        assert_eq!(shortcut_then(&BoxConfig::add("proj", false)), Then::Refine);
        assert_eq!(shortcut_then(&BoxConfig::add("proj", true)), Then::Refine);
        let t: crate::task::Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        let edit = BoxConfig::for_task(Mode::Edit, t);
        assert_eq!(default_then(&edit), Then::Nothing);
        assert_eq!(shortcut_then(&edit), Then::Nothing);
    }

    /// Only the add box has the two extra buttons; the hint names the one
    /// with a key and is unchanged by the one without.
    #[test]
    fn only_the_add_box_has_add_and_refine_and_add_and_all() {
        assert!(has_add_buttons(Mode::Add));
        assert!(!has_add_buttons(Mode::Edit));
        assert!(!has_add_buttons(Mode::Note));
        assert!(!hint(Mode::Add, false).contains("all"));
        assert!(!hint(Mode::Add, true).contains("all"));
    }
```

- [ ] **Step 4: Implement the button and the helpers in `src/taskbox.rs`**

Line 33 becomes `pub use form::{Submission, Then};`.

After `hint` (before the `WIDTH` constants):

```rust
/// Whether the box has Add & refine and Add & all beside Add: adding only.
/// Refining from Edit or Note is the card's Refine's job, and a task that
/// exists is not planned by saving it.
fn has_add_buttons(mode: Mode) -> bool {
    mode == Mode::Add
}

/// Which button Ctrl+Enter presses in this box: Add & refine in the box
/// Mod+Alt+Shift+T opens, else Add or Save.
fn default_then(cfg: &BoxConfig) -> Then {
    if cfg.refine { Then::Refine } else { Then::Nothing }
}

/// Which button Ctrl+Shift+Enter presses: Add & refine where there is one,
/// else the default, which saves, as it always did. Add & all has no key:
/// a run that takes minutes and opens nothing is worth reaching for.
fn shortcut_then(cfg: &BoxConfig) -> Then {
    if has_add_buttons(cfg.mode) { Then::Refine } else { Then::Nothing }
}
```

In `build_window`'s footer, replace from `// Add mode only: refining from Edit or Note is the card's Refine's job.` through `root.append(&footer);` with:

```rust
    // Add mode only (see `has_add_buttons`): Add & refine opens a tab; Add &
    // all plans and reports in the background with no tab and no key.
    let extra = has_add_buttons(cfg.mode)
        .then(|| (gtk4::Button::with_label("Add & refine"), gtk4::Button::with_label("Add & all")));
    if let Some((refine, all)) = &extra {
        footer.append(refine);
        footer.append(all);
    }
    root.append(&footer);
```

`do_submit`'s closure takes a `Then`:

```rust
        move |then: Then| {
            let submission = form::submission(
                &loaded_description,
                &buffer_text(&notes.description),
                &notes.rows(),
            );
            // An empty description saves nothing, and closing anyway would
            // throw the note edits away without a word: stay open, with the
            // cursor where the fix is. Esc and Cancel are what discard.
            let Some(mut submission) = submission else {
                focus_end(&notes.description);
                return;
            };
            submission.then = then;
            if let Some(window) = window.upgrade() {
                window.close();
            }
            on_submit(submission);
        }
```

The button handlers become:

```rust
    {
        let do_submit = do_submit.clone();
        submit.connect_clicked(move |_| do_submit(Then::Nothing));
    }
    if let Some((refine, all)) = &extra {
        {
            let do_submit = do_submit.clone();
            refine.connect_clicked(move |_| do_submit(Then::Refine));
        }
        let do_submit = do_submit.clone();
        all.connect_clicked(move |_| do_submit(Then::All));
    }
```

In the key controller, replace the two bindings and their comment:

```rust
        // Which button each shortcut presses (`default_then`,
        // `shortcut_then`): Ctrl+Enter the default, Ctrl+Shift+Enter Add &
        // refine where there is one, and saves where there is not, as it
        // always did. Nothing presses Add & all.
        let default = default_then(cfg);
        let shortcut = shortcut_then(cfg);
```

and the two match arms:

```rust
                KeyAction::Save => do_submit(default),
                KeyAction::Refine => do_submit(shortcut),
```

- [ ] **Step 5: Implement the daemon's spawn**

In `src/daemon.rs`'s `Request::Add` closure, replace from `// Spawned, not run: herdr must not hold up the` through the closing of that `if let`:

```rust
                            // Spawned, not run: neither herdr nor a Claude
                            // run that takes minutes may hold up the panels'
                            // main loop.
                            if let Some(uuid) = uuid {
                                match sub.then {
                                    taskbox::Then::Nothing => {}
                                    taskbox::Then::Refine => crate::refine::spawn_quick(&uuid),
                                    taskbox::Then::All => crate::refine::spawn_unattended(&uuid),
                                }
                            }
```

- [ ] **Step 6: Run every unit test**

Run: `cargo test 2>&1 | tail -3`
Expected: all pass.

- [ ] **Step 7: Build and check the box by hand**

Run: `bash try.sh`
In the nested niri window press Alt+T: the footer reads `Cancel`, `Add`, `Add & refine`, `Add & all`, left to right, with the hint wrapping beside them, and the window is still 800×760 (it is fixed size; if the four buttons no longer fit, shorten nothing — report it). Type `chore: throwaway add & all` and click **Add & all** with the pointer. In the try shell: a `[notify] Added to +niri_tasks: …` line, then within a couple of seconds the panel's card for it dimmed with an hourglass (Alt+Ctrl+T to open the panel: no buttons on it, hint `Planning in the background`, Ctrl+Enter and Ctrl+Delete do nothing). Then either the run proceeds (claude on the try shell's PATH) or a `[notify]` line names the failed step and the log under the sandbox's `XDG_DATA_HOME`; in both cases `task <uuid8> export` shows the `processing` tag gone once the notification arrives. Type `exit` in the try shell.

- [ ] **Step 8: Commit**

```bash
git add src/taskbox/form.rs src/taskbox.rs src/daemon.rs
git commit -F- <<'EOF'
feat(taskbox): add Add & all, which plans and reports unattended

A third button on the add box, after Add & refine and with no key: it
adds the task and spawns niritasks task refine <uuid> --unattended on
it, so the task comes back planned and reported with nothing opening on
screen. Submission.refine becomes the Then enum, one value per button,
and the daemon matches on it.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 7: Docs, the ADR addendum, the full test run and the end-to-end check

**Files:**
- Modify: `README.md:316-320` (the task box section), `:331-332` (the box's key table stays), `:181` (requirements)
- Modify: `llms.txt:14` (rule 6), `:31` (the `task add` row)
- Modify: `CONTEXT.md:52-72` (a Processing task term after Up next task), `:81-99` (Task card), `:100-110` (Task action), `:192-197` (Task box)
- Modify: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md:3-32` (the dated addenda)

**Interfaces:**
- Consumes: everything Tasks 1–6 built.
- Produces: docs that say what the button, the flag, the tag and the mode do, and a green `tests/all.sh`.

- [ ] **Step 1: README**

In `### The task box`, replace the sentence beginning `Adding has a second button, **Add & refine**:` through `and a notification says why.` with:

```markdown
Adding has two more buttons. **Add & refine** adds the task and hands it
straight to Claude's refine-task skill, in a new tab of the workspace's herdr
session, as a card's Refine does. **Add & all** adds it and plans it in the
background instead: `niritasks task refine <uuid> --unattended` runs Claude
headless in the project folder, behind the same fence as a refine tab, first
to write the plan asking nothing (each question it would have asked is
decided with its recommended answer and recorded as a `Decided:` note), then
to build the report and link it from the task's notes as `Report: <path>`,
without opening it. The card is dimmed with an hourglass meanwhile and offers
nothing; a notification says `Planned and reported` when both are done, or
which step failed and where its log is (`~/.local/share/niri-tasks/unattended/`).
Add & all has no key and needs Claude Code but not herdr. If a refine fails,
either way the task stays added and a notification says why.
```

In `### Requirements`, change `A card's **Refine**, **Grill me** and **Report** need both herdr and Claude Code` to `A card's **Refine**, **Grill me** and **Report** need both herdr and Claude Code, and the box's **Add & all** Claude Code alone,` keeping the rest of the sentence.

- [ ] **Step 2: llms.txt**

Rule 6 becomes:

```markdown
6. **Never run a GUI command unattended.** The Runs column below says which commands open a window and wait for a person, which open a herdr tab, which are scriptable, and which are scriptable but run for minutes in the background with no window (`task refine --unattended`). A scriptable command prints, or changes the database, and exits; a background one returns when its Claude runs end, and says so with a notification rather than output.
```

The `niritasks task add` (no text) row's Does column becomes: `With no text, the task box, to type a task and its notes; its Add & refine button adds and refines in a herdr tab, its Add & all adds and runs `task refine <uuid> --unattended`; exit 1 with no daemon`.

- [ ] **Step 3: CONTEXT.md**

After the **Up next task** entry, add:

```markdown
**Processing task**:
A pending task an unattended refine — `niritasks task refine <uuid>
--unattended`, the task box's Add & all — is planning, marked with the
`+processing` tag from before its first Claude run until the run ends,
however it ends. Its task card is dimmed, with an hourglass, and has no
action row, its hint reading Planning in the background; Refine, Report and
Start working refuse it at the CLI too. A tag a killed run left is cleared
with `task <uuid8> modify -processing`.
_Avoid_: busy, locked, running (an active task is the one being worked)
```

In **Task card**, after `A finished task's card, on the Finished tab alone, is dimmed, carries a check, and counts its age from when the task was finished.` add: `A processing task's card is dimmed too, carries an hourglass, and has no action row.`

In **Task action**, change `Which ones a task gets goes by its state: only a planned task on the list gets Report,` (wrapped over two lines in the file) to `Which ones a task gets goes by its state: a processing task gets none, only a planned task on the list gets Report,`, rewrapping the paragraph.

**Task box** becomes:

```markdown
**Task box**:
The GTK window for adding a task or editing one — its description and its
note rows. Edit and Note open the same box. When adding, two more buttons:
Add & refine (Ctrl+Shift+Enter) adds the task and refines it straight away
in a herdr tab, and Add & all, with no key, adds it and plans and reports it
unattended in the background (see Processing task); Mod+Alt+Shift+T opens
the box with Add & refine as the default.
Only one is ever open: asking for another brings the open one forward.
_Avoid_: dialog, prompt
```

- [ ] **Step 4: The ADR addendum**

In `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, after the `*2026-10-10:*` paragraph on Report (ending `The decision is unchanged.`), add:

```markdown
*2026-10-10:* the fence also runs with no tab. `niritasks task refine <uuid>
--unattended`, which the task box's Add & all starts, runs `claude -p` twice
in the project folder with the tab's own Claude arguments
(`session::refiner_flags`, now shared with `agent_start_claude_refiner` so
the two cannot drift) and `--permission-prompts none`, so anything that
would ask is refused instead of waited on: `/refine-task <uuid> auto`, then
`/report-task <uuid>`. The mod's `unattended` option makes `write_task_plan`
write with no `$.ui.ask` (which rejects in a `-p` run) and
`show_task_report` not open the browser; the plan lines are still logged and
the stale-read refusal still stands. Candidate B's approval is therefore not
enforced in this mode: the person's approval is the button press or the
command, given up front, as a Report session's is, and the skill's `auto`
mode records every question it decided for them as a `Decided:` note. What
bounds the run is unchanged: the sandbox, the hidden sockets and
credentials, the removed tools and the standing instruction, none of which
the mode touches. The task carries `+processing` while it runs, so nothing
else writes it meanwhile. The decision is unchanged.
```

- [ ] **Step 5: Run every suite**

Run: `cargo test 2>&1 | tail -3 && (cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -3) && bash tests/all.sh 2>&1 | tail -12`
Expected: `cargo test` all pass; the mod validates and its tests all pass; `tests/all.sh` ends with `N passed   0 failed   M skipped`, every skip naming a missing prerequisite, none a failure. The box and panel suites, if they run, park a nested niri on the last workspace: keep off it.

- [ ] **Step 6: Check "done when" end to end, once**

With `claude` on `$PATH` and the installed skills linked (Task 5 step 5), in `bash try.sh`'s try shell:

```bash
niritasks task add "chore: Throwaway for Add & all"      # note the uuid8 in the notification, or task +niri_tasks newest
niritasks task refine <uuid8> --unattended
```

While it runs (a few minutes): `task <uuid8> export` shows `processing` in `tags`; the nested panel's card is dimmed with the hourglass and offers nothing; `niritasks task refine <uuid8>`, `niritasks task report <uuid8>` and `niritasks task start <uuid8>` each refuse with the processing message; nothing opens on screen. When it returns: the try shell prints `[notify] Planned and reported: chore: …`; `task <uuid8> export` shows `planned` and no `processing`, `Goal:`, at least one `Decided:`, `Steps:` and `Done when:` notes, and a last `Report: <path>` note whose file exists under the sandbox's `XDG_DATA_HOME/niri-tasks/reviews`; the log under `XDG_DATA_HOME/niri-tasks/unattended/<uuid8>-<stamp>.log` holds both runs under `== refine:` and `== report:` headings. Then the failure path: `PATH=/usr/bin:/bin niritasks task refine <other uuid8> --unattended` refuses with `Claude Code (\`claude\`) is not on PATH`, the error names the log, and `task <other> export` has no `processing`. Finally the pointer path of Task 6 step 7 once more, letting the run finish. Type `exit`.

- [ ] **Step 7: Commit**

```bash
git add README.md llms.txt CONTEXT.md docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -F- <<'EOF'
docs: describe Add & all and the unattended refine

The task box section and CONTEXT.md's Task box name the third button;
a Processing task term, the Task card and Task action entries say what
a card being planned shows and refuses; llms.txt's rule 6 gains the
background kind of command; ADR 0002 records that the fence now also
runs headless, with the approval given up front rather than at the
tool's question.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

## Self-review

- **Spec coverage.** Goal: the button (Task 6), the background refine and report (Task 3), the processing card (Tasks 1, 2). Headless with the shared flags, `--permission-prompts none`, default mode, no herdr (Task 3: `refiner_flags`, `claude_unattended_argv`). Two runs, the planned check between, one log beside reviews (Task 3: `unattended_steps`, `log_path`). The mod's `unattended` option in both modes, attended unchanged (Task 4). `refine-task auto` asking nothing, `Decided:` notes, no report, one write; `report-task`'s note; `install.sh` untouched (Task 5). `+processing` set before and cleared on every path, `PROCESSING_TAG`, `is_processing`, `Card.processing`, `Status::Processing`, dimmed hourglass, `TaskState.processing`, `applies` offers nothing, hint, Ctrl+Enter and Ctrl+Delete no-ops (Tasks 1, 2), the four CLI refusals (Task 3). `Then` enum, the button after Add & refine, the daemon's spawn through `niri::spawn`, no key, no hint change, no IPC change (Task 6). Not opened; `Planned and reported`; the failure says the step and the log (Task 3). README, llms.txt, CONTEXT.md, ADR (Tasks 3 and 7). `cargo test`, `claude plugin validate`/`test`, `tests/all.sh`, `try.sh` (Task 7). "Done when" is walked in Task 7 step 6 and Task 6 step 7. Out of scope is left out: no key, no `task add --all`, no unattended grill (`conflicts_with`), no cancel or watch, no Start after the report, no recovery of a leftover tag, no timeout.
- **Placeholders.** Every code step shows its code. The only elided text is inside `hooks/refine.ts`, where the plan names the exact first and last lines of the block being replaced in a file the implementer has open.
- **Type consistency.** `session_settings(…, mode: Mode, unattended: bool)` is called with eight arguments in `launch`, `unattended_steps` and every test; `refiner_flags(settings: &str, mod_dir: &Path)` has that shape in `herdr.rs`, its re-export, `claude_unattended_argv` and both tests; `log_path(&Dirs, &str, i64)` matches `run_unattended`'s call and its test; `task::stamp(i64) -> String` is what `log_path` calls; `Then::{Nothing, Refine, All}` is spelled the same in `form.rs`, `taskbox.rs`, `daemon.rs` and the tests; `Action::PROCESSING_HINT` is the name in `panel/actions.rs`, `panel/state.rs`'s test and the hint; the mod's result and refusal strings in `refine.ts` match the tests and the skills character for character.
