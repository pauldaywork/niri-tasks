//! What the task panel is showing and what a key or a press does to it, as
//! plain data.
//!
//! The cards, the filter tab, "+N more" opened or not, which card and button
//! has the keyboard's focus, and what a first press has armed all live here,
//! keyed by task uuid and slot rather than by widget. Every change comes back
//! as a list of [`Effect`]s for `surface.rs` to run: draw again, move the
//! focus, spawn a command. So the rules between them (one arming at a time,
//! any re-render disarming, where the focus lands) are tested without a
//! window, and `surface.rs` only draws what this says and runs what it asks.

use super::actions::Action;
use super::keys::{self, KeyAction};
use super::model::{self, Card, Filter};

/// Where on a card the keyboard's focus is: its body, or one of the buttons
/// on its action row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Body,
    Button(Action),
}

/// The focused card, by its task's uuid, and the slot on it. Never "+N
/// more": Down onto it shows the cards it stands for instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Focus {
    pub uuid: String,
    pub slot: Slot,
}

impl Focus {
    pub fn body(uuid: &str) -> Focus {
        Focus { uuid: uuid.to_string(), slot: Slot::Body }
    }
}

/// What a first press has armed, waiting for its second. One enum, so only
/// one of Remove and Clear all can be armed at a time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Armed {
    #[default]
    None,
    /// This task's Remove reads Confirm remove.
    Remove(String),
    /// Clear all reads Confirm clear all, and the focus is off the cards.
    /// `before` is where it was, and where cancelling puts it back.
    ClearAll { before: Option<Focus> },
}

/// A card as the panel draws it: the card, and the buttons on its action
/// row, left to right. None off the keyboard, or on "+N more".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub card: Card,
    pub actions: Vec<Action>,
}

impl Shown {
    /// What the keyboard can stop on, left to right: the body, then the
    /// buttons.
    pub fn slots(&self) -> Vec<Slot> {
        std::iter::once(Slot::Body).chain(self.actions.iter().map(|a| Slot::Button(*a))).collect()
    }
}

/// What `surface.rs` does once the state has changed, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Draw [`PanelState::visible`] again, with the tabs and Clear all, then
    /// put the focus on [`PanelState::focus`]. Or hide the panel, when
    /// [`PanelState::hidden`].
    Render,
    /// Put the focus on this card's slot, or on nothing: Clear all armed.
    Focus(Option<Focus>),
    /// The keyboard was given back: off exclusive keyboard, and the cards
    /// snapped back to the peek.
    Release,
    /// Run `niritasks` with these arguments on this monitor.
    Spawn(Vec<String>),
    /// Clear all's deletes: these tasks, one after another.
    DeleteAll(Vec<String>),
    /// Say this in a notification.
    Notify(String),
}

#[derive(Debug, Default)]
pub struct PanelState {
    /// Every card, uncapped and unfiltered.
    all: Vec<Card>,
    /// The panel has the keyboard, from Mod+Alt+Ctrl+T.
    keyboard: bool,
    /// The filter tab picked. All whenever the panel takes the keyboard;
    /// kept across a refresh while it has it.
    filter: Filter,
    /// "+N more" was clicked, or Down reached it: every card shows until the
    /// panel tucks away or gives the keyboard back.
    expanded: bool,
    /// The live herdr agents in the workspace's session, asked once as the
    /// panel takes the keyboard: which cards get Go to session.
    agents: Vec<String>,
    /// Where the keyboard's focus is. None without the keyboard, and while
    /// Clear all is armed.
    focus: Option<Focus>,
    armed: Armed,
}

impl PanelState {
    pub fn keyboard(&self) -> bool {
        self.keyboard
    }

    pub fn filter(&self) -> Filter {
        self.filter
    }

    pub fn focus(&self) -> Option<&Focus> {
        self.focus.as_ref()
    }

    pub fn armed(&self) -> &Armed {
        &self.armed
    }

    /// Nothing to show, so the panel hides. The hover and the peek show no
    /// waiting task, so they hide with nothing else; the keyboard's panel
    /// still has the Waiting tab, and hides only with no task at all.
    pub fn hidden(&self) -> bool {
        if self.keyboard {
            self.all.is_empty()
        } else {
            Filter::All.pick(&self.all).is_empty()
        }
    }

    /// The filter tabs on show, in `Filter::TABS` order: All, and every
    /// other tab with a task under it. None without the keyboard.
    pub fn tabs(&self) -> Vec<Filter> {
        if self.keyboard {
            Filter::shown(&self.all)
        } else {
            Vec::new()
        }
    }

    /// Clear all ends the tab bar on the Waiting tab alone.
    pub fn shows_clear_all(&self) -> bool {
        self.keyboard && self.filter == Filter::Waiting
    }

