# Report a Planned Task From Its Action Row Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A **Report** button on a planned task's action row opens a "Report: …" Claude tab that builds the refine report of the plan the task already holds, opens it in the browser and links it from the task's notes, without refining again.

**Architecture:** A new `Action::Report` in the Rust action model runs `niritasks task report <uuid>`, which refuses a task that is not pending or not planned and then starts the same fenced herdr session Refine does, in a third `refine::Mode::Report`, with the prompt `/report-task <uuid>`. The refine mod gets a `report` boolean option: in report mode it registers only `show_task_report`, armed from the start on the task's own description and notes, and after writing and opening the report it adds a `Report: <path>` note to the task by `task import` with no question. A new `report-task` skill drives that session; `refine-task` is untouched.

**Tech Stack:** Rust (clap, serde_json, anyhow; `cargo test`), the TypeScript Claude Code mod in `claude/refine-mod` (`claude plugin test`, `claude plugin validate`), Taskwarrior 3.5, herdr, bash (`install.sh`, `tests/all.sh`).

**Spec:** Taskwarrior task `0db7f500-bd6a-4683-aea9-baeefe6087a0`, "feat: Report a planned task from its action row". Read it with `task rc.json.array=on 0db7f500-bd6a-4683-aea9-baeefe6087a0 export`; its notes are the spec and are quoted in **Global Constraints**.

## Global Constraints

- **Who gets Report:** only a planned task on the list: `state.planned && !state.off_list()`. An unplanned task gets Refine or Grill me first; a waiting or finished task gets nothing new.
- **The button's facts:** `Action::Report`, after `Grill` in `Action::ALL`; label `Report`; key `p`; Refine's mauve (`c::REFINE`); Font Awesome's file-text glyph `\u{f15c}`; `keeps_keyboard: false`; `leaves_the_list: false`; command `task report <uuid>`. `Action::advance` (Ctrl+Enter) is unchanged.
- **The command:** `niritasks task report <uuid>`, a new `TaskCommand` in `src/main.rs`. It refuses a task that is not pending or not planned, then calls `refine::launch` with `Mode::Report`: prompt `/report-task <uuid>`, tab label `Report: …`, the same fence, settings, mod and agent name (`task-<uuid8>`) as Refine, so a Report pressed while a refine (or another report) is open on the task goes to that tab.
- **The mod's option:** a `report` boolean in `claude/refine-mod/.claude-plugin/plugin.json`'s `userConfig`, set through `pluginConfigs` by `refine::session_settings`. In report mode only `show_task_report` is registered, armed from session start with the task's own description and notes as the plan it shows, and after writing and opening the report it adds a `Report: <path>` note to the task itself by `task import`, other notes and tags kept, with no question: the button press was the ask. **Refine mode behaves exactly as now.**
- **The skill:** `.claude/skills/report-task/SKILL.md`, linked by `install.sh`: read the task (stop unless pending and planned), ground it as refine-task step 2 does, fill the fields from the shared `report-catalogue.md`, call `show_task_report`, then export the task and confirm the new `Report:` note. It never calls `write_task_plan`; refine-task's "Report, when asked" stays as it is.
- **Docs:** README (button table, commands block), llms.txt (commands table), CONTEXT.md (Task action, Action row, Refine report) and a dated addendum to `docs/adr/0002`.
- **Done when:** a planned card shows Report (p) between Grill me and Edit and an unplanned, waiting or finished card does not; pressing it opens a "Report: …" tab whose Claude writes the report under the reviews folder, opens it in the browser, and the card's notes gain `Report: <path>`; `niritasks task report` refuses an unplanned task; a refine's "Show me a report first" still works; `cargo test`, `claude plugin test claude/refine-mod` and `tests/all.sh` pass, the README and llms.txt command tests included.
- **Out of scope:** reopening an existing report from the card; reporting on an unplanned task; changing the report's slides, limits or catalogue.
- **Commits:** Conventional Commits, subject at most 72 characters, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- **Tests stay green at every commit.** `cargo test` checks that README's Commands block and llms.txt run every subcommand, so the command's doc lines land in the same commit as the command.

## File Structure

- `src/actions.rs` (modify): the `Report` variant, its `facts` row, `applies`, `label`, `args`, and the tests.
- `src/panel/actions.rs` (modify): row and hint tests; `src/panel/keys.rs` (modify): the `p` key tests; `src/panel/surface.rs` (modify): the module doc's key list.
- `src/refine.rs` (modify): `Mode::Report`, `prompt`, `tab_label`, `session_settings` gets the mode and sets `options.report`.
- `src/main.rs` (modify): `TaskCommand::Report` and its handler.
- `README.md`, `llms.txt` (modify, in two tasks): the command line and row with the command; the button table, keybind paragraph and requirements with the docs.
- `claude/refine-mod/.claude-plugin/plugin.json` (modify): the `report` option.
- `claude/refine-mod/hooks/plan.ts` (modify): export `notesOf`.
- `claude/refine-mod/hooks/report.ts` (modify): `unreportable`, `withReportNote`, `madeAlready`.
- `claude/refine-mod/hooks/refine.ts` (modify): report mode in `session.start` and the report tool; `reportSpec(reportOnly)`.
- `claude/refine-mod/tests/report.test.ts`, `claude/refine-mod/tests/refine.test.ts` (modify): the pure helpers and the report-mode session.
- `.claude/skills/report-task/SKILL.md` (create); `install.sh` (modify): its link.
- `CONTEXT.md`, `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` (modify).

---

### Task 1: `Action::Report` on the action row

**Files:**
- Modify: `src/actions.rs`
- Modify: `src/panel/actions.rs` (tests only)
- Modify: `src/panel/keys.rs:175-200` (tests only)
- Modify: `src/panel/surface.rs:41-43` (module doc)

**Interfaces:**
- Consumes: `TaskState`, `Action::facts`, `panel::style::REFINE`.
- Produces: `Action::Report`, with `Report.label(_) == "Report"`, `Report.args(u) == ["task", "report", u]`, `Report.facts().key == Some("p")`, `Action::ALL: [Action; 14]` with `Report` right after `Grill`. Task 2's CLI subcommand is what `args` names.

- [ ] **Step 1: Write the failing tests in `src/actions.rs`**

In `mod tests`, change these tests and add the new ones:

