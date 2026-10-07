//! The Ideas tab's text area: the workspace's notepad, typed into while the
//! task panel has the keyboard, and saved to `ideas.rs`'s file for its tag a
//! second after typing stops and again when the keyboard is given back.
//!
//! One per panel, kept across renders: the column is torn down on every
//! render, but the buffer, its cursor and what is waiting to be saved are
//! this, not the column's.

use super::style::{CARD_WIDTH_PX, PADDING_PX};
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// How long typing has to stop before the notepad saves.
const SAVE_AFTER: Duration = Duration::from_secs(1);

/// The text area's height: a large page, scrolling inside itself past that,
/// which keeps the cursor in view however long the ideas run.
const HEIGHT_PX: i32 = 480;

pub struct Notepad {
    /// What the panel puts in its column on the Ideas tab: the text view in
    /// a scroller of its own.
    pub root: gtk4::ScrolledWindow,
    view: gtk4::TextView,
    /// The tag whose file the buffer holds. None before the first load, for
    /// a workspace with no tag, and after a file that would not read.
    tag: RefCell<Option<String>>,
    /// Typed into since the last save.
    dirty: Cell<bool>,
    /// The save waiting for typing to stop.
    pending: RefCell<Option<glib::SourceId>>,
    /// The buffer is being filled from the file, which is not typing.
    loading: Cell<bool>,
}

impl Notepad {
    pub fn new() -> Rc<Notepad> {
        let view = gtk4::TextView::new();
        // WordChar, as the cards wrap: a long path or URL still breaks.
        view.set_wrap_mode(gtk4::WrapMode::WordChar);
        view.set_top_margin(PADDING_PX);
        view.set_bottom_margin(PADDING_PX);
        view.set_left_margin(PADDING_PX);
        view.set_right_margin(PADDING_PX);
        let root = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&view)
            .build();
        root.add_css_class("ideas");
        root.set_size_request(CARD_WIDTH_PX, HEIGHT_PX);
        // Clips the text to the rounded corners, as a card's action row is.
        root.set_overflow(gtk4::Overflow::Hidden);
        let notepad = Rc::new(Notepad {
            root,
            view,
            tag: RefCell::new(None),
            dirty: Cell::new(false),
            pending: RefCell::new(None),
            loading: Cell::new(false),
        });
        let weak = Rc::downgrade(&notepad);
        notepad.view.buffer().connect_changed(move |_| {
            if let Some(n) = weak.upgrade() {
                n.typed();
            }
        });
        notepad
    }

    /// Fill the buffer from `tag`'s file, with the cursor at the end to carry
    /// on from. What was typed for the tag before is saved to it first, so a
    /// workspace's ideas never land in another's file.
    ///
    /// A file that will not read leaves the notepad empty and read-only:
    /// saving it would write over ideas that are still there.
    pub fn load(&self, tag: &str) {
        self.flush();
        let (tag, text) = match crate::ideas::file_for(tag).map(|path| crate::ideas::read(&path)) {
            Some(Ok(text)) => (Some(tag.to_string()), text),
            Some(Err(e)) => {
                crate::notify::tasks(&format!("Could not read your ideas: {e}"));
                (None, String::new())
            }
            None => (None, String::new()),
        };
        self.view.set_editable(tag.is_some());
        *self.tag.borrow_mut() = tag;
        let buffer = self.view.buffer();
        self.loading.set(true);
        buffer.set_text(&text);
        buffer.place_cursor(&buffer.end_iter());
        self.loading.set(false);
        self.dirty.set(false);
    }

    /// Save now what was typed since the last save, if anything, to the tag
    /// it was typed for; the save waiting for typing to stop is not needed
    /// after this. A save that fails says so and stays owed.
    pub fn flush(&self) {
        if let Some(id) = self.pending.borrow_mut().take() {
            id.remove();
        }
        if !self.dirty.get() {
            return;
        }
        let Some(path) = self.tag.borrow().as_deref().and_then(crate::ideas::file_for) else { return };
        let buffer = self.view.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        match crate::ideas::write(&path, &text) {
            Ok(()) => self.dirty.set(false),
            Err(e) => crate::notify::tasks(&format!("Could not save your ideas: {e}")),
        }
    }

    /// Give the text area the keyboard's focus.
    pub fn focus(&self) {
        self.view.grab_focus();
    }

    /// The buffer changed: unless it was a load, owe a save, and put it off
    /// until typing has stopped for [`SAVE_AFTER`].
    fn typed(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        self.dirty.set(true);
        if let Some(id) = self.pending.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(SAVE_AFTER, move || {
            if let Some(n) = weak.upgrade() {
                // Fired, so gone: removing it again would be an error.
                n.pending.borrow_mut().take();
                n.flush();
            }
        });
        *self.pending.borrow_mut() = Some(id);
    }
}
