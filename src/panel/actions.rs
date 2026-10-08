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
    /// The row's buttons, left to right: every task action, the row being
    /// the only view of them.
    pub const ROW: [Action; 13] = Self::ALL;

    /// What Remove reads between its first press and its second.
    pub const CONFIRM_REMOVE: &'static str = "Confirm remove";

    /// What Speak shows in place of its speaker while the speech is being got
    /// ready, one frame after another: braille dots going round. Iosevka has
    /// them, so they sit centred in the cell like the text around them.
    pub const SPINNER: [&'static str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    /// The buttons a card in `state` gets, left to right: the row's actions
    /// that apply. An active task gets Stop in the place of Start working,
    /// which would only go back to its worktree; Go to session goes back to
    /// its Claude.
    pub fn row(state: TaskState) -> Vec<Action> {
        Self::ROW
            .into_iter()
            .filter(|a| a.applies(state) && !(*a == Start && state.active))
            .collect()
    }

    /// The button Ctrl+Enter presses for a card: Refine until the task has a
    /// plan, then Start working. Nothing once a planned task is being
    /// worked, nor on a waiting or finished task, which have no step to take.
    pub fn advance(state: TaskState) -> Option<Action> {
        match state {
            TaskState { finished: true, .. } => None,
            TaskState { waiting: true, .. } => None,
            TaskState { active: true, planned: true, .. } => None,
            TaskState { planned: true, .. } => Some(Start),
            _ => Some(Refine),
        }
    }

    /// What the focused card's hint reads beside its buttons: the keys that
    /// change from card to card and button to button. The focused button's
    /// key and name ("g: Go to session", "Del: Remove", or just "Speak" for
    /// a button only Enter presses), or, on the body of a card with notes,
    /// what Space does to them; then what Ctrl+Enter does to this task, when
    /// it presses a button `row` has. `notes` is whether the card's notes
    /// show, None on a card with none. Empty on the body of a task with no
    /// notes that Ctrl+Enter leaves alone. Enter and Ctrl+Delete do the same
    /// on every card, so they are the footer's, [`CARD_KEYS`]. Pure, so every
    /// card's hint is tested without a window; `surface.rs` only sets the
    /// label.
    pub fn hint(state: TaskState, row: &[Action], focused: Option<Action>, notes: Option<bool>) -> String {
        let mut parts = Vec::new();
        match (focused, notes) {
            (Some(action), _) => {
                let name = action.label(state.up_next);
                parts.push(match action.key() {
                    Some(key) => format!("{key}: {name}"),
                    None => name.to_string(),
                });
            }
            // Space, not Enter: Enter is the footer's, and either presses
            // the body.
            (None, Some(shown)) => parts.push(format!("Space: {} notes", if shown { "hide" } else { "view" })),
            (None, None) => {}
        }
        if let Some(step) = Self::advance(state).filter(|a| row.contains(a)) {
            let does = if step == Refine { "refine" } else { "start working" };
            parts.push(format!("Ctrl+Enter: {does}"));
        }
        parts.join(" · ")
    }

    /// The key that presses the button, as the hint names it: its letter, or
    /// Del for Remove. None for a button only Enter presses.
    fn key(self) -> Option<String> {
        match self {
            Remove => Some("Del".to_string()),
            _ => self.letter().map(String::from),
        }
    }

    /// Whether the panel keeps the keyboard after the button runs. Back to
    /// list, Up next, Complete, Waiting and Remove only change the task, and
    /// Speak plays in the background, so none opens anything that needs the
    /// keyboard and the list stays up. Move opens the project list in the
    /// panel itself. The rest open a box, a terminal or a herdr tab, which
    /// takes it.
    pub fn keeps_keyboard(self) -> bool {
        matches!(self, Back | Speak | UpNext | Complete | Move | Wait | Remove)
    }

    /// Whether the button takes its card off the list, so the focus has to
    /// move to a neighbour first. Speak's card stays where it is, and so does
    /// Up next's, which only moves up or down the list; the focus stays on the
    /// button, so a second press undoes it.
    pub fn leaves_the_list(self) -> bool {
        matches!(self, Back | Complete | Wait | Remove)
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
            Speak => "speak",
            UpNext => "up-next",
            Complete => "complete",
            Move => "move",
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
            Grill => Some('i'),
            Edit => Some('e'),
            Complete => Some('c'),
            Move => Some('m'),
            Stop => Some('t'),
            Speak | UpNext | Wait | Remove => None,
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

/// The footer under the keyboard's list: the keys that act on whichever card
/// has the focus, the same on every one, so the card's own hint leaves them
/// out. Enter or Space on a button presses it; on the card's body it shows
/// the task's notes, or hides them.
pub const CARD_KEYS: &str = "Enter/Space: press the button or show notes · Ctrl+Del: delete the task";

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

    fn finished(planned: bool) -> TaskState {
        TaskState { finished: true, planned, ..TaskState::default() }
    }

    fn with_claude(state: TaskState) -> TaskState {
        TaskState { has_session: true, ..state }
    }

    /// Every state a task card can be in, with a Claude on it and without.
    fn every_state() -> Vec<TaskState> {
        let mut all = Vec::new();
        for planned in [false, true] {
            for state in [on_list(planned), active(planned), waiting(planned), finished(planned)] {
                all.push(state);
                all.push(with_claude(state));
            }
        }
        all
    }

    /// Refine until the task has a plan, then Start working; nothing once a
    /// planned task is being worked, nor on a waiting or finished task.
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
        assert_eq!(classes, vec!["refine", "grill", "edit", "speak", "up-next", "complete", "move", "stop", "wait", "remove"]);
    }

    #[test]
    fn a_task_not_yet_active_gets_start_and_no_stop() {
        for planned in [false, true] {
            assert_eq!(
                Action::row(on_list(planned)),
                vec![Start, Refine, Grill, Edit, Speak, UpNext, Complete, Move, Wait, Remove],
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
            vec![Session, Refine, Grill, Edit, Speak, UpNext, Complete, Move, Stop, Wait, Remove]
        );
        // A refine open on a task not yet started.
        assert_eq!(
            Action::row(with_claude(on_list(true))),
            vec![Session, Start, Refine, Grill, Edit, Speak, UpNext, Complete, Move, Wait, Remove]
        );
    }

    /// A waiting task gets the way back, and what still works on it, a
    /// Claude or not.
    #[test]
    fn a_waiting_task_gets_back_to_list_and_what_still_works_on_it() {
        for state in [waiting(false), with_claude(waiting(true))] {
            assert_eq!(Action::row(state), vec![Back, Edit, Speak, Complete, Move, Remove], "{state:?}");
        }
    }

    /// A finished task gets the way back, and what still works on it, as a
    /// waiting one does, less Complete: it is done already.
    #[test]
    fn a_finished_task_gets_back_to_list_edit_speak_and_remove() {
        for state in [finished(false), with_claude(finished(true))] {
            assert_eq!(Action::row(state), vec![Back, Edit, Speak, Remove], "{state:?}");
        }
    }

    /// Nothing to refine or start on a task already done.
    #[test]
    fn ctrl_enter_leaves_a_finished_task_alone() {
        assert_eq!(Action::advance(finished(false)), None);
        assert_eq!(Action::advance(finished(true)), None);
        assert_eq!(hint(finished(false), None), "");
        assert_eq!(hint(finished(false), Some(Back)), "b: Back to list");
    }

    /// Every card that stands for a task can be listened to, whatever its
    /// state.
    #[test]
    fn every_task_card_gets_speak() {
        for state in every_state() {
            assert!(Action::row(state).contains(&Speak), "{state:?}");
        }
    }

    /// Every card on the list gets Up next, active ones too; a waiting or
    /// finished task is off the list.
    #[test]
    fn every_card_on_the_list_gets_up_next() {
        for state in every_state() {
            assert_eq!(Action::row(state).contains(&UpNext), !state.off_list(), "{state:?}");
        }
    }

    /// Grill me goes where Refine goes: every card on the list, not a
    /// waiting or finished one.
    #[test]
    fn every_card_on_the_list_gets_grill_me() {
        for state in every_state() {
            assert_eq!(Action::row(state).contains(&Grill), !state.off_list(), "{state:?}");
        }
    }

    /// Every card can be completed, a waiting one too, and moved to another
    /// project's workspace; a finished one is done already, and stays put.
    #[test]
    fn every_card_but_a_finished_one_gets_complete_and_move() {
        for state in every_state() {
            assert_eq!(Action::row(state).contains(&Complete), !state.finished, "{state:?}");
            assert_eq!(Action::row(state).contains(&Move), !state.finished, "{state:?}");
        }
    }

    /// Speak and Up next open nothing, so the list stays up, as it does for
    /// the buttons that only change the task. Move opens the project list in
    /// the panel itself, so it keeps the keyboard with the buttons that only
    /// change the task.
    #[test]
    fn the_buttons_that_open_nothing_keep_the_list_open() {
        let kept: Vec<Action> = Action::ROW.into_iter().filter(|a| a.keeps_keyboard()).collect();
        assert_eq!(kept, vec![Back, Speak, UpNext, Complete, Move, Wait, Remove]);
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
    fn back_complete_wait_and_remove_take_their_card_off_the_list() {
        let leaving: Vec<Action> = Action::ROW.into_iter().filter(|a| a.leaves_the_list()).collect();
        assert_eq!(leaving, vec![Back, Complete, Wait, Remove]);
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

    /// g b s r i e c m t press Go to session, Back to list, Start working,
    /// Refine, Grill me, Edit, Complete, Move to workspace and Stop, and each
    /// letter finds its own button.
    #[test]
    fn letters_press_their_own_buttons() {
        let lettered: Vec<(char, Action)> =
            Action::ROW.into_iter().filter_map(|a| a.letter().map(|c| (c, a))).collect();
        assert_eq!(
            lettered,
            vec![
                ('g', Session), ('b', Back), ('s', Start), ('r', Refine), ('i', Grill), ('e', Edit),
                ('c', Complete), ('m', Move), ('t', Stop)
            ]
        );
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

    fn hint(state: TaskState, focused: Option<Action>) -> String {
        Action::hint(state, &Action::row(state), focused, None)
    }

    /// On the body of a card with notes, the hint names what Space does to
    /// them, before what Ctrl+Enter does; a button's hint leaves them out.
    #[test]
    fn the_body_of_a_card_with_notes_hints_space() {
        let row = Action::row(on_list(false));
        assert_eq!(Action::hint(on_list(false), &row, None, Some(false)), "Space: view notes · Ctrl+Enter: refine");
        assert_eq!(Action::hint(on_list(false), &row, None, Some(true)), "Space: hide notes · Ctrl+Enter: refine");
        assert_eq!(Action::hint(active(true), &Action::row(active(true)), None, Some(false)), "Space: view notes");
        assert_eq!(Action::hint(on_list(false), &row, Some(Edit), Some(true)), "e: Edit · Ctrl+Enter: refine");
    }

    /// Up and Down land on the body: what Ctrl+Enter does to this task.
    #[test]
    fn the_body_hints_what_ctrl_enter_does() {
        assert_eq!(hint(on_list(false), None), "Ctrl+Enter: refine");
        assert_eq!(hint(active(false), None), "Ctrl+Enter: refine");
        assert_eq!(hint(on_list(true), None), "Ctrl+Enter: start working");
    }

    #[test]
    fn the_body_of_a_task_ctrl_enter_leaves_alone_hints_nothing() {
        assert_eq!(hint(active(true), None), "");
        assert_eq!(hint(waiting(false), None), "");
        assert_eq!(hint(with_claude(waiting(true)), None), "");
    }

    #[test]
    fn a_lettered_button_hints_its_letter_and_name() {
        assert_eq!(hint(with_claude(on_list(true)), Some(Session)), "g: Go to session · Ctrl+Enter: start working");
        assert_eq!(hint(on_list(true), Some(Start)), "s: Start working · Ctrl+Enter: start working");
        assert_eq!(hint(on_list(false), Some(Refine)), "r: Refine · Ctrl+Enter: refine");
        assert_eq!(hint(active(false), Some(Edit)), "e: Edit · Ctrl+Enter: refine");
        assert_eq!(hint(on_list(false), Some(Grill)), "i: Grill me · Ctrl+Enter: refine");
        assert_eq!(hint(on_list(true), Some(Complete)), "c: Complete · Ctrl+Enter: start working");
        assert_eq!(hint(on_list(true), Some(Move)), "m: Move to workspace · Ctrl+Enter: start working");
        assert_eq!(hint(active(true), Some(Stop)), "t: Stop");
        assert_eq!(hint(waiting(false), Some(Back)), "b: Back to list");
    }

    /// Remove's key is Delete, not a letter.
    #[test]
    fn remove_hints_del() {
        assert_eq!(hint(on_list(false), Some(Remove)), "Del: Remove · Ctrl+Enter: refine");
        assert_eq!(hint(waiting(true), Some(Remove)), "Del: Remove");
    }

    /// Only Enter presses Speak, Up next and Waiting, and Enter is the
    /// footer's, so they hint their name alone. Up next reads as it does on
    /// its tooltip.
    #[test]
    fn a_button_with_no_key_hints_its_name_alone() {
        assert_eq!(hint(on_list(false), Some(Speak)), "Speak · Ctrl+Enter: refine");
        assert_eq!(hint(on_list(true), Some(Wait)), "Waiting · Ctrl+Enter: start working");
        let up_next = TaskState { up_next: true, ..on_list(true) };
        assert_eq!(hint(up_next, Some(UpNext)), "Not up next · Ctrl+Enter: start working");
    }

    /// Ctrl+Enter is left out when what it would press is not on the card.
    #[test]
    fn ctrl_enter_is_left_out_when_the_card_lacks_its_button() {
        assert_eq!(Action::hint(on_list(false), &[Edit, Remove], None, None), "");
        assert_eq!(Action::hint(on_list(true), &[Edit, Remove], Some(Edit), None), "e: Edit");
    }

    /// Every state, up next or not, with the body (with no notes, and with
    /// notes hidden and shown) and with each of its buttons focused.
    fn every_hint() -> Vec<(TaskState, Option<Action>, String)> {
        let mut all = Vec::new();
        for state in every_state() {
            for up_next in [false, true] {
                let state = TaskState { up_next, ..state };
                let row = Action::row(state);
                for focused in std::iter::once(None).chain(row.iter().copied().map(Some)) {
                    for notes in [None, Some(false), Some(true)] {
                        all.push((state, focused, Action::hint(state, &row, focused, notes)));
                    }
                }
            }
        }
        all
    }

    /// Enter and Ctrl+Delete do the same on every card, so they are the
    /// footer's, and the card's hint leaves them out.
    #[test]
    fn the_hint_leaves_enter_and_ctrl_delete_to_the_footer() {
        for (state, focused, text) in every_hint() {
            assert!(!text.contains("Ctrl+Del"), "{state:?} {focused:?}: {text}");
            assert!(!text.split(" · ").any(|part| part.starts_with("Enter")), "{state:?} {focused:?}: {text}");
        }
        assert_eq!(CARD_KEYS, "Enter/Space: press the button or show notes · Ctrl+Del: delete the task");
    }

    /// Room for the hint beside a row of `buttons`, in characters at the
    /// hint's 9pt, where Iosevka Term Extended is 7px a character: the 760px
    /// card, less each button's one 8px glyph and 24px of padding, a 1px
    /// line between each two, and the hint's own 24px of padding.
    fn hint_room(buttons: usize) -> usize {
        (760 - buttons * 32 - (buttons - 1) - 24) / 7
    }

    /// The most buttons any card gets.
    fn widest_row() -> usize {
        every_state().into_iter().map(|s| Action::row(s).len()).max().unwrap()
    }

    #[test]
    fn every_hint_fits_beside_its_row() {
        for (state, focused, text) in every_hint() {
            let room = hint_room(Action::row(state).len());
            assert!(text.chars().count() <= room, "{state:?} {focused:?}: {text:?} is {} long, room {room}", text.chars().count());
        }
    }

    /// Armed, Remove reads "  Confirm remove" (16 characters at the
    /// buttons' 10pt, 128px), and the surface leaves the hint blank: the
    /// button says what Enter does. The widest row still fits the card.
    #[test]
    fn confirm_remove_fits_on_the_widest_row() {
        let n = widest_row();
        assert!(n * 32 + (n - 1) + 128 <= 760, "{n} buttons");
    }
}