    /// The cards to draw, top to bottom: the picked tab's, or All's off the
    /// keyboard, filtered before the cap so "+N more" is the rest of this
    /// tab, and uncapped once expanded.
    pub fn visible(&self) -> Vec<Shown> {
        let picked = self.shown_filter().pick(&self.all);
        let cards = if self.expanded { picked } else { model::cap(&picked, model::CAP) };
        cards
            .into_iter()
            .map(|card| {
                let actions = match card.uuid.as_deref().filter(|_| self.keyboard) {
                    Some(uuid) => {
                        let has_session = crate::link::session_agent(&self.agents, uuid).is_some();
                        Action::for_status(card.status, has_session)
                    }
                    None => Vec::new(),
                };
                Shown { card, actions }
            })
            .collect()
    }

    /// The line shown in place of cards when the tab has none: only All, on
    /// a workspace whose tasks are all waiting.
    pub fn empty_text(&self) -> Option<&'static str> {
        let filter = self.shown_filter();
        filter.pick(&self.all).is_empty().then(|| filter.empty_text())
    }

    /// The daemon's cards, on each tick. Nothing when they have not changed;
    /// otherwise a re-render, which disarms and keeps the focus where it can.
    /// The last task going while the panel has the keyboard gives it back, so
    /// the next card shows as a peek and the next Mod+Alt+Ctrl+T opens on All.
    pub fn set_cards(&mut self, cards: &[Card]) -> Vec<Effect> {
        if self.all == cards {
            return Vec::new();
        }
        self.all = cards.to_vec();
        if self.keyboard && self.all.is_empty() {
            return self.release();
        }
        self.rerender()
    }

    /// Take the keyboard, on All every time, also when taken again while it
    /// has it, with the focus on the first card. `agents` are the session's
    /// live agent names, for Go to session. False with no cards to take it for.
    pub fn take_keyboard(&mut self, agents: Vec<String>) -> bool {
        if self.all.is_empty() {
            return false;
        }
        self.keyboard = true;
        self.agents = agents;
        self.filter = Filter::All;
        self.armed = Armed::None;
        self.focus = first_task(&self.visible());
        true
    }

    /// The pointer left and the cards slid back: an expanded list folds up.
    pub fn on_tuck(&mut self) -> Vec<Effect> {
        if std::mem::take(&mut self.expanded) {
            vec![Effect::Render]
        } else {
            Vec::new()
        }
    }

    /// "+N more" clicked, or Down onto it: every card in its place, the focus
    /// on the first it hid so the next Down carries on down the list.
    pub fn on_more(&mut self) -> Vec<Effect> {
        self.expanded = true;
        self.armed = Armed::None;
        if self.keyboard {
            let shown = self.visible();
            self.focus = shown.get(model::CAP).and_then(|s| s.card.uuid.as_deref()).map(Focus::body);
        }
        vec![Effect::Render]
    }

    /// A click on a filter tab.
    pub fn on_tab(&mut self, filter: Filter) -> Vec<Effect> {
        self.pick(filter)
    }

    /// GTK moved the focus: a click on a button, or the focus an
    /// [`Effect::Focus`] asked for, which is no change. Moving off an armed
    /// Remove disarms it, and moving at all disarms Clear all. Without the
    /// keyboard it is ignored: GTK moves focus in a window that is only being
    /// shown, and the hover has none.
    pub fn on_focus(&mut self, focus: Option<Focus>) {
        if !self.keyboard || self.focus == focus {
            return;
        }
        self.focus = focus;
        let keep = match &self.armed {
            Armed::Remove(uuid) => self.focus.as_ref().is_some_and(|f| {
                f.uuid == *uuid && f.slot == Slot::Button(Action::Remove)
            }),
            Armed::ClearAll { .. } => false,
            Armed::None => true,
        };
        if !keep {
            self.armed = Armed::None;
        }
    }

    /// A key while the panel has the keyboard. None when it is not the
    /// panel's key, and GTK should have it: Enter and Space press the focused
    /// button. Armed, Clear all takes every key: Enter confirms rather than
    /// opening a card, Escape and the keys that move cancel, the tab keys
    /// switch tab as ever, and a card's keys do nothing, there being no card
    /// focused to act on. Without the keyboard no key is the panel's: one
    /// landing after it was given back is not for it.
    pub fn on_key(&mut self, key: KeyAction) -> Option<Vec<Effect>> {
        if !self.keyboard {
            return None;
        }
        if matches!(self.armed, Armed::ClearAll { .. }) {
            return Some(match key {
                KeyAction::Enter | KeyAction::ClearAll => self.on_clear_all(),
                KeyAction::Release
                | KeyAction::PrevCard
                | KeyAction::NextCard
                | KeyAction::PrevSlot
                | KeyAction::NextSlot => self.cancel_clear(),
                KeyAction::Filter(_) | KeyAction::PrevFilter | KeyAction::NextFilter => self.filter_key(key),
                KeyAction::Run(_) | KeyAction::Advance | KeyAction::Ignore => Vec::new(),
            });
        }
        Some(match key {
            KeyAction::Enter | KeyAction::Ignore => return None,
            KeyAction::Release => self.release(),
            KeyAction::ClearAll => self.on_clear_all(),
            KeyAction::Filter(_) | KeyAction::PrevFilter | KeyAction::NextFilter => self.filter_key(key),
            KeyAction::PrevCard | KeyAction::NextCard => self.move_card(key == KeyAction::NextCard),
            KeyAction::PrevSlot | KeyAction::NextSlot => self.move_slot(key == KeyAction::NextSlot),
            KeyAction::Run(action) => self.run(action),
            KeyAction::Advance => self.advance(),
        })
    }

    /// A press on a task's card: its body, which opens the whole menu, or a
    /// button. Remove only arms itself the first time, as the menu's delete
    /// asks first; the second press runs it. Everything that opens something
    /// gives the keyboard back first, so the box or terminal it opens can
    /// take it. Back, Waiting and Remove take the card off the list, so the
    /// focus moves to the next card (the one above, from the last) to still be
    /// there when the next tick drops this one. Speak and Up next keep the
    /// card and the focus, so a second press undoes them.
    pub fn on_press(&mut self, uuid: &str, slot: Slot) -> Vec<Effect> {
        let Slot::Button(action) = slot else {
            let mut effects = self.release();
            effects.push(Effect::Spawn(vec!["task".into(), "menu".into(), uuid.into()]));
            return effects;
        };
        if action == Action::Remove && self.armed != Armed::Remove(uuid.into()) {
            // Focus first: the move disarms whatever was armed before, and
            // this one is armed only after it.
            let effects = self.focus_on(Focus { uuid: uuid.into(), slot });
            self.armed = Armed::Remove(uuid.into());
            return effects;
        }
        let mut effects = Vec::new();
        if action.leaves_the_list() {
            if let Some(next) = self.neighbour(uuid) {
                effects = self.focus_on(next);
            }
        } else if !action.keeps_keyboard() {
            effects = self.release();
        }
        effects.push(Effect::Spawn(action.args(uuid)));
        effects
    }

    /// Clear all, by its button or Ctrl+Delete. The first press arms it and
    /// takes the focus off the cards, so Enter confirms rather than opening a
    /// card's menu. The second deletes every task the Waiting tab lists,
    /// those past "+N more" too, and puts the panel on All at once rather
    /// than as each delete lands, keeping the keyboard. Nothing off the
    /// Waiting tab, where Clear all is hidden.
    pub fn on_clear_all(&mut self) -> Vec<Effect> {
        if !self.shows_clear_all() {
            return Vec::new();
        }
        if !matches!(self.armed, Armed::ClearAll { .. }) {
            let before = self.focus.take();
            self.armed = Armed::ClearAll { before };
            return vec![Effect::Focus(None)];
        }
        let uuids = Filter::Waiting.uuids(&self.all);
        let mut effects = self.pick(Filter::All);
        if !uuids.is_empty() {
            effects.push(Effect::DeleteAll(uuids));
        }
        effects
    }

    /// Give the keyboard back: every card to one line again, All for the next
    /// time, and nothing expanded, armed or focused.
    fn release(&mut self) -> Vec<Effect> {
        if !self.keyboard {
            return Vec::new();
        }
        self.keyboard = false;
        self.agents.clear();
        self.expanded = false;
        self.filter = Filter::All;
        self.armed = Armed::None;
        self.focus = None;
        vec![Effect::Render, Effect::Release]
    }

    /// The tab the cards are drawn under: only the keyboard's panel has tabs.
    fn shown_filter(&self) -> Filter {
        if self.keyboard {
            self.filter
        } else {
            Filter::All
        }
    }

    /// Show this tab's cards. Nothing without the keyboard, for a tab hidden
    /// for having no tasks, or for the tab already picked.
    fn pick(&mut self, filter: Filter) -> Vec<Effect> {
        if !self.keyboard || self.filter == filter || !Filter::shown(&self.all).contains(&filter) {
            return Vec::new();
        }
        self.filter = filter;
        self.rerender()
    }

    /// 1 to 5 pick a tab; [ and ] step along the tabs on show, stopping at
    /// the ends.
    fn filter_key(&mut self, key: KeyAction) -> Vec<Effect> {
        let to = match key {
            KeyAction::Filter(filter) => filter,
            _ => {
                let shown = Filter::shown(&self.all);
                let at = shown.iter().position(|f| *f == self.filter).unwrap_or(0);
                shown[keys::step(at, shown.len(), key == KeyAction::NextFilter)]
            }
        };
        self.pick(to)
    }

    /// Everything a re-render does to the state. A tab shows only while it
    /// has tasks, so once the last under the picked tab goes, the panel is on
    /// All. Any re-render disarms, a tab switch included. The focus stays on
    /// the same card and slot; on the card's body when it lost that button
    /// (Stop on a task that just stopped); on the first card when the card
    /// left.
    fn rerender(&mut self) -> Vec<Effect> {
        if self.keyboard && !Filter::shown(&self.all).contains(&self.filter) {
            self.filter = Filter::All;
        }
        self.armed = Armed::None;
        self.focus = if self.keyboard {
            let shown = self.visible();
            let kept = self.focus.take().and_then(|f| {
                let card = shown.iter().find(|s| s.card.uuid.as_deref() == Some(f.uuid.as_str()))?;
                let slot = if card.slots().contains(&f.slot) { f.slot } else { Slot::Body };
                Some(Focus { uuid: f.uuid, slot })
            });
            kept.or_else(|| first_task(&shown))
        } else {
            None
        };
        vec![Effect::Render]
    }

    /// Move the focus here, as GTK will once it runs the effect.
    fn focus_on(&mut self, focus: Focus) -> Vec<Effect> {
        self.on_focus(Some(focus.clone()));
        vec![Effect::Focus(Some(focus))]
    }

    /// The index in `shown` of the card the keys act on: the focused one, or
    /// the first when the focus is on none.
    fn current(&self, shown: &[Shown]) -> Option<usize> {
        let focused = self
            .focus
            .as_ref()
            .and_then(|f| shown.iter().position(|s| s.card.uuid.as_deref() == Some(f.uuid.as_str())));
        focused.or_else(|| (!shown.is_empty()).then_some(0))
    }

    /// Up and Down land on a card's body. "+N more" is always last, so only
    /// Down reaches it, and it shows the cards it stands for there and then.
    fn move_card(&mut self, forward: bool) -> Vec<Effect> {
        let shown = self.visible();
        let Some(at) = self.current(&shown) else { return Vec::new() };
        match shown[keys::step(at, shown.len(), forward)].card.uuid.as_deref() {
            Some(uuid) => self.focus_on(Focus::body(uuid)),
            None => self.on_more(),
        }
    }

    /// Left, Right and Tab move along the focused card.
    fn move_slot(&mut self, forward: bool) -> Vec<Effect> {
        let shown = self.visible();
        let Some(card) = self.current(&shown).map(|at| &shown[at]) else { return Vec::new() };
        let Some(uuid) = card.card.uuid.clone() else { return Vec::new() };
        let slots = card.slots();
        let slot = self.focus.as_ref().filter(|f| f.uuid == uuid).map_or(Slot::Body, |f| f.slot);
        let at = slots.iter().position(|s| *s == slot).unwrap_or(0);
        self.focus_on(Focus { uuid, slot: slots[keys::step(at, slots.len(), forward)] })
    }

    /// A letter, or Delete: press that button on the focused card, as a
    /// click does. Nothing when the card has no such button: Stop on a task
    /// that is not active, a letter on "+N more".
    fn run(&mut self, action: Action) -> Vec<Effect> {
        let shown = self.visible();
        let Some(card) = self.current(&shown).map(|at| &shown[at]) else { return Vec::new() };
        match &card.card.uuid {
            Some(uuid) if card.actions.contains(&action) => self.on_press(uuid, Slot::Button(action)),
            _ => Vec::new(),
        }
    }

    /// Ctrl+Enter: the focused card's Refine, or its Start once it is
    /// planned, keeping the keyboard and the focus so the user can go on down
    /// the list. Not through `on_press`, which gives the keyboard back. Like
    /// the letters, only a button the card has.
    fn advance(&self) -> Vec<Effect> {
        let shown = self.visible();
        let Some(card) = self.current(&shown).map(|at| &shown[at]) else { return Vec::new() };
        let Some(uuid) = &card.card.uuid else { return Vec::new() };
        let Some(action) = Action::advance(card.card.status, card.card.planned) else { return Vec::new() };
        if !card.actions.contains(&action) {
            return Vec::new();
        }
        let verb = if action == Action::Refine { "Refining" } else { "Starting" };
        vec![Effect::Notify(format!("{verb}: {}", card.card.text)), Effect::Spawn(action.args(uuid))]
    }

    /// Put Clear all back and the focus where it was before it armed, or on
    /// the first card when that card has gone.
    fn cancel_clear(&mut self) -> Vec<Effect> {
        let Armed::ClearAll { before } = std::mem::take(&mut self.armed) else { return Vec::new() };
        let shown = self.visible();
        let before = before.filter(|f| shown.iter().any(|s| s.card.uuid.as_deref() == Some(f.uuid.as_str())));
        self.focus = before.or_else(|| first_task(&shown));
        vec![Effect::Focus(self.focus.clone())]
    }

    /// The card the focus moves to when this one leaves the list: the next
    /// task's, or, from the last, the one above.
    fn neighbour(&self, uuid: &str) -> Option<Focus> {
        let tasks: Vec<String> = self.visible().into_iter().filter_map(|s| s.card.uuid).collect();
        let at = tasks.iter().position(|u| u == uuid)?;
        let next = tasks.get(at + 1).or_else(|| at.checked_sub(1).and_then(|i| tasks.get(i)));
        next.map(|u| Focus::body(u))
    }
}

