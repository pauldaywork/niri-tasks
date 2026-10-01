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
//!
//! ## The keyboard
//!
//! Mod+Alt+Ctrl+T slides the panel out and hands it the keyboard, and every
//! card lays itself out again: its whole description, wrapped, above a row of
//! buttons for the menu's most-used actions. A card is a box holding a body
//! button and that row. Up and Down move
//! between cards, Left, Right and Tab along the focused one, and s, r, e, t
//! and Delete press its Start, Refine, Edit, Stop and Remove by emitting the
//! button's `clicked`, so a key, Enter on the focused button, and a click all
//! take one path. The controller runs in the capture phase, ahead of GTK's
//! own focus chain, which would otherwise walk every button on the panel. The
//! body opens the whole menu. Escape, or anything that runs, hands the
//! keyboard back and folds the cards to one line again.
//!
//! It is held with exclusive keyboard mode throughout, as fuzzel does. niri
//! gives an on-demand layer surface focus only after a click on it, so
//! switching to on-demand once focused would drop the focus at once.

use super::actions::Action;
use super::blur::{self, Blur};
use super::keys::{self, KeyAction};
use super::model::{self, Card, Status};
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

/// How much of each card shows while the panel is tucked away: the card's
/// 12px padding plus about one glyph, so the status icon peeks out and the
/// text stays hidden.
pub const PEEK_PX: i32 = 30;

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
    /// The monitor's connector, which is also niri's output name.
    output: String,
    column: gtk4::Box,
    base: gtk4::Box,
    slide: Rc<Slide>,
    /// Every card, uncapped; what is on screen is `model::cap` of these unless
    /// `expanded`.
    all: RefCell<Vec<Card>>,
    /// The "+N more" card was clicked: show every card until the panel tucks
    /// away again.
    expanded: Cell<bool>,
    /// The panel has the keyboard, from Mod+Alt+Ctrl+T.
    keyboard: Cell<bool>,
    /// The Remove button pressed once and waiting for its second press, which
    /// deletes. Moving the focus off it, or any re-render, puts it back.
    armed: RefCell<Option<gtk4::Button>>,
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
        // Never take focus or reserve space: the panel takes clicks but never
        // keys, and an exclusive zone would push every window left by its width.
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
            output: monitor.connector().map(|c| c.to_string()).unwrap_or_default(),
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
            all: RefCell::new(Vec::new()),
            expanded: Cell::new(false),
            keyboard: Cell::new(false),
            armed: RefCell::new(None),
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
                // The keyboard's panel stays out until the keyboard is done.
                if p.keyboard.get() {
                    return;
                }
                let weak = weak.clone();
                let id = glib::timeout_add_local_once(GRACE, move || {
                    if let Some(p) = weak.upgrade() {
                        p.slide.grace.set(None);
                        p.tuck();
                    }
                });
                if let Some(old) = p.slide.grace.replace(Some(id)) {
                    old.remove();
                }
            });
        }
        panel.window.add_controller(motion);
        // A click on a card lands on this surface, because it is inside the
        // input region, and goes no further than the card's own handler — see
        // card_widget.

        let key_controller = gtk4::EventControllerKey::new();
        // Capture, so the arrows and Tab reach this before the window's own
        // focus chain, which would walk every button on the panel in turn
        // rather than between cards, or along one.
        key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
        {
            let weak = Rc::downgrade(&panel);
            key_controller.connect_key_pressed(move |_, key, _, state| {
                // Ctrl, Alt and Super chords belong to the compositor and the
                // focused widget, not to the letters: Ctrl+T must not Stop.
                if state.intersects(
                    gdk::ModifierType::CONTROL_MASK
                        | gdk::ModifierType::ALT_MASK
                        | gdk::ModifierType::SUPER_MASK,
                ) {
                    return glib::Propagation::Proceed;
                }
                let Some(p) = weak.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                match keys::key_action(key) {
                    KeyAction::Ignore => glib::Propagation::Proceed,
                    action => {
                        p.key(action);
                        glib::Propagation::Stop
                    }
                }
            });
        }
        panel.window.add_controller(key_controller);

        // Moving off an armed Remove disarms it.
        {
            let weak = Rc::downgrade(&panel);
            panel.window.connect_notify_local(Some("focus-widget"), move |_, _| {
                if let Some(p) = weak.upgrade() {
                    p.disarm_unless_focused();
                }
            });
        }

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

    /// Show these cards, capped, or hide the panel when there are none.
    pub fn show(self: &Rc<Self>, cards: &[Card]) {
        if *self.all.borrow() == cards {
            return;
        }
        *self.all.borrow_mut() = cards.to_vec();
        self.render();
    }

    /// Slide out and take the keyboard, every card wrapped with its buttons,
    /// focusing the first. False when there are no cards to take it for.
    pub fn take_keyboard(self: &Rc<Self>) -> bool {
        if self.all.borrow().is_empty() {
            return false;
        }
        self.keyboard.set(true);
        self.cancel_grace();
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.render();
        self.slide_to(EXPANDED_X);
        if let Some(first) = self.column.first_child() {
            focus_card(&first, None);
        }
        true
    }

    /// Give the keyboard back, folding every card to its one line again as
    /// the panel tucks away.
    fn release_keyboard(self: &Rc<Self>) {
        if !self.keyboard.replace(false) {
            return;
        }
        self.window.set_keyboard_mode(KeyboardMode::None);
        self.expanded.set(false);
        self.render();
        self.slide_to(TUCKED_X);
    }

    /// Slide back, folding an expanded list up again.
    fn tuck(self: &Rc<Self>) {
        self.slide_to(TUCKED_X);
        if self.expanded.replace(false) {
            self.render();
        }
    }

    /// Show every card in place of "+N more", focusing the first one it hid.
    fn expand(self: &Rc<Self>) {
        self.expanded.set(true);
        self.render();
        let mut child = self.column.first_child();
        for _ in 0..model::CAP {
            child = child.and_then(|c| c.next_sibling());
        }
        if let Some(c) = child {
            focus_card(&c, None);
        }
    }

    fn render(self: &Rc<Self>) {
        let cards = if self.expanded.get() {
            self.all.borrow().clone()
        } else {
            model::cap(&self.all.borrow(), model::CAP)
        };

        if cards.is_empty() {
            if self.keyboard.replace(false) {
                self.window.set_keyboard_mode(KeyboardMode::None);
            }
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

        // A refresh while the keyboard is on the panel keeps focus on the same
        // task, and on the same button of it: each card is named after its
        // task's uuid, and each button after its action.
        let focused = GtkWindowExt::focus(&self.window)
            .and_then(|w| Some((self.card_of(&w)?.widget_name(), w.widget_name())));
        // The buttons go with the widgets they were armed on.
        self.armed.replace(None);
        while let Some(child) = self.column.first_child() {
            self.column.remove(&child);
        }
        for card in &cards {
            self.column.append(&self.card_widget(card));
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

        if self.keyboard.get() {
            let mut child = self.column.first_child();
            while let Some(c) = &child {
                if Some(c.widget_name()) == focused.as_ref().map(|(card, _)| card.clone()) {
                    break;
                }
                child = c.next_sibling();
            }
            if let Some(c) = child.or_else(|| self.column.first_child()) {
                focus_card(&c, focused.as_ref().map(|(_, slot)| slot.as_str()));
            }
        }
    }

    /// One card: a box named after its task's uuid, so render() can put focus
    /// back on it, holding the body, a button so the keyboard can focus and
    /// press it, and, while the panel has the keyboard, its action row along
    /// the bottom. The body opens the task's whole menu, or shows the rest in
    /// place of "+N more".
    fn card_widget(self: &Rc<Self>, card: &Card) -> gtk4::Box {
        let keyboard = self.keyboard.get();
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        widget.add_css_class("task-card");
        match card.status {
            Status::Active => widget.add_css_class("active"),
            Status::Blocked => widget.add_css_class("blocked"),
            Status::Planned => widget.add_css_class("planned"),
            Status::More => widget.add_css_class("more"),
            Status::Pending => {}
        }
        widget.set_size_request(CARD_WIDTH_PX, -1);
        // Clips the action row to the card's rounded bottom corners.
        widget.set_overflow(gtk4::Overflow::Hidden);
        widget.set_widget_name(card.uuid.as_deref().unwrap_or("more"));

        let body = gtk4::Button::builder().child(&card_label(card, keyboard)).build();
        body.add_css_class("card-body");
        body.set_widget_name("body");
        // A mouse click opens the menu without leaving the card darkened.
        body.set_focus_on_click(false);
        {
            let weak = Rc::downgrade(self);
            let uuid = card.uuid.clone();
            body.connect_clicked(move |_| {
                let Some(p) = weak.upgrade() else { return };
                match &uuid {
                    Some(uuid) => {
                        p.release_keyboard();
                        open_menu(&p.output, &["task".into(), "menu".into(), uuid.clone()]);
                    }
                    None => p.expand(),
                }
            });
        }
        widget.append(&body);

        let Some(uuid) = card.uuid.as_ref().filter(|_| keyboard) else {
            return widget;
        };
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        row.add_css_class("card-actions");
        row.set_homogeneous(true);
        for action in Action::for_status(card.status) {
            let button = gtk4::Button::with_label(action.label());
            button.add_css_class(action.name());
            button.set_widget_name(action.name());
            let weak = Rc::downgrade(self);
            let uuid = uuid.clone();
            button.connect_clicked(move |b| {
                if let Some(p) = weak.upgrade() {
                    p.press(action, &uuid, b);
                }
            });
            row.append(&button);
        }
        widget.append(&row);
        widget
    }

    /// Press one of a card's buttons. Remove only arms itself the first time,
    /// as the menu's delete asks "delete?" first; the second press runs it.
    /// Everything that runs gives the keyboard back first, so the box or
    /// terminal it opens can take it.
    fn press(self: &Rc<Self>, action: Action, uuid: &str, button: &gtk4::Button) {
        if action == Action::Remove && self.armed.borrow().as_ref() != Some(button) {
            // Focus first: the move disarms whatever was armed before, and
            // this one is armed only after it.
            button.grab_focus();
            button.set_label(Action::CONFIRM_REMOVE);
            button.add_css_class("confirm");
            *self.armed.borrow_mut() = Some(button.clone());
            return;
        }
        self.release_keyboard();
        open_menu(&self.output, &action.args(uuid));
    }

    /// Act on a key while the panel has the keyboard. Up and Down land on the
    /// next card's body, Left, Right and Tab move along the focused card, and
    /// a letter presses that card's button. If the card has no such button
    /// (Stop on a task that is not active, anything on "+N more"), nothing
    /// happens.
    fn key(self: &Rc<Self>, action: KeyAction) {
        if action == KeyAction::Release {
            self.release_keyboard();
            return;
        }
        let mut cards = Vec::new();
        let mut child = self.column.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            cards.push(c);
        }
        let focus = GtkWindowExt::focus(&self.window);
        let Some(card) = focus
            .as_ref()
            .and_then(|f| self.card_of(f))
            .or_else(|| cards.first().cloned())
        else {
            return;
        };
        let at = cards.iter().position(|c| *c == card).unwrap_or(0);
        let slots = slots(&card);
        let slot = focus
            .as_ref()
            .and_then(|f| slots.iter().position(|s| s == f))
            .unwrap_or(0);

        match action {
            KeyAction::PrevCard | KeyAction::NextCard => {
                let to = keys::step(at, cards.len(), action == KeyAction::NextCard);
                focus_card(&cards[to], None);
            }
            KeyAction::PrevSlot | KeyAction::NextSlot => {
                let to = keys::step(slot, slots.len(), action == KeyAction::NextSlot);
                slots[to].grab_focus();
            }
            KeyAction::Run(run) => {
                // Through the button, so a key does exactly what a click does.
                if let Some(button) = slots
                    .iter()
                    .find(|s| s.widget_name() == run.name())
                    .and_then(|s| s.downcast_ref::<gtk4::Button>())
                {
                    button.emit_clicked();
                }
            }
            KeyAction::Release | KeyAction::Ignore => {}
        }
    }

    /// Moving the focus away from an armed Remove puts it back to Remove, so a
    /// later press starts over at the first press.
    fn disarm_unless_focused(&self) {
        let focus = GtkWindowExt::focus(&self.window);
        let Some(button) = self.armed.take() else { return };
        if focus.as_ref() == Some(button.upcast_ref()) {
            *self.armed.borrow_mut() = Some(button);
            return;
        }
        button.set_label(Action::Remove.label());
        button.remove_css_class("confirm");
    }

    /// The card a widget is in: the one of the column's children it sits
    /// inside.
    fn card_of(&self, widget: &gtk4::Widget) -> Option<gtk4::Widget> {
        let column: &gtk4::Widget = self.column.upcast_ref();
        let mut w = widget.clone();
        loop {
            let parent = w.parent()?;
            if &parent == column {
                return Some(w);
            }
            w = parent;
        }
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

/// The card's icon and description: one line cut off with "…" for the peek
/// and the hover, or all of it, wrapped, while the panel has the keyboard.
fn card_label(card: &Card, wrap: bool) -> gtk4::Label {
    let text = glib::markup_escape_text(&card.text);
    let markup = match card.icon() {
        "" => text.to_string(),
        icon => format!("{icon}  {text}"),
    };
    let label = gtk4::Label::new(None);
    label.set_markup(&markup);
    label.set_xalign(0.0);
    if wrap {
        label.set_wrap(true);
        // WordChar: a long path or URL with no spaces still breaks.
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    } else {
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        label.set_single_line_mode(true);
    }
    label
}

/// What the keyboard can stop on in a card, left to right: its body, then its
/// action buttons.
fn slots(card: &gtk4::Widget) -> Vec<gtk4::Widget> {
    let mut slots = Vec::new();
    let Some(body) = card.first_child() else { return slots };
    if let Some(row) = body.next_sibling() {
        let mut child = row.first_child();
        while let Some(button) = child {
            child = button.next_sibling();
            slots.push(button);
        }
    }
    slots.insert(0, body);
    slots
}

/// Focus the slot in `card` with this widget name, or its body when it has
/// none: a re-render that dropped the button, such as Stop on a task that
/// just stopped.
fn focus_card(card: &gtk4::Widget, slot: Option<&str>) {
    let slots = slots(card);
    let target = slot
        .and_then(|name| slots.iter().find(|s| s.widget_name() == name))
        .or_else(|| slots.first());
    if let Some(target) = target {
        target.grab_focus();
    }
}
/// Run a `niritasks` command on this monitor: a task's action menu, the
/// fuzzel list, or one of a card's buttons.
///
/// This monitor is focused first. The menu then files under the workspace the
/// panel shows, rather than whichever monitor had focus, and fuzzel opens on
/// the screen that was clicked. It runs as its own `niritasks` process,
/// spawned by niri: fuzzel blocks until it closes, and the daemon must not.
pub fn open_menu(output: &str, args: &[String]) {
    let result = std::env::current_exe()
        .map_err(anyhow::Error::from)
        .and_then(|exe| {
            crate::niri::focus_monitor(output)?;
            let mut command = vec![exe.to_string_lossy().into_owned()];
            command.extend_from_slice(args);
            crate::niri::spawn(command)
        });
    if let Err(e) = result {
        crate::notify::tasks(&e.to_string());
    }
}
