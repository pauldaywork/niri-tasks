//! The multi-line task box.
//!
//! This is the one surface fuzzel cannot be: fuzzel is a single-line picker,
//! and the box exists so a task description can be seen and edited whole. Every
//! other prompt in this tool stays fuzzel.
//!
//! It is a plain fixed-size window, not layer-shell. niri auto-floats windows
//! whose minimum and maximum sizes are equal, so setting both is all it takes
//! to get a floating box — no window rule needed. (The QML modal this replaces
//! relied on exactly the same behaviour.)
//!
//! Replaces `TaskBoxDaemon.qml` and `TaskBoxModal.qml`, and with them the
//! `dms ipc call taskBox` boundary: there is no daemon, no IPC, and no uuid
//! being passed between processes. It also means the box works when DMS is not
//! running, which is what retired the one-line fuzzel fallback the shell
//! version kept for that case.

use crate::theme::Theme;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
use std::cell::RefCell;
use std::rc::Rc;

/// Which of the three jobs the box is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Add,
    Edit,
    Annotate,
}

impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Mode::Add => "Add Task",
            Mode::Edit => "Edit Task",
            Mode::Annotate => "Add Note",
        }
    }

    fn submit_label(self) -> &'static str {
        match self {
            Mode::Add => "Add",
            Mode::Edit => "Save",
            Mode::Annotate => "Add note",
        }
    }
}

/// What a keypress in the box means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Submit,
    Cancel,
    /// Pass it through to the text view — ordinary typing, including a bare
    /// Return, which must insert a newline for a multi-line box to be worth
    /// having at all.
    Ignore,
}

/// Map a keypress to what it should do.
///
/// Separated from the controller so the mapping is testable. Note this does not
/// cover the propagation phase, which is the other half of making Ctrl+Enter
/// work and the half that was actually broken — see where the controller is
/// created.
pub fn key_action(key: gdk::Key, ctrl: bool) -> KeyAction {
    match key {
        gdk::Key::Escape => KeyAction::Cancel,
        gdk::Key::Return | gdk::Key::KP_Enter if ctrl => KeyAction::Submit,
        _ => KeyAction::Ignore,
    }
}

/// Whether accepted text is worth acting on.
///
/// Pulled out of the submit closure so it can be tested: it is the rule that
/// decides whether anything reaches taskwarrior at all, and inside a GTK
/// callback it could only be checked by typing into a window by hand.
pub fn is_worth_submitting(mode: Mode, text: &str, original: &str) -> bool {
    !text.is_empty() && !(mode == Mode::Edit && text == original)
}

/// Fixed, because a resizable window here would be a decision to make every
/// time rather than a box that is always the same shape — except when there are
/// notes to list above the input, which need the room. Same dimensions the QML
/// modal used.
const WIDTH: i32 = 560;
const HEIGHT: i32 = 300;
const HEIGHT_WITH_NOTES: i32 = 440;

/// Stable, so `window-rule { match app-id="dev.niri-tasks.box" }` works.
pub const APP_ID: &str = "dev.niri-tasks.box";

pub struct BoxConfig {
    pub mode: Mode,
    /// Shown in the header: the tag for Add, the nothing-useful case aside.
    pub subtitle: String,
    /// Pre-filled text — the current description when editing.
    pub initial: String,
    /// Existing notes, listed above the input when annotating.
    pub notes: String,
}

/// Open the box inside an Application that is already running, calling
/// `on_submit` with the text when it is accepted.
///
/// This is what the daemon uses. The standalone `show` below wraps the same
/// window in a throwaway Application; the window itself is built once, in
/// `build_window`, so the two paths cannot drift apart in appearance or in
/// which keys do what.
pub fn open_in(app: &Application, cfg: BoxConfig, on_submit: impl Fn(String) + 'static) {
    let theme = Theme::load();
    build_window(app, &cfg, &theme, Rc::new(on_submit));
}

/// Show the box and return what was submitted.
///
/// `None` means cancelled, submitted empty, or (when editing) submitted text
/// identical to what was already there — all of which mean "do nothing".
pub fn show(cfg: BoxConfig) -> Option<String> {
    let theme = Theme::load();
    let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    // Stable app id, so niri window rules can match the box. NON_UNIQUE is what
    // keeps two boxes opened at once from colliding on the bus and handing the
    // second one to the first's process — putting the pid in the id would do
    // that too, but at the cost of an app_id nothing can ever match.
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    let cfg = Rc::new(cfg);
    {
        let result = result.clone();
        app.connect_activate(move |app| {
            let result = result.clone();
            build_window(
                app,
                &cfg,
                &theme,
                Rc::new(move |text| *result.borrow_mut() = Some(text)),
            );
        });
    }

    // Stop GTK from parsing our argv as its own.
    app.run_with_args::<&str>(&[]);

    let taken = result.borrow().clone();
    taken
}

