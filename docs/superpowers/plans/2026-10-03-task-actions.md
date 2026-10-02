# One Task-Action List Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put every task action (Go to session, Back to list, Start working, Refine, Grill me, Edit, Note, Speak, Up next, Stop, Waiting, Remove) in one library module, `src/actions.rs`. It says which actions a task gets in each state, with each one's label, icon and `niritasks` command words. The action menu and the action row become two views of that list, and the menu stops offering Start working, Refine, Grill me and Up next on a waiting task.

**Architecture:** `src/actions.rs` holds the `Action` enum, a `TaskState` struct (active, waiting, planned, up next, live Claude) and the catalogue methods `Action::applies(state)`, `label(up_next)`, `icon()` and `args(uuid)`. `src/panel/actions.rs` becomes the action row's view, as an `impl Action` block with `ROW`, `row(state)`, `advance(state)`, `keeps_keyboard`, `leaves_the_list`, `class` and `letter`. A new `src/menu.rs` is the action menu's view: `Entry::{Run(Action), Status, Move}`, `entries(state)` and `picked(label, state)`. `main.rs`'s `task_menu` runs a picked action by passing its own command words through the CLI's clap parser, so the menu runs exactly what the row's button runs. There is no match from label string to command any more.

**Tech Stack:** Rust 2021, clap 4 (derive), gtk4-rs 0.11 (`gdk::Key` in `keys.rs` only), Taskwarrior 2.6.2.

**Spec:** Taskwarrior task `fb3da470-7449-4e75-839c-420ca0487f55`. Read it with `task rc.json.array=on fb3da470-7449-4e75-839c-420ca0487f55 export`; its description and notes are the spec. Its report link is the 2026-10-02 architecture review, candidate 2 (`~/.local/share/niri-tasks/reviews/architecture-review-2026-10-02.html#c2`). The task it follows, 637796b7 (PanelState), is done.

## Global Constraints

- Replace, don't layer. `panel/actions.rs`'s own `Action` enum, `for_status` and `label_on`, `task::up_next_label`, `style::ACTION_COLOURS`, `main.rs`'s `GO_TO_SESSION` and `menu_entries`, and the label-string `match` in `task_menu` all go. Their tests move to the catalogue or a view; none stays alongside its replacement.
- No label is spelled twice. Every task action's words live in `Action::label`. Only the menu's own two entries, "Update status" and "Move to workspace", are spelled in `menu.rs`.
- Behaviour stays the same except for one deliberate change. On a **waiting** task, the action menu offers only Edit, Note, Speak, Update status and Move to workspace: no Go to session, Up next, Refine, Grill me or Start working. The action row's buttons, order, tooltips, colours and keys stay exactly as they are.
- `refine.rs` must not depend on `crate::panel`: `refine::quick_command` builds from `crate::actions::Action::Refine.args`.
- Out of scope: Clear all (it stays in `panel/actions.rs`), the IPC protocol, and moving `task_command`'s rules out of `main.rs` (candidate 3 of the review). That includes the `t.status == "pending"` checks in `TaskCommand::Start` and `TaskCommand::Refine`.
- Unit tests need no display (`cargo test` runs over SSH). Only `keys.rs`'s tests touch `gdk::Key`, and they already do, without GTK init.
- Keep green: `every_subcommand_and_argument_has_help`, `readme_commands_block_runs_every_subcommand_and_flag`, `llms_txt_runs_every_subcommand_and_flag` (no CLI changes are planned, so they should stay green untouched), and `tests/e2e-panel.sh`.
- House style: doc comments say *why*, in the plain voice of the surrounding code. Commits use Conventional Commits, imperative, lowercase, subject ≤72 characters, and end with a blank line and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Run every e2e check against this worktree's build: `cargo build && NIRITASKS=./target/debug/niritasks bash tests/e2e-panel.sh`. While it runs, keep off the workspace the nested niri is parked on.

## Design decisions

The spec's proposed shape was marked "to grill before building". These are the choices this plan makes, and where it differs from or sharpens the sketch:

