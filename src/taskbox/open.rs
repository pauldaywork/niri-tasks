//! One task box at a time: what a box is open on, what a second request does
//! while one is open, and bringing the open box forward through niri.
//!
//! Pure decision (`opened`) apart from the GTK and niri calls, so it is tested
//! on its own.

use super::{build_window, BoxConfig, Submission, APP_ID};
use crate::niri;
use gtk4::glib::WeakRef;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use std::cell::RefCell;
use std::rc::Rc;

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
    /// The box on screen and what it is open on, so a second request brings it
    /// forward rather than opening another box over it. Weak, so this never
    /// keeps a closed box alive; the window's destroy handler empties it, and
    /// a reference that no longer upgrades counts as no box either. Only ever
    /// touched on the GTK main thread.
    static OPEN: RefCell<Option<(WeakRef<ApplicationWindow>, Subject)>> =
        const { RefCell::new(None) };
}

#[cfg(test)]
mod tests {
    use super::*;

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