```rust
    /// A task on the list, not started: everything but the ways back from
    /// somewhere it is not. Report only once it has a plan.
    #[test]
    fn a_task_on_the_list_gets_all_but_session_back_and_stop() {
        assert_eq!(
            offered(TaskState::default()),
            vec![Start, Refine, Grill, Edit, Speak, UpNext, Complete, Move, Wait, Remove]
        );
        assert_eq!(
            offered(TaskState { planned: true, ..TaskState::default() }),
            vec![Start, Refine, Grill, Report, Edit, Speak, UpNext, Complete, Move, Wait, Remove]
        );
    }

    /// Report needs a plan to report on, and a task on the list: never an
    /// unplanned one, which gets Refine or Grill me first, nor a waiting or
    /// finished one, planned or not.
    #[test]
    fn report_needs_a_plan_and_a_task_on_the_list() {
        assert!(!Report.applies(TaskState::default()));
        assert!(Report.applies(TaskState { planned: true, ..TaskState::default() }));
        assert!(Report.applies(TaskState { planned: true, active: true, ..TaskState::default() }));
        assert!(!Report.applies(TaskState { planned: true, waiting: true, ..TaskState::default() }));
        assert!(!Report.applies(TaskState { planned: true, finished: true, ..TaskState::default() }));
    }
```

In `labels_read_as_the_tooltips_do`, the expected list becomes:

```rust
            vec![
                "Go to session", "Back to list", "Start working", "Refine", "Grill me", "Report", "Edit",
                "Speak", "Up next", "Complete", "Move to workspace", "Stop", "Waiting", "Remove",
            ]
```

In `each_action_runs_its_command`, after the Grill line add:

```rust
        assert_eq!(Report.args(u), vec!["task", "report", u]);
```

In `the_table_says_what_a_press_does_to_the_panel`, add before the `keeps` assertion:

```rust
        // Report opens a herdr tab, which takes the keyboard, and the card stays.
        assert!(!Report.keeps_keyboard() && !Report.leaves_the_list());
```

- [ ] **Step 2: Write the failing tests in `src/panel/actions.rs`**

Replace `a_task_not_yet_active_gets_start_and_no_stop` with:

```rust
    /// A task not yet started gets Start working and no Stop; a planned one
    /// gets Report too, after Grill me.
    #[test]
    fn a_task_not_yet_active_gets_start_and_no_stop() {
        assert_eq!(
            Action::row(on_list(false)),
            vec![Start, Refine, Grill, Edit, Speak, UpNext, Complete, Move, Wait, Remove]
        );
        assert_eq!(
            Action::row(on_list(true)),
            vec![Start, Refine, Grill, Report, Edit, Speak, UpNext, Complete, Move, Wait, Remove]
        );
    }
```

In `a_task_with_a_live_claude_gets_go_to_session_first`, the second assertion (the planned task) becomes:

```rust
        // A refine open on a task not yet started.
        assert_eq!(
            Action::row(with_claude(on_list(true))),
            vec![Session, Start, Refine, Grill, Report, Edit, Speak, UpNext, Complete, Move, Wait, Remove]
        );
```

After `every_card_on_the_list_gets_grill_me` add:

```rust
    /// Report sits between Grill me and Edit on every planned card on the
    /// list, active or not, and on no other card.
    #[test]
    fn every_planned_card_on_the_list_gets_report() {
        for state in every_state() {
            let row = Action::row(state);
            assert_eq!(row.contains(&Report), state.planned && !state.off_list(), "{state:?}");
            if row.contains(&Report) {
                let at = row.iter().position(|a| *a == Report).unwrap();
                assert_eq!((row[at - 1], row[at + 1]), (Grill, Edit), "{state:?}");
            }
        }
    }
```

In `letters_press_their_own_buttons`, the doc comment and expected list become:

```rust
    /// g b s r i p e c m t press Go to session, Back to list, Start working,
    /// Refine, Grill me, Report, Edit, Complete, Move to workspace and Stop,
    /// and each letter finds its own button.
```

```rust
            vec![
                ('g', Session), ('b', Back), ('s', Start), ('r', Refine), ('i', Grill), ('p', Report),
                ('e', Edit), ('c', Complete), ('m', Move), ('t', Stop)
            ]
```

In `a_lettered_button_hints_its_letter_and_name`, after the Grill line add:

```rust
        assert_eq!(hint(on_list(true), Some(Report)), "p: Report · Ctrl+Enter: Start working");
        assert_eq!(hint(with_claude(active(true)), Some(Report)), "p: Report · Ctrl+Enter: Go to session");
```

The existing `every_hint_fits_beside_its_row` and `confirm_remove_fits_on_the_widest_row` tests need no change: they derive the widest row (now 12 buttons) themselves. Note that the longest hint on a 12-button row, `m: Move to workspace · Ctrl+Enter: Start working`, is 48 characters in a room of exactly 48, so they pass with no margin; if either fails, the row has grown past the spec and the plan is wrong, not the test.

- [ ] **Step 3: Write the failing tests in `src/panel/keys.rs`**

In the two tests near lines 182 and 197 that map `r`/`i` and `R`/`I`, add after each Grill line:

```rust
        assert_eq!(key_action(gdk::Key::p, false, false), KeyAction::Run(Action::Report));
```

```rust
        assert_eq!(key_action(gdk::Key::P, false, false), KeyAction::Run(Action::Report));
```

- [ ] **Step 4: Run the tests to see them fail**

Run: `cargo test actions 2>&1 | tail -20`
Expected: a compile error, `no variant named Report` (the tests name a variant that does not exist yet).

- [ ] **Step 5: Add the variant and its facts in `src/actions.rs`**

In `enum Action`, after `Grill`:

```rust
    /// The refine report of a planned task's plan, built by Claude in the
    /// workspace's herdr session, opened in the browser and linked from the
    /// task's notes, without refining again.
    Report,
```

In `Action::facts`, after the `Grill` row:

```rust
            // Font Awesome's file-text: the report. Refine's mauve, being a
            // view of what a refine wrote.
            Report => Facts { icon: "\u{f15c}", class: "report", key: Some("p"), colour: c::REFINE, keeps_keyboard: false, leaves_the_list: false },
```

Update the glyph list in `facts`' doc comment: `terminal, undo arrow, play, magic wand, comments, file-text, pencil, …`, and the colour sentence: `Grill me and Report wear Refine's mauve, one being a refine that interviews first and the other a report of what a refine wrote`.

`ALL` becomes:

```rust
    /// Every task action, in the order the action row puts the ones it has.
    pub const ALL: [Action; 14] =
        [Session, Back, Start, Refine, Grill, Report, Edit, Speak, UpNext, Complete, Move, Stop, Wait, Remove];
```

In `applies`, add an arm before `Start | Refine | Grill | UpNext | Wait`, and add to the doc comment `Report is for a planned task on the list: an unplanned one has no plan to report on.`:

```rust
            // A plan to report on, and a task still on the list.
            Report => state.planned && !state.off_list(),
