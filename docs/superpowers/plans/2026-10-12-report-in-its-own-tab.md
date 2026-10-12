# Report in Its Own Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate report always starts a Report session, even when a refine tab from earlier is still open on the task.

**Architecture:** Report gets its own herdr agent name, `report-<uuid8>`, spelled in `src/names.rs` beside `task-` and `work-`. `refine::launch` picks the name by mode, so `launch_in`'s `focus_agent` only finds an earlier Report tab, never the refine's. The mode-dependent steps of `launch` (name, tab label, pane name, prompt, the Already open notice) move into a small `open_in` that takes the `Session`, so the fake session can check them end to end.

**Tech Stack:** Rust, herdr (through `src/session`'s `Port` and its `Fake`).

**Spec:** Taskwarrior task `8ef3ab64-fbfc-4381-84af-e280eb6eca5c` (its description and notes; no separate spec file).

## Global Constraints

- Report's agent name is exactly `report-<uuid8>` (`names::uuid8`), e.g. `report-7cd9fd3a`. herdr names must match `[a-z][a-z0-9_-]{0,31}`.
- Refine and Grill keep `task-<uuid8>`; Start working keeps `work-<uuid8>`.
- The old refine tab stays open and is not touched. A second Report press while a report tab is open goes to that tab.
- Out of scope: a Go to report session row in the Go to list (`link::Which`, `actions::TaskState::has_refine` stay as they are), and closing or reusing a refine tab that has finished.
- Commits: Conventional Commits, `fix(refine): …` / `fix(names): …`, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Background for the implementer

- `src/names.rs` spells every task agent name. `refine_agent(uuid)` → `task-<uuid8>`, `work_agent(uuid)` → `work-<uuid8>`. `is_task_agent(name)` says whether a name was given for a task; `link::pane_link_name` uses it so a Claude marking its task active is not renamed to `work-` when it already has a task name. A report session must count too, or a report Claude that marks its task active would lose its name.
- `src/refine.rs` `launch(ws, t, mode)` builds the sandbox settings, then calls `launch_in(session, name, label, workspace_label, claude, pane, text)`, which takes the session claim, calls `session.focus_agent(name)` and returns `Launched::AlreadyRunning` if that agent is live, else opens a tab, starts Claude under `name` and prompts it. Today `name` is always `names::refine_agent`, so a Report press with a refine tab open focuses the refine tab and sends no prompt — the bug.
- `src/session/fake.rs`'s `Fake` logs each herdr call as a line: `agent_start <session> <name> <pane> <kind>`, `agent_focus <session> <name>`, `agent_prompt <session> <name> <text>`, `notify <text>`. `fake.agents()` lists the live agents. The tests at the bottom of `src/refine.rs` (`refine`, `at`, `refiner`, `tk`) show the pattern.
- Run tests with `cargo test`. Baseline: all pass.

---

### Task 1: Name Report's agent `report-<uuid8>`

**Files:**
- Modify: `src/names.rs` (module doc lines 1-7, `refine_agent` doc at 17-21, `is_task_agent` at 36-40, tests at 117-135)
- Modify: `src/link.rs` (module doc lines 1-7, `pane_link_name` doc at 17-20)

**Interfaces:**
- Produces: `pub fn names::report_agent(uuid: &str) -> String` returning `report-<uuid8>`; `names::is_task_agent` now true for `report-` names too.

- [ ] **Step 1: Write the failing tests**

In `src/names.rs`'s tests, extend `the_agents_are_named_after_their_task` and `is_task_agent_knows_both_prefixes` (rename it, since it now knows three):

```rust
    /// herdr names must match `[a-z][a-z0-9_-]{0,31}`; a uuid's first eight
    /// characters are hex, and enough to tell one task's session from another.
    #[test]
    fn the_agents_are_named_after_their_task() {
        assert_eq!(refine_agent("00DEEEE1-3cbd-465d-8c85-c4c4d643b1d0"), "task-00deeee1");
        assert_eq!(work_agent("7CD9FD3A-d27b-4387-8249-aaf0d6785f90"), "work-7cd9fd3a");
        assert_eq!(report_agent("7CD9FD3A-d27b-4387-8249-aaf0d6785f90"), "report-7cd9fd3a");
    }
```

```rust
    #[test]
    fn is_task_agent_knows_every_prefix() {
        assert!(is_task_agent("task-x"));
        assert!(is_task_agent("work-x"));
        assert!(is_task_agent("report-x"));
        assert!(!is_task_agent("reviewer"));
        assert!(!is_task_agent(""));
    }
```

In `src/link.rs`'s tests (~line 166), extend and rename the existing test, so a report Claude marking its task active keeps its name, as a refine does:

```rust
    #[test]
    fn a_pane_named_by_refine_report_or_start_working_is_left_alone() {
        assert_eq!(pane_link_name(Some("task-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("report-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("work-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("work-7cd9fd3a"), "7cd9fd3a-d27b"), None, "already linked");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test names:: link::`
Expected: compile error, `cannot find function report_agent`.

- [ ] **Step 3: Implement**

In `src/names.rs`, after `work_agent`:

```rust
/// The herdr agent name for a task's report session. Its own `report-`, not
/// Refine's `task-`, so Generate report never lands in a refine tab still
/// open on the task — that tab was started without the report option, and
/// may be mid-interview — while a second press finds the first report.
pub fn report_agent(uuid: &str) -> String {
    format!("report-{}", uuid8(uuid))
}
```

Replace `is_task_agent`:

```rust
/// Whether `name` is a `task-`, `work-` or `report-` agent: one named by
/// Refine, Start working or Generate report, or linked already, and so named
/// for a task.
pub fn is_task_agent(name: &str) -> bool {
    ["task-", "work-", "report-"].iter().any(|p| name.starts_with(p))
}
```

Update the docs that list the names:
- `src/names.rs` module doc, lines 3-4: `Refine names its agent \`task-<uuid8>\`, Generate report its \`report-<uuid8>\` and Start working its \`work-<uuid8>\`, so nothing is stored on the task …` and "so the pane link, Refine, Report, Start working and the panel's lookup cannot drift apart."
- `refine_agent`'s doc: "The herdr agent name for a task's refine session, Refine's and Grill me's. One name per task is what lets a second Refine find the first instead of opening another."
- `src/link.rs` module doc lines 2-3: `Refine names its agent \`task-<uuid8>\`, Generate report its \`report-<uuid8>\` and Start working its \`work-<uuid8>\`, …`
- `pane_link_name`'s doc: "A `task-`, `work-` or `report-` agent is left alone: it was named by Refine, Generate report or Start working, or linked already, …"

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/names.rs src/link.rs
git commit -m "fix(names): give a report session its own report- agent name

Generate report shared Refine's task-<uuid8> name, so a refine tab
still open on the task was taken for the report's.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Open Report under its own name

**Files:**
- Modify: `src/refine.rs` (`launch` at ~286-323; new `agent_name`, `already_open` and `open_in` after it; tests at the end of the file)
- Modify: `README.md:365-367` (`task report` line)

**Interfaces:**
- Consumes: `names::report_agent(uuid: &str) -> String` from Task 1; `names::refine_agent`.
- Produces (private to `refine.rs`): `fn agent_name(mode: Mode, uuid: &str) -> String`, `fn already_open(mode: Mode) -> &'static str`, `fn open_in(session: &Session, t: &task::Task, workspace_label: &str, claude: &Claude, mode: Mode) -> Result<()>`.

- [ ] **Step 1: Write the failing tests**

Add to the end of `src/refine.rs`'s `tests` module (it already has `Fake`, `refiner`, `at` and `tk`; `tk()`'s uuid is `7cd9fd3a-d27b-4387-8249-aaf0d6785f90`):

