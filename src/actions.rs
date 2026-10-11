//! The task actions: everything that can be done to one task, each with its
//! words, the `niritasks` command it runs, which of them a task gets in each
//! state, and the facts the panel draws and keys it by (`Action::facts`):
//! glyph, CSS class, key, colour, and what a press does to the panel.
//!
//! The task panel's action row (`panel/actions.rs`) is the view of them:
//! which it shows and the hints beside them. A new action is a variant here,
//! a row of `Action::facts` and a CLI subcommand.

use crate::names;
use crate::task::Task;

/// One thing that can be done to a task. Each is one `niritasks` command,
/// which is what its button runs, so a script can do anything a click can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The task's Go to list in the panel: what can be done with it beyond
    /// this row, and where it can be gone to (`panel::goto`). Runs no
    /// command itself; each row of the list runs its own.
    GoTo,
    /// Back on the list: `task status <uuid> stopped`, which clears a waiting
    /// task's wait date and reopens a finished one.
    Back,
    /// The task's own worktree and a Claude to plan it; run again, back to
    /// both.
    Start,
    /// Work the task up into a plan with Claude, in the workspace's herdr
    /// session.
    Refine,
    /// The task box on the description and every note. The box, not a
    /// one-line prompt: the descriptions you reach for it to fix are the long
    /// ones.
    Edit,
    /// Mark the task up next; on one already up next, clear it.
    UpNext,
    /// Mark it done: it leaves the list, as `task status <uuid> completed`.
    Complete,
    /// Move it to another `~/Projects` folder's workspace, picked on the
    /// panel's project list.
    Move,
    /// Stop working on it: `task status <uuid> stopped`.
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
    /// Being planned in the background by `niritasks task refine
    /// --unattended`, which tags it `+processing` while it runs: nothing may
    /// be done to it until the tag comes off.
    pub processing: bool,
    /// Parked with Waiting, until a wait date still to come.
    pub waiting: bool,
    /// Completed: on the Finished tab, off the list as a waiting task is.
    pub finished: bool,
    /// Carries `+planned`, from Refine or Grill me.
    pub planned: bool,
    /// Carries `+next`.
    pub up_next: bool,
    /// Its working Claude, `work-<uuid8>`, the one Start working starts, is
    /// live in the workspace's herdr session.
    pub has_work: bool,
    /// Its refine Claude, `task-<uuid8>`, the one Refine starts, is live
    /// there.
    pub has_refine: bool,
    /// Its notes link a refine report, `Report: <path>`: the Go to list
    /// offers Open report and Regenerate report.
    pub has_report: bool,
    /// Its notes link a design spec, `Spec: <path>`.
    pub has_spec: bool,
    /// Its notes link an implementation plan, `Plan: <path>`.
    pub has_plan: bool,
}

impl TaskState {
    /// Off the list: parked as waiting, or finished. Either way it gets the
    /// way back and what works on any task, and nothing that moves a task
    /// along the list.
    pub fn off_list(self) -> bool {
        self.waiting || self.finished
    }

    /// `task`'s state, finished included, read from its status and notes.
    /// Whether it is waiting is passed in, because its export cannot say (see
    /// `task::is_waiting`), and so are the herdr session's live agent names,
    /// which say which of its Claudes are on it: herdr knows, the task does
    /// not.
    pub fn of(task: &Task, waiting: bool, agents: &[String]) -> TaskState {
        TaskState {
            active: task.is_active(),
            waiting,
            // A completed task's export says so, unlike a waiting one's.
            finished: task.status == "completed",
            planned: task.is_planned(),
            up_next: task.is_up_next(),
            has_report: task.report_path().is_some(),
            has_spec: task.spec_path().is_some(),
            has_plan: task.plan_path().is_some(),
            processing: task.is_processing(),
            ..TaskState::default()
        }
        .with_agents(agents, &task.uuid)
    }

    /// This state with which of `uuid`'s Claudes are live among `agents`,
    /// the herdr session's agent names: its working Claude and its refine,
    /// told apart by name (see `names`).
    pub fn with_agents(self, agents: &[String], uuid: &str) -> TaskState {
        let live = |name: String| agents.contains(&name);
        TaskState { has_work: live(names::work_agent(uuid)), has_refine: live(names::refine_agent(uuid)), ..self }
    }
}

