# Pure PanelState Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the task panel's state (cards, keyboard, filter tab, expanded, focus, arming) out of loose interior-mutable fields and widget lookups in `src/panel/surface.rs` into a pure, unit-tested `PanelState` in a new `src/panel/state.rs`, with `surface.rs` reduced to the GTK adapter that draws the state and runs its effects.

**Architecture:** `PanelState` holds the cards, the keyboard flag, the filter tab, `expanded`, the herdr agents, the focus as `Focus { uuid, slot }` and an `Armed` enum (`None`, `Remove(uuid)`, `ClearAll { before }`). It is keyed by task uuid and slot, never by widget. Its interface is a few queries (`visible()`, `tabs()`, `hidden()`, `focus()`, `armed()`...) and event methods (`set_cards`, `take_keyboard`, `on_key`, `on_press`, `on_clear_all`, `on_more`, `on_tab`, `on_tuck`, `on_focus`) that return `Vec<Effect>` for `surface.rs` to run in order: `Render`, `Focus`, `Release`, `Spawn`, `DeleteAll`, `Notify`. `surface.rs` keeps the widgets of each drawn card in a `CardWidgets` list, so it never finds anything by widget name or CSS class. It tells the state when GTK's focus moves, and it draws arming and the focused card's action row in a `sync()` after every change. The already-pure `model.rs` (Filter, cap), `keys.rs` (key_action, step) and `actions.rs` (for_status, advance, args) become the state's internals.

**Tech Stack:** Rust 2021, gtk4-rs 0.11, gtk4-layer-shell; bash + Pillow + wtype for the nested-niri e2e test.

**Spec:** Taskwarrior task `637796b7-0316-4a4b-a373-cd7d77b95a97`. Read it with `task rc.json.array=on 637796b7-0316-4a4b-a373-cd7d77b95a97 export`; its description and notes are the spec. Its report link is the 2026-10-02 architecture review, candidate 1.

## Global Constraints

- Behaviour stays the same. Every rule the spec lists keeps working as it does today, checked by the state's unit tests and by `tests/e2e-panel.sh`. The one deliberate change: when Back, Waiting or Remove takes a card off the list, the focus goes to the next *task* card, skipping "+N more". Today it could land on the "+N more" body.
- Unit tests go through `PanelState`'s public interface. They use no GTK and no `gdk::Key`, and need no display (the repo's rule, `docs/superpowers/plans/2026-09-30-task-box-style-once.md`: `cargo test` runs over SSH). `keys.rs`'s own tests already call `gdk::Key` constants without GTK init, and they stay.
- Replace, don't layer. `keys::Armed` and `keys::while_clear_armed`, with their four tests, go once `PanelState::on_key` covers them (Task 3). In e2e-panel.sh, the checks the unit tests take over go (Task 4).
- Out of scope: the hover grace timer and the slide (they stay in `surface.rs` and are checked by hand), the blur (ADR-0001 stands), the spinner's timer (it stays in `surface.rs`, now finding Speak through `CardWidgets`), and the follow-up task-action catalogue (candidate 2 of the review).
- Fix along the way: the stale module doc at `surface.rs:36-38` (lists s/r/e/t/Delete; misses g and b), and `render`'s second read of `keyboard` after `replace(false)`. Both go with the rewrite in Task 3.
- House style: doc comments say *why*, in the plain voice of the surrounding code ("Moving the focus off an armed Remove disarms it"). Commits follow Conventional Commits, `refactor(panel): …` here, imperative, lowercase, subject ≤72 characters, ending with a blank line and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Run every e2e check against the build in this worktree, not the installed binary: `cargo build && NIRITASKS=./target/debug/niritasks bash tests/e2e-panel.sh`. Without `NIRITASKS` the script runs whatever `niritasks` is on `PATH`. While it runs, keep off the workspace the nested niri is parked on.

## Design decisions

The spec's proposed shape was marked "to grill before building". The code in this plan was drafted and checked before the plan was written: all 40 state tests passed, clippy was clean, and `tests/e2e-panel.sh` passed 37/37 against the rewritten adapter, then 24/24 once trimmed. These are the choices it settled, where it differs from the spec's sketch:

1. **The state owns the focus.** `PanelState.focus: Option<Focus>` is the truth. `Effect::Focus` moves GTK's focus to match it, and GTK's `focus-widget` notify (a click on a button) calls `on_focus`, which does the disarming. When an `Effect::Focus` comes back through that notify it changes nothing, because the focus is already equal. While the adapter tears the column down, or hides the row the focus is leaving, a `drawing` guard keeps GTK's own focus moves from reaching the state.
2. **`refocus(prev)` is internal.** The spec sketched a public `refocus(prev)`. Because the state already knows the previous focus, every re-render (`set_cards`, a tab switch) computes the new one itself: the same card and slot; the body when the card lost that button; the first card when the card left. Tests check it through `set_cards` and `focus()`.
3. **`on_press(uuid, Slot)`, not `on_press(Action, uuid)`.** A body press opens the menu and goes through the same method (`Slot::Body`). "+N more" has no uuid, so it gets its own `on_more()`.
4. **`on_key` returns `Option<Vec<Effect>>`.** `None` means the key isn't the panel's, and GTK gets it: Enter and Space press the focused button. A new `KeyAction::Enter` lets an armed Clear all take Enter as Confirm. That is what `keys::while_clear_armed` did with raw keys, and why it can go.
5. **One arming at a time is structural.** `Armed` is one enum, so arming Remove replaces an armed Clear all, and the reverse. The explicit `disarm_remove` / `disarm_clear` calls go.
6. **Arming faces and rows are drawn, not effected.** No effect says "relabel Remove". After every `apply`, and on every GTK focus move, `sync()` draws Remove's and Clear all's labels from `armed()`, and the row and hint from `focus()`.

