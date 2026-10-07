//! One monitor's task panel: a layer surface on the right edge, tucked away to
//! a peek until the pointer comes over it, or in the middle of the screen
//! while it has the keyboard.
//!
//! What it shows, and what a key or a press does to that, is `state.rs`'s
//! [`PanelState`]: this file draws what the state says and runs the
//! [`Effect`]s it hands back. A key, a click and Enter on a focused button all
//! go through the state, so they take one path.
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
//! and every card lays itself out again: its whole description, wrapped, and,
//! on the focused card alone, a row of buttons for the menu's most-used
//! actions under it, so the list stays short enough to scan. A card is a box
//! holding a body button and that row; every card has its row, shown and
//! hidden as the focus moves rather than built again, since a render would
//! lose the focused button. Up and Down move between cards, Down onto "+N
//! more" showing the cards it stands for and landing on the first of them;
//! Left, Right and Tab move along the focused one, and g, b, s, r, e, t and
//! Delete press its Go to session, Back to list, Start, Refine, Edit, Stop and
//! Remove. The controller runs in the capture phase, ahead of GTK's own focus
//! chain, which would otherwise walk every button on the panel. The focused
//! card is darkened. The body opens the whole menu. Escape, or anything that
//! runs, hands the keyboard back, folds the cards to one line again, and puts
//! the panel back on the right edge as a peek.
//!
//! GTK's focus and the state's are kept the same both ways: an
//! [`Effect::Focus`] moves GTK's, and GTK's moving (a click on a button)
//! tells the state, which is how moving off an armed Remove disarms it. A
//! render tears the column down, and the focus moves GTK makes meanwhile are
//! not the user's, so they are not passed on.
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
//! On the Waiting tab alone, Clear all ends the tab bar. Like Remove, its
//! first press arms it as Confirm clear all, and its second deletes: every
//! task the tab lists, each through Remove's own `task status <uuid> deleted
//! --yes`, one after another. The panel goes to All at once and keeps the
//! keyboard. Moving the focus, or any re-render, a tab switch included,
//! disarms it. It is out of the focus chain with the tabs, so Ctrl+Delete
//! presses it; Delete alone is still the focused card's Remove. Armed, it
//! takes the focus off the cards and every key with it: Enter confirms rather
//! than opening the card it was on, Escape and the keys that move put it back
//! with the focus where it was, and a card's keys do nothing.
//!
//! After the filter tabs, Ideas is always shown: not a filter but the
//! workspace's notepad, `notepad.rs`'s text area in the column in place of the
//! cards, one per workspace tag. 6 picks it, and ] from the last filter tab.
//! While it is picked the text area takes every key but Escape, Ctrl+[ and
//! Ctrl+] (`keys::ideas_key_action`), and a tick's render leaves it in place
//! so typing keeps its focus. Escape saves and goes back to the task list, on
//! the tab picked before Ideas, keeping the keyboard. What was typed is saved
//! a second after typing stops and again on leaving Ideas or giving the
//! keyboard back, to the tag it was typed for.
//!
//! Waiting tasks are on the Waiting tab and nowhere else: not on All, not on
//! the hover or the peek, so a workspace whose tasks are all waiting shows
//! nothing on its edge. With the keyboard it opens on All, which then says it
//! has no tasks, beside the Waiting tab.
//!
//! Wrapped, the cards can stand taller than the screen, so the column sits in
//! a scroller under the tabs, capped at the screen's height less its margins
//! and the tabs. Moving the focus measures the cards again, its row having
//! moved, and scrolls the focused card wholly into view, and the blur region
//! moves with the scroll and stops at the view's edges.
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

