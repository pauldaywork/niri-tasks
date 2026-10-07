//! What a keypress on the task panel means while it has the keyboard.
//!
//! Pure, as the task box's keys are, so the mapping is testable without a
//! window. What the key then does is `state.rs`'s; the controller's
//! propagation phase that lets it see the arrows first is `surface.rs`'s.

use crate::actions::Action;
use super::model::{Filter, Tab};
use gtk4::gdk;

/// What a keypress on the panel should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Give the keyboard back and tuck the panel away.
    Release,
    /// Up: the card above, landing on its body.
    PrevCard,
    /// Down: the card below, landing on its body. Reaching "+N more", it
    /// shows every card instead, landing on the first that was hidden.
    NextCard,
    /// Along the focused card: its body, then its buttons.
    PrevSlot,
    NextSlot,
    /// Press this button on the focused card, if it has one.
    Run(Action),
    /// 1 to 6: show this filter tab's cards, when it is shown. Its label
    /// says which ([`tab_number`]).
    Filter(Filter),
    /// 7: the Ideas tab, after the filter tabs.
    Ideas,
    /// [ and ]: the tab either side, Ideas the last, stopping at the ends as
    /// the cards do. Ctrl+[ and Ctrl+] on Ideas.
    PrevFilter,
    NextFilter,
    /// Ctrl+Enter: move the focused card's task on a step — refine it, or
    /// start working on it once it is planned — keeping the keyboard, so the
    /// list stays up to pick the next one.
    Advance,
    /// Ctrl+Delete: delete the focused card's task at once, with no Confirm
    /// remove, from whichever of its slots has the focus, keeping the
    /// keyboard. The stronger Delete, as Ctrl+Enter is the stronger Enter.
    Delete,
    /// Ctrl+Shift+Delete: press the Waiting tab's Clear all, which sits with
    /// the tabs outside the focus chain, so no focused button can stand for
    /// it. Stronger again than Ctrl+Delete, as it deletes every waiting task.
    ClearAll,
    /// Enter, which presses the focused button, as GTK does by itself; but
    /// with Clear all armed and no button focused, it confirms.
    Enter,
    /// Not the panel's key: Space presses the focused button, and the rest
    /// do nothing.
    Ignore,
}

/// Map a keypress to what it should do, off the Ideas tab (on it,
/// [`ideas_key_action`]). The letters work without a modifier, because
/// nothing on the cards takes typing. With Caps Lock on the keyval
/// arrives as a capital (`S`, not `s`), so the key is lowercased first and
/// capitals press the same buttons. Ctrl is the exception: Ctrl+Enter advances
/// the task, Ctrl+Delete deletes it and Ctrl+Shift+Delete clears the Waiting
/// tab, and every other Ctrl chord is left alone so Ctrl+T cannot Stop. Shift
/// matters only there.
pub fn key_action(key: gdk::Key, ctrl: bool, shift: bool) -> KeyAction {
    if ctrl {
        return match key {
            gdk::Key::Return | gdk::Key::KP_Enter => KeyAction::Advance,
            gdk::Key::Delete | gdk::Key::KP_Delete if shift => KeyAction::ClearAll,
            gdk::Key::Delete | gdk::Key::KP_Delete => KeyAction::Delete,
            _ => KeyAction::Ignore,
        };
    }
    match key.to_lower() {
        gdk::Key::Escape => KeyAction::Release,
        gdk::Key::Return | gdk::Key::KP_Enter => KeyAction::Enter,
        gdk::Key::Up => KeyAction::PrevCard,
        gdk::Key::Down => KeyAction::NextCard,
        gdk::Key::Left | gdk::Key::ISO_Left_Tab => KeyAction::PrevSlot,
        gdk::Key::Right | gdk::Key::Tab => KeyAction::NextSlot,
        gdk::Key::Delete | gdk::Key::KP_Delete => KeyAction::Run(Action::Remove),
        gdk::Key::_1 => KeyAction::Filter(Filter::TABS[0]),
        gdk::Key::_2 => KeyAction::Filter(Filter::TABS[1]),
        gdk::Key::_3 => KeyAction::Filter(Filter::TABS[2]),
        gdk::Key::_4 => KeyAction::Filter(Filter::TABS[3]),
        gdk::Key::_5 => KeyAction::Filter(Filter::TABS[4]),
        gdk::Key::_6 => KeyAction::Filter(Filter::TABS[5]),
        gdk::Key::_7 => KeyAction::Ideas,
        gdk::Key::bracketleft => KeyAction::PrevFilter,
        gdk::Key::bracketright => KeyAction::NextFilter,
        // A letter presses the row's button that has it.
        key => match key.to_unicode().and_then(Action::for_letter) {
            Some(action) => KeyAction::Run(action),
            None => KeyAction::Ignore,
        },
    }
}

