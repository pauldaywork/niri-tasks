//! The notes list in the task box: one row per note, editable in place, with
//! its date and an × to delete it; rows are inserted, deleted and read back as
//! the form is submitted.

use super::keys::Place;
use super::{buffer_text, focus_end, form, text_view};
use crate::task::Annotation;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// The note rows on screen, in order, and the box that holds them.
pub(super) struct Notes {
    pub(super) list: gtk4::Box,
    /// Where the cursor goes when the first row is deleted.
    pub(super) description: gtk4::TextView,
    pub(super) rows: RefCell<Vec<NoteRow>>,
}

pub(super) struct NoteRow {
    entry: Option<String>,
    loaded: String,
    container: gtk4::Box,
    view: gtk4::TextView,
}

impl Notes {
    pub(super) fn len(&self) -> usize {
        self.rows.borrow().len()
    }

    /// Put a row at `index` — `note`'s, or an empty new one — and return its
    /// text view, for the caller to focus if it should.
    pub(super) fn insert(self: &Rc<Self>, index: usize, note: Option<&Annotation>) -> gtk4::TextView {
        let text = note.map(|n| n.description.clone()).unwrap_or_default();
        let view = text_view(&text);
        view.set_hexpand(true);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.add_css_class("field");
        container.append(&view);

        // Small and dim at the right end. A new row has no date until
        // taskwarrior gives it one on save.
        if let Some(note) = note {
            let date = gtk4::Label::new(Some(&note.date()));
            date.add_css_class("date");
            date.set_valign(gtk4::Align::Start);
            container.append(&date);
        }

        let delete = gtk4::Button::with_label("×");
        delete.add_css_class("delete");
        delete.set_valign(gtk4::Align::Start);
        delete.set_tooltip_text(Some("Delete this note"));
        container.append(&delete);
        {
            // Weak, both of them: the row's own button holding the list that
            // holds the row is a cycle. See the note above `do_submit` for why
            // a closed box must not be kept alive.
            let notes = Rc::downgrade(self);
            let view = view.downgrade();
            delete.connect_clicked(move |_| {
                if let (Some(notes), Some(view)) = (notes.upgrade(), view.upgrade()) {
                    if let Some(i) = notes.index_of(&view) {
                        notes.remove(i);
                    }
                }
            });
        }

        let mut rows = self.rows.borrow_mut();
        match index.checked_sub(1).and_then(|above| rows.get(above)) {
            Some(above) => self.list.insert_child_after(&container, Some(&above.container)),
            None => self.list.prepend(&container),
        }
        rows.insert(
            index,
            NoteRow {
                entry: note.map(|n| n.entry.clone()),
                loaded: text,
                container,
                view: view.clone(),
            },
        );
        view
    }

    /// Delete the row at `index` and put the cursor at the end of the row
    /// above it — or of the description, when it was the first.
    pub(super) fn remove(&self, index: usize) {
        let row = self.rows.borrow_mut().remove(index);
        self.list.remove(&row.container);
        let above = match index.checked_sub(1) {
            Some(i) => self.rows.borrow()[i].view.clone(),
            None => self.description.clone(),
        };
        focus_end(&above);
    }

    /// The first row's text view, making an empty row when there is none, so
    /// Enter in the description always has somewhere to go.
    pub(super) fn first_or_new(self: &Rc<Self>) -> gtk4::TextView {
        let first = self.rows.borrow().first().map(|r| r.view.clone());
        first.unwrap_or_else(|| self.insert(0, None))
    }

    fn index_of(&self, view: &gtk4::TextView) -> Option<usize> {
        self.rows.borrow().iter().position(|r| &r.view == view)
    }

    /// Where `focus` is, in the terms `keys::key_action` asks about.
    pub(super) fn place_of(&self, focus: &gtk4::Widget) -> Option<Place> {
        if focus == self.description.upcast_ref::<gtk4::Widget>() {
            return Some(Place::Description);
        }
        self.rows
            .borrow()
            .iter()
            .enumerate()
            .find(|(_, r)| r.view.upcast_ref::<gtk4::Widget>() == focus)
            .map(|(index, r)| Place::Note {
                index,
                empty: r.view.buffer().char_count() == 0,
            })
    }

    pub(super) fn rows(&self) -> Vec<form::Row> {
        self.rows
            .borrow()
            .iter()
            .map(|r| form::Row {
                entry: r.entry.clone(),
                loaded: r.loaded.clone(),
                text: buffer_text(&r.view),
            })
            .collect()
    }
}