use super::actions;
use crate::actions::Action;
use super::blur::{self, Blur};
use super::keys;
use super::notepad::Notepad;
use super::model::{Card, Filter, Status, Tab};
use super::state::{Armed, Effect, Focus, PanelState, Shown, Slot};
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
/// How often Speak's spinner turns a frame while a speech is being got
/// ready: the usual pace of a braille spinner in a terminal.
const SPIN: Duration = Duration::from_millis(80);

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
    /// The Ideas tab, after the filter tabs and always shown with them.
    ideas_button: gtk4::Button,
    /// Clear all, at the tab bar's far end, shown only on the Waiting tab:
    /// deletes every task the tab lists, on its second press.
    clear: gtk4::Button,
    /// For its height, which caps the column's: read at each render, since a
    /// scale change alters it.
    monitor: gdk::Monitor,
    slide: Rc<Slide>,
    /// What the panel shows, and what keys and presses do to it.
    state: RefCell<PanelState>,
    /// The cards on screen, as the last render drew them.
    cards: RefCell<Vec<CardWidgets>>,
    /// The panel is drawing the state: tearing the column down, or hiding
    /// the rows the focus is leaving. GTK's focus moves meanwhile are its own
    /// doing, not the user's, and the state is not told of them.
    drawing: Cell<bool>,
    /// Speak's spinner is turning: a timer runs while the panel has the
    /// keyboard, the only time the buttons show.
    spinning: Cell<bool>,
    /// The spinner frame it is on.
    frame: Cell<usize>,
    /// Each card's height, top to bottom, for the blur region.
    heights: RefCell<Vec<i32>>,
    blur: RefCell<Option<Blur>>,
    /// The Ideas tab's text area, shown in the column in place of the cards.
    notepad: Rc<Notepad>,
    /// The workspace tag this panel shows, whose notepad Ideas opens.
    tag: RefCell<String>,
}

/// One card on screen: the widgets a focus, a spinner frame or an arming is
/// drawn on, found by task rather than by widget name.
struct CardWidgets {
    /// None on "+N more".
    uuid: Option<String>,
    root: gtk4::Box,
    body: gtk4::Button,
    /// None on "+N more", and on every card off the keyboard.
    row: Option<ActionRow>,
}

/// A card's action row, and the separator over it: shown while the card
/// has the focus.
struct ActionRow {
    separator: gtk4::Separator,
    row: gtk4::Box,
    /// The focused button's name in words, after the icons.
    hint: gtk4::Label,
    buttons: Vec<(Action, gtk4::Button)>,
    /// The task is up next, so its Up next reads Not up next.
    up_next: bool,
}

impl CardWidgets {
    fn button(&self, action: Action) -> Option<&gtk4::Button> {
        self.row.as_ref()?.buttons.iter().find(|(a, _)| *a == action).map(|(_, b)| b)
    }
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

impl Slide {
    fn tucked() -> Slide {
        Slide {
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
        }
    }
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
        let scroller = scroller(&column);
        let (tabs, tab_buttons, ideas_button, clear) = tab_bar();
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
            ideas_button,
            clear,
            slide: Rc::new(Slide::tucked()),
            state: RefCell::new(PanelState::default()),
            cards: RefCell::new(Vec::new()),
            drawing: Cell::new(false),
            spinning: Cell::new(false),
            frame: Cell::new(0),
            heights: RefCell::new(Vec::new()),
            blur: RefCell::new(None),
            notepad: Notepad::new(),
            tag: RefCell::new(String::new()),
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
        panel.connect_tab_bar();
        panel.connect_hover();
        panel.connect_keys();
        panel.connect_focus();
        panel.connect_surface();

        // Map once, then hide in the same main-loop iteration. A layer surface
        // that has never been mapped ignores a later present(), so a daemon
        // started on an empty workspace would otherwise never show a panel.
        panel.window.present();
        panel.window.set_visible(false);

        panel
    }