/// What the panel draws and keys an action by: its glyph, its CSS class, the
/// key that presses it, its colour, and what pressing it does to the panel.
/// One row per action, so a new action is one arm of [`Action::facts`] and
/// a CLI subcommand, nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    /// The glyph on the action row, from the Nerd Font the cards use.
    pub icon: &'static str,
    /// The button's CSS class, which the stylesheet colours.
    pub class: &'static str,
    /// The key that presses the button while the panel has the keyboard, as
    /// the hint names it: a letter, or "Del" for Remove. None for a button
    /// only Enter presses.
    pub key: Option<&'static str>,
    /// The button's colour, from `panel::style`'s palette.
    pub colour: &'static str,
    /// The panel keeps the keyboard after the button runs: nothing opens that
    /// would need it, and the list stays up.
    pub keeps_keyboard: bool,
    /// The button takes its card off the list, so the focus moves to a
    /// neighbour first.
    pub leaves_the_list: bool,
}

impl Action {
    /// The row of the table for this action.
    ///
    /// The glyphs keep the row narrow. Font Awesome's, from the same Nerd
    /// Font as the cards' lock: ellipsis, undo arrow, play, magic wand,
    /// pencil, bookmark, check, open folder, stop, pause and trash can.
    ///
    /// The colours are Catppuccin mocha's, one per button (see
    /// `panel::style`); Start shares the active task's green.
    ///
    /// Back to list, Up next, Complete, Waiting and Remove only change the
    /// task, so none opens anything that needs the keyboard and the list
    /// stays up. Go to… and Move open a list in the panel itself. The rest
    /// open a box, a terminal or a herdr tab, which takes it. Up next's card
    /// stays where it is, only moving up or down the list; the focus stays
    /// on the button, so a second press undoes it.
    pub const fn facts(self) -> Facts {
        use crate::panel::style as c;
        match self {
            // Font Awesome's ellipsis: more, as the words' own "…".
            GoTo => Facts { icon: "\u{f141}", class: "go-to", key: Some("o"), colour: c::GO_TO, keeps_keyboard: true, leaves_the_list: false },
            Back => Facts { icon: "\u{f0e2}", class: "back", key: Some("b"), colour: c::BACK, keeps_keyboard: true, leaves_the_list: true },
            Start => Facts { icon: "\u{f04b}", class: "start", key: Some("s"), colour: c::ACTIVE, keeps_keyboard: false, leaves_the_list: false },
            Refine => Facts { icon: "\u{f0d0}", class: "refine", key: Some("r"), colour: c::REFINE, keeps_keyboard: false, leaves_the_list: false },
            Edit => Facts { icon: "\u{f040}", class: "edit", key: Some("e"), colour: c::EDIT, keeps_keyboard: false, leaves_the_list: false },
            // Font Awesome's bookmark: marked as the one to do next.
            UpNext => Facts { icon: "\u{f02e}", class: "up-next", key: None, colour: c::UP_NEXT, keeps_keyboard: true, leaves_the_list: false },
            // Font Awesome's check: done.
            Complete => Facts { icon: "\u{f00c}", class: "complete", key: Some("c"), colour: c::COMPLETE, keeps_keyboard: true, leaves_the_list: true },
            // Font Awesome's open folder: off to another project.
            Move => Facts { icon: "\u{f07c}", class: "move", key: Some("m"), colour: c::MOVE, keeps_keyboard: true, leaves_the_list: false },
            Stop => Facts { icon: "\u{f04d}", class: "stop", key: Some("t"), colour: c::STOP, keeps_keyboard: false, leaves_the_list: false },
            Wait => Facts { icon: "\u{f04c}", class: "wait", key: None, colour: c::WAIT, keeps_keyboard: true, leaves_the_list: true },
            Remove => Facts { icon: "\u{f1f8}", class: "remove", key: Some("Del"), colour: c::REMOVE, keeps_keyboard: true, leaves_the_list: true },
        }
    }

