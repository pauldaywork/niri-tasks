//! What a keypress in the task box means, by where the cursor is.
//!
//! Pure, so the mapping is testable without a window. The other half of
//! making these keys work is the controller's propagation phase, which only a
//! real compositor exercises (`tests/e2e-box.sh`).

use gtk4::gdk;

/// Where the keyboard focus is, as far as the box's keys care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Description,
    /// The note row at `index`, and whether it has any text — Backspace only
    /// deletes a row there is nothing left in.
    Note { index: usize, empty: bool },
}

/// What a keypress in the box should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Save,
    Cancel,
    /// From the description to the first note, making one if there are none.
    ToFirstNote,
    /// A new empty row below the one at this index, with the cursor in it.
    NewNoteBelow(usize),
    /// Delete the row at this index, and move up.
    DeleteNote(usize),
    /// Pass it through: ordinary typing, and Enter on a button, which presses
    /// it.
    Ignore,
}

/// Map a keypress to what it should do. `place` is `None` when the focus is
/// on something other than a text field — a button.
pub fn key_action(key: gdk::Key, ctrl: bool, place: Option<Place>) -> KeyAction {
    let enter = matches!(key, gdk::Key::Return | gdk::Key::KP_Enter);
    match (key, place) {
        (gdk::Key::Escape, _) => KeyAction::Cancel,
        _ if enter && ctrl => KeyAction::Save,
        (_, Some(Place::Description)) if enter => KeyAction::ToFirstNote,
        (_, Some(Place::Note { index, .. })) if enter => KeyAction::NewNoteBelow(index),
        (gdk::Key::BackSpace, Some(Place::Note { index, empty: true })) => KeyAction::DeleteNote(index),
        _ => KeyAction::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERYWHERE: [Option<Place>; 4] = [
        None,
        Some(Place::Description),
        Some(Place::Note { index: 0, empty: true }),
        Some(Place::Note { index: 2, empty: false }),
    ];

    #[test]
    fn escape_discards_from_anywhere() {
        for place in EVERYWHERE {
            assert_eq!(key_action(gdk::Key::Escape, false, place), KeyAction::Cancel);
            assert_eq!(key_action(gdk::Key::Escape, true, place), KeyAction::Cancel);
        }
    }

    #[test]
    fn ctrl_enter_saves_from_anywhere() {
        for place in EVERYWHERE {
            assert_eq!(key_action(gdk::Key::Return, true, place), KeyAction::Save);
            assert_eq!(key_action(gdk::Key::KP_Enter, true, place), KeyAction::Save);
        }
    }

    #[test]
    fn enter_in_the_description_moves_to_the_first_note() {
        assert_eq!(
            key_action(gdk::Key::Return, false, Some(Place::Description)),
            KeyAction::ToFirstNote
        );
        assert_eq!(
            key_action(gdk::Key::KP_Enter, false, Some(Place::Description)),
            KeyAction::ToFirstNote
        );
    }

    #[test]
    fn enter_in_a_note_adds_a_row_below_it() {
        for empty in [true, false] {
            assert_eq!(
                key_action(gdk::Key::Return, false, Some(Place::Note { index: 3, empty })),
                KeyAction::NewNoteBelow(3)
            );
        }
    }

    /// A focused button is pressed by Enter; taking the key would break it.
    #[test]
    fn enter_on_a_button_is_passed_through() {
        assert_eq!(key_action(gdk::Key::Return, false, None), KeyAction::Ignore);
    }

    #[test]
    fn backspace_deletes_only_an_empty_row() {
        assert_eq!(
            key_action(gdk::Key::BackSpace, false, Some(Place::Note { index: 1, empty: true })),
            KeyAction::DeleteNote(1)
        );
        assert_eq!(
            key_action(gdk::Key::BackSpace, false, Some(Place::Note { index: 1, empty: false })),
            KeyAction::Ignore,
            "in a row with text, Backspace deletes a character"
        );
        assert_eq!(
            key_action(gdk::Key::BackSpace, false, Some(Place::Description)),
            KeyAction::Ignore,
            "the description is never deleted"
        );
    }

    /// Ctrl+A and friends must still reach the text view, or select-all and
    /// the usual editing keys stop working inside the box.
    #[test]
    fn ordinary_typing_is_passed_through() {
        for key in [gdk::Key::a, gdk::Key::space, gdk::Key::Tab, gdk::Key::Up] {
            for place in EVERYWHERE {
                assert_eq!(key_action(key, false, place), KeyAction::Ignore);
                assert_eq!(key_action(key, true, place), KeyAction::Ignore);
            }
        }
    }
}