```

In `label`: `Report => "Report",` after `Grill`. In `args`: `Report => &["task", "report", uuid],` after `Grill`.

- [ ] **Step 6: Add `p` to the surface's key list**

In `src/panel/surface.rs` lines 41-43, the module doc sentence becomes:

```rust
//! Left, Right and Tab move along the focused one, and g, b, s, r, i, p, e,
//! c, m, t and Delete press its Go to session, Back to list, Start, Refine,
//! Grill me, Report, Edit, Complete, Move to workspace, Stop and Remove. Ctrl+Delete deletes the focused card's task at once and keeps the
```

- [ ] **Step 7: Run the tests to see them pass**

Run: `cargo test 2>&1 | tail -5`
Expected: every test passes (the `README`/`llms.txt` command tests check only CLI subcommands, which Task 1 does not add). The stylesheet test `every_action_button_has_its_colour` passes by itself: `style::css` loops over `Action::ALL`.

- [ ] **Step 8: Commit**

```bash
git add src/actions.rs src/panel/actions.rs src/panel/keys.rs src/panel/surface.rs
git commit -F- <<'EOF'
feat(panel): add Report to a planned card's action row

A planned task's card gets Report (p) between Grill me and Edit, in
Refine's mauve, running `niritasks task report <uuid>`. An unplanned
task has no plan to report on and keeps Refine and Grill me; a waiting
or finished task gets nothing new. Ctrl+Enter is unchanged.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 2: `niritasks task report` and `refine::Mode::Report`

**Files:**
- Modify: `src/refine.rs:14-45` (`Mode`, `prompt`, `tab_label`), `:129-160` (`session_settings`), `:260` (`launch`), and its tests
- Modify: `src/main.rs:165-172` (`TaskCommand`), `:358-368` (the handler)
- Modify: `README.md:270-272` (the Commands block), `llms.txt:9` and `:42-43` (rule 1 and the commands table)

**Interfaces:**
- Consumes: `Action::Report.args` from Task 1 (`task report <uuid>`), `task::Task::is_planned`, `refine::launch`.
- Produces: `refine::Mode::Report`; `prompt(uuid, Mode::Report) == "/report-task <uuid>"`; `tab_label(Mode::Report, d) == "Report: <elided d>"`; `session_settings(project, task_data, hidden, uuid, reports, mode: Mode)` whose JSON carries `pluginConfigs["niri-tasks-refine"].options.report: bool`, `true` only for `Mode::Report`. Task 3's mod reads that `report` option; Task 4's skill answers the `/report-task` prompt.

- [ ] **Step 1: Write the failing tests in `src/refine.rs`**

Change the three existing `session_settings` calls in tests to pass a mode: add `, Mode::Quick` as the last argument in `the_sandbox_fences_the_session_to_the_task_database`, `the_session_may_search_the_web_but_not_read_credentials` and `the_mod_is_told_which_task_it_may_write_and_where_reports_go`. In the last of those add, after the `reports` assertion:

```rust
        assert_eq!(options["report"], false, "a refine is not a report session");
```

Add after it:

```rust
    /// A Report session tells the mod so: it offers only the report tool,
    /// armed from the start, and links the report from the task itself.
    #[test]
    fn a_report_session_tells_the_mod_so() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), Mode::Report);
        let v: Value = serde_json::from_str(&json).unwrap();
        let options = &v["pluginConfigs"][REFINE_MOD]["options"];
        assert_eq!(options["report"], true);
        assert_eq!(options["uuid"], u, "the same fence and task as a refine");
        assert_eq!(options["reports"], "/r");
        for mode in [Mode::Quick, Mode::Grill] {
            let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u, Path::new("/r"), mode);
            let v: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["report"], false, "{mode:?}");
        }
    }
```

Extend the two prompt and label tests:

```rust
    #[test]
    fn the_prompt_invokes_the_skill_with_the_uuid() {
        assert_eq!(prompt("u-1", Mode::Quick), "/refine-task u-1");
        assert_eq!(prompt("u-1", Mode::Grill), "/refine-task u-1 grill");
        // Its own skill: it reports on the plan, never refines.
        assert_eq!(prompt("u-1", Mode::Report), "/report-task u-1");
    }

    #[test]
    fn the_tab_says_what_it_is_for() {
        assert_eq!(tab_label(Mode::Quick, "fix the peek"), "Refine: fix the peek");
        assert_eq!(tab_label(Mode::Grill, "fix the peek"), "Grill: fix the peek");
        assert_eq!(tab_label(Mode::Report, "fix the peek"), "Report: fix the peek");
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test refine:: 2>&1 | tail -20`
Expected: a compile error, `no variant or associated item named Report found for enum Mode`.

- [ ] **Step 3: Add `Mode::Report` to `src/refine.rs`**

Replace the `Mode` enum, `prompt` and `tab_label`:

```rust
/// Which of a card's buttons, Refine, Grill me or Report, opened Claude. They
/// differ in the prompt [`prompt`] sends, and so in the skill on the other
/// end: the two refines share `refine-task`, which reads the mode to decide
/// whether to interview before drafting, while a report runs `report-task`
/// on the plan the task already holds. A report session also tells the mod
/// so, through [`session_settings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Draft straight away; ask only what the code cannot answer.
    Quick,
    /// A full interview, via the `grilling` skill, before the draft.
    Grill,
    /// No refine: the report of a planned task's plan, opened in the browser
    /// and linked from the task.
    Report,
}

/// Only the uuid crosses to the skill — the description and everything else
/// about the task is read by the skill itself, so nothing here needs quoting
/// through two CLIs (this one, then herdr's `agent prompt`).
pub fn prompt(uuid: &str, mode: Mode) -> String {
    match mode {
        Mode::Quick => format!("/refine-task {uuid}"),
        Mode::Grill => format!("/refine-task {uuid} grill"),
        Mode::Report => format!("/report-task {uuid}"),
    }
}