    /// The action's glyph on the action row, so the row stays narrow; the
    /// glyphs are named on [`Action::facts`].
    pub fn icon(self) -> &'static str {
        self.facts().icon
    }

    /// The button's CSS class, which `style::colour` gives its colour.
    pub fn class(self) -> &'static str {
        self.facts().class
    }

    /// The letter that presses the button while the panel has the keyboard.
    /// Not every button has one; Remove's key is Delete, which is not a
    /// letter and is `keys.rs`'s, and some buttons only Enter presses.
    pub fn letter(self) -> Option<char> {
        let key = self.facts().key?;
        let mut chars = key.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    }

    /// Whether the panel keeps the keyboard after the button runs. See
    /// [`Action::facts`].
    pub fn keeps_keyboard(self) -> bool {
        self.facts().keeps_keyboard
    }

    /// Whether the button takes its card off the list. See [`Action::facts`].
    pub fn leaves_the_list(self) -> bool {
        self.facts().leaves_the_list
    }
}

impl Action {
    /// Every task action, in the order the action row puts the ones it has.
    pub const ALL: [Action; 11] = [GoTo, Back, Start, Refine, Edit, UpNext, Complete, Move, Stop, Wait, Remove];

    /// Whether the action makes sense on a task in `state`. A waiting or
    /// finished task is off the list: it gets the way back to it, and what
    /// works on a task whatever its place. Start working and Refine are for
    /// a task on the list, and Up next and Waiting only move one on it. Stop
    /// needs a task that was started. Start working stays on an active task,
    /// where it goes back to the worktree and the Claude; a view with no room
    /// for both it and Stop drops it there. Go to… is on every task: its
    /// list has rows for any (`panel::goto::Item::applies`). A processing
    /// task gets nothing at all: a Claude is writing it, and anything done
    /// to it meanwhile would race that write.
    pub fn applies(self, state: TaskState) -> bool {
        if state.processing {
            return false;
        }
        match self {
            GoTo => true,
            Back => state.off_list(),
            Stop => state.active,
            Start | Refine | UpNext | Wait => !state.off_list(),
            // A finished task is done already, and stays where it was done.
            Complete | Move => !state.finished,
            Edit | Remove => true,
        }
    }

