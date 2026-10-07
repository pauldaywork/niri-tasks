//! What the task panel is showing and what a key or a press does to it, as
//! plain data.
//!
//! The cards, the filter tab, "+N more" opened or not, which card and button
//! has the keyboard's focus, what a first press has armed, and the project
//! list Move to workspace and Mod+Alt+W swap in all live here, keyed by task uuid and
//! slot rather than by widget. Every change comes back
//! as a list of [`Effect`]s for `surface.rs` to run: draw again, move the
//! focus, spawn a command. So the rules between them (one arming at a time,
//! any re-render disarming, where the focus lands) are tested without a
//! window, and `surface.rs` only draws what this says and runs what it asks.

use crate::actions::Action;
use super::keys::{self, KeyAction};
use super::model::{self, Card, Filter, Tab};
use super::projects::{Escaped, Matcher, Picked, ProjectList, Purpose, NO_DESTINATIONS};
use crate::project::{Projects, Row};

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
    /// Save what was typed on Ideas now, the panel having left it.
    SaveIdeas,
    /// Read the `~/Projects` folders this task can move to and hand them to
    /// [`PanelState::show_projects`]: Move to workspace's first step, which
    /// the state cannot take, having no disk to read.
    ListProjects(String),
    /// Scroll the project list's highlighted folder into view.
    ShowFolder,
    /// Empty the project list's text field: Escape with text typed.
    ClearQuery,
}

#[derive(Debug, Default)]
pub struct PanelState {
    /// Every card, uncapped and unfiltered.
    all: Vec<Card>,
    /// The panel has the keyboard, from Mod+Alt+Ctrl+T.
    keyboard: bool,
    /// The filter tab picked, or, on Ideas, the one picked before it. All
    /// whenever the panel takes the keyboard; kept across a refresh while it
    /// has it.
    filter: Filter,
    /// The Ideas tab is picked: the notepad in place of the cards. Only ever
    /// with the keyboard, and off whenever the panel takes it.
    ideas: bool,
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
    /// The project list is up in place of the cards: Move to workspace was
    /// pressed. Only ever with the keyboard.
    projects: Option<ProjectList>,
    /// The folders and repos the project list was built from, for its hint
    /// and its pick to mean what `project open` will. Some only while the
    /// list is up.
    loaded: Option<Projects>,
}

impl PanelState {
    pub fn keyboard(&self) -> bool {
        self.keyboard
    }

    pub fn filter(&self) -> Filter {
        self.filter
    }

    /// The tab picked: a filter tab, or Ideas.
    pub fn tab(&self) -> Tab {
        if self.ideas {
            Tab::Ideas
        } else {
            Tab::Filter(self.filter)
        }
    }

    /// On the Ideas tab, whose text area takes every key but Escape and the
    /// tab keys.
    pub fn on_ideas(&self) -> bool {
        self.ideas
    }

    pub fn focus(&self) -> Option<&Focus> {
        self.focus.as_ref()
    }

    pub fn armed(&self) -> &Armed {
        &self.armed
    }

    pub fn projects(&self) -> Option<&ProjectList> {
        self.projects.as_ref()
    }

    /// Nothing to show, so the panel hides. The hover and the peek show no
    /// waiting or finished task, so they hide with nothing else; the
    /// keyboard's panel still has the Waiting and Finished tabs, and hides
    /// only with no task at all. On
    /// Ideas it never hides: the notepad is there with no task too. Nor on
    /// the project list, which Mod+Alt+W opens with no task.
    pub fn hidden(&self) -> bool {
        if self.keyboard {
            self.all.is_empty() && !self.ideas && self.projects.is_none()
        } else {
            Filter::All.pick(&self.all).is_empty()
        }
    }

    /// The filter tabs on show, in `Filter::TABS` order: All, and every
    /// other tab with a task under it. None without the keyboard. Ideas,
    /// always shown, follows them. None on the project list either.
    pub fn tabs(&self) -> Vec<Filter> {
        if self.keyboard && self.projects.is_none() {
            Filter::shown(&self.all)
        } else {
            Vec::new()
        }
    }

    /// Clear all shows, on its strip under the tab bar, on the Waiting tab alone.
    pub fn shows_clear_all(&self) -> bool {
        self.keyboard && !self.ideas && self.projects.is_none() && self.filter == Filter::Waiting
    }

    /// The cards to draw, top to bottom: the picked tab's, or All's off the
    /// keyboard, filtered before the cap so "+N more" is the rest of this
    /// tab, and uncapped once expanded. None on Ideas, which is a notepad, not
    /// cards, nor on the project list, which draws folders.
    pub fn visible(&self) -> Vec<Shown> {
        if self.ideas || self.projects.is_some() {
            return Vec::new();
        }
        let picked = self.shown_filter().pick(&self.all);
        let cards = if self.expanded { picked } else { model::cap(&picked, model::CAP) };
        cards
            .into_iter()
            .map(|card| {
                let actions = match card.uuid.as_deref().filter(|_| self.keyboard) {
                    Some(uuid) => {
                        let has_session = crate::link::session_agent(&self.agents, uuid).is_some();
                        card.state(has_session).map(Action::row).unwrap_or_default()
                    }
                    None => Vec::new(),
                };
                Shown { card, actions }
            })
            .collect()
    }

