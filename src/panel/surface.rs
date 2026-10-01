//! One monitor's task panel: a layer surface on the right edge, tucked away to
//! a peek until the pointer comes over it, or in the middle of the screen
//! while it has the keyboard.
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
//! Mod+Alt+Ctrl+T hands the panel the keyboard in the middle of the screen,
//! and every card lays itself out again: its whole description, wrapped, above
//! a row of buttons for the menu's most-used actions. A card is a box holding
//! a body button and that row. Up and Down move between cards, Left, Right and
//! Tab along the focused one, and s, r, e, t and Delete press its Start,
//! Refine, Edit, Stop and Remove by emitting the button's `clicked`, so a key,
//! Enter on the focused button, and a click all take one path. The controller
//! runs in the capture phase, ahead of GTK's own focus chain, which would
//! otherwise walk every button on the panel. The focused card is darkened. The
//! body opens the whole menu. Escape, or anything that runs, hands the
//! keyboard back, folds the cards to one line again, and puts the panel back
//! on the right edge as a peek.
//!
//! Above the cards, a bar of filter tabs, All, Active, Planned, To refine and
//! Waiting, narrows them to the tasks it names. A tab shows only while it has
//! a task under it, All apart. 1 to 5 pick one, always the same one, and do
//! nothing for a hidden tab; [ and ] step along the shown ones, stopping at
//! the ends; a click picks one too. The tabs never take focus, so the arrows
//! and Tab still move only between cards and buttons. The panel takes the
//! keyboard on All every time; a refresh keeps the tab, falling back to All
//! once it has nothing left, and focus falls back to the first card when the
//! one it was on has left it.
//!
//! Waiting tasks are on the Waiting tab and nowhere else: not on All, not on
//! the hover or the peek, so a workspace whose tasks are all waiting shows
//! nothing on its edge. With the keyboard it opens on All, which then says it
//! has no tasks, beside the Waiting tab.
//!
//! Wrapped, the cards can stand taller than the screen, so the column sits in
//! a scroller under the tabs, capped at the screen's height less its margins
//! and the tabs. Moving the focus scrolls the focused card wholly into view,
//! and the blur region moves with the scroll and stops at the view's edges.
//!
//! To sit in the middle, the surface keeps its right anchor, which centres it
//! vertically, and grows its right margin to half the room it leaves on the
//! monitor. The margin rides on the cards' own slide, so the cards slide out
//! from the peek all the way to the middle of the screen, ending in the middle
//! of the surface with equal margins either side. Moving the surface means a
//! new margin for the compositor on every frame of that one slide; the hover's
//! slides still move only the cards. Giving the keyboard back snaps both
//! back, since the box or terminal it opened should not wait on a slide. One
//! surface means no peek on the right edge while it is centred, and the
//! pointer coming over the centred cards does not slide them.
//!
//! It is held with exclusive keyboard mode throughout, as fuzzel does. niri
//! gives an on-demand layer surface focus only after a click on it, so
//! switching to on-demand once focused would drop the focus at once.

use super::actions::Action;
use super::blur::{self, Blur};
use super::keys::{self, KeyAction};
use super::model::{self, Card, Filter, Status};
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

/// The spread of each card's outline ring (style.rs's `OUTLINE`), which is
/// drawn outside the card's box. The scroller clips its content, so the column
/// keeps this much margin inside it and the scroller is this much bigger than
/// the cards on every side; SHADOW_PX already leaves the surface room for it.
const RING_PX: i32 = 4;

/// The expanded cards' distance from the screen edge, matching mako's
/// `outer-margin`.
const EDGE_GAP_PX: i32 = 8;

const SURFACE_WIDTH: i32 = SHADOW_PX + CARD_WIDTH_PX + EDGE_GAP_PX;
const EXPANDED_X: f64 = SHADOW_PX as f64;
const TUCKED_X: f64 = (SURFACE_WIDTH - PEEK_PX) as f64;

