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
    /// Pass it through: Enter and Space press the focused button.
    Ignore,
}

/// Map a keypress to what it should do. The letters work without a modifier,
/// because nothing on the panel takes typing. With Caps Lock on the keyval
/// arrives as a capital (`S`, not `s`), so the key is lowercased first and
/// capitals press the same buttons.
pub fn key_action(key: gdk::Key) -> KeyAction {
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
        assert_eq!(key_action(gdk::Key::Escape), KeyAction::Release);
    }

    #[test]
    fn up_and_down_move_between_cards() {
        assert_eq!(key_action(gdk::Key::Up), KeyAction::PrevCard);
        assert_eq!(key_action(gdk::Key::Down), KeyAction::NextCard);
    }

    #[test]
    fn left_right_and_tab_move_along_the_card() {
        assert_eq!(key_action(gdk::Key::Left), KeyAction::PrevSlot);
        assert_eq!(key_action(gdk::Key::Right), KeyAction::NextSlot);
        assert_eq!(key_action(gdk::Key::Tab), KeyAction::NextSlot);
        // Shift+Tab arrives as ISO_Left_Tab.
        assert_eq!(key_action(gdk::Key::ISO_Left_Tab), KeyAction::PrevSlot);
    }

    #[test]
    fn letters_and_delete_run_their_buttons() {
        assert_eq!(key_action(gdk::Key::g), KeyAction::Run(Action::Session));
        assert_eq!(key_action(gdk::Key::b), KeyAction::Run(Action::Back));
        assert_eq!(key_action(gdk::Key::s), KeyAction::Run(Action::Start));
        assert_eq!(key_action(gdk::Key::r), KeyAction::Run(Action::Refine));
        assert_eq!(key_action(gdk::Key::e), KeyAction::Run(Action::Edit));
        assert_eq!(key_action(gdk::Key::t), KeyAction::Run(Action::Stop));
        assert_eq!(key_action(gdk::Key::Delete), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::KP_Delete), KeyAction::Run(Action::Remove));
    }

    #[test]
    fn capitals_run_their_buttons_too() {
        assert_eq!(key_action(gdk::Key::G), KeyAction::Run(Action::Session));
        assert_eq!(key_action(gdk::Key::B), KeyAction::Run(Action::Back));
        assert_eq!(key_action(gdk::Key::S), KeyAction::Run(Action::Start));
        assert_eq!(key_action(gdk::Key::R), KeyAction::Run(Action::Refine));
        assert_eq!(key_action(gdk::Key::E), KeyAction::Run(Action::Edit));
        assert_eq!(key_action(gdk::Key::T), KeyAction::Run(Action::Stop));
    }

    /// Enter and Space press the focused button, which GTK does itself.
    #[test]
    fn enter_and_space_pass_through() {
        for key in [gdk::Key::Return, gdk::Key::KP_Enter, gdk::Key::space, gdk::Key::x] {
            assert_eq!(key_action(key), KeyAction::Ignore, "{key:?}");
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
            assert_eq!(key_action(key), KeyAction::Filter(filter), "{key:?}");
        }
    }

    #[test]
    fn brackets_step_between_tabs() {
        assert_eq!(key_action(gdk::Key::bracketleft), KeyAction::PrevFilter);
        assert_eq!(key_action(gdk::Key::bracketright), KeyAction::NextFilter);
    }

    /// Past the five tabs a number is nothing, not a sixth tab.
    #[test]
    fn other_numbers_pass_through() {
        for key in [gdk::Key::_0, gdk::Key::_6, gdk::Key::_9] {
            assert_eq!(key_action(key), KeyAction::Ignore, "{key:?}");
        }
    }
}