    /// The line shown in place of cards when the tab has none: only All, on
    /// a workspace whose tasks are all waiting or finished. Never on Ideas, which has no
    /// cards to have none of.
    pub fn empty_text(&self) -> Option<&'static str> {
        if self.ideas || self.projects.is_some() {
            return None;
        }
        let filter = self.shown_filter();
        filter.pick(&self.all).is_empty().then(|| filter.empty_text())
    }

    /// The daemon's cards, on each tick. Nothing when they have not changed;
    /// otherwise a re-render, which disarms and keeps the focus where it can.
    /// The last task going while the panel has the keyboard gives it back, so
    /// the next card shows as a peek and the next Mod+Alt+Ctrl+T opens on All.
    /// Not on Ideas, where it could be mid-typing: giving the keyboard back
    /// there would send the next keystrokes to another window. Nor on the
    /// project list, which needs no task.
    pub fn set_cards(&mut self, cards: &[Card]) -> Vec<Effect> {
        if self.all == cards {
            return Vec::new();
        }
        self.all = cards.to_vec();
        // The task being moved has gone (done elsewhere, say): its project
        // list goes with it.
        let gone = self
            .projects
            .as_ref()
            .and_then(ProjectList::moving)
            .is_some_and(|uuid| !self.all.iter().any(|c| c.uuid.as_deref() == Some(uuid)));
        if gone {
            self.projects = None;
            self.loaded = None;
        }
        if self.keyboard && self.all.is_empty() && !self.ideas && self.projects.is_none() {
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
        self.ideas = false;
        self.projects = None;
        self.loaded = None;
        self.armed = Armed::None;
        self.focus = first_task(&self.visible());
        true
    }

    /// Mod+Alt+W: take the keyboard on the project list for opening a
    /// project, `projects` being the `~/Projects` folders and the GitHub
    /// repos not cloned yet. With no cards too, and on an unnamed workspace.
    /// It takes the panel over as `take_keyboard` does: All, nothing
    /// expanded, armed or focused.
    pub fn open_projects(&mut self, projects: Projects) -> Vec<Effect> {
        self.keyboard = true;
        self.filter = Filter::All;
        self.ideas = false;
        self.expanded = false;
        self.armed = Armed::None;
        self.focus = None;
        self.projects = Some(ProjectList::new(Purpose::Open, projects.rows()));
        self.loaded = Some(projects);
        vec![Effect::Render]
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

    /// A click on a tab: a filter tab, or Ideas.
    pub fn on_tab(&mut self, tab: Tab) -> Vec<Effect> {
        self.pick(tab)
    }

    /// GTK moved the focus: a click on a button, or the focus an
    /// [`Effect::Focus`] asked for, which is no change. Moving off an armed
    /// Remove disarms it, and moving at all disarms Clear all. Without the
    /// keyboard it is ignored: GTK moves focus in a window that is only being
    /// shown, and the hover has none. On the project list too: a folder card
    /// is no task's, and the task's focus is kept for Escape.
    pub fn on_focus(&mut self, focus: Option<Focus>) {
        if !self.keyboard || self.projects.is_some() || self.focus == focus {
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
    /// landing after it was given back is not for it. On Ideas only Escape,
    /// which goes back to the tab picked before it, and the tab keys are the
    /// panel's; the rest are typing, for the text area. On the project list
    /// Up and Down, Enter and Escape are the panel's, and the rest are
    /// typing, for its text field.
    pub fn on_key(&mut self, key: KeyAction) -> Option<Vec<Effect>> {
        if !self.keyboard {
            return None;
        }
        if self.ideas {
            // The text area's: every key is typing but these.
            return match key {
                KeyAction::Release => Some(self.leave_ideas()),
                KeyAction::Filter(_) | KeyAction::Ideas | KeyAction::PrevFilter | KeyAction::NextFilter => {
                    Some(self.filter_key(key))
                }
                _ => None,
            };
        }
        if self.projects.is_some() {
            // The project list's: Up and Down walk it, Enter picks the folder,
            // Escape clears the text, then goes back to the cards. Every
            // other key is typing, for the text field.
            return match key {
                KeyAction::Release => Some(self.escape_projects()),
                KeyAction::PrevCard | KeyAction::NextCard => Some(self.move_folder(key == KeyAction::NextCard)),
                KeyAction::Enter => {
                    let at = self.projects.as_ref().map_or(0, ProjectList::at);
                    Some(self.on_folder(at))
                }
                _ => None,
            };
        }
        if matches!(self.armed, Armed::ClearAll { .. }) {
            return Some(match key {
                KeyAction::Enter | KeyAction::ClearAll => self.on_clear_all(),
                KeyAction::Release
                | KeyAction::PrevCard
                | KeyAction::NextCard
                | KeyAction::PrevSlot
                | KeyAction::NextSlot => self.cancel_clear(),
                KeyAction::Filter(_) | KeyAction::Ideas | KeyAction::PrevFilter | KeyAction::NextFilter => {
                    self.filter_key(key)
                }
                KeyAction::Run(_) | KeyAction::Advance | KeyAction::Delete | KeyAction::Ignore => Vec::new(),
            });
        }
        Some(match key {
            KeyAction::Enter | KeyAction::Ignore => return None,
            KeyAction::Release => self.release(),
            KeyAction::ClearAll => self.on_clear_all(),
            KeyAction::Filter(_) | KeyAction::Ideas | KeyAction::PrevFilter | KeyAction::NextFilter => {
                self.filter_key(key)
            }
            KeyAction::PrevCard | KeyAction::NextCard => self.move_card(key == KeyAction::NextCard),
            KeyAction::PrevSlot | KeyAction::NextSlot => self.move_slot(key == KeyAction::NextSlot),
            KeyAction::Run(action) => self.run(action),
            KeyAction::Advance => self.advance(),
            KeyAction::Delete => self.delete(),
        })
    }

    /// A press on a task's card: a button, as a click or a key presses it.
    /// The body does nothing: the buttons are the card's actions. Remove
    /// only arms itself the first time; the second press runs it.
    /// Everything that opens something gives the keyboard back first, so the
    /// box or terminal it opens can take it. Back, Waiting and Remove take the card off the list, so the
    /// focus moves to the next card (the one above, from the last) to still be
    /// there when the next tick drops this one. Speak and Up next keep the
    /// card and the focus, so a second press undoes them. Move to workspace
    /// disarms and asks for the project list.
    pub fn on_press(&mut self, uuid: &str, slot: Slot) -> Vec<Effect> {
        if self.projects.is_some() {
            return Vec::new();
        }
        let Slot::Button(action) = slot else { return Vec::new() };
        // The folders are on disk, which the surface reads: it answers
        // with show_projects. The keyboard and the focus stay put.
        if action == Action::Move {
            self.armed = Armed::None;
            return vec![Effect::ListProjects(uuid.into())];
        }
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

    /// Clear all, by its button or Ctrl+Shift+Delete. The first press arms it and
    /// takes the focus off the cards, so Enter confirms rather than pressing a
    /// card's button. The second deletes every task the Waiting tab lists,
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
        let mut effects = self.pick(Tab::Filter(Filter::All));
        if !uuids.is_empty() {
            effects.push(Effect::DeleteAll(uuids));
        }
        effects
    }

    /// The folders Move to workspace asked for, `rows`, out of `projects`:
    /// the project list in place of the cards, the keyboard on the first.
    /// With none, a notification and the cards stay. Nothing without the
    /// keyboard, or once the task has left the cards.
    pub fn show_projects(&mut self, uuid: &str, rows: Vec<Row>, projects: Projects) -> Vec<Effect> {
        if !self.keyboard {
            return Vec::new();
        }
        let Some(text) = self.all.iter().find(|c| c.uuid.as_deref() == Some(uuid)).map(|c| c.text.clone()) else {
            return Vec::new();
        };
        if rows.is_empty() {
            return vec![Effect::Notify(NO_DESTINATIONS.into())];
        }
        self.armed = Armed::None;
        self.projects = Some(ProjectList::new(Purpose::Move { uuid: uuid.into(), text }, rows));
        self.loaded = Some(projects);
        vec![Effect::Render]
    }

    /// The project list's text field changed: narrow the list to `query`,
    /// ranked by `matcher`. Nothing when the text is what it was, as when
    /// Escape empties the field.
    pub fn on_query(&mut self, query: &str, matcher: Matcher) -> Vec<Effect> {
        if self.projects.as_mut().is_some_and(|list| list.narrow(query, matcher)) {
            vec![Effect::Render]
        } else {
            Vec::new()
        }
    }

    /// The project list's line under its title, when it has one: what Enter
    /// will do with the text typed.
    pub fn hint(&self) -> Option<String> {
        self.projects.as_ref()?.hint(self.loaded.as_ref()?)
    }

    /// A folder on the project list, by Enter or a click. Moving, `task
    /// move` takes the task there, back on the cards with the focus on the
    /// next one, since the task leaves this workspace; the keyboard stays.
    /// Opening, `project open` opens it, or makes a folder of a new name
    /// with nothing matching, and the keyboard goes back first: the user is
    /// off to that workspace. Nothing with no row and no new name.
    pub fn on_folder(&mut self, at: usize) -> Vec<Effect> {
        let (Some(list), Some(loaded)) = (self.projects.as_ref(), self.loaded.as_ref()) else {
            return Vec::new();
        };
        let Picked::Spawn(args) = list.pick(at, loaded) else { return Vec::new() };
        let purpose = list.purpose().clone();
        self.projects = None;
        self.loaded = None;
        match purpose {
            Purpose::Open => {
                let mut effects = self.release();
                effects.push(Effect::Spawn(args));
                effects
            }
            Purpose::Move { uuid, .. } => {
                if let Some(next) = self.neighbour(&uuid) {
                    self.focus = Some(next);
                }
                let mut effects = self.rerender();
                effects.push(Effect::Spawn(args));
                effects
            }
        }
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
        self.ideas = false;
        self.projects = None;
        self.loaded = None;
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

    /// Show this tab: a filter tab's cards, or Ideas. Nothing without the
    /// keyboard, for a filter tab hidden for having no tasks, or for the tab
    /// already picked. Leaving Ideas for a filter tab once the last task has
    /// gone gives the keyboard back, as the last task going does off Ideas:
    /// with no cards the panel hides, and must not hide holding the keyboard.
    /// Leaving Ideas saves what was typed there at once.
    fn pick(&mut self, tab: Tab) -> Vec<Effect> {
        if !self.keyboard || self.projects.is_some() || self.tab() == tab {
            return Vec::new();
        }
        if matches!(tab, Tab::Filter(_)) && self.all.is_empty() {
            return self.release();
        }
        let leaving_ideas = self.ideas;
        match tab {
            Tab::Filter(filter) if !Filter::shown(&self.all).contains(&filter) => return Vec::new(),
            Tab::Filter(filter) => {
                self.filter = filter;
                self.ideas = false;
            }
            Tab::Ideas => self.ideas = true,
        }
        let mut effects = self.rerender();
        if leaving_ideas {
            effects.push(Effect::SaveIdeas);
        }
        effects
    }

    /// Escape on Ideas: back to the task list, on the filter tab picked
    /// before Ideas, or All once that tab has nothing left under it. The
    /// panel keeps the keyboard; a second Escape gives it back.
    fn leave_ideas(&mut self) -> Vec<Effect> {
        let back = if Filter::shown(&self.all).contains(&self.filter) { self.filter } else { Filter::All };
        self.pick(Tab::Filter(back))
    }

    /// Escape on the project list: with text typed, clear it and show every
    /// folder again; with none, back to the cards, on the card and slot the
    /// focus was on; or, on the open list, the keyboard given back.
    fn escape_projects(&mut self) -> Vec<Effect> {
        let Some(list) = self.projects.as_mut() else { return Vec::new() };
        match list.escape() {
            Escaped::Cleared => vec![Effect::ClearQuery, Effect::Render],
            Escaped::Closed => {
                // The open list was asked for from anywhere, not from the
                // cards, which there may be none of: it closes the panel.
                if list.purpose() == &Purpose::Open {
                    return self.release();
                }
                self.projects = None;
                self.loaded = None;
                self.rerender()
            }
        }
    }

    /// Up and Down on the project list, stopping at the ends. Nothing when no
    /// folder matches.
    fn move_folder(&mut self, forward: bool) -> Vec<Effect> {
        if self.projects.as_mut().is_some_and(|list| list.step(forward)) {
            vec![Effect::ShowFolder]
        } else {
            Vec::new()
        }
    }

    /// 1 to 6 pick a filter tab and 7 Ideas; [ and ] step along the tabs on
    /// show, Ideas the last, stopping at the ends.
    fn filter_key(&mut self, key: KeyAction) -> Vec<Effect> {
        let to = match key {
            KeyAction::Filter(filter) => Tab::Filter(filter),
            KeyAction::Ideas => Tab::Ideas,
            _ => {
                let shown = Tab::shown(&self.all);
                let at = shown.iter().position(|t| *t == self.tab()).unwrap_or(0);
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
    /// left. On Ideas no card has the focus: the text area has it.
    fn rerender(&mut self) -> Vec<Effect> {
        if self.keyboard && !Filter::shown(&self.all).contains(&self.filter) {
            self.filter = Filter::All;
        }
        self.armed = Armed::None;
        // The project list keeps the task's focus for Escape to go back to.
        if self.projects.is_some() {
            return vec![Effect::Render];
        }
        self.focus = if self.keyboard && !self.ideas {
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

    /// Ctrl+Delete: the focused card's Remove as its second press, from
    /// whichever slot has the focus — deleted at once with no Confirm
    /// remove, the focus on to the neighbour and the keyboard kept. Nothing
    /// with no card focused, or on one without Remove.
    fn delete(&mut self) -> Vec<Effect> {
        let shown = self.visible();
        let Some(card) = self.current(&shown).map(|at| &shown[at]) else { return Vec::new() };
        let Some(uuid) = card.card.uuid.clone().filter(|_| card.actions.contains(&Action::Remove)) else {
            return Vec::new();
        };
        // Armed first, so on_press takes this as the second press. Moving to
        // the neighbour is what disarms a second Delete; the only card has
        // none, so it is disarmed here.
        self.armed = Armed::Remove(uuid.clone());
        let effects = self.on_press(&uuid, Slot::Button(Action::Remove));
        self.armed = Armed::None;
        effects
    }

    /// Ctrl+Enter: the focused card's Refine, or its Start once it is
    /// planned, keeping the keyboard and the focus so the user can go on down
    /// the list. Not through `on_press`, which gives the keyboard back. Like
    /// the letters, only a button the card has.
    fn advance(&self) -> Vec<Effect> {
        let shown = self.visible();
        let Some(card) = self.current(&shown).map(|at| &shown[at]) else { return Vec::new() };
        let Some(uuid) = &card.card.uuid else { return Vec::new() };
        let Some(action) = card.card.state(false).and_then(Action::advance) else { return Vec::new() };
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
        Card { status, text: uuid.into(), uuid: Some(uuid.into()), planned: status == Status::Planned, up_next: false, since: String::new() }
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
        assert_eq!(key(&mut state, KeyAction::NextFilter), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Ideas, "after the last filter tab shown");
        assert_eq!(key(&mut state, KeyAction::NextFilter), Vec::new(), "Ideas is the last");
        key(&mut state, KeyAction::PrevFilter);
        key(&mut state, KeyAction::PrevFilter);
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
        assert_eq!(key(&mut state, KeyAction::PrevFilter), Vec::new());
    }

    // ─── the Finished tab ────────────────────────────────────────────────

    /// Finished tasks are off the hover and the peek, as waiting ones are,
    /// and a workspace with only finished tasks shows nothing on its edge.
    #[test]
    fn finished_tasks_are_off_the_hover() {
        let mut state = PanelState::default();
        state.set_cards(&[card("a", Status::Pending), card("f", Status::Finished)]);
        assert_eq!(uuids(&state), vec!["a"]);
        state.set_cards(&[card("f", Status::Finished)]);
        assert!(state.hidden());
    }

    /// Only finished tasks: the keyboard's panel opens on All saying it has
    /// none, beside the Finished tab, as with only waiting tasks.
    #[test]
    fn only_finished_tasks_open_on_all_beside_the_finished_tab() {
        let state = keyboard(vec![card("f", Status::Finished)]);
        assert!(!state.hidden());
        assert_eq!(state.empty_text(), Some("No tasks"));
        assert_eq!(state.tabs(), vec![Filter::All, Filter::Finished]);
        assert_eq!(state.focus(), None);
    }

    /// After Waiting, and only while it has a task.
    #[test]
    fn the_finished_tab_shows_after_waiting_while_it_has_a_task() {
        let state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting), card("f", Status::Finished)]);
        assert_eq!(state.tabs(), vec![Filter::All, Filter::ToRefine, Filter::Waiting, Filter::Finished]);
        assert!(!keyboard(pending(&["p"])).tabs().contains(&Filter::Finished));
    }

    /// Its cards alone, each with Back to list, Edit, Speak and Remove, and
    /// no Clear all.
    #[test]
    fn the_finished_tab_lists_its_cards_with_their_buttons_and_no_clear_all() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("f1", Status::Finished), card("f2", Status::Finished)]);
        assert_eq!(uuids(&state), vec!["p"], "All leaves them out");
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::Finished)), vec![Effect::Render]);
        assert_eq!(uuids(&state), vec!["f1", "f2"]);
        assert_eq!(state.visible()[0].actions, vec![Action::Back, Action::Edit, Action::Speak, Action::Remove]);
        assert_eq!(state.focus(), focused("f1", Slot::Body).as_ref());
        assert!(!state.shows_clear_all());
        assert_eq!(key(&mut state, KeyAction::ClearAll), Vec::new());
    }

    /// Back to list reopens the task with `task status <uuid> stopped`: it
    /// leaves the tab, the focus moves on and the keyboard stays. Once the
    /// daemon's next cards have it pending, it is back on All.
    #[test]
    fn back_to_list_on_a_finished_card_puts_it_back_on_all() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("f1", Status::Finished), card("f2", Status::Finished)]);
        key(&mut state, KeyAction::Filter(Filter::Finished));
        assert_eq!(
            key(&mut state, KeyAction::Run(Action::Back)),
            vec![Effect::Focus(focused("f2", Slot::Body)), Effect::Spawn(vec![
                "task".into(), "status".into(), "f1".into(), "stopped".into(),
            ])],
        );
        assert!(state.keyboard());
        state.set_cards(&[card("p", Status::Pending), card("f1", Status::Pending), card("f2", Status::Finished)]);
        assert_eq!(state.filter(), Filter::Finished);
        assert_eq!(uuids(&state), vec!["f2"]);
        key(&mut state, KeyAction::Filter(Filter::All));
        assert_eq!(uuids(&state), vec!["p", "f1"]);
    }

    /// Finishing a task while the panel is up takes it off All and onto
    /// the Finished tab, which comes up with it.
    #[test]
    fn finishing_a_task_moves_it_to_the_finished_tab() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert!(!state.tabs().contains(&Filter::Finished));
        state.set_cards(&[card("b", Status::Pending), card("a", Status::Finished)]);
        assert_eq!(uuids(&state), vec!["b"]);
        assert!(state.tabs().contains(&Filter::Finished));
        key(&mut state, KeyAction::Filter(Filter::Finished));
        assert_eq!(uuids(&state), vec!["a"]);
    }

    /// [ and ] step from Waiting to Finished, then Ideas.
    #[test]
    fn the_brackets_step_from_waiting_to_finished_to_ideas() {
        let mut state = keyboard(vec![card("w", Status::Waiting), card("f", Status::Finished)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        key(&mut state, KeyAction::NextFilter);
        assert_eq!(state.tab(), Tab::Filter(Filter::Finished));
        key(&mut state, KeyAction::NextFilter);
        assert_eq!(state.tab(), Tab::Ideas);
        key(&mut state, KeyAction::PrevFilter);
        assert_eq!(state.tab(), Tab::Filter(Filter::Finished));
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

    /// Enter or a click on a card's body does nothing: its actions are its
    /// buttons. The keyboard and the focus stay where they were.
    #[test]
    fn a_press_on_the_body_does_nothing() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.on_press("a", Slot::Body), Vec::new());
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
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

    // ─── Ctrl+Delete ─────────────────────────────────────────────────────

    /// One press: Remove's own command, no Confirm remove, the focus on to
    /// the next card and the list kept up.
    #[test]
    fn ctrl_delete_deletes_the_focused_task_in_one_press() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert_eq!(
            key(&mut state, KeyAction::Delete),
            vec![Effect::Focus(focused("b", Slot::Body)), Effect::Spawn(Action::Remove.args("a"))],
        );
        assert!(state.keyboard(), "the list stays up");
        assert_eq!(state.armed(), &Armed::None, "no Confirm remove left behind");
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
    }

    /// It acts on the focused task, not the focused button: the same from
    /// any of the card's buttons.
    #[test]
    fn ctrl_delete_works_from_any_button_on_the_card() {
        for slot in [Slot::Button(Action::Edit), Slot::Button(Action::Start), Slot::Button(Action::Remove)] {
            let mut state = keyboard(pending(&["a", "b"]));
            state.on_focus(focused("a", slot));
            assert_eq!(
                key(&mut state, KeyAction::Delete),
                vec![Effect::Focus(focused("b", Slot::Body)), Effect::Spawn(Action::Remove.args("a"))],
                "{slot:?}",
            );
        }
    }

    #[test]
    fn ctrl_delete_on_an_armed_remove_deletes_without_asking_again() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Run(Action::Remove));
        assert_eq!(
            key(&mut state, KeyAction::Delete),
            vec![Effect::Focus(focused("b", Slot::Body)), Effect::Spawn(Action::Remove.args("a"))],
        );
        assert_eq!(state.armed(), &Armed::None);
    }

    #[test]
    fn ctrl_delete_on_the_last_card_focuses_the_one_above() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::NextCard);
        assert_eq!(
            key(&mut state, KeyAction::Delete),
            vec![Effect::Focus(focused("a", Slot::Body)), Effect::Spawn(Action::Remove.args("b"))],
        );
    }

    /// No neighbour to move to, so nothing moves the focus off it to disarm
    /// it: it is disarmed all the same.
    #[test]
    fn ctrl_delete_on_the_only_card_deletes_it_and_disarms() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(key(&mut state, KeyAction::Delete), vec![Effect::Spawn(Action::Remove.args("a"))]);
        assert_eq!(state.armed(), &Armed::None);
        assert!(state.keyboard());
    }

    /// No card to act on — All, with every task waiting — and nothing to
    /// delete. ("+N more" is never focused and has no Remove either.)
    #[test]
    fn ctrl_delete_with_no_card_does_nothing() {
        let mut state = keyboard(vec![card("w", Status::Waiting)]);
        assert_eq!(key(&mut state, KeyAction::Delete), Vec::new());
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

    /// Ctrl+Shift+Delete again confirms, as Enter does.
    #[test]
    fn a_second_clear_all_key_confirms_it() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        assert_eq!(
            key(&mut state, KeyAction::ClearAll),
            vec![Effect::Render, Effect::DeleteAll(vec!["w1".into(), "w2".into()])],
        );
    }

    #[test]
    fn a_cards_keys_do_nothing_while_clear_all_is_armed() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        for k in [KeyAction::Run(Action::Edit), KeyAction::Run(Action::Remove), KeyAction::Advance, KeyAction::Delete, KeyAction::Ignore] {
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

    // ─── the Ideas tab ───────────────────────────────────────────────────

    #[test]
    fn six_opens_ideas_with_no_cards_no_focus_and_no_clear_all() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        assert_eq!(key(&mut state, KeyAction::Ideas), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Ideas);
        assert!(state.on_ideas());
        assert!(state.visible().is_empty(), "Ideas shows no cards");
        assert_eq!(state.empty_text(), None, "nor says it has none");
        assert_eq!(state.focus(), None);
        assert!(!state.shows_clear_all());
        assert_eq!(key(&mut state, KeyAction::Ideas), Vec::new(), "already there");
    }

    /// ] from Waiting, the last filter tab, reaches Ideas; [ comes back, onto
    /// the first card.
    #[test]
    fn the_bracket_past_waiting_is_ideas() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        assert_eq!(key(&mut state, KeyAction::NextFilter), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Ideas);
        assert_eq!(key(&mut state, KeyAction::NextFilter), Vec::new(), "Ideas is the last");
        assert_eq!(key(&mut state, KeyAction::PrevFilter), vec![Effect::Render, Effect::SaveIdeas]);
        assert_eq!(state.tab(), Tab::Filter(Filter::Waiting));
        assert_eq!(state.focus(), focused("w", Slot::Body).as_ref());
    }

    /// The text area takes every key as typing: on Ideas a card's keys, the
    /// arrows, Enter and the rest are not the panel's.
    #[test]
    fn on_ideas_only_escape_and_the_tab_keys_are_the_panels() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        for k in [
            KeyAction::Run(Action::Edit),
            KeyAction::Run(Action::Remove),
            KeyAction::PrevCard,
            KeyAction::NextCard,
            KeyAction::PrevSlot,
            KeyAction::NextSlot,
            KeyAction::Enter,
            KeyAction::Advance,
            KeyAction::ClearAll,
            KeyAction::Ignore,
        ] {
            assert_eq!(state.on_key(k), None, "{k:?}");
        }
        assert!(state.on_ideas());
    }

    /// Escape on Ideas saves and goes back to the task list, on the tab
    /// picked before Ideas, keeping the keyboard; a second Escape gives it
    /// back.
    #[test]
    fn escape_on_ideas_saves_and_goes_back_to_the_tab_before_it() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::SaveIdeas]);
        assert!(state.keyboard());
        assert_eq!(state.tab(), Tab::Filter(Filter::Waiting));
        assert_eq!(state.focus(), focused("w", Slot::Body).as_ref());
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
        assert!(!state.keyboard());
    }

    /// The tab picked before Ideas may have emptied meanwhile: then All.
    #[test]
    fn escape_on_ideas_goes_to_all_when_the_tab_before_it_emptied() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        key(&mut state, KeyAction::Ideas);
        state.set_cards(&pending(&["p"]));
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::SaveIdeas]);
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
    }

    /// Leaving Ideas works for the filter tab picked before it too: All here.
    #[test]
    fn a_tab_key_or_click_leaves_ideas_for_its_tab() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::All)), vec![Effect::Render, Effect::SaveIdeas]);
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(state.on_tab(Tab::Ideas), vec![Effect::Render], "nothing to save on the way in");
        assert_eq!(state.on_tab(Tab::Filter(Filter::ToRefine)), vec![Effect::Render, Effect::SaveIdeas]);
        assert_eq!(state.tab(), Tab::Filter(Filter::ToRefine));
    }

    #[test]
    fn a_hidden_tabs_key_on_ideas_stays_on_ideas() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::Planned)), Vec::new());
        assert!(state.on_ideas());
    }

    /// A tick mid-typing draws again and stays on Ideas, no card focused.
    #[test]
    fn a_refresh_keeps_ideas() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(state.set_cards(&pending(&["a", "b"])), vec![Effect::Render]);
        assert!(state.on_ideas());
        assert_eq!(state.focus(), None);
    }

    /// The last task going mid-typing keeps the keyboard on Ideas: giving it
    /// back would send the next keystrokes to whatever window is under it.
    #[test]
    fn the_last_task_going_on_ideas_keeps_the_keyboard() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(state.set_cards(&[]), vec![Effect::Render]);
        assert!(state.keyboard());
        assert!(state.on_ideas());
        assert!(!state.hidden(), "the notepad still shows");
    }

    /// With no cards left, leaving Ideas for a filter tab gives the keyboard
    /// back rather than hide the panel while it holds it.
    #[test]
    fn leaving_ideas_with_no_cards_gives_the_keyboard_back() {
        for k in [KeyAction::Filter(Filter::All), KeyAction::PrevFilter, KeyAction::Release] {
            let mut state = keyboard(pending(&["a"]));
            key(&mut state, KeyAction::Ideas);
            state.set_cards(&[]);
            assert_eq!(key(&mut state, k), vec![Effect::Render, Effect::Release], "{k:?}");
            assert!(!state.keyboard());
            assert!(state.hidden());
        }
    }

    #[test]
    fn clicking_a_filter_tab_from_ideas_with_no_cards_gives_the_keyboard_back() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        state.set_cards(&[]);
        assert_eq!(state.on_tab(Tab::Filter(Filter::All)), vec![Effect::Render, Effect::Release]);
        assert!(!state.keyboard());
    }

    #[test]
    fn the_keyboard_opens_on_all_even_from_ideas() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn six_disarms_clear_all_on_the_way_to_ideas() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        assert_eq!(key(&mut state, KeyAction::Ideas), vec![Effect::Render]);
        assert_eq!(state.armed(), &Armed::None);
        assert!(state.on_ideas());
    }

    // ─── Move to workspace ───────────────────────────────────────────────

    fn projects(local: &[&str], remote: &[&str]) -> crate::project::Projects {
        crate::project::Projects {
            dir: std::path::PathBuf::from("/home/x/Projects"),
            local: local.iter().map(|s| s.to_string()).collect(),
            remote: remote.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn folders(names: &[&str]) -> Vec<Row> {
        names.iter().map(|n| Row::Folder(n.to_string())).collect()
    }

    /// The matcher the tests rank with: fzf is the daemon's.
    fn substring(names: &[String], query: &str) -> Vec<String> {
        crate::project::substring_matches(names, query)
    }

    /// The panel on `a`'s project list, of folders x and y.
    fn moving_a() -> PanelState {
        let mut state = keyboard(pending(&["a", "b"]));
        assert_eq!(key(&mut state, KeyAction::Run(Action::Move)), vec![Effect::ListProjects("a".into())]);
        assert_eq!(state.show_projects("a", folders(&["x", "y"]), projects(&["a", "x", "y"], &[])), vec![Effect::Render]);
        state
    }

    /// m asks for the folders, keeping the keyboard and the focus; they come
    /// back as the project list, on the first.
    #[test]
    fn move_shows_the_project_list_in_place_of_the_cards() {
        let state = moving_a();
        assert!(state.keyboard());
        let list = state.projects().unwrap();
        assert_eq!(list.purpose(), &Purpose::Move { uuid: "a".into(), text: "a".into() });
        assert_eq!(list.shown(), folders(&["x", "y"]));
        assert_eq!(list.query(), "");
        assert_eq!(list.at(), 0);
        assert!(state.visible().is_empty(), "no task cards");
        assert!(state.tabs().is_empty(), "no tabs");
        assert_eq!(state.empty_text(), None);
        assert!(!state.shows_clear_all());
        assert!(!state.hidden());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref(), "kept for Escape");
    }

    /// The line over the folders names the task, and the footer Move's keys.
    #[test]
    fn the_move_list_names_its_task_and_its_keys() {
        let state = moving_a();
        let list = state.projects().unwrap();
        assert_eq!(list.title(), format!("{}: a", Action::Move.label(false)));
        assert_eq!(list.keys(), crate::panel::projects::MOVE_KEYS);
        assert_eq!(list.moving(), Some("a"));
    }

    #[test]
    fn no_other_project_says_so_and_stays_on_the_cards() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.show_projects("a", Vec::new(), projects(&[], &[])), vec![Effect::Notify(NO_DESTINATIONS.into())]);
        assert_eq!(state.projects(), None);
    }

    /// An armed Remove must not survive an m that finds no folders: the next
    /// Enter would delete the task.
    #[test]
    fn move_disarms_remove_even_when_there_is_no_folder() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Run(Action::Remove));
        assert_eq!(state.armed(), &Armed::Remove("a".into()));
        assert_eq!(key(&mut state, KeyAction::Run(Action::Move)), vec![Effect::ListProjects("a".into())]);
        assert_eq!(state.armed(), &Armed::None);
        assert_eq!(state.show_projects("a", Vec::new(), projects(&[], &[])), vec![Effect::Notify(NO_DESTINATIONS.into())]);
        assert_eq!(state.armed(), &Armed::None);
    }

    #[test]
    fn up_and_down_walk_the_folders_and_stop_at_the_ends() {
        let mut state = moving_a();
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::ShowFolder]);
        assert_eq!(state.projects().map(ProjectList::at), Some(1));
        key(&mut state, KeyAction::NextCard);
        assert_eq!(state.projects().map(ProjectList::at), Some(1), "stops at the last");
        key(&mut state, KeyAction::PrevCard);
        key(&mut state, KeyAction::PrevCard);
        assert_eq!(state.projects().map(ProjectList::at), Some(0), "stops at the first");
    }

    /// Enter moves the task to the folder the keyboard is on, back on the
    /// cards with the focus on the next one, as the task leaves this list.
    #[test]
    fn enter_moves_the_task_to_the_focused_folder() {
        let mut state = moving_a();
        key(&mut state, KeyAction::NextCard);
        assert_eq!(
            key(&mut state, KeyAction::Enter),
            vec![Effect::Render, Effect::Spawn(vec!["task".into(), "move".into(), "a".into(), "y".into()])],
        );
        assert_eq!(state.projects(), None);
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
    }

    #[test]
    fn a_click_on_a_folder_moves_the_task_there() {
        let mut state = moving_a();
        assert_eq!(
            state.on_folder(0),
            vec![Effect::Render, Effect::Spawn(vec!["task".into(), "move".into(), "a".into(), "x".into()])],
        );
        assert_eq!(state.on_folder(0), Vec::new(), "once the list has gone");
    }

    /// Escape goes back to the cards on the same card and slot; a second
    /// gives the keyboard back.
    #[test]
    fn escape_goes_back_to_the_same_card() {
        let mut state = moving_a();
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render]);
        assert_eq!(state.projects(), None);
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
    }

    /// Every other key is typing, for the text field: not the panel's.
    #[test]
    fn the_project_list_leaves_every_other_key_to_the_text_field() {
        let mut state = moving_a();
        for k in [
            KeyAction::Run(Action::Edit),
            KeyAction::Run(Action::Remove),
            KeyAction::PrevSlot,
            KeyAction::NextSlot,
            KeyAction::Filter(Filter::All),
            KeyAction::Ideas,
            KeyAction::PrevFilter,
            KeyAction::NextFilter,
            KeyAction::Advance,
            KeyAction::Delete,
            KeyAction::ClearAll,
            KeyAction::Ignore,
        ] {
            assert_eq!(state.on_key(k), None, "{k:?}");
        }
        assert!(state.projects().is_some());
        assert_eq!(state.on_tab(Tab::Ideas), Vec::new());
    }

    /// What is typed narrows the list to the matches the surface ranked, and
    /// puts the highlight back on the top one.
    #[test]
    fn typing_narrows_the_list_to_the_top_match() {
        let mut state = moving_a();
        key(&mut state, KeyAction::NextCard);
        assert_eq!(state.on_query("y", &substring), vec![Effect::Render]);
        let m = state.projects().unwrap();
        assert_eq!((m.query(), m.shown().to_vec(), m.at()), ("y", folders(&["y"]), 0));
        assert_eq!(
            key(&mut state, KeyAction::Enter),
            vec![Effect::Render, Effect::Spawn(vec!["task".into(), "move".into(), "a".into(), "y".into()])],
        );
    }

    /// The field's text set to what it already is (as clearing it does)
    /// changes nothing.
    #[test]
    fn the_same_text_again_changes_nothing() {
        let mut state = moving_a();
        assert_eq!(state.on_query("", &substring), Vec::new());
        state.on_query("y", &substring);
        assert_eq!(state.on_query("y", &substring), Vec::new());
    }

    #[test]
    fn down_then_enter_picks_the_second_match() {
        let mut state = moving_a();
        // A ranking that puts y over x, as fzf can.
        let ranked = |_: &[String], _: &str| vec!["y".to_string(), "x".to_string()];
        state.on_query("q", &ranked);
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::ShowFolder]);
        assert_eq!(
            key(&mut state, KeyAction::Enter),
            vec![Effect::Render, Effect::Spawn(vec!["task".into(), "move".into(), "a".into(), "x".into()])],
        );
    }

    /// Nothing matches: Enter and the arrows do nothing, and the list stays.
    #[test]
    fn with_no_match_enter_does_nothing() {
        let mut state = moving_a();
        state.on_query("zz", &substring);
        assert_eq!(key(&mut state, KeyAction::NextCard), Vec::new());
        assert_eq!(key(&mut state, KeyAction::Enter), Vec::new());
        assert!(state.projects().is_some());
    }

    /// A click picks from the list as it shows, narrowed.
    #[test]
    fn a_click_picks_from_the_narrowed_list() {
        let mut state = moving_a();
        state.on_query("y", &substring);
        assert_eq!(
            state.on_folder(0),
            vec![Effect::Render, Effect::Spawn(vec!["task".into(), "move".into(), "a".into(), "y".into()])],
        );
    }

    /// With text typed, Escape clears it and shows every folder again; the
    /// next Escape goes back to the cards.
    #[test]
    fn escape_clears_the_text_then_goes_back() {
        let mut state = moving_a();
        state.on_query("y", &substring);
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::ClearQuery, Effect::Render]);
        let m = state.projects().unwrap();
        assert_eq!((m.query(), m.shown().to_vec(), m.at()), ("", folders(&["x", "y"]), 0));
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render]);
        assert_eq!(state.projects(), None);
    }

    /// The text typed for one task's list does not carry over to the next.
    #[test]
    fn a_new_list_starts_with_no_text() {
        let mut state = moving_a();
        state.on_query("y", &substring);
        key(&mut state, KeyAction::Release);
        key(&mut state, KeyAction::Release);
        key(&mut state, KeyAction::Run(Action::Move));
        state.show_projects("a", folders(&["x", "y"]), projects(&["a", "x", "y"], &[]));
        assert_eq!(state.projects().map(ProjectList::query), Some(""));
    }

    /// GTK's focus on a folder card is no card's: the task's focus is kept
    /// for Escape.
    #[test]
    fn focus_moves_on_the_project_list_keep_the_tasks_focus() {
        let mut state = moving_a();
        state.on_focus(None);
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn a_refresh_keeps_the_list_while_its_task_is_there() {
        let mut state = moving_a();
        key(&mut state, KeyAction::NextCard);
        assert_eq!(state.set_cards(&pending(&["a", "b", "c"])), vec![Effect::Render]);
        assert_eq!(state.projects().map(ProjectList::at), Some(1));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    /// The task done or moved elsewhere meanwhile: the list goes with it.
    #[test]
    fn a_refresh_without_its_task_closes_the_list() {
        let mut state = moving_a();
        assert_eq!(state.set_cards(&pending(&["b"])), vec![Effect::Render]);
        assert_eq!(state.projects(), None);
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
    }

    #[test]
    fn taking_the_keyboard_again_closes_the_list() {
        let mut state = moving_a();
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.projects(), None);
    }

    #[test]
    fn no_project_list_without_the_keyboard() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a"]));
        assert_eq!(state.show_projects("a", folders(&["x"]), projects(&["a", "x"], &[])), Vec::new());
        assert_eq!(state.projects(), None);
    }

    // ─── Open a project (Mod+Alt+W) ──────────────────────────────────────

    /// The panel on the open list of folder x and GitHub repo y.
    fn opening(cards: &[Card]) -> PanelState {
        let mut state = PanelState::default();
        state.set_cards(cards);
        assert_eq!(state.open_projects(projects(&["x"], &["y"])), vec![Effect::Render]);
        state
    }

    fn open(row: &str) -> Effect {
        Effect::Spawn(crate::project::open_args(row))
    }

    /// With no task on the workspace, the list still takes the keyboard and
    /// shows: no cards, no tabs, no focus.
    #[test]
    fn the_open_list_shows_with_no_cards() {
        let state = opening(&[]);
        assert!(state.keyboard());
        assert!(!state.hidden());
        let list = state.projects().unwrap();
        assert_eq!(list.purpose(), &Purpose::Open);
        assert_eq!(list.shown(), [Row::Folder("x".into()), Row::Repo("y".into())]);
        assert_eq!(
            (list.title(), list.keys()),
            (crate::panel::projects::OPEN_TITLE.to_string(), crate::panel::projects::OPEN_KEYS)
        );
        assert_eq!(list.moving(), None);
        assert!(state.visible().is_empty());
        assert!(state.tabs().is_empty());
        assert_eq!(state.empty_text(), None);
        assert_eq!(state.focus(), None);
    }

    /// Over cards it takes their place, dropping the focus and anything armed.
    #[test]
    fn the_open_list_takes_the_place_of_the_cards() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Run(Action::Remove));
        assert_eq!(state.open_projects(projects(&["x"], &[])), vec![Effect::Render]);
        assert!(state.visible().is_empty());
        assert_eq!(state.focus(), None);
        assert_eq!(state.armed(), &Armed::None);
    }

    /// Opening the list from Ideas leaves the notepad: the text there was
    /// saved on the way in, and the list is what shows.
    #[test]
    fn the_open_list_leaves_ideas() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert!(state.on_ideas());
        assert_eq!(state.open_projects(projects(&["x"], &[])), vec![Effect::Render]);
        assert!(!state.on_ideas());
        assert!(state.projects().is_some());
    }

    /// Opening it over Move's list replaces Move's purpose and drops the
    /// text typed there, so a leftover filter cannot hide the rows.
    #[test]
    fn the_open_list_replaces_moves_list_and_its_query() {
        let mut state = moving_a();
        state.on_query("zz", &substring);
        assert_eq!(state.open_projects(projects(&["x"], &[])), vec![Effect::Render]);
        let list = state.projects().unwrap();
        assert_eq!(list.purpose(), &Purpose::Open);
        assert_eq!(list.query(), "");
        assert_eq!(list.shown(), folders(&["x"]));
    }

    /// Without fzf the ranking is plain text, so "my project" shows no row
    /// for the folder "my-project". Enter still opens it, and the line says so
    /// rather than leaving a dead key.
    #[test]
    fn typed_text_naming_an_existing_folder_opens_it_with_no_row() {
        let mut state = PanelState::default();
        state.open_projects(projects(&["my-project"], &[]));
        state.on_query("my project", &substring);
        assert_eq!(state.projects().map(|l| l.shown().len()), Some(0));
        assert_eq!(state.hint(), Some("Enter opens ~/Projects/my-project".to_string()));
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("my-project")]);
    }

    /// Enter opens the highlighted row by its bare name, a repo as a folder,
    /// giving the keyboard back first: the user is off to that workspace.
    #[test]
    fn enter_opens_the_highlighted_row_and_gives_the_keyboard_back() {
        let mut state = opening(&[]);
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::ShowFolder]);
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("y")]);
        assert!(!state.keyboard());
        assert_eq!(state.projects(), None);
        assert!(state.hidden(), "no cards to peek");
    }

    #[test]
    fn a_click_opens_that_row() {
        let mut state = opening(&pending(&["a"]));
        assert_eq!(state.on_folder(0), vec![Effect::Render, Effect::Release, open("x")]);
        assert!(!state.hidden(), "the card peeks again");
    }

    /// A name matching nothing makes a folder of it, normalised as
    /// `project open` will, and the line under the title says so first.
    #[test]
    fn a_name_matching_nothing_makes_a_typed_pick() {
        let mut state = opening(&[]);
        state.on_query("my thing", &substring);
        assert_eq!(state.hint(), Some("Enter makes ~/Projects/my-thing".to_string()));
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("my-thing")]);
    }

    /// While anything matches, Enter takes the match. The ranking shows x
    /// for "xx", as fzf can, so the typed text alone would be a new name.
    #[test]
    fn a_match_wins_over_a_new_name() {
        let mut state = opening(&[]);
        state.on_query("xx", &|_: &[String], _: &str| vec!["x".to_string()]);
        assert_eq!(state.hint(), None);
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("x")]);
    }

    /// A typed repo name the ranking missed clones, and the line says so:
    /// the hint is decided against the repos the list was built from.
    #[test]
    fn a_typed_repo_name_with_no_row_says_it_clones() {
        let mut state = PanelState::default();
        state.open_projects(projects(&[], &["y"]));
        state.on_query("y", &|_: &[String], _: &str| Vec::new());
        assert_eq!(state.hint(), Some("Enter clones y from GitHub".into()));
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("y")]);
    }

    /// A name that cannot be a folder says why, and Enter does nothing.
    #[test]
    fn a_name_that_cannot_be_a_folder_says_why() {
        let mut state = opening(&[]);
        state.on_query("a/b", &substring);
        let text = state.hint().unwrap();
        assert!(text.contains("can't contain '/'"), "{text}");
        assert_eq!(key(&mut state, KeyAction::Enter), Vec::new());
        assert!(state.projects().is_some());
    }

    /// An empty ~/Projects with nothing typed asks for a name.
    #[test]
    fn no_folder_and_nothing_typed_asks_for_a_name() {
        let mut state = PanelState::default();
        state.open_projects(projects(&[], &[]));
        assert_eq!(state.hint(), Some(crate::panel::projects::TYPE_A_NAME.to_string()));
        assert_eq!(key(&mut state, KeyAction::Enter), Vec::new());
    }

    /// Move's list never makes a folder: with nothing matching, Enter does
    /// nothing and no line says otherwise.
    #[test]
    fn the_move_list_makes_no_typed_pick() {
        let mut state = moving_a();
        state.on_query("zz", &substring);
        assert_eq!(state.hint(), None);
        assert_eq!(key(&mut state, KeyAction::Enter), Vec::new());
    }

    /// Escape clears the text, then gives the keyboard back: there may be
    /// no cards to go back to.
    #[test]
    fn escape_clears_the_text_then_closes_the_open_list() {
        let mut state = opening(&pending(&["a"]));
        state.on_query("y", &substring);
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::ClearQuery, Effect::Render]);
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
        assert!(!state.keyboard());
        assert_eq!(state.projects(), None);
    }

    /// The daemon's ticks with no task, or with tasks coming and going,
    /// leave the open list up and the keyboard held.
    #[test]
    fn ticks_keep_the_open_list() {
        let mut state = opening(&[]);
        assert_eq!(state.set_cards(&[]), Vec::new());
        assert_eq!(state.set_cards(&pending(&["a"])), vec![Effect::Render]);
        assert_eq!(state.set_cards(&[]), vec![Effect::Render]);
        assert!(state.keyboard());
        assert_eq!(state.projects().map(|l| l.purpose().clone()), Some(Purpose::Open));
    }

    /// Mod+Alt+Ctrl+T over the open list puts the cards back.
    #[test]
    fn taking_the_keyboard_closes_the_open_list() {
        let mut state = opening(&pending(&["a"]));
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.projects(), None);
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }
}