```rust
    /// `launch`'s session steps for `mode` on [`tk`] over `fake`.
    fn open(fake: &Fake, mode: Mode) -> Result<()> {
        let session = Session::with_port("alpha", PathBuf::from("/p/alpha"), Box::new(fake.clone()));
        open_in(&session, &tk(), "alpha", &refiner(), mode)
    }

    #[test]
    fn each_mode_has_its_agent() {
        assert_eq!(agent_name(Mode::Quick, "7CD9FD3A-d27b"), "task-7cd9fd3a");
        assert_eq!(agent_name(Mode::Grill, "7CD9FD3A-d27b"), "task-7cd9fd3a");
        assert_eq!(agent_name(Mode::Report, "7CD9FD3A-d27b"), "report-7cd9fd3a");
    }

    /// Generate report with a refine tab still open on the task: the refine
    /// tab was started without the report option and may be mid-interview,
    /// so the report starts a Claude of its own, prompted, and the refine is
    /// left alone.
    #[test]
    fn a_report_opens_its_own_tab_beside_a_refine() {
        let fake = Fake::running(&[("w1", "alpha")]);
        open(&fake, Mode::Quick).unwrap();
        open(&fake, Mode::Report).unwrap();
        let log = fake.log();
        assert!(!log.iter().any(|l| l.starts_with("agent_focus alpha task-7cd9fd3a")), "{log:?}");
        assert!(log.iter().any(|l| l.starts_with("agent_start alpha report-7cd9fd3a ")), "{log:?}");
        assert!(
            log.iter().any(|l| l == "agent_prompt alpha report-7cd9fd3a /report-task 7cd9fd3a-d27b-4387-8249-aaf0d6785f90"),
            "{log:?}"
        );
        assert!(!log.iter().any(|l| l.starts_with("notify ")), "{log:?}");
        assert_eq!(fake.agents().len(), 2);
    }

    /// A second press of each goes back to its own tab, and says so in its
    /// own words; nothing new starts.
    #[test]
    fn a_second_press_goes_back_to_its_own_tab() {
        let fake = Fake::running(&[("w1", "alpha")]);
        open(&fake, Mode::Quick).unwrap();
        open(&fake, Mode::Report).unwrap();
        let before = fake.log().len();
        open(&fake, Mode::Report).unwrap();
        open(&fake, Mode::Grill).unwrap();
        let log: Vec<String> = fake.log().into_iter().skip(before).collect();
        assert!(!log.iter().any(|l| l.starts_with("agent_start ")), "{log:?}");
        assert!(at(&log, "agent_focus alpha report-7cd9fd3a") < at(&log, "agent_focus alpha task-7cd9fd3a"), "{log:?}");
        assert!(log.iter().any(|l| *l == format!("notify {}", already_open(Mode::Report))), "{log:?}");
        assert!(log.iter().any(|l| *l == format!("notify {}", already_open(Mode::Grill))), "{log:?}");
        assert_ne!(already_open(Mode::Report), already_open(Mode::Quick));
        assert_eq!(fake.agents().len(), 2);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test refine::tests`
