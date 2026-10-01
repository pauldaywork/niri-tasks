//! What a keypress on the task panel means while it has the keyboard.
//!
//! Pure, as the task box's keys are, so the mapping is testable without a
//! window. Acting on it, and the controller's propagation phase that lets it
//! see the arrows first, are `surface.rs`'s.

use super::actions::Action;
use gtk4::gdk;

/// What a keypress on the panel should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Give the keyboard back and tuck the panel away.
    Release,
    PrevCard,
    NextCard,
    /// Along the focused card: its body, then its buttons.
    PrevSlot,
    NextSlot,
    /// Press this button on the focused card, if it has one.
    Run(Action),
    /// Pass it through: Enter and Space press the focused button.
    Ignore,
}

/// Map a keypress to what it should do. The letters work without a modifier,
/// because nothing on the panel takes typing.
pub fn key_action(key: gdk::Key) -> KeyAction {
    match key {
        gdk::Key::Escape => KeyAction::Release,
        gdk::Key::Up => KeyAction::PrevCard,
        gdk::Key::Down => KeyAction::NextCard,
        gdk::Key::Left | gdk::Key::ISO_Left_Tab => KeyAction::PrevSlot,
        gdk::Key::Right | gdk::Key::Tab => KeyAction::NextSlot,
        gdk::Key::s => KeyAction::Run(Action::Start),
        gdk::Key::r => KeyAction::Run(Action::Refine),
        gdk::Key::e => KeyAction::Run(Action::Edit),
        gdk::Key::t => KeyAction::Run(Action::Stop),
        gdk::Key::Delete | gdk::Key::KP_Delete => KeyAction::Run(Action::Remove),
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
        assert_eq!(key_action(gdk::Key::s), KeyAction::Run(Action::Start));
        assert_eq!(key_action(gdk::Key::r), KeyAction::Run(Action::Refine));
        assert_eq!(key_action(gdk::Key::e), KeyAction::Run(Action::Edit));
        assert_eq!(key_action(gdk::Key::t), KeyAction::Run(Action::Stop));
        assert_eq!(key_action(gdk::Key::Delete), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::KP_Delete), KeyAction::Run(Action::Remove));
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
}
