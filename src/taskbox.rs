//! The task box: one window for adding a task and for editing one.
//!
//! It is a plain fixed-size window, not layer-shell. niri auto-floats windows
//! whose minimum and maximum sizes are equal, so setting both is all it takes
//! to get a floating box — no window rule needed.
//!
//! Its description is a wrapping text area and its notes are a list of rows,
//! one per note, each editable in place with its date at its right end and an
//! × to delete it. Add, Edit and `task note` all open this same window;
//! they differ only in what is filled in and where the cursor starts.
//!
//! Only the daemon opens it, over IPC, so the submit logic exists once.

pub mod form;
pub mod keys;
pub mod style;

pub use form::Submission;

use crate::niri;
use crate::task::{Annotation, Task};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::glib::WeakRef;
use gtk4::{Application, ApplicationWindow, CssProvider};
use keys::{KeyAction, Place};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// Which job the box is doing. Edit and Note are the same window on the same
/// task; Note only starts the cursor in a new empty note row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Add,
    Edit,
    Note,
}

impl Mode {
    /// The window title, which niri shows and window rules can match. Edit and
    /// Note share one, being the same window on the same task.
    pub fn title(self) -> &'static str {
        match self {
            Mode::Add => "Add Task",
            Mode::Edit | Mode::Note => "Edit Task",
        }
    }

    fn submit_label(self) -> &'static str {
        match self {
            Mode::Add => "Add",
            Mode::Edit | Mode::Note => "Save",
        }
    }
}

/// What a box is open on, so a second request can tell the box it asked for
/// from another one. Edit and Note on one task are the same box, as are the
/// two add boxes: only where the cursor starts, or the default button, differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A new task.
    Add,
    /// An existing task, by its full uuid.
    Task(String),
}

/// What `open_in` did with a request. There is only ever one box, so a
/// request while one is open brings that box forward and is dropped: opening
/// a second over it, or replacing it, would lose whatever was typed there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// No box was open, so this one is now.
    New,
    /// The box asked for was the one already open.
    Same,
    /// Another box was open; it is the one brought forward.
    Other,
}

/// What a request for a box on `asked` does while `open` is the box on
/// screen, if any.
fn opened(open: Option<&Subject>, asked: &Subject) -> Opened {
    match open {
        None => Opened::New,
        Some(open) if open == asked => Opened::Same,
        Some(_) => Opened::Other,
    }
}

/// The line of keys beside the buttons. It names what Ctrl+Enter presses in
/// this box, and offers Ctrl+Shift+Enter only where that is a different
/// button — in a box opened to refine, both refine.
fn hint(mode: Mode, refine: bool) -> &'static str {
    match (mode, refine) {
        (Mode::Add, false) => {
            "Enter: next note · Ctrl+Enter: add · Ctrl+Shift+Enter: add & refine · Esc: discard"
        }
        (Mode::Add, true) => "Enter: next note · Ctrl+Enter: add & refine · Esc: discard",
        (Mode::Edit | Mode::Note, _) => "Enter: next note · Ctrl+Enter: save · Esc: discard",
    }
}

/// Fixed, because a resizable window here would be a decision to make every
/// time rather than a box that is always the same shape. Big enough that a
/// planned task's ten long notes read as a list rather than a keyhole.
const WIDTH: i32 = 800;
const HEIGHT: i32 = 760;
/// About three lines of the terminal font plus the field's padding: room to
/// see a long description whole, while the notes keep the rest.
const DESCRIPTION_HEIGHT: i32 = 84;

/// Stable, so `window-rule { match app-id="dev.niri-tasks.box" }` works.
pub const APP_ID: &str = "dev.niri-tasks.box";

/// Everything that varies between one box and the next: the job, the line
/// above the description, and what is filled in. Built by `add` or `for_task`
/// so the daemon and the CLI open identical windows.
pub struct BoxConfig {
    pub mode: Mode,
    /// What the box is open on, which is how a second request finds it.
    pub subject: Subject,
    /// A dim line above the description — the tag a new task goes to. Hidden
    /// when empty.
    pub subtitle: String,
    pub description: String,
    /// The task's notes, shown as stored and in stored order.
    pub notes: Vec<Annotation>,
    /// Ctrl+Enter presses Add & refine rather than Add — the box
    /// Mod+Alt+Shift+T opens. Only add mode has that button, so it is false
    /// for an existing task.
    pub refine: bool,
}