    /// The action's words, on its button's tooltip and in the card's hint.
    /// Up next reads as the step it takes: Not up next on a task already up
    /// next.
    pub fn label(self, state: TaskState) -> &'static str {
        match self {
            GoTo => crate::panel::goto::LABEL,
            Back => "Back to list",
            Start => "Start working",
            Refine => "Refine",
            Edit => "Edit",
            UpNext if state.up_next => "Not up next",
            UpNext => "Up next",
            Complete => "Complete",
            Move => "Move to workspace",
            Stop => "Stop",
            Wait => "Waiting",
            Remove => "Remove",
        }
    }

    /// The `niritasks` arguments the action runs, without the program: what
    /// its button spawns. Remove carries `--yes`, because its button asks
    /// first. Move's stop short of the folder, which the project list adds.
    /// Go to…'s are none: its button opens the Go to list, whose rows run
    /// their own (`panel::goto::Item::args`).
    pub fn args(self, uuid: &str) -> Vec<String> {
        let words: &[&str] = match self {
            GoTo => &[],
            Back => &["task", "status", uuid, "stopped"],
            Start => &["task", "start", uuid],
            Refine => &["task", "refine", uuid],
            Edit => &["task", "edit", uuid],
            UpNext => &["task", "up-next", uuid],
            Complete => &["task", "status", uuid, "completed"],
            Move => &["task", "move", uuid],
            Stop => &["task", "status", uuid, "stopped"],
            Wait => &["task", "status", uuid, "waiting"],
            Remove => &["task", "status", uuid, "deleted", "--yes"],
        };
        words.iter().map(|w| w.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The actions a task in `state` gets, in `ALL`'s order.
    fn offered(state: TaskState) -> Vec<Action> {
        Action::ALL.into_iter().filter(|a| a.applies(state)).collect()
    }

    /// A task on the list, not started: everything but the way back from
    /// somewhere it is not, and Stop.
    #[test]
    fn a_task_on_the_list_gets_all_but_back_and_stop() {
        let row = vec![GoTo, Start, Refine, Edit, UpNext, Complete, Move, Wait, Remove];
        assert_eq!(offered(TaskState::default()), row);
        assert_eq!(offered(TaskState { planned: true, has_report: true, ..TaskState::default() }), row);
    }

    /// A task being planned in the background gets nothing at all, whatever
    /// else is true of it: a Claude is writing it, and anything done to it
    /// meanwhile, a second Claude, an edit, a status change, would race that
    /// write. Off the list too: nothing brings it back until the run ends.
    #[test]
    fn a_processing_task_gets_nothing() {
        let processing = TaskState { processing: true, ..TaskState::default() };
        assert_eq!(offered(processing), Vec::<Action>::new());
        let every_way = TaskState {
            processing: true, active: true, planned: true, up_next: true, has_work: true, has_refine: true, has_report: true, has_spec: true, has_plan: true,
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
        assert_eq!(TaskState::of(&t, false, &[]), TaskState { processing: true, ..TaskState::default() });
    }

    /// The state knows which of the task's Claudes are live, by herdr name,
    /// and which documents its notes link: another task's agents are not
    /// this one's.
    #[test]
    fn of_reads_the_live_claudes_and_the_linked_documents() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"7cd9fd3a-d27b","description":"d","status":"pending",
                "annotations":[{"entry":"20261006T010000Z","description":"Spec: docs/s.md"},
                               {"entry":"20261006T020000Z","description":"Plan: docs/p.md"}]}"#,
        )
        .unwrap();
        let refine_only = vec!["task-7cd9fd3a".to_string(), "work-11111111".to_string()];
        assert_eq!(
            TaskState::of(&t, false, &refine_only),
            TaskState { has_refine: true, has_spec: true, has_plan: true, ..TaskState::default() }
        );
        let both = vec!["work-7cd9fd3a".to_string(), "task-7cd9fd3a".to_string()];
        let state = TaskState::of(&t, false, &both);
        assert!(state.has_work && state.has_refine);
    }

    /// An active task can be stopped, and started again, which goes back to
    /// its worktree and its Claude.
    #[test]
    fn an_active_task_can_be_stopped_and_started_again() {
        let state = TaskState { active: true, ..TaskState::default() };
        assert_eq!(offered(state), vec![GoTo, Start, Refine, Edit, UpNext, Complete, Move, Stop, Wait, Remove]);
    }

    /// Go to… is on every task, on the list or off it: its list always has
    /// a row. Never on a processing one.
    #[test]
    fn go_to_is_on_every_task_but_a_processing_one() {
        for state in [
            TaskState::default(),
            TaskState { active: true, ..TaskState::default() },
            TaskState { waiting: true, ..TaskState::default() },
            TaskState { finished: true, ..TaskState::default() },
        ] {
            assert!(GoTo.applies(state), "{state:?}");
            assert!(!GoTo.applies(TaskState { processing: true, ..state }), "{state:?}");
        }
    }

    /// A waiting task is parked: the way back, and what still works on a
    /// task off the list. Not Start working or Refine, nor Up next,
    /// whatever else is true of it.
    #[test]
    fn a_waiting_task_gets_back_to_list_and_what_still_works_on_it() {
        for has_work in [false, true] {
            for up_next in [false, true] {
                for planned in [false, true] {
                    let state = TaskState { waiting: true, has_work, up_next, planned, ..TaskState::default() };
                    assert_eq!(offered(state), vec![GoTo, Back, Edit, Complete, Move, Remove], "{state:?}");
                }
            }
        }
    }

    #[test]
    fn labels_read_as_the_tooltips_do() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label(TaskState::default())).collect();
        assert_eq!(
            labels,
            vec![
                "Go to…", "Back to list", "Start working", "Refine", "Edit", "Up next", "Complete",
                "Move to workspace", "Stop", "Waiting", "Remove",
            ]
        );
    }

    /// Each is its button's tooltip, so no two may share one, whichever way
    /// Up next reads.
    #[test]
    fn no_two_actions_share_a_label() {
        for up_next in [false, true] {
            let state = TaskState { up_next, ..TaskState::default() };
            let mut labels: Vec<&str> = Action::ALL.iter().map(|a| a.label(state)).collect();
            labels.sort();
            labels.dedup();
            assert_eq!(labels.len(), Action::ALL.len(), "{state:?}");
        }
    }

    /// Up next reads as the step it takes: Not up next on a task already up
    /// next, the word `task up-next` notifies with too. No other action
    /// changes its words.
    #[test]
    fn only_up_next_reads_as_the_step_it_takes() {
        let plain = TaskState::default();
        assert_eq!(UpNext.label(plain), "Up next");
        assert_eq!(UpNext.label(TaskState { up_next: true, ..plain }), "Not up next");
        let every_way = TaskState { active: true, planned: true, has_work: true, up_next: true, has_report: true, ..plain };
        for action in Action::ALL.into_iter().filter(|a| *a != UpNext) {
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
            TaskState::of(&with, false, &[]),
            TaskState { planned: true, has_report: true, ..TaskState::default() }
        );
        assert_eq!(TaskState::of(&without, false, &[]), TaskState { planned: true, ..TaskState::default() });
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
        assert!(GoTo.args(u).is_empty(), "the list's rows run their own");
        // Stopped is the status that clears a wait date.
        assert_eq!(Back.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Start.args(u), vec!["task", "start", u]);
        assert_eq!(Refine.args(u), vec!["task", "refine", u]);
        assert_eq!(Edit.args(u), vec!["task", "edit", u]);
        assert_eq!(UpNext.args(u), vec!["task", "up-next", u]);
        assert_eq!(Complete.args(u), vec!["task", "status", u, "completed"]);
        // The project list adds the folder.
        assert_eq!(Move.args(u), vec!["task", "move", u]);
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
            TaskState::of(&task, false, &[names::work_agent("u")]),
            TaskState { active: true, planned: true, up_next: true, has_work: true, ..TaskState::default() }
        );
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(TaskState::of(&bare, true, &[]), TaskState { waiting: true, ..TaskState::default() });
        let done: Task = serde_json::from_str(r#"{"uuid":"u","description":"d","status":"completed"}"#).unwrap();
        assert_eq!(TaskState::of(&done, false, &[]), TaskState { finished: true, ..TaskState::default() });
    }

    /// A finished task is off the list, as a waiting one is: the way back,
    /// and what still works on any task. Never Start working or Up next, and
    /// not Complete, being done already.
    #[test]
    fn a_finished_task_gets_back_to_list_and_what_still_works_on_it() {
        for has_work in [false, true] {
            for up_next in [false, true] {
                for planned in [false, true] {
                    let state = TaskState { finished: true, has_work, up_next, planned, ..TaskState::default() };
                    assert_eq!(offered(state), vec![GoTo, Back, Edit, Remove], "{state:?}");
                }
            }
        }
    }

    /// One row per action, and no two rows alike where the panel tells
    /// buttons apart: by glyph, by class, and by key.
    #[test]
    fn every_action_has_its_own_glyph_class_and_key() {
        let mut icons: Vec<&str> = Action::ALL.iter().map(|a| a.facts().icon).collect();
        let mut classes: Vec<&str> = Action::ALL.iter().map(|a| a.facts().class).collect();
        let mut keys: Vec<&str> = Action::ALL.iter().filter_map(|a| a.facts().key).collect();
        assert!(icons.iter().all(|i| !i.is_empty()));
        assert!(classes.iter().all(|c| !c.is_empty()));
        for list in [&mut icons, &mut classes, &mut keys] {
            let before = list.len();
            list.sort();
            list.dedup();
            assert_eq!(list.len(), before);
        }
        assert_eq!(Action::Remove.facts().key, Some("Del"));
        assert_eq!(Action::UpNext.facts().key, None);
        assert_eq!(Action::GoTo.facts().key, Some("o"));
    }

    /// What a press does to the panel, read off the table.
    #[test]
    fn the_table_says_what_a_press_does_to_the_panel() {
        let keeps: Vec<Action> = Action::ALL.into_iter().filter(|a| a.keeps_keyboard()).collect();
        assert_eq!(keeps, vec![GoTo, Back, UpNext, Complete, Move, Wait, Remove]);
        // Go to… opens its list in the panel, and the card stays.
        assert!(GoTo.keeps_keyboard() && !GoTo.leaves_the_list());
        let leaves: Vec<Action> = Action::ALL.into_iter().filter(|a| a.leaves_the_list()).collect();
        assert_eq!(leaves, vec![Back, Complete, Wait, Remove]);
    }
}
