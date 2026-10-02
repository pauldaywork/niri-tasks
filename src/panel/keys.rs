//! What a keypress on the task panel means while it has the keyboard.
//!
//! Pure, as the task box's keys are, so the mapping is testable without a
//! window. Acting on it, and the controller's propagation phase that lets it
//! see the arrows first, are `surface.rs`'s.

use super::actions::Action;
use super::model::Filter;
use gtk4::gdk;

/// What a keypress on the panel should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Give the keyboard back and tuck the panel away.
    Release,
    /// Up: the card above, landing on its body.
    PrevCard,
    /// Down: the card below, landing on its body.
    NextCard,
    /// Along the focused card: its body, then its buttons.
    PrevSlot,
    NextSlot,
    /// Press this button on the focused card, if it has one.
    Run(Action),
    /// 1 to 5: show this filter tab's cards, when it is shown.
    Filter(Filter),
    /// [ and ]: the tab either side, stopping at the ends as the cards do.
    PrevFilter,
    NextFilter,
    /// Ctrl+Enter: move the focused card's task on a step — refine it, or
    /// start working on it once it is planned — keeping the keyboard, so the
    /// list stays up to pick the next one.
    Advance,
    /// Shift+Delete: press the Waiting tab's Clear all, which sits with the
    /// tabs outside the focus chain, so no focused button can stand for it.
    ClearAll,
    /// Pass it through: Enter and Space press the focused button.
    Ignore,
}

/// Map a keypress to what it should do. The letters work without a modifier,
/// because nothing on the panel takes typing. With Caps Lock on the keyval
/// arrives as a capital (`S`, not `s`), so the key is lowercased first and
/// capitals press the same buttons. Ctrl is the exception: Ctrl+Enter advances
/// the task, and every other Ctrl chord is left alone so Ctrl+T cannot Stop.
/// `shift` matters to Delete alone, making it Clear all: Shift+Tab already
/// arrives as ISO_Left_Tab, and a Shift+letter as its capital.
pub fn key_action(key: gdk::Key, ctrl: bool, shift: bool) -> KeyAction {
    if ctrl {
        return match key {
            gdk::Key::Return | gdk::Key::KP_Enter => KeyAction::Advance,
            _ => KeyAction::Ignore,
        };
    }
    match key.to_lower() {
        gdk::Key::Escape => KeyAction::Release,
        gdk::Key::Up => KeyAction::PrevCard,
        gdk::Key::Down => KeyAction::NextCard,
        gdk::Key::Left | gdk::Key::ISO_Left_Tab => KeyAction::PrevSlot,
        gdk::Key::Right | gdk::Key::Tab => KeyAction::NextSlot,
        gdk::Key::g => KeyAction::Run(Action::Session),
        gdk::Key::b => KeyAction::Run(Action::Back),
        gdk::Key::s => KeyAction::Run(Action::Start),
        gdk::Key::r => KeyAction::Run(Action::Refine),
        gdk::Key::e => KeyAction::Run(Action::Edit),
        gdk::Key::t => KeyAction::Run(Action::Stop),
        gdk::Key::Delete | gdk::Key::KP_Delete if shift => KeyAction::ClearAll,
        gdk::Key::Delete | gdk::Key::KP_Delete => KeyAction::Run(Action::Remove),
        gdk::Key::_1 => KeyAction::Filter(Filter::TABS[0]),
        gdk::Key::_2 => KeyAction::Filter(Filter::TABS[1]),
        gdk::Key::_3 => KeyAction::Filter(Filter::TABS[2]),
        gdk::Key::_4 => KeyAction::Filter(Filter::TABS[3]),
        gdk::Key::_5 => KeyAction::Filter(Filter::TABS[4]),
        gdk::Key::bracketleft => KeyAction::PrevFilter,
        gdk::Key::bracketright => KeyAction::NextFilter,
        _ => KeyAction::Ignore,
    }
}

/// What a keypress does while Clear all is armed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Armed {
    /// Press it the second time: delete.
    Confirm,
    /// Put it back, and the focus on the card it was taken from.
    Cancel,
    /// Act on it as ever: a tab key, whose re-render puts Clear all back.
    Pass(KeyAction),
    /// Nothing. A card's key, with no card focused to act on.
    Swallow,
}