/// Where the cards sit while the panel has the keyboard and the surface is
/// pushed into the middle of the monitor by its right margin: the middle of
/// the surface, so the cards are in the middle of the screen. 784 less 760
/// leaves 12px each side, enough for the ring.
const CENTRED_X: f64 = ((SURFACE_WIDTH - CARD_WIDTH_PX) / 2) as f64;

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
    /// Scrolls the column once it is taller than the screen.
    scroller: gtk4::ScrolledWindow,
    /// The filter tabs above the scroller, shown only while the panel has the
    /// keyboard. It does not scroll with the cards.
    tabs: gtk4::Box,
    /// One button per filter tab, in `Filter::TABS` order, for which ones show
    /// and which one is picked.
    tab_buttons: Vec<gtk4::Button>,
    /// For its height, which caps the column's: read at each render, since a
    /// scale change alters it.
    monitor: gdk::Monitor,
    slide: Rc<Slide>,
    /// Every card, uncapped; what is on screen is `model::cap` of these unless
    /// `expanded`.
    all: RefCell<Vec<Card>>,
    /// The "+N more" card was clicked: show every card until the panel tucks
    /// away again.
    expanded: Cell<bool>,
    /// The panel has the keyboard, from Mod+Alt+Ctrl+T.
    keyboard: Cell<bool>,
    /// The filter tab the keyboard's panel shows. All whenever the panel
    /// takes the keyboard; kept across a refresh while it has it.
    filter: Cell<Filter>,
    /// The live herdr agents in the workspace's session, asked once as the
    /// panel takes the keyboard: which cards get Go to session. Asked then
    /// and no other time — not on each focus move, which would move the row
    /// under the user and spawn herdr on every key, and not on the tucked
    /// refresh, whose cards show no buttons.
    agents: RefCell<Vec<String>>,
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
    /// The cards' height on screen: all of them, or the scroller's when they
    /// run past the screen. The input region needs it.
    cards_h: Cell<i32>,
    /// The filter tabs' height, with the gap under them: what the cards sit
    /// below. 0 without the keyboard, which has no tabs.
    tabs_h: Cell<i32>,
    /// The surface's right margin, which slides it off the screen edge into
    /// the middle while the panel has the keyboard, and where it is going.
    margin: Cell<i32>,
    margin_from: Cell<i32>,
    margin_to: Cell<i32>,
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
        column.set_margin_top(RING_PX);
        column.set_margin_bottom(RING_PX);
        column.set_margin_start(RING_PX);
        column.set_margin_end(RING_PX);
        let base = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&base));
        // A viewport made by hand, to turn off its own scroll-to-focus. That
        // would show only the focused button, leaving the rest of its card
        // off screen; follow_focus scrolls the whole card into view instead.
        let viewport = gtk4::Viewport::new(None::<&gtk4::Adjustment>, None::<&gtk4::Adjustment>);
        viewport.set_scroll_to_focus(false);
        viewport.set_child(Some(&column));
        let scroller = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            // External still scrolls (follow_focus, the wheel) but draws no
            // bar over the Remove button or out of the 760px column's width.
            .vscrollbar_policy(gtk4::PolicyType::External)
            .child(&viewport)
            .build();
        // The filter tabs, over the scroller rather than in it, so they stay
        // put while the cards scroll. Ring room on three sides, as the column
        // keeps, and under them the card gap less the ring the column keeps
        // above the first card: the first card then sits a card gap below.
        let tabs = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        tabs.add_css_class("filter-tabs");
        tabs.set_margin_top(RING_PX);
        tabs.set_margin_start(RING_PX);
        tabs.set_margin_end(RING_PX);
        tabs.set_margin_bottom(GAP_PX - RING_PX);
        // Clips the picked tab's fill to the bar's rounded corners.
        tabs.set_overflow(gtk4::Overflow::Hidden);
        tabs.set_visible(false);
        let tab_buttons: Vec<gtk4::Button> = Filter::TABS
            .iter()
            .map(|filter| {
                let button = gtk4::Button::with_label(filter.label());
                // Out of the focus chain: the arrows and Tab stay between the
                // cards and their buttons, and a click does not take the
                // focus.
                button.set_focusable(false);
                button.set_focus_on_click(false);
                tabs.append(&button);
                button
            })
            .collect();
        scroller.set_vexpand(true);
        let front = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        front.append(&tabs);
        front.append(&scroller);
        overlay.add_overlay(&front);
        window.set_child(Some(&overlay));

        let panel = Rc::new(Panel {
            window,
            output: monitor.connector().map(|c| c.to_string()).unwrap_or_default(),
            monitor: monitor.clone(),
            column,
            base,
            scroller,
            tabs,
            tab_buttons,
            slide: Rc::new(Slide {
                x: Cell::new(TUCKED_X),
                from: Cell::new(TUCKED_X),
                to: Cell::new(TUCKED_X),
                start_us: Cell::new(0),
                ticking: Cell::new(false),
                grace: Cell::new(None),
                cards_h: Cell::new(0),
                tabs_h: Cell::new(0),
                margin: Cell::new(0),
                margin_from: Cell::new(0),
                margin_to: Cell::new(0),
            }),
            all: RefCell::new(Vec::new()),
            expanded: Cell::new(false),
            keyboard: Cell::new(false),
            filter: Cell::new(Filter::All),
            agents: RefCell::new(Vec::new()),
            armed: RefCell::new(None),
            heights: RefCell::new(Vec::new()),
            blur: RefCell::new(None),
        });

        {
            let slide = panel.slide.clone();
            overlay.connect_get_child_position(move |_, _| {
                Some(gdk::Rectangle::new(
                    slide.x.get().round() as i32 - RING_PX,
                    SHADOW_PX - RING_PX,
                    CARD_WIDTH_PX + 2 * RING_PX,
                    slide.tabs_h.get() + slide.cards_h.get() + 2 * RING_PX,
                ))
            });
        }

        for (button, filter) in panel.tab_buttons.iter().zip(Filter::TABS) {
            let weak = Rc::downgrade(&panel);
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.pick(filter);
                }
            });
        }

        let motion = gtk4::EventControllerMotion::new();
        {
            let weak = Rc::downgrade(&panel);
            motion.connect_enter(move |_, _, _| {
                let Some(p) = weak.upgrade() else { return };
                // The keyboard's panel is in the middle and stays put.
                if p.keyboard.get() {
                    return;
                }
                p.cancel_grace();
                p.slide_to(EXPANDED_X, 0);
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

        // The blur region is in surface coordinates, so it moves with the scroll.
        {
            let weak = Rc::downgrade(&panel);
            panel.scroller.vadjustment().connect_value_changed(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.update_blur(p.slide.x.get());
                }
            });
        }

        // Moving off an armed Remove disarms it.
        {
            let weak = Rc::downgrade(&panel);
            panel.window.connect_notify_local(Some("focus-widget"), move |_, _| {
                if let Some(p) = weak.upgrade() {
                    p.disarm_unless_focused();
                    p.follow_focus();
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

    /// Take the keyboard in the middle of the monitor, every card wrapped with
    /// its buttons, focusing the first. `agents` are the session's live agent
    /// names, for which cards get Go to session. False when there are no cards
    /// to take it for.
    ///
    /// The right anchor alone already centres the surface vertically; its
    /// right margin grows to half the room the surface leaves, centring it
    /// across too. The margin rides on the same slide as the cards, so they
    /// slide out from the peek all the way to the middle. The peek goes with
    /// it: there is one surface, and it is in the middle now.
    pub fn take_keyboard(self: &Rc<Self>, agents: Vec<String>) -> bool {
        if self.all.borrow().is_empty() {
            return false;
        }
        *self.agents.borrow_mut() = agents;
        self.keyboard.set(true);
        self.cancel_grace();
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        // Opens on All every time, also when pressed again while open.
        self.filter.set(Filter::All);
        self.render();
        // After render(), which measures the cards the region needs.
        self.slide_to(CENTRED_X, centre_margin(self.monitor.geometry().width()));
        if let Some(first) = self.column.first_child() {
            focus_card(&first, None);
        }
        true
    }

    /// Give the keyboard back, folding every card to its one line again and
    /// putting the panel back on the right edge, tucked away to its peek. The
    /// cards snap there rather than slide, so whatever the keyboard opened
    /// does not wait on them crossing half the screen.
    fn release_keyboard(self: &Rc<Self>) {
        if !self.keyboard.replace(false) {
            return;
        }
        self.window.set_keyboard_mode(KeyboardMode::None);
        self.agents.borrow_mut().clear();
        self.expanded.set(false);
        // The next Mod+Alt+Ctrl+T opens on All.
        self.filter.set(Filter::All);
        self.render();
        self.jump_to(TUCKED_X, 0);
    }

    /// Slide back, folding an expanded list up again.
    fn tuck(self: &Rc<Self>) {
        self.slide_to(TUCKED_X, 0);
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

    /// Show the cards under this filter tab. Focus stays on the card it was
    /// on when the tab has it, and goes to the first card otherwise, as on a
    /// refresh. Nothing without the keyboard, whose panel has no tabs, or for
    /// a tab hidden for having no tasks.
    fn pick(self: &Rc<Self>, filter: Filter) {
        if !self.keyboard.get() || !Filter::shown(&self.all.borrow()).contains(&filter) {
            return;
        }
        if self.filter.replace(filter) != filter {
            self.render();
        }
    }

    /// Which tabs show, and which one is picked.
    fn update_tabs(&self) {
        let shown = Filter::shown(&self.all.borrow());
        for (button, filter) in self.tab_buttons.iter().zip(Filter::TABS) {
            button.set_visible(shown.contains(&filter));
            if filter == self.filter.get() {
                button.add_css_class("current");
            } else {
                button.remove_css_class("current");
            }
        }
    }

    /// Measure the cards and the tabs, and size the surface to them: the
    /// heights the blur region and the input region work from. Run by every
    /// render, and again whenever a card's action row shows or hides, which
    /// changes its height without a render.
    fn fit(&self) {
        let keyboard = self.keyboard.get();
        // Measured only once they are in the window: a label outside it has no
        // stylesheet, so it measures without its padding, and GTK keeps that
        // wrong size for the column's own measurement too.
        let mut heights = Vec::new();
        let mut child = self.column.first_child();
        while let Some(c) = child {
            heights.push(c.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX).1);
            child = c.next_sibling();
        }
        *self.heights.borrow_mut() = heights;

        // Measured, like the cards, once in the window; margins included,
        // so this is the bar and the gap under it.
        let tabs_h = if keyboard {
            self.tabs.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX).1
        } else {
            0
        };

        // The column's margins are in what it measures, and in the width it
        // is measured for; the cards' height is without them.
        let (_, with_ring, _, _) =
            self.column.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX);
        let cards_h = with_ring - 2 * RING_PX;
        let shown = shown_height(cards_h, tabs_h, self.monitor.geometry().height());
        self.slide.tabs_h.set(tabs_h);
        self.slide.cards_h.set(shown);
        let height = tabs_h + shown + 2 * SHADOW_PX;
        // Both calls: the size request lets the surface grow, the default size
        // lets it shrink back when the list gets shorter.
        self.base.set_size_request(SURFACE_WIDTH, height);
        self.window.set_size_request(SURFACE_WIDTH, height);
        self.window.set_default_size(SURFACE_WIDTH, height);
    }

    fn render(self: &Rc<Self>) {
        let keyboard = self.keyboard.get();
        // The hover and the peek show no waiting task, so they hide with
        // nothing else to show; the keyboard's panel still has the Waiting
        // tab, and hides only with no task at all.
        let nothing = if keyboard {
            self.all.borrow().is_empty()
        } else {
            Filter::All.pick(&self.all.borrow()).is_empty()
        };
        if nothing {
            // The last task went while the panel had the keyboard: back to
            // the right edge, so the next card shows as a peek there, and the
            // next Mod+Alt+Ctrl+T opens on All, as after Escape.
            if self.keyboard.replace(false) {
                self.window.set_keyboard_mode(KeyboardMode::None);
                self.filter.set(Filter::All);
            }
            // Only hide something that is up; hiding a never-mapped layer
            // surface leaves it deaf to a later present().
            if self.window.is_visible() {
                self.window.set_visible(false);
            }
            self.cancel_grace();
            self.jump_to(TUCKED_X, 0);
            return;
        }

        // A tab shows only while it has tasks, so once the last one under the
        // picked tab goes (started, off Planned, say), the panel is on All.
        if keyboard && !Filter::shown(&self.all.borrow()).contains(&self.filter.get()) {
            self.filter.set(Filter::All);
        }
        // Only the keyboard's panel has tabs; the peek and the hover show
        // All's cards. Filtered before the cap, so "+N more" is the rest of
        // this tab.
        let filter = if keyboard { self.filter.get() } else { Filter::All };
        let picked = filter.pick(&self.all.borrow());
        let cards = if self.expanded.get() {
            picked
        } else {
            model::cap(&picked, model::CAP)
        };

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
        if cards.is_empty() {
            // Only All, with every task waiting: it says so, and the
            // Waiting tab beside it has them.
            self.column.append(&empty_line(filter));
        }
        self.tabs.set_visible(keyboard);
        self.update_tabs();
        self.fit();

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
            // The slot only goes with the same card: on the first card, which
            // stands in when the focused one left, it would land on a button
            // of that name rather than on the body.
            match child {
                Some(c) => focus_card(&c, focused.as_ref().map(|(_, slot)| slot.as_str())),
                None => {
                    if let Some(first) = self.column.first_child() {
                        focus_card(&first, None);
                    }
                }
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
            Status::Waiting => widget.add_css_class("waiting"),
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
        // Left-aligned and only as wide as its icons, not spread across the card.
        row.set_halign(gtk4::Align::Start);
        // After the icons, the focused one's name in words: the icons alone do
        // not say what they do. Empty while the focus is on the description.
        let hint = gtk4::Label::new(None);
        hint.add_css_class("card-hint");
        let has_session = crate::link::session_agent(&self.agents.borrow(), uuid).is_some();
        for action in Action::for_status(card.status, has_session) {
            let button = gtk4::Button::with_label(action.icon());
            // The icon's name in words, under the pointer and for a screen reader.
            button.set_tooltip_text(Some(action.label()));
            button.add_css_class(action.name());
            button.set_widget_name(action.name());
            let weak = Rc::downgrade(self);
            let uuid = uuid.clone();
            button.connect_clicked(move |b| {
                if let Some(p) = weak.upgrade() {
                    p.press(action, &uuid, b);
                }
            });
            let focus = gtk4::EventControllerFocus::new();
            {
                let hint = hint.downgrade();
                focus.connect_enter(move |_| {
                    if let Some(hint) = hint.upgrade() {
                        hint.set_label(action.label());
                    }
                });
            }
            {
                let hint = hint.downgrade();
                focus.connect_leave(move |_| {
                    if let Some(hint) = hint.upgrade() {
                        hint.set_label("");
                    }
                });
            }
            button.add_controller(focus);
            row.append(&button);
        }
        row.append(&hint);
        // A line between the description and the buttons.
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        separator.add_css_class("card-separator");
        widget.append(&separator);
        widget.append(&row);
        widget
    }

    /// Press one of a card's buttons. Remove only arms itself the first time,
    /// as the menu's delete asks "delete?" first; the second press runs it.
    /// Everything that opens something gives the keyboard back first, so the
    /// box or terminal it opens can take it. Waiting and Remove open nothing:
    /// the list stays up, focus moving to the next card (the one above, from
    /// the last) so it is still there when the next tick drops this one.
    fn press(self: &Rc<Self>, action: Action, uuid: &str, button: &gtk4::Button) {
        if action == Action::Remove && self.armed.borrow().as_ref() != Some(button) {
            // Focus first: the move disarms whatever was armed before, and
            // this one is armed only after it.
            button.grab_focus();
            button.set_label(&format!("{}  {}", Action::Remove.icon(), Action::CONFIRM_REMOVE));
            button.add_css_class("confirm");
            *self.armed.borrow_mut() = Some(button.clone());
            return;
        }
        if action.keeps_keyboard() {
            let neighbour = self
                .card_of(button.upcast_ref())
                .and_then(|c| c.next_sibling().or_else(|| c.prev_sibling()));
            if let Some(c) = neighbour {
                focus_card(&c, None);
            }
        } else {
            self.release_keyboard();
        }
        open_menu(&self.output, &action.args(uuid));
    }

    /// Act on a key while the panel has the keyboard. Up and Down land on the
    /// next card's body, Left, Right and Tab move along the focused card, and
    /// a letter presses that card's button. If the card has no such button
    /// (Stop on a task that is not active, Go to session on one with no Claude,
    /// anything on "+N more"), nothing happens. 1 to 5, [ and ] pick a filter
    /// tab instead, whatever has focus.
    fn key(self: &Rc<Self>, action: KeyAction) {
        match action {
            KeyAction::Release => return self.release_keyboard(),
            KeyAction::Filter(filter) => return self.pick(filter),
            KeyAction::PrevFilter | KeyAction::NextFilter => {
                // Along the tabs on show, skipping the hidden ones.
                let shown = Filter::shown(&self.all.borrow());
                let at = shown.iter().position(|f| *f == self.filter.get()).unwrap_or(0);
                let to = keys::step(at, shown.len(), action == KeyAction::NextFilter);
                return self.pick(shown[to]);
            }
            _ => {}
        }
        let mut cards = Vec::new();
        let mut child = self.column.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            // Not the line All shows when every task is waiting, which has
            // nothing to focus.
            if c.has_css_class("task-card") {
                cards.push(c);
            }
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
            KeyAction::Release
            | KeyAction::Ignore
            | KeyAction::Filter(_)
            | KeyAction::PrevFilter
            | KeyAction::NextFilter => {}
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
        button.set_label(Action::Remove.icon());
        button.remove_css_class("confirm");
    }

    /// Scroll the focused card wholly into view, body and buttons, once the
    /// frame after the move has laid it out. A card just rendered has no place
    /// in the column until then.
    fn follow_focus(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        crate::taskbox::after_next_paint(&self.window, move || {
            let Some(p) = weak.upgrade() else { return };
            let Some(focus) = GtkWindowExt::focus(&p.window) else { return };
            let Some(card) = p.card_of(&focus) else { return };
            let Some(bounds) = card.compute_bounds(&p.column) else { return };
            let adjustment = p.scroller.vadjustment();
            // Bounds are in the column; the viewport's content starts RING_PX
            // above it, and the card's ring takes RING_PX on each side. So
            // the card's ring box runs from its top to that plus its height
            // and both rings, and scrolled there the card sits in the band.
            let top = bounds.y() as f64;
            let bottom = top + bounds.height() as f64 + 2.0 * RING_PX as f64;
            adjustment.set_value(scroll_to_show(adjustment.value(), adjustment.page_size(), top, bottom));
        });
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

    /// Accept pointer input only over the cards, from `x`. Tucked away or
    /// sliding, that runs to the screen edge, which keeps the hover alive
    /// there; centred, it stops at the cards, so the strip beside them in the
    /// middle of the screen does not swallow clicks.
    fn set_region(&self, x: f64) {
        let Some(surface) = self.window.surface() else { return };
        let x = x.round() as i32;
        let width = region_width(x, self.keyboard.get());
        // From the top of the tabs, when there are any, to the bottom of the
        // cards on screen.
        let height = self.slide.tabs_h.get() + self.slide.cards_h.get();
        let rect = cairo::RectangleInt::new(x, SHADOW_PX, width, height);
        surface.set_input_region(Some(&cairo::Region::create_rectangle(&rect)));
    }

    /// Blur behind the cards where they are now, corners and all.
    fn update_blur(&self, x: f64) {
        let mut blur = self.blur.borrow_mut();
        let Some(blur) = blur.as_mut() else { return };
        let x = x.round() as i32;
        let width = CARD_WIDTH_PX.min(SURFACE_WIDTH - x);
        let on_screen = x + CARD_WIDTH_PX <= SURFACE_WIDTH;
        let tabs_h = self.slide.tabs_h.get();
        let mut rects = Vec::new();
        if tabs_h > 0 {
            // The tab bar, which does not scroll; not the card gap under it.
            rects.extend(blur::card_region((x, SHADOW_PX, width, tabs_h - GAP_PX), RADIUS_PX, on_screen));
        }
        // The cards as laid out in the column under the tabs, moved up by
        // however far it is scrolled, and cut to the part of the column on
        // screen.
        let scrolled = self.scroller.vadjustment().value().round() as i32;
        let top = SHADOW_PX + tabs_h;
        let mut y = top - scrolled;
        let mut cards = Vec::new();
        for &h in self.heights.borrow().iter() {
            cards.extend(blur::card_region((x, y, width, h), RADIUS_PX, on_screen));
            y += h + GAP_PX;
        }
        rects.extend(blur::clip_rows(&cards, top, top + self.slide.cards_h.get()));
        blur.set(&rects);
    }

    /// Put the cards at `x` and the surface at `margin` from the screen edge
    /// at once, with no slide: back to the peek when the panel gives up the
    /// keyboard. A slide already running ends here too, since every tick it
    /// has left works out `from + (to - from) * eased`, which is where this
    /// put it.
    fn jump_to(&self, x: f64, margin: i32) {
        let slide = &self.slide;
        slide.x.set(x);
        slide.from.set(x);
        slide.to.set(x);
        slide.margin_from.set(margin);
        slide.margin_to.set(margin);
        if slide.margin.replace(margin) != margin {
            self.window.set_margin(Edge::Right, margin);
        }
        if let Some(child) = self.window.child() {
            child.queue_allocate();
        }
        self.set_region(x);
        self.update_blur(x);
    }

    /// Slide the cards to `target` inside the surface, and the surface to
    /// `margin` from the screen edge, together.
    fn slide_to(self: &Rc<Self>, target: f64, margin: i32) {
        let slide = &self.slide;
        slide.from.set(slide.x.get());
        slide.to.set(target);
        slide.margin_from.set(slide.margin.get());
        slide.margin_to.set(margin);
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
            let (from, to) = (s.margin_from.get() as f64, s.margin_to.get() as f64);
            let margin = (from + (to - from) * eased).round() as i32;
            if s.margin.replace(margin) != margin {
                w.set_margin(Edge::Right, margin);
            }
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
///
/// The icon is a label of its own beside the text, so wrapped lines start
/// under the first line's text rather than back under the icon.
fn card_label(card: &Card, wrap: bool) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    if let Some(icon) = Some(card.icon()).filter(|i| !i.is_empty()) {
        // The two spaces are the gap the one-label card had after its icon.
        let icon = gtk4::Label::new(Some(&format!("{icon}  ")));
        icon.set_valign(gtk4::Align::Start);
        row.append(&icon);
    }
    let text = gtk4::Label::new(Some(&card.text));
    text.set_xalign(0.0);
    text.set_hexpand(true);
    if wrap {
        text.set_wrap(true);
        // WordChar: a long path or URL with no spaces still breaks.
        text.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    } else {
        text.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        text.set_single_line_mode(true);
    }
    row.append(&text);
    row
}

/// The one line a filter tab with nothing under it shows where its cards
/// would be (only All, when every task is waiting), in a card's look so it reads as part of the panel. Not a card:
/// it has no task and nothing to focus, and the keys pass over it.
fn empty_line(filter: Filter) -> gtk4::Label {
    let line = gtk4::Label::new(Some(filter.empty_text()));
    line.add_css_class("filter-empty");
    line.set_xalign(0.0);
    line.set_size_request(CARD_WIDTH_PX, -1);
    line
}

/// What the keyboard can stop on in a card, left to right: its body, then its
/// action buttons.
fn slots(card: &gtk4::Widget) -> Vec<gtk4::Widget> {
    let mut slots = Vec::new();
    let Some(body) = card.first_child() else { return slots };
    // The row is the card's last child, after the separator; a card without
    // one ends with its body.
    if let Some(row) = card.last_child().filter(|row| row.has_css_class("card-actions")) {
        // Its buttons, and not the hint label that follows them.
        let mut child = row.first_child();
        while let Some(button) = child {
            child = button.next_sibling();
            if button.is::<gtk4::Button>() {
                slots.push(button);
            }
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

/// The right margin that puts the surface in the middle of a monitor this
/// wide: half the room it leaves, or none on a monitor too narrow for it.
fn centre_margin(monitor_w: i32) -> i32 {
    ((monitor_w - SURFACE_WIDTH) / 2).max(0)
}

/// How wide the input region is from `x`: to the surface's edge, or, with the
/// panel centred, just the cards.
fn region_width(x: i32, centred: bool) -> i32 {
    if centred { CARD_WIDTH_PX } else { SURFACE_WIDTH - x }
}

/// How tall the column of cards is on screen: all of it, or as much as fits
/// inside the screen's margins under the `tabs_h` the filter tabs take, with
/// the rest scrolled. Only the keyboard's wrapped cards, or "+N more" opened
/// onto a long list, get that tall.
fn shown_height(cards_h: i32, tabs_h: i32, screen_h: i32) -> i32 {
    cards_h.min(screen_h - 2 * (SHADOW_PX + EDGE_GAP_PX) - tabs_h).max(0)
}

/// The scroll position that shows all of `top..bottom`, moving as little as it
/// can from `value` with `page` of the column in view. A card taller than the
/// page shows its top.
fn scroll_to_show(value: f64, page: f64, top: f64, bottom: f64) -> f64 {
    if top < value || bottom - top > page {
        top
    } else if bottom > value + page {
        bottom - page
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_region_runs_to_the_edge_unless_centred() {
        assert_eq!(region_width(100, false), SURFACE_WIDTH - 100);
        assert_eq!(region_width(CENTRED_X as i32, true), CARD_WIDTH_PX);
    }

    #[test]
    fn the_ring_margin_is_the_outlines_spread() {
        // A mismatch would clip the ring against the scroller again.
        assert!(super::super::style::OUTLINE.contains(&format!(" {RING_PX}px ")));
    }

    #[test]
    fn centred_cards_have_equal_margins_in_the_surface() {
        // The compositor centres the surface, so the cards are centred on the
        // screen only if they are centred inside it.
        let left = CENTRED_X as i32;
        let right = SURFACE_WIDTH - CARD_WIDTH_PX - left;
        assert_eq!(left, right);
    }

    #[test]
    fn the_centre_margin_leaves_equal_room_either_side() {
        let margin = centre_margin(3440);
        assert_eq!(3440 - margin - SURFACE_WIDTH, margin);
    }

    #[test]
    fn a_monitor_narrower_than_the_surface_gets_no_margin() {
        assert_eq!(centre_margin(600), 0);
    }

    #[test]
    fn centred_cards_leave_room_for_their_ring() {
        // Less than this and the surface cuts the outline ring off square.
        assert!(CENTRED_X as i32 >= RING_PX);
    }

    #[test]
    fn a_short_column_shows_whole() {
        assert_eq!(shown_height(400, 0, 1080), 400);
    }

    #[test]
    fn a_tall_column_stops_at_the_screens_margins() {
        // 1080 less the shadow room and the edge gap, top and bottom.
        assert_eq!(shown_height(5000, 0, 1080), 1080 - 2 * (SHADOW_PX + EDGE_GAP_PX));
    }

    /// The tab strip does not scroll, so the cards get the screen less it.
    #[test]
    fn the_tabs_take_their_height_off_the_cards() {
        assert_eq!(shown_height(400, 50, 1080), 400, "a short column still shows whole");
        assert_eq!(shown_height(5000, 50, 1080), 1080 - 2 * (SHADOW_PX + EDGE_GAP_PX) - 50);
    }

    /// The strip's bottom margin is the card gap less the ring the column
    /// keeps above the first card; less than none would not build.
    #[test]
    fn the_gap_under_the_tabs_leaves_room_for_the_ring() {
        assert!(GAP_PX >= RING_PX);
    }

    #[test]
    fn a_card_in_view_does_not_scroll() {
        assert_eq!(scroll_to_show(100.0, 500.0, 150.0, 300.0), 100.0);
    }

    #[test]
    fn a_card_below_scrolls_up_until_its_bottom_shows() {
        assert_eq!(scroll_to_show(0.0, 500.0, 450.0, 600.0), 100.0);
    }

    #[test]
    fn a_card_above_scrolls_down_to_its_top() {
        assert_eq!(scroll_to_show(300.0, 500.0, 100.0, 250.0), 100.0);
    }

    /// Taller than the screen: its top, where the icon and the start of the
    /// description are.
    #[test]
    fn a_card_taller_than_the_page_shows_its_top() {
        assert_eq!(scroll_to_show(0.0, 500.0, 200.0, 900.0), 200.0);
    }
}