/// The first task card's body: where the focus goes with nowhere better.
fn first_task(shown: &[Shown]) -> Option<Focus> {
    shown.iter().find_map(|s| s.card.uuid.as_deref()).map(Focus::body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::model::Status;

    fn card(uuid: &str, status: Status) -> Card {
        Card { status, text: uuid.into(), uuid: Some(uuid.into()), planned: status == Status::Planned, up_next: false }
    }

    fn pending(uuids: &[&str]) -> Vec<Card> {
        uuids.iter().map(|u| card(u, Status::Pending)).collect()
    }

    fn uuids(state: &PanelState) -> Vec<String> {
        state.visible().into_iter().map(|s| s.card.uuid.unwrap_or_else(|| "more".into())).collect()
    }

    /// A panel with these cards and the keyboard, as Mod+Alt+Ctrl+T leaves it.
    fn keyboard(cards: Vec<Card>) -> PanelState {
        let mut state = PanelState::default();
        state.set_cards(&cards);
        assert!(state.take_keyboard(Vec::new()));
        state
    }

    fn key(state: &mut PanelState, key: KeyAction) -> Vec<Effect> {
        state.on_key(key).expect("the panel's key")
    }

    fn focused(uuid: &str, slot: Slot) -> Option<Focus> {
        Some(Focus { uuid: uuid.into(), slot })
    }

    // ─── what shows ──────────────────────────────────────────────────────

    #[test]
    fn the_hover_shows_alls_cards_with_no_tabs_or_buttons() {
        let mut state = PanelState::default();
        assert_eq!(state.set_cards(&[card("a", Status::Pending), card("w", Status::Waiting)]), vec![Effect::Render]);
        assert_eq!(uuids(&state), vec!["a"]);
        assert!(state.tabs().is_empty());
        assert!(state.visible()[0].actions.is_empty());
        assert_eq!(state.focus(), None);
    }

    #[test]
    fn the_same_cards_again_change_nothing() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.set_cards(&pending(&["a"])), Vec::new());
    }

    /// A workspace whose tasks are all waiting shows nothing on its edge, but
    /// the keyboard's panel opens on All saying so, beside the Waiting tab.
    #[test]
    fn only_waiting_tasks_hide_the_hover_but_not_the_keyboards_panel() {
        let mut state = PanelState::default();
        state.set_cards(&[card("w", Status::Waiting)]);
        assert!(state.hidden());
        assert!(state.take_keyboard(Vec::new()));
        assert!(!state.hidden());
        assert!(state.visible().is_empty());
        assert_eq!(state.empty_text(), Some("No tasks"));
        assert_eq!(state.tabs(), vec![Filter::All, Filter::Waiting]);
        assert_eq!(state.focus(), None);
    }

    #[test]
    fn no_cards_no_keyboard() {
        assert!(!PanelState::default().take_keyboard(Vec::new()));
    }

    #[test]
    fn a_tab_shows_only_while_it_has_tasks() {
        let state = keyboard(vec![card("a", Status::Active), card("p", Status::Pending)]);
        assert_eq!(state.tabs(), vec![Filter::All, Filter::Active, Filter::ToRefine]);
    }

    /// Filtered before the cap, so "+N more" is the rest of this tab.
    #[test]
    fn the_tab_filters_before_the_cap() {
        let mut cards = pending(&["p"]);
        cards.extend((0..10).map(|i| card(&format!("w{i}"), Status::Waiting)));
        let mut state = keyboard(cards);
        assert_eq!(uuids(&state), vec!["p"]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        let shown = uuids(&state);
        assert_eq!(shown.len(), model::CAP + 1);
        assert_eq!(shown.last().unwrap(), "more");
        assert_eq!(state.visible().last().unwrap().card.text, "+2 more");
    }

    #[test]
    fn only_a_task_card_on_the_keyboard_gets_buttons() {
        let names: Vec<String> = (0..10).map(|i| format!("t{i}")).collect();
        let state = keyboard(pending(&names.iter().map(String::as_str).collect::<Vec<_>>()));
        let shown = state.visible();
        assert!(shown[0].actions.contains(&Action::Start));
        assert!(!shown[0].actions.contains(&Action::Session), "no agent, no Go to session");
        assert!(shown.last().unwrap().actions.is_empty(), "\"+N more\" has none");
    }

    #[test]
    fn go_to_session_only_on_a_card_with_a_live_agent() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b"]));
        state.take_keyboard(vec![crate::work::work_agent_name("b")]);
        let shown = state.visible();
        assert!(!shown[0].actions.contains(&Action::Session));
        assert_eq!(shown[1].actions[0], Action::Session);
    }

    #[test]
    fn the_keyboard_opens_on_all_every_time() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        assert_eq!(state.filter(), Filter::Waiting);
        assert!(state.take_keyboard(Vec::new()), "taken again while open");
        assert_eq!(state.filter(), Filter::All);
        assert_eq!(state.focus(), focused("p", Slot::Body).as_ref());
    }

    #[test]
    fn a_hidden_tabs_key_does_nothing() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::Planned)), Vec::new());
        assert_eq!(state.filter(), Filter::All);
    }

    #[test]
    fn the_brackets_skip_hidden_tabs_and_stop_at_the_ends() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert_eq!(key(&mut state, KeyAction::NextFilter), vec![Effect::Render]);
        assert_eq!(state.filter(), Filter::ToRefine, "past the hidden Active and Planned");
        assert_eq!(key(&mut state, KeyAction::NextFilter), Vec::new(), "To refine is the last shown");
        key(&mut state, KeyAction::PrevFilter);
        assert_eq!(state.filter(), Filter::All);
        assert_eq!(key(&mut state, KeyAction::PrevFilter), Vec::new());
    }

    #[test]
    fn the_tab_falls_back_to_all_when_it_empties() {
        let mut state = keyboard(vec![card("a", Status::Active), card("p", Status::Pending)]);
        key(&mut state, KeyAction::Filter(Filter::Active));
        state.set_cards(&pending(&["a", "p"]));
        assert_eq!(state.filter(), Filter::All);
        assert_eq!(state.tabs(), vec![Filter::All, Filter::ToRefine]);
    }

    #[test]
    fn the_last_task_going_gives_the_keyboard_back() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.set_cards(&[]), vec![Effect::Render, Effect::Release]);
        assert!(!state.keyboard());
        assert!(state.hidden());
    }

    /// Escape: one line per card again, the hover's All, on the right edge.
    #[test]
    fn escape_gives_the_keyboard_back_on_all_folded_up() {
        let mut cards = pending(&["p"]);
        cards.extend((0..10).map(|i| card(&format!("w{i}"), Status::Waiting)));
        let mut state = keyboard(cards);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        state.on_more();
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
        assert!(!state.keyboard());
        assert_eq!(state.filter(), Filter::All);
        assert_eq!(state.focus(), None);
        assert_eq!(uuids(&state), vec!["p"]);
        assert_eq!(state.on_key(KeyAction::Release), None, "already given back, not the panel's key");
    }

    #[test]
    fn tucking_away_folds_an_expanded_list() {
        let names: Vec<String> = (0..10).map(|i| format!("t{i}")).collect();
        let mut state = PanelState::default();
        state.set_cards(&pending(&names.iter().map(String::as_str).collect::<Vec<_>>()));
        assert_eq!(state.on_more(), vec![Effect::Render]);
        assert_eq!(state.visible().len(), 10);
        assert_eq!(state.focus(), None, "the hover has no focus");
        assert_eq!(state.on_tuck(), vec![Effect::Render]);
        assert_eq!(state.visible().len(), model::CAP + 1);
        assert_eq!(state.on_tuck(), Vec::new());
    }

    // ─── the focus ───────────────────────────────────────────────────────

    #[test]
    fn the_keyboard_lands_on_the_first_card() {
        let state = keyboard(pending(&["a", "b"]));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn up_and_down_move_between_bodies_and_stop_at_the_ends() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::Focus(focused("b", Slot::Body))]);
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::Focus(focused("b", Slot::Body))]);
        key(&mut state, KeyAction::PrevCard);
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn down_onto_more_shows_the_rest_and_lands_on_the_first_it_hid() {
        let names: Vec<String> = (0..10).map(|i| format!("t{i}")).collect();
        let mut state = keyboard(pending(&names.iter().map(String::as_str).collect::<Vec<_>>()));
        for _ in 0..model::CAP - 1 {
            key(&mut state, KeyAction::NextCard);
        }
        assert_eq!(state.focus(), focused("t7", Slot::Body).as_ref());
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::Render]);
        assert_eq!(uuids(&state).len(), 10, "no \"+N more\" once expanded");
        assert_eq!(state.focus(), focused("t8", Slot::Body).as_ref());
        key(&mut state, KeyAction::NextCard);
        assert_eq!(state.focus(), focused("t9", Slot::Body).as_ref());
    }

    #[test]
    fn left_and_right_walk_the_card_and_stop_at_the_ends() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(key(&mut state, KeyAction::PrevSlot), vec![Effect::Focus(focused("a", Slot::Body))]);
        key(&mut state, KeyAction::NextSlot);
        let first = state.visible()[0].actions[0];
        assert_eq!(state.focus(), focused("a", Slot::Button(first)).as_ref());
        for _ in 0..20 {
            key(&mut state, KeyAction::NextSlot);
        }
        assert_eq!(state.focus(), focused("a", Slot::Button(Action::Remove)).as_ref());
    }

    #[test]
    fn a_refresh_keeps_the_focus_on_the_same_card_and_button() {
        let mut state = keyboard(pending(&["a", "b"]));
        state.on_focus(focused("b", Slot::Button(Action::Edit)));
        state.set_cards(&pending(&["new", "a", "b"]));
        assert_eq!(state.focus(), focused("b", Slot::Button(Action::Edit)).as_ref());
    }

    /// Stop on a task that just stopped has gone: the body, not another card.
    #[test]
    fn a_refresh_that_drops_the_button_lands_on_the_body() {
        let mut state = keyboard(vec![card("a", Status::Active)]);
        state.on_focus(focused("a", Slot::Button(Action::Stop)));
        state.set_cards(&pending(&["a"]));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn a_refresh_that_drops_the_card_lands_on_the_first() {
        let mut state = keyboard(pending(&["a", "b"]));
        state.on_focus(focused("b", Slot::Button(Action::Edit)));
        state.set_cards(&pending(&["a"]));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn a_letter_presses_only_a_button_the_card_has() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(key(&mut state, KeyAction::Run(Action::Stop)), Vec::new(), "not active, no Stop");
        assert_eq!(
            key(&mut state, KeyAction::Run(Action::Edit)),
            vec![Effect::Render, Effect::Release, Effect::Spawn(Action::Edit.args("a"))],
        );
    }

    #[test]
    fn enter_and_space_are_gtks_while_nothing_is_armed() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.on_key(KeyAction::Enter), None);
        assert_eq!(state.on_key(KeyAction::Ignore), None);
    }

    // ─── presses ─────────────────────────────────────────────────────────

    #[test]
    fn the_body_gives_the_keyboard_back_and_opens_the_menu() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(
            state.on_press("a", Slot::Body),
            vec![Effect::Render, Effect::Release, Effect::Spawn(vec!["task".into(), "menu".into(), "a".into()])],
        );
        assert!(!state.keyboard());
    }

    #[test]
    fn speak_and_up_next_keep_the_keyboard_and_the_focus() {
        let mut state = keyboard(pending(&["a"]));
        state.on_focus(focused("a", Slot::Button(Action::Speak)));
        for action in [Action::Speak, Action::UpNext] {
            assert_eq!(state.on_press("a", Slot::Button(action)), vec![Effect::Spawn(action.args("a"))]);
        }
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Button(Action::Speak)).as_ref());
    }

    #[test]
    fn ctrl_enter_refines_or_starts_keeping_the_keyboard_and_the_focus() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("q", Status::Planned)]);
        assert_eq!(
            key(&mut state, KeyAction::Advance),
            vec![Effect::Notify("Refining: p".into()), Effect::Spawn(Action::Refine.args("p"))],
        );
        key(&mut state, KeyAction::NextCard);
        assert_eq!(
            key(&mut state, KeyAction::Advance),
            vec![Effect::Notify("Starting: q".into()), Effect::Spawn(Action::Start.args("q"))],
        );
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("q", Slot::Body).as_ref());
    }

    #[test]
    fn ctrl_enter_does_nothing_on_a_planned_task_being_worked() {
        let mut planned_active = card("a", Status::Active);
        planned_active.planned = true;
        let mut state = keyboard(vec![planned_active]);
        assert_eq!(key(&mut state, KeyAction::Advance), Vec::new());
    }

    // ─── arming ──────────────────────────────────────────────────────────

    #[test]
    fn the_first_remove_arms_and_the_second_deletes() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert_eq!(
            key(&mut state, KeyAction::Run(Action::Remove)),
            vec![Effect::Focus(focused("a", Slot::Button(Action::Remove)))],
        );
        assert_eq!(state.armed(), &Armed::Remove("a".into()));
        let effects = key(&mut state, KeyAction::Run(Action::Remove));
        assert_eq!(effects, vec![Effect::Focus(focused("b", Slot::Body)), Effect::Spawn(Action::Remove.args("a"))]);
        assert_eq!(state.armed(), &Armed::None);
    }

    #[test]
    fn leaving_the_list_from_the_last_card_focuses_the_one_above() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::NextCard);
        assert_eq!(
            state.on_press("b", Slot::Button(Action::Wait)),
            vec![Effect::Focus(focused("a", Slot::Body)), Effect::Spawn(Action::Wait.args("b"))],
        );
        assert!(state.keyboard());
    }

    #[test]
    fn moving_off_an_armed_remove_disarms_it() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Run(Action::Remove));
        state.on_focus(focused("a", Slot::Button(Action::Remove)));
        assert_eq!(state.armed(), &Armed::Remove("a".into()), "its own focus coming back");
        key(&mut state, KeyAction::NextCard);
        assert_eq!(state.armed(), &Armed::None);
    }

    #[test]
    fn a_refresh_disarms_remove() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Run(Action::Remove));
        state.set_cards(&pending(&["a", "b", "c"]));
        assert_eq!(state.armed(), &Armed::None);
        assert_eq!(state.focus(), focused("a", Slot::Button(Action::Remove)).as_ref());
    }

    fn waiting_tab() -> PanelState {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w1", Status::Waiting), card("w2", Status::Waiting)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        state
    }

    #[test]
    fn clear_all_is_only_on_the_waiting_tab() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        assert!(!state.shows_clear_all());
        assert_eq!(key(&mut state, KeyAction::ClearAll), Vec::new());
        assert_eq!(state.armed(), &Armed::None);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        assert!(state.shows_clear_all());
    }

    #[test]
    fn the_first_clear_all_arms_it_and_takes_the_focus_off_the_cards() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::Run(Action::Remove));
        assert_eq!(key(&mut state, KeyAction::ClearAll), vec![Effect::Focus(None)]);
        assert_eq!(
            state.armed(),
            &Armed::ClearAll { before: focused("w1", Slot::Button(Action::Remove)) },
            "and Remove is not armed beside it",
        );
        assert_eq!(state.focus(), None);
        state.on_focus(None);
        assert!(matches!(state.armed(), Armed::ClearAll { .. }), "its own focus move coming back");
    }

    #[test]
    fn enter_confirms_clear_all_deleting_every_waiting_task_onto_all() {
        let mut state = keyboard({
            let mut cards = pending(&["p"]);
            cards.extend((0..10).map(|i| card(&format!("w{i}"), Status::Waiting)));
            cards
        });
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        key(&mut state, KeyAction::ClearAll);
        let every: Vec<String> = (0..10).map(|i| format!("w{i}")).collect();
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::DeleteAll(every)]);
        assert!(state.keyboard());
        assert_eq!(state.filter(), Filter::All);
        assert_eq!(state.armed(), &Armed::None);
        assert_eq!(state.focus(), focused("p", Slot::Body).as_ref());
    }

    #[test]
    fn escape_and_moving_cancel_clear_all_putting_the_focus_back() {
        for cancel in [KeyAction::Release, KeyAction::PrevCard, KeyAction::NextCard, KeyAction::PrevSlot, KeyAction::NextSlot] {
            let mut state = waiting_tab();
            key(&mut state, KeyAction::NextCard);
            key(&mut state, KeyAction::ClearAll);
            assert_eq!(key(&mut state, cancel), vec![Effect::Focus(focused("w2", Slot::Body))], "{cancel:?}");
            assert_eq!(state.armed(), &Armed::None);
            assert!(state.keyboard(), "Escape cancels without giving the keyboard back");
        }
    }

    #[test]
    fn a_tab_key_switches_tab_and_disarms_clear_all() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::All)), vec![Effect::Render]);
        assert_eq!(state.armed(), &Armed::None);
        assert_eq!(state.focus(), focused("p", Slot::Body).as_ref());
    }

    #[test]
    fn a_cards_keys_do_nothing_while_clear_all_is_armed() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        for k in [KeyAction::Run(Action::Edit), KeyAction::Run(Action::Remove), KeyAction::Advance, KeyAction::Ignore] {
            assert_eq!(state.on_key(k), Some(Vec::new()), "{k:?}");
        }
        assert!(matches!(state.armed(), Armed::ClearAll { .. }));
    }

    /// A click on Remove moves the focus onto it, which puts an armed Clear
    /// all back: one arming at a time.
    #[test]
    fn clicking_remove_disarms_clear_all() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        state.on_focus(focused("w2", Slot::Button(Action::Remove)));
        state.on_press("w2", Slot::Button(Action::Remove));
        assert_eq!(state.armed(), &Armed::Remove("w2".into()));
    }

    #[test]
    fn a_refresh_disarms_clear_all() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        state.set_cards(&[card("p", Status::Pending), card("w1", Status::Waiting)]);
        assert_eq!(state.armed(), &Armed::None);
        assert_eq!(state.focus(), focused("w1", Slot::Body).as_ref());
    }

    #[test]
    fn leaving_the_list_above_more_focuses_the_one_above() {
        let names: Vec<String> = (0..model::CAP + 2).map(|i| format!("t{i}")).collect();
        let mut state = keyboard(pending(&names.iter().map(String::as_str).collect::<Vec<_>>()));
        for _ in 0..model::CAP - 1 {
            key(&mut state, KeyAction::NextCard);
        }
        let last = format!("t{}", model::CAP - 1);
        let above = format!("t{}", model::CAP - 2);
        assert_eq!(state.focus(), focused(&last, Slot::Body).as_ref());
        assert_eq!(
            state.on_press(&last, Slot::Button(Action::Wait)),
            vec![Effect::Focus(focused(&above, Slot::Body)), Effect::Spawn(Action::Wait.args(&last))],
            "the next task card, not \"+N more\""
        );
    }

    #[test]
    fn focus_moves_without_the_keyboard_are_ignored() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b"]));
        state.on_focus(focused("a", Slot::Body));
        assert_eq!(state.focus(), None);
    }

    #[test]
    fn keys_without_the_keyboard_are_not_the_panels() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b"]));
        assert_eq!(state.on_key(KeyAction::NextCard), None);
    }
}