impl BoxConfig {
    /// An empty box for a new task on `tag`, with Add & refine as the default
    /// button when `refine` is set.
    pub fn add(tag: &str, refine: bool) -> Self {
        Self {
            mode: Mode::Add,
            subject: Subject::Add,
            subtitle: format!("+{tag}"),
            description: String::new(),
            notes: Vec::new(),
            refine,
        }
    }

    /// The box for an existing task. It fetches the description and notes
    /// itself, rather than taking them as arguments: the ones you open the box
    /// to fix are the long ones, which a one-line row shows a fraction of.
    pub fn for_task(mode: Mode, task: Task) -> Self {
        Self {
            mode,
            subject: Subject::Task(task.uuid),
            subtitle: String::new(),
            description: task.description,
            notes: task.annotations,
            refine: false,
        }
    }
}

/// Open the box inside the daemon's running Application, calling `on_submit`
/// with what was saved. The window is built once, in `build_window`.
///
/// While a box is open this opens nothing: it brings the open box forward and
/// drops the request, saying whether that box was the one asked for, so the
/// daemon can tell you when it was not.
pub fn open_in(app: &Application, cfg: BoxConfig, on_submit: impl Fn(Submission) + 'static) -> Opened {
    let open = OPEN.with(|o| {
        o.borrow()
            .as_ref()
            .and_then(|(window, subject)| Some((window.upgrade()?, subject.clone())))
    });
    // A box that is closing but not yet destroyed is hidden: it is no box.
    let open = open.filter(|(window, _)| window.is_visible());
    if let Some((window, subject)) = open {
        window.present();
        focus_through_niri();
        return opened(Some(&subject), &cfg.subject);
    }
    let window = build_window(app, &cfg, Rc::new(on_submit));
    // Destroy, not close-request: every way out — Esc, Cancel, a save, niri's
    // close-window — ends in it, and it comes after the window is gone.
    window.connect_destroy(|_| {
        OPEN.with(|o| {
            o.borrow_mut().take();
        });
    });
    OPEN.with(|o| *o.borrow_mut() = Some((window.downgrade(), cfg.subject)));
    Opened::New
}

/// Focus the open box through niri, which `present()` alone may not do.
///
/// The daemon has no xdg-activation token, since the keypress that asked for
/// the box went to another process, so niri may only mark the box urgent when
/// the focus is elsewhere. Asking niri directly is what raises it. Only this
/// process's own box is focused, and a niri that cannot be reached is logged
/// and ignored: the box is still shown by `present()`.
fn focus_through_niri() {
    let pid = std::process::id() as i32;
    let id = niri::windows().map(|windows| {
        windows
            .iter()
            .find(|w| w.app_id.as_deref() == Some(APP_ID) && w.pid == Some(pid))
            .map(|w| w.id)
    });
    match id {
        Ok(Some(id)) => {
            if let Err(e) = niri::focus_window(id) {
                eprintln!("could not focus the open task box: {e:#}");
            }
        }
        Ok(None) => {}
        Err(e) => eprintln!("could not look for the open task box: {e:#}"),
    }
}

thread_local! {
    /// The box's stylesheet, once it is on the display. The daemon opens a box
    /// for every keypress over a process that runs for days, and a provider
    /// added per box would stay on the display for good, so every style
    /// lookup, the panels' included, would walk one more each time. Only ever
    /// touched on the GTK main thread.
    static STYLE: OnceCell<CssProvider> = const { OnceCell::new() };

    /// The box on screen and what it is open on, so a second request brings it
    /// forward rather than opening another box over it. Weak, so this never
    /// keeps a closed box alive; the window's destroy handler empties it, and
    /// a reference that no longer upgrades counts as no box either. Only ever
    /// touched on the GTK main thread.
    static OPEN: RefCell<Option<(WeakRef<ApplicationWindow>, Subject)>> =
        const { RefCell::new(None) };
}

/// Put the box's stylesheet on the display, unless it is already there.
///
/// It is never taken off again: `style::css()` is built from constants, so no
/// box ever needs a different one.
fn install_style() {
    STYLE.with(|installed| {
        if installed.get().is_some() {
            return;
        }
        // No display means nothing to style yet. The cell stays empty, so the
        // next box tries again.
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let provider = CssProvider::new();
        // load_from_data, not load_from_string: the latter is gated behind gtk4's
        // v4_12 feature, and this needs no minimum beyond what the crate requires.
        provider.load_from_data(&style::css());
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let _ = installed.set(provider);
    });
}