/// The verb says which mode the tab is in; the description is cut short
/// because the label shares herdr's sidebar with every other tab's label.
pub fn tab_label(mode: Mode, description: &str) -> String {
    let verb = match mode {
        Mode::Quick => "Refine",
        Mode::Grill => "Grill",
        Mode::Report => "Report",
    };
    format!("{verb}: {}", names::elide(description))
}
```

Change `session_settings`'s signature and `pluginConfigs`, and add to its doc comment `It is told `report` too: true in a Report session, where it offers only the report tool and links the report from the task itself.`:

```rust
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str, reports: &Path, mode: Mode) -> String {
```

```rust
        "pluginConfigs": {
            (REFINE_MOD): { "options": { "uuid": uuid, "reports": reports, "report": (mode == Mode::Report) } },
        },
```

In `launch`, pass the mode and word the "already open" notice by it:

```rust
    let settings = session_settings(session.dir(), &task::data_location()?, &hidden, &t.uuid, &reports_dir(&dirs), mode);
```

```rust
    if launch_in(&session, &name, &label, ws.name(), &claude, &prompt(&t.uuid, mode))? == Launched::AlreadyRunning {
        // The same agent name for all three, so a Report on a task being
        // refined lands in the refine's tab, and the other way round.
        session.notify(match mode {
            Mode::Report => "Already open on this task — switched to its tab.",
            Mode::Quick | Mode::Grill => "Already being refined — switched to its tab.",
        });
    }
```

Update the module doc's first paragraph: `Handing a task to Claude in its workspace's herdr session, to be worked up into a plan by the `refine-task` skill, or, once it has one, reported on by `report-task`.`

- [ ] **Step 4: Add `TaskCommand::Report` to `src/main.rs`**

After the `Refine { … }` variant (line 172):

```rust
    /// Report on a planned task's plan with Claude, in a new tab of the workspace's herdr session
    ///
    /// Claude builds the HTML report of the plan the task already holds, as a
    /// refine's "Show me a report first" does, opens it in the browser and
    /// links it from the task's notes as `Report: <path>`, without refining
    /// again. Refused for a task that is not pending or not tagged planned:
    /// refine it first. The workspace is this herdr session's in a herdr
    /// pane, else the focused one.
    Report {
        /// The task's uuid, or its first 8 characters
        uuid: String,
    },
```

After the `TaskCommand::Refine { … } => { … }` arm (line 368):

```rust
        TaskCommand::Report { uuid } => {
            let workspace = Workspace::of_caller()?;
            let t = task::get(&uuid)?.context("task not found")?;
            anyhow::ensure!(t.status == "pending", "Only a pending task can be reported on.");
            // An unplanned task has no plan to report on: the card offers
            // Refine or Grill me instead, and a script gets the same refusal.
            anyhow::ensure!(t.is_planned(), "Only a planned task has a plan to report on. Refine it first.");
            refine::launch(&workspace, &t, refine::Mode::Report)?;
        }
```

- [ ] **Step 5: Document the command where the tests look**

In `README.md`'s Commands block, after the `niritasks task refine <uuid> --grill` line:

```
niritasks task report <uuid>       # a report of its plan, built by Claude in the workspace's herdr session,
                                   #   opened in the browser and linked from the task's notes (a planned task only)
```

In `llms.txt`'s commands table, after the `--grill` row:

```
| `niritasks task report <uuid>` | Open a tab in the workspace's herdr session with Claude building the HTML report of the task's plan (the `report-task` skill), opening it in the browser and adding a `Report: <path>` note to the task. Refused for a task not pending or not tagged `planned`: refine it first | herdr tab, follows the herdr session, else focus |
```

In `llms.txt` rule 1 (line 9), change `` `task add`, `task move`, `task refine`, `task start` and `task session` do not in a pane of a named herdr session `` to `` `task add`, `task move`, `task refine`, `task report`, `task start` and `task session` do not in a pane of a named herdr session ``, and `the other three open in its herdr session` to `the other four open in its herdr session`.

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test 2>&1 | tail -5`
Expected: all pass, including `readme_commands_block_runs_every_subcommand_and_flag`, `llms_txt_runs_every_subcommand_and_flag` and `every_subcommand_and_argument_has_help`.

Then: `cargo run -- task report 0db7f500 2>&1 | tail -2` from a terminal that is not a herdr pane of a named session, on this task (it is planned). Expected: either a "Report: feat: Report a planned task…" tab opens in the workspace's herdr session (the skill does not exist yet, so Claude will say so), or, outside a named workspace, the usual "name this workspace" refusal. Then on an unplanned pending task's uuid8: `Only a planned task has a plan to report on. Refine it first.` and exit 1. Close any tab it opened.

- [ ] **Step 7: Commit**

```bash
git add src/refine.rs src/main.rs README.md llms.txt
git commit -F- <<'EOF'
feat(task): add task report, a Report mode of the refine session

`niritasks task report <uuid>` opens the same fenced herdr session a
refine does, labelled Report, prompted with /report-task, and tells the
mod it is a report session through a `report` option. A task that is
not pending or not planned is refused: there is no plan to report on.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 3: The mod's report mode

**Files:**
- Modify: `claude/refine-mod/.claude-plugin/plugin.json`
- Modify: `claude/refine-mod/hooks/plan.ts:68` (export `notesOf`)
- Modify: `claude/refine-mod/hooks/report.ts` (append helpers)
- Modify: `claude/refine-mod/hooks/refine.ts` (`REPORT_SPEC` → `reportSpec`, `register`)
- Modify: `claude/refine-mod/tests/report.test.ts`, `claude/refine-mod/tests/refine.test.ts`

**Interfaces:**
- Consumes: `options.report` (boolean) from Task 2's `session_settings`; `options.uuid`, `options.reports` as today; `parseExport`, `taskDate`, `Task` from `./plan`; `parseReport`, `reportPath`, `openArgv` from `./report`; `reportPage` from `./page`.
- Produces, from `hooks/report.ts`: `unreportable(task: Task | undefined): string | undefined`, `withReportNote(task: Task, path: string, nowMs: number): Task`, `madeAlready(path: string): string`. From `hooks/plan.ts`: `notesOf(task: Task): string[]`. The tool `mcp__niri-tasks-refine__show_task_report` in report mode: no `write_task_plan` is registered, no question is asked, the task gains one `Report: <path>` note, one report a session.

- [ ] **Step 1: Write the failing pure tests in `tests/report.test.ts`**

Add to the imports: `madeAlready, unreportable, withReportNote` from `'../hooks/report'`, and `import type { Task } from '../hooks/plan'`. Append at the end of the file:

```ts
const planned = (over: Partial<Task> = {}): Task => ({
  uuid: UUID,
  status: 'pending',
  description: 'feat: Old words',
  annotations: [{ entry: '20260101T000001Z', description: 'first note' }],
  tags: ['planned', 'zeta'],
  ...over,
})

describe('report mode', () => {
  test('a pending, planned task can be reported on', () => {
    expect(unreportable(planned())).toBeUndefined()
  })

  test('a task that is gone, not pending or not planned cannot, and the reason says which', () => {
    expect(unreportable(undefined)).toBe('the task no longer exists')
    expect(unreportable(planned({ status: 'completed' }))).toBe('the task is completed, not pending')
    expect(unreportable(planned({ tags: ['zeta'] }))).toBe('the task has no plan: it is not tagged planned. Refine it first')
    expect(unreportable(planned({ tags: undefined }))).toBe('the task has no plan: it is not tagged planned. Refine it first')
  })

  test('the report note goes after the others, at now, with everything else kept', () => {
    const now = Date.UTC(2026, 9, 6, 1, 2, 3)
    expect(withReportNote(planned(), '/r/refine-0b8f6a52-20261006-010203.html', now)).toEqual({
      ...planned(),
      annotations: [
        { entry: '20260101T000001Z', description: 'first note' },
        { entry: '20261006T010203Z', description: 'Report: /r/refine-0b8f6a52-20261006-010203.html' },
      ],
    })
  })

  test('a task with no notes yet gets its first', () => {
    const task = withReportNote(planned({ annotations: undefined }), '/r/x.html', Date.UTC(2026, 9, 6, 1, 2, 3))
    expect(task.annotations).toEqual([{ entry: '20261006T010203Z', description: 'Report: /r/x.html' }])
  })

  test('the note never takes a second a note already has: Taskwarrior keys a note by it', () => {
    const now = Date.UTC(2026, 9, 6, 1, 2, 3)
    const task = planned({
      annotations: [
        { entry: '20261006T010203Z', description: 'at now' },
        { entry: '20261006T010204Z', description: 'a second later' },
      ],
    })
    expect(withReportNote(task, '/r/x.html', now).annotations?.at(-1)).toEqual({
      entry: '20261006T010205Z',
      description: 'Report: /r/x.html',
    })
  })

  test('a second report in one session is refused, naming the first', () => {
    expect(madeAlready('/r/x.html')).toBe(
      "show_task_report: this session's report is made and linked from the task: /r/x.html. " +
        'Press Report on the card again for another.',
    )
  })
})
```

- [ ] **Step 2: Write the failing session tests in `tests/refine.test.ts`**

After the `REPORT_PATH` constant add:

```ts
// A Report session: the card's Report button, not a refine.
const REPORT_MODE = { uuid: UUID, reports: '/r', report: true }
```

After the `TASK` constant add:

```ts
// TASK once a refine has written it: the plan a Report reports on.
const PLANNED = { ...TASK, tags: ['planned', 'zeta'] }
```

Append at the end of the file:

```ts
test('report mode: only the report tool, armed from the start', { options: REPORT_MODE }, async ($, on) => {
  const { registered } = world(on, SANDBOX, WRITE, [PLANNED])
  await $.session.start(START)
  expect(registered).toEqual([REPORT_TOOL])
})

test('report mode without a reports folder: no tool at all', { options: { uuid: UUID, report: true } }, async ($, on) => {
  const { registered } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('report mode: writes the page, opens it, and links it from the task, asking nothing', { options: REPORT_MODE }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })

  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH}, opened it in the browser, and linked it from the task ` +
      `as the note "Report: ${REPORT_PATH}".`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH])
  // The plan the page shows is the task's own description and notes.
  const page = writes[0]?.text ?? ''
  const block = page.slice(page.indexOf('id="decision"'))
  expect(block).toContain('<p><strong>Description:</strong> feat: Old words</p>')
  expect(block).toContain('<li>first note</li>')
  expect(seen).toEqual([`log: Report: ${REPORT_PATH}`])
  expect(runs.map(run => run.argv)).toEqual([
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    ['sh', '-c', 'xdg-open "$1" >/dev/null 2>&1 </dev/null &', 'sh', REPORT_PATH],
    ['task', 'rc.hooks=off', 'rc.verbose=nothing', 'import'],
  ])
  // One note added; the description, the other notes and the tags kept.
  expect(JSON.parse(runs[2]?.init?.stdin ?? 'null')).toEqual([
    {
      ...PLANNED,
      annotations: [
        { entry: '20260101T000001Z', description: 'first note' },
        { entry: '20261006T010203Z', description: `Report: ${REPORT_PATH}` },
      ],
    },
  ])
})

