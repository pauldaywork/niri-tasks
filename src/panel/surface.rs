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
//! on the focused card alone, a row of buttons for the task's actions
//! under it, so the list stays short enough to scan. A card is a box
//! holding a body button and that row; every card has its row, shown and
//! hidden as the focus moves rather than built again, since a render would
//! lose the focused button. Up and Down move between cards, Down onto "+N
//! more" showing the cards it stands for and landing on the first of them;
//! Left, Right and Tab move along the focused one, and g, b, s, r, i, e, c,
//! m, t and Delete press its Go to session, Back to list, Start, Refine,
//! Grill me, Edit, Complete, Move to workspace, Stop and Remove. Ctrl+Delete deletes the focused card's task at once and keeps the
//! keyboard. After the buttons a dimmed hint names the keys that change per
//! card, the focused button's and what Ctrl+Enter does to the task
//! (`Action::hint`); a footer under the scroller names Enter and Ctrl+Delete,
//! which act alike on every card. The controller runs in the capture phase,
//! ahead of GTK's own focus chain, which would otherwise walk every button on
//! the panel. The focused card is darkened. Enter, Space or a click on its
//! body shows the task's notes, or hides them.
//! Escape, or anything that runs, hands the keyboard back, folds the cards to
//! one line again, and puts the panel back on the right edge as a peek.
//!
//! Move to workspace (m) swaps the cards for the project list: the
//! `~/Projects` folders the task can move to, drawn as cards under a line
//! naming it, with a footer of its own. A text field takes the tab bar's
//! place and the keyboard: what is typed narrows the folders to fzf's
//! matches, best first (`project::fzf_matches`), or to those holding the
//! text when fzf cannot run. The highlighted folder is darkened by a class,
//! the field keeping the focus; each keystroke puts it on the top match. Up
//! and Down move it, Enter or a click runs `task move` and goes back to the
//! cards on the next one, and Escape clears the text, then goes back to the
//! cards on the same one.
//!
//! Mod+Alt+W shows the same list for opening a project (`state::Purpose::Open`):
//! every `~/Projects` folder, then the GitHub repos not cloned yet, under
//! "Open a project", with or without cards, on a named workspace or not.
//! Enter or a click gives the keyboard back and runs `project open` with the
//! row; text matching no row makes a folder of it, and a line says so
//! first. Escape clears the text, then gives the keyboard back.
//!
//! GTK's focus and the state's are kept the same both ways: an
//! [`Effect::Focus`] moves GTK's, and GTK's moving (a click on a button)
//! tells the state, which is how moving off an armed Remove disarms it. A
//! render tears the column down, and the focus moves GTK makes meanwhile are
//! not the user's, so they are not passed on.
//!
//! Above the cards, a bar of filter tabs, All, Active, Planned, To refine,
//! Waiting and Finished, narrows them to the tasks it names. A tab shows only
//! while it has a task under it, All apart. 1 to 6 pick one, always the same
//! one, and each wears its key in brackets after its name, as To refine (4);
//! they do nothing for a hidden tab; [ and ] step along the shown ones,
//! stopping at the ends; a click picks one too. The tabs never take focus, so
//! the arrows and Tab still move only between cards and buttons. The panel
//! takes the keyboard on All every time; a refresh keeps the tab, falling back
//! to All once it has nothing left, and focus falls back to the first card
//! when the one it was on has left it.
//!
//! On the Waiting tab alone, Clear all sits under the tab bar, at the right on
//! a strip of its own, the numbered tabs leaving no room on the bar. Like
//! Remove, its first press arms it as Confirm clear all, and its second
//! deletes: every task the tab lists, each through Remove's own `task status
//! <uuid> deleted --yes`, one after another. The panel goes to All at once and
//! keeps the keyboard. Moving the focus, or any re-render, a tab switch
//! included, disarms it. It is out of the focus chain with the tabs, so
//! Ctrl+Shift+Delete presses it; Ctrl+Delete and Delete are still the focused
//! card's. Armed, it takes the focus off the cards and every key with it: Enter
//! confirms rather than pressing a button on the card it was on, Escape and the
//! keys that move put it back with the focus where it was, and a card's keys do
//! nothing.
//!
//! After the filter tabs, Ideas is always shown: not a filter but the
//! workspace's notepad, `notepad.rs`'s text area in the column in place of the
//! cards, one per workspace tag. 7 picks it, and ] from the last filter tab.
//! While it is picked the text area takes every key but Escape, Ctrl+[ and
//! Ctrl+] (the state maps keys by its mode), and a tick's render leaves it in place
//! so typing keeps its focus. Escape saves and goes back to the task list, on
//! the tab picked before Ideas, keeping the keyboard. What was typed is saved
//! a second after typing stops and again on leaving Ideas or giving the
//! keyboard back, to the tag it was typed for.
//!
//! Waiting tasks are on the Waiting tab and nowhere else, and the last
//! twelve finished ones on the Finished tab and nowhere else: not on All,
//! not on the hover or the peek, so a workspace whose tasks are all waiting
//! or finished shows nothing on its edge. With the keyboard it opens on
//! All, which then says it has no tasks, beside those tabs. A finished
//! card's Back to list reopens its task.
//!
//! Wrapped, the cards can stand taller than the screen, so the column sits in
//! a scroller under the tabs, capped at the screen's height less its margins,
//! the tabs and the footer. Moving the focus measures the cards again, its row
//! having moved, and scrolls the focused card wholly into view, and the blur
//! region moves with the scroll and stops at the view's edges.
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
//! It is held with exclusive keyboard mode throughout. niri gives an on-demand
//! layer surface focus only after a click on it, so switching to on-demand
//! once focused would drop the focus at once.

