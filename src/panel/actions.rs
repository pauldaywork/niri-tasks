//! The buttons on a task card while the panel has the keyboard, and what each
//! one runs.
//!
//! Plain data, like `model.rs`, so which card gets which buttons and what they
//! spawn is testable without a compositor. Each runs the same `niritasks`
//! command as its entry in the fuzzel menu, so a button cannot drift from the
//! menu entry it stands in for.

use super::model::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Refine,
    Edit,
    Stop,
    Remove,
}

use Action::*;

impl Action {
    /// In the order the buttons sit, left to right.
    pub const ALL: [Action; 5] = [Start, Refine, Edit, Stop, Remove];

    /// What Remove reads between its first press and its second, the way the
    /// menu's delete asks "delete?" before it deletes.
    pub const CONFIRM_REMOVE: &'static str = "Confirm remove";

    /// The buttons a card gets, left to right. Stop only on an active task,
    /// since there is nothing to stop on the rest; none on "+N more", which
    /// stands for no one task.
    pub fn for_status(status: Status) -> Vec<Action> {
        match status {
            Status::More => Vec::new(),
            Status::Active => Self::ALL.to_vec(),
            Status::Pending | Status::Blocked | Status::Planned => {
                Self::ALL.into_iter().filter(|a| *a != Stop).collect()
            }
        }
    }

    /// The button's text: the menu's own word for it.
    pub fn label(self) -> &'static str {
        match self {
            Start => "Start working",
            Refine => "Refine",
            Edit => "Edit",
            Stop => "Stop",
            Remove => "Remove",
        }
    }

    /// The button's CSS class, which gives it its colour, and its widget name,
    /// which lets a re-render put focus back on the same button.
    pub fn name(self) -> &'static str {
        match self {
            Start => "start",
            Refine => "refine",
            Edit => "edit",
            Stop => "stop",
            Remove => "remove",
        }
    }

    /// The `niritasks` arguments the button runs, without the program: what
    /// its menu entry runs. Remove carries `--yes` because its second press is
    /// the confirmation.
    pub fn args(self, uuid: &str) -> Vec<String> {
        let words: &[&str] = match self {
            Start => &["task", "start", uuid],
            Refine => &["task", "refine", uuid],
            Edit => &["task", "edit", uuid],
            Stop => &["task", "status", uuid, "stopped"],
            Remove => &["task", "status", uuid, "deleted", "--yes"],
        };
        words.iter().map(|w| w.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_active_task_gets_every_button_in_order() {
        let names: Vec<&str> = Action::for_status(Status::Active).iter().map(|a| a.name()).collect();
        assert_eq!(names, vec!["start", "refine", "edit", "stop", "remove"]);
    }

    #[test]
    fn stop_is_only_on_an_active_task() {
        for status in [Status::Pending, Status::Blocked, Status::Planned] {
            let got = Action::for_status(status);
            assert_eq!(got, vec![Action::Start, Action::Refine, Action::Edit, Action::Remove], "{status:?}");
        }
    }

    #[test]
    fn more_is_no_one_task_and_gets_no_buttons() {
        assert!(Action::for_status(Status::More).is_empty());
    }

    #[test]
    fn labels_read_as_the_menu_does() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label()).collect();
        assert_eq!(labels, vec!["Start working", "Refine", "Edit", "Stop", "Remove"]);
        assert_eq!(Action::CONFIRM_REMOVE, "Confirm remove");
    }

    #[test]
    fn each_button_runs_its_menu_entrys_command() {
        let u = "c53b6e3d-ca05-4aae-8588-4ee1abc25f5b";
        assert_eq!(Action::Start.args(u), vec!["task", "start", u]);
        assert_eq!(Action::Refine.args(u), vec!["task", "refine", u]);
        assert_eq!(Action::Edit.args(u), vec!["task", "edit", u]);
        assert_eq!(Action::Stop.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Action::Remove.args(u), vec!["task", "status", u, "deleted", "--yes"]);
    }

    /// Start working is the menu's: worktree, herdr and Claude. Marking the
    /// task active alone would be `task status <uuid> active`, which is not it.
    #[test]
    fn start_never_only_marks_the_task_active() {
        assert!(!Action::Start.args("x").contains(&"active".to_string()));
    }
}