## File Structure

- Create `src/panel/state.rs`: `PanelState`, `Focus`, `Slot`, `Armed`, `Shown`, `Effect`, and their unit tests. Every rule between the panel's pieces of state.
- Modify `src/panel/mod.rs`: add `pub mod state;`.
- Modify `src/panel/model.rs`: `Filter` derives `Default` (All).
- Modify `src/panel/keys.rs`: add `KeyAction::Enter` (Task 2); remove `Armed` and `while_clear_armed` (Task 3).
- Modify `src/panel/surface.rs`: becomes the adapter (Task 3). It shrinks from 1565 to about 1300 lines. `Panel::new` goes from ~277 lines to ~80, with `connect_*` methods and `scroller()` / `tab_bar()` builders; `render` goes from ~94 lines to ~45; `card_widget` (~100) splits into `card_widget` (~45) and `action_row`.
- Modify `tests/e2e-panel.sh`: drop 13 checks now unit-tested (Task 4).
- Modify `README.md`: the paragraph on what e2e-panel.sh measures (Task 4).

---

### Task 1: PanelState: what shows and where the focus is

**Files:**
- Create: `src/panel/state.rs`
- Modify: `src/panel/mod.rs` (add the module)
- Modify: `src/panel/model.rs:78` (`Filter` derives `Default`)

**Interfaces:**
- Consumes: `model::{Card, Filter, CAP, cap}`, `Filter::{pick, shown, TABS, empty_text}`, `keys::{KeyAction, step}`, `actions::Action::for_status`, `crate::link::session_agent(&[String], &str) -> Option<String>`, all existing.
- Produces (later tasks rely on these exact names):
  - `pub enum Slot { Body, Button(Action) }` (Copy, Eq)
  - `pub struct Focus { pub uuid: String, pub slot: Slot }`, `Focus::body(&str) -> Focus`
  - `pub enum Armed { None, Remove(String), ClearAll { before: Option<Focus> } }` (Default = None, Clone, Eq)
  - `pub struct Shown { pub card: Card, pub actions: Vec<Action> }`, `Shown::slots(&self) -> Vec<Slot>`
  - `pub enum Effect { Render, Focus(Option<Focus>), Release, Spawn(Vec<String>), DeleteAll(Vec<String>), Notify(String) }`
  - `#[derive(Default)] pub struct PanelState` with `keyboard() -> bool`, `filter() -> Filter`, `focus() -> Option<&Focus>`, `armed() -> &Armed`, `hidden() -> bool`, `tabs() -> Vec<Filter>`, `shows_clear_all() -> bool`, `visible() -> Vec<Shown>`, `empty_text() -> Option<&'static str>`, `set_cards(&mut, &[Card]) -> Vec<Effect>`, `take_keyboard(&mut, Vec<String>) -> bool`, `on_tuck(&mut) -> Vec<Effect>`, `on_more(&mut) -> Vec<Effect>`, `on_tab(&mut, Filter) -> Vec<Effect>`, `on_focus(&mut, Option<Focus>)`, `on_key(&mut, KeyAction) -> Option<Vec<Effect>>`
  - private, used by Task 2: `release`, `pick`, `filter_key`, `rerender`, `focus_on(Focus) -> Vec<Effect>`, `current(&[Shown]) -> Option<usize>`, `move_card`, `move_slot`, free fn `first_task(&[Shown]) -> Option<Focus>`

This task writes the whole file in one go: its tests and code share helpers, and the module has no callers until Task 3. Follow the TDD order inside it: put the tests in with the types and `todo!()` bodies first and watch them fail, then fill the bodies in.

- [ ] **Step 1: Give `Filter` a default**