test('report mode: one report a session; the second names the first', { options: REPORT_MODE }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX, WRITE, [PLANNED])
  await $.session.start(START)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  const again = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(again.deny).toBe(
    `show_task_report: this session's report is made and linked from the task: ${REPORT_PATH}. ` +
      'Press Report on the card again for another.',
  )
  expect(writes).toHaveLength(1)
  expect(verbs(runs).filter(v => v === 'import')).toHaveLength(1)
})

test('report mode: an unplanned task is refused before anything is written', { options: REPORT_MODE }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX, WRITE, [TASK])
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe('show_task_report: nothing written: the task has no plan: it is not tagged planned. Refine it first.')
  expect(writes).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
})

test('report mode: a task no longer pending is refused', { options: REPORT_MODE }, async ($, on) => {
  const { writes } = world(on, SANDBOX, WRITE, [{ ...PLANNED, status: 'completed' }])
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe('show_task_report: nothing written: the task is completed, not pending.')
  expect(writes).toEqual([])
})

test('report mode: a report over its limits is refused with every reason, and nothing is linked', { options: REPORT_MODE }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX, WRITE, [PLANNED])
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, changes: [], checks: [] })
  expect(answer.deny).toBe(
    'show_task_report: changes must have 1 to 3 items, not 0; checks must have 1 to 6 items, not 0',
  )
  expect(writes).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
  // Fixed, it goes through: the refusal did not spend the session's one report.
  expect((await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })).result).toContain(`Wrote the report to ${REPORT_PATH}`)
})

test('report mode: a failed import is reported, with the page already open', { options: REPORT_MODE }, async ($, on) => {
  engine(on, SANDBOX)
  on('process.run', ($, e) =>
    e.argv.includes('export')
      ? ran(JSON.stringify([PLANNED]))
      : e.argv.includes('import')
        ? { value: { ...ran('', 2).value, stderr: 'Not a valid JSON value.' } }
        : ran(''),
  )
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe(
    `Wrote the report to ${REPORT_PATH} and opened it, but task import failed (2): Not a valid JSON value. ` +
      'The task does not link it.',
  )
})

