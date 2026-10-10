//! What is on screen in the task box, turned into what to save.
//!
//! Pure, so the rules that decide what reaches taskwarrior are tested without
//! typing into a window: which rows count, which notes keep their date, and
//! when there is nothing to do at all.

use crate::task::NoteEdit;
use crate::text::collapse_whitespace;

/// One note row as it stands when the box is saved.
pub struct Row {
    /// The stamp of the note this row was loaded from; `None` for a row added
    /// in the box.
    pub entry: Option<String>,
    /// The text it was loaded with — empty for a new row.
    pub loaded: String,
    /// The text in it now.
    pub text: String,
}

/// What the daemon does once an added task has its uuid. One value per
/// button on the add box's footer; an existing task's box has only Save, so
/// its submissions are always `Nothing`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Then {
    /// Add, or Save: the task is added or saved, and that is all.
    #[default]
    Nothing,
    /// Add & refine: `niritasks task refine <uuid>`, a tab in the
    /// workspace's herdr session.
    Refine,
    /// Add & all: `niritasks task refine <uuid> --unattended`, a headless
    /// refine and report in the background.
    All,
}

/// What the box hands back when it is saved.
pub struct Submission {
    pub description: String,
    /// In the order the rows are on screen, empty rows left out.
    pub notes: Vec<NoteEdit>,
    /// What follows the add, by which button was pressed: nothing (Add), a
    /// refine in a herdr tab (Add & refine) or an unattended refine and
    /// report in the background (Add & all). Set by the button, not by what
    /// is on screen, so [`submission`] always leaves it `Nothing`.
    pub then: Then,
}

impl Submission {
    /// The notes' text alone, for adding a task — a new task's notes are all
    /// new, so there is no date to keep.
    pub fn note_texts(&self) -> Vec<String> {
        self.notes.iter().map(|n| n.text.clone()).collect()
    }
}

/// The text to save for a field that was loaded with `loaded`.
///
/// Typed text has its whitespace collapsed: taskwarrior descriptions and notes
/// are single-line, and a pasted newline has to go somewhere. Text left exactly
/// as it was loaded is kept verbatim instead, so opening a task and saving it
/// untouched never rewrites a stored double space into a change.
pub fn cleaned(loaded: &str, text: &str) -> String {
    if text == loaded {
        text.to_string()
    } else {
        collapse_whitespace(text)
    }
}

/// What to save, or `None` when there is nothing to do — an empty description,
/// which gives a task nothing to be called and its notes nothing to hang off.
pub fn submission(loaded_description: &str, description: &str, rows: &[Row]) -> Option<Submission> {
    let description = cleaned(loaded_description, description);
    if description.trim().is_empty() {
        return None;
    }
    let notes = rows
        .iter()
        .filter_map(|r| {
            let text = cleaned(&r.loaded, &r.text);
            (!text.trim().is_empty()).then(|| NoteEdit { entry: r.entry.clone(), text })
        })
        .collect();
    Some(Submission { description, notes, then: Then::Nothing })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(entry: Option<&str>, loaded: &str, text: &str) -> Row {
        Row { entry: entry.map(Into::into), loaded: loaded.into(), text: text.into() }
    }

    #[test]
    fn an_empty_description_saves_nothing() {
        let notes = [row(None, "", "a note")];
        assert!(submission("", "", &notes).is_none());
        assert!(submission("", "  \n ", &notes).is_none());
        assert!(submission("was here", "", &[]).is_none(), "clearing it is not deleting the task");
    }

    #[test]
    fn typed_text_is_collapsed_and_untouched_text_is_kept() {
        assert_eq!(cleaned("", "  two\nlines  "), "two lines");
        assert_eq!(cleaned("stored  as is", "stored  as is"), "stored  as is");
        assert_eq!(cleaned("stored  as is", "stored  as  was"), "stored as was");
    }

    #[test]
    fn rows_become_notes_in_screen_order() {
        let s = submission(
            "d",
            "d",
            &[
                row(Some("20260802T000000Z"), "two", "two, edited"),
                row(None, "", "new one"),
                row(Some("20260801T000000Z"), "one", "one"),
            ],
        )
        .expect("something to save");
        assert_eq!(
            s.notes,
            vec![
                NoteEdit { entry: Some("20260802T000000Z".into()), text: "two, edited".into() },
                NoteEdit { entry: None, text: "new one".into() },
                NoteEdit { entry: Some("20260801T000000Z".into()), text: "one".into() },
            ],
            "an edited note keeps its stamp; a new row has none"
        );
    }

    /// An empty row is a row nobody typed into — Note opens one, Enter makes
    /// one — and an existing note emptied out is a note deleted.
    #[test]
    fn empty_rows_are_left_out() {
        let s = submission(
            "d",
            "d",
            &[row(None, "", ""), row(None, "", "   "), row(Some("20260801T000000Z"), "one", "")],
        )
        .expect("the description alone is worth saving");
        assert!(s.notes.is_empty());
    }

    #[test]
    fn note_texts_are_the_notes_in_order() {
        let s = submission("", "d", &[row(None, "", "a"), row(None, "", "b")]).unwrap();
        assert_eq!(s.note_texts(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(s.then, Then::Nothing, "what is on screen says nothing about which button was pressed");
        assert_eq!(Then::default(), Then::Nothing);
    }
}