/// Build and show the window. `on_submit` fires only with something worth
/// saving: never on Esc, and never with an empty description, which keeps the
/// window open instead.
fn build_window(app: &Application, cfg: &BoxConfig, on_submit: Rc<dyn Fn(Submission)>) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(cfg.mode.title())
        .default_width(WIDTH)
        .default_height(HEIGHT)
        .resizable(false)
        .build();
    // Equal minimum and maximum is what makes niri float this rather than tile
    // it into the column layout.
    window.set_size_request(WIDTH, HEIGHT);
    // What every rule in the box's stylesheet is scoped to (style.rs).
    window.add_css_class("task-box");

    install_style();

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(20);
    root.set_margin_end(20);

    // ─── header ───────────────────────────────────────────────────────────
    let header = gtk4::Label::new(Some(&cfg.subtitle));
    header.add_css_class("dim");
    header.set_xalign(0.0);
    header.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    header.set_visible(!cfg.subtitle.is_empty());
    root.append(&header);

    // ─── description ──────────────────────────────────────────────────────
    let description = text_view(&cfg.description);
    let description_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .height_request(DESCRIPTION_HEIGHT)
        .child(&description)
        .build();
    description_scroll.add_css_class("field");
    root.append(&description_scroll);

    // ─── notes ────────────────────────────────────────────────────────────
    let notes_label = gtk4::Label::new(Some("Notes"));
    notes_label.add_css_class("dim");
    notes_label.set_xalign(0.0);
    root.append(&notes_label);

    let list = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    // Rows wrap, so there is never anything to scroll to sideways. The
    // viewport's own scroll-to-focus is off: it measured a row made by Enter
    // before that row had a place, read it as sitting at the top, and jumped
    // the list there. `follow_focus` does the job instead, after layout.
    let viewport = gtk4::Viewport::builder()
        .scroll_to_focus(false)
        .child(&list)
        .build();
    let notes_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&viewport)
        .build();
    root.append(&notes_scroll);

    let notes = Rc::new(Notes {
        list,
        description: description.clone(),
        rows: RefCell::new(Vec::new()),
    });
    for note in &cfg.notes {
        notes.insert(notes.len(), Some(note));
    }

    let add_note = gtk4::Button::with_label("+ Add note");
    add_note.set_halign(gtk4::Align::Start);
    root.append(&add_note);

    // ─── footer ───────────────────────────────────────────────────────────
    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let hint_label = gtk4::Label::new(Some(hint(cfg.mode, cfg.refine)));
    hint_label.add_css_class("dim");
    hint_label.set_xalign(0.0);
    hint_label.set_hexpand(true);
    // The add hint is longer than the room beside three buttons; wrapping
    // keeps it whole rather than pushing a fixed-size window wider.
    hint_label.set_wrap(true);
    let cancel = gtk4::Button::with_label("Cancel");
    let submit = gtk4::Button::with_label(cfg.mode.submit_label());
    footer.append(&hint_label);
    footer.append(&cancel);
    footer.append(&submit);
    // Add mode only: refining from Edit or Note is the card's Refine's job.
    let refine = (cfg.mode == Mode::Add).then(|| gtk4::Button::with_label("Add & refine"));
    if let Some(refine) = &refine {
        footer.append(refine);
    }
    root.append(&footer);

    window.set_child(Some(&root));

    // ─── save / discard ───────────────────────────────────────────────────
    // Every handler below holds the window weakly, and the row handlers hold
    // their rows weakly: the window owns its children and their handlers, so a
    // strong reference back to it is a cycle, and in the daemon that would keep
    // every closed box in memory. `notes` is safe to hold strongly — it reaches
    // only the list, which is below the window, never the window itself.
    let do_submit = {
        let loaded_description = cfg.description.clone();
        let window = window.downgrade();
        let notes = notes.clone();
        move |refine: bool| {
            let submission = form::submission(
                &loaded_description,
                &buffer_text(&notes.description),
                &notes.rows(),
            );
            // An empty description saves nothing, and closing anyway would
            // throw the note edits away without a word: stay open, with the
            // cursor where the fix is. Esc and Cancel are what discard.
            let Some(mut submission) = submission else {
                focus_end(&notes.description);
                return;
            };
            submission.refine = refine;
            if let Some(window) = window.upgrade() {
                window.close();
            }
            on_submit(submission);
        }
    };

    {
        let do_submit = do_submit.clone();
        submit.connect_clicked(move |_| do_submit(false));
    }
    if let Some(refine) = &refine {
        let do_submit = do_submit.clone();
        refine.connect_clicked(move |_| do_submit(true));
    }
    {
        let window = window.downgrade();
        cancel.connect_clicked(move |_| {
            if let Some(window) = window.upgrade() {
                window.close();
            }
        });
    }
    {
        let notes = notes.clone();
        add_note.connect_clicked(move |_| focus_end(&notes.insert(notes.len(), None)));
    }

    // Capture phase, so the window sees a key before the focused text view
    // does. Defaulting to bubble was a bug once: the view took Return,
    // inserted a newline and stopped it there, so Ctrl+Enter did nothing while
    // Escape — which a text view does not consume — kept working. Everything
    // `key_action` does not claim still returns Proceed and reaches the view.
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let window = window.downgrade();
        let notes = notes.clone();
        // Which button each shortcut presses. Ctrl+Enter presses the default;
        // Ctrl+Shift+Enter presses Add & refine where there is one, and saves
        // where there is not, as it always did.
        let default_refines = cfg.refine;
        let can_refine = cfg.mode == Mode::Add;
        keys.connect_key_pressed(move |_, key, _, state| {
            let Some(window) = window.upgrade() else {
                return gtk4::glib::Propagation::Proceed;
            };
            let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
            // GtkWindowExt and RootExt both have a `focus()`; either answers.
            let place = GtkWindowExt::focus(&window).and_then(|w| notes.place_of(&w));
            match keys::key_action(key, ctrl, shift, place) {
                KeyAction::Cancel => window.close(),
                KeyAction::Save => do_submit(default_refines),
                KeyAction::Refine => do_submit(can_refine),
                KeyAction::ToFirstNote => focus_end(&notes.first_or_new()),
                KeyAction::NewNoteBelow(i) => focus_end(&notes.insert(i + 1, None)),
                KeyAction::DeleteNote(i) => notes.remove(i),
                KeyAction::Ignore => return gtk4::glib::Propagation::Proceed,
            }
            gtk4::glib::Propagation::Stop
        });
    }
    window.add_controller(keys);

    window.present();
    settle_after_opening(&window, &notes.list);
    follow_focus(&window, &notes_scroll, &notes.list);

    // The Wayland app_id comes from the GtkApplication, and inside the daemon
    // that application is the daemon's — so a box opened there arrived as
    // dev.niri-tasks.daemon while one opened by the CLI was dev.niri-tasks.box.
    // It is set per-toplevel rather than per-application, so set it here once
    // the surface exists.
    if let Some(surface) = window.surface() {
        if let Ok(toplevel) = surface.downcast::<gdk4_wayland::WaylandToplevel>() {
            toplevel.set_application_id(APP_ID);
        }
    }

    // Where the cursor starts is what tells Note from Edit. At the end of the
    // text, so typing carries on rather than landing in front of it.
    match cfg.mode {
        Mode::Note => focus_end(&notes.insert(notes.len(), None)),
        Mode::Add | Mode::Edit => focus_end(&description),
    }
    window
}