/// The number key that picks this tab, which its label shows: 1 to 6 the
/// filter tabs in `Filter::TABS` order, 7 Ideas after them, as
/// [`key_action`] maps them. Fixed, so a hidden tab keeps its number and
/// the shown ones do not renumber.
pub fn tab_number(tab: Tab) -> u32 {
    match tab {
        Tab::Filter(filter) => {
            let index = Filter::TABS.iter().position(|f| *f == filter).expect("every filter is a tab");
            index as u32 + 1
        }
        Tab::Ideas => Filter::TABS.len() as u32 + 1,
    }
}

/// Map a keypress while the Ideas tab is picked, its text area having the
/// focus. Every key is typing, the letters, digits and brackets that press
/// buttons and pick tabs elsewhere included, but Escape, which saves and
/// goes back to the task list, and Ctrl+[ and Ctrl+], which switch tab as [
/// and ] do off it.
pub fn ideas_key_action(key: gdk::Key, ctrl: bool) -> KeyAction {
    match (key, ctrl) {
        (gdk::Key::Escape, _) => KeyAction::Release,
        (gdk::Key::bracketleft, true) => KeyAction::PrevFilter,
        (gdk::Key::bracketright, true) => KeyAction::NextFilter,
        _ => KeyAction::Ignore,
    }
}

