//! Running something after GTK's next frame has been painted, when everything
//! queued before it has been measured and placed.
//!
//! The task box settles its rows with it and the panel scrolls its focused
//! card into view with it; it lives on its own so neither window imports the
//! other.

use gtk4::gdk;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Run `f` once, after the window's next frame has been painted — by which
/// point everything queued before it has been measured and placed.
pub fn after_next_paint(window: &gtk4::ApplicationWindow, f: impl FnOnce() + 'static) {
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