In `src/panel/model.rs`, change the derive on `pub enum Filter` and mark All:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
```

In `src/panel/mod.rs`, add the module after `pub mod model;`:

```rust
pub mod model;
pub mod state;
```

- [ ] **Step 2: Write `src/panel/state.rs` with its tests, the method bodies `todo!()`**

Write the file below in full, then replace the body of every `pub fn` that has a `&mut self` receiver (`set_cards`, `take_keyboard`, `on_tuck`, `on_more`, `on_tab`, `on_focus`, `on_key`) with `todo!()`.

````rust
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
    /// Remove disarms it, and moving at all disarms Clear all.
    pub fn on_focus(&mut self, focus: Option<Focus>) {
        if self.focus == focus {
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
    /// panel's key, and GTK should have it.
    pub fn on_key(&mut self, key: KeyAction) -> Option<Vec<Effect>> {
        Some(match key {
            KeyAction::Release => self.release(),
            KeyAction::Filter(_) | KeyAction::PrevFilter | KeyAction::NextFilter => self.filter_key(key),
            KeyAction::PrevCard | KeyAction::NextCard => self.move_card(key == KeyAction::NextCard),
            KeyAction::PrevSlot | KeyAction::NextSlot => self.move_slot(key == KeyAction::NextSlot),
            KeyAction::Run(_) | KeyAction::Advance | KeyAction::ClearAll | KeyAction::Ignore => return None,
        })
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
        assert_eq!(key(&mut state, KeyAction::Release), Vec::new(), "already given back");
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
}
````

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test --lib panel::state`
Expected: it compiles, and the tests panic with `not yet implemented`.

- [ ] **Step 4: Put the method bodies back**

Restore the seven bodies exactly as Step 2's listing has them.

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --lib panel::state`
Expected: `test result: ok. 22 passed`.

Run: `cargo clippy --all-targets 2>&1 | grep -E '^(warning|error)'`
Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add src/panel/state.rs src/panel/mod.rs src/panel/model.rs
git commit -m "refactor(panel): add a pure PanelState for what the panel shows

The cards, the filter tab, \"+N more\" opened or not and the keyboard's
focus, keyed by task uuid and slot rather than by widget, with the tab
fallback, the cap and the refocus after a re-render under unit test.
Nothing draws from it yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: PanelState: presses, arming and Ctrl+Enter

**Files:**
- Modify: `src/panel/state.rs` (add methods, replace `on_key`, add tests)
- Modify: `src/panel/keys.rs` (add `KeyAction::Enter`)
- Modify: `src/panel/surface.rs:445` and `:1009-1014` (treat `Enter` as today's `Ignore`, until Task 3 rewrites the file)

**Interfaces:**
- Consumes: everything Task 1 produced; `Action::{advance, args, leaves_the_list, keeps_keyboard}` and `Filter::uuids`, existing.
- Produces: `KeyAction::Enter`; `PanelState::on_press(&mut, &str, Slot) -> Vec<Effect>`, `PanelState::on_clear_all(&mut) -> Vec<Effect>`; `on_key` now handles `Run`, `Advance`, `ClearAll` and `Enter`, and an armed Clear all.

- [ ] **Step 1: Add `KeyAction::Enter`**

In `src/panel/keys.rs`, replace the `Ignore` variant and its doc:

```rust
    /// Enter, which presses the focused button, as GTK does by itself; but
    /// with Clear all armed and no button focused, it confirms.
    Enter,
    /// Not the panel's key: Space presses the focused button, and the rest
    /// do nothing.
    Ignore,
```

In `key_action`, add a line after the `Escape` arm of the unmodified match:

```rust
        gdk::Key::Escape => KeyAction::Release,
        gdk::Key::Return | gdk::Key::KP_Enter => KeyAction::Enter,
```

In `while_clear_armed` (which stays until Task 3), make Enter confirm:

```rust
        KeyAction::ClearAll | KeyAction::Enter => Armed::Confirm,
```

In its tests, replace `enter_and_space_pass_through` with:

```rust
    /// Enter is its own, for an armed Clear all to confirm on; Space and the
    /// rest are not the panel's.
    #[test]
    fn enter_is_enter_and_space_passes_through() {
        assert_eq!(key_action(gdk::Key::Return, false), KeyAction::Enter);
        assert_eq!(key_action(gdk::Key::KP_Enter, false), KeyAction::Enter);
        for key in [gdk::Key::space, gdk::Key::x] {
            assert_eq!(key_action(key, false), KeyAction::Ignore, "{key:?}");
        }
    }
```

and in `ctrl_enter_advances_the_focused_task` change its last line to:

```rust
        assert_eq!(key_action(gdk::Key::Return, false), KeyAction::Enter);