/// Build and show the window. `on_submit` fires with the accepted text, and
/// only when there is something to do: never on cancel, never on empty input,
/// and never on an edit that changed nothing.
fn build_window(
    app: &Application,
    cfg: &BoxConfig,
    theme: &Theme,
    on_submit: Rc<dyn Fn(String)>,
) {
    let has_notes = cfg.mode == Mode::Annotate && !cfg.notes.is_empty();
    let height = if has_notes { HEIGHT_WITH_NOTES } else { HEIGHT };

    let window = ApplicationWindow::builder()
        .application(app)
        .title(cfg.mode.title())
        .default_width(WIDTH)
        .default_height(height)
        .resizable(false)
        .build();

    // Equal minimum and maximum is what makes niri float this rather than tile
    // it into the column layout.
    window.set_size_request(WIDTH, height);

    let provider = CssProvider::new();
    // load_from_data, not load_from_string: the latter is gated behind gtk4's
    // v4_12 feature, and this needs no minimum beyond what the crate requires.
    provider.load_from_data(&theme.css());
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

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
    root.append(&header);

    // ─── existing notes, when annotating ──────────────────────────────────
    if has_notes {
        let notes = gtk4::Label::new(Some(&cfg.notes));
        notes.add_css_class("dim");
        notes.set_xalign(0.0);
        notes.set_yalign(0.0);
        // Notes wrap, so there is never anything to scroll to sideways — and a
        // horizontal bar would imply there was.
        notes.set_wrap(true);
        notes.set_wrap_mode(gtk4::pango::WrapMode::WordChar);

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .height_request(130)
            .child(&notes)
            .build();
        scroll.add_css_class("notes");
        root.append(&scroll);
    }

    // ─── input ────────────────────────────────────────────────────────────
    let buffer = gtk4::TextBuffer::new(None);
    buffer.set_text(&cfg.initial);

    let input = gtk4::TextView::with_buffer(&buffer);
    input.set_wrap_mode(gtk4::WrapMode::WordChar);
    input.set_accepts_tab(false);

    let input_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&input)
        .build();
    root.append(&input_scroll);

    let hint = gtk4::Label::new(Some("Ctrl+Enter to save · Esc to cancel"));
    hint.add_css_class("dim");
    hint.set_xalign(0.0);
    root.append(&hint);

    // ─── buttons ──────────────────────────────────────────────────────────
    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    buttons.set_halign(gtk4::Align::End);
    let cancel = gtk4::Button::with_label("Cancel");
    let submit = gtk4::Button::with_label(cfg.mode.submit_label());
    submit.add_css_class("suggested");
    buttons.append(&cancel);
    buttons.append(&submit);
    root.append(&buttons);

    window.set_child(Some(&root));

    // ─── submit / cancel ──────────────────────────────────────────────────
    let original = cfg.initial.clone();
    let mode = cfg.mode;

    let do_submit = {
        let buffer = buffer.clone();
        let window = window.clone();
        let on_submit = on_submit.clone();
        move || {
            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();

            // Taskwarrior descriptions are single-line, and the picker renders
            // rows through a tab-separated format where a newline would show as
            // a literal escape. The extra room here is for seeing what you
            // type, not for storing shape — so whitespace collapses on the way
            // out.
            let text = crate::text::collapse_whitespace(&text);

            // Nothing typed, or an edit that changed nothing, means do nothing.
            let worth_doing = is_worth_submitting(mode, &text, &original);
            window.close();
            if worth_doing {
                on_submit(text);
            }
        }
    };

    {
        let do_submit = do_submit.clone();
        submit.connect_clicked(move |_| do_submit());
    }
    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }

    // Enter has to insert a newline for a multi-line box to be worth having, so
    // submitting moves to Ctrl+Enter. Escape cancels.
    //
    // The phase matters and defaulting to it was a bug: an EventControllerKey on
    // the window bubbles, so the focused TextView saw Return first, inserted a
    // newline and stopped propagation — Ctrl+Enter did nothing at all. Escape
    // kept working the whole time, because a TextView does not consume that,
    // which is exactly the shape of "only one of the two shortcuts is broken".
    //
    // Capture phase gets the window in first. Everything except the two keys
    // handled here still returns Proceed, so ordinary typing reaches the
    // TextView untouched.
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let window = window.clone();
        let do_submit = do_submit.clone();
        keys.connect_key_pressed(move |_, key, _, state| {
            let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
            match key_action(key, ctrl) {
                KeyAction::Cancel => {
                    window.close();
                    gtk4::glib::Propagation::Stop
                }
                KeyAction::Submit => {
                    do_submit();
                    gtk4::glib::Propagation::Stop
                }
                KeyAction::Ignore => gtk4::glib::Propagation::Proceed,
            }
        });
    }
    window.add_controller(keys);

    window.present();

    // The Wayland app_id comes from the GtkApplication, and inside the daemon
    // that application is the overlay's — so a box opened there arrived as
    // dev.niri-tasks.overlay while one opened by the CLI was dev.niri-tasks.box.
    // Two ids for one window defeats the point of having a stable one at all,
    // and it is set per-toplevel rather than per-application, so set it here
    // once the surface exists.
    if let Some(surface) = window.surface() {
        if let Ok(toplevel) = surface.downcast::<gdk4_wayland::WaylandToplevel>() {
            toplevel.set_application_id(APP_ID);
        }
    }

    input.grab_focus();

    // Put the cursor at the end, so editing starts where you would keep typing
    // rather than in front of the existing text.
    let end = buffer.end_iter();
    buffer.place_cursor(&end);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_match_the_mode() {
        assert_eq!(Mode::Add.title(), "Add Task");
        assert_eq!(Mode::Edit.title(), "Edit Task");
        assert_eq!(Mode::Annotate.title(), "Add Note");
    }

    #[test]
    fn submit_labels_are_distinct() {
        assert_ne!(Mode::Add.submit_label(), Mode::Edit.submit_label());
        assert_ne!(Mode::Edit.submit_label(), Mode::Annotate.submit_label());
    }

    #[test]
    fn ctrl_enter_submits_and_bare_enter_does_not() {
        assert_eq!(key_action(gdk::Key::Return, true), KeyAction::Submit);
        assert_eq!(key_action(gdk::Key::KP_Enter, true), KeyAction::Submit);
        // A bare Return has to reach the text view, or the box cannot be
        // multi-line, which is its only reason to exist.
        assert_eq!(key_action(gdk::Key::Return, false), KeyAction::Ignore);
        assert_eq!(key_action(gdk::Key::KP_Enter, false), KeyAction::Ignore);
    }

    #[test]
    fn escape_cancels_with_or_without_ctrl() {
        assert_eq!(key_action(gdk::Key::Escape, false), KeyAction::Cancel);
        assert_eq!(key_action(gdk::Key::Escape, true), KeyAction::Cancel);
    }

    #[test]
    fn ordinary_typing_is_passed_through() {
        for k in [gdk::Key::a, gdk::Key::space, gdk::Key::Tab, gdk::Key::BackSpace] {
            assert_eq!(key_action(k, false), KeyAction::Ignore);
            // Ctrl+A and friends must still reach the text view, or select-all
            // and the usual editing keys stop working inside the box.
            assert_eq!(key_action(k, true), KeyAction::Ignore);
        }
    }

    #[test]
    fn empty_input_never_submits() {
        for mode in [Mode::Add, Mode::Edit, Mode::Annotate] {
            assert!(!is_worth_submitting(mode, "", "anything"));
        }
    }

    #[test]
    fn an_edit_that_changed_nothing_does_nothing() {
        assert!(!is_worth_submitting(Mode::Edit, "same text", "same text"));
        assert!(is_worth_submitting(Mode::Edit, "new text", "same text"));
    }

    /// Only Edit compares against the original. Re-adding a task whose wording
    /// matches an existing one is a legitimate thing to do, and a note repeating
    /// the description it is attached to is too.
    #[test]
    fn add_and_annotate_ignore_the_original() {
        assert!(is_worth_submitting(Mode::Add, "same text", "same text"));
        assert!(is_worth_submitting(Mode::Annotate, "same text", "same text"));
    }

    /// The box is fixed-size specifically so niri floats it; if these ever
    /// diverge the window tiles instead and the whole point is lost.
    #[test]
    fn notes_variant_is_taller_but_no_wider() {
        assert!(HEIGHT_WITH_NOTES > HEIGHT);
        assert_eq!(WIDTH, 560, "width is shared by both variants");
    }
}
