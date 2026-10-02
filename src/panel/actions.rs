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
        Self::ROW
            .into_iter()
            .filter(|a| a.applies(state) && !(*a == Start && state.active))
            .collect()
    }

    /// The button Ctrl+Enter presses for a card: Refine until the task has a
    /// plan, then Start working. Nothing once a planned task is being
    /// worked, nor on a waiting task, which have no step to take.
    pub fn advance(state: TaskState) -> Option<Action> {
        match state {
            TaskState { waiting: true, .. } => None,
            TaskState { active: true, planned: true, .. } => None,
            TaskState { planned: true, .. } => Some(Start),
            _ => Some(Refine),
        }
    }

    /// Whether the panel keeps the keyboard after the button runs. Back to
    /// list, Up next, Waiting and Remove only change the task, and Speak
    /// plays in the background, so none opens anything that needs the
    /// keyboard and the list stays up; the rest open a box, a terminal or a
    /// menu, which takes it.
    pub fn keeps_keyboard(self) -> bool {
        matches!(self, Back | Speak | UpNext | Wait | Remove)
    }

    /// Whether the button takes its card off the list, so the focus has to
    /// move to a neighbour first. Speak's card stays where it is, and so does
    /// Up next's, which only moves up or down the list; the focus stays on the
    /// button, so a second press undoes it.
    pub fn leaves_the_list(self) -> bool {
        matches!(self, Back | Wait | Remove)
    }

    /// The button's CSS class, which `style::colour` gives its colour.
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

    /// The letter that presses the button while the panel has the keyboard.
    /// Not every button has one; Remove's key is Delete, which is not a
    /// letter and is `keys.rs`'s.
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

    /// The row's button `c` presses, if any.
    pub fn for_letter(c: char) -> Option<Action> {
        Self::ROW.into_iter().find(|a| a.letter() == Some(c))
    }
}

/// What the Waiting tab's Clear all reads, and what it reads between its first
/// press and its second, as Remove asks before it deletes.
pub const CLEAR_ALL: &str = "Clear all";
pub const CONFIRM_CLEAR_ALL: &str = "Confirm clear all";

/// The command Clear all spawns: Remove's own `task status <uuid> deleted
/// --yes` for each of `uuids`, one after another in one shell. So every task
/// goes the way Remove takes it, notification and all, and no two
/// taskwarrior writes race over `pending.data`. `exe` is niritasks itself,
/// passed as the shell's `$0`, and the uuids as its arguments, so neither is
/// ever quoted into the script; `$uuid` goes in bare, a uuid being hex and
/// dashes. `;`, not `&&`: one task that fails to delete does not keep the
/// rest.
pub fn clear_all_command(exe: &str, uuids: &[String]) -> Vec<String> {
    let remove = Action::Remove.args("$uuid").join(" ");
    let mut command = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("for uuid; do \"$0\" {remove}; done"),
        exe.to_string(),
    ];
    command.extend(uuids.iter().cloned());
    command
}

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