/// Have every note row measure itself again once the window has been drawn.
///
/// A text view does not size itself to the width it is given: it answers a
/// height from the width its layout last had, and before its first allocation
/// that is one line. So on opening, every note showed as a single cut-off line,
/// and only filled out once scrolling happened to make the list lay out again.
/// By the end of the first frame each view has been allocated and has laid its
/// text out at the real width, so asking the rows once more gets the true
/// heights. Once is enough: the box is a fixed width, and a row that grows or
/// shrinks while being typed in asks for its own resize.
fn settle_after_opening(window: &ApplicationWindow, list: &gtk4::Box) {
    let list = list.downgrade();
    after_next_paint(window, move || {
        let Some(list) = list.upgrade() else {
            return;
        };
        let mut row = list.first_child();
        while let Some(widget) = row {
            widget.queue_resize();
            row = widget.next_sibling();
        }
    });
}

/// Keep the row with the cursor in view, wherever the cursor goes — Enter,
/// Tab, a deleted row, or the empty row Note opens with.
///
/// Scrolling waits for the frame after the move, because a row just made has
/// no place in the list until it has been laid out; and on opening, until
/// [`settle_after_opening`] has given the rows their real heights, which is a
/// frame later still — so the first scroll is simply the one those resizes
/// cause, when the focus has not moved at all.
fn follow_focus(window: &ApplicationWindow, scroll: &gtk4::ScrolledWindow, list: &gtk4::Box) {
    let (scroll, list) = (scroll.downgrade(), list.downgrade());
    let follow = move |window: &ApplicationWindow| {
        let (scroll, list) = (scroll.clone(), list.clone());
        let weak = window.downgrade();
        after_next_paint(window, move || {
            if let (Some(window), Some(scroll), Some(list)) =
                (weak.upgrade(), scroll.upgrade(), list.upgrade())
            {
                scroll_focus_into_view(&window, &scroll, &list);
            }
        });
    };
    window.connect_notify_local(Some("focus-widget"), {
        let follow = follow.clone();
        move |window, _| follow(window)
    });
    // Once for the opening: the focus is placed before this is connected, and
    // the rows only reach their real heights two frames in.
    let weak = window.downgrade();
    after_next_paint(window, move || {
        if let Some(window) = weak.upgrade() {
            follow(&window);
        }
    });
}