    /// A click on a tab picks it; a click on Clear all presses it.
    fn connect_tab_bar(self: &Rc<Self>) {
        for (button, filter) in self.tab_buttons.iter().zip(Filter::TABS) {
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    let effects = p.state.borrow_mut().on_tab(Tab::Filter(filter));
                    p.apply(effects);
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.ideas_button.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                let effects = p.state.borrow_mut().on_tab(Tab::Ideas);
                p.apply(effects);
            }
        });
        let weak = Rc::downgrade(self);
        self.clear.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                let effects = p.state.borrow_mut().on_clear_all();
                p.apply(effects);
            }
        });
    }

    /// The pointer coming over the peek slides the cards out, and leaving
    /// slides them back after the grace. A click on a card lands on this
    /// surface, because it is inside the input region, and goes no further
    /// than the card's own handler — see card_widget.
    fn connect_hover(self: &Rc<Self>) {
        let motion = gtk4::EventControllerMotion::new();
        {
            let weak = Rc::downgrade(self);
            motion.connect_enter(move |_, _, _| {
                let Some(p) = weak.upgrade() else { return };
                // The keyboard's panel is in the middle and stays put.
                if p.state.borrow().keyboard() {
                    return;
                }
                p.cancel_grace();
                p.slide_to(EXPANDED_X, 0);
            });
        }
        {
            let weak = Rc::downgrade(self);
            motion.connect_leave(move |_| {
                let Some(p) = weak.upgrade() else { return };
                // The keyboard's panel stays out until the keyboard is done.
                if p.state.borrow().keyboard() {
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
        self.window.add_controller(motion);
    }

    /// Every key goes to the state, which says whether it was the panel's.
    /// Capture, so the arrows and Tab reach this before the window's own
    /// focus chain, which would walk every button on the panel in turn rather
    /// than between cards, or along one.
    fn connect_keys(self: &Rc<Self>) {
        let controller = gtk4::EventControllerKey::new();
        controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        controller.connect_key_pressed(move |_, key, _, modifiers| {
            // Alt and Super chords belong to the compositor and the focused
            // widget, not to the letters. Ctrl chords do too, bar Ctrl+Enter
            // and Ctrl+Delete, which key_action picks out: Ctrl+T must not
            // Stop.
            if modifiers.intersects(gdk::ModifierType::ALT_MASK | gdk::ModifierType::SUPER_MASK) {
                return glib::Propagation::Proceed;
            }
            let Some(p) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
            // On Ideas the text area takes every key as typing, bar Escape
            // and the Ctrl+[ and Ctrl+] that switch tab.
            let action = if p.state.borrow().on_ideas() {
                keys::ideas_key_action(key, ctrl)
            } else {
                keys::key_action(key, ctrl)
            };
            let effects = p.state.borrow_mut().on_key(action);
            match effects {
                Some(effects) => {
                    p.apply(effects);
                    glib::Propagation::Stop
                }
                None => glib::Propagation::Proceed,
            }
        });
        self.window.add_controller(controller);
    }

    /// GTK's focus moving, by a click or by an `Effect::Focus` coming back,
    /// goes to the state, which disarms what the move should disarm. Then
    /// the action row moves to the card the focus is on, before follow_focus
    /// scrolls that card, row and all, into view.
    fn connect_focus(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.window.connect_notify_local(Some("focus-widget"), move |_, _| {
            let Some(p) = weak.upgrade() else { return };
            if p.drawing.get() {
                return;
            }
            let focus = GtkWindowExt::focus(&p.window).and_then(|w| p.focus_of(&w));
            p.state.borrow_mut().on_focus(focus);
            p.sync();
            p.follow_focus();
        });
    }

    /// The blur region is in surface coordinates, so it moves with the
    /// scroll. GTK may reset the input region when the surface maps or
    /// resizes, and a surface shown again may be a new wl_surface, which
    /// needs its own blur object — the old one goes first, since niri allows
    /// one each.
    fn connect_surface(self: &Rc<Self>) {
        {
            let weak = Rc::downgrade(self);
            self.scroller.vadjustment().connect_value_changed(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.update_blur(p.slide.x.get());
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.window.connect_map(move |_| {
            if let Some(p) = weak.upgrade() {
                p.set_region(p.slide.x.get());
                drop(p.blur.borrow_mut().take());
                *p.blur.borrow_mut() = Blur::new(&p.window);
                p.update_blur(p.slide.x.get());
            }
        });
    }

    /// Show the cards of the workspace this tag is for, capped, or hide the
    /// panel when there are none. A new tag moves the notepad: what was typed
    /// is saved to the tag it was typed for, and, with the keyboard still
    /// held, the new tag's ideas load at once; without it they load when the
    /// panel next takes it.
    pub fn show(self: &Rc<Self>, tag: &str, cards: &[Card]) {
        if self.tag.borrow().as_str() != tag {
            *self.tag.borrow_mut() = tag.to_string();
            if self.state.borrow().keyboard() {
                self.notepad.load(tag);
            } else {
                self.notepad.flush();
            }
        }
        let effects = self.state.borrow_mut().set_cards(cards);
        self.apply(effects);
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
        if !self.state.borrow_mut().take_keyboard(agents) {
            return false;
        }
        self.cancel_grace();
        // Afresh every time: another panel may have saved this tag's ideas
        // since, its workspace having moved monitor.
        self.notepad.load(&self.tag.borrow());
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.apply(vec![Effect::Render]);
        // After the render, which measures the cards the region needs.
        self.slide_to(CENTRED_X, centre_margin(self.monitor.geometry().width()));
        self.start_spinner();
        true
    }

    /// Run what the state asked for, in order, then draw what it says that a
    /// render does not.
    fn apply(self: &Rc<Self>, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Render => self.render(),
                Effect::Focus(focus) => self.focus_on(focus.as_ref()),
                Effect::Release => {
                    // What was typed on Ideas, saved as the keyboard goes.
                    self.notepad.flush();
                    // Snapped back rather than slid, so whatever the keyboard
                    // opened does not wait on the cards crossing half the
                    // screen.
                    self.window.set_keyboard_mode(KeyboardMode::None);
                    self.jump_to(TUCKED_X, 0);
                }
                Effect::Spawn(args) => open_menu(&self.output, &args),
                Effect::DeleteAll(uuids) => delete_all(&self.output, &uuids),
                Effect::Notify(text) => crate::notify::tasks(&text),
                Effect::SaveIdeas => self.notepad.flush(),
            }
        }
        self.sync();
    }

    /// Turn the spinner on the Speak button of the task whose speech is being
    /// got ready, for as long as the panel has the keyboard. The background
    /// speech says in its pid file when its audio starts, and the speaker
    /// comes back then. A timer rather than a watch on that file: it has to
    /// tick for the animation anyway, and it stops with the keyboard.
    fn start_spinner(self: &Rc<Self>) {
        if self.spinning.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(SPIN, move || {
            let Some(p) = weak.upgrade() else { return glib::ControlFlow::Break };
            if !p.state.borrow().keyboard() {
                p.spinning.set(false);
                return glib::ControlFlow::Break;
            }
            p.spin();
            glib::ControlFlow::Continue
        });
    }

    /// One turn: the next frame on the preparing task's Speak button, the
    /// speaker on every other. A re-render's new buttons start on the speaker
    /// and are caught up here.
    fn spin(&self) {
        let preparing = crate::speak::preparing();
        let frame = self.frame.get().wrapping_add(1);
        self.frame.set(frame);
        for card in self.cards.borrow().iter() {
            let Some(button) = card.button(Action::Speak) else { continue };
            let label = if preparing.is_some() && preparing == card.uuid {
                Action::SPINNER[frame % Action::SPINNER.len()]
            } else {
                Action::Speak.icon()
            };
            set_label(button, label);
        }
    }

    /// Slide back, folding an expanded list up again.
    fn tuck(self: &Rc<Self>) {
        self.slide_to(TUCKED_X, 0);
        let effects = self.state.borrow_mut().on_tuck();
        self.apply(effects);
    }

    /// Which filter tabs show, which tab is picked, and whether Clear all
    /// shows. Ideas is always shown: the bar itself hides without the
    /// keyboard.
    fn update_tabs(&self) {
        let state = self.state.borrow();
        let shown = state.tabs();
        for (button, filter) in self.tab_buttons.iter().zip(Filter::TABS) {
            button.set_visible(shown.contains(&filter));
            set_class(button, "current", state.tab() == Tab::Filter(filter));
        }
        set_class(&self.ideas_button, "current", state.on_ideas());
        self.clear.set_visible(state.shows_clear_all());
    }

    /// Measure the cards and the tabs, and size the surface to them: the
    /// heights the blur region and the input region work from. Run by every
    /// render, and again whenever a card's action row shows or hides, which
    /// changes its height without a render.
    fn fit(&self) {
        let keyboard = self.state.borrow().keyboard();
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

    /// Draw the state's cards afresh, or hide the panel when it has nothing
    /// to show, and put the focus where the state has it.
    fn render(self: &Rc<Self>) {
        if self.state.borrow().hidden() {
            // Only hide something that is up; hiding a never-mapped layer
            // surface leaves it deaf to a later present().
            if self.window.is_visible() {
                self.window.set_visible(false);
            }
            self.cancel_grace();
            self.jump_to(TUCKED_X, 0);
            return;
        }
        let (shown, keyboard, empty, focus, ideas) = {
            let state = self.state.borrow();
            (state.visible(), state.keyboard(), state.empty_text(), state.focus().cloned(), state.on_ideas())
        };

        self.while_drawing(|| {
            // Mid-typing, a tick's render leaves the notepad where it is:
            // taking it out of the column would take its focus with it.
            let column: &gtk4::Widget = self.column.upcast_ref();
            if ideas && self.notepad.root.parent().as_ref() == Some(column) {
                return;
            }
            while let Some(child) = self.column.first_child() {
                self.column.remove(&child);
            }
            let cards: Vec<CardWidgets> = shown.iter().map(|s| self.card_widget(s, keyboard)).collect();
            for card in &cards {
                self.column.append(&card.root);
            }
            *self.cards.borrow_mut() = cards;
            if ideas {
                self.column.append(&self.notepad.root);
            }
            if let Some(text) = empty {
                // Only All, with every task waiting: it says so, and the
                // Waiting tab beside it has them.
                self.column.append(&empty_line(text));
            }
        });
        self.tabs.set_visible(keyboard);
        self.update_tabs();
        self.fit();

        // present(), not set_visible(true): see new().
        self.window.present();
        self.set_region(self.slide.x.get().min(self.slide.to.get()));
        self.update_blur(self.slide.x.get());

        if ideas {
            self.notepad.focus();
        } else if keyboard {
            self.focus_on(focus.as_ref());
        }
    }

    /// One card: a box holding the body, a button so the keyboard can focus
    /// and press it, and, while the panel has the keyboard, its action row
    /// along the bottom, hidden until the card has focus. The body opens the
    /// task's whole menu, or shows the rest in place of "+N more".
    fn card_widget(self: &Rc<Self>, shown: &Shown, keyboard: bool) -> CardWidgets {
        let card = &shown.card;
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        root.add_css_class("task-card");
        match card.status {
            Status::Active => root.add_css_class("active"),
            Status::Blocked => root.add_css_class("blocked"),
            Status::Planned => root.add_css_class("planned"),
            Status::More => root.add_css_class("more"),
            Status::Waiting => root.add_css_class("waiting"),
            Status::Pending => {}
        }
        // Its own class, beside the status's: an up next card is yellow
        // whatever its icon, unless it is active or waiting.
        if card.shows_up_next() {
            root.add_css_class("up-next");
        }
        root.set_size_request(CARD_WIDTH_PX, -1);
        // Clips the action row to the card's rounded bottom corners.
        root.set_overflow(gtk4::Overflow::Hidden);

        let body = gtk4::Button::builder().child(&card_label(card, keyboard)).build();
        body.add_css_class("card-body");
        // A mouse click opens the menu without leaving the card darkened.
        body.set_focus_on_click(false);
        {
            let weak = Rc::downgrade(self);
            let uuid = card.uuid.clone();
            body.connect_clicked(move |_| {
                let Some(p) = weak.upgrade() else { return };
                let effects = match &uuid {
                    Some(uuid) => p.state.borrow_mut().on_press(uuid, Slot::Body),
                    None => p.state.borrow_mut().on_more(),
                };
                p.apply(effects);
            });
        }
        root.append(&body);

        let row = card.uuid.as_ref().filter(|_| !shown.actions.is_empty()).map(|uuid| {
            let row = self.action_row(uuid, card.up_next, &shown.actions);
            root.append(&row.separator);
            root.append(&row.row);
            row
        });
        CardWidgets { uuid: card.uuid.clone(), root, body, row }
    }

    /// A card's buttons, left-aligned and only as wide as their icons, then
    /// the focused one's name in words: the icons alone do not say what they
    /// do. Hidden, with the line over them, until the card has focus.
    fn action_row(self: &Rc<Self>, uuid: &str, up_next: bool, actions: &[Action]) -> ActionRow {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        row.add_css_class("card-actions");
        row.set_halign(gtk4::Align::Start);
        let hint = gtk4::Label::new(None);
        hint.add_css_class("card-hint");
        let mut buttons = Vec::new();
        for &action in actions {
            let button = gtk4::Button::with_label(action.icon());
            // The icon's name in words, under the pointer and for a screen
            // reader. Up next reads Not up next on a task already up next.
            button.set_tooltip_text(Some(action.label(up_next)));
            button.add_css_class(action.class());
            let weak = Rc::downgrade(self);
            let uuid = uuid.to_string();
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    let effects = p.state.borrow_mut().on_press(&uuid, Slot::Button(action));
                    p.apply(effects);
                }
            });
            row.append(&button);
            buttons.push((action, button));
        }
        row.append(&hint);
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        separator.add_css_class("card-separator");
        separator.set_visible(false);
        row.set_visible(false);
        ActionRow { separator, row, hint, buttons, up_next }
    }

    /// The state's focus as GTK's, its card's row shown first: a hidden
    /// button is no place for the focus. Hiding the row it leaves moves
    /// GTK's focus off it on the way, which the state has no need to hear.
    /// None takes the focus off the cards.
    fn focus_on(&self, focus: Option<&Focus>) {
        self.while_drawing(|| self.show_row(focus.map(|f| f.uuid.as_str())));
        match focus.and_then(|f| self.widget_of(f)) {
            Some(widget) => {
                widget.grab_focus();
            }
            None => GtkWindowExt::set_focus(&self.window, None::<&gtk4::Widget>),
        }
    }

    /// Run `draw` with GTK's focus moves kept from the state.
    fn while_drawing(&self, draw: impl FnOnce()) {
        let was = self.drawing.replace(true);
        draw();
        self.drawing.set(was);
    }

    /// The widget a focus is on.
    fn widget_of(&self, focus: &Focus) -> Option<gtk4::Widget> {
        let cards = self.cards.borrow();
        let card = cards.iter().find(|c| c.uuid.as_deref() == Some(focus.uuid.as_str()))?;
        match focus.slot {
            Slot::Body => Some(card.body.clone().upcast()),
            Slot::Button(action) => card.button(action).map(|b| b.clone().upcast()),
        }
    }

    /// The focus a widget is: a task card's body or one of its buttons.
    fn focus_of(&self, widget: &gtk4::Widget) -> Option<Focus> {
        self.cards.borrow().iter().find_map(|card| {
            let uuid = card.uuid.clone()?;
            if card.body.upcast_ref::<gtk4::Widget>() == widget {
                return Some(Focus { uuid, slot: Slot::Body });
            }
            let (action, _) = card.row.as_ref()?.buttons.iter().find(|(_, b)| b.upcast_ref::<gtk4::Widget>() == widget)?;
            Some(Focus { uuid, slot: Slot::Button(*action) })
        })
    }

    /// Draw what the state says that a render does not: which Remove, and
    /// whether Clear all, reads as armed, and which card shows its action row
    /// and the focused button's name.
    fn sync(&self) {
        let (focus, armed) = {
            let state = self.state.borrow();
            (state.focus().cloned(), state.armed().clone())
        };
        let clear_armed = matches!(armed, Armed::ClearAll { .. });
        set_label(&self.clear, &clear_label(clear_armed));
        set_class(&self.clear, "confirm", clear_armed);
        for card in self.cards.borrow().iter() {
            let Some(row) = &card.row else { continue };
            let removing = matches!(&armed, Armed::Remove(uuid) if Some(uuid) == card.uuid.as_ref());
            if let Some(remove) = card.button(Action::Remove) {
                set_label(remove, &remove_label(removing));
                set_class(remove, "confirm", removing);
            }
            let hint = match &focus {
                Some(Focus { uuid, slot: Slot::Button(action) }) if Some(uuid) == card.uuid.as_ref() => {
                    action.label(row.up_next)
                }
                _ => "",
            };
            row.hint.set_label(hint);
        }
        self.show_row(focus.as_ref().map(|f| f.uuid.as_str()));
    }

    /// Scroll the focused card wholly into view, body and buttons, once the
    /// frame after the move has laid it out. A card just rendered has no place
    /// in the column until then.
    fn follow_focus(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        crate::taskbox::after_next_paint(&self.window, move || {
            let Some(p) = weak.upgrade() else { return };
            let Some(uuid) = p.state.borrow().focus().map(|f| f.uuid.clone()) else { return };
            let root = p.cards.borrow().iter().find(|c| c.uuid.as_ref() == Some(&uuid)).map(|c| c.root.clone());
            let Some(bounds) = root.and_then(|r| r.compute_bounds(&p.column)) else { return };
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

    /// Show this task's card's action row and hide every other card's, so
    /// only the card being worked on stands taller by its buttons, and the
    /// list stays short enough to scan. The cards change height without a
    /// render, so the surface, the input region and the blur are fitted to
    /// them again, but only when a row actually showed or hid.
    fn show_row(&self, uuid: Option<&str>) {
        let mut changed = false;
        for card in self.cards.borrow().iter() {
            let Some(row) = &card.row else { continue };
            let show = uuid.is_some() && card.uuid.as_deref() == uuid;
            if row.row.is_visible() != show {
                row.separator.set_visible(show);
                row.row.set_visible(show);
                changed = true;
            }
        }
        if changed {
            self.fit();
            self.set_region(self.slide.x.get().min(self.slide.to.get()));
            self.update_blur(self.slide.x.get());
        }
    }

    pub fn close(&self) {
        self.notepad.flush();
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
        let width = region_width(x, self.state.borrow().keyboard());
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
/// The column's scroller, for cards that run taller than the screen. A
/// viewport made by hand, to turn off its own scroll-to-focus. That would
/// show only the focused button, leaving the rest of its card off screen;
/// follow_focus scrolls the whole card into view instead.
fn scroller(column: &gtk4::Box) -> gtk4::ScrolledWindow {
    let viewport = gtk4::Viewport::new(None::<&gtk4::Adjustment>, None::<&gtk4::Adjustment>);
    viewport.set_scroll_to_focus(false);
    viewport.set_child(Some(column));
    let scroller = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        // External still scrolls (follow_focus, the wheel) but draws no
        // bar over the Remove button or out of the 760px column's width.
        .vscrollbar_policy(gtk4::PolicyType::External)
        .child(&viewport)
        .build();
    scroller.set_vexpand(true);
    scroller
}

/// The filter tabs, Ideas and Clear all, over the scroller rather than in it,
/// so they stay put while the cards scroll. Ring room on three sides, as the
/// column keeps, and under them the card gap less the ring the column keeps
/// above the first card: the first card then sits a card gap below.
fn tab_bar() -> (gtk4::Box, Vec<gtk4::Button>, gtk4::Button, gtk4::Button) {
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
            // cards and their buttons, and a click does not take the focus.
            button.set_focusable(false);
            button.set_focus_on_click(false);
            tabs.append(&button);
            button
        })
        .collect();
    // Ideas, last of the tabs: not a filter, and always shown.
    let ideas = gtk4::Button::with_label(Tab::Ideas.label());
    ideas.set_focusable(false);
    ideas.set_focus_on_click(false);
    tabs.append(&ideas);
    // Clear all, at the bar's far end: hexpand takes the room the tabs
    // leave, and End keeps the button its own width at the end of it. Out
    // of the focus chain like the tabs, which is why Ctrl+Delete presses it.
    let clear = gtk4::Button::with_label(&clear_label(false));
    clear.add_css_class("clear-all");
    clear.set_tooltip_text(Some("Ctrl+Delete"));
    clear.set_hexpand(true);
    clear.set_halign(gtk4::Align::End);
    clear.set_focusable(false);
    clear.set_focus_on_click(false);
    clear.set_visible(false);
    tabs.append(&clear);
    (tabs, tab_buttons, ideas, clear)
}

/// The one line a filter tab with nothing under it shows where its cards
/// would be (only All, when every task is waiting), in a card's look so it
/// reads as part of the panel. Not a card: it has no task and nothing to
/// focus.
fn empty_line(text: &str) -> gtk4::Label {
    let line = gtk4::Label::new(Some(text));
    line.add_css_class("filter-empty");
    line.set_xalign(0.0);
    line.set_size_request(CARD_WIDTH_PX, -1);
    line
}

/// Clear all's face: Remove's trash can and its words, or, armed, what it asks.
fn clear_label(armed: bool) -> String {
    let words = if armed { actions::CONFIRM_CLEAR_ALL } else { actions::CLEAR_ALL };
    format!("{}  {words}", Action::Remove.icon())
}

/// Remove's face: its trash can alone, or, armed, the can and what it asks.
fn remove_label(armed: bool) -> String {
    if armed {
        format!("{}  {}", Action::Remove.icon(), Action::CONFIRM_REMOVE)
    } else {
        Action::Remove.icon().to_string()
    }
}

/// Set a button's label only when it differs: every sync sets them all, and
/// an unchanged label should not cost a relayout.
fn set_label(button: &gtk4::Button, label: &str) {
    if button.label().as_deref() != Some(label) {
        button.set_label(label);
    }
}

fn set_class(widget: &impl IsA<gtk4::Widget>, class: &str, on: bool) {
    if on {
        widget.add_css_class(class);
    } else {
        widget.remove_css_class(class);
    }
}

/// Run a `niritasks` command on this monitor: a task's action menu, the
/// fuzzel list, or one of a card's buttons. See `spawn_on`.
pub fn open_menu(output: &str, args: &[String]) {
    spawn_on(output, |exe| {
        let mut command = vec![exe.to_string()];
        command.extend_from_slice(args);
        command
    });
}

/// Clear all's deletes, one after another in one process: see
/// `actions::clear_all_command`.
fn delete_all(output: &str, uuids: &[String]) {
    spawn_on(output, |exe| actions::clear_all_command(exe, uuids));
}

/// Have niri spawn the command `build` makes from this program's own path.
///
/// This monitor is focused first. The menu then files under the workspace the
/// panel shows, rather than whichever monitor had focus, and fuzzel opens on
/// the screen that was clicked. It runs as its own process, spawned by niri:
/// fuzzel blocks until it closes, and the daemon must not.
fn spawn_on(output: &str, build: impl FnOnce(&str) -> Vec<String>) {
    let result = std::env::current_exe()
        .map_err(anyhow::Error::from)
        .and_then(|exe| {
            crate::niri::focus_monitor(output)?;
            crate::niri::spawn(build(&exe.to_string_lossy()))
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
        const _: () = assert!(GAP_PX >= RING_PX);
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

    /// Remove's trash can and its words, then, armed, what it asks.
    #[test]
    fn clear_all_wears_removes_trash_can_and_asks_once_armed() {
        assert_eq!(clear_label(false), format!("{}  Clear all", Action::Remove.icon()));
        assert_eq!(clear_label(true), format!("{}  Confirm clear all", Action::Remove.icon()));
    }

    /// Remove's trash can alone, then, armed, the can and what it asks.
    #[test]
    fn remove_asks_once_armed() {
        assert_eq!(remove_label(false), Action::Remove.icon());
        assert_eq!(remove_label(true), format!("{}  Confirm remove", Action::Remove.icon()));
    }
}