/// Map a keypress while Clear all is armed. Arming takes the focus off the
/// cards, so Enter cannot open a card's menu and confirms instead, as the
/// second Shift+Delete does. Escape and the keys that move the focus cancel,
/// Escape without giving the keyboard back. A card's keys are swallowed rather
/// than left to fall back on the first card, which no one picked.
pub fn while_clear_armed(key: gdk::Key, ctrl: bool, shift: bool) -> Armed {
    if !ctrl && matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
        return Armed::Confirm;
    }
    match key_action(key, ctrl, shift) {
        KeyAction::ClearAll => Armed::Confirm,
        KeyAction::Release
        | KeyAction::PrevCard
        | KeyAction::NextCard
        | KeyAction::PrevSlot
        | KeyAction::NextSlot => Armed::Cancel,
        action @ (KeyAction::Filter(_) | KeyAction::PrevFilter | KeyAction::NextFilter) => {
            Armed::Pass(action)
        }
        KeyAction::Run(_) | KeyAction::Advance | KeyAction::Ignore => Armed::Swallow,
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
        assert_eq!(key_action(gdk::Key::e, false, false), KeyAction::Run(Action::Edit));
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
        assert_eq!(key_action(gdk::Key::E, false, false), KeyAction::Run(Action::Edit));
        assert_eq!(key_action(gdk::Key::T, false, false), KeyAction::Run(Action::Stop));
    }

    /// Enter and Space press the focused button, which GTK does itself.
    #[test]
    fn enter_and_space_pass_through() {
        for key in [gdk::Key::Return, gdk::Key::KP_Enter, gdk::Key::space, gdk::Key::x] {
            assert_eq!(key_action(key, false, false), KeyAction::Ignore, "{key:?}");
        }
    }

    /// Ctrl+Enter moves the task on without leaving the list; plain Enter
    /// still presses the focused button.
    #[test]
    fn ctrl_enter_advances_the_focused_task() {
        assert_eq!(key_action(gdk::Key::Return, true, false), KeyAction::Advance);
        assert_eq!(key_action(gdk::Key::KP_Enter, true, false), KeyAction::Advance);
        assert_eq!(key_action(gdk::Key::Return, false, false), KeyAction::Ignore);
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

    /// 1 to 5 are the tabs left to right.
    #[test]
    fn numbers_pick_the_tabs_in_order() {
        let numbers = [gdk::Key::_1, gdk::Key::_2, gdk::Key::_3, gdk::Key::_4, gdk::Key::_5];
        for (key, filter) in numbers.into_iter().zip(Filter::TABS) {
            assert_eq!(key_action(key, false, false), KeyAction::Filter(filter), "{key:?}");
        }
    }

    #[test]
    fn brackets_step_between_tabs() {
        assert_eq!(key_action(gdk::Key::bracketleft, false, false), KeyAction::PrevFilter);
        assert_eq!(key_action(gdk::Key::bracketright, false, false), KeyAction::NextFilter);
    }

    /// Past the five tabs a number is nothing, not a sixth tab.
    #[test]
    fn other_numbers_pass_through() {
        for key in [gdk::Key::_0, gdk::Key::_6, gdk::Key::_9] {
            assert_eq!(key_action(key, false, false), KeyAction::Ignore, "{key:?}");
        }
    }

    /// Shift+Delete is Clear all; Delete alone still removes the focused card.
    #[test]
    fn shift_delete_clears_all_and_delete_alone_removes() {
        assert_eq!(key_action(gdk::Key::Delete, false, true), KeyAction::ClearAll);
        assert_eq!(key_action(gdk::Key::KP_Delete, false, true), KeyAction::ClearAll);
        assert_eq!(key_action(gdk::Key::Delete, false, false), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::KP_Delete, false, false), KeyAction::Run(Action::Remove));
    }

    /// Shift changes nothing else: a capital and Shift+Tab already carry it.
    #[test]
    fn shift_leaves_every_other_key_alone() {
        for key in [gdk::Key::S, gdk::Key::Escape, gdk::Key::ISO_Left_Tab, gdk::Key::Down, gdk::Key::bracketright] {
            assert_eq!(key_action(key, false, true), key_action(key, false, false), "{key:?}");
        }
    }

    /// Armed, Clear all has the keyboard to itself: Enter confirms as Shift+Delete
    /// does, since no card has focus for Enter to open.
    #[test]
    fn enter_and_shift_delete_confirm_an_armed_clear_all() {
        for key in [gdk::Key::Return, gdk::Key::KP_Enter] {
            assert_eq!(while_clear_armed(key, false, false), Armed::Confirm, "{key:?}");
        }
        assert_eq!(while_clear_armed(gdk::Key::Delete, false, true), Armed::Confirm);
    }

    /// Escape and the keys that move the focus take it back, without closing
    /// the panel or moving on past the card it was on.
    #[test]
    fn escape_and_moving_cancel_an_armed_clear_all() {
        for key in [gdk::Key::Escape, gdk::Key::Up, gdk::Key::Down, gdk::Key::Left, gdk::Key::Right, gdk::Key::Tab, gdk::Key::ISO_Left_Tab] {
            assert_eq!(while_clear_armed(key, false, false), Armed::Cancel, "{key:?}");
        }
    }

    /// A tab key switches tab as ever, which puts Clear all back on its way.
    #[test]
    fn the_tab_keys_still_switch_tab_while_armed() {
        assert_eq!(while_clear_armed(gdk::Key::_1, false, false), Armed::Pass(KeyAction::Filter(Filter::All)));
        assert_eq!(while_clear_armed(gdk::Key::bracketleft, false, false), Armed::Pass(KeyAction::PrevFilter));
        assert_eq!(while_clear_armed(gdk::Key::bracketright, false, false), Armed::Pass(KeyAction::NextFilter));
    }

    /// No card has focus, so a card's keys have nothing to act on; they must
    /// not fall through to the first card, nor plain Delete arm its Remove.
    #[test]
    fn a_cards_keys_do_nothing_while_armed() {
        for key in [gdk::Key::b, gdk::Key::e, gdk::Key::Delete, gdk::Key::space, gdk::Key::x] {
            assert_eq!(while_clear_armed(key, false, false), Armed::Swallow, "{key:?}");
        }
        assert_eq!(while_clear_armed(gdk::Key::Return, true, false), Armed::Swallow, "Ctrl+Enter");
    }
}