/// One along a list of `len`, stopping at either end rather than wrapping: a
/// key held down settles on the last card instead of cycling past it.
pub fn step(index: usize, len: usize, forward: bool) -> usize {
    if forward {
        (index + 1).min(len.saturating_sub(1))
    } else {
        index.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_gives_the_keyboard_back() {
        assert_eq!(key_action(gdk::Key::Escape, false, false), KeyAction::Release);
    }

    #[test]
    fn up_and_down_move_between_cards() {
        assert_eq!(key_action(gdk::Key::Up, false, false), KeyAction::PrevCard);
        assert_eq!(key_action(gdk::Key::Down, false, false), KeyAction::NextCard);
    }

    #[test]
    fn left_right_and_tab_move_along_the_card() {
        assert_eq!(key_action(gdk::Key::Left, false, false), KeyAction::PrevSlot);
        assert_eq!(key_action(gdk::Key::Right, false, false), KeyAction::NextSlot);
        assert_eq!(key_action(gdk::Key::Tab, false, false), KeyAction::NextSlot);
        // Shift+Tab arrives as ISO_Left_Tab.
        assert_eq!(key_action(gdk::Key::ISO_Left_Tab, false, false), KeyAction::PrevSlot);
    }

    #[test]
    fn letters_and_delete_run_their_buttons() {
        assert_eq!(key_action(gdk::Key::g, false, false), KeyAction::Run(Action::Session));
        assert_eq!(key_action(gdk::Key::b, false, false), KeyAction::Run(Action::Back));
        assert_eq!(key_action(gdk::Key::s, false, false), KeyAction::Run(Action::Start));
        assert_eq!(key_action(gdk::Key::r, false, false), KeyAction::Run(Action::Refine));
        assert_eq!(key_action(gdk::Key::i, false, false), KeyAction::Run(Action::Grill));
        assert_eq!(key_action(gdk::Key::e, false, false), KeyAction::Run(Action::Edit));
        assert_eq!(key_action(gdk::Key::c, false, false), KeyAction::Run(Action::Complete));
        assert_eq!(key_action(gdk::Key::t, false, false), KeyAction::Run(Action::Stop));
        assert_eq!(key_action(gdk::Key::Delete, false, false), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::KP_Delete, false, false), KeyAction::Run(Action::Remove));
    }

    #[test]
    fn capitals_run_their_buttons_too() {
        assert_eq!(key_action(gdk::Key::G, false, false), KeyAction::Run(Action::Session));
        assert_eq!(key_action(gdk::Key::B, false, false), KeyAction::Run(Action::Back));
        assert_eq!(key_action(gdk::Key::S, false, false), KeyAction::Run(Action::Start));
        assert_eq!(key_action(gdk::Key::R, false, false), KeyAction::Run(Action::Refine));
        assert_eq!(key_action(gdk::Key::I, false, false), KeyAction::Run(Action::Grill));
        assert_eq!(key_action(gdk::Key::E, false, false), KeyAction::Run(Action::Edit));
        assert_eq!(key_action(gdk::Key::C, false, false), KeyAction::Run(Action::Complete));
        assert_eq!(key_action(gdk::Key::T, false, false), KeyAction::Run(Action::Stop));
    }

    /// Enter is its own, for an armed Clear all to confirm on; Space and the
    /// rest are not the panel's.
    #[test]
    fn enter_is_enter_and_space_passes_through() {
        assert_eq!(key_action(gdk::Key::Return, false, false), KeyAction::Enter);
        assert_eq!(key_action(gdk::Key::KP_Enter, false, false), KeyAction::Enter);
        for key in [gdk::Key::space, gdk::Key::x] {
            assert_eq!(key_action(key, false, false), KeyAction::Ignore, "{key:?}");
        }
    }

    /// Ctrl+Enter moves the task on without leaving the list; plain Enter
    /// still presses the focused button.
    #[test]
    fn ctrl_enter_advances_the_focused_task() {
        assert_eq!(key_action(gdk::Key::Return, true, false), KeyAction::Advance);
        assert_eq!(key_action(gdk::Key::KP_Enter, true, false), KeyAction::Advance);
        assert_eq!(key_action(gdk::Key::Return, false, false), KeyAction::Enter);
    }

    /// Every other Ctrl chord belongs to the compositor and the focused
    /// widget: Ctrl+T must not Stop, nor Ctrl+1 change tab.
    #[test]
    fn other_ctrl_chords_pass_through() {
        for key in [gdk::Key::t, gdk::Key::r, gdk::Key::s, gdk::Key::_1, gdk::Key::Escape, gdk::Key::Down] {
            assert_eq!(key_action(key, true, false), KeyAction::Ignore, "{key:?}");
        }
    }

    #[test]
    fn step_moves_one_and_stops_at_the_ends() {
        assert_eq!(step(0, 3, true), 1);
        assert_eq!(step(2, 3, true), 2);
        assert_eq!(step(1, 3, false), 0);
        assert_eq!(step(0, 3, false), 0);
        assert_eq!(step(0, 0, true), 0);
    }

    /// 1 to 6 are the tabs left to right.
    #[test]
    fn numbers_pick_the_tabs_in_order() {
        let numbers = [gdk::Key::_1, gdk::Key::_2, gdk::Key::_3, gdk::Key::_4, gdk::Key::_5, gdk::Key::_6];
        for (key, filter) in numbers.into_iter().zip(Filter::TABS) {
            assert_eq!(key_action(key, false, false), KeyAction::Filter(filter), "{key:?}");
        }
    }

    #[test]
    fn brackets_step_between_tabs() {
        assert_eq!(key_action(gdk::Key::bracketleft, false, false), KeyAction::PrevFilter);
        assert_eq!(key_action(gdk::Key::bracketright, false, false), KeyAction::NextFilter);
    }

    /// 7 is the Ideas tab, after the six filter tabs.
    #[test]
    fn seven_picks_ideas() {
        assert_eq!(key_action(gdk::Key::_7, false, false), KeyAction::Ideas);
    }

    /// The number on a tab's label is the key that picks it: 1 to 6 the
    /// filter tabs left to right, 7 Ideas.
    #[test]
    fn each_tabs_number_is_the_key_that_picks_it() {
        let numbers: Vec<u32> = Filter::TABS.into_iter().map(|f| tab_number(Tab::Filter(f))).collect();
        assert_eq!(numbers, [1, 2, 3, 4, 5, 6]);
        assert_eq!(tab_number(Tab::Ideas), 7);
        for filter in Filter::TABS {
            let key = gdk::Key::from_name(tab_number(Tab::Filter(filter)).to_string()).unwrap();
            assert_eq!(key_action(key, false, false), KeyAction::Filter(filter), "{filter:?}");
        }
        let key = gdk::Key::from_name(tab_number(Tab::Ideas).to_string()).unwrap();
        assert_eq!(key_action(key, false, false), KeyAction::Ideas);
    }

    /// Past the seven tabs a number is nothing, not an eighth tab.
    #[test]
    fn other_numbers_pass_through() {
        for key in [gdk::Key::_0, gdk::Key::_8, gdk::Key::_9] {
            assert_eq!(key_action(key, false, false), KeyAction::Ignore, "{key:?}");
        }
    }

    /// On Ideas the text area takes every key as typing: the letters,
    /// digits and brackets that press buttons and pick tabs elsewhere, Enter,
    /// Tab, the arrows, Delete, and the Ctrl chords the cards use.
    #[test]
    fn on_ideas_every_other_key_is_typing() {
        for key in [
            gdk::Key::s,
            gdk::Key::S,
            gdk::Key::_1,
            gdk::Key::_6,
            gdk::Key::_7,
            gdk::Key::bracketleft,
            gdk::Key::bracketright,
            gdk::Key::Return,
            gdk::Key::Tab,
            gdk::Key::Up,
            gdk::Key::Down,
            gdk::Key::Left,
            gdk::Key::Delete,
            gdk::Key::space,
        ] {
            assert_eq!(ideas_key_action(key, false), KeyAction::Ignore, "{key:?}");
        }
        for key in [gdk::Key::Return, gdk::Key::Delete, gdk::Key::t, gdk::Key::a] {
            assert_eq!(ideas_key_action(key, true), KeyAction::Ignore, "Ctrl+{key:?}");
        }
    }

    /// Escape still leaves the text area, and Ctrl+[ and Ctrl+] switch tab,
    /// as [ and ] do off Ideas.
    #[test]
    fn on_ideas_escape_gives_back_and_ctrl_brackets_switch_tab() {
        assert_eq!(ideas_key_action(gdk::Key::Escape, false), KeyAction::Release);
        assert_eq!(ideas_key_action(gdk::Key::bracketleft, true), KeyAction::PrevFilter);
        assert_eq!(ideas_key_action(gdk::Key::bracketright, true), KeyAction::NextFilter);
    }

    /// Ctrl+Delete deletes the focused task, the stronger Delete as
    /// Ctrl+Enter is the stronger Enter; Ctrl+Shift+Delete, stronger again,
    /// is the Waiting tab's Clear all. Delete alone still arms Remove.
    #[test]
    fn ctrl_delete_deletes_and_ctrl_shift_delete_clears_all() {
        for key in [gdk::Key::Delete, gdk::Key::KP_Delete] {
            assert_eq!(key_action(key, true, false), KeyAction::Delete, "{key:?}");
            assert_eq!(key_action(key, true, true), KeyAction::ClearAll, "{key:?}");
            assert_eq!(key_action(key, false, false), KeyAction::Run(Action::Remove), "{key:?}");
        }
    }

    /// Shift only tells Ctrl+Delete from Ctrl+Shift+Delete: Shift+Delete is
    /// still Remove, and Ctrl+Shift+Enter still advances, as it always has.
    #[test]
    fn shift_alone_changes_nothing() {
        assert_eq!(key_action(gdk::Key::Delete, false, true), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::Return, true, true), KeyAction::Advance);
        assert_eq!(key_action(gdk::Key::Escape, false, true), KeyAction::Release);
    }
}