test('report mode: sandbox off, no tool', { options: REPORT_MODE }, async ($, on) => {
  const { registered } = world(on, { sandbox: { enabled: true, failIfUnavailable: false } })
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('refine mode is unchanged by the option being false', { options: { ...REPORTS, report: false } }, async ($, on) => {
  const { registered, seen } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([TOOL, REPORT_TOOL])
  await $.tool.call(CALL)
  expect(seen.at(-1)).toBe(`ask: Write this to the task? [${WRITE} | ${REPORT} | Change something] (Task plan)`)
})
```

- [ ] **Step 3: Run the mod tests to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: the new `report.test.ts` tests fail on the missing exports (`unreportable is not a function` or a type error), and the report-mode session tests fail: both tools registered, and the report call denied with `the person has not asked for a report`.

- [ ] **Step 4: Export `notesOf` and add the helpers**

In `hooks/plan.ts`, line 68, change `const notesOf =` to:

```ts
// The task's notes as the plan holds them: each annotation's text, in order.
export const notesOf = (task: Task): string[] =>
  (task.annotations ?? []).map(annotation => annotation.description)
```

In `hooks/report.ts`, change the import to `import { HIDDEN, taskDate } from './plan'` and `import type { Plan, Task } from './plan'`, and append at the end:

```ts
// Report mode: the card's Report button on a planned task, with no refine
// before it. Why the task may not be reported on, or undefined when it is
// pending and has a plan.
export const unreportable = (task: Task | undefined): string | undefined => {
  if (task === undefined) return 'the task no longer exists'
  if (task.status !== 'pending') return `the task is ${task.status}, not pending`
  if (!(task.tags ?? []).includes('planned')) return 'the task has no plan: it is not tagged planned. Refine it first'
  return undefined
}

// The task with one more note, linking the report, after the others: at
// `nowMs`, or the first later second no note has yet, since Taskwarrior keys
// a note by its entry time and two at one second would collapse into one.
// Everything else about the task is kept.
export const withReportNote = (task: Task, path: string, nowMs: number): Task => {
  const notes = task.annotations ?? []
  const taken = new Set(notes.map(note => note.entry))
  let at = Math.floor(nowMs / 1000) * 1000
  while (taken.has(taskDate(at))) at += 1000
  return { ...task, annotations: [...notes, { entry: taskDate(at), description: `Report: ${path}` }] }
}

// What the model reads when it calls show_task_report a second time in a
// report session: one report per press of the button.
export const madeAlready = (path: string): string =>
  `show_task_report: this session's report is made and linked from the task: ${path}. ` +
  'Press Report on the card again for another.'
```

- [ ] **Step 5: Teach `register` the report mode in `hooks/refine.ts`**

Change the imports from `./plan` to include `notesOf`, and from `./report` to include `madeAlready, unreportable, withReportNote`.

Replace `const REPORT_SPEC = { name: 'show_task_report', description: …, inputSchema: { … } }` with a function that keeps the schema and words the description by mode:

```ts
// The report tool's schema is one; what the model is told differs by mode:
// in a refine the report follows the person's "Show me a report first", in a
// report session it is the whole job.
const reportSpec = (reportOnly: boolean) => ({
  name: 'show_task_report',
  description: reportOnly
    ? "Writes an HTML report of this task's plan, its own description and notes as they are now, in one fixed, " +
      'easy-to-read layout, opens it in the browser and links it from the task as a Report: <path> note. ' +
      "Fill the fields as the refine-task skill's report-catalogue.md says. Every text field is one line of " +
      'plain text; a report over the limits is refused with every reason at once. One report a session.'
    : 'Writes an HTML report of the plan in one fixed, easy-to-read layout and opens it in the browser, once the ' +
      'person has chosen "Show me a report first" in write_task_plan. Fill the fields as the refine-task skill\'s ' +
      'report-catalogue.md says. Every text field is one line of plain text; a report over the limits is refused ' +
      'with every reason at once.',
  inputSchema: {
    // … exactly the inputSchema REPORT_SPEC had, unchanged …
  },
})
```

(Move the whole existing `inputSchema` object into it verbatim.)

In `register`, after `const reports = reportsDir(options.reports)`, add:

```ts
  // Report mode: the card's Report button on a planned task, not a refine.
  // Only the report tool is offered, armed from the start on the task's own
  // description and notes, and the mod links the report from the task
  // itself: the button press was the ask, so no question is put. One a
  // session, as a refine makes one per ask. Module state, like `askedFor`.
  const reportOnly = options.report === true
  let linked: string | undefined
```

Change `session.start`:

```ts
  on('session.start', async ($, e, next) => {
    if ((await armedUuid($, options.uuid)) !== undefined) {
      if (!reportOnly) await $.tool.register(SPEC)
      if (reports !== undefined) await $.tool.register(reportSpec(reportOnly))
    }
    return next(e)
  })
```

At the top of the `write_task_plan` handler, after the `armedUuid` check:

```ts
    if (reportOnly) return { deny: 'write_task_plan is not offered in a report session: nothing here refines.' }
```

In the `show_task_report` handler, replace the `if (askedFor === undefined) return { deny: NOT_ASKED_FOR }` line with:

```ts
    if (reportOnly) {
      if (linked !== undefined) return { deny: madeAlready(linked) }
      // The plan is the task as it is now: read here, not at session start,
      // so an edit made since the button was pressed is what is reported.
      const exported = await $.process.run([...TASK, 'rc.json.array=on', uuid, 'export'])
      if (exported.exitCode !== 0) {
        return { deny: `task export failed (${exported.exitCode}): ${exported.stderr.trim()}` }
      }
      const task = parseExport(exported.stdout)
      const why = unreportable(task)
      if (why !== undefined || task === undefined) return { deny: `show_task_report: nothing written: ${why}.` }

      const report = parseReport(e)
      if (typeof report === 'string') return { deny: `show_task_report: ${report}` }

      const shown = { description: task.description, notes: notesOf(task) }
      const path = reportPath(reports, uuid, await $.clock.now())
      await $.fs.write(path, reportPage(report, shown, await readFonts($, fonts), await drawDiagrams($, report)))
      $.ui.log(`Report: ${path}`)
      await $.process.run(openArgv(path))

      // The session's one write to the task, and the mod's, not the model's:
      // the note's path is the one just written.
      const imported = await $.process.run([...TASK, 'rc.verbose=nothing', 'import'], {
        stdin: JSON.stringify([withReportNote(task, path, await $.clock.now())]),
      })
      if (imported.exitCode !== 0) {
        return {
          deny:
            `Wrote the report to ${path} and opened it, but task import failed (${imported.exitCode}): ` +
            `${imported.stderr.trim()} The task does not link it.`,
        }
      }
      linked = path
      return {
        result:
          `Wrote the report to ${path}, opened it in the browser, and linked it from the task ` +
          `as the note "Report: ${path}".`,
      }
    }
    if (askedFor === undefined) return { deny: NOT_ASKED_FOR }
```

The refine-mode code after that line stays exactly as it is.

- [ ] **Step 6: Declare the option in `plugin.json`**

In `claude/refine-mod/.claude-plugin/plugin.json`, change the `description` to:

```json
  "description": "Gives a refine session two tools: write_task_plan, which rewrites one Taskwarrior task's description and notes, and show_task_report, which writes and opens an HTML report of the plan when the person asks for one. With `report` set, a Report session gets show_task_report alone, on the task's own plan, and the mod links the report from the task.",
```

and add to `userConfig`, after `reports`:

```json
    "report": {
      "type": "boolean",
      "title": "Report only",
      "description": "The session is a Report on a planned task, not a refine: only show_task_report is offered, armed from the start on the task's own description and notes, and the mod adds a Report: <path> note to the task when the report is written. Unset, the session is a refine.",
      "required": false,
      "default": false
    }
```

- [ ] **Step 7: Run the mod tests and validation to see them pass**

Run: `cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -6`
Expected: validation passes; every test passes, the 136 that were there and the new ones. The existing refine-mode tests prove "Refine mode behaves exactly as now".

- [ ] **Step 8: Commit**

```bash
git add claude/refine-mod
git commit -F- <<'EOF'
feat(refine): give the mod a report mode for a planned task

With the `report` option set, the session gets show_task_report alone,
armed from the start on the task's own description and notes, and the
mod links the written report from the task by task import, with no
question: the card's Report press was the ask. One report a session. A
refine session, the option unset, is unchanged.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 4: The `report-task` skill and its link

**Files:**
- Create: `.claude/skills/report-task/SKILL.md`
- Modify: `install.sh:88-92` (the skill links), `README.md:162` (the install line's skill list), `llms.txt:59-65` (the Skills list)

**Interfaces:**
- Consumes: the `/report-task <uuid>` prompt from Task 2; the tool `mcp__niri-tasks-refine__show_task_report` in report mode from Task 3, whose answers are `Wrote the report to <path>, opened it in the browser, and linked it from the task as the note "Report: <path>".`, `show_task_report: nothing written: …`, `show_task_report: <every field over its limit>`, `show_task_report: this session's report is made and linked …`, `show_task_report is not armed in this session.`; `report-catalogue.md` in the `refine-task` skill's directory.
- Produces: the skill installed at `~/.claude/skills/report-task/SKILL.md`.

- [ ] **Step 1: Write the skill**

Create `.claude/skills/report-task/SKILL.md`:

````markdown
---
name: report-task
description: >
  Build the HTML report of a planned Taskwarrior task's plan — the description
  and notes a refine wrote — open it in the browser and link it from the task,
  without refining again. Invoked by niri-tasks' task card's Report button as
  `/report-task <uuid>`.
disable-model-invocation: true
argument-hint: <uuid>
---

# report-task: explain a planned task's plan as a report

Arguments: `$ARGUMENTS` — a task uuid.

The task already has a plan: a refine wrote its description and notes and
tagged it `+planned`. Your job is to explain that plan as the refine report
would have — what changes, what needs the reader's judgement, diagrams, the
files and the checks — so the person can read it before deciding what to do
with the task. You do not refine, and you do not do the task.

**You report on the plan; you never change it and never carry it out.** No
file edits, no config changes, no commands beyond reading. The task's one
change is the `Report: <path>` note the report tool adds itself when it
writes the report. There is no `write_task_plan` in this session, and you
must not change the task any other way: no `task modify`, no `task
annotate`, no `task import`.

Your Bash commands run in a sandbox that can read anything but write only to
the task database. A command that fails with `Read-only file system` or
`Operation not permitted` hit that fence on purpose: do not look for a way
around it, and do not ask the user to lift it.

## 1. Read the task

```bash
task rc.json.array=on <uuid> export
```

Stop and say so if it returns `[]`, if its `status` is not `pending`, or if
`planned` is not among its `tags`: an unplanned task has no plan to report
on, and its card's Refine or Grill me is the way to get one. Show the user
the description and every note before going further.

Then select the report tool with ToolSearch
(`select:mcp__niri-tasks-refine__show_task_report`). If it is not there,
stop and tell the user the refine mod did not load, so the report cannot be
made: they should run `install.sh` and check that `claude --version` is
2.1.287 or later.

## 2. Ground it

Read enough of the project to understand what the plan touches: `CONTEXT.md`,
`AGENTS.md`/`CLAUDE.md`, the docs and the code the notes name. Facts you can
look up are yours to find; never ask the user for one. When the plan turns on
something outside the project — a tool's options, a library's API — search
the web and read the primary sources. Read only — nothing here changes a
file. Do not interview the user: the plan is decided, and the report
explains it.

## 3. Fill the report

Read `report-catalogue.md` in the `refine-task` skill's directory, beside
this skill's (`~/.claude/skills/refine-task/report-catalogue.md` when
installed): who the report is for, its fields and their limits, the lenses to
look through, how to draw diagrams, and a worked example. Then fill the
fields from the task's plan and the code you read in step 2:

- `title` is the task's description, as read in step 1.
- Go through every lens the catalogue lists and give a part for each one
  that applies: before/after, structure, data flow, outside tools and how
  hard each is to swap, styling, code before/after, and whether a newcomer
  could follow it. Read the code each lens needs: count the files that name
  a tool before you rate its swap.
- Real paths go in `files`; everywhere else, plain words the user would use.
- The tool shows the plan itself — the task's description and notes, read
  when you call — on the report's last slide. Do not repeat them in the
  fields.

## 4. Show it

Call `mcp__niri-tasks-refine__show_task_report` with the fields. The tool
builds the page in its fixed layout, saves it under the reviews folder,
opens it in the browser, and adds a `Report: <path>` note to the task. It
asks the user nothing: pressing Report was the ask. Its answer says what
happened:

- **Wrote the report to … and linked it from the task** — go to step 5.
- **show_task_report: nothing written: the task has no plan**, **… is
  completed, not pending** or **… no longer exists** — stop and tell the
  user; nothing was written.
- **show_task_report: this session's report is made and linked** — the
  report is done already; do not make another. Go to step 5.
- Any other **show_task_report: …** answer lists every field over its limit
  or not allowed: fix them all and call again.
- **Wrote the report to … but task import failed** — the page is open but
  the task does not link it: tell the user the path and the error, and stop.
- **not armed**, **task export failed**, or **show_task_report failed** —
  do not retry and do not write anything any other way: tell the user the
  tool's answer and stop.

## 5. Verify and report

Export the task once more and check: the description and the earlier notes
are as you read them in step 1, the last note is `Report: <path>` with the
path the tool answered, and the tags are unchanged, `planned` still among
them. Tell the user the path in one line, and stop. Leave the session open.

Always address the task by uuid, never its numeric id: ids are renumbered as
tasks complete.
````

- [ ] **Step 2: Link it from `install.sh` and list it in the docs**

In `install.sh`, after the two `refine-task` links (line 92), add:

```bash
# report-task: what a planned card's Report button opens Claude with. It
# reads refine-task's report catalogue, linked above, beside it.
link "$REPO/.claude/skills/report-task/SKILL.md" "$CLAUDE_SKILLS/report-task/SKILL.md"
```

In `README.md` line 162, the skill list becomes `` (`workspace-tasks`, `refine-task`, `report-task`, `finish-worktree`, `reject-worktree`, `private-github-repo`) ``.

In `llms.txt`'s `## Skills` list, after the `refine-task` line:

```
- [report-task](.claude/skills/report-task/SKILL.md): build the HTML report of a planned task's plan, open it and link it from the task; what `niritasks task report` starts
```

- [ ] **Step 3: Install and try it on this task**

Run: `bash install.sh 2>&1 | grep -i "report-task\|Done"` and then `ls -l ~/.claude/skills/report-task/SKILL.md`.
Expected: `[+] Linked …/skills/report-task/SKILL.md` on the first run, and the symlink points into this worktree's `.claude/skills/report-task/SKILL.md`. (`install.sh` also rebuilds and restarts the daemon on this branch's binary, which the end-to-end check in Task 5 needs.)

Then, in the niri-tasks workspace's panel (`Mod+Alt+Ctrl+T`), focus this task's card and press `p`. Expected: a herdr tab labelled `Report: feat: Report a planned task from its action…` opens, Claude reads the task, reads the catalogue, calls the tool, and the browser opens a report whose last slide shows this task's description and notes; then `task rc.json.array=on 0db7f500 export | jq '.[0].annotations[-1].description'` prints `Report: /home/paul/.local/share/niri-tasks/reviews/refine-0db7f500-<stamp>.html`. Pressing `p` again on the card while that tab is open notifies `Already open on this task — switched to its tab.`

If the report is wrong in a way the skill's words caused (it refined, it asked a question, it skipped the catalogue), fix the skill and press again; one `Report:` note per press is expected, so leave the notes as they are.

- [ ] **Step 4: Run the cargo tests**

Run: `cargo test 2>&1 | tail -3`
Expected: all pass (`llms_txt_has_the_llmstxt_shape` still sees only H2s).

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/report-task/SKILL.md install.sh README.md llms.txt
git commit -F- <<'EOF'
feat(refine): add the report-task skill a Report tab runs

The skill reads a planned task, grounds it as refine-task does, fills
the shared report catalogue's fields and calls show_task_report, which
writes, opens and links the report. It never refines and never writes
the task itself. install.sh links it beside refine-task.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

### Task 5: Docs, the ADR addendum, and the full test run

**Files:**
- Modify: `README.md:21` (the keybind paragraph), `:50-54` (the button table), `:180` (Requirements)
- Modify: `CONTEXT.md:55-60` (Refine report), `:97-105` (Task action), `:107-122` (Action row)
- Modify: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md:3-20` (the dated addenda)

**Interfaces:**
- Consumes: everything Tasks 1–4 built.
- Produces: docs that say what the button, command, mode and skill do, and a green `tests/all.sh`.

- [ ] **Step 1: README**

Line 21, in the `Mod+Alt+Ctrl+T` row, change `refine it into a plan with Claude (or be grilled about it first), edit it,` to `refine it into a plan with Claude (or be grilled about it first), see a report of that plan once it has one, edit it,`.

In the button table, after the **Grill me** row:

```
| **Report** (mauve) | `p` | Claude builds the HTML report of the task's plan — what changes, what needs your eye, diagrams, files and checks — opens it in the browser and adds a `Report: <path>` note to the task, without refining again — only on a planned task on the list |
```

Line 180, change `A card's **Refine** and **Grill me** need both herdr and Claude Code` to `A card's **Refine**, **Grill me** and **Report** need both herdr and Claude Code`.

- [ ] **Step 2: CONTEXT.md**

**Refine report** becomes:

```markdown
**Refine report**:
HTML slides explaining a task's plan — what changes, what needs the
reader's judgement, diagrams, the files and the checks — made by a refine
session when the user chooses Show me a report first, or by a Report session
from a planned task card's Report, on the plan the task already holds. It is
saved under the reviews folder and linked from the task's notes as `Report:
<path>`.
_Avoid_: plan report, explainer
```

**Task action**: change `Start working, Refine, Grill me, Edit, Speak, Up next, Complete, Move to workspace, Stop, Waiting or Remove` to `Start working, Refine, Grill me, Report, Edit, Speak, Up next, Complete, Move to workspace, Stop, Waiting or Remove`, and after `Which ones a task gets goes by its state:` insert `only a planned task on the list gets Report,`.

**Action row**: change `Refine, Grill me, Edit, Speak, Up next, Complete, Move to workspace, Stop, Waiting and Remove, an active task getting Stop in place of Start working,` to `Refine, Grill me, Report, Edit, Speak, Up next, Complete, Move to workspace, Stop, Waiting and Remove, an active task getting Stop in place of Start working, only a planned task getting Report,`.

- [ ] **Step 3: The ADR addendum**

In `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, after the `*2026-10-09:*` paragraph about the write tool's own notes (ending `is still what is written.`), add:

```markdown
*2026-10-10:* a planned task's card gets Report, which opens the same
fenced session (`refine.rs`'s `Mode::Report`, the same settings, mod and
agent name as a refine) with the mod's `report` option set. In that mode
the mod registers `show_task_report` alone, armed from the start on the
task's own description and notes, and after writing and opening the report
it adds the `Report: <path>` note to the task itself by `task import`, with
no `$.ui.ask`: the button press was the ask, and the note's path is the
one the mod just wrote, not one the model printed. `write_task_plan` is
not offered there, so a Report session has no way to change the plan. The
`report-task` skill drives it; `refine-task` and its "Show me a report
first" are unchanged. The decision is unchanged.
```

- [ ] **Step 4: Run every suite**

Run: `cargo test 2>&1 | tail -3 && (cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -3) && bash tests/all.sh 2>&1 | tail -12`
Expected: `cargo test` all pass; the mod validates and its tests all pass; `tests/all.sh` ends with `N passed   0 failed   M skipped`, every skip naming a missing prerequisite, none a failure. The panel suite, if it runs, parks a nested niri on the last workspace: keep off it.

- [ ] **Step 5: Check "done when" by hand, once**

With the daemon on this branch's binary (Task 4's `install.sh`): open the panel with `Mod+Alt+Ctrl+T` and confirm a planned card shows `Report` between `Grill me` and `Edit`, its hint reads `p: Report · Ctrl+Enter: Start working`, and an unplanned, waiting and finished card show no Report. Then on a planned task that has a refine tab open, press `p` and confirm it switches to that tab. Finally, from a Refine tab on any pending task, choose **Show me a report first** at the write tool's question and confirm the report still opens and the plan is still written with its `Report:` note afterwards.

- [ ] **Step 6: Commit**

```bash
git add README.md CONTEXT.md docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -F- <<'EOF'
docs: describe Report on a planned card and its session

The button table, the keybind row and the requirements name Report;
CONTEXT.md's task action, action row and refine report entries take it
in; ADR 0002 records that a Report session is the refine fence with the
mod in report mode, linking the report from the task itself.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
EOF
```

---

## Self-review

- **Spec coverage.** Report's who/facts/order (Task 1); `task report`, `Mode::Report`, prompt, label, same fence and agent (Task 2); the mod's `report` option, report-only registration, armed from the start, the note by import with no question, refine unchanged (Task 3); the skill and its link (Task 4); README button table and commands block (Tasks 5 and 2), llms.txt commands table (Task 2), CONTEXT.md's three entries and the ADR addendum (Task 5). "Done when" is checked in Task 5 step 5 and the three test runs in step 4. Out of scope is left out: no reopening, no slide changes, no catalogue changes.
- **Placeholders.** The one ellipsis in the plan, inside `reportSpec`'s `inputSchema`, points at the existing object to move verbatim, which the implementer has in front of them in the same file. Every other step shows its code.
- **Type consistency.** `session_settings(…, reports: &Path, mode: Mode)` is used that way in `launch` and every test; `Mode::Report` is the name in `refine.rs`, `main.rs` and the tests; `withReportNote(task, path, nowMs)`, `unreportable(task)` and `madeAlready(path)` have the same names and shapes in `report.ts`, `refine.ts` and both test files; the tool's result and refusal strings in `refine.ts` match the ones the session tests and the skill expect character for character.
