//! The buttons on a task card while the panel has the keyboard, and what each
//! one runs.
//!
//! Plain data, like `model.rs`, so which card gets which buttons and what they
//! spawn is testable without a compositor. Each runs the same `niritasks`
//! command as its entry in the fuzzel menu, so a button cannot drift from the
//! menu entry it stands in for.

use super::model::Status;

/// The card's buttons, in the order they sit left to right. Each stands in for
/// its entry in the fuzzel menu, running the same `niritasks` command so a
/// button cannot drift from the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Back to the Claude working on the task; only on a card that has one.
    Session,
    /// Off waiting and back on the list: the menu's Update status → Stopped,
    /// which clears the wait date. Only on a waiting task, which Start working
    /// and Refine refuse.
    Back,
    Start,
    Refine,
    Edit,
    Stop,
    /// Park the task: it leaves the panel until it is stopped again.
    Wait,
    Remove,
}

use Action::*;

impl Action {
    /// In the order the buttons sit, left to right.
    pub const ALL: [Action; 8] = [Session, Back, Start, Refine, Edit, Stop, Wait, Remove];

    /// What Remove reads between its first press and its second, the way the
    /// menu's delete asks "delete?" before it deletes.
    pub const CONFIRM_REMOVE: &'static str = "Confirm remove";

    /// The buttons a card gets, left to right. Go to session leads while a
    /// Claude is working on the task (`has_session`), as it leads the menu. An
    /// active task is already being worked, so it gets Stop and no Start
    /// working; the rest have nothing to stop. A waiting task gets only Back
    /// to list, Edit and Remove: Start working and Refine refuse a task that
    /// is not pending. None on "+N more", which stands for no one task.
    pub fn for_status(status: Status, has_session: bool) -> Vec<Action> {
        let skip = match status {
            Status::More => return Vec::new(),
            Status::Waiting => return vec![Back, Edit, Remove],
            Status::Active => Start,
            Status::Pending | Status::Blocked | Status::Planned => Stop,
        };
        Self::ALL
            .into_iter()
            .filter(|a| *a != skip && *a != Back && (*a != Session || has_session))
            .collect()
    }

    /// Whether the panel keeps the keyboard after the button runs. Back to
    /// list, Waiting and Remove only change the task and open nothing that needs the keyboard,
    /// so the list stays up for the next one; the rest open a box, a terminal
    /// or a menu, which takes it.
    pub fn keeps_keyboard(self) -> bool {
        matches!(self, Back | Wait | Remove)
    }

