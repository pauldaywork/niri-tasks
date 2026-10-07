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
    /// Back on the list: Update status → Stopped, which clears a waiting
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

    /// `task`'s state, finished included, read from its status. Whether it is waiting is passed in, because its export
    /// cannot say (see `task::is_waiting`), and so is whether a Claude is on
    /// it, which herdr knows and the task does not.
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

impl Action {
    /// Every task action, in the order the action row puts the ones it has.
    pub const ALL: [Action; 12] = [Session, Back, Start, Refine, Grill, Edit, Note, Speak, UpNext, Stop, Wait, Remove];

    /// Whether the action makes sense on a task in `state`. A waiting or
    /// finished task is off the list: it gets the way back to it, and what works on a task
    /// whatever its place. Start working, Refine and Grill me are for a task
    /// on the list, and Up next and Waiting only move one on it. Go to
    /// session needs a Claude to go to, and Stop a task that was started.
    /// Start working stays on an active task, where it goes back to the
    /// worktree and the Claude; a view with no room for both it and Stop
    /// drops it there.
    pub fn applies(self, state: TaskState) -> bool {
        match self {
            Session => state.has_session && !state.off_list(),
            Back => state.off_list(),
            Stop => state.active,
            Start | Refine | Grill | UpNext | Wait => !state.off_list(),
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
            TaskState { active: true, waiting: false, finished: false, planned: true, up_next: true, has_session: true }
        );
        let bare: Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(TaskState::of(&bare, true, false), TaskState { waiting: true, ..TaskState::default() });
        let done: Task = serde_json::from_str(r#"{"uuid":"u","description":"d","status":"completed"}"#).unwrap();
        assert_eq!(TaskState::of(&done, false, false), TaskState { finished: true, ..TaskState::default() });
    }

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
}