1. **One enum, twelve actions, all with a command.** `Action` covers every task action that is one `niritasks` command: the row's ten plus Note and Grill me. The menu's Update status and Move to workspace are *not* actions, because each asks a second question in fuzzel (which state, which project) before it does anything. They are the menu's own `Entry::Status` and `Entry::Move`. So `args(uuid)` returns a plain `Vec<String>`, with no `Option`.
2. **Every action has an icon, Note and Grill me too.** Note gets Font Awesome's sticky note (`\u{f249}`) and Grill me its comments (`\u{f086}`), both from the Nerd Font the row already uses. Putting either on the row later then needs no new icon. In `style.rs`, Note wears Edit's yellow and Grill me Refine's mauve, so the colour match is exhaustive and needs no fallback.
3. **The rules for when an action applies live in the catalogue. The row's narrowness lives in the row.** `Action::applies(state)`: Go to session needs a live Claude and a task that is not waiting. Back to list only on a waiting task. Stop only on an active task. Start working, Refine, Grill me, Up next and Waiting only on a task that is not waiting. Edit, Note, Speak and Remove always. The menu keeps Start working on an active task, which reopens its worktree and Claude ("run again, back to both"). The row drops it there, because Stop takes its place. That is `Action::row`'s one extra rule.
4. **`TaskState` is plain data with public fields,** made by `TaskState::of(&Task, waiting, has_session)` in the CLI and by `Card::state(has_session)` in the panel. `Card::state` returns `None` on "+N more", which stands for no one task, so `row` needs no `More` case.
5. **The menu dispatches through clap.** A picked `Entry::Run(action)` becomes `Cli::try_parse_from(["niritasks", action.args(uuid)...])` and goes through the same `dispatch` the shell's command line does. The menu therefore runs the same words as the row's button. The test `every_task_action_is_a_command_the_cli_accepts` covers both views (it is the spec's widened `every_card_button_is_a_command_the_cli_accepts`). Adding an action needs no new match arm.
6. **The menu learns the task is waiting from Taskwarrior's `+WAITING`.** Taskwarrior 2.6 exports a waiting task with `"status":"pending"` and a `wait` date. A wait date that has passed stays on the task, which was checked against 2.6.2 in a scratch database. So `task::get` can't tell, and `wait.is_some()` would be wrong. The new `task::is_waiting(uuid)` asks `task <uuid> +WAITING export`. Only `TaskCommand::Menu` calls it. The fuzzel picker lists `pending_for_tag`, which leaves waiting tasks out, so it passes `false`.
7. **A spec correction to keep in mind:** the spec says `task_command` refuses Start working and Refine on a waiting task. It does not. Its `t.status == "pending"` check passes a waiting task, for the reason in decision 6. So the menu fix here is what keeps Start working and Refine off a waiting task. Tightening the CLI belongs with candidate 3, and is out of scope.
8. **The row view adds to `Action` with an `impl Action` block in `panel/actions.rs`.** Rust allows inherent impls anywhere in the defining crate. Call sites keep reading `action.keeps_keyboard()`, `Action::SPINNER` and `Action::CONFIRM_REMOVE`.
9. **Keys go by the enum.** Each row action's letter is `Action::letter()` in the row view, and `keys.rs` looks the typed character up with `Action::for_letter`. Delete stays Remove's key in `keys.rs`, since it is not a letter. Colours are `style::colour(Action)`, an exhaustive match, and the CSS loop walks `Action::ROW` with `Action::class()`.
10. **CONTEXT.md term:** "Task action", added in Task 1. Task 3 updates the Action menu entry for the waiting case. The Action row entry needs no change.

## File Structure

- Create `src/actions.rs`: the catalogue. `Action`, `TaskState`, `applies`, `label`, `icon`, `args`, and their tests.
- Create `src/menu.rs`: the action menu's view. `Entry`, `entries`, `picked`, `Entry::label`, and their tests.
- Modify `src/lib.rs`: `pub mod actions;` and `pub mod menu;`.
- Modify `src/panel/actions.rs`: from its own enum to the row's view, an `impl Action` block, keeping Clear all.
- Modify `src/panel/model.rs`: `Card::state(has_session) -> Option<TaskState>`.
- Modify `src/panel/state.rs`, `src/panel/keys.rs`, `src/panel/style.rs`, `src/panel/surface.rs`: use the catalogue and the row view.
- Modify `src/task.rs`: remove `up_next_label`, add `is_waiting`.
- Modify `src/refine.rs`: `quick_command` from `crate::actions`.
- Modify `src/main.rs`: `run` → `dispatch`, `task_menu` on `menu.rs`, the CLI test over `actions::Action::ALL`.
- Modify `tests/write_path.rs`: `is_waiting` against a real database.
- Modify `CONTEXT.md` and `README.md`: the new term and the waiting menu.

---

### Task 1: The task-action catalogue

**Files:**
- Create: `src/actions.rs`
- Modify: `src/lib.rs` (add `pub mod actions;` first, before `pub mod daemon;`: the list is alphabetical)
- Modify: `src/task.rs:527-538` (remove `up_next_label`) and its test `the_toggle_reads_as_the_step_it_takes` (~`src/task.rs:731-736`)
- Modify: `src/refine.rs:369-375` (`quick_command`)
- Modify: `src/panel/actions.rs` (`label` and `label_on`: the Up next words now come from the catalogue)
- Modify: `src/main.rs` (the `UpNext` notification, `menu_entries`, the `task_menu` up next arm, and the CLI test)
- Modify: `CONTEXT.md` (new term)

**Interfaces:**
- Consumes: `crate::task::Task` with `is_active()`, `is_planned()`, `is_up_next()` (existing).
- Produces (Tasks 2 and 3 rely on these exact names):
  - `pub enum Action { Session, Back, Start, Refine, Grill, Edit, Note, Speak, UpNext, Stop, Wait, Remove }` (Debug, Clone, Copy, PartialEq, Eq)
  - `Action::ALL: [Action; 12]`, in that order
  - `pub struct TaskState { pub active: bool, pub waiting: bool, pub planned: bool, pub up_next: bool, pub has_session: bool }` (Debug, Clone, Copy, Default, PartialEq, Eq)
  - `TaskState::of(task: &Task, waiting: bool, has_session: bool) -> TaskState`
  - `Action::applies(self, state: TaskState) -> bool`
  - `Action::label(self, up_next: bool) -> &'static str`
  - `Action::icon(self) -> &'static str`
  - `Action::args(self, uuid: &str) -> Vec<String>`

- [ ] **Step 1: Write the catalogue with its tests, behaviour stubbed**

Create `src/actions.rs`. The tests are final. The four methods start out returning wrong values, so the tests fail for the right reason.

```rust
//! The task actions: everything that can be done to one task, each with its
//! words, its icon and the `niritasks` command it runs, and which of them a
//! task gets in each state.
//!
//! The action menu (`menu.rs`) and the task panel's action row
//! (`panel/actions.rs`) are two views of this one list, each choosing its own
//! subset and order. So a label is spelled once, neither view offers a task
//! an action the other knows it would refuse, and a new action is a variant
//! here and a place in each view that shows it.

use crate::task::Task;

/// One thing that can be done to a task. Each is one `niritasks` command,
/// which is what both views run, so a button cannot drift from its menu
/// entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Back to the Claude working on the task. Never starts one.
    Session,
    /// Off waiting and back on the list: Update status → Stopped, which
    /// clears the wait date.
    Back,
    /// The task's own worktree and a Claude to plan it; run again, back to
    /// both.
    Start,
    /// Work the task up into a plan with Claude, in the workspace's herdr
    /// session.
    Refine,
    /// Refine, interviewing first rather than drafting straight away.
    Grill,
    /// The task box on the description. The box rather than a one-line
    /// picker: the descriptions you reach for it to fix are the long ones.
    Edit,
    /// The task box on a new note, under the notes already there.
    Note,
    /// Read the task aloud in the background; run again, on any task, stop.
    Speak,
    /// Mark the task up next; on one already up next, clear it.
    UpNext,
    /// Stop working on it: Update status → Stopped.
    Stop,
    /// Park the task: it leaves the list until it is stopped again.
    Wait,
    /// Delete the task. Its command carries `--yes`, so whatever offers it
    /// must ask first.
    Remove,
}

use Action::*;

/// What decides which actions a task gets. Plain data, so the CLI makes it
/// from a `Task` and the panel from a `Card`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskState {
    /// Started: being worked on.
    pub active: bool,
    /// Parked with Waiting, until a wait date still to come.
    pub waiting: bool,
    /// Carries `+planned`, from Refine or Grill me.
    pub planned: bool,
    /// Carries `+next`.
    pub up_next: bool,
    /// A Claude is working on it in the workspace's herdr session.
    pub has_session: bool,
}

impl TaskState {
    /// `task`'s state. Whether it is waiting is passed in, because its export
    /// cannot say (see `task::is_waiting`), and so is whether a Claude is on
    /// it, which herdr knows and the task does not.
    pub fn of(task: &Task, waiting: bool, has_session: bool) -> TaskState {
        let _ = (task, waiting, has_session);
        TaskState::default()
    }
}

impl Action {
    /// Every task action, in the order the action row puts the ones it has.
    pub const ALL: [Action; 12] = [Session, Back, Start, Refine, Grill, Edit, Note, Speak, UpNext, Stop, Wait, Remove];

    /// Whether the action makes sense on a task in `state`.
    pub fn applies(self, state: TaskState) -> bool {
        let _ = state;
        true
    }

    /// The action's words, the same in the menu and on the row's tooltip.
    pub fn label(self, up_next: bool) -> &'static str {
        let _ = up_next;
        ""
    }

    /// The action's glyph on the action row.
    pub fn icon(self) -> &'static str {
        ""
    }

    /// The `niritasks` arguments the action runs, without the program.
    pub fn args(self, uuid: &str) -> Vec<String> {
        let _ = uuid;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The actions a task in `state` gets, in `ALL`'s order.
    fn offered(state: TaskState) -> Vec<Action> {
        Action::ALL.into_iter().filter(|a| a.applies(state)).collect()
    }

    /// A task on the list, not started: everything but the ways back from
    /// somewhere it is not.
    #[test]
    fn a_task_on_the_list_gets_all_but_session_back_and_stop() {
        assert_eq!(
            offered(TaskState::default()),
            vec![Start, Refine, Grill, Edit, Note, Speak, UpNext, Wait, Remove]
        );
    }

    /// An active task can be stopped, and started again, which goes back to
    /// its worktree and its Claude.
    #[test]
    fn an_active_task_can_be_stopped_and_started_again() {
        let state = TaskState { active: true, ..TaskState::default() };
        assert_eq!(offered(state), vec![Start, Refine, Grill, Edit, Note, Speak, UpNext, Stop, Wait, Remove]);
    }

    /// Go to session never starts a Claude, so it needs one already there.
    #[test]
    fn go_to_session_needs_a_live_claude() {
        assert!(!Session.applies(TaskState::default()));
        assert!(Session.applies(TaskState { has_session: true, ..TaskState::default() }));
    }

    /// A waiting task is parked: the way back, and what still works on a
    /// task off the list. Not Start working, Refine or Grill me, nor Up next,
    /// whatever else is true of it.
    #[test]
    fn a_waiting_task_gets_back_to_list_and_what_still_works_on_it() {
        for has_session in [false, true] {
            for up_next in [false, true] {
                for planned in [false, true] {
                    let state = TaskState { waiting: true, has_session, up_next, planned, ..TaskState::default() };
                    assert_eq!(offered(state), vec![Back, Edit, Note, Speak, Remove], "{state:?}");
                }
            }
        }
    }

    #[test]
    fn labels_read_as_the_menu_does() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label(false)).collect();
        assert_eq!(
            labels,
            vec![
                "Go to session", "Back to list", "Start working", "Refine", "Grill me", "Edit", "Note",
                "Speak", "Up next", "Stop", "Waiting", "Remove",
            ]
        );
    }

    /// The menu finds the picked action by its words, so no two may share
    /// them, either way up next reads.
    #[test]
    fn no_two_actions_share_a_label() {
        for up_next in [false, true] {
            let mut labels: Vec<&str> = Action::ALL.iter().map(|a| a.label(up_next)).collect();
            labels.sort();
            labels.dedup();
            assert_eq!(labels.len(), Action::ALL.len(), "up_next={up_next}");
        }
    }

    /// Up next reads as the step it takes: Not up next on a task already up
    /// next. That is also the word `task up-next` notifies with, for where
    /// the task is once the step is taken. No other action changes its words.
    #[test]
    fn up_next_reads_as_the_step_it_takes() {
        assert_eq!(UpNext.label(false), "Up next");
        assert_eq!(UpNext.label(true), "Not up next");
        for action in Action::ALL.into_iter().filter(|a| *a != UpNext) {
            assert_eq!(action.label(true), action.label(false), "{action:?}");
        }
    }

    /// Icons alone tell the row's buttons apart, so no two may share one.
    #[test]
    fn every_action_has_its_own_icon() {
        let mut icons: Vec<&str> = Action::ALL.iter().map(|a| a.icon()).collect();
        assert!(icons.iter().all(|i| !i.is_empty()));
        icons.sort();
        icons.dedup();
        assert_eq!(icons.len(), Action::ALL.len());
    }

    #[test]
    fn each_action_runs_its_command() {
        let u = "c53b6e3d-ca05-4aae-8588-4ee1abc25f5b";
        assert_eq!(Session.args(u), vec!["task", "session", u]);
        // Stopped is the status that clears a wait date.
        assert_eq!(Back.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Start.args(u), vec!["task", "start", u]);
        assert_eq!(Refine.args(u), vec!["task", "refine", u]);
        assert_eq!(Grill.args(u), vec!["task", "refine", u, "--grill"]);
        assert_eq!(Edit.args(u), vec!["task", "edit", u]);
        assert_eq!(Note.args(u), vec!["task", "note", u]);
        assert_eq!(Speak.args(u), vec!["task", "speak", u]);
        assert_eq!(UpNext.args(u), vec!["task", "up-next", u]);
        assert_eq!(Stop.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Wait.args(u), vec!["task", "status", u, "waiting"]);
        assert_eq!(Remove.args(u), vec!["task", "status", u, "deleted", "--yes"]);
    }

    /// Start working is worktree, herdr and Claude. Marking the task active
    /// alone would be `task status <uuid> active`, which is not it.
    #[test]
    fn start_never_only_marks_the_task_active() {
        assert!(!Start.args("x").contains(&"active".to_string()));
    }

    #[test]
    fn a_tasks_state_is_read_from_the_task() {
        let task: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","start":"20261001T000000Z","tags":["planned","next"]}"#,
        )
        .unwrap();
        assert_eq!(
            TaskState::of(&task, false, true),
            TaskState { active: true, waiting: false, planned: true, up_next: true, has_session: true }
        );
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(TaskState::of(&bare, true, false), TaskState { waiting: true, ..TaskState::default() });
    }
}
```

Add to `src/lib.rs`, as the first `pub mod` line (the list is alphabetical):

```rust
pub mod actions;
```

- [ ] **Step 2: Run the tests to watch them fail**

Run: `cargo test --lib actions::tests`
Expected: it compiles, and every test fails on an assertion, except `start_never_only_marks_the_task_active`, which an empty `args` passes vacuously.

- [ ] **Step 3: Implement the catalogue**

In `src/actions.rs`, replace the stub bodies:

```rust
impl TaskState {
    /// `task`'s state. Whether it is waiting is passed in, because its export
    /// cannot say (see `task::is_waiting`), and so is whether a Claude is on
    /// it, which herdr knows and the task does not.
    pub fn of(task: &Task, waiting: bool, has_session: bool) -> TaskState {
        TaskState {
            active: task.is_active(),
            waiting,
            planned: task.is_planned(),
            up_next: task.is_up_next(),
            has_session,
        }
    }
}

impl Action {
    /// Every task action, in the order the action row puts the ones it has.
    pub const ALL: [Action; 12] = [Session, Back, Start, Refine, Grill, Edit, Note, Speak, UpNext, Stop, Wait, Remove];

    /// Whether the action makes sense on a task in `state`. A waiting task
    /// is parked: it gets the way back to the list, and what works on a task
    /// whatever its place. Start working, Refine and Grill me are for a task
    /// on the list, and Up next and Waiting only move one on it. Go to
    /// session needs a Claude to go to, and Stop a task that was started.
    /// Start working stays on an active task, where it goes back to the
    /// worktree and the Claude; a view with no room for both it and Stop
    /// drops it there.
    pub fn applies(self, state: TaskState) -> bool {
        match self {
            Session => state.has_session && !state.waiting,
            Back => state.waiting,
            Stop => state.active,
            Start | Refine | Grill | UpNext | Wait => !state.waiting,
            Edit | Note | Speak | Remove => true,
        }
    }

    /// The action's words, the same in the menu and on the row's tooltip. Up
    /// next reads as the step it takes, Not up next on a task already up
    /// next, so `up_next` is whether the task is.
    pub fn label(self, up_next: bool) -> &'static str {
        match self {
            Session => "Go to session",
            Back => "Back to list",
            Start => "Start working",
            Refine => "Refine",
            Grill => "Grill me",
            Edit => "Edit",
            Note => "Note",
            Speak => "Speak",
            UpNext if up_next => "Not up next",
            UpNext => "Up next",
            Stop => "Stop",
            Wait => "Waiting",
            Remove => "Remove",
        }
    }

    /// The action's glyph on the action row, so the row stays narrow. Font
    /// Awesome's, from the same Nerd Font as the cards' lock: terminal, undo
    /// arrow, play, magic wand, comments, pencil, sticky note, bookmark,
    /// stop, pause and trash can. Speak's speaker is Material Design's, from
    /// the same font.
    pub fn icon(self) -> &'static str {
        match self {
            Session => "\u{f120}",
            Back => "\u{f0e2}",
            Start => "\u{f04b}",
            Refine => "\u{f0d0}",
            Grill => "\u{f086}",
            Edit => "\u{f040}",
            Note => "\u{f249}",
            // Material Design's volume-medium, not Font Awesome's volume-up,
            // which is drawn nearly twice as wide as its cell and sat off
            // centre; this one fits its cell exactly.
            Speak => "\u{f0580}",
            // Font Awesome's bookmark: marked as the one to do next.
            UpNext => "\u{f02e}",
            Stop => "\u{f04d}",
            Wait => "\u{f04c}",
            Remove => "\u{f1f8}",
        }
    }

    /// The `niritasks` arguments the action runs, without the program: what
    /// the menu runs when it is picked and the row when its button is
    /// pressed. Remove carries `--yes`, because whatever offers it asks
    /// first.
    pub fn args(self, uuid: &str) -> Vec<String> {
        let words: &[&str] = match self {
            Session => &["task", "session", uuid],
            Back => &["task", "status", uuid, "stopped"],
            Start => &["task", "start", uuid],
            Refine => &["task", "refine", uuid],
            Grill => &["task", "refine", uuid, "--grill"],
            Edit => &["task", "edit", uuid],
            Note => &["task", "note", uuid],
            Speak => &["task", "speak", uuid],
            UpNext => &["task", "up-next", uuid],
            Stop => &["task", "status", uuid, "stopped"],
            Wait => &["task", "status", uuid, "waiting"],
            Remove => &["task", "status", uuid, "deleted", "--yes"],
        };
        words.iter().map(|w| w.to_string()).collect()
    }
}
```

- [ ] **Step 4: Move the Up next words and Refine's command onto the catalogue**

`src/task.rs`: delete `up_next_label` (the doc comment and function at ~527-538) and its test `the_toggle_reads_as_the_step_it_takes` (~731-736). Its test lives on as `actions::tests::up_next_reads_as_the_step_it_takes`.

`src/refine.rs`, `quick_command`:

```rust
/// Refine's own command on `uuid`, with `exe` as the `niritasks` binary:
/// built from the task action's arguments, so Add & refine runs what the
/// menu's Refine and the panel's button run.
pub fn quick_command(exe: &str, uuid: &str) -> Vec<String> {
    let mut command = vec![exe.to_string()];
    command.extend(crate::actions::Action::Refine.args(uuid));
    command
}
```

`src/panel/actions.rs` (its own enum lives one more task). In `label`, change the `UpNext` arm to `UpNext => crate::actions::Action::UpNext.label(false),`. In `label_on`, change the `UpNext` arm to `UpNext => crate::actions::Action::UpNext.label(up_next),`.

`src/main.rs`:
- Add `actions::Action` to the `use niri_tasks::{...}` list at the top.
- In `TaskCommand::UpNext`: `notify::tasks(&format!("{}: {}", Action::UpNext.label(t.is_up_next()), t.description));`
- In `menu_entries`: `Action::UpNext.label(up_next),` in place of `task::up_next_label(up_next),`.
- In `task_menu`: `Some(picked) if picked == Action::UpNext.label(up_next) => {`.
- In `mod tests`: delete `use niri_tasks::panel::actions::Action;` (`use super::*;` now brings the catalogue's). Replace `every_card_button_is_a_command_the_cli_accepts` with:

```rust
    /// The menu and the action row both run a task action as its own
    /// `niritasks` words, so each has to be a command the real CLI accepts.
    /// Otherwise a typo shows up as a pick or a click that does nothing.
    #[test]
    fn every_task_action_is_a_command_the_cli_accepts() {
        for action in Action::ALL {
            let mut argv = vec!["niritasks".to_string()];
            argv.extend(action.args("c53b6e3d"));
            if let Err(e) = Cli::try_parse_from(&argv) {
                panic!("{} runs {argv:?}, which the CLI rejects: {e}", action.label(false));
            }
        }
    }
```

- [ ] **Step 5: Name the term in CONTEXT.md**

In `CONTEXT.md`, under `### On screen`, insert this entry directly before `**Action row**:`:

```markdown
**Task action**:
One thing that can be done to a task — Go to session, Back to list, Start
working, Refine, Grill me, Edit, Note, Speak, Up next, Stop, Waiting or
Remove — with its words, its icon and the `niritasks` command it runs. Which
ones a task gets goes by its state: a waiting task gets only Back to list,
Edit, Note, Speak and Remove. The action row and the action menu each show
their own share of them, in their own order.
_Avoid_: command (the CLI's word), button, menu entry, verb
```

- [ ] **Step 6: Run the tests and clippy**

Run: `cargo test && cargo clippy --all-targets`
Expected: every test passes, including the new `actions::tests` and `every_task_action_is_a_command_the_cli_accepts`, and `refine::tests::quick_command_is_the_refine_buttons_command` unchanged. Clippy prints no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/actions.rs src/lib.rs src/task.rs src/refine.rs src/panel/actions.rs src/main.rs CONTEXT.md
git commit -m "refactor: add one list of task actions for the menu and the row

Every task action, its words, icon and niritasks command, and which
ones a task gets in each state, in a library module outside the panel.
refine no longer reaches into the panel for Refine's command, and the
Up next words leave task.rs for the action they name.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The action row as a view of the catalogue

**Files:**
- Modify: `src/panel/actions.rs` (the whole enum and its tests become the row view; Clear all stays)
- Modify: `src/panel/model.rs` (`Card::state`, plus a test)
- Modify: `src/panel/state.rs:12`, `:154-170` (`visible`), `:474-485` (`advance`)
- Modify: `src/panel/keys.rs` (import, letters)
- Modify: `src/panel/style.rs:77-89` (`ACTION_COLOURS` → `colour`), the CSS loop at `:208`, the test at `:258-269`
- Modify: `src/panel/surface.rs:102`, `:742-743`, `:826`

**Interfaces:**
- Consumes: Task 1's `crate::actions::{Action, TaskState}`, `Action::{ALL, applies, label, icon, args}`.
- Produces:
  - `Card::state(&self, has_session: bool) -> Option<TaskState>` (`None` on "+N more")
  - `Action::ROW: [Action; 10]`: `[Session, Back, Start, Refine, Edit, Speak, UpNext, Stop, Wait, Remove]`
  - `Action::row(state: TaskState) -> Vec<Action>`
  - `Action::advance(state: TaskState) -> Option<Action>`
  - `Action::keeps_keyboard(self) -> bool`, `Action::leaves_the_list(self) -> bool` (unchanged meaning)
  - `Action::class(self) -> &'static str`, the CSS class (was `name()`)
  - `Action::letter(self) -> Option<char>`, `Action::for_letter(c: char) -> Option<Action>`
  - `Action::CONFIRM_REMOVE`, `Action::SPINNER`, `CLEAR_ALL`, `CONFIRM_CLEAR_ALL`, `clear_all_command` (unchanged)
  - `style::colour(action: Action) -> &'static str`

- [ ] **Step 1: Write `Card::state` and its failing test**

In `src/panel/model.rs`, add `use crate::actions::TaskState;` under `use crate::task::Task;`. In `impl Card`, after `shows_up_next`:

```rust
    /// The task's state, as the task actions read it, with whether a Claude
    /// is on it. None on "+N more", which stands for no one task.
    pub fn state(&self, has_session: bool) -> Option<TaskState> {
        let _ = has_session;
        None
    }
```

In model.rs's `mod tests`:

```rust
    /// A card's status and tags are its task's state; "+N more" has none.
    #[test]
    fn a_cards_state_is_its_tasks() {
        let card = |status, planned, up_next| Card { status, text: "t".into(), uuid: Some("u".into()), planned, up_next };
        assert_eq!(
            card(Status::Active, true, true).state(true),
            Some(TaskState { active: true, waiting: false, planned: true, up_next: true, has_session: true })
        );
        assert_eq!(
            card(Status::Waiting, false, false).state(false),
            Some(TaskState { waiting: true, ..TaskState::default() })
        );
        assert_eq!(card(Status::Blocked, false, false).state(false), Some(TaskState::default()));
        let more = cap(&[card(Status::Pending, false, false), card(Status::Pending, false, false)], 1).pop().unwrap();
        assert_eq!(more.status, Status::More);
        assert_eq!(more.state(true), None);
    }
```

Run: `cargo test --lib panel::model::tests::a_cards_state_is_its_tasks`
Expected: FAIL (`left: None`).

- [ ] **Step 2: Implement `Card::state`**

```rust
    /// The task's state, as the task actions read it, with whether a Claude
    /// is on it. None on "+N more", which stands for no one task.
    pub fn state(&self, has_session: bool) -> Option<TaskState> {
        (self.status != Status::More).then(|| TaskState {
            active: self.status == Status::Active,
            waiting: self.status == Status::Waiting,
            planned: self.planned,
            up_next: self.up_next,
            has_session,
        })
    }
```

Run: `cargo test --lib panel::model::tests::a_cards_state_is_its_tasks`
Expected: PASS.

- [ ] **Step 3: Rewrite `panel/actions.rs` as the row view, tests first**

Replace everything in `src/panel/actions.rs` above `/// What the Waiting tab's Clear all reads` (the module doc, the enum, `use Action::*;` and the whole `impl Action`) with the following. `CLEAR_ALL`, `CONFIRM_CLEAR_ALL` and `clear_all_command` stay below it, unchanged.

```rust
//! The action row: the buttons on a task card while the panel has the
//! keyboard. A view of the task actions (`crate::actions`): which of them
//! the row shows and in what order, their keys, their CSS classes, and what
//! pressing one does to the panel. Also the Waiting tab's Clear all, which
//! is Remove on every waiting card.
//!
//! Plain data, like `model.rs`, so which card gets which buttons and what
//! they spawn is testable without a compositor.

use crate::actions::{Action, TaskState};
use Action::*;

impl Action {
    /// The row's buttons, left to right. Grill me and Note are the menu's
    /// alone: Enter on the card opens it.
    pub const ROW: [Action; 10] = [Session, Back, Start, Refine, Edit, Speak, UpNext, Stop, Wait, Remove];

    /// What Remove reads between its first press and its second, the way the
    /// menu's delete asks "delete?" before it deletes.
    pub const CONFIRM_REMOVE: &'static str = "Confirm remove";

    /// What Speak shows in place of its speaker while the speech is being got
    /// ready, one frame after another: braille dots going round. Iosevka has
    /// them, so they sit centred in the cell like the text around them.
    pub const SPINNER: [&'static str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    /// The buttons a card in `state` gets, left to right: the row's actions
    /// that apply. The row is narrow, so an active task gets Stop in the
    /// place of Start working, which the menu still offers to go back to its
    /// worktree.
    pub fn row(state: TaskState) -> Vec<Action> {
        let _ = state;
        Vec::new()
    }

    /// The button Ctrl+Enter presses for a card: Refine until the task has a
    /// plan, then Start working. Nothing once a planned task is being
    /// worked, nor on a waiting task, which have no step to take.
    pub fn advance(state: TaskState) -> Option<Action> {
        let _ = state;
        None
    }

    /// Whether the panel keeps the keyboard after the button runs. Back to
    /// list, Up next, Waiting and Remove only change the task, and Speak
    /// plays in the background, so none opens anything that needs the
    /// keyboard and the list stays up; the rest open a box, a terminal or a
    /// menu, which takes it.
    pub fn keeps_keyboard(self) -> bool {
        false
    }

    /// Whether the button takes its card off the list, so the focus has to
    /// move to a neighbour first. Speak's card stays where it is, and so does
    /// Up next's, which only moves up or down the list; the focus stays on the
    /// button, so a second press undoes it.
    pub fn leaves_the_list(self) -> bool {
        false
    }

    /// The button's CSS class, which `style::colour` gives its colour.
    pub fn class(self) -> &'static str {
        ""
    }

    /// The letter that presses the button while the panel has the keyboard.
    /// Not every button has one; Remove's key is Delete, which is not a
    /// letter and is `keys.rs`'s.
    pub fn letter(self) -> Option<char> {
        None
    }

    /// The row's button `c` presses, if any.
    pub fn for_letter(c: char) -> Option<Action> {
        let _ = c;
        None
    }
}
```

Replace the whole `#[cfg(test)] mod tests` in `src/panel/actions.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn on_list(planned: bool) -> TaskState {
        TaskState { planned, ..TaskState::default() }
    }

    fn active(planned: bool) -> TaskState {
        TaskState { active: true, planned, ..TaskState::default() }
    }

    fn waiting(planned: bool) -> TaskState {
        TaskState { waiting: true, planned, ..TaskState::default() }
    }

    fn with_claude(state: TaskState) -> TaskState {
        TaskState { has_session: true, ..state }
    }

    /// Every state a task card can be in, with a Claude on it and without.
    fn every_state() -> Vec<TaskState> {
        let mut all = Vec::new();
        for planned in [false, true] {
            for state in [on_list(planned), active(planned), waiting(planned)] {
                all.push(state);
                all.push(with_claude(state));
            }
        }
        all
    }

    /// Refine until the task has a plan, then Start working; nothing once a
    /// planned task is being worked, nor on a waiting task.
    #[test]
    fn ctrl_enter_refines_an_unplanned_task_and_starts_a_planned_one() {
        assert_eq!(Action::advance(on_list(false)), Some(Refine));
        assert_eq!(Action::advance(active(false)), Some(Refine));
        assert_eq!(Action::advance(on_list(true)), Some(Start));
        assert_eq!(Action::advance(active(true)), None);
        assert_eq!(Action::advance(waiting(false)), None);
        assert_eq!(Action::advance(waiting(true)), None);
    }

    /// What Ctrl+Enter picks is always one of the card's own buttons, so the
    /// key never does something no click could.
    #[test]
    fn ctrl_enter_only_presses_a_button_the_card_has() {
        for state in every_state() {
            if let Some(action) = Action::advance(state) {
                assert!(Action::row(state).contains(&action), "{state:?} picks {action:?}, which it has no button for");
            }
        }
    }

    #[test]
    fn an_active_task_gets_stop_in_place_of_start() {
        let classes: Vec<&str> = Action::row(active(false)).iter().map(|a| a.class()).collect();
        assert_eq!(classes, vec!["refine", "edit", "speak", "up-next", "stop", "wait", "remove"]);
    }

    #[test]
    fn a_task_not_yet_active_gets_start_and_no_stop() {
        for planned in [false, true] {
            assert_eq!(
                Action::row(on_list(planned)),
                vec![Start, Refine, Edit, Speak, UpNext, Wait, Remove],
                "planned={planned}"
            );
        }
    }

    /// Go to session leads the row only while a Claude is on the task — on an
    /// active one, in the place Start working has on the others.
    #[test]
    fn a_task_with_a_live_claude_gets_go_to_session_first() {
        assert_eq!(
            Action::row(with_claude(active(false))),
            vec![Session, Refine, Edit, Speak, UpNext, Stop, Wait, Remove]
        );
        // A refine open on a task not yet started.
        assert_eq!(
            Action::row(with_claude(on_list(true))),
            vec![Session, Start, Refine, Edit, Speak, UpNext, Wait, Remove]
        );
    }

    /// A waiting task gets the way back, and what still works on it, a
    /// Claude or not.
    #[test]
    fn a_waiting_task_gets_back_to_list_edit_speak_and_remove() {
        for state in [waiting(false), with_claude(waiting(true))] {
            assert_eq!(Action::row(state), vec![Back, Edit, Speak, Remove], "{state:?}");
        }
    }

    /// Every card that stands for a task can be listened to, whatever its
    /// state.
    #[test]
    fn every_task_card_gets_speak() {
        for state in every_state() {
            assert!(Action::row(state).contains(&Speak), "{state:?}");
        }
    }

    /// Every card on the list gets Up next, active ones too; a waiting task
    /// is off the list.
    #[test]
    fn every_card_but_a_waiting_one_gets_up_next() {
        for state in every_state() {
            assert_eq!(Action::row(state).contains(&UpNext), !state.waiting, "{state:?}");
        }
    }

    /// Note and Grill me are the menu's alone.
    #[test]
    fn the_row_leaves_note_and_grill_me_to_the_menu() {
        for state in every_state() {
            let row = Action::row(state);
            assert!(!row.contains(&Note) && !row.contains(&Grill), "{state:?}");
        }
    }

    /// Speak and Up next open nothing, so the list stays up, as it does for
    /// the buttons that only change the task.
    #[test]
    fn back_speak_up_next_wait_and_remove_keep_the_list_open() {
        let kept: Vec<Action> = Action::ROW.into_iter().filter(|a| a.keeps_keyboard()).collect();
        assert_eq!(kept, vec![Back, Speak, UpNext, Wait, Remove]);
    }

    /// Up next's card stays on the list, only moving up it, so the focus
    /// stays on the button and a second press clears it.
    #[test]
    fn up_next_keeps_its_card_on_the_list() {
        assert!(!UpNext.leaves_the_list());
    }

    /// Their card drops off the list, so the focus moves to a neighbour;
    /// Speak's card stays, and so does the focus, on the button that stops it.
    #[test]
    fn only_back_wait_and_remove_take_their_card_off_the_list() {
        let leaving: Vec<Action> = Action::ROW.into_iter().filter(|a| a.leaves_the_list()).collect();
        assert_eq!(leaving, vec![Back, Wait, Remove]);
        assert!(leaving.iter().all(|a| a.keeps_keyboard()), "a button that releases the keyboard moves no focus");
    }

    /// The class is what gives a button its colour, so no two may share one.
    #[test]
    fn every_button_has_its_own_class() {
        let mut classes: Vec<&str> = Action::ROW.iter().map(|a| a.class()).collect();
        assert!(classes.iter().all(|c| !c.is_empty()));
        classes.sort();
        classes.dedup();
        assert_eq!(classes.len(), Action::ROW.len());
    }

    /// g b s r e t press Go to session, Back to list, Start working, Refine,
    /// Edit and Stop, and each letter finds its own button.
    #[test]
    fn letters_press_their_own_buttons() {
        let lettered: Vec<(char, Action)> =
            Action::ROW.into_iter().filter_map(|a| a.letter().map(|c| (c, a))).collect();
        assert_eq!(lettered, vec![('g', Session), ('b', Back), ('s', Start), ('r', Refine), ('e', Edit), ('t', Stop)]);
        for (c, action) in lettered {
            assert_eq!(Action::for_letter(c), Some(action));
        }
        assert_eq!(Action::for_letter('x'), None);
    }

    /// The spinner's frames turn in Speak's place, so none may look like an
    /// action's icon, and each differs from the one before.
    #[test]
    fn the_spinner_turns_and_is_no_buttons_icon() {
        for (i, frame) in Action::SPINNER.iter().enumerate() {
            assert!(!Action::ALL.iter().any(|a| a.icon() == *frame), "{frame}");
            assert_ne!(*frame, Action::SPINNER[(i + 1) % Action::SPINNER.len()]);
        }
    }

    #[test]
    fn remove_asks_before_it_deletes() {
        assert_eq!(Action::CONFIRM_REMOVE, "Confirm remove");
    }

    /// Clear all asks in the same words as Remove, with its own name.
    #[test]
    fn clear_all_reads_as_remove_does() {
        assert_eq!(CLEAR_ALL, "Clear all");
        assert_eq!(CONFIRM_CLEAR_ALL, "Confirm clear all");
    }

    /// Run for real, with `echo` standing in for niritasks: each uuid gets
    /// Remove's own command, in order.
    #[test]
    fn clear_all_runs_remove_on_each_task_in_turn() {
        let uuids = vec!["aaaa-1".to_string(), "bbbb-2".to_string()];
        let command = clear_all_command("echo", &uuids);
        assert_eq!(&command[..2], ["sh", "-c"]);
        let out = std::process::Command::new(&command[0]).args(&command[1..]).output().unwrap();
        assert!(out.status.success());
        let expected: String = uuids.iter().map(|u| Remove.args(u).join(" ") + "\n").collect();
        assert_eq!(String::from_utf8(out.stdout).unwrap(), expected);
    }

    /// niritasks's path and the uuids are the shell's arguments, never spliced
    /// into its script, so a path with a space still works.
    #[test]
    fn clear_all_passes_niritasks_and_the_uuids_as_arguments() {
        let command = clear_all_command("/opt/my tools/niritasks", &["u1".into()]);
        assert_eq!(command[3], "/opt/my tools/niritasks");
        assert_eq!(command[4], "u1");
        assert!(!command[2].contains("my tools"));
    }
}
```

(The catalogue's tests already cover the dropped `labels_read_as_the_menu_does`, `up_next_reads_not_up_next_on_a_task_up_next`, `every_button_has_its_own_icon`, `each_button_runs_its_menu_entrys_command`, `start_never_only_marks_the_task_active`, `back_to_list_is_only_on_a_waiting_task` and `more_is_no_one_task_and_gets_no_buttons`. The last one moved to `model::tests::a_cards_state_is_its_tasks` and `state::tests::only_a_task_card_on_the_keyboard_gets_buttons`.)

- [ ] **Step 4: Point the panel's other modules at the catalogue**

`src/panel/state.rs`:
- Line 12: `use super::actions::Action;` → `use crate::actions::Action;`
- In `visible()`, the `Some(uuid)` arm:

```rust
                    Some(uuid) => {
                        let has_session = crate::link::session_agent(&self.agents, uuid).is_some();
                        card.state(has_session).map(Action::row).unwrap_or_default()
                    }
```

- In `advance()`, replace `let Some(action) = Action::advance(card.card.status, card.card.planned) else { return Vec::new() };` with:

```rust
        let Some(action) = card.card.state(false).and_then(Action::advance) else { return Vec::new() };
```

`src/panel/keys.rs`:
- `use super::actions::Action;` → `use crate::actions::Action;`
- In `key_action`, delete the six letter arms (`gdk::Key::g` … `gdk::Key::t`) and replace the final `_ => KeyAction::Ignore,` with:

```rust
        // A letter presses the row's button that has it.
        key => match key.to_unicode().and_then(Action::for_letter) {
            Some(action) => KeyAction::Run(action),
            None => KeyAction::Ignore,
        },
```

  The `Delete | KP_Delete => KeyAction::Run(Action::Remove)` arm stays. `keys.rs`'s tests are unchanged. `letters_and_delete_run_their_buttons` and `capitals_run_their_buttons_too` now check the row view's letters through `key_action`.

`src/panel/style.rs`:
- Add `use crate::actions::Action;` at the top of the file, after the module doc.
- Replace `ACTION_COLOURS` (its doc comment and the array) with:

```rust
/// Each task action's colour. Exhaustive, so a new action cannot be a grey
/// button. Note and Grill me are the menu's alone. They wear the colour of
/// the button nearest them, Edit's and Refine's, so one moved onto the row
/// needs no new colour.
pub fn colour(action: Action) -> &'static str {
    match action {
        Action::Session => SESSION,
        Action::Back => BACK,
        Action::Start => ACTIVE,
        Action::Refine | Action::Grill => REFINE,
        Action::Edit | Action::Note => EDIT,
        Action::Speak => SPEAK,
        Action::UpNext => UP_NEXT,
        Action::Stop => STOP,
        Action::Wait => WAIT,
        Action::Remove => REMOVE,
    }
}
```

- In `css()`, replace the `for (name, colour) in ACTION_COLOURS {` loop with:

```rust
    for action in Action::ROW {
        let (name, colour) = (action.class(), colour(action));
        css.push_str(&format!(
            ".task-panel .card-actions .{name} {{ color: {colour}; }}\n\
             .task-panel .card-actions .{name}:focus {{ background-color: alpha({colour}, 0.2); }}\n"
        ));
    }
```

- Replace the test `every_action_button_has_its_colour` with:

```rust
    /// Each button in its own colour, Start sharing the active task's green.
    #[test]
    fn every_action_button_has_its_colour() {
        let css = css();
        for action in Action::ROW {
            let rule = format!(".card-actions .{} {{ color: {}; }}", action.class(), colour(action));
            assert!(css.contains(&rule), "missing `{rule}`");
        }
        assert!(css.contains(".card-actions .start { color: #8cd283; }"), "Start is ACTIVE's green");
    }
```

`src/panel/surface.rs`:
- Line 102: `use super::actions::{self, Action};` → `use super::actions;` and `use crate::actions::Action;`
- In `action_row` (~742-743): `button.set_tooltip_text(Some(action.label(up_next)));` and `button.add_css_class(action.class());`
- In the hint (~826): `action.label(row.up_next)`

- [ ] **Step 5: Run the row's tests to watch the stubs fail**

Run: `cargo test --lib panel::`
Expected: it compiles, and the row view's tests fail on assertions (for example `a_task_not_yet_active_gets_start_and_no_stop`: `left: []`). So do state.rs's button tests and keys.rs's letter tests.

- [ ] **Step 6: Fill in the row view**

In `src/panel/actions.rs`, replace the stub bodies:

```rust
    pub fn row(state: TaskState) -> Vec<Action> {
        Self::ROW
            .into_iter()
            .filter(|a| a.applies(state) && !(*a == Start && state.active))
            .collect()
    }

    pub fn advance(state: TaskState) -> Option<Action> {
        match state {
            TaskState { waiting: true, .. } => None,
            TaskState { active: true, planned: true, .. } => None,
            TaskState { planned: true, .. } => Some(Start),
            _ => Some(Refine),
        }
    }

    pub fn keeps_keyboard(self) -> bool {
        matches!(self, Back | Speak | UpNext | Wait | Remove)
    }

    pub fn leaves_the_list(self) -> bool {
        matches!(self, Back | Wait | Remove)
    }

    pub fn class(self) -> &'static str {
        match self {
            Session => "session",
            Back => "back",
            Start => "start",
            Refine => "refine",
            Grill => "grill",
            Edit => "edit",
            Note => "note",
            Speak => "speak",
            UpNext => "up-next",
            Stop => "stop",
            Wait => "wait",
            Remove => "remove",
        }
    }

    pub fn letter(self) -> Option<char> {
        match self {
            Session => Some('g'),
            Back => Some('b'),
            Start => Some('s'),
            Refine => Some('r'),
            Edit => Some('e'),
            Stop => Some('t'),
            Grill | Note | Speak | UpNext | Wait | Remove => None,
        }
    }

    pub fn for_letter(c: char) -> Option<Action> {
        Self::ROW.into_iter().find(|a| a.letter() == Some(c))
    }
```

(Keep each method's doc comment from Step 3.)

- [ ] **Step 7: Run every test, clippy and the panel e2e**

Run: `cargo test && cargo clippy --all-targets`
Expected: all pass. `grep -rn "ACTION_COLOURS\|for_status\|label_on\|\.name()" src/panel` finds nothing except `Filter`'s and `button.label()` hits that are not about actions.

Run: `cargo build && NIRITASKS=./target/debug/niritasks bash tests/e2e-panel.sh`
Expected: every check passes. If the machine can't run the nested niri, say so in the report. Don't skip it silently.

- [ ] **Step 8: Commit**

```bash
git add src/panel/actions.rs src/panel/model.rs src/panel/state.rs src/panel/keys.rs src/panel/style.rs src/panel/surface.rs
git commit -m "refactor(panel): build the action row from the task actions

The row's buttons are now the task actions that apply to the card,
less Start working on an active task. Its colours and letters go by
the action, not by a CSS class string that had to match it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The action menu as a view of the catalogue

**Files:**
- Create: `src/menu.rs`
- Modify: `src/lib.rs` (`pub mod menu;` between `pub mod link;` and `pub mod niri;`)
- Modify: `src/task.rs` (add `is_waiting` after `waiting_for_tag`)
- Modify: `tests/write_path.rs` (~`:417-440`, the wait and stop section)
- Modify: `src/main.rs` (`run` → `dispatch`, `TaskCommand::Menu`, `task_list`'s tail, delete `GO_TO_SESSION` and `menu_entries`, rewrite `task_menu`, and in tests delete `go_to_session_leads_the_menu_only_when_there_is_one` and `the_menu_offers_to_clear_up_next_on_a_task_up_next`)
- Modify: `CONTEXT.md` (the Action menu entry), `README.md:66-68`

**Interfaces:**
- Consumes: Task 1's `Action::{applies, label, args}`, `TaskState::of`. Existing `task::export` (private to task.rs), `link::live_agent(&str, &str) -> Option<_>`.
- Produces:
  - `pub fn task::is_waiting(uuid: &str) -> Result<bool>`
  - `pub enum menu::Entry { Run(Action), Status, Move }` (Debug, Clone, Copy, PartialEq, Eq)
  - `menu::Entry::label(self, up_next: bool) -> &'static str`
  - `pub fn menu::entries(state: TaskState) -> Vec<Entry>`
  - `pub fn menu::picked(label: &str, state: TaskState) -> Option<Entry>`

- [ ] **Step 1: Write the failing `is_waiting` check against a real database**

In `tests/write_path.rs`, after the existing `assert!(raw(&parked, "start").is_empty(), …);` that follows `task::wait(&parked)`, add:

```rust
    // The export says pending, so whether the task is waiting is
    // taskwarrior's +WAITING: what the menu asks before it offers Start
    // working.
    assert!(task::is_waiting(&parked).expect("is_waiting"), "a parked task is waiting");
```

After `task::stop(&parked).expect("stop the waiting task");` and its wait-date assert, add:

```rust
    assert!(!task::is_waiting(&parked).expect("is_waiting"), "a stopped task is not waiting");

    // A wait date that has passed stays on the task, but the task is back
    // on the list: not waiting.
    task::add(TAG, &text::add_args("waited past wait:now-1h")).expect("add");
    let waited = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description == "waited past")
        .expect("a passed wait date leaves the task pending")
        .uuid;
    assert!(!raw(&waited, "wait").is_empty(), "the passed wait date is still on the task");
    assert!(!task::is_waiting(&waited).expect("is_waiting"), "a passed wait date is not waiting");
```

The extra "waited past" task can stay. The later sections count pending tasks relative to a `before` taken just before them.

Run: `cargo test --test write_path`
Expected: compile error, `cannot find function is_waiting in module task`.

- [ ] **Step 2: Implement `is_waiting`**

In `src/task.rs`, after `waiting_for_tag`:

```rust
/// Whether the task is parked as waiting: taskwarrior's own `+WAITING`, a
/// wait date still to come. `get` cannot say, since 2.6 exports a waiting
/// task as `pending`, and a wait date that has passed stays on the task.
pub fn is_waiting(uuid: &str) -> Result<bool> {
    Ok(!export(&[uuid, "+WAITING"])?.is_empty())
}
```

Run: `cargo test --test write_path`
Expected: PASS.

- [ ] **Step 3: Write `menu.rs` with its tests, behaviour stubbed**

Create `src/menu.rs`:

```rust
//! The action menu: the fuzzel menu of what can be done to one task, as
//! plain data. A view of the task actions (`actions.rs`): which of them it
//! offers and in what order. Plus its two entries that are not one command,
//! Update status and Move to workspace, which ask a second question first.

use crate::actions::{Action, TaskState};

/// One row of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// A task action. Picking it runs the action's own `niritasks` command.
    Run(Action),
    /// Pick a state from `task::Status::ALL`, then move the task to it.
    Status,
    /// Pick a `~/Projects` folder, then retag the task to its workspace.
    Move,
}

/// The task actions the menu offers, in its order. Go to session leads,
/// while there is a Claude to go to. The row's Back to list, Stop, Waiting
/// and Remove are Update status's states here.
const ACTIONS: [Action; 8] = [
    Action::Session,
    Action::Edit,
    Action::Note,
    Action::Speak,
    Action::UpNext,
    Action::Refine,
    Action::Grill,
    Action::Start,
];

impl Entry {
    /// The row's words. `up_next` is whether the task is, for Up next's.
    pub fn label(self, up_next: bool) -> &'static str {
        let _ = (self, up_next);
        ""
    }
}

/// The menu for a task in `state`, top to bottom: the actions that apply,
/// then Update status and Move to workspace, which every task gets.
pub fn entries(state: TaskState) -> Vec<Entry> {
    let _ = (state, ACTIONS);
    Vec::new()
}

/// The entry fuzzel handed back, by its words. None for anything else:
/// fuzzel echoes typed text that matches no row.
pub fn picked(label: &str, state: TaskState) -> Option<Entry> {
    let _ = (label, state);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(state: TaskState) -> Vec<&'static str> {
        entries(state).into_iter().map(|e| e.label(state.up_next)).collect()
    }

    const ON_LIST: [&str; 9] = [
        "Edit", "Note", "Speak", "Up next", "Refine", "Grill me", "Start working", "Update status", "Move to workspace",
    ];

    #[test]
    fn a_task_on_the_list_gets_the_whole_menu() {
        assert_eq!(labels(TaskState::default()), ON_LIST);
    }

    /// Go to session leads the menu when there is a session to go to, and a
    /// task with none gets exactly the menu it always had.
    #[test]
    fn go_to_session_leads_the_menu_only_when_there_is_one() {
        let with = labels(TaskState { has_session: true, ..TaskState::default() });
        assert_eq!(with[0], "Go to session");
        assert_eq!(with[1..], ON_LIST);
    }

    /// The menu offers the step up next would take: Not up next, in the same
    /// place, on a task already up next.
    #[test]
    fn the_menu_offers_to_clear_up_next_on_a_task_up_next() {
        let marked = labels(TaskState { up_next: true, ..TaskState::default() });
        assert_eq!(marked[3], "Not up next");
        assert!(!marked.contains(&"Up next"));
    }

    /// Start working on an active task goes back to its worktree and its
    /// Claude, so the menu keeps it there; stopping is Update status's.
    #[test]
    fn an_active_task_keeps_start_working() {
        assert_eq!(labels(TaskState { active: true, ..TaskState::default() }), ON_LIST);
    }

    /// A waiting task is parked: no Start working, Refine or Grill me,
    /// which are for a task on the list, nor Up next or Go to session.
    /// Update status → Stopped brings it back.
    #[test]
    fn a_waiting_task_gets_no_start_working_or_refine() {
        for has_session in [false, true] {
            for up_next in [false, true] {
                let state = TaskState { waiting: true, has_session, up_next, ..TaskState::default() };
                assert_eq!(labels(state), ["Edit", "Note", "Speak", "Update status", "Move to workspace"], "{state:?}");
            }
        }
    }

    /// Every row the menu shows finds its own entry again, and no two rows
    /// read the same.
    #[test]
    fn every_row_picks_its_own_entry() {
        for waiting in [false, true] {
            for has_session in [false, true] {
                for up_next in [false, true] {
                    let state = TaskState { waiting, has_session, up_next, ..TaskState::default() };
                    for entry in entries(state) {
                        assert_eq!(picked(entry.label(up_next), state), Some(entry), "{state:?}");
                    }
                }
            }
        }
    }

    /// Typed text that matches no row, or a row this task's menu does not
    /// have, picks nothing.
    #[test]
    fn words_off_the_menu_pick_nothing() {
        assert_eq!(picked("Edti", TaskState::default()), None);
        assert_eq!(picked("Start working", TaskState { waiting: true, ..TaskState::default() }), None);
        assert_eq!(picked("Up next", TaskState { up_next: true, ..TaskState::default() }), None);
        assert_eq!(picked("Stop", TaskState { active: true, ..TaskState::default() }), None);
    }

    #[test]
    fn the_status_and_move_rows_run_their_own_pickers() {
        assert_eq!(picked("Update status", TaskState::default()), Some(Entry::Status));
        assert_eq!(picked("Move to workspace", TaskState::default()), Some(Entry::Move));
        assert_eq!(picked("Edit", TaskState::default()), Some(Entry::Run(Action::Edit)));
    }
}
```

In `src/lib.rs`, add `pub mod menu;` between `pub mod link;` and `pub mod niri;`.

Run: `cargo test --lib menu::tests`
Expected: FAIL on assertions (`left: []`), not on compile errors.

- [ ] **Step 4: Implement the menu view**

In `src/menu.rs`, replace the stubs:

```rust
impl Entry {
    /// The row's words. `up_next` is whether the task is, for Up next's.
    pub fn label(self, up_next: bool) -> &'static str {
        match self {
            Entry::Run(action) => action.label(up_next),
            Entry::Status => "Update status",
            Entry::Move => "Move to workspace",
        }
    }
}

/// The menu for a task in `state`, top to bottom: the actions that apply,
/// then Update status and Move to workspace, which every task gets.
pub fn entries(state: TaskState) -> Vec<Entry> {
    ACTIONS
        .into_iter()
        .filter(|a| a.applies(state))
        .map(Entry::Run)
        .chain([Entry::Status, Entry::Move])
        .collect()
}

/// The entry fuzzel handed back, by its words. None for anything else:
/// fuzzel echoes typed text that matches no row.
pub fn picked(label: &str, state: TaskState) -> Option<Entry> {
    entries(state).into_iter().find(|e| e.label(state.up_next) == label)
}
```

Run: `cargo test --lib menu::tests`
Expected: PASS.

- [ ] **Step 5: Run the menu from `main.rs` through the catalogue**

In `src/main.rs`:

1. Change the `use niri_tasks::{...}` list to include `actions::{Action, TaskState}` and `menu` (in place of the `actions::Action` added in Task 1).

2. Split `run` so a menu pick can reuse it:

```rust
fn run() -> Result<()> {
    dispatch(Cli::parse())
}

/// Run one parsed command line: the shell's, or a picked task action's own
/// words, so the menu runs what the same words run anywhere.
fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        // ... the existing arms of run()'s match, unchanged ...
    }
    Ok(())
}
```

3. `TaskCommand::Menu`:

```rust
        TaskCommand::Menu { uuid } => {
            let tag = require_workspace_tag()?;
            let t = task::get(&uuid)?.context("task not found")?;
            // A waiting task's card opens this too, and its export says
            // pending, so ask taskwarrior.
            let waiting = task::is_waiting(&t.uuid)?;
            let width = niri_tasks::picker::clamp_task_width(t.description.chars().count());
            return task_menu(&tag, &t, waiting, width);
        }
```

4. The tail of `task_list`, from `let picked = tasks.iter()…` to the end:

```rust
    let Some(picked) = tasks.iter().find(|t| t.uuid == selected) else { return Ok(()) };
    // pending_for_tag leaves waiting tasks out, so none picked here is one.
    task_menu(&tag, picked, false, niri_tasks::picker::clamp_task_width(longest))
}
```

5. Delete `GO_TO_SESSION` and `menu_entries` (with their doc comments). Replace `task_menu` with:

```rust
/// The menu for one task, and doing the entry picked. `t` is the task as
/// found, so its uuid is the full one even when a prefix was typed: the link
/// to its Claude is derived from it. `waiting` is whether it is parked, which
/// its export cannot say. `width` is the delete confirmation's, matched to
/// the list it was reached from.
fn task_menu(tag: &str, t: &task::Task, waiting: bool, width: usize) -> Result<()> {
    // Asked of herdr on every open; a session that is not running answers
    // at once, and no answer just means no Go to session.
    let workspace = niri::focused_workspace_name()?.unwrap_or_default();
    let has_session = link::live_agent(&workspace, &t.uuid).is_some();
    let state = TaskState::of(t, waiting, has_session);
    let labels: Vec<String> = menu::entries(state).iter().map(|e| e.label(state.up_next).to_string()).collect();
    let picked = Picker::new()
        .lines(labels.len())
        .width(20)
        .prompt("")
        .run(&labels)?;

    match picked.as_deref().and_then(|label| menu::picked(label, state)) {
        // The action's own words, through the same parser as a shell's, so
        // the menu runs exactly what the action row's button runs.
        Some(menu::Entry::Run(action)) => {
            let argv = std::iter::once("niritasks".to_string()).chain(action.args(&t.uuid));
            dispatch(Cli::try_parse_from(argv)?)
        }
        Some(menu::Entry::Status) => task_status(&t.uuid, &t.description, width),
        Some(menu::Entry::Move) => task_move(tag, &t.uuid, &t.description),
        None => Ok(()),
    }
}
```

6. In `mod tests`, delete `go_to_session_leads_the_menu_only_when_there_is_one` and `the_menu_offers_to_clear_up_next_on_a_task_up_next`. They now live in `menu::tests`. Keep `going_to_a_tasks_session_is_a_command` and `speaking_a_task_is_a_command`.

- [ ] **Step 6: Say it in CONTEXT.md and README.md**

`CONTEXT.md`, the **Action menu** entry. After "…Update status and Move to workspace." add the sentence below, so the entry reads:

```markdown
**Action menu**:
The fuzzel menu of what can be done to one task, opened by picking it in the
picker, clicking its task card, or Enter on the card: Go to session while a
Claude is working on it, then Edit, Note, Speak, Up next (Not up next on a
task already up next), Refine, Grill me, Start working, Update status and
Move to workspace. A waiting task's is just Edit, Note, Speak, Update status
and Move to workspace. Speak reads the task aloud in the background; picked
again, on any task, it stops.
_Avoid_: context menu, actions list
```

`README.md`, the paragraph at ~66-68. Change "…led by Go to session while a Claude is working on the task." to:

```markdown
workspace — led by Go to session while a Claude is working on the task, and
on a waiting task just Edit, Note, Speak, Update status and Move to
workspace. Every
```

Keep the line wrapping close to the surrounding text's.

- [ ] **Step 7: Run everything**

Run: `cargo test && cargo clippy --all-targets`
Expected: all pass, the docs tests included. Then `grep -rn 'Some("Edit")\|GO_TO_SESSION\|menu_entries\|up_next_label\|panel::actions::Action' src` finds nothing.

Check by hand, with the build: pick a waiting task's uuid (`task +WAITING export` on any tag) and run `./target/debug/niritasks task menu <uuid>` from that task's workspace. The fuzzel menu should list only Edit, Note, Speak, Update status and Move to workspace. Pick Speak, then Speak again to stop it. On a pending task, check that the menu still has its nine rows and that Edit opens the task box. If you can't reach a display, say so in the report.

- [ ] **Step 8: Commit**

```bash
git add src/menu.rs src/lib.rs src/task.rs tests/write_path.rs src/main.rs CONTEXT.md README.md
git commit -m "refactor: run the action menu from the task actions

The menu's rows are now the task actions that apply to the task, plus
Update status and Move to workspace, and a pick runs the action's own
niritasks words through the CLI's parser instead of a match on its
label. The menu now reads whether the task is waiting, so a waiting
task's menu no longer offers Start working, Refine, Grill me or Up
next.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