```

In `src/panel/surface.rs`, the key controller passes Enter on to GTK as before:

```rust
                match keys::key_action(key, ctrl) {
                    KeyAction::Ignore | KeyAction::Enter => glib::Propagation::Proceed,
```

and `Panel::key`'s last arm lists it with the other no-ops:

```rust
            KeyAction::Release
            | KeyAction::Ignore
            | KeyAction::Enter
            | KeyAction::Filter(_)
```

- [ ] **Step 2: Write the failing tests**

At the end of `src/panel/state.rs`'s `mod tests`, before its closing `}`, add:

```rust
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
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --lib panel::state`
Expected: compile errors, `no method named on_press` and `no method named on_clear_all`.

- [ ] **Step 4: Implement**

In `impl PanelState`, replace `on_key` and its doc comment with:

```rust
    /// A key while the panel has the keyboard. None when it is not the
    /// panel's key, and GTK should have it: Enter and Space press the focused
    /// button. Armed, Clear all takes every key: Enter confirms rather than
    /// opening a card, Escape and the keys that move cancel, the tab keys
    /// switch tab as ever, and a card's keys do nothing, there being no card
    /// focused to act on.
    pub fn on_key(&mut self, key: KeyAction) -> Option<Vec<Effect>> {
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
```

Directly after it, add the two public presses:

```rust
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
```

After `move_slot`, add:

```rust
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
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --lib panel`
Expected: `panel::state` reports 40 tests, all ok; the `keys` tests pass with the Enter change.

Run: `cargo clippy --all-targets 2>&1 | grep -E '^(warning|error)'`
Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add src/panel/state.rs src/panel/keys.rs src/panel/surface.rs
git commit -m "refactor(panel): give PanelState the presses and the arming

Remove and Clear all armed as one enum, so only one can be at a time;
the focus moving to a neighbour when a card leaves the list; Ctrl+Enter
pressing only a button the card has; and an armed Clear all taking
every key, Enter included, which gets a KeyAction of its own.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: surface.rs draws PanelState

**Files:**
- Modify: `src/panel/surface.rs` (rewrite from the top of the file through `pub fn close`; add builders and label helpers; one edit in `set_region`; one test)
- Modify: `src/panel/keys.rs` (remove `Armed`, `while_clear_armed` and their four tests)

**Interfaces:**
- Consumes: all of `state.rs`'s public interface (Tasks 1 and 2); `keys::key_action`.
- Produces: `Panel::new`, `Panel::show(&[Card])`, `Panel::take_keyboard(Vec<String>) -> bool`, `Panel::close`, `surface::open_menu`, `surface::NAMESPACE`, `surface::PEEK_PX`, all with the same signatures as today, so `src/daemon.rs` doesn't change.

No new unit test can drive GTK here, so the check is the whole suite plus the full e2e-panel.sh, untrimmed, against this build. All 37 checks passing shows the adapter draws what the old code drew.

- [ ] **Step 1: Run the e2e test on the current build, as the baseline**

Run: `cargo build && NIRITASKS=./target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -3`
Expected: `passed: 37   failed: 0   skipped: 0`. Without wtype, or with herdr or wt on the nested spawn PATH, some checks report SKIP instead; write down the three numbers to compare in Step 6.

- [ ] **Step 2: Replace the top of `surface.rs`, from line 1 through the end of `pub fn close`**

Everything from the first line of the file down to and including `pub fn close(&self) { … }` (today lines 1-1190) becomes the listing below. The listing ends inside `impl Panel`. Leave the rest of the file as it is: `cancel_grace`, `set_region`, `update_blur`, `jump_to` and `slide_to`, the `}` closing `impl Panel`, and `card_label`. Step 3 makes the one edit they need. This removes the fields `all`, `expanded`, `keyboard`, `filter`, `agents`, `armed`, `clear_armed` and `before_clear`, and the methods `release_keyboard`, `expand`, `pick`, `press`, `key`, `advance`, `disarm_unless_focused`, `disarm_remove`, `put_remove_back`, `clear_all`, `cancel_clear`, `disarm_clear`, `show_focused_row` and `card_of`.

```rust
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