    /// The button's name in words, the menu's own: its icon's tooltip.
    pub fn label(self) -> &'static str {
        match self {
            Session => "Go to session",
            Back => "Back to list",
            Start => "Start working",
            Refine => "Refine",
            Edit => "Edit",
            Stop => "Stop",
            Wait => "Waiting",
            Remove => "Remove",
        }
    }

    /// What the button shows: a glyph, so the row stays narrow. Font Awesome's,
    /// from the same Nerd Font as the cards' lock: terminal, play, magic wand,
    /// pencil, stop, pause and trash can, and Back to list's undo arrow.
    pub fn icon(self) -> &'static str {
        match self {
            Session => "\u{f120}",
            Back => "\u{f0e2}",
            Start => "\u{f04b}",
            Refine => "\u{f0d0}",
            Edit => "\u{f040}",
            Stop => "\u{f04d}",
            Wait => "\u{f04c}",
            Remove => "\u{f1f8}",
        }
    }

    /// The button Ctrl+Enter presses for a card: Refine until the task has a
    /// plan, then Start. Nothing once a planned task is being worked, nor on a
    /// waiting task or "+N more", which have no step to take.
    pub fn advance(status: Status, planned: bool) -> Option<Action> {
        match status {
            Status::Waiting | Status::More => None,
            Status::Active if planned => None,
            _ if planned => Some(Start),
            _ => Some(Refine),
        }
    }

    /// The button's CSS class, which gives it its colour, and its widget name,
    /// which lets a re-render put focus back on the same button.
    pub fn name(self) -> &'static str {
        match self {
            Session => "session",
            Back => "back",
            Start => "start",
            Refine => "refine",
            Edit => "edit",
            Stop => "stop",
            Wait => "wait",
            Remove => "remove",
        }
    }

    /// The `niritasks` arguments the button runs, without the program: what
    /// its menu entry runs. Remove carries `--yes` because its second press is
    /// the confirmation.
    pub fn args(self, uuid: &str) -> Vec<String> {
        let words: &[&str] = match self {
            Session => &["task", "session", uuid],
            Back => &["task", "status", uuid, "stopped"],
            Start => &["task", "start", uuid],
            Refine => &["task", "refine", uuid],
            Edit => &["task", "edit", uuid],
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

    /// Refine until the task has a plan, then Start working; nothing once a
    /// planned task is being worked, nor on a waiting task or "+N more".
    #[test]
    fn ctrl_enter_refines_an_unplanned_task_and_starts_a_planned_one() {
        assert_eq!(Action::advance(Status::Pending, false), Some(Refine));
        assert_eq!(Action::advance(Status::Blocked, false), Some(Refine));
        assert_eq!(Action::advance(Status::Active, false), Some(Refine));
        assert_eq!(Action::advance(Status::Planned, true), Some(Start));
        assert_eq!(Action::advance(Status::Blocked, true), Some(Start));
        assert_eq!(Action::advance(Status::Active, true), None);
        assert_eq!(Action::advance(Status::Waiting, false), None);
        assert_eq!(Action::advance(Status::Waiting, true), None);
        assert_eq!(Action::advance(Status::More, false), None);
    }

    /// What Ctrl+Enter picks is always one of the card's own buttons, so the
    /// key never does something no click could.
    #[test]
    fn ctrl_enter_only_presses_a_button_the_card_has() {
        let all = [Status::Active, Status::Pending, Status::Blocked, Status::Planned, Status::Waiting, Status::More];
        for status in all {
            for planned in [false, true] {
                if let Some(action) = Action::advance(status, planned) {
                    assert!(
                        Action::for_status(status, false).contains(&action),
                        "{status:?} planned={planned} picks {action:?}, which it has no button for"
                    );
                }
            }
        }
    }

    #[test]
    fn an_active_task_gets_stop_in_place_of_start() {
        let names: Vec<&str> = Action::for_status(Status::Active, false).iter().map(|a| a.name()).collect();
        assert_eq!(names, vec!["refine", "edit", "stop", "wait", "remove"]);
    }

    #[test]
    fn a_task_not_yet_active_gets_start_and_no_stop() {
        for status in [Status::Pending, Status::Blocked, Status::Planned] {
            let got = Action::for_status(status, false);
            assert_eq!(got, vec![Action::Start, Action::Refine, Action::Edit, Action::Wait, Action::Remove], "{status:?}");
        }
    }

    /// Go to session leads the row only while a Claude is on the task — on an
    /// active one, in the place Start working has on the others.
    #[test]
    fn a_task_with_a_live_claude_gets_go_to_session_first() {
        assert_eq!(
            Action::for_status(Status::Active, true),
            vec![Action::Session, Action::Refine, Action::Edit, Action::Stop, Action::Wait, Action::Remove]
        );
        // A refine open on a task not yet started.
        assert_eq!(
            Action::for_status(Status::Planned, true),
            vec![Action::Session, Action::Start, Action::Refine, Action::Edit, Action::Wait, Action::Remove]
        );
    }

    /// Start working and Refine refuse a task that is not pending, so a
    /// waiting one gets the way back instead, and what still works on it.
    #[test]
    fn a_waiting_task_gets_back_to_list_edit_and_remove() {
        assert_eq!(
            Action::for_status(Status::Waiting, false),
            vec![Action::Back, Action::Edit, Action::Remove]
        );
    }

    #[test]
    fn back_to_list_is_only_on_a_waiting_task() {
        for status in [Status::Active, Status::Pending, Status::Blocked, Status::Planned] {
            assert!(!Action::for_status(status, true).contains(&Action::Back), "{status:?}");
        }
    }

    #[test]
    fn more_is_no_one_task_and_gets_no_buttons() {
        assert!(Action::for_status(Status::More, false).is_empty());
        assert!(Action::for_status(Status::More, true).is_empty());
    }

    #[test]
    fn only_back_wait_and_remove_keep_the_list_open() {
        let kept: Vec<Action> = Action::ALL.into_iter().filter(|a| a.keeps_keyboard()).collect();
        assert_eq!(kept, vec![Action::Back, Action::Wait, Action::Remove]);
    }

    #[test]
    fn labels_read_as_the_menu_does() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label()).collect();
        assert_eq!(labels, vec!["Go to session", "Back to list", "Start working", "Refine", "Edit", "Stop", "Waiting", "Remove"]);
        assert_eq!(Action::CONFIRM_REMOVE, "Confirm remove");
    }

    /// Icons alone tell the buttons apart, so no two may share one.
    #[test]
    fn every_button_has_its_own_icon() {
        let mut icons: Vec<&str> = Action::ALL.iter().map(|a| a.icon()).collect();
        assert!(icons.iter().all(|i| !i.is_empty()));
        icons.sort();
        icons.dedup();
        assert_eq!(icons.len(), Action::ALL.len());
    }

    #[test]
    fn each_button_runs_its_menu_entrys_command() {
        let u = "c53b6e3d-ca05-4aae-8588-4ee1abc25f5b";
        assert_eq!(Action::Session.args(u), vec!["task", "session", u]);
        assert_eq!(Action::Start.args(u), vec!["task", "start", u]);
        assert_eq!(Action::Refine.args(u), vec!["task", "refine", u]);
        assert_eq!(Action::Edit.args(u), vec!["task", "edit", u]);
        assert_eq!(Action::Stop.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Action::Wait.args(u), vec!["task", "status", u, "waiting"]);
        // Stopped is the status that clears a wait date.
        assert_eq!(Action::Back.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Action::Remove.args(u), vec!["task", "status", u, "deleted", "--yes"]);
    }

    /// Start working is the menu's: worktree, herdr and Claude. Marking the
    /// task active alone would be `task status <uuid> active`, which is not it.
    #[test]
    fn start_never_only_marks_the_task_active() {
        assert!(!Action::Start.args("x").contains(&"active".to_string()));
    }
}
