//! The task actions: everything that can be done to one task, each with its
//! words, the `niritasks` command it runs, which of them a task gets in each
//! state, and the facts the panel draws and keys it by (`Action::facts`):
//! glyph, CSS class, key, colour, and what a press does to the panel.
//!
//! The task panel's action row (`panel/actions.rs`) is the view of them:
//! which it shows and the hints beside them. A new action is a variant here,
//! a row of `Action::facts` and a CLI subcommand.

use crate::task::Task;

/// One thing that can be done to a task. Each is one `niritasks` command,
/// which is what its button runs, so a script can do anything a click can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Back to the Claude working on the task. Never starts one.
    Session,
    /// Back on the list: `task status <uuid> stopped`, which clears a waiting
    /// task's wait date and reopens a finished one.
    Back,
    /// The task's own worktree and a Claude to plan it; run again, back to
    /// both.
    Start,
    /// Work the task up into a plan with Claude, in the workspace's herdr
    /// session.
    Refine,
    /// Refine, interviewing first rather than drafting straight away.
    Grill,
    /// The refine report of a planned task's plan, built by Claude in the
    /// workspace's herdr session, opened in the browser and linked from the
    /// task's notes, without refining again.
    Report,
    /// The task box on the description and every note. The box, not a
    /// one-line prompt: the descriptions you reach for it to fix are the long
    /// ones.
    Edit,
    /// Read the task aloud in the background; run again, on any task, stop.
    Speak,
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
    /// Parked with Waiting, until a wait date still to come.
    pub waiting: bool,
    /// Completed: on the Finished tab, off the list as a waiting task is.
    pub finished: bool,
    /// Carries `+planned`, from Refine or Grill me.
    pub planned: bool,
    /// Carries `+next`.
    pub up_next: bool,
    /// A Claude is working on it in the workspace's herdr session.
    pub has_session: bool,
}

impl TaskState {
    /// Off the list: parked as waiting, or finished. Either way it gets the
    /// way back and what works on any task, and nothing that moves a task
    /// along the list.
    pub fn off_list(self) -> bool {
        self.waiting || self.finished
    }