/// Run `f` once, after the window's next frame has been painted — by which
/// point everything queued before it has been measured and placed.
pub(crate) fn after_next_paint(window: &ApplicationWindow, f: impl FnOnce() + 'static) {
    let Some(clock) = window.frame_clock() else {
        return;
    };
    let handler = Rc::new(RefCell::new(None));
    // The signal wants a Fn; the cell is what lets it run `f` only once.
    let f = RefCell::new(Some(f));
    let id = clock.connect_after_paint({
        let handler = handler.clone();
        move |clock| {
            if let Some(id) = handler.borrow_mut().take() {
                clock.disconnect(id);
            }
            if let Some(f) = f.borrow_mut().take() {
                f();
            }
        }
    });
    *handler.borrow_mut() = Some(id);
    // A focus move that changes nothing on screen would not otherwise bring
    // the next frame.
    clock.request_phase(gdk::FrameClockPhase::AFTER_PAINT);
}

/// Scroll the notes list just enough to show the whole row the cursor is in,
/// if it is in one.
fn scroll_focus_into_view(window: &ApplicationWindow, scroll: &gtk4::ScrolledWindow, list: &gtk4::Box) {
    // GtkWindowExt and RootExt both have a `focus()`; either answers.
    let Some(mut row) = GtkWindowExt::focus(window) else {
        return;
    };
    // Up from the text view to the row that holds it, so the row's padding
    // and date come into view along with the text.
    while row.parent().as_ref() != Some(list.upcast_ref()) {
        let Some(parent) = row.parent() else {
            return;
        };
        row = parent;
    }
    let Some(bounds) = row.compute_bounds(list) else {
        return;
    };
    let adjustment = scroll.vadjustment();
    let (top, bottom) = (bounds.y() as f64, (bounds.y() + bounds.height()) as f64);
    if bottom > adjustment.value() + adjustment.page_size() {
        adjustment.set_value(bottom - adjustment.page_size());
    } else if top < adjustment.value() {
        adjustment.set_value(top);
    }
}

/// The note rows on screen, in order, and the box that holds them.
struct Notes {
    list: gtk4::Box,
    /// Where the cursor goes when the first row is deleted.
    description: gtk4::TextView,
    rows: RefCell<Vec<NoteRow>>,
}

struct NoteRow {
    entry: Option<String>,
    loaded: String,
    container: gtk4::Box,
    view: gtk4::TextView,
}

impl Notes {
    fn len(&self) -> usize {
        self.rows.borrow().len()
    }