use super::actions;
use crate::actions::{Action, TaskState};
use super::blur::{self, Blur};
use super::keys;
use super::notepad::Notepad;
use super::model::{Card, Filter, Status, Tab};
use super::projects::ProjectList;
use super::state::{Armed, Effect, Focus, Mode, PanelState, Shown, Slot};
use super::style::{CARD_WIDTH_PX, GAP_PX, RADIUS_PX};
use crate::project::{Projects, Row};
use gtk4::prelude::*;
use gtk4::{cairo, gdk, glib, Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Layer-shell namespace, so `layer-rule { match namespace="niri-tasks-panel" }`
/// can target the panel.
const NAMESPACE: &str = "niri-tasks-panel";

/// How much of each card shows while the panel is tucked away: the card's
/// 12px padding plus about one glyph, so the status icon peeks out and the
/// text stays hidden.
const PEEK_PX: i32 = 30;

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
    /// The filter tabs above the scroller, and under them on the Waiting tab
    /// Clear all's strip, shown only while the panel has the keyboard. It
    /// does not scroll with the cards.
    tabs: gtk4::Box,
    /// The tabs' own strip inside `tabs`, for the blur behind it.
    bar: gtk4::Box,
    /// One button per filter tab, in `Filter::TABS` order, for which ones show
    /// and which one is picked.
    tab_buttons: Vec<gtk4::Button>,
    /// The Ideas tab, after the filter tabs and always shown with them.
    ideas_button: gtk4::Button,
    /// Clear all's strip under the bar, shown only on the Waiting tab.
    clear_strip: gtk4::Box,
    /// The project list's text field, on the bar in the tabs' place while it
    /// shows: what is typed there narrows the folders.
    query: gtk4::Entry,
    /// fzf could not run once already, and a notification said so: the
    /// project list matches plain text from then on without saying it again.
    fzf_missing: Cell<bool>,
    /// Clear all, on its strip: deletes every task the tab lists, on its
    /// second press.
    clear: gtk4::Button,
    /// The keys every card shares, under the scroller so it stays put while
    /// the cards scroll, shown only while the panel has the keyboard.
    footer: gtk4::Label,
    /// For its height, which caps the column's: read at each render, since a
    /// scale change alters it.
    monitor: gdk::Monitor,
    slide: Rc<Slide>,
    /// What the panel shows, and what keys and presses do to it.
    state: RefCell<PanelState>,
    /// The cards on screen, as the last render drew them.
    cards: RefCell<Vec<CardWidgets>>,
    /// The project list's folder cards, top to bottom, while a task is
    /// being moved: each card's box, to scroll to, and its body, to focus.
    folders: RefCell<Vec<(gtk4::Box, gtk4::Button)>>,
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
    /// The task's notes under the description, shown while the state says.
    /// None on "+N more", on a task with no notes, and on every card off
    /// the keyboard.
    notes: Option<gtk4::Box>,
    /// The age at the right end, and the stamp it counts from, for the
    /// minute timer. None on "+N more".
    age: Option<(gtk4::Label, String)>,
}