Expected: compile error, `cannot find function open_in` / `agent_name` / `already_open`.

- [ ] **Step 3: Implement**

In `src/refine.rs`'s `launch`, delete `let name = names::refine_agent(&t.uuid);` and replace its tail (from `let claude = Claude::Refiner { settings, mod_dir };` to the end of the function) with:

```rust
    let claude = Claude::Refiner { settings, mod_dir };
    open_in(&session, t, ws.name(), &claude, mode)
}

/// The herdr agent `mode`'s session runs under: Refine and Grill me share
/// the task's refine agent, so either press lands in a refine already open;
/// a report has its own, since a refine tab cannot run one.
fn agent_name(mode: Mode, uuid: &str) -> String {
    match mode {
        Mode::Quick | Mode::Grill => names::refine_agent(uuid),
        Mode::Report => names::report_agent(uuid),
    }
}

/// What a press says when `mode`'s session was open on the task already.
fn already_open(mode: Mode) -> &'static str {
    match mode {
        Mode::Quick | Mode::Grill => "Already refining this task — switched to its tab.",
        Mode::Report => "Already reporting on this task — switched to its tab.",
    }
}

/// [`launch`]'s steps in `session` once `claude` is built: the agent, tab,
/// pane name and prompt for `mode` on `t`, handed to [`launch_in`], and a
/// notice when its session was open already. Kept apart from what reads the
/// machine so the fake can check that each mode lands in its own tab.
fn open_in(session: &Session, t: &task::Task, workspace_label: &str, claude: &Claude, mode: Mode) -> Result<()> {
    let name = agent_name(mode, &t.uuid);
    let label = tab_label(mode, &t.description);
    let pane = names::PaneName::for_task(names::Role::Refine, t.id, &t.uuid, &t.description);
    if launch_in(session, &name, &label, workspace_label, claude, &pane, &prompt(&t.uuid, mode))? == Launched::AlreadyRunning {
        session.notify(already_open(mode));
    }
    Ok(())
}
```

Also update `launch`'s doc comment: "goes back to the task's existing tab if it is already being refined" → "goes back to the task's existing tab for `mode` if there is one: a refine for Refine and Grill me, a report for Generate report."

In `README.md`, line 366, change the `task report` comment's second line to say the report gets its own tab:

```
niritasks task report <uuid>       # open the report of its plan linked from its notes; without one, a report
                                   #   built by Claude in a Report: tab of its own in the workspace's herdr session
                                   #   (beside any refine tab), opened in the browser and linked from the task's
                                   #   notes (a planned task only)
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: all pass, including the existing `refine_*` launch_in tests unchanged.

Also run: `cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | head` — expected: nothing new.

- [ ] **Step 5: Commit**

```bash
git add src/refine.rs README.md
git commit -m "fix(refine): open Generate report in its own tab, not the refine's

A refine tab still open on the task took the report's press and sent
it no prompt; that tab was started without the report option and may
be mid-interview. Report now runs under report-<uuid8>, so a second
press goes back to the report tab and Refine to the refine tab.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 6: Check it in the real app (done-when)**

After landing (`finish-worktree`), with herdr running: on a planned task, press Refine and leave the tab open; then press Generate report. Expected: a new `Report: …` tab opens and runs `/report-task <uuid>`; the refine tab is untouched. Press Generate report again: back to that Report tab, notice "Already reporting on this task — switched to its tab." Press Refine: back to the refine tab.
