//! One monitor's task panel: a layer surface on the right edge, tucked away to
//! a peek until the pointer comes over it.
//!
//! ## Why an Overlay, and why the input region
//!
//! The surface is a fixed width, wide enough for an expanded card, and the
//! cards slide inside it. Moving the surface itself instead would mean asking
//! the compositor for a new size or margin on every frame of the slide.
//!
//! The cards ride on a `gtk::Overlay` above an empty base box, because an
//! overlay's children do not count towards its size. In a `gtk::Fixed`, a card
//! pushed out past the right edge would make the window ask to grow by exactly
//! that much, which is the opposite of tucked away.
//!
//! Most of that fixed-width surface is transparent, and it must not swallow
//! clicks meant for the windows beneath. So the surface's input region is kept
//! to where the cards are: the peek column while tucked away, the whole cards
//! while out. Pointer enter and leave follow the region, which is what makes
//! the peek the hover target.
//!
//! The region grows *before* a slide out, so the pointer stays inside it while
//! the cards move under it, and shrinks only *after* a slide back finishes, so
//! a pointer returning mid-slide still counts as hovering.

use super::blur::{self, Blur};
use super::model::{Card, Status};
use super::style::{CARD_WIDTH_PX, GAP_PX, RADIUS_PX};
use gtk4::prelude::*;
use gtk4::{cairo, gdk, glib, Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Layer-shell namespace, so `layer-rule { match namespace="niri-tasks-panel" }`
/// can target the panel.
pub const NAMESPACE: &str = "niri-tasks-panel";

/// How much of each card shows while the panel is tucked away.
pub const PEEK_PX: i32 = 60;

/// Room around the cards for their CSS shadow, which the surface has to
/// contain or it is cut off square.
const SHADOW_PX: i32 = 16;

/// The expanded cards' distance from the screen edge, matching mako's
/// `outer-margin`.
const EDGE_GAP_PX: i32 = 8;

const SURFACE_WIDTH: i32 = SHADOW_PX + CARD_WIDTH_PX + EDGE_GAP_PX;
const EXPANDED_X: f64 = SHADOW_PX as f64;
const TUCKED_X: f64 = (SURFACE_WIDTH - PEEK_PX) as f64;

const SLIDE_MS: f64 = 180.0;

/// How long after the pointer leaves before the cards slide back. Long enough
/// that overshooting the edge of a card does not make the panel flicker.
const GRACE: Duration = Duration::from_millis(400);

pub struct Panel {
    window: ApplicationWindow,
    column: gtk4::Box,
    base: gtk4::Box,
    slide: Rc<Slide>,
    shown: RefCell<Vec<Card>>,
    /// Each card's height, top to bottom, for the blur region.
    heights: RefCell<Vec<i32>>,
    blur: RefCell<Option<Blur>>,
}

/// Where the cards are and where they are going.
struct Slide {
    x: Cell<f64>,
    from: Cell<f64>,
    to: Cell<f64>,
    start_us: Cell<i64>,
    ticking: Cell<bool>,
    grace: Cell<Option<glib::SourceId>>,
    /// The cards' total height, which the input region needs.
    cards_h: Cell<i32>,
}

impl Panel {
    pub fn new(app: &Application, monitor: &gdk::Monitor) -> Rc<Panel> {
        let window = ApplicationWindow::builder().application(app).build();
        window.add_css_class("task-panel");

        window.init_layer_shell();
        window.set_namespace(Some(NAMESPACE));
        // Top, not overlay: a fullscreen window covers the panel.
        window.set_layer(Layer::Top);
        window.set_monitor(Some(monitor));
        // The right edge only, so the compositor centres it vertically, clear
        // of notifications at the top and the dock at the bottom.
        window.set_anchor(Edge::Right, true);
        // Never take focus or reserve space: the panel is a readout, and an
        // exclusive zone would push every window left by its width.
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_exclusive_zone(0);

        let column = gtk4::Box::new(gtk4::Orientation::Vertical, GAP_PX);
        let base = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&base));
        overlay.add_overlay(&column);
        window.set_child(Some(&overlay));

        let panel = Rc::new(Panel {
            window,
            column,
            base,
            slide: Rc::new(Slide {
                x: Cell::new(TUCKED_X),
                from: Cell::new(TUCKED_X),
                to: Cell::new(TUCKED_X),
                start_us: Cell::new(0),
                ticking: Cell::new(false),
                grace: Cell::new(None),
                cards_h: Cell::new(0),
            }),
            shown: RefCell::new(Vec::new()),
            heights: RefCell::new(Vec::new()),
            blur: RefCell::new(None),
        });

        {
            let slide = panel.slide.clone();
            overlay.connect_get_child_position(move |_, _| {
                Some(gdk::Rectangle::new(
                    slide.x.get().round() as i32,
                    SHADOW_PX,
                    CARD_WIDTH_PX,
                    slide.cards_h.get(),
                ))
            });
        }

        let motion = gtk4::EventControllerMotion::new();
        {
            let weak = Rc::downgrade(&panel);
            motion.connect_enter(move |_, _, _| {
                if let Some(p) = weak.upgrade() {
                    p.cancel_grace();
                    p.slide_to(EXPANDED_X);
                }
            });
        }
        {
            let weak = Rc::downgrade(&panel);
            motion.connect_leave(move |_| {
                let Some(p) = weak.upgrade() else { return };
                let weak = weak.clone();
                let id = glib::timeout_add_local_once(GRACE, move || {
                    if let Some(p) = weak.upgrade() {
                        p.slide.grace.set(None);
                        p.slide_to(TUCKED_X);
                    }
                });
                if let Some(old) = p.slide.grace.replace(Some(id)) {
                    old.remove();
                }
            });
        }
        panel.window.add_controller(motion);
        // Cards are read-only. A click on one lands on this surface, because
        // it is inside the input region, and goes no further — no handler
        // needed to stop it reaching the window beneath.

        // GTK may reset the input region when the surface maps or resizes, and
        // a surface shown again may be a new wl_surface, which needs its own
        // blur object — the old one goes first, since niri allows one each.
        {
            let weak = Rc::downgrade(&panel);
            panel.window.connect_map(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.set_region(p.slide.x.get());
                    drop(p.blur.borrow_mut().take());
                    *p.blur.borrow_mut() = Blur::new(&p.window);
                    p.update_blur(p.slide.x.get());
                }
            });
        }

        // Map once, then hide in the same main-loop iteration. A layer surface
        // that has never been mapped ignores a later present(), so a daemon
        // started on an empty workspace would otherwise never show a panel.
        panel.window.present();
        panel.window.set_visible(false);

        panel
    }

    /// Show these cards, or hide the panel when there are none.
    pub fn show(&self, cards: &[Card]) {
        if *self.shown.borrow() == cards {
            return;
        }
        *self.shown.borrow_mut() = cards.to_vec();

        if cards.is_empty() {
            // Only hide something that is up; hiding a never-mapped layer
            // surface leaves it deaf to a later present().
            if self.window.is_visible() {
                self.window.set_visible(false);
            }
            self.cancel_grace();
            self.slide.x.set(TUCKED_X);
            self.slide.to.set(TUCKED_X);
            return;
        }

        while let Some(child) = self.column.first_child() {
            self.column.remove(&child);
        }
        for card in cards {
            self.column.append(&card_label(card));
        }
        // Measured only once they are in the window: a label outside it has no
        // stylesheet, so it measures without its padding, and GTK keeps that
        // wrong size for the column's own measurement too.
        let mut heights = Vec::with_capacity(cards.len());
        let mut child = self.column.first_child();
        while let Some(c) = child {
            heights.push(c.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX).1);
            child = c.next_sibling();
        }
        *self.heights.borrow_mut() = heights;

        let (_, cards_h, _, _) = self.column.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX);
        self.slide.cards_h.set(cards_h);
        let height = cards_h + 2 * SHADOW_PX;
        // Both calls: the size request lets the surface grow, the default size
        // lets it shrink back when the list gets shorter.
        self.base.set_size_request(SURFACE_WIDTH, height);
        self.window.set_size_request(SURFACE_WIDTH, height);
        self.window.set_default_size(SURFACE_WIDTH, height);

        // present(), not set_visible(true): see new().
        self.window.present();
        self.set_region(self.slide.x.get().min(self.slide.to.get()));
        self.update_blur(self.slide.x.get());
    }

    pub fn close(&self) {
        self.cancel_grace();
        self.window.close();
    }

    fn cancel_grace(&self) {
        if let Some(id) = self.slide.grace.take() {
            id.remove();
        }
    }

    /// Accept pointer input only over the cards, from `x` to the screen edge.
    fn set_region(&self, x: f64) {
        let Some(surface) = self.window.surface() else { return };
        let x = x.round() as i32;
        let rect = cairo::RectangleInt::new(x, SHADOW_PX, SURFACE_WIDTH - x, self.slide.cards_h.get());
        surface.set_input_region(Some(&cairo::Region::create_rectangle(&rect)));
    }

    /// Blur behind the cards where they are now, corners and all.
    fn update_blur(&self, x: f64) {
        let mut blur = self.blur.borrow_mut();
        let Some(blur) = blur.as_mut() else { return };
        let x = x.round() as i32;
        let width = CARD_WIDTH_PX.min(SURFACE_WIDTH - x);
        let on_screen = x + CARD_WIDTH_PX <= SURFACE_WIDTH;
        let mut y = SHADOW_PX;
        let mut rects = Vec::new();
        for &h in self.heights.borrow().iter() {
            rects.extend(blur::card_region((x, y, width, h), RADIUS_PX, on_screen));
            y += h + GAP_PX;
        }
        blur.set(&rects);
    }

    fn slide_to(self: &Rc<Self>, target: f64) {
        let slide = &self.slide;
        slide.from.set(slide.x.get());
        slide.to.set(target);
        slide.start_us.set(glib::monotonic_time());
        if target < slide.x.get() {
            // Sliding out: grow the region first, so the pointer stays inside
            // it for the whole slide.
            self.set_region(target);
        }
        if slide.ticking.replace(true) {
            return; // the running tick picks up the new target
        }
        let weak = Rc::downgrade(self);
        self.window.add_tick_callback(move |w, _| {
            let Some(p) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let s = &p.slide;
            let t = ((glib::monotonic_time() - s.start_us.get()) as f64 / 1000.0 / SLIDE_MS).min(1.0);
            let eased = 1.0 - (1.0 - t).powi(3);
            s.x.set(s.from.get() + (s.to.get() - s.from.get()) * eased);
            if let Some(child) = w.child() {
                child.queue_allocate();
            }
            p.update_blur(s.x.get());
            if t >= 1.0 {
                // Tucked away (or fully out): only now fit the region to
                // where the cards ended up.
                p.set_region(s.to.get());
                s.ticking.set(false);
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }
}

fn card_label(card: &Card) -> gtk4::Label {
    let text = glib::markup_escape_text(&card.text);
    let markup = match card.icon() {
        "" => text.to_string(),
        icon => format!("{icon}  {text}"),
    };
    let label = gtk4::Label::new(None);
    label.set_markup(&markup);
    label.add_css_class("task-card");
    match card.status {
        Status::Active => label.add_css_class("active"),
        Status::Blocked => label.add_css_class("blocked"),
        Status::More => label.add_css_class("more"),
        Status::Pending => {}
    }
    label.set_xalign(0.0);
    label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    label.set_single_line_mode(true);
    label.set_size_request(CARD_WIDTH_PX, -1);
    label
}