    /// Put a row at `index` — `note`'s, or an empty new one — and return its
    /// text view, for the caller to focus if it should.
    fn insert(self: &Rc<Self>, index: usize, note: Option<&Annotation>) -> gtk4::TextView {
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
    fn remove(&self, index: usize) {
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
    fn first_or_new(self: &Rc<Self>) -> gtk4::TextView {
        let first = self.rows.borrow().first().map(|r| r.view.clone());
        first.unwrap_or_else(|| self.insert(0, None))
    }

    fn index_of(&self, view: &gtk4::TextView) -> Option<usize> {
        self.rows.borrow().iter().position(|r| &r.view == view)
    }

    /// Where `focus` is, in the terms `keys::key_action` asks about.
    fn place_of(&self, focus: &gtk4::Widget) -> Option<Place> {
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

    fn rows(&self) -> Vec<form::Row> {
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

/// A wrapping text field. Tab is left to move the focus, so the keyboard can
/// reach a row's × and the buttons.
fn text_view(text: &str) -> gtk4::TextView {
    let view = gtk4::TextView::new();
    view.set_wrap_mode(gtk4::WrapMode::WordChar);
    view.set_accepts_tab(false);
    view.buffer().set_text(text);
    view
}

fn buffer_text(view: &gtk4::TextView) -> String {
    let buffer = view.buffer();
    buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
}

fn focus_end(view: &gtk4::TextView) {
    view.grab_focus();
    let buffer = view.buffer();
    buffer.place_cursor(&buffer.end_iter());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One window for an existing task, whichever command opened it; only
    /// adding reads differently.
    #[test]
    fn titles_match_the_mode() {
        assert_eq!(Mode::Add.title(), "Add Task");
        assert_eq!(Mode::Edit.title(), "Edit Task");
        assert_eq!(Mode::Note.title(), "Edit Task");
    }

    #[test]
    fn a_task_opens_with_its_description_and_every_note() {
        let t: crate::task::Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20260801T000000Z","description":"Goal: one"},
                {"entry":"20260802T000000Z","description":"Decided: two"}]}"#,
        )
        .unwrap();
        let cfg = BoxConfig::for_task(Mode::Note, t);
        assert_eq!(cfg.description, "d");
        let notes: Vec<&str> = cfg.notes.iter().map(|a| a.description.as_str()).collect();
        assert_eq!(notes, ["Goal: one", "Decided: two"], "as stored, in stored order");
    }

    /// Mod+Alt+Shift+T opens the same empty box; only the default button
    /// differs.
    #[test]
    fn an_add_box_says_which_button_ctrl_enter_presses() {
        let plain = BoxConfig::add("proj", false);
        let refining = BoxConfig::add("proj", true);
        assert_eq!((plain.mode, refining.mode), (Mode::Add, Mode::Add));
        assert_eq!(refining.subtitle, "+proj");
        assert!(!plain.refine);
        assert!(refining.refine);
    }

    #[test]
    fn an_existing_task_never_opens_to_refine() {
        let t: crate::task::Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert!(!BoxConfig::for_task(Mode::Edit, t).refine);
    }

    /// The hint names what Ctrl+Enter does in this box, and offers
    /// Ctrl+Shift+Enter only where it does something different.
    #[test]
    fn the_hint_names_the_default_button() {
        assert_eq!(
            hint(Mode::Add, false),
            "Enter: next note · Ctrl+Enter: add · Ctrl+Shift+Enter: add & refine · Esc: discard"
        );
        assert_eq!(
            hint(Mode::Add, true),
            "Enter: next note · Ctrl+Enter: add & refine · Esc: discard"
        );
        for mode in [Mode::Edit, Mode::Note] {
            assert_eq!(hint(mode, false), "Enter: next note · Ctrl+Enter: save · Esc: discard");
        }
    }

    /// A box remembers what it is open on: adding, in either add box, or one
    /// task by its uuid, in either Edit or Note.
    #[test]
    fn a_box_knows_what_it_is_open_on() {
        assert_eq!(BoxConfig::add("proj", false).subject, Subject::Add);
        assert_eq!(BoxConfig::add("proj", true).subject, Subject::Add);
        let t: crate::task::Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(BoxConfig::for_task(Mode::Note, t).subject, Subject::Task("u".into()));
    }

    /// With no box open a request opens one. With one open, it is the same box
    /// only when it is open on the same thing; anything else is another box.
    #[test]
    fn a_second_request_finds_the_open_box() {
        let task = |uuid: &str| Subject::Task(uuid.into());
        assert_eq!(opened(None, &Subject::Add), Opened::New);
        assert_eq!(opened(None, &task("a")), Opened::New);
        assert_eq!(opened(Some(&Subject::Add), &Subject::Add), Opened::Same);
        assert_eq!(opened(Some(&task("a")), &task("a")), Opened::Same);
        assert_eq!(opened(Some(&Subject::Add), &task("a")), Opened::Other);
        assert_eq!(opened(Some(&task("a")), &Subject::Add), Opened::Other);
        assert_eq!(opened(Some(&task("a")), &task("b")), Opened::Other);
    }
}