use super::actions::{self, Action};
use super::blur::{self, Blur};
use super::keys;
use super::model::{Card, Filter, Status};
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
        let (tabs, tab_buttons, clear) = tab_bar();
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
            clear,
            slide: Rc::new(Slide::tucked()),
            state: RefCell::new(PanelState::default()),
            cards: RefCell::new(Vec::new()),
            drawing: Cell::new(false),
            spinning: Cell::new(false),
            frame: Cell::new(0),
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

    /// A click on a filter tab picks it; a click on Clear all presses it.
    fn connect_tab_bar(self: &Rc<Self>) {
        for (button, filter) in self.tab_buttons.iter().zip(Filter::TABS) {
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    let effects = p.state.borrow_mut().on_tab(filter);
                    p.apply(effects);
                }
            });
        }
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
            let action = keys::key_action(key, modifiers.contains(gdk::ModifierType::CONTROL_MASK));
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

    /// Show these cards, capped, or hide the panel when there are none.
    pub fn show(self: &Rc<Self>, cards: &[Card]) {
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
                    // Snapped back rather than slid, so whatever the keyboard
                    // opened does not wait on the cards crossing half the
                    // screen.
                    self.window.set_keyboard_mode(KeyboardMode::None);
                    self.jump_to(TUCKED_X, 0);
                }
                Effect::Spawn(args) => open_menu(&self.output, &args),
                Effect::DeleteAll(uuids) => delete_all(&self.output, &uuids),
                Effect::Notify(text) => crate::notify::tasks(&text),
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

    /// Which tabs show, which one is picked, and whether Clear all shows.
    fn update_tabs(&self) {
        let state = self.state.borrow();
        let shown = state.tabs();
        for (button, filter) in self.tab_buttons.iter().zip(Filter::TABS) {
            button.set_visible(shown.contains(&filter));
            set_class(button, "current", filter == state.filter());
        }
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
        let (shown, keyboard, empty, focus) = {
            let state = self.state.borrow();
            (state.visible(), state.keyboard(), state.empty_text(), state.focus().cloned())
        };

        self.while_drawing(|| {
            while let Some(child) = self.column.first_child() {
                self.column.remove(&child);
            }
            let cards: Vec<CardWidgets> = shown.iter().map(|s| self.card_widget(s, keyboard)).collect();
            for card in &cards {
                self.column.append(&card.root);
            }
            *self.cards.borrow_mut() = cards;
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

        if keyboard {
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
            button.set_tooltip_text(Some(action.label_on(up_next)));
            button.add_css_class(action.name());
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
                    action.label_on(row.up_next)
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
        self.cancel_grace();
        self.window.close();
    }
```

- [ ] **Step 3: Point `set_region` at the state**

In `set_region`, the one place in the kept code that read the old field:

```rust
        let width = region_width(x, self.state.borrow().keyboard());
```

- [ ] **Step 4: Replace the free helpers after `card_label`**

Replace `empty_line`, `clear_label`, `row_parts`, `slots` and `focus_card` (everything from the doc comment of `empty_line` down to the doc comment of `open_menu`) with the listing below. `row_parts`, `slots` and `focus_card` have no callers any more; the new `scroller` and `tab_bar` builders hold what `Panel::new` used to build inline.

```rust
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

/// The filter tabs and Clear all, over the scroller rather than in it, so
/// they stay put while the cards scroll. Ring room on three sides, as the
/// column keeps, and under them the card gap less the ring the column keeps
/// above the first card: the first card then sits a card gap below.
fn tab_bar() -> (gtk4::Box, Vec<gtk4::Button>, gtk4::Button) {
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
    (tabs, tab_buttons, clear)
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
```

In `mod tests`, add a test for Remove's face next to Clear all's:

```rust
    /// Remove's trash can alone, then, armed, the can and what it asks.
    #[test]
    fn remove_asks_once_armed() {
        assert_eq!(remove_label(false), Action::Remove.icon());
        assert_eq!(remove_label(true), format!("{}  Confirm remove", Action::Remove.icon()));
    }
```

- [ ] **Step 5: Remove `keys::Armed` and `keys::while_clear_armed`**

`PanelState::on_key` now does what they did, and its tests (`enter_confirms_clear_all_…`, `escape_and_moving_cancel_clear_all_…`, `a_tab_key_switches_tab_and_disarms_clear_all`, `a_cards_keys_do_nothing_while_clear_all_is_armed`) cover theirs. In `src/panel/keys.rs`, delete:

- `pub enum Armed` with its doc comment (from `/// What a keypress does while Clear all is armed.` through the enum's closing `}`),
- `pub fn while_clear_armed` with its doc comment,
- the tests `enter_and_ctrl_delete_confirm_an_armed_clear_all`, `escape_and_moving_cancel_an_armed_clear_all`, `the_tab_keys_still_switch_tab_while_armed` and `a_cards_keys_do_nothing_while_armed`, each with its doc comment.

Also in `keys.rs`, the module doc says acting on a key is `surface.rs`'s. Make it the state's:

```rust
//! What a keypress on the task panel means while it has the keyboard.
//!
//! Pure, as the task box's keys are, so the mapping is testable without a
//! window. What the key then does is `state.rs`'s; the controller's
//! propagation phase that lets it see the arrows first is `surface.rs`'s.
```

- [ ] **Step 6: Build, test, lint, and run the e2e test**

Run: `cargo test 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: every line `ok`, no `FAILED`.

Run: `cargo clippy --all-targets 2>&1 | grep -E '^(warning|error)'`
Expected: no output.

Run: `grep -n 'widget_name\|has_css_class' src/panel/surface.rs`
Expected: no output. Nothing is found by name any more.

Run: `cargo build && NIRITASKS=./target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -3`
Expected: the same `passed` / `failed` / `skipped` as Step 1, `failed: 0`. If one fails, run it again with `NIRITASKS_E2E_KEEP=1` to keep the frames, and use superpowers:systematic-debugging; don't trim the check.

- [ ] **Step 7: Commit**

```bash
git add src/panel/surface.rs src/panel/keys.rs
git commit -m "refactor(panel): draw the panel from PanelState

surface.rs becomes the GTK adapter: it draws what PanelState says, runs
the effects it hands back, and tells it when GTK's focus moves. Cards
are found through the widgets each render keeps, not by widget name or
CSS class. Panel::new, render and card_widget shrink, and the module
doc lists the g and b keys it missed. keys::while_clear_armed goes, as
PanelState::on_key covers it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Trim the e2e checks the unit tests took over

**Files:**
- Modify: `tests/e2e-panel.sh` (header comment; the keyboard section)
- Modify: `README.md:426-438` (what e2e-panel.sh measures)

**Interfaces:**
- Consumes: the Task 3 build.
- Produces: nothing new in code.

e2e-panel.sh stays as the check that GTK draws what the state says, and that what the state runs reaches taskwarrior. The 13 checks below go because a `state.rs` unit test now checks the same rule without a display:

| e2e check that goes | Unit test that covers it |
|---|---|
| 3 does nothing while Planned is hidden | `a_hidden_tabs_key_does_nothing` |
| `]` skips hidden tabs; stops at the end; `[` back to All (3 checks) | `the_brackets_skip_hidden_tabs_and_stop_at_the_ends` |
| taking the keyboard again opens on All | `the_keyboard_opens_on_all_every_time` |
| Ctrl+Enter leaves the same card focused | `ctrl_enter_refines_or_starts_keeping_the_keyboard_and_the_focus` |
| Ctrl+Enter on a planned card starts it; panel still up (2 checks) | the same test, second half |
| a card's key does nothing while Clear all is armed | `a_cards_keys_do_nothing_while_clear_all_is_armed` |
| Escape cancels Clear all, focus back | `escape_and_moving_cancel_clear_all_putting_the_focus_back` |
| switching tab disarms Clear all | `a_tab_key_switches_tab_and_disarms_clear_all` |
| the Waiting tab is gone after Clear all | `enter_confirms_clear_all_…` (filter All) and `a_tab_shows_only_while_it_has_tasks` |
| after expanding, the focus is on the ninth card | `down_onto_more_shows_the_rest_and_lands_on_the_first_it_hid` |

What stays: the peek and the stack; the keyboard moving the panel to the middle; a tab's fill drawing; Down moving the row and only the focused card showing one; Escape snapping back; Ctrl+Enter's refine reaching `task refine`; the Waiting tab; Clear all taking the focus off the cards, then Enter deleting through `task status` with a notification each, the keyboard kept; "+2 more" growing the panel; the empty panel; the cold start.

- [ ] **Step 1: Apply the trim**

Apply this patch to `tests/e2e-panel.sh` (save it to a file and `git apply` it, or make the edits by hand). Note the one change of flow in the Clear all section: the cancel and tab-switch checks are gone, so Clear all is still armed from the `clear_armed` check, and Enter confirms it directly. The second Ctrl+Delete before Enter goes too. Kept, it would confirm on its own, and Enter would then open the first card's menu.

```diff
diff --git a/tests/e2e-panel.sh b/tests/e2e-panel.sh
index 71b7d4a..6c737a3 100755
--- a/tests/e2e-panel.sh
+++ b/tests/e2e-panel.sh
@@ -33,9 +33,14 @@
 # out, the slide back and clicks passing beside the peek are checked by hand
 # (README, "Testing"). The keyboard it can: `task panel` moves the panel to the
 # middle of the screen, and with wtype, Down (which moves the action row, and
-# shows the rest on reaching "+N more"), the filter tabs' keys, Ctrl+Delete and
-# Enter for the Waiting tab's Clear all, and Escape are pressed in the nested
-# niri, never on your desktop.
+# shows the rest on reaching "+N more"), the filter tabs' keys, Ctrl+Enter,
+# Ctrl+Delete and Enter for the Waiting tab's Clear all, and Escape are
+# pressed in the nested niri, never on your desktop.
+#
+# What a key does to the panel's state (which tab, which card has the focus,
+# what is armed) is src/panel/state.rs's, and its unit tests check every rule
+# of it. This checks that GTK draws what the state says and that what it runs
+# reaches taskwarrior: one press of each kind, not every rule again.
 set -uo pipefail
 
 python3 -c "import PIL" 2>/dev/null || {
@@ -237,18 +242,8 @@ fi
 # frames is those two cards' columns, and the panel is as tall as it was.
 if command -v wtype >/dev/null; then
     # None of these tasks is started, planned or waiting, so only All and To
-    # refine have a tab. 3 is Planned's key, and its tab is hidden: nothing.
-    "${NENV[@]}" wtype 3
-    sleep 1
-    shot tab_hidden || { summary; exit 1; }
-    if same keyboard tab_hidden; then
-        ok "3 does nothing while no task is planned, its tab hidden"
-    else
-        bad "3 changed the panel with no planned task; Planned's tab should be hidden"
-    fi
-
-    # 4 is To refine: the same three cards, so only the tab bar changes, the
-    # picked tab's fill moving off All.
+    # refine have a tab. 4 is To refine: the same three cards, so only the
+    # tab bar changes, the picked tab's fill moving off All.
     "${NENV[@]}" wtype 4
     sleep 1
     shot tab_refine || { summary; exit 1; }
@@ -260,33 +255,8 @@ if command -v wtype >/dev/null; then
       nothing; more than the tab bar means the cards changed too"
     fi
 
-    # ] from All skips the hidden Active and Planned to To refine, and stops
-    # there, Waiting being hidden too; [ goes back to All.
     "${NENV[@]}" wtype 1
-    "${NENV[@]}" wtype -k bracketright
-    sleep 1
-    shot tab_right || { summary; exit 1; }
-    if same tab_refine tab_right; then
-        ok "] from All skips the hidden tabs to To refine"
-    else
-        bad "] from All did not land on To refine, the next tab shown"
-    fi
-    "${NENV[@]}" wtype -k bracketright
-    sleep 1
-    shot tab_end || { summary; exit 1; }
-    if same tab_refine tab_end; then
-        ok "] on the last tab shown stays there"
-    else
-        bad "] on To refine moved; it is the last tab shown and should stop"
-    fi
-    "${NENV[@]}" wtype -k bracketleft
     sleep 1
-    shot tab_left || { summary; exit 1; }
-    if same keyboard tab_left; then
-        ok "[ steps back to All, as the panel opened"
-    else
-        bad "[ from To refine is not All as the panel opened"
-    fi
 
     "${NENV[@]}" wtype -k Down
     sleep 1
@@ -335,17 +305,11 @@ if command -v wtype >/dev/null; then
 
     "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
     settle
-    shot reopened || { summary; exit 1; }
-    if same keyboard reopened; then
-        ok "taking the keyboard again opens on All, whatever tab it was left on"
-    else
-        bad "the keyboard reopened on something other than All with the first card focused"
-    fi
 
-    # Ctrl+Enter acts on the focused card (the first, as the panel opened) and
-    # leaves the panel as it was. The spawned refine stops at the missing herdr.
+    # Ctrl+Enter refines the focused card, the first as the panel opened. The
+    # spawned refine stops at the missing herdr.
     if PATH="$NESTED_SPAWN_PATH" command -v herdr >/dev/null; then
-        skip "Ctrl+Enter refining and starting (herdr is in $NESTED_SPAWN_PATH, so a refine would really open)"
+        skip "Ctrl+Enter refining (herdr is in $NESTED_SPAWN_PATH, so a refine would really open)"
     else
         refines() { cat "$SB/notifications" 2>/dev/null | grep -c "herdr is not installed"; }
         before=$(refines)
@@ -356,41 +320,6 @@ if command -v wtype >/dev/null; then
         else
             bad "Ctrl+Enter started no refine (notifications: $(tail -n 3 "$SB/notifications" 2>/dev/null | tr '\n' '|'))"
         fi
-        sleep 1
-        shot ctrl_enter || { summary; exit 1; }
-        if same reopened ctrl_enter; then
-            ok "and leaves the panel up with the same card focused, as it was"
-        else
-            read -r x0 x1 y0 y1 < <(measure ctrl_enter reopened)
-            bad "Ctrl+Enter changed the screen in columns ${x0}-${x1}, rows ${y0}-${y1}"
-        fi
-
-        # Once planned, the same key starts working. That is `task start`, which
-        # in this sandbox stops at once: ~/Projects/e2e is no git repository, and
-        # wt is not on the spawn PATH, so no worktree or herdr is ever touched.
-        if PATH="$NESTED_SPAWN_PATH" command -v wt >/dev/null; then
-            skip "Ctrl+Enter starting a planned task (wt is in $NESTED_SPAWN_PATH)"
-        else
-            first=$(grep -o "Refining: .*" "$SB/notifications" | tail -n 1 | sed 's/^Refining: //')
-            task rc.verbose=nothing rc.confirmation=no "+$TAG" "description.is:$first" \
-                modify +planned </dev/null >/dev/null 2>&1
-            settle
-            "${NENV[@]}" wtype -M ctrl -k Return -m ctrl
-            for _ in $(seq 1 50); do grep -q "Starting: $first" "$SB/notifications" 2>/dev/null && break; sleep 0.1; done
-            if grep -q "Starting: $first" "$SB/notifications" 2>/dev/null; then
-                ok "Ctrl+Enter on a planned card starts working on it"
-            else
-                bad "Ctrl+Enter on the planned '$first' did not start it (notifications: $(tail -n 3 "$SB/notifications" 2>/dev/null | tr '\n' '|'))"
-            fi
-            sleep 1
-            shot ctrl_enter_start || { summary; exit 1; }
-            read -r x0 x1 y0 y1 < <(measure ctrl_enter_start)
-            if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ]; then
-                ok "and the panel is still up in the middle of the screen"
-            else
-                bad "after Ctrl+Enter started a task the panel covers columns ${x0}-${x1}, not the middle"
-            fi
-        fi
     fi
 
     # Park one as waiting while the panel is open: All loses it and the
@@ -469,47 +398,11 @@ if command -v wtype >/dev/null; then
       card's row gone, and 2"
     fi
 
-    # Armed, a card's key has no card to act on: e opens no box.
-    "${NENV[@]}" wtype e
-    sleep 1
-    shot clear_armed_e || { summary; exit 1; }
-    if same clear_armed clear_armed_e; then
-        ok "a card's key does nothing while Clear all is armed"
-    else
-        bad "e changed the screen while Clear all was armed; it should do nothing"
-    fi
-
-    # Escape cancels: Clear all back, the focus back on the card it was on,
-    # and the panel still up.
-    "${NENV[@]}" wtype -k Escape
-    sleep 1
-    shot clear_cancelled || { summary; exit 1; }
-    if same clear_before clear_cancelled; then
-        ok "Escape puts Clear all back and the focus on its card, keeping the panel"
-    else
-        bad "after Escape on an armed Clear all the screen is not as it was before arming"
-    fi
-
-    # Moving away disarms it: off the tab and back is the frame from before.
-    "${NENV[@]}" wtype -M ctrl -k Delete -m ctrl
-    sleep 0.5
-    "${NENV[@]}" wtype 1
-    "${NENV[@]}" wtype 5
-    sleep 1
-    shot clear_disarmed || { summary; exit 1; }
-    if same clear_before clear_disarmed; then
-        ok "switching tab puts Clear all back"
-    else
-        bad "Clear all is still armed after switching tab and back"
-    fi
-
-    # Armed, Enter confirms: both waiting tasks deleted, one after the other,
+    # Still armed, Enter confirms: both waiting tasks deleted, one after the other,
     # each through task status; the task still on All and the other tag's are
     # left alone.
     notified_before=$(grep -c '^Tasks Deleted: ' "$SB/notifications" 2>/dev/null || true)
     notified_before=${notified_before:-0}
-    "${NENV[@]}" wtype -M ctrl -k Delete -m ctrl
-    sleep 0.5
     "${NENV[@]}" wtype -k Return
     for _ in $(seq 1 20); do
         [ "$(count "+$TAG" status:waiting)" = 0 ] && break
@@ -534,8 +427,7 @@ if command -v wtype >/dev/null; then
       run task status on each"
     fi
 
-    # The panel keeps the keyboard, back on All with the one task left, and
-    # the Waiting tab is gone, so 5 does nothing.
+    # The panel keeps the keyboard, back on All with the one task left.
     shot clear_after || { summary; exit 1; }
     read -r x0 x1 y0 y1 < <(measure clear_after)
     if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ] &&
@@ -545,14 +437,6 @@ if command -v wtype >/dev/null; then
         bad "after Clear all the panel covers columns ${x0}-${x1}, rows ${y0}-${y1} —
       reaching ${OUT_W} means it gave up the keyboard; 0-0 means it is gone"
     fi
-    "${NENV[@]}" wtype 5
-    sleep 1
-    shot clear_no_tab || { summary; exit 1; }
-    if same clear_after clear_no_tab; then
-        ok "and the Waiting tab is gone: 5 does nothing"
-    else
-        bad "5 changed the panel after Clear all; the Waiting tab should be hidden"
-    fi
     "${NENV[@]}" wtype -k Escape
     settle
 
@@ -582,18 +466,6 @@ if command -v wtype >/dev/null; then
       it should grow by the two cards \"+2 more\" hid; shorter means it focused
       \"+2 more\" instead of showing them"
     fi
-    # The focus is on the ninth card, not the last, so one more Down moves the
-    # buttons to the tenth: the screen changes, the panel as tall as before.
-    "${NENV[@]}" wtype -k Down
-    sleep 1
-    shot long_tenth || { summary; exit 1; }
-    read -r x0 x1 y0 y1 < <(measure long_tenth)
-    if ! same long_more long_tenth && [ "$((y1 - y0))" -eq "$more_h" ]; then
-        ok "and focuses the first card it hid, so Down goes on to the next"
-    else
-        bad "after showing every card, Down changed nothing or the panel's height
-      (${more_h}px to $((y1 - y0))px) — the focus was not on the ninth card"
-    fi
     "${NENV[@]}" wtype -k Escape
     settle
 else
```

Then rewrap the one comment the patch leaves over 80 columns:

```bash
    # Still armed, Enter confirms: both waiting tasks deleted, one after the
    # other, each through task status; the task still on All and the other
    # tag's are left alone.
```

and say why the bare `wtype 1` that's left is there:

```bash
    # Back to All for Down.
    "${NENV[@]}" wtype 1
    sleep 1
```

- [ ] **Step 2: Run the trimmed test**

Run: `cargo build && NIRITASKS=./target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -3`
Expected: `passed: 24   failed: 0   skipped: 0` (with wtype and no herdr on the spawn PATH; fewer passes and more skips otherwise, never a failure).

- [ ] **Step 3: Update the README's account of the test**

In `README.md`, replace the sentence that starts `So it` and runs to `cold-started with nothing to show.` (around lines 431-438) with:

```markdown
So it
screenshots a nested niri of its own and measures the panel against a frame taken
with no tasks: the peek's width, the stack's height as tasks are added, the
keyboard's cards in the middle of the screen and the focused one's buttons, a
filter tab's fill, a waiting task on the Waiting tab and off the tucked panel,
Clear all taking the focus off the cards on its first Ctrl+Delete and Enter then
deleting both waiting tasks and nothing else, "+N more" opening on Down, Escape
putting them back, nothing for another tag's task, nothing once they are done,
and a daemon cold-started with nothing to show. Which tab, card and button a key
leads to, and what it arms, are `src/panel/state.rs`'s unit tests, which need no
display.
```

- [ ] **Step 4: Commit**

```bash
git add tests/e2e-panel.sh README.md
git commit -m "test(panel): drop the e2e checks the PanelState tests cover

Thirteen keyboard checks, and the sleeps and frames they took, now run
as unit tests of PanelState. The e2e keeps the checks that GTK draws
what the state says and that what it runs reaches taskwarrior.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Hand the manual check to the user**

The hover can't be scripted. When the branch is done, ask the user for README's manual check ("Testing", the paragraph that starts "It cannot move the pointer"), on a daemon running this build: the peek slides out and back, clicks beside the peek pass through, a click on a card opens its menu, a click on "+N more" shows the rest, and on the keyboard panel a mouse click on Remove arms it and a click elsewhere disarms it.