    /// `task`'s state, finished included, read from its status. Whether it is
    /// waiting is passed in, because its export cannot say (see
    /// `task::is_waiting`), and so is whether a Claude is on it, which herdr
    /// knows and the task does not.
    pub fn of(task: &Task, waiting: bool, has_session: bool) -> TaskState {
        TaskState {
            active: task.is_active(),
            waiting,
            // A completed task's export says so, unlike a waiting one's.
            finished: task.status == "completed",
            planned: task.is_planned(),
            up_next: task.is_up_next(),
            has_session,
        }
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
    /// Font as the cards' lock: terminal, undo arrow, play, magic wand,
    /// comments, file-text, pencil, bookmark, check, open folder, stop, pause
    /// and trash can. Speak's speaker is Material Design's, from the same font.
    ///
    /// The colours are Catppuccin mocha's, one per button (see
    /// `panel::style`); Grill me and Report wear Refine's mauve, one being a
    /// refine that interviews first and the other a report of what a refine
    /// wrote, and Start shares the active task's green.
    ///
    /// Back to list, Up next, Complete, Waiting and Remove only change the
    /// task, and Speak plays in the background, so none opens anything that
    /// needs the keyboard and the list stays up. Move opens the project list
    /// in the panel itself. The rest open a box, a terminal or a herdr tab,
    /// which takes it. Speak's card stays where it is, and so does Up next's,
    /// which only moves up or down the list; the focus stays on the button,
    /// so a second press undoes it.
    pub const fn facts(self) -> Facts {
        use crate::panel::style as c;
        match self {
            Session => Facts { icon: "\u{f120}", class: "session", key: Some("g"), colour: c::SESSION, keeps_keyboard: false, leaves_the_list: false },
            Back => Facts { icon: "\u{f0e2}", class: "back", key: Some("b"), colour: c::BACK, keeps_keyboard: true, leaves_the_list: true },
            Start => Facts { icon: "\u{f04b}", class: "start", key: Some("s"), colour: c::ACTIVE, keeps_keyboard: false, leaves_the_list: false },
            Refine => Facts { icon: "\u{f0d0}", class: "refine", key: Some("r"), colour: c::REFINE, keeps_keyboard: false, leaves_the_list: false },
            Grill => Facts { icon: "\u{f086}", class: "grill", key: Some("i"), colour: c::REFINE, keeps_keyboard: false, leaves_the_list: false },
            // Font Awesome's file-text: the report. Refine's mauve, being a
            // view of what a refine wrote.
            Report => Facts { icon: "\u{f15c}", class: "report", key: Some("p"), colour: c::REFINE, keeps_keyboard: false, leaves_the_list: false },
            Edit => Facts { icon: "\u{f040}", class: "edit", key: Some("e"), colour: c::EDIT, keeps_keyboard: false, leaves_the_list: false },
            // Material Design's volume-medium, not Font Awesome's volume-up,
            // which is drawn nearly twice as wide as its cell and sat off
            // centre; this one fits its cell exactly.
            Speak => Facts { icon: "\u{f0580}", class: "speak", key: None, colour: c::SPEAK, keeps_keyboard: true, leaves_the_list: false },
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
    pub const ALL: [Action; 14] =
        [Session, Back, Start, Refine, Grill, Report, Edit, Speak, UpNext, Complete, Move, Stop, Wait, Remove];

    /// Whether the action makes sense on a task in `state`. A waiting or
    /// finished task is off the list: it gets the way back to it, and what
    /// works on a task whatever its place. Start working, Refine and Grill me
    /// are for a task on the list, and Up next and Waiting only move one on
    /// it. Go to session needs a Claude to go to, and Stop a task that was
    /// started. Start working stays on an active task, where it goes back to
    /// the worktree and the Claude; a view with no room for both it and Stop
    /// drops it there. Report is for a planned task on the list: an unplanned
    /// one has no plan to report on.
    pub fn applies(self, state: TaskState) -> bool {
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

    /// The action's words, on its button's tooltip and in the card's hint. Up
    /// next reads as the step it takes, Not up next on a task already up
    /// next, so `up_next` is whether the task is.
    pub fn label(self, up_next: bool) -> &'static str {
        match self {
            Session => "Go to session",
            Back => "Back to list",
            Start => "Start working",
            Refine => "Refine",
            Grill => "Grill me",
            Report => "Report",
            Edit => "Edit",
            Speak => "Speak",
            UpNext if up_next => "Not up next",
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
    pub fn args(self, uuid: &str) -> Vec<String> {
        let words: &[&str] = match self {
            Session => &["task", "session", uuid],
            Back => &["task", "status", uuid, "stopped"],
            Start => &["task", "start", uuid],
            Refine => &["task", "refine", uuid],
            Grill => &["task", "refine", uuid, "--grill"],
            Report => &["task", "report", uuid],
            Edit => &["task", "edit", uuid],
            Speak => &["task", "speak", uuid],
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

    /// An active task can be stopped, and started again, which goes back to
    /// its worktree and its Claude.
    #[test]
    fn an_active_task_can_be_stopped_and_started_again() {
        let state = TaskState { active: true, ..TaskState::default() };
        assert_eq!(offered(state), vec![Start, Refine, Grill, Edit, Speak, UpNext, Complete, Move, Stop, Wait, Remove]);
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
                    assert_eq!(offered(state), vec![Back, Edit, Speak, Complete, Move, Remove], "{state:?}");
                }
            }
        }
    }

    #[test]
    fn labels_read_as_the_tooltips_do() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label(false)).collect();
        assert_eq!(
            labels,
            vec![
                "Go to session", "Back to list", "Start working", "Refine", "Grill me", "Report", "Edit",
                "Speak", "Up next", "Complete", "Move to workspace", "Stop", "Waiting", "Remove",
            ]
        );
    }

    /// Each is its button's tooltip, so no two may share one, either way up next reads.
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
        assert_eq!(Report.args(u), vec!["task", "report", u]);
        assert_eq!(Edit.args(u), vec!["task", "edit", u]);
        assert_eq!(Speak.args(u), vec!["task", "speak", u]);
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
            TaskState::of(&task, false, true),
            TaskState { active: true, waiting: false, finished: false, planned: true, up_next: true, has_session: true }
        );
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(TaskState::of(&bare, true, false), TaskState { waiting: true, ..TaskState::default() });
        let done: Task = serde_json::from_str(r#"{"uuid":"u","description":"d","status":"completed"}"#).unwrap();
        assert_eq!(TaskState::of(&done, false, false), TaskState { finished: true, ..TaskState::default() });
    }

    /// A finished task is off the list, as a waiting one is: the way back,
    /// and what still works on any task. Never Start working or Up next, and
    /// not Complete, being done already.
    #[test]
    fn a_finished_task_gets_back_to_list_and_what_still_works_on_it() {
        for has_session in [false, true] {
            for up_next in [false, true] {
                for planned in [false, true] {
                    let state = TaskState { finished: true, has_session, up_next, planned, ..TaskState::default() };
                    assert_eq!(offered(state), vec![Back, Edit, Speak, Remove], "{state:?}");
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
        assert_eq!(Action::Speak.facts().key, None);
    }

    /// What a press does to the panel, read off the table.
    #[test]
    fn the_table_says_what_a_press_does_to_the_panel() {
        let keeps: Vec<Action> = Action::ALL.into_iter().filter(|a| a.keeps_keyboard()).collect();
        assert_eq!(keeps, vec![Back, Speak, UpNext, Complete, Move, Wait, Remove]);
        // Report opens a herdr tab, which takes the keyboard, and the card stays.
        assert!(!Report.keeps_keyboard() && !Report.leaves_the_list());
        let leaves: Vec<Action> = Action::ALL.into_iter().filter(|a| a.leaves_the_list()).collect();
        assert_eq!(leaves, vec![Back, Complete, Wait, Remove]);
    }
}