/// A card's action row, and the separator over it: shown while the card
/// has the focus.
struct ActionRow {
    separator: gtk4::Separator,
    row: gtk4::Box,
    /// The keys that change from card to card, after the icons: the focused
    /// button's, and what Ctrl+Enter does to this task.
    hint: gtk4::Label,
    buttons: Vec<(Action, gtk4::Button)>,
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
    /// The filter tabs' height, Clear all's strip included when it shows,
    /// with the gap under them: what the cards sit below. 0 without the
    /// keyboard, which has no tabs.
    tabs_h: Cell<i32>,
    /// The tab bar's own height, without the strip or the gaps: the blur's.
    bar_h: Cell<i32>,
    /// Clear all's strip's width and height, (0, 0) while it is hidden: the
    /// blur's.
    clear: Cell<(i32, i32)>,
    /// The footer's height, with the gap over it: what sits under the cards.
    /// 0 without the keyboard, which has no footer.
    footer_h: Cell<i32>,
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
            bar_h: Cell::new(0),
            clear: Cell::new((0, 0)),
            footer_h: Cell::new(0),
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
        let TabBar { head: tabs, bar, tab_buttons, ideas: ideas_button, query, clear_strip, clear } = tab_bar();
        let front = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        front.append(&tabs);
        front.append(&scroller);
        let footer = keys_footer();
        front.append(&footer);
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
            bar,
            tab_buttons,
            ideas_button,
            clear_strip,
            query,
            fzf_missing: Cell::new(false),
            clear,
            footer,
            slide: Rc::new(Slide::tucked()),
            state: RefCell::new(PanelState::default()),
            cards: RefCell::new(Vec::new()),
            folders: RefCell::new(Vec::new()),
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
                    slide.tabs_h.get() + slide.cards_h.get() + slide.footer_h.get() + 2 * RING_PX,
                ))
            });
        }
        panel.connect_tab_bar();
        panel.connect_hover();
        panel.connect_keys();
        panel.connect_focus();
        panel.connect_surface();

        // The ages count up while the cards stay put. Weak, so a panel
        // dropped when its monitor goes ends its timer.
        {
            let weak = Rc::downgrade(&panel);
            glib::timeout_add_seconds_local(60, move || match weak.upgrade() {
                Some(p) => {
                    p.refresh_ages();
                    glib::ControlFlow::Continue
                }
                None => glib::ControlFlow::Break,
            });
        }

        // Map once, then hide in the same main-loop iteration. A layer surface
        // that has never been mapped ignores a later present(), so a daemon
        // started on an empty workspace would otherwise never show a panel.
        panel.window.present();
        panel.window.set_visible(false);

        panel
    }

    /// A click on a tab picks it; a click on Clear all presses it.
    fn connect_tab_bar(self: &Rc<Self>) {
        {
            // Every change of the project list's text ranks the rows again,
            // by `rank`; clearing it to what the state already has is no
            // change. The state borrow and `rank`'s `fzf_missing` are
            // different cells, so the matcher may run while the state is
            // borrowed.
            let weak = Rc::downgrade(self);
            self.query.connect_changed(move |entry| {
                let Some(p) = weak.upgrade() else { return };
                let query = entry.text().to_string();
                let matcher = |names: &[String], q: &str| p.rank(names, q);
                let effects = p.state.borrow_mut().on_query(&query, &matcher);
                p.apply(effects);
            });
        }
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
            // widget, not to the letters. Ctrl chords do too, bar Ctrl+Enter,
            // Ctrl+Delete and Ctrl+Shift+Delete, which key_action picks out:
            // Ctrl+T must not Stop.
            if modifiers.intersects(gdk::ModifierType::ALT_MASK | gdk::ModifierType::SUPER_MASK) {
                return glib::Propagation::Proceed;
            }
            let Some(p) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
            // The state maps the key by its mode: on Ideas and the project
            // list most keys are typing, for the text area or field.
            let effects = p.state.borrow_mut().on_key(key, ctrl, shift);
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
    pub fn take_keyboard(self: &Rc<Self>, agents: Vec<String>) -> bool {
        if !self.state.borrow_mut().take_keyboard(agents) {
            return false;
        }
        self.centre(vec![Effect::Render]);
        true
    }

    /// Mod+Alt+W: the project list for opening a project, in the middle of
    /// the monitor with the keyboard, as `take_keyboard` puts the cards
    /// there. With no cards too: the panel shows for the list alone.
    pub fn open_projects(self: &Rc<Self>, projects: Projects) {
        let effects = self.state.borrow_mut().open_projects(projects);
        self.centre(effects);
    }

    /// Hold the keyboard in the middle of the monitor, running `effects`,
    /// the state's render, on the way.
    ///
    /// The right anchor alone already centres the surface vertically; its
    /// right margin grows to half the room the surface leaves, centring it
    /// across too. The margin rides on the same slide as the cards, so they
    /// slide out from the peek all the way to the middle. The peek goes with
    /// it: there is one surface, and it is in the middle now.
    fn centre(self: &Rc<Self>, effects: Vec<Effect>) {
        self.cancel_grace();
        // Afresh every time: another panel may have saved this tag's ideas
        // since, its workspace having moved monitor.
        self.notepad.load(&self.tag.borrow());
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.apply(effects);
        // After the render, which measures the cards the region needs.
        self.slide_to(CENTRED_X, centre_margin(self.monitor.geometry().width()));
        self.start_spinner();
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
                Effect::Spawn(args) => run_niritasks(&self.output, &args),
                Effect::DeleteAll(uuids) => delete_all(&self.output, &uuids),
                Effect::Notify(text) => crate::notify::tasks(&text),
                Effect::SaveIdeas => self.notepad.flush(),
                Effect::ListProjects(uuid) => {
                    // Read on every press, so a folder made since shows.
                    let effects = match Projects::load() {
                        Ok(projects) => {
                            let rows = projects.move_destinations(&self.tag.borrow());
                            self.state.borrow_mut().show_projects(&uuid, rows, projects)
                        }
                        Err(e) => vec![Effect::Notify(e.to_string())],
                    };
                    self.apply(effects);
                }
                Effect::ShowFolder => self.follow_focus(),
                // Its changed handler finds the state already cleared.
                Effect::ClearQuery => self.query.set_text(""),
                // The card grew or shrank: keep the whole of the focused one
                // in view, as a move of the focus does.
                Effect::Notes => {
                    self.show_notes();
                    self.follow_focus();
                }
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

    /// Which filter tabs show, which tab is picked, and whether Clear all's
    /// strip shows. Ideas shows with the tabs: the bar itself hides without
    /// the keyboard. On the project list its text field stands in for them
    /// all.
    fn update_tabs(&self) {
        let text = {
            let state = self.state.borrow();
            let shown = state.tabs();
            for (button, filter) in self.tab_buttons.iter().zip(Filter::TABS) {
                button.set_visible(shown.contains(&filter));
                set_class(button, "current", state.tab() == Tab::Filter(filter));
            }
            set_class(&self.ideas_button, "current", state.on_ideas());
            self.clear_strip.set_visible(state.shows_clear_all());
            let list = state.projects();
            self.ideas_button.set_visible(list.is_none());
            self.query.set_visible(list.is_some());
            list.map_or(String::new(), |m| m.query().to_string())
        };
        // Outside the borrow: setting the text runs the field's changed
        // handler there and then, which finds the state already has it.
        if self.query.text() != text {
            self.query.set_text(&text);
        }
    }

    /// The names matching what is typed on the project list, best first:
    /// fzf's ranking, or, when fzf cannot run, the names holding the text,
    /// with a notification the first time saying why. The list never asks
    /// with nothing typed: it shows every row then.
    fn rank(&self, folders: &[String], query: &str) -> Vec<String> {
        match crate::project::fzf_matches(folders, query) {
            Ok(matches) => matches,
            Err(e) => {
                if !self.fzf_missing.replace(true) {
                    crate::notify::tasks(&format!("{e:#}; the project list matches plain text instead."));
                }
                crate::project::substring_matches(folders, query)
            }
        }
    }

    /// Measure the cards, the tabs and the footer, and size the surface to them: the
    /// heights the blur region and the input region work from. Run by every
    /// render, and again whenever a card's action row shows or hides, which
    /// changes its height without a render.
    fn fit(&self) {
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
        let tabs_h = if self.tabs.is_visible() {
            self.tabs.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX).1
        } else {
            0
        };

        // The bar and Clear all's strip apart, for the blur, which leaves
        // out the gap between them and the room left of the strip.
        let bar_shown = self.tabs.is_visible();
        let bar_h = if bar_shown { self.bar.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX).1 } else { 0 };
        let clear = if bar_shown && self.clear_strip.is_visible() {
            (
                self.clear_strip.measure(gtk4::Orientation::Horizontal, -1).1,
                self.clear_strip.measure(gtk4::Orientation::Vertical, -1).1,
            )
        } else {
            (0, 0)
        };

        // The footer, measured the same way: margins included, so the gap
        // over it and the ring room under it. Hidden on Ideas, so 0 there.
        let footer_h = if self.footer.is_visible() {
            self.footer.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX).1
        } else {
            0
        };

        // The column's margins are in what it measures, and in the width it
        // is measured for; the cards' height is without them.
        let (_, with_ring, _, _) =
            self.column.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX);
        let cards_h = with_ring - 2 * RING_PX;
        let shown = shown_height(cards_h, tabs_h + footer_h, self.monitor.geometry().height());
        self.slide.tabs_h.set(tabs_h);
        self.slide.bar_h.set(bar_h);
        self.slide.clear.set(clear);
        self.slide.footer_h.set(footer_h);
        self.slide.cards_h.set(shown);
        let height = tabs_h + shown + footer_h + 2 * SHADOW_PX;
        // Both calls: the size request lets the surface grow, the default size
        // lets it shrink back when the list gets shorter.
        self.base.set_size_request(SURFACE_WIDTH, height);
        self.window.set_size_request(SURFACE_WIDTH, height);
        self.window.set_default_size(SURFACE_WIDTH, height);
    }

    /// Rewrite each card's age where it stands. Not a render: the cards are
    /// the same, and a render would tear the column down and disarm a
    /// half-pressed Remove. A wrapped card can gain or lose a line when its
    /// age changes width, so the surface is fitted again if any text changed.
    fn refresh_ages(&self) {
        let now = crate::task::now_secs();
        let mut changed = false;
        for card in self.cards.borrow().iter() {
            if let Some((label, entry)) = &card.age {
                if let Some(age) = crate::task::age(entry, now) {
                    if label.text() != age {
                        label.set_text(&age);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.fit();
            self.set_region(self.slide.x.get().min(self.slide.to.get()));
            self.update_blur(self.slide.x.get());
        }
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
        let (shown, keyboard, empty, focus, mode, hint) = {
            let state = self.state.borrow();
            (
                state.visible(),
                state.keyboard(),
                state.empty_text(),
                state.focus().cloned(),
                state.mode().clone(),
                state.hint(),
            )
        };
        let ideas = matches!(mode, Mode::Ideas);
        let list = match &mode {
            Mode::Projects { list, .. } => Some(list),
            _ => None,
        };

        self.while_drawing(|| match &mode {
            Mode::Ideas => self.draw_ideas(),
            Mode::Projects { list, .. } => self.draw_projects(list, hint.as_deref()),
            Mode::Tasks => self.draw_tasks(&shown, keyboard, empty),
        });
        // On the project list the bar holds its text field instead of the
        // tabs, which filter task cards.
        self.tabs.set_visible(keyboard);
        // The footer names the cards' keys, so not on Ideas, where the
        // text area takes Enter and Delete as typing; on the project list,
        // its own.
        let footer = self.state.borrow().footer();
        self.footer.set_visible(footer.is_some());
        if let Some(text) = footer {
            self.footer.set_label(text);
        }
        self.update_tabs();
        self.fit();

        // present(), not set_visible(true): see new().
        self.window.present();
        self.set_region(self.slide.x.get().min(self.slide.to.get()));
        self.update_blur(self.slide.x.get());

        if ideas {
            self.notepad.focus();
        } else if list.is_some() {
            // Without selecting: a render mid-typing must not select what
            // was typed, for the next key to replace.
            self.query.grab_focus_without_selecting();
        } else if keyboard {
            self.focus_on(focus.as_ref());
        }
    }

    /// The Ideas notepad in the column in place of the cards. Mid-typing, a
    /// tick's render leaves the notepad where it is: taking it out of the
    /// column would take its focus with it.
    fn draw_ideas(&self) {
        let column: &gtk4::Widget = self.column.upcast_ref();
        if self.notepad.root.parent().as_ref() == Some(column) {
            return;
        }
        self.clear_column();
        self.cards.borrow_mut().clear();
        self.folders.borrow_mut().clear();
        self.column.append(&self.notepad.root);
    }

    /// The project list in the column: what it is for over its folders,
    /// then, with nothing matching on the open list, what Enter does
    /// instead.
    fn draw_projects(self: &Rc<Self>, list: &ProjectList, hint: Option<&str>) {
        self.clear_column();
        self.cards.borrow_mut().clear();
        self.column.append(&empty_line(&list.title()));
        let mut folders = Vec::new();
        for (at, row) in list.shown().iter().enumerate() {
            let card = self.folder_card(at, row);
            self.column.append(&card.0);
            folders.push(card);
        }
        *self.folders.borrow_mut() = folders;
        if let Some(text) = hint {
            self.column.append(&empty_line(text));
        }
    }

    /// The task cards in the column, or the line that says the tab has none.
    fn draw_tasks(self: &Rc<Self>, shown: &[Shown], keyboard: bool, empty: Option<&str>) {
        self.clear_column();
        let cards: Vec<CardWidgets> = shown.iter().map(|s| self.card_widget(s, keyboard)).collect();
        for card in &cards {
            self.column.append(&card.root);
        }
        *self.cards.borrow_mut() = cards;
        self.folders.borrow_mut().clear();
        if let Some(text) = empty {
            // Only All, with every task waiting or finished: it says so,
            // and the tabs beside it have them.
            self.column.append(&empty_line(text));
        }
    }

    /// Take every widget out of the column, for a screen drawn afresh.
    fn clear_column(&self) {
        while let Some(child) = self.column.first_child() {
            self.column.remove(&child);
        }
    }

    /// One card: a box holding the body, a button so the keyboard can focus
    /// and press it, and, while the panel has the keyboard, its action row
    /// along the bottom, hidden until the card has focus. The body shows or
    /// hides the task's notes on a task's card, and shows the rest in place
    /// of "+N more".
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
            Status::Finished => root.add_css_class("finished"),
            Status::Pending => {}
        }
        // Its own class, beside the status's: an up next card is yellow
        // whatever its icon, unless it is active, waiting or finished.
        if card.shows_up_next() {
            root.add_css_class("up-next");
        }
        root.set_size_request(CARD_WIDTH_PX, -1);
        // Clips the action row to the card's rounded bottom corners.
        root.set_overflow(gtk4::Overflow::Hidden);

        let (label, age) = card_label(card, keyboard);
        // The notes go inside the body, under the description, so a click
        // on them hides them as a click on the description does.
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.append(&label);
        let notes = card.uuid.as_deref().filter(|_| keyboard && !card.notes.is_empty()).map(|uuid| {
            let notes = notes_box(&card.notes);
            notes.set_visible(self.state.borrow().shows_notes(uuid));
            content.append(&notes);
            notes
        });
        let body = gtk4::Button::builder().child(&content).build();
        body.add_css_class("card-body");
        // A click leaves the card as it was, not darkened.
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

        let row = card
            .uuid
            .as_ref()
            .zip(card.state(false))
            .filter(|_| !shown.actions.is_empty())
            .map(|(uuid, state)| {
                let row = self.action_row(uuid, state, &shown.actions);
                root.append(&row.separator);
                root.append(&row.row);
                row
            });
        let age = age.map(|label| (label, card.since.clone()));
        CardWidgets { uuid: card.uuid.clone(), root, body, row, notes, age }
    }

    /// A card's buttons, left-aligned and only as wide as their icons, then
    /// the focused one's name in words: the icons alone do not say what they
    /// do. Hidden, with the line over them, until the card has focus.
    fn action_row(self: &Rc<Self>, uuid: &str, state: TaskState, actions: &[Action]) -> ActionRow {
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
            button.set_tooltip_text(Some(action.label(state.up_next)));
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
        ActionRow { separator, row, hint, buttons }
    }

    /// One row on the project list, a folder or a repo, drawn as a task card
    /// is: a box holding a body button with the row's icon and name. A click
    /// on it picks it. Out of the focus chain: the text field keeps
    /// the keyboard, and the highlight (`sync`) says which folder Enter
    /// takes.
    fn folder_card(self: &Rc<Self>, at: usize, row: &Row) -> (gtk4::Box, gtk4::Button) {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        root.add_css_class("task-card");
        root.set_size_request(CARD_WIDTH_PX, -1);
        root.set_overflow(gtk4::Overflow::Hidden);
        let label = gtk4::Label::new(Some(&ProjectList::label(row)));
        label.set_xalign(0.0);
        let body = gtk4::Button::builder().child(&label).build();
        body.add_css_class("card-body");
        body.set_focusable(false);
        body.set_focus_on_click(false);
        let weak = Rc::downgrade(self);
        body.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                let effects = p.state.borrow_mut().on_folder(at);
                p.apply(effects);
            }
        });
        root.append(&body);
        (root, body)
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
    /// and its hint for the focused slot.
    fn sync(&self) {
        let (focus, armed) = {
            let state = self.state.borrow();
            (state.focus().cloned(), state.armed().clone())
        };
        let picked = self.state.borrow().projects().map(ProjectList::at);
        for (at, (root, _)) in self.folders.borrow().iter().enumerate() {
            set_class(root, "picked", picked == Some(at));
        }
        let clear_armed = matches!(armed, Armed::ClearAll { .. });
        // Armed, its label is longer and the strip wider: the blur follows it.
        let relabelled = self.clear.label().as_deref() != Some(clear_label(clear_armed).as_str());
        set_label(&self.clear, &clear_label(clear_armed));
        set_class(&self.clear, "confirm", clear_armed);
        for card in self.cards.borrow().iter() {
            let Some(row) = &card.row else { continue };
            let removing = matches!(&armed, Armed::Remove(uuid) if Some(uuid) == card.uuid.as_ref());
            if let Some(remove) = card.button(Action::Remove) {
                set_label(remove, &remove_label(removing));
                set_class(remove, "confirm", removing);
            }
            let hint = card.uuid.as_deref().map_or(String::new(), |u| self.state.borrow().card_hint(u));
            // Only when it changed: every sync passes every card, and an
            // unchanged label should not cost a relayout.
            if row.hint.label().as_str() != hint.as_str() {
                row.hint.set_label(&hint);
            }
        }
        self.show_row(focus.as_ref().map(|f| f.uuid.as_str()));
        if relabelled && self.clear_strip.is_visible() {
            self.fit();
            self.update_blur(self.slide.x.get());
        }
    }

    /// Scroll the focused card wholly into view, body and buttons, once the
    /// frame after the move has laid it out. A card just rendered has no place
    /// in the column until then.
    fn follow_focus(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        crate::taskbox::after_next_paint(&self.window, move || {
            let Some(p) = weak.upgrade() else { return };
            let folder = p.state.borrow().projects().map(ProjectList::at);
            let root = match folder {
                Some(at) => p.folders.borrow().get(at).map(|(root, _)| root.clone()),
                None => {
                    let Some(uuid) = p.state.borrow().focus().map(|f| f.uuid.clone()) else { return };
                    p.cards.borrow().iter().find(|c| c.uuid.as_ref() == Some(&uuid)).map(|c| c.root.clone())
                }
            };
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

    /// Show the notes of the cards the state says and hide the rest. Like
    /// the action row, they change a card's height without a render, so the
    /// surface, the input region and the blur are fitted to the cards again,
    /// but only when some card's notes actually showed or hid.
    fn show_notes(&self) {
        let mut changed = false;
        {
            let state = self.state.borrow();
            for card in self.cards.borrow().iter() {
                let (Some(uuid), Some(notes)) = (&card.uuid, &card.notes) else { continue };
                let show = state.shows_notes(uuid);
                if notes.is_visible() != show {
                    notes.set_visible(show);
                    changed = true;
                }
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
        // footer under the cards on screen.
        let height = self.slide.tabs_h.get() + self.slide.cards_h.get() + self.slide.footer_h.get();
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
            let bar_h = self.slide.bar_h.get();
            rects.extend(blur::card_region((x, SHADOW_PX, width, bar_h), RADIUS_PX, on_screen));
            // Clear all's strip under it, when it shows; not the gap over it
            // or the room to its left.
            let clear = self.slide.clear.get();
            if clear.1 > 0 {
                let strip = clear_strip_rect(x, bar_h, clear);
                if strip.2 > 0 {
                    rects.extend(blur::card_region(strip, RADIUS_PX, on_screen));
                }
            }
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
        let footer_h = self.slide.footer_h.get();
        if footer_h > 0 {
            // The footer, which does not scroll: under the cards on screen
            // and the card gap, not the gap itself.
            let y = top + self.slide.cards_h.get() + GAP_PX;
            rects.extend(blur::card_region((x, y, width, footer_h - GAP_PX), RADIUS_PX, on_screen));
        }
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

/// A task's notes, for under its card's description: one dimmed label per
/// note, wrapped, text only. The panel shows the box once the card's body is
/// pressed.
fn notes_box(notes: &[String]) -> gtk4::Box {
    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    column.add_css_class("card-notes");
    for note in notes {
        let label = gtk4::Label::new(Some(note));
        label.add_css_class("card-note");
        label.set_xalign(0.0);
        label.set_wrap(true);
        // WordChar: a long path or URL with no spaces still breaks.
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        column.append(&label);
    }
    column
}

/// The card's icon, description and age: one line cut off with "…" for the
/// peek and the hover, or all of it, wrapped, while the panel has the
/// keyboard. The age label comes back too, for the minute timer to update.
///
/// The icon is a label of its own beside the text, so wrapped lines start
/// under the first line's text rather than back under the icon. The age is
/// one too, at the right end, so the text's "…" stops short of it.
fn card_label(card: &Card, wrap: bool) -> (gtk4::Box, Option<gtk4::Label>) {
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
    let age = card.age(crate::task::now_secs()).map(|a| {
        let age = gtk4::Label::new(Some(&a));
        age.add_css_class("card-age");
        // Level with the first line when the text wraps.
        age.set_valign(gtk4::Align::Start);
        row.append(&age);
        age
    });
    (row, age)
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

/// Where Clear all's strip is, for the blur behind it: a card gap under the
/// bar, its right edge on the cards', and cut at the surface's edge as the
/// cards are, to no width at all once it is wholly past it.
fn clear_strip_rect(x: i32, bar_h: i32, (w, h): (i32, i32)) -> blur::Rect {
    let left = x + CARD_WIDTH_PX - w;
    (left, SHADOW_PX + bar_h + GAP_PX, w.min(SURFACE_WIDTH - left).max(0), h)
}

/// A tab button's face, as Pango markup: its name, then the key that picks
/// it in brackets, so the keys can be learnt off the bar. The key is set
/// smaller, a caption to the name rather than part of it. The plain name
/// stays `Tab::label`'s, for the tests and the empty tab's line.
fn tab_label(tab: Tab) -> String {
    format!(
        "{} <span size=\"smaller\">({})</span>",
        glib::markup_escape_text(tab.label()),
        keys::tab_number(tab)
    )
}

/// A tab button wearing [`tab_label`]'s markup, which `Button::with_label`
/// would print as it stands.
fn tab_button(tab: Tab) -> gtk4::Button {
    let face = gtk4::Label::new(None);
    face.set_markup(&tab_label(tab));
    let button = gtk4::Button::new();
    button.set_child(Some(&face));
    button
}

/// What [`tab_bar`] builds, for the panel to keep.
struct TabBar {
    /// The bar and Clear all's strip, one over the other, with the ring
    /// and gap margins: what the panel shows, hides and measures.
    head: gtk4::Box,
    /// The tabs' strip alone, for the blur behind it.
    bar: gtk4::Box,
    tab_buttons: Vec<gtk4::Button>,
    ideas: gtk4::Button,
    /// The project list's text field, on the bar in the tabs' place.
    query: gtk4::Entry,
    /// Clear all's own strip, at the right under the bar, shown on the
    /// Waiting tab alone.
    clear_strip: gtk4::Box,
    clear: gtk4::Button,
}

/// The filter tabs and Ideas on a bar (the project list's text field in
/// their place while it shows), and under it Clear all on a strip of its
/// own, over the scroller rather than in it, so they stay put while the
/// cards scroll. Clear all is off the bar because the seven tabs fill the
/// card's width. Ring room on three sides, as the column keeps, and under
/// them the card gap less the ring the column keeps above the first card:
/// the first card then sits a card gap below.
fn tab_bar() -> TabBar {
    let head = gtk4::Box::new(gtk4::Orientation::Vertical, GAP_PX);
    head.set_margin_top(RING_PX);
    head.set_margin_start(RING_PX);
    head.set_margin_end(RING_PX);
    head.set_margin_bottom(GAP_PX - RING_PX);
    head.set_visible(false);
    let bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    bar.add_css_class("filter-tabs");
    // Clips the picked tab's fill to the bar's rounded corners.
    bar.set_overflow(gtk4::Overflow::Hidden);
    head.append(&bar);
    let tab_buttons: Vec<gtk4::Button> = Filter::TABS
        .iter()
        .map(|filter| {
            let button = tab_button(Tab::Filter(*filter));
            // Out of the focus chain: the arrows and Tab stay between the
            // cards and their buttons, and a click does not take the focus.
            button.set_focusable(false);
            button.set_focus_on_click(false);
            bar.append(&button);
            button
        })
        .collect();
    // Ideas, last of the tabs: not a filter, and always shown.
    let ideas = tab_button(Tab::Ideas);
    ideas.set_focusable(false);
    ideas.set_focus_on_click(false);
    bar.append(&ideas);
    // The project list's text field, the whole bar wide while it shows and
    // the tabs hide.
    let query = gtk4::Entry::new();
    query.add_css_class("project-query");
    query.set_placeholder_text(Some("Type to find a project"));
    query.set_hexpand(true);
    query.set_visible(false);
    bar.append(&query);
    // Clear all's strip: dressed as the bar is, and End keeps it its own
    // width at the right, under the bar's end.
    let clear_strip = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    clear_strip.add_css_class("filter-tabs");
    clear_strip.set_halign(gtk4::Align::End);
    // Clips the armed fill to the strip's rounded corners.
    clear_strip.set_overflow(gtk4::Overflow::Hidden);
    clear_strip.set_visible(false);
    // Out of the focus chain like the tabs, which is why Ctrl+Shift+Delete
    // presses it.
    let clear = gtk4::Button::with_label(&clear_label(false));
    clear.add_css_class("clear-all");
    clear.set_tooltip_text(Some("Ctrl+Shift+Delete"));
    clear.set_focusable(false);
    clear.set_focus_on_click(false);
    clear_strip.append(&clear);
    head.append(&clear_strip);
    TabBar { head, bar, tab_buttons, ideas, query, clear_strip, clear }
}

/// The keys that act on whichever card has the focus, in a strip shaped like
/// the tab bar: ring room on three sides, as the column keeps, and over it the
/// card gap less the ring the column keeps under the last card, so it sits a
/// card gap below.
fn keys_footer() -> gtk4::Label {
    let footer = gtk4::Label::new(Some(actions::CARD_KEYS));
    footer.add_css_class("keys-footer");
    footer.set_xalign(0.0);
    footer.set_margin_top(GAP_PX - RING_PX);
    footer.set_margin_start(RING_PX);
    footer.set_margin_end(RING_PX);
    footer.set_margin_bottom(RING_PX);
    footer.set_visible(false);
    footer
}

/// The one line a filter tab with nothing under it shows where its cards
/// would be (only All, when every task is waiting or finished), in a card's
/// look so it reads as part of the panel. Not a card: it has no task and
/// nothing to focus. Also the line naming the task over the project list.
fn empty_line(text: &str) -> gtk4::Label {
    let line = gtk4::Label::new(Some(text));
    line.add_css_class("filter-empty");
    line.set_xalign(0.0);
    line.set_wrap(true);
    line.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
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

/// Run a `niritasks` command on this monitor: one of a card's buttons. See
/// `spawn_on`.
fn run_niritasks(output: &str, args: &[String]) {
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
/// This monitor is focused first, so the command files under the workspace
/// the panel shows rather than whichever monitor had focus, and a box it
/// opens comes up on the screen that was clicked. It runs as its own process,
/// spawned by niri, so the daemon never waits on it.
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
/// inside the screen's margins less the `bars_h` the filter tabs over them and
/// the keys footer under them take, with the rest scrolled. Only the
/// keyboard's wrapped cards, or "+N more" opened onto a long list, get that
/// tall.
fn shown_height(cards_h: i32, bars_h: i32, screen_h: i32) -> i32 {
    cards_h.min(screen_h - 2 * (SHADOW_PX + EDGE_GAP_PX) - bars_h).max(0)
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

    /// Clear all's strip sits a card gap under the bar, its right edge on
    /// the cards' right edge, its own size.
    #[test]
    fn clear_alls_strip_sits_under_the_bar_at_the_cards_right_edge() {
        let x = SURFACE_WIDTH - CARD_WIDTH_PX;
        assert_eq!(
            clear_strip_rect(x, 30, (120, 36)),
            (x + CARD_WIDTH_PX - 120, SHADOW_PX + 30 + GAP_PX, 120, 36)
        );
    }

    /// Pushed past the surface's edge, the strip is cut there, as the cards are.
    #[test]
    fn clear_alls_strip_is_cut_at_the_surface_edge() {
        let x = SURFACE_WIDTH - CARD_WIDTH_PX + 50;
        assert_eq!(clear_strip_rect(x, 30, (120, 36)).2, 70);
    }

    /// At the peek's x, as when 5 is pressed during the slide-in, the cards
    /// are far past the edge: the strip's width is 0, never negative, which
    /// the Wayland region would take.
    #[test]
    fn clear_alls_strip_has_no_width_at_the_peeks_x() {
        assert_eq!(clear_strip_rect(TUCKED_X as i32, 30, (120, 36)).2, 0);
    }

    /// A tab wears its name, a space, and the key that picks it in
    /// brackets, set smaller.
    #[test]
    fn a_tab_wears_its_key_in_brackets() {
        assert_eq!(tab_label(Tab::Filter(Filter::All)), "All <span size=\"smaller\">(1)</span>");
        assert_eq!(tab_label(Tab::Filter(Filter::ToRefine)), "To refine <span size=\"smaller\">(4)</span>");
        assert_eq!(tab_label(Tab::Ideas), "Ideas <span size=\"smaller\">(7)</span>");
    }

    /// Every tab shown at once, numbered, still fits the bar's one line:
    /// Iosevka Term Extended at 10pt is 8px a character, and each tab adds
    /// 24px of padding and, after the first, a 1px line. Counting the
    /// smaller key at the full size, 758px of 760, so it fits with room over.
    #[test]
    fn the_numbered_tabs_fit_the_bar() {
        let tabs: Vec<Tab> = Filter::TABS.into_iter().map(Tab::Filter).chain([Tab::Ideas]).collect();
        let width: usize = tabs
            .iter()
            .map(|t| format!("{} ({})", t.label(), keys::tab_number(*t)).chars().count() * 8 + 24)
            .sum::<usize>()
            + tabs.len()
            - 1;
        assert!(width <= CARD_WIDTH_PX as usize, "{width}px");
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
