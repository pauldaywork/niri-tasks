# Peek Only the Focused herdr Pane's Task Card Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While working in herdr, the task panel's tucked-away edge shows only the task whose herdr pane is selected, as one whole card (description, id line, notes, action row) that slides out under the pointer and never takes the keyboard, instead of a one-line list of every task.

**Architecture:** herdr's `task-docs` plugin gets `[[events]]` hooks on `pane.focused`, `tab.focused` and `workspace.focused`. Each runs `task_docs.py focused`, which asks herdr for the session's focused pane, finds its task the way the plugin's actions already do, and runs `niritasks task peek <uuid8>` (or `--clear`). The CLI resolves the herdr session's workspace tag and the full uuid and sends a new one-line IPC request, `peek <tag> [<uuid>]`. The daemon keeps one peeked uuid per workspace tag and hands it to the panel showing that tag; `PanelState` draws, off the keyboard, that card alone, whole, with its buttons working by click. Mod+Alt+Ctrl+T's keyboard panel is unchanged.

**Tech Stack:** Rust (gtk4, gtk4-layer-shell, clap, anyhow), Python 3 (the herdr plugin, `unittest`), bash (the nested-niri e2e test), herdr 0.9.3, Taskwarrior 3.5.

**Spec:** Taskwarrior task `cbb49ff3-5664-43fa-90a3-d3cf9dc3562a` — its description and notes (`task rc.json.array=on cbb49ff3 export`). No separate spec file. One gap the notes left open was settled with the user while planning (2026-10-26): on the peeked card, a click on **Go to…** or **Move to workspace** hands the panel the keyboard on that card and opens that list, since both lists are driven by the keyboard; every other button runs and leaves herdr the keyboard.

## Global Constraints

- Mod+Alt+Ctrl+T (`niritasks task panel`) opens the full list with its filter tabs exactly as now; herdr's `task-docs.panel` action is unchanged.
- The peek never takes the keyboard on its own. Selecting a pane, tab or workspace never does; only a click on the peeked card's Go to… or Move to workspace does.
- Selecting a pane, a tab or a workspace in herdr all count, each resolved to the now-focused pane; a repeat of the same pane is ignored; a focused pane linked to no task clears the edge to nothing.
- `niritasks task peek` works only from a named herdr session (`HERDR_SESSION`, or a `…/sessions/<name>/herdr.sock` `HERDR_SOCKET_PATH`). It never falls back to the folder or the focused workspace: herdr runs event hooks with the plugin's folder as cwd, which would name the wrong workspace.
- A herdr session that is not a niri-tasks workspace's shows nothing (out of scope).
- The plugin's focus hook never sends a notification: it fires on every pane switch. Failures go to stderr, which herdr keeps in `herdr plugin log list --plugin task-docs`.
- The plugin manifest's `min_herdr_version` becomes `"0.9.3"`, the herdr whose API schema was checked for `pane_focused`, `tab_focused` and `workspace_focused`.
- Vocabulary is CONTEXT.md's: **Peek**, **Task panel**, **Task card**, **Action row**. Avoid "sliver", "handle", "hover card".
- Comments and doc comments follow the surrounding code: full sentences, why over what, the same density.
- Commits are Conventional Commits (`feat(panel): …`), subject ≤ 72 characters, body wrapped at 72, ending with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Tests: `cargo test`; `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs`; `NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh` (without `NIRITASKS` it tests the installed binary, not this worktree).
- Never send a request to the real daemon from this worktree's build: `target/debug/niritasks task peek` in a real herdr pane would reach the user's running (older) daemon. Check the CLI's refusal with `env -u HERDR_SESSION -u HERDR_SOCKET_PATH`, and everything else in the nested niri of `tests/e2e-panel.sh`.

---

## File Structure

- `src/panel/state.rs` — `PanelState` gains `peek: Option<String>`, `set_peek`, `row_card`, `take_keyboard_for`; `visible`, `hidden`, `shows_notes`, `on_press`, `on_tuck` change for the tucked-away panel; `Effect::TakeKeyboard`. All rules unit-tested here.
- `src/panel/surface.rs` — draws every card whole (no one-line mode), shows the action row on `row_card`, runs `Effect::TakeKeyboard`, `show(name, cards, peek)`, plus `peek` and `shows` for the daemon.
- `src/link.rs` — `live_agents_named`, moved from `daemon.rs` so the surface can ask for the session's agents too.
- `src/daemon.rs` — passes each panel its workspace name and peek; keeps `PEEKS` per tag; serves `Request::Peek`.
- `src/ipc.rs` — `Request::Peek { tag, uuid }`, encoded `peek <tag> [<uuid>]`.
- `src/main.rs` — `niritasks task peek <uuid> | --clear`.
- `tests/e2e-panel.sh` — the tucked panel is now the peek: the sections that measured a one-line stack measure the peeked card instead.
- `herdr/task-docs/herdr-plugin.toml`, `herdr/task-docs/task_docs.py`, `herdr/task-docs/tests/test_task_docs.py` — the three focus hooks and `focused`.
- `CONTEXT.md`, `README.md`, `llms.txt` — Peek, Task panel, Task card, Action row; the plugin's hooks; the new command.

---

### Task 1: The tucked-away panel shows the peeked card alone

**Files:**
- Modify: `src/panel/state.rs` (`Effect`, `Mode` doc, `Shown` doc, `PanelState` field and methods, tests)
- Modify: `src/panel/surface.rs` (module doc, `Panel` struct, `show`, `apply`, `render`, `draw_tasks`, `card_widget`, `sync`, `card_label`)
- Modify: `src/link.rs` (add `live_agents_named`)
- Modify: `src/daemon.rs` (`live_agents` removed, `tick` calls the new `show`)

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces:
  - `PanelState::set_peek(&mut self, uuid: Option<&str>) -> Vec<Effect>`
  - `PanelState::row_card(&self) -> Option<&str>`
  - `PanelState::take_keyboard_for(&mut self, uuid: &str, action: Action, agents: Vec<String>) -> Vec<Effect>`
  - `Effect::TakeKeyboard { uuid: String, action: Action }`
  - `Panel::show(self: &Rc<Self>, name: Option<&str>, cards: &[Card], peek: Option<&str>)`
  - `crate::link::live_agents_named(name: Option<&str>) -> Vec<String>`

- [ ] **Step 1: Write the failing state tests**

In `src/panel/state.rs`'s `mod tests`, under `// ─── what shows ───`, replace the whole test `the_hover_shows_alls_cards_with_no_tabs_or_buttons` (from its `#[test]` to its closing brace) with:

```rust
    /// Tucked away, the panel shows the peeked card alone, its buttons on and
    /// its notes shown, with no tabs and no focus; nothing peeked shows
    /// nothing.
    #[test]
    fn tucked_away_only_the_peeked_card_shows_with_its_buttons() {
        let mut state = PanelState::default();
        state.set_cards(&[noted("a"), card("b", Status::Pending)]);
        assert!(state.hidden(), "nothing peeked, nothing on the edge");
        assert!(state.visible().is_empty());
        assert_eq!(state.set_peek(Some("b")), vec![Effect::Render]);
        assert!(!state.hidden());
        assert_eq!(uuids(&state), vec!["b"]);
        let shown = &state.visible()[0];
        assert_eq!(shown.actions, Action::row(shown.state.expect("a task's card has a state")));
        assert!(!shown.actions.is_empty());
        assert!(state.tabs().is_empty());
        assert_eq!(state.focus(), None);
        assert_eq!(state.row_card(), Some("b"));
        state.set_peek(Some("a"));
        assert!(state.shows_notes("a"));
        assert!(!state.shows_notes("b"));
    }

    /// The three focus events one herdr switch can fire draw once; a clear
    /// hides the panel, and a second clear does nothing.
    #[test]
    fn the_same_peek_again_changes_nothing() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a"]));
        state.set_peek(Some("a"));
        assert_eq!(state.set_peek(Some("a")), Vec::new());
        assert_eq!(state.set_peek(None), vec![Effect::Render]);
        assert!(state.hidden());
        assert_eq!(state.set_peek(None), Vec::new());
    }

    /// A peeked task not under All shows nothing: waiting, finished, or on
    /// another workspace. Back on the list, it shows again.
    #[test]
    fn a_peeked_task_off_all_shows_nothing() {
        let mut state = PanelState::default();
        state.set_cards(&[card("a", Status::Waiting), card("f", Status::Finished)]);
        for uuid in ["a", "f", "elsewhere"] {
            state.set_peek(Some(uuid));
            assert!(state.hidden(), "{uuid}");
            assert_eq!(state.row_card(), None, "{uuid}");
        }
        state.set_peek(Some("a"));
        state.set_cards(&pending(&["a"]));
        assert!(!state.hidden(), "Back to list puts the peek back");
    }

    /// Mod+Alt+Ctrl+T is as it was: every card, the focus on the first. A
    /// peek set meanwhile redraws nothing, and shows once the keyboard is
    /// given back.
    #[test]
    fn the_keyboard_shows_every_card_and_keeps_the_peek_for_after() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b"]));
        state.set_peek(Some("b"));
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(uuids(&state), vec!["a", "b"]);
        assert_eq!(state.row_card(), Some("a"), "the focused card's row");
        assert_eq!(state.set_peek(Some("a")), Vec::new(), "nothing redraws under the keyboard");
        key(&mut state, KeyAction::Release);
        assert_eq!(uuids(&state), vec!["a"]);
        assert_eq!(state.row_card(), Some("a"));
    }

    /// The peek's buttons run as they do with the keyboard, which stays
    /// herdr's: Complete and Start working spawn at once, Edit's box will
    /// not hand the keyboard to the panel as it closes, and Remove arms
    /// first, with no focus to move.
    #[test]
    fn the_peeks_buttons_run_by_click_without_the_keyboard() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a"]));
        state.set_peek(Some("a"));
        for action in [Action::Complete, Action::Start, Action::Edit] {
            assert_eq!(state.on_press("a", Slot::Button(action)), vec![Effect::Spawn(action.args("a"))], "{action:?}");
        }
        assert!(!state.returns_to("a"), "Edit's box closing leaves herdr the keyboard");
        assert_eq!(state.on_press("a", Slot::Button(Action::Remove)), Vec::new(), "armed, no focus to move");
        assert_eq!(state.armed(), &Armed::Remove("a".into()));
        assert_eq!(state.on_press("a", Slot::Button(Action::Remove)), vec![Effect::Spawn(Action::Remove.args("a"))]);
        assert!(!state.keyboard());
    }

    /// Go to… and Move to workspace on the peek open lists the keyboard
    /// drives: the click asks for the keyboard on that card, and taking it
    /// presses the button there, after a render of the keyboard's cards.
    #[test]
    fn the_peeks_go_to_and_move_take_the_keyboard_on_its_card() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b"]));
        state.set_peek(Some("b"));
        for action in [Action::GoTo, Action::Move] {
            assert_eq!(state.on_press("b", Slot::Button(action)), vec![Effect::TakeKeyboard { uuid: "b".into(), action }]);
            assert!(!state.keyboard(), "only asked for");
        }
        assert_eq!(
            state.take_keyboard_for("b", Action::GoTo, Vec::new()),
            vec![Effect::Render, Effect::ListGoTo("b".into())]
        );
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
    }

    /// An answer that comes late finds the peek moved on, or the keyboard
    /// already taken: nothing.
    #[test]
    fn taking_the_keyboard_for_a_peek_that_left_does_nothing() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b"]));
        state.set_peek(Some("b"));
        state.set_peek(Some("a"));
        assert_eq!(state.take_keyboard_for("b", Action::GoTo, Vec::new()), Vec::new());
        assert!(!state.keyboard());
        state.set_peek(Some("b"));
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.take_keyboard_for("b", Action::Move, Vec::new()), Vec::new());
    }

    /// The pointer leaving, or the peek moving to another task, disarms a
    /// Remove armed on the peek.
    #[test]
    fn tucking_away_or_a_new_peek_disarms_the_peeks_remove() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b"]));
        state.set_peek(Some("a"));
        state.on_press("a", Slot::Button(Action::Remove));
        assert_eq!(state.on_tuck(), Vec::new());
        assert_eq!(state.armed(), &Armed::None);
        state.on_press("a", Slot::Button(Action::Remove));
        state.set_peek(Some("b"));
        assert_eq!(state.armed(), &Armed::None);
    }
```

Delete the test `finished_tasks_are_off_the_hover` (with the doc comment above its `#[test]`): `a_peeked_task_off_all_shows_nothing` covers it.

Delete the test `tucking_away_folds_an_expanded_list`: the tucked-away panel has no "+N more" any more.

Replace the test `the_peek_shows_no_notes` (with its doc comment) with:

```rust
    /// The peeked card always shows its notes: a click on its body neither
    /// hides them nor takes the keyboard.
    #[test]
    fn a_click_on_the_peeks_body_changes_nothing() {
        let mut state = PanelState::default();
        state.set_cards(&[noted("a")]);
        state.set_peek(Some("a"));
        assert_eq!(state.on_press("a", Slot::Body), Vec::new());
        assert!(state.shows_notes("a"));
        assert!(!state.keyboard());
    }
```

In `escape_gives_the_keyboard_back_on_all_folded_up`, replace its doc comment line

```rust
    /// Escape: one line per card again, the hover's All, on the right edge.
```

with

```rust
    /// Escape: All for the next time, nothing expanded or focused, and the
    /// peek back on the right edge.
```

and insert `        state.set_peek(Some("p"));` on the line before `        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);` in that test.

- [ ] **Step 2: Run the tests to see them fail to build**

Run: `cargo test --lib panel::state 2>&1 | tail -20`
Expected: compile errors — no method `set_peek`, `row_card`, `take_keyboard_for`; no variant `Effect::TakeKeyboard`.

- [ ] **Step 3: Implement the state**

In `src/panel/state.rs`:

Append to the module doc (after the line ending `take the keyboard back to, lives here too.`):

```rust
//! So does the peek: which task herdr's focused pane is on, whose card alone
//! the panel shows tucked away.
```

In `pub struct Shown`'s doc, replace `/// row, left to right. None off the keyboard, or on "+N more".` with `/// row, left to right. None on "+N more".`

In `pub enum Effect`, after the `Notes` variant, add:

```rust
    /// Take the keyboard on this task's card and press this button there:
    /// Go to… or Move to workspace clicked on the peek, whose lists the
    /// keyboard drives. The surface asks herdr for the session's live agents
    /// first, as the daemon does for Mod+Alt+Ctrl+T, and hands them to
    /// [`PanelState::take_keyboard_for`].
    TakeKeyboard { uuid: String, action: Action },
```

In `pub enum Mode`'s doc, replace

```rust
/// Without the keyboard it is always on Tasks: the hover and the peek are
/// the task cards.
```

with

```rust
/// Without the keyboard it is always on Tasks: the peek is a task card.
```

In `pub struct PanelState`, after the `returning` field, add:

```rust
    /// The task herdr's focused pane is on, by uuid, whose card alone the
    /// panel shows tucked away: the peek. None peeks at nothing. Set by the
    /// daemon from `niritasks task peek`, and kept while the keyboard is
    /// taken and given back.
    peek: Option<String>,
```

Replace `shows_notes` with:

```rust
    /// This task's card shows its notes: with the keyboard, the card whose
    /// body was pressed; tucked away, the peeked card, always.
    pub fn shows_notes(&self, uuid: &str) -> bool {
        if self.keyboard {
            self.notes.as_deref() == Some(uuid)
        } else {
            self.peeked().is_some_and(|c| c.uuid.as_deref() == Some(uuid))
        }
    }

    /// The card whose action row shows: the focused one with the keyboard,
    /// the peeked one tucked away.
    pub fn row_card(&self) -> Option<&str> {
        if self.keyboard {
            self.focus.as_ref().map(|f| f.uuid.as_str())
        } else {
            self.peeked().and_then(|c| c.uuid.as_deref())
        }
    }

    /// The peeked task's card, while its task is under All: pending on this
    /// workspace, neither waiting nor finished. None otherwise, and the
    /// tucked-away panel then shows nothing.
    fn peeked(&self) -> Option<&Card> {
        let uuid = self.peek.as_deref()?;
        self.all.iter().find(|c| c.uuid.as_deref() == Some(uuid) && Filter::All.matches(c))
    }
```

Replace `hidden` (doc comment included) with:

```rust
    /// Nothing to show, so the panel hides. Tucked away it shows the peeked
    /// card alone, so it hides with nothing peeked, or with a peeked task
    /// that is not under All. The keyboard's panel still has the Waiting and
    /// Finished tabs, and hides only with no task at all. On Ideas it never
    /// hides: the notepad is there with no task too. Nor on the project
    /// list, which Mod+Alt+W opens with no task.
    pub fn hidden(&self) -> bool {
        if self.keyboard {
            self.all.is_empty() && self.on_tasks()
        } else {
            self.peeked().is_none()
        }
    }
```

Replace `visible` (doc comment included) with:

```rust
    /// The cards to draw, top to bottom. With the keyboard, the picked tab's,
    /// filtered before the cap so "+N more" is the rest of this tab, and
    /// uncapped once expanded. Tucked away, the peeked card alone, with its
    /// buttons: they are clicked while herdr keeps the keyboard, and with no
    /// agents asked for, its state knows of no live Claude. None on Ideas,
    /// which is a notepad, not cards, nor on the project list, which draws
    /// folders.
    pub fn visible(&self) -> Vec<Shown> {
        if !self.on_tasks() {
            return Vec::new();
        }
        if !self.keyboard {
            return self.peeked().map(|card| shown_card(card.clone(), &[])).into_iter().collect();
        }
        let picked = self.filter.pick(&self.all);
        let cards = if self.expanded { picked } else { model::cap(&picked, model::CAP) };
        cards.into_iter().map(|card| shown_card(card, &self.agents)).collect()
    }
```

After `take_keyboard_on`, add:

```rust
    /// herdr's focused pane is on this task, or on none: the card the panel
    /// shows tucked away. Nothing when it is the task already peeked at, so
    /// the focus events one switch fires draw once. With the keyboard nothing
    /// redraws: the peek shows once the keyboard is given back. Tucked away
    /// the card redraws, and a Remove armed on the old one is disarmed.
    pub fn set_peek(&mut self, uuid: Option<&str>) -> Vec<Effect> {
        if self.peek.as_deref() == uuid {
            return Vec::new();
        }
        self.peek = uuid.map(String::from);
        if self.keyboard {
            return Vec::new();
        }
        self.armed = Armed::None;
        vec![Effect::Render]
    }

    /// A click on the peek's Go to… or Move to workspace: take the keyboard
    /// on that card, as herdr's task-docs.panel does, and press the button
    /// there, its list needing the keyboard. A render of the keyboard's cards
    /// first, then the press's own effects. Nothing once the peek has moved
    /// off that card or left All, and nothing with the keyboard already
    /// taken. `agents` are as `take_keyboard`'s.
    pub fn take_keyboard_for(&mut self, uuid: &str, action: Action, agents: Vec<String>) -> Vec<Effect> {
        if self.keyboard || self.peeked().and_then(|c| c.uuid.as_deref()) != Some(uuid) {
            return Vec::new();
        }
        self.take_keyboard_on(uuid, agents);
        let mut effects = vec![Effect::Render];
        effects.extend(self.on_press(uuid, Slot::Button(action)));
        effects
    }
```

Replace `on_tuck` (doc comment included) with:

```rust
    /// The pointer left and the peek slid back: a Remove armed on it is
    /// disarmed, so the next click starts from Remove again. Its label comes
    /// back with the sync that follows every effect the surface runs.
    pub fn on_tuck(&mut self) -> Vec<Effect> {
        if !self.keyboard {
            self.armed = Armed::None;
        }
        Vec::new()
    }
```

In `on_press`, append to its doc comment (after `disarms and asks for its list.`):

```rust
    /// Tucked away, the peek's buttons are clicked while herdr keeps the
    /// keyboard, and run as they would with it, but for Go to… and Move to
    /// workspace, which take the keyboard on the card first.
```

In `on_press`'s body, right after the `let action = match slot { … };` block, insert:

```rust
        // Tucked away, Go to… and Move to workspace open lists the keyboard
        // drives: the surface takes it on this card, then presses the button
        // again from there.
        if !self.keyboard && matches!(action, Action::GoTo | Action::Move) {
            self.armed = Armed::None;
            return vec![Effect::TakeKeyboard { uuid: uuid.into(), action }];
        }
```

and replace the Remove-arming block

```rust
        if action == Action::Remove && self.armed != Armed::Remove(uuid.into()) {
            // Focus first: the move disarms whatever was armed before, and
            // this one is armed only after it.
            let effects = self.focus_on(Focus { uuid: uuid.into(), slot });
            self.armed = Armed::Remove(uuid.into());
            return effects;
        }
```

with

```rust
        if action == Action::Remove && self.armed != Armed::Remove(uuid.into()) {
            // Focus first: the move disarms whatever was armed before, and
            // this one is armed only after it. Tucked away there is no focus
            // to move: the peek's buttons are clicked, never focused.
            let effects = if self.keyboard { self.focus_on(Focus { uuid: uuid.into(), slot }) } else { Vec::new() };
            self.armed = Armed::Remove(uuid.into());
            return effects;
        }
```

After the free function `first_task` (above `#[cfg(test)]`), add:

```rust
/// A card as drawn: its buttons and its state, from the session's live
/// agents. "+N more" has neither.
fn shown_card(card: Card, agents: &[String]) -> Shown {
    let state = card.state(agents);
    let actions = state.map(Action::row).unwrap_or_default();
    Shown { card, actions, state }
}
```

- [ ] **Step 4: Handle the new effect in the surface, and draw every card whole**

In `src/link.rs`, after `live_agent_names`, add:

```rust
/// The live herdr agents in the session of the workspace named `name`: one
/// `herdr agent list`, for every card at once. None for an unnamed
/// workspace, or a session that is not running, which answers at once with
/// none.
pub fn live_agents_named(name: Option<&str>) -> Vec<String> {
    name.and_then(|n| Workspace::named(n).ok()).map(|w| live_agent_names(&w)).unwrap_or_default()
}
```

In `src/daemon.rs`, delete the function `live_agents` with its doc comment, and change its two callers: `let agents = live_agents(focused.name.as_deref());` becomes `let agents = crate::link::live_agents_named(focused.name.as_deref());`, and `panel.take_keyboard_back(uuid, live_agents(name.as_deref()));` becomes `panel.take_keyboard_back(uuid, crate::link::live_agents_named(name.as_deref()));`. In `tick`, replace

```rust
        let name = outputs.get(connector).cloned().flatten();
        // The panel's tag, for the notepad its Ideas tab opens.
        let t = tag::workspace_tag(name.as_deref().unwrap_or_default());
        let cards = by_name
            .entry(name.clone())
            .or_insert_with(|| cards_for_workspace(name.as_deref()));
        panel.show(&t, cards);
```

with

```rust
        let name = outputs.get(connector).cloned().flatten();
        let cards = by_name
            .entry(name.clone())
            .or_insert_with(|| cards_for_workspace(name.as_deref()));
        panel.show(name.as_deref(), cards, None);
```

If `tag` is now unused among `daemon.rs`'s imports, leave it: `cards_for_workspace` still calls `tag::workspace_tag`.

In `src/panel/surface.rs`:

In the module doc, replace

```rust
//! Mod+Alt+Ctrl+T hands the panel the keyboard in the middle of the screen,
//! and every card lays itself out again: its whole description, wrapped, and,
//! on the focused card alone, a row of buttons for the task's actions
//! under it, so the list stays short enough to scan.
```

with

```rust
//! Mod+Alt+Ctrl+T hands the panel the keyboard in the middle of the screen,
//! with every pending card of the workspace, each its whole description,
//! wrapped, and, on the focused card alone, a row of buttons for the task's
//! actions under it, so the list stays short enough to scan.
```

replace

```rust
//! Escape, or anything that runs, hands the keyboard back, folds the cards to
//! one line again, and puts the panel back on the right edge as a peek. The
```

with

```rust
//! Escape, or anything that runs, hands the keyboard back and puts the panel
//! back on the right edge, on the peek. The
```

and, before `//! ## The keyboard`, add:

```rust
//! ## The peek
//!
//! Tucked away, the panel shows one card at most: the task herdr's focused
//! pane is on, which the daemon hands `show` (`PanelState::visible`). It is
//! drawn whole, as the keyboard draws a card, its id line and notes shown
//! and its action row on, so sliding it out shows everything at once; only
//! its left `PEEK_PX` shows until then. Its buttons are clicked while herdr
//! keeps the keyboard, the surface staying off keyboard focus. Go to… and
//! Move to workspace open lists the keyboard drives, so a click on either
//! takes the keyboard on that card first (`Effect::TakeKeyboard`) and
//! presses the button from there.
//!
```

In `pub struct Panel`, after the `tag` field, add:

```rust
    /// The name of the workspace this panel shows, for the herdr session
    /// whose agents a click on the peek's Go to… or Move to workspace asks
    /// for.
    name: RefCell<Option<String>>,
```

and in `Panel::new`'s `Rc::new(Panel { … })`, after `tag: RefCell::new(String::new()),`, add `            name: RefCell::new(None),`.

In `struct CardWidgets`, change the `row` field's doc to `/// None on "+N more", and on a card with no buttons: a processing one.` and the `notes` field's doc to:

```rust
    /// The task's id line and notes under the description, shown while the
    /// state says. None on "+N more".
```

Replace `show` (doc comment included) with:

```rust
    /// Show the cards of the workspace named `name`, and `peek`, the task
    /// herdr's focused pane is on, as the tucked-away panel's one card; or
    /// hide the panel when there is nothing to show. A new workspace moves
    /// the notepad: what was typed is saved to the tag it was typed for,
    /// and, with the keyboard still held, the new tag's ideas load at once;
    /// without it they load when the panel next takes it.
    pub fn show(self: &Rc<Self>, name: Option<&str>, cards: &[Card], peek: Option<&str>) {
        let tag = crate::tag::workspace_tag(name.unwrap_or_default());
        *self.name.borrow_mut() = name.map(String::from);
        if self.tag.borrow().as_str() != tag {
            *self.tag.borrow_mut() = tag.clone();
            // Edit's box was for the old workspace's card.
            self.state.borrow_mut().forget_return();
            if self.state.borrow().keyboard() {
                self.notepad.load(&tag);
            } else {
                self.notepad.flush();
            }
        }
        let mut effects = self.state.borrow_mut().set_peek(peek);
        effects.extend(self.state.borrow_mut().set_cards(cards));
        // Both may ask for a render; one does.
        effects.dedup();
        self.apply(effects);
    }
```

In `apply`, after the `Effect::Notes => { … }` arm, add:

```rust
                Effect::TakeKeyboard { uuid, action } => {
                    // As the daemon does for Mod+Alt+Ctrl+T: the session's
                    // live agents, for the Go to list's sessions.
                    let agents = crate::link::live_agents_named(self.name.borrow().as_deref());
                    let effects = self.state.borrow_mut().take_keyboard_for(&uuid, action, agents);
                    if !effects.is_empty() {
                        self.centre(effects);
                    }
                }
```

In `render`, change `Mode::Tasks => self.draw_tasks(&shown, keyboard, empty),` to `Mode::Tasks => self.draw_tasks(&shown, empty),`.

Replace `draw_tasks`'s signature and its `map` line:

```rust
    fn draw_tasks(self: &Rc<Self>, shown: &[Shown], empty: Option<&str>) {
        self.clear_column();
        let cards: Vec<CardWidgets> = shown.iter().map(|s| self.card_widget(s)).collect();
```

In `card_widget`, replace its doc comment and signature with:

```rust
    /// One card: a box holding the body, a button so the keyboard can focus
    /// and press it, and its action row along the bottom, shown on the card
    /// `PanelState::row_card` names: the focused one with the keyboard, the
    /// peek tucked away. The body shows or hides the task's notes on a
    /// task's card while the panel has the keyboard, and shows the rest in
    /// place of "+N more".
    fn card_widget(self: &Rc<Self>, shown: &Shown) -> CardWidgets {
```

and in its body change `let (label, age) = card_label(card, keyboard);` to `let (label, age) = card_label(card);`, and

```rust
        let notes = card.uuid.as_deref().zip(card.id_line()).filter(|_| keyboard).map(|(uuid, id_line)| {
```

to

```rust
        let notes = card.uuid.as_deref().zip(card.id_line()).map(|(uuid, id_line)| {
```

In `sync`, replace

```rust
        let (focus, armed) = {
            let state = self.state.borrow();
            (state.focus().cloned(), state.armed().clone())
        };
```

with

```rust
        let (row, armed) = {
            let state = self.state.borrow();
            (state.row_card().map(String::from), state.armed().clone())
        };
```

and its last-but-one statement `self.show_row(focus.as_ref().map(|f| f.uuid.as_str()));` with `self.show_row(row.as_deref());`.

Replace `card_label` (doc comment included) with:

```rust
/// The card's icon, its whole description, wrapped, and its age. The age
/// label comes back too, for the minute timer to update.
///
/// The icon is a label of its own beside the text, so wrapped lines start
/// under the first line's text rather than back under the icon. The age is
/// one too, at the right end, level with the first line.
fn card_label(card: &Card) -> (gtk4::Box, Option<gtk4::Label>) {
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
    text.set_wrap(true);
    // WordChar: a long path or URL with no spaces still breaks.
    text.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
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
```

- [ ] **Step 5: Run the tests**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked|error" | head -30`
Expected: every `test result: ok`; no `FAILED`.

Run: `cargo build 2>&1 | grep -A4 '^warning'`
Expected: no output. (`render`'s `keyboard` binding stays in use by `tabs.set_visible` and `focus_on`; an unused-variable warning there means a call was missed.)

- [ ] **Step 6: Commit**

```bash
git add src/panel/state.rs src/panel/surface.rs src/link.rs src/daemon.rs
git commit -m "$(cat <<'EOF'
feat(panel): show only the peeked card when tucked away

The tucked-away panel drops its one-line list of every task and shows
one card at most: the task herdr's focused pane is on, whole, with its
notes and action row, whose buttons work by click while herdr keeps
the keyboard. Go to… and Move to workspace take the keyboard on that
card first, since their lists need it. Nothing sets the peek yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: `niritasks task peek` asks the daemon to peek

**Files:**
- Modify: `src/ipc.rs` (module doc, `Request::Peek`, encode, decode, tests)
- Modify: `src/daemon.rs` (`PEEKS`, `remember_peek`, `serve_request`, `tick`, test)
- Modify: `src/panel/surface.rs` (`Panel::peek`, `Panel::shows`)
- Modify: `src/main.rs` (`TaskCommand::Peek`, dispatch, `session` import, CLI test)

**Interfaces:**
- Consumes: `Panel::show(name, cards, peek)` and `PanelState::set_peek` from Task 1.
- Produces:
  - `ipc::Request::Peek { tag: String, uuid: Option<String> }`, on the wire `peek <tag> <uuid>` / `peek <tag>`
  - `niritasks task peek <uuid>` and `niritasks task peek --clear`, exit 1 outside a named herdr session, with no daemon, or for an unknown task
  - `Panel::peek(self: &Rc<Self>, uuid: Option<&str>)`, `Panel::shows(&self, tag: &str) -> bool`

- [ ] **Step 1: Write the failing tests**

In `src/ipc.rs`'s tests, add:

```rust
    /// A peek names the workspace tag it is for, then the task, or none to
    /// peek at nothing; with no tag it is no request.
    #[test]
    fn peek_carries_a_tag_and_an_optional_uuid() {
        let peek = |uuid: Option<&str>| Request::Peek { tag: "e2e".into(), uuid: uuid.map(String::from) };
        assert_eq!(peek(Some("abc")).encode(), "peek e2e abc");
        assert_eq!(peek(None).encode(), "peek e2e");
        assert_eq!(Request::decode("peek e2e abc\n"), Some(peek(Some("abc"))));
        assert_eq!(Request::decode("peek e2e\n"), Some(peek(None)));
        assert_eq!(Request::decode("peek\n"), None);
        assert_eq!(Request::decode("peek \n"), None);
    }
```

and in `round_trips_every_request`'s array, after `Request::Projects,`, add:

```rust
            Request::Peek { tag: "web".into(), uuid: Some("9e400fbc-d371-4da6-afe1-6320df840239".into()) },
            Request::Peek { tag: "web".into(), uuid: None },
```

In `src/daemon.rs`'s tests, add:

```rust
    /// A peek is kept per workspace tag, replaced by the next for that tag
    /// and dropped by a clear, leaving other tags' alone.
    #[test]
    fn a_peek_is_kept_per_workspace_tag() {
        let mut peeks = HashMap::new();
        remember_peek(&mut peeks, "web", Some("a"));
        remember_peek(&mut peeks, "api", Some("b"));
        remember_peek(&mut peeks, "web", Some("c"));
        assert_eq!(peeks.get("web").map(String::as_str), Some("c"));
        remember_peek(&mut peeks, "web", None);
        assert_eq!(peeks.get("web"), None);
        assert_eq!(peeks.get("api").map(String::as_str), Some("b"));
    }
```

In `src/main.rs`'s tests, after `task_active_is_no_command`, add:

```rust
    /// herdr's focus hooks run `task peek <uuid8>`, or `task peek --clear`
    /// for a pane on no task: one or the other, never both or neither.
    #[test]
    fn peeking_takes_a_task_or_clear() {
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "c53b6e3d"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "--clear"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "peek", "c53b6e3d", "--clear"]).is_err());
    }
```

- [ ] **Step 2: Run them to see them fail to build**

Run: `cargo test 2>&1 | grep -E "^error" | head`
Expected: `no variant … Peek`, `cannot find function remember_peek`, and the CLI test failing to parse `peek`.

- [ ] **Step 3: Implement the request**

In `src/ipc.rs`, change the module doc's first sentence to

```rust
//! Asking the running daemon to open the task box, to hand the task panel
//! the keyboard, to show its project list, or to peek at a task.
```

replace `//! The protocol is one line per request, because it only ever carries a mode,\n//! a uuid and the add box's refine flag:` with

```rust
//! The protocol is one line per request, because it only ever carries a mode,
//! a workspace tag, a uuid and the add box's refine flag:
```

and add `//! peek <tag> <uuid>` and `//! peek <tag>` as the last two lines of the ```` ```text ```` block, after `//! projects`.

In `pub enum Request`, after `Projects,`, add:

```rust
    /// Peek at this task's card on the edge of the panel showing this
    /// workspace tag, while it is tucked away, or at nothing: herdr's focused
    /// pane, from task-docs' focus hooks. Kept per tag, so a workspace on no
    /// monitor yet shows it once a monitor shows that workspace.
    Peek { tag: String, uuid: Option<String> },
```

In `encode`, after `Request::Projects => "projects".into(),`, add:

```rust
            Request::Peek { tag, uuid: None } => format!("peek {tag}"),
            Request::Peek { tag, uuid: Some(uuid) } => format!("peek {tag} {uuid}"),
```

In `decode`, before `_ => None,`, add:

```rust
            ("peek", Some(rest)) => {
                let mut words = rest.split_whitespace();
                let tag = words.next()?.to_string();
                Some(Request::Peek { tag, uuid: words.next().map(String::from) })
            }
```

In `src/panel/surface.rs`, after `show`, add:

```rust
    /// herdr's focused pane is on this task now, or on none: the card the
    /// panel shows tucked away. The daemon asks only the panel whose
    /// workspace the peek is for (see `shows`).
    pub fn peek(self: &Rc<Self>, uuid: Option<&str>) {
        let effects = self.state.borrow_mut().set_peek(uuid);
        self.apply(effects);
    }

    /// This panel shows the workspace with this tag.
    pub fn shows(&self, tag: &str) -> bool {
        *self.tag.borrow() == tag
    }
```

In `src/daemon.rs`, inside `thread_local! { … }` after `PANELS`, add:

```rust
    /// The task each workspace tag peeks at, by uuid: herdr's focused pane's,
    /// from `niritasks task peek`. Kept per tag rather than per panel, so a
    /// workspace on no monitor yet peeks at it once a monitor shows it.
    static PEEKS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
```

After `forget_returns`, add:

```rust
/// Note `uuid` as the task the workspace with this tag peeks at, or that it
/// peeks at nothing.
fn remember_peek(peeks: &mut HashMap<String, String>, tag: &str, uuid: Option<&str>) {
    match uuid {
        Some(uuid) => {
            peeks.insert(tag.to_string(), uuid.to_string());
        }
        None => {
            peeks.remove(tag);
        }
    }
}
```

In `serve_request`, change its doc comment's first sentence to `/// Serve one request from the CLI: a task box, the panel's keyboard, the\n/// project list, or a peek.` and, after the `Request::Panel(uuid) => { … }` arm, add:

```rust
        Request::Peek { tag, uuid } => {
            PEEKS.with(|p| remember_peek(&mut p.borrow_mut(), &tag, uuid.as_deref()));
            // Every monitor showing that workspace; none until one does, and
            // the tick hands it over then.
            let showing: Vec<Rc<Panel>> =
                PANELS.with(|p| p.borrow().values().filter(|panel| panel.shows(&tag)).cloned().collect());
            for panel in showing {
                panel.peek(uuid.as_deref());
            }
        }
```

In `tick`, replace the loop body written in Task 1 with:

```rust
        let name = outputs.get(connector).cloned().flatten();
        let t = tag::workspace_tag(name.as_deref().unwrap_or_default());
        let peek = PEEKS.with(|p| p.borrow().get(&t).cloned());
        let cards = by_name
            .entry(name.clone())
            .or_insert_with(|| cards_for_workspace(name.as_deref()));
        panel.show(name.as_deref(), cards, peek.as_deref());
```

In `build`'s `eprintln!` for an IPC failure, change `"IPC unavailable ({e}); task add, edit, note, panel and project open will report no daemon"` to `"IPC unavailable ({e}); task add, edit, note, panel, peek and project open will report no daemon"`.

In `src/main.rs`, add `session,` to the `use niri_tasks::{ … }` list, between `refine,` and `speak,`. In `enum TaskCommand`, after the `Panel { … }` variant, add:

```rust
    /// Peek at a task's card on the edge of its workspace's task panel, as herdr's focused pane does
    ///
    /// What herdr's task-docs plugin runs as a pane, tab or workspace is
    /// selected, so only from a pane of a named herdr session: the peek is
    /// that session's workspace's. Tucked away, the panel shows that card
    /// alone, whole, with its notes and buttons, sliding out under the
    /// pointer without taking the keyboard. A task not pending on that
    /// workspace shows nothing, as --clear does. Exit 1 with no daemon, an
    /// unknown task, or outside a named herdr session.
    Peek {
        /// The task's uuid, or its first 8 characters, as in a
        /// `task/<slug>-<uuid8>` branch
        #[arg(required_unless_present = "clear", conflicts_with = "clear")]
        uuid: Option<String>,
        /// Peek at nothing: the focused pane runs no task
        #[arg(long)]
        clear: bool,
    },
```

In `task_command`, after the `TaskCommand::Panel { uuid } => { … }` arm, add:

```rust
        // The session's workspace, never the focused one or the folder's:
        // herdr runs the hook in its server, in the plugin's folder, and the
        // user may be looking anywhere. A uuid8 is resolved here, as for the
        // panel.
        TaskCommand::Peek { uuid, clear: _ } => {
            anyhow::ensure!(
                session::current_pane().is_some(),
                "task peek is for a pane of a named herdr session, whose workspace it peeks on; this is not one."
            );
            let tag = Workspace::of_session()?.tag().to_string();
            let uuid = match uuid {
                Some(u) => Some(task::get(&u)?.context("task not found")?.uuid),
                None => None,
            };
            ipc::send(&ipc::Request::Peek { tag, uuid })?
        }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head -30`
Expected: all `ok`, no `FAILED`.

Then check the refusal outside a session, which errors before any request is sent:

Run: `cargo build -q && env -u HERDR_SESSION -u HERDR_SOCKET_PATH target/debug/niritasks task peek --clear; echo "exit $?"`
Expected: `Error: task peek is for a pane of a named herdr session, whose workspace it peeks on; this is not one.` then `exit 1`.

- [ ] **Step 5: Commit**

```bash
git add src/ipc.rs src/daemon.rs src/panel/surface.rs src/main.rs
git commit -m "$(cat <<'EOF'
feat(ipc): add niritasks task peek

`task peek <uuid>` and `task peek --clear` tell the daemon which task
the herdr session's workspace peeks at, over a new `peek <tag> [<uuid>]`
request. The daemon keeps one peek per workspace tag and hands it to
the panel showing that workspace. Only a named herdr session's pane may
ask: its workspace is the session's, never the focused one.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: The nested-niri e2e test measures the peek

The tucked panel used to show a one-line stack of every pending task, and `tests/e2e-panel.sh` measured it ("one task pokes out a 30px peek", "three tasks stack three cards", "Escape puts it back exactly as it was"). Now nothing shows until a task is peeked, so those sections peek at a task with `task peek`, the way herdr will, from a session named after the nested workspace (`e2e`).

**Files:**
- Modify: `tests/e2e-panel.sh`

**Interfaces:**
- Consumes: `niritasks task peek <uuid> | --clear` from Task 2, run with `HERDR_SESSION=e2e`.
- Produces: shell helpers `peek` and `uuid_of` used by later sections of the same script; the variable `key_one_h` (the keyboard's panel on one card), from which `pair_h` (two unfocused one-line cards and their gaps) replaces `three_h - one_h`.

- [ ] **Step 1: Confirm the current script fails against the new build**

Run: `cargo build -q && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -25`
Expected: FAIL lines, the first being `one task drew columns 0-0, expected 1567-1600` (nothing peeked, nothing drawn). Needs `wtype` and the nested-niri setup the README's Testing section lists; if the script skips or cannot start a nested niri here, say so in the report and carry on with Step 2.

- [ ] **Step 2: Rewrite the opening sections around the peek**

Replace everything from the line `# ─── one task: a peek on the right edge ──────────────────────────────────────` up to, but not including, the line `# ─── the keyboard: every card wrapped, with its buttons, mid-screen ──────────` with:

```bash
# ─── the peek: one card, the one herdr's focused pane is on ──────────────────
# With nothing peeked the edge shows nothing, however many tasks there are.
# `task peek` is what herdr's task-docs plugin runs as a pane is focused,
# from that pane's session: HERDR_SESSION names the nested niri's
# workspace, e2e, as a herdr session opened for it would. It reaches only
# this command, never the nested niri.
peek() {  # uuid | --clear
    "${NENV[@]}" HERDR_SESSION=e2e "$NIRITASKS" task peek "$@" >/dev/null 2>&1
}
uuid_of() {  # description
    task "description.is:$1" _uuids 2>/dev/null | head -n 1
}

add "ship it"
settle
shot unpeeked || { summary; exit 1; }
if same baseline unpeeked; then
    ok "a task with nothing peeked leaves the edge empty"
else
    read -r x0 x1 y0 y1 < <(measure unpeeked)
    bad "with nothing peeked the panel drew columns ${x0}-${x1}, rows ${y0}-${y1} —
      the edge still lists the cards"
fi

peek "$(uuid_of 'ship it')"
settle
shot one || { summary; exit 1; }
read -r x0 x1 y0 y1 < <(measure one)
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "the peeked task pokes out a ${PEEK}px peek and its ${RING}px ring (columns ${x0}-${x1})"
else
    bad "the peeked task drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W} —
      0-0 means nothing drew; an earlier start means it is not tucked away.
      NIRITASKS_E2E_KEEP=1 keeps the frames to tell which"
fi

# The keyboard on one card, for the keyboard section's heights: the panel
# on three cards is taller than this by two unfocused one-line cards and
# their gaps. Escape puts the peek back as it was.
if command -v wtype >/dev/null; then
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    shot key_one || { summary; exit 1; }
    read -r _ _ y0 y1 < <(measure key_one)
    key_one_h=$((y1 - y0))
    "${NENV[@]}" wtype -k Escape
    settle
    shot peek_back || { summary; exit 1; }
    if same one peek_back; then
        ok "Escape puts the peeked card back as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure peek_back one)
        bad "after Escape the screen differs from the peek in columns ${x0}-${x1}, rows ${y0}-${y1}"
    fi
fi

# ─── more tasks: still one card ──────────────────────────────────────────────
add "write the glossary"
add "a much longer description that has to ellipsise rather than wrap onto a second line"
settle
shot three || { summary; exit 1; }
if same one three; then
    ok "more tasks on the workspace leave the peek one card"
else
    read -r x0 x1 y0 y1 < <(measure three one)
    bad "adding two tasks changed the peek in columns ${x0}-${x1}, rows ${y0}-${y1} —
      the edge shows more than the peeked card"
fi

# ─── another tag's task: nothing here, peeked or not ─────────────────────────
task rc.verbose=nothing rc.confirmation=no add +niritasks_e2e_elsewhere -- "not here" >/dev/null 2>&1
settle
shot elsewhere || { summary; exit 1; }
if same three elsewhere; then
    ok "a task on another workspace's tag changes nothing on screen"
else
    read -r x0 x1 y0 y1 < <(measure elsewhere three)
    bad "a task on another tag changed columns ${x0}-${x1}, rows ${y0}-${y1}"
fi
peek "$(uuid_of 'not here')"
settle
shot peek_elsewhere || { summary; exit 1; }
if same baseline peek_elsewhere; then
    ok "peeking at another workspace's task leaves this edge empty"
else
    read -r x0 x1 y0 y1 < <(measure peek_elsewhere)
    bad "peeking at another tag's task drew columns ${x0}-${x1}, rows ${y0}-${y1}"
fi

# ─── a whole card: its notes, and gone once its task is ──────────────────────
# A throwaway task, deleted before the keyboard section counts the cards.
add "peek me"
me=$(uuid_of 'peek me')
peek "$me"
settle
shot peek_me || { summary; exit 1; }
read -r _ _ y0 y1 < <(measure peek_me)
plain_h=$((y1 - y0))
task rc.verbose=nothing rc.confirmation=no "$me" annotate -- "a note on the peek" >/dev/null 2>&1
settle
shot peek_noted || { summary; exit 1; }
read -r x0 x1 y0 y1 < <(measure peek_noted)
if [ "$plain_h" -gt 0 ] && [ "$x0" -eq "$PEEK_X" ] && [ "$((y1 - y0))" -gt "$plain_h" ]; then
    ok "the peeked card shows its notes, growing to fit (${plain_h}px to $((y1 - y0))px)"
else
    bad "with a note the peek covers columns ${x0}-${x1} and is $((y1 - y0))px tall, against
      ${plain_h}px without — its notes are not shown"
fi
task rc.verbose=nothing rc.confirmation=no "$me" delete >/dev/null 2>&1
settle
shot peek_gone || { summary; exit 1; }
if same baseline peek_gone; then
    ok "the peeked task leaving pending leaves the edge empty"
else
    read -r x0 x1 y0 y1 < <(measure peek_gone)
    bad "with the peeked task deleted the edge still draws columns ${x0}-${x1}, rows ${y0}-${y1}"
fi

# ─── peeking again, and --clear ──────────────────────────────────────────────
peek "$(uuid_of 'ship it')"
settle
shot reshown || { summary; exit 1; }
if same one reshown; then
    ok "peeking at a task again shows its card as before"
else
    read -r x0 x1 y0 y1 < <(measure reshown one)
    bad "peeking at 'ship it' again differs from the first time in columns ${x0}-${x1}, rows ${y0}-${y1}"
fi
# The keyboard section below works from an empty edge.
peek --clear
settle
shot cleared || { summary; exit 1; }
if same baseline cleared; then
    ok "task peek --clear leaves the edge empty"
else
    read -r x0 x1 y0 y1 < <(measure cleared)
    bad "after --clear the edge still draws columns ${x0}-${x1}, rows ${y0}-${y1}"
fi

```

- [ ] **Step 3: Rebase the keyboard section's heights and tucked frames on the peek**

In the keyboard section, right after the line `if command -v wtype >/dev/null; then` that follows the `# frames is those two cards' columns, and the panel is as tall as it was.` comment, insert:

```bash
    # Two unfocused one-line cards and their gaps: the keyboard's panel on
    # three cards less the same panel on one, both with the same tabs, row
    # and footer.
    pair_h=$((keyboard_h - key_one_h))
```

Replace

```bash
    if [ "$key_h" -ge $((three_h - one_h + 12)) ]; then
        ok "and the focused card shows its buttons (${key_h}px for two cards and one row, ${three_h}px for three tucked)"
    else
        bad "the two cards Down touched are ${key_h}px tall, against $((three_h - one_h))px for
      two one-line cards and their gaps — no card shows its action row"
    fi
```

with

```bash
    if [ "$key_h" -ge $((pair_h + 12)) ]; then
        ok "and the focused card shows its buttons (${key_h}px for two cards and one row, ${pair_h}px for two without)"
    else
        bad "the two cards Down touched are ${key_h}px tall, against ${pair_h}px for
      two one-line cards and their gaps — no card shows its action row"
    fi
```

Replace

```bash
    # Escape from a tab other than All: the hover panel has no tabs, so it
    # comes back exactly as before, and the next keyboard opens on All.
    "${NENV[@]}" wtype 4
    sleep 1
    "${NENV[@]}" wtype -k Escape
    settle
    shot released || { summary; exit 1; }
    if same three released; then
        ok "Escape puts it back exactly as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure released three)
        bad "after Escape the screen differs from the tucked panel in columns
      ${x0}-${x1}, rows ${y0}-${y1}"
    fi
```

with

```bash
    # Escape from a tab other than All: with nothing peeked the edge comes
    # back empty, as before the keyboard took it, and the next keyboard
    # opens on All.
    "${NENV[@]}" wtype 4
    sleep 1
    "${NENV[@]}" wtype -k Escape
    settle
    shot released || { summary; exit 1; }
    if same baseline released; then
        ok "Escape puts it back exactly as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure released)
        bad "after Escape the screen differs from the empty edge in columns
      ${x0}-${x1}, rows ${y0}-${y1}"
    fi
```

Replace

```bash
        if same three outside_closed; then
            ok "a box opened from outside the panel closes with the panel tucked away"
        else
            read -r x0 x1 y0 y1 < <(measure outside_closed three)
```

with

```bash
        if same baseline outside_closed; then
            ok "a box opened from outside the panel closes with the panel tucked away"
        else
            read -r x0 x1 y0 y1 < <(measure outside_closed)
```

Replace

```bash
    # Park one as waiting while the panel is open: All loses it and the
    # Waiting tab comes up, and 5 shows it there alone.
    task rc.verbose=nothing rc.confirmation=no "+$TAG" "description.is:ship it" \
```

with

```bash
    # Park one as waiting while the panel is open: All loses it and the
    # Waiting tab comes up, and 5 shows it there alone. It is peeked at
    # first, which draws nothing under the keyboard, for the check after
    # Escape that a peek follows its task off the list.
    peek "$(uuid_of 'ship it')"
    task rc.verbose=nothing rc.confirmation=no "+$TAG" "description.is:ship it" \
```

Replace

```bash
    # one-line cards and their gaps, as three tucked cards are than one, less
```

with

```bash
    # one-line cards and their gaps (pair_h), less
```

Replace

```bash
    extra=$((keyboard_h - waiting_h - (three_h - one_h) + clear_strip_h))
```

with

```bash
    extra=$((keyboard_h - waiting_h - pair_h + clear_strip_h))
```

Replace

```bash
      tab's one, ${extra}px off two one-line cards ($((three_h - one_h))px) — the unfocused
```

with

```bash
      tab's one, ${extra}px off two one-line cards (${pair_h}px) — the unfocused
```

Replace

```bash
    # Off the keyboard, the waiting task is off the hover panel too: two
    # cards tucked away, where there were three.
    "${NENV[@]}" wtype -k Escape
    settle
    shot parked || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure parked)
    if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ] && [ "$((y1 - y0))" -lt "$three_h" ]; then
        ok "a waiting task leaves the tucked panel (${three_h}px to $((y1 - y0))px)"
    else
        bad "with one task waiting the tucked panel covers columns ${x0}-${x1}, rows
      ${y0}-${y1}, against ${three_h}px for three — the waiting task is still on it"
    fi
```

with

```bash
    # Off the keyboard, the peeked task, now waiting, is off the edge.
    "${NENV[@]}" wtype -k Escape
    settle
    shot parked || { summary; exit 1; }
    if same baseline parked; then
        ok "a peeked task parked as waiting leaves the edge empty"
    else
        read -r x0 x1 y0 y1 < <(measure parked)
        bad "with the peeked task waiting the edge still draws columns ${x0}-${x1}, rows ${y0}-${y1}"
    fi
```

In the Ideas section, replace

```bash
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    shot ideas_before || { summary; exit 1; }
```

with

```bash
    # A peek, for the second Escape to tuck the panel back onto.
    peek "$(uuid_of 'past the cap 1')"
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    shot ideas_before || { summary; exit 1; }
```

In the cold-start section, replace

```bash
add "after a cold start"
settle
shot cold_start || { summary; exit 1; }
read -r x0 x1 _ _ < <(measure cold_start)
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "a daemon started with nothing to show still shows the next task (columns ${x0}-${x1})"
```

with

```bash
add "after a cold start"
peek "$(uuid_of 'after a cold start')"
settle
shot cold_start || { summary; exit 1; }
read -r x0 x1 _ _ < <(measure cold_start)
if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
    ok "a daemon started with nothing to show still shows the next peek (columns ${x0}-${x1})"
```

In the header comment, replace

```bash
# What it cannot check is the hover: nothing can move the pointer, so the slide
# out, the slide back and clicks passing beside the peek are checked by hand
# (README, "Testing").
```

with

```bash
# What it cannot check is the hover: nothing can move the pointer, so the slide
# out, the slide back, clicks on the peek's buttons and clicks passing beside
# it are checked by hand (README, "Testing"). The peek itself it sets with
# `task peek`, as herdr's task-docs plugin does from a focused pane.
```

Then confirm nothing still reads the old variables:

Run: `grep -n 'three_h\|one_h' tests/e2e-panel.sh`
Expected: no output.

- [ ] **Step 4: Run the e2e test**

Run: `cargo build -q && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -40`
Expected: every check `ok` or `skip`, the summary reporting 0 failed. Keep off the nested niri's workspace while it runs. If a frame comparison fails, rerun with `NIRITASKS_E2E_KEEP=1` and look at the frames under the printed sandbox folder before changing an expectation.

- [ ] **Step 5: Commit**

```bash
git add tests/e2e-panel.sh
git commit -m "$(cat <<'EOF'
test(e2e): measure the peeked card instead of the stack

The tucked panel shows nothing until a task is peeked, so the panel
test peeks with `task peek` from a session named after the nested
workspace, as herdr will, and checks the peek's width, its notes, that
another tag's task or one leaving pending empties the edge, and that
--clear does. Heights the stack gave now come from the keyboard's panel.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: herdr's focus hooks peek at the focused pane's task

**Files:**
- Modify: `herdr/task-docs/herdr-plugin.toml`
- Modify: `herdr/task-docs/task_docs.py`
- Test: `herdr/task-docs/tests/test_task_docs.py`

**Interfaces:**
- Consumes: `niritasks task peek <uuid8>` / `niritasks task peek --clear` from Task 2; the plugin's own `find_task(pane, cwd)`, `herdr(*args)` and `state_dir()`.
- Produces: `task_docs.py focused`, run by herdr for `pane.focused`, `tab.focused` and `workspace.focused`; functions `focused_pane()`, `last_focused()`, `peek(*args)`, `focused_action()`.

How herdr answers, checked against herdr 0.9.3 on 2026-10-26: `herdr pane list` prints `{"result":{"panes":[…]}}`, each pane with `pane_id`, `focused` (exactly one `true` in the session), `cwd` and `foreground_cwd`; the three events' payloads carry only ids, so `focused` ignores `HERDR_PLUGIN_EVENT_JSON` and asks for the focused pane, which resolves all three the same way. Hooks run with the plugin root as cwd and herdr's runtime env, `HERDR_SESSION` among it.

- [ ] **Step 1: Write the failing tests**

In `herdr/task-docs/tests/test_task_docs.py`, replace `FAKE_HERDR` with:

```python
# herdr: logs each call as a JSON list, answers `agent get <target>` from the
# FAKE_AGENTS map, exiting 1 for a target it does not hold, as herdr does, and
# `pane list` with the FAKE_PANES list.
FAKE_HERDR = """#!/usr/bin/env python3
import json, os, sys
with open(os.environ["FAKE_HERDR_LOG"], "a") as log:
    log.write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[1:3] == ["agent", "get"]:
    agent = json.load(open(os.environ["FAKE_AGENTS"])).get(sys.argv[3])
    if agent is None:
        print('{"error":{"code":"agent_not_found"}}', file=sys.stderr)
        sys.exit(1)
    print(json.dumps({"result": {"agent": agent}}))
if sys.argv[1:3] == ["pane", "list"]:
    print(json.dumps({"result": {"panes": json.load(open(os.environ["FAKE_PANES"]))}}))
"""
```

After `FAKE_NIRITASKS_PANEL`, add:

```python
# niritasks: appends its arguments as one line per call; `task peek <uuid8>`
# exits $FAKE_UUID_EXIT, as niritasks does for an unknown task, and every call
# exits $FAKE_PEEK_EXIT, as with no daemon.
FAKE_NIRITASKS_PEEK = """#!/bin/sh
echo "$*" >> "$FAKE_NIRITASKS_LOG"
if [ -n "$FAKE_PEEK_EXIT" ]; then
    echo "Error: The niri-tasks daemon is not running" >&2
    exit "$FAKE_PEEK_EXIT"
fi
if [ -n "$FAKE_UUID_EXIT" ] && [ "$3" != "--clear" ]; then
    echo "Error: task not found" >&2
    exit "$FAKE_UUID_EXIT"
fi
"""
```

In `PluginTest.setUp`, after `self.with_agents({})`, add:

```python
        self.panes = self.tmp / "panes.json"
        self.with_panes([])
```

and add `"FAKE_PANES": str(self.panes),` and `"HERDR_SESSION": "web",` to its `env` dict. After `with_agents`, add:

```python
    def with_panes(self, panes):
        self.panes.write_text(json.dumps(panes))
```

After the `ThePanelAction` class, add:

```python
class TheFocusHook(PluginTest):
    def setUp(self):
        super().setUp()
        fake(self.bin, "niritasks", FAKE_NIRITASKS_PEEK)
        self.niritasks_log = self.tmp / "niritasks.log"
        patcher = mock.patch.dict(os.environ, {"FAKE_NIRITASKS_LOG": str(self.niritasks_log)})
        patcher.start()
        self.addCleanup(patcher.stop)

    def asked(self):
        return self.niritasks_log.read_text().splitlines() if self.niritasks_log.exists() else []

    def focus(self, pane, cwd=None):
        """herdr's panes with `pane` the focused one, in `cwd`, the work folder by default."""
        self.with_panes([
            {"pane_id": "w1:p9", "focused": False, "cwd": "/elsewhere", "foreground_cwd": "/elsewhere"},
            {"pane_id": pane, "focused": True, "cwd": "/launched", "foreground_cwd": str(cwd or self.folder)},
        ])

    def test_a_focused_tasks_pane_peeks_at_its_task(self):
        self.with_agents({"w1:p2": {"name": "work-1234abcd", "pane_id": "w1:p2"}})
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd"])
        self.assertEqual(self.notified(), "")

    def test_a_focused_shell_on_the_tasks_branch_peeks_at_it(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd"])

    def test_a_pane_on_no_task_clears_the_peek(self):
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek --clear"])

    def test_the_same_pane_again_is_ignored(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd"])

    def test_another_pane_peeks_again(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        self.focus("w1:p3")
        task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek 1234abcd"])

    def test_with_no_pane_focused_it_does_nothing(self):
        self.with_panes([{"pane_id": "w1:p2", "focused": False, "cwd": str(self.folder)}])
        task_docs.focused_action()
        self.assertEqual(self.asked(), [])

    def test_a_task_niritasks_does_not_know_clears_the_peek(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        with mock.patch.dict(os.environ, {"FAKE_UUID_EXIT": "1"}):
            task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek --clear"])

    def test_a_failing_niritasks_notifies_nothing_and_tries_again_next_time(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        with mock.patch.dict(os.environ, {"FAKE_PEEK_EXIT": "1"}):
            task_docs.focused_action()
            task_docs.focused_action()
        self.assertEqual(self.notified(), "")
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek --clear"] * 2)

    def test_without_niritasks_it_notifies_nothing(self):
        (self.bin / "niritasks").unlink()
        self.focus("w1:p2")
        task_docs.focused_action()
        self.assertEqual(self.notified(), "")

    def test_each_session_remembers_its_own_pane(self):
        self.on_branch("task/fix-1234abcd")
        self.focus("w1:p2")
        task_docs.focused_action()
        with mock.patch.dict(os.environ, {"HERDR_SESSION": "api"}):
            task_docs.focused_action()
        self.assertEqual(self.asked(), ["task peek 1234abcd", "task peek 1234abcd"])


class TheManifest(unittest.TestCase):
    def test_selecting_a_pane_a_tab_or_a_workspace_runs_focused(self):
        import tomllib
        manifest = tomllib.loads((ROOT / "herdr-plugin.toml").read_text())
        hooks = {event["on"]: event["command"] for event in manifest["events"]}
        focused = ["python3", "task_docs.py", "focused"]
        self.assertEqual(hooks, {"pane.focused": focused, "tab.focused": focused, "workspace.focused": focused})
```

In `TheCommandLine`, add:

```python
    def test_focused_is_a_command(self):
        with mock.patch.object(task_docs, "focused_action") as focused:
            self.assertEqual(task_docs.main(["task_docs.py", "focused"]), 0)
        focused.assert_called_once_with()
```

- [ ] **Step 2: Run them to see them fail**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs 2>&1 | tail -8`
Expected: FAILED, with `AttributeError: module 'task_docs' has no attribute 'focused_action'` and `KeyError: 'events'`.

- [ ] **Step 3: Implement the hook**

In `herdr/task-docs/task_docs.py`, append to the module docstring, before its closing `"""`:

```python
`task_docs.py focused` is the focus hook herdr runs as a pane, a tab or a
workspace is selected: it finds the session's focused pane's task the same
way and runs `niritasks task peek <uuid8>`, or `--clear` for a pane on no
task, so that task's card alone peeks out of its workspace's task panel. A
pane already peeked for is skipped, and a failure goes to stderr, herdr's
plugin log, never to a notification.
```

After `panel_action`, add:

```python
def focused_pane():
    """The session's focused pane, as (pane id, its live folder), or None when
    herdr cannot say or no pane is focused. The focus events carry only ids, a
    tab's or a workspace's, so every one is resolved here the same way."""
    result = herdr("pane", "list")
    if result.returncode != 0:
        return None
    try:
        panes = json.loads(result.stdout)["result"]["panes"]
    except (ValueError, KeyError, TypeError):
        return None
    for pane in panes:
        if pane.get("focused") and pane.get("pane_id"):
            return pane["pane_id"], pane.get("foreground_cwd") or pane.get("cwd")
    return None


def last_focused():
    """Where the pane last peeked for is noted: one file per herdr session,
    since each session has its own focused pane."""
    return state_dir() / f"focused-{os.environ.get('HERDR_SESSION') or 'default'}"


def peek(*args):
    """`niritasks task peek` with these arguments; whether it worked. A failure
    goes to stderr, which herdr keeps in the plugin's log: a notification on
    every pane switch would be noise."""
    try:
        result = subprocess.run(["niritasks", "task", "peek", *args],
                                capture_output=True, text=True, check=False)
    except OSError:
        print("task-docs focused: niritasks not found on PATH", file=sys.stderr)
        return False
    if result.returncode != 0:
        print(f"task-docs focused: {result.stderr.strip()[-300:]}", file=sys.stderr)
    return result.returncode == 0


def focused_action():
    """The pane, tab and workspace focus hooks: peek at the focused pane's task,
    or at nothing when it runs none. The three events one switch can fire find
    the same pane, and a pane already peeked for is skipped. A task niritasks
    does not know, one whose branch outlived it, peeks at nothing. The pane is
    noted only once a peek went through, so a missing daemon is tried again on
    the next switch."""
    found = focused_pane()
    if not found:
        return
    pane, cwd = found
    seen = last_focused()
    try:
        if seen.read_text() == pane:
            return
    except OSError:
        pass
    uuid8 = find_task(pane, cwd)
    if (uuid8 and peek(uuid8)) or peek("--clear"):
        try:
            seen.parent.mkdir(parents=True, exist_ok=True)
            seen.write_text(pane)
        except OSError:
            pass
```

In `main`, change the commands dict and the usage line to:

```python
    commands = {"open": open_action, "panel": panel_action, "focused": focused_action,
                "pick": pick, "review": review}
    if len(argv) != 2 or argv[1] not in commands:
        print("usage: task_docs.py open|panel|focused|pick|review", file=sys.stderr)
        return 2
```

In `herdr/task-docs/herdr-plugin.toml`, change the header comment's first sentence to cover the peek:

```toml
# Annotate one of a task's documents, open the task panel on it, or peek at
# it on the panel's edge, from the herdr pane where its Claude runs: its
# refine reports, its own description and notes, and the plans written since
# it was added. The pick opens in plannotator-tui (from herdr-annotate Full)
# beside that Claude, and Send makes the comments its next message. Linked by
# niri-tasks' install.sh; the keys are the README's.
```

change `min_herdr_version = "0.9.1"` to `min_herdr_version = "0.9.3"`, change the `description` to `"Annotate a niri-tasks task's reports, notes and plans in plannotator-tui beside its Claude, open the task panel on the task, or peek at the focused pane's task"`, and, after the `panel` action's `[[actions]]` block, add:

```toml
# Run as a pane, a tab or a workspace is selected: finds the now-focused
# pane's task as `open` does and runs `niritasks task peek <uuid8>`, or
# `--clear` for a pane on no task, so only that task's card peeks out of its
# workspace's task panel. A pane already peeked for is skipped; failures go
# to `herdr plugin log list --plugin task-docs`, never a notification.
[[events]]
on = "pane.focused"
command = ["python3", "task_docs.py", "focused"]

[[events]]
on = "tab.focused"
command = ["python3", "task_docs.py", "focused"]

[[events]]
on = "workspace.focused"
command = ["python3", "task_docs.py", "focused"]
```

- [ ] **Step 4: Run the tests**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs 2>&1 | tail -4`
Expected: `OK`, with 40 tests (28 before, plus 10 in `TheFocusHook`, 1 in `TheManifest`, 1 in `TheCommandLine`).

- [ ] **Step 5: Commit**

```bash
git add herdr/task-docs/herdr-plugin.toml herdr/task-docs/task_docs.py herdr/task-docs/tests/test_task_docs.py
git commit -m "$(cat <<'EOF'
feat(task-docs): peek at the focused pane's task

Hooks on pane.focused, tab.focused and workspace.focused run
`task_docs.py focused`, which finds the session's focused pane's task
from its agent's name or branch and runs `niritasks task peek`, or
--clear for a pane on no task. A pane already peeked for is skipped,
and failures go to herdr's plugin log rather than a notification.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Document the peek

**Files:**
- Modify: `CONTEXT.md` (Task panel, Task card, Action row, Peek)
- Modify: `README.md` (Task panel intro, the herdr plugin section, Commands, Testing)
- Modify: `llms.txt` (command table, the task-docs link)

**Interfaces:**
- Consumes: the behaviour of Tasks 1–4; no code.
- Produces: nothing other tasks use.

- [ ] **Step 1: CONTEXT.md**

Replace the **Task panel** entry's definition

```markdown
The list of task cards on a monitor's right edge, showing the pending tasks of
that monitor's active workspace. It is tucked away to a peek until hovered,
or until Mod+Alt+Ctrl+T hands it the keyboard, which moves it to the middle of the
screen until the keyboard is given back.
```

with

```markdown
The task cards on a monitor's right edge, for that monitor's active
workspace. Tucked away it shows one card at most, the peek, which slides out
when hovered; Mod+Alt+Ctrl+T hands it the keyboard, which moves it to the
middle of the screen with every pending task of the workspace under its filter
tabs, until the keyboard is given back.
```

In the **Task card** entry, replace

```markdown
One task in the task panel, drawn like a notification: a status icon, a
one-line description and, at its right end, how long ago the task was added
(5m, 3h, 2d, 4w) — or, while the task panel has the keyboard, the whole
description wrapped, above its action row while it has focus. The cards run
```

with

```markdown
One task in the task panel, drawn like a notification: a status icon, its
whole description wrapped and, at its right end, how long ago the task was
added (5m, 3h, 2d, 4w), above its action row while it has focus, or always
on the peek. The cards run
```

and, after the sentence ending `inside that height by wheel, touchpad or scrollbar.`, add:

```markdown
The peek's card shows its id line and notes all the time, and a click on its
body changes nothing.
```

In the **Action row** entry, replace its first line

```markdown
The buttons along the focused task card's bottom edge while the task panel has
the keyboard; the other cards hide theirs — Go to… first, then Start working,
```

with

```markdown
The buttons along the focused task card's bottom edge while the task panel has
the keyboard, and along the peek's card always; the other cards hide theirs —
Go to… first, then Start working,
```

and, after the sentence ending `Go to… opening the Go to list.` in that entry, add:

```markdown
On the peek they are clicked while herdr keeps the keyboard; Go to… and Move
to workspace, whose lists the keyboard drives, take it on that card first.
```

Replace the **Peek** entry

```markdown
**Peek**:
The strip of each task card left showing when the task panel is tucked away.
_Avoid_: sliver, handle, tab
```

with

```markdown
**Peek**:
What the task panel shows on its monitor's edge while tucked away: the one
card of the task herdr's focused pane runs, found by the pane's agent name or
its branch as herdr's task-docs plugin finds it, drawn whole (its id line,
notes and action row), with only its left 30px showing until the pointer
slides it out. herdr's task-docs plugin sets it with `niritasks task peek` as
a pane, a tab or a workspace is selected, and the daemon keeps it per
workspace tag. Nothing shows when that pane runs no task, the task is not
pending on that workspace, or nothing has been peeked. It never takes the
keyboard itself.
_Avoid_: sliver, handle, tab, hover card
```

- [ ] **Step 2: README.md**

Replace

```markdown
Each monitor shows the pending tasks of the workspace it is displaying, as a
stack of cards on the right edge — the active tasks first (there can be
```

with

```markdown
Each monitor's task panel holds the pending tasks of the workspace it is
displaying, as a stack of cards — the active tasks first (there can be
```

Replace

```markdown
The panel sits tucked away with only a 30px peek of each card showing. Move the
pointer onto it and the cards slide out to full width; move away and they slide
back. The space around the cards is click-through, so the panel never gets in
the way of the windows beneath, and a fullscreen window covers it. An unnamed
workspace, or one with nothing pending, shows no panel at all.

Clicking a card does nothing: its actions are the buttons the keyboard shows (below). The "+N more" card shows the rest of the tasks in the panel.
```

with

```markdown
Tucked away on the right edge, the panel shows one card at most: the task of
the herdr pane you have selected, on that workspace's monitor (see [Peeking at
the focused pane's task](#peeking-at-the-focused-panes-task)). Only 30px of it
shows. Move the pointer onto it and it slides out to full width, whole: its
description, its id and notes, and its buttons, which work by click while
herdr keeps the keyboard; move away and it slides back. Go to… and Move to
workspace open lists the keyboard drives, so a click on either hands the panel
the keyboard on that card first. The space around the card is click-through,
so the panel never gets in the way of the windows beneath, and a fullscreen
window covers it. With no task's pane selected, nothing shows at all.

The rest of the workspace's tasks are on the keyboard's panel (below), whose
"+N more" card shows any past the first eight.
```

After the section `### Opening the task panel on a herdr pane's task` (after its last line, `nothing opens. It runs \`niritasks task panel <uuid8>\`.`), add:

```markdown

### Peeking at the focused pane's task

The plugin also hooks herdr's `pane.focused`, `tab.focused` and
`workspace.focused` events; no key to bind. Selecting a pane, a tab or a
workspace finds the now-focused pane's task, as Task docs does, and that
task's card becomes the peek on its workspace's task panel: the one card the
edge shows. A pane with no task empties the edge, and so does the task
leaving the list (completed, waiting or removed). Selecting the same pane
again does nothing. It runs `niritasks task peek <uuid8>`, or `niritasks task
peek --clear`, and needs herdr 0.9.3. It sends no notification; if the peek
does not follow the focus, `herdr plugin log list --plugin task-docs` says
why.
```

In the Commands block, after the two `niritasks task panel [<uuid>]` lines, add:

```
niritasks task peek <uuid>         # peek at that task's card on its workspace's panel edge, from a
                                   #   named herdr session's pane (herdr's focus hooks run it)
niritasks task peek --clear        # peek at nothing: the edge shows no card
```

In the Testing section, replace

```markdown
It cannot move the pointer, so the hover is checked by hand after a change to
`src/panel/surface.rs`: the peek slides out when the pointer reaches it and back
about 0.4s after it leaves, the slide is smooth, and a click just left of the
peek, or between two cards, lands on the window beneath, while a click on a card does nothing, and a click on "+N more" shows the
rest.
```

with

```markdown
It cannot move the pointer, so the hover is checked by hand after a change to
`src/panel/surface.rs`, with a task peeked (select its herdr pane, or run
`niritasks task peek <uuid8>` from a pane of its workspace's session): the peek
slides out when the pointer reaches it and back about 0.4s after it leaves, the
slide is smooth, its notes and buttons show, a click just left of the peek lands
on the window beneath, a click on its description does nothing, Complete
completes it and empties the edge without taking the keyboard from herdr,
Remove asks once and the pointer leaving puts it back, and Go to… slides the
panel to the middle with the keyboard, on that card's Go to list.
```

and replace

```markdown
with no tasks: the peek's width, the stack's height as tasks are added, the
```

with

```markdown
with no tasks: nothing on the edge until a task is peeked, the peek's width and
its notes, an empty edge for another tag's task, a task leaving pending and
`--clear`, the
```

- [ ] **Step 3: llms.txt**

After the `niritasks task panel [<uuid>]` row, add:

```markdown
| `niritasks task peek <uuid>` / `niritasks task peek --clear` | Peek at that task's card on the edge of its workspace's task panel, or at nothing: tucked away the panel shows that card alone, whole, with notes and buttons, without taking the keyboard. Only from a pane of a named herdr session, whose workspace it peeks on; a task not pending there shows nothing. What herdr's task-docs focus hooks run. Exit 1 with no daemon, an unknown task, or outside a named herdr session | GUI, follows the herdr session |
```

In the `task-docs` link line, replace

```markdown
its second action, task-docs.panel, runs `niritasks task panel <uuid8>` to open the task panel on the pane's task
```

with

```markdown
its second action, task-docs.panel, runs `niritasks task panel <uuid8>` to open the task panel on the pane's task; its pane, tab and workspace focus hooks run `niritasks task peek` so the focused pane's task is the panel's peek
```

- [ ] **Step 4: Check the docs build nothing broken**

Run: `grep -n "one-line description\|30px peek of each card\|Clicking a card does nothing" README.md CONTEXT.md llms.txt`
Expected: no output.

Run: `cargo test -q 2>&1 | grep -E "^test result" | head -3 && python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs 2>&1 | tail -1`
Expected: all `ok`, then `OK`.

- [ ] **Step 5: Commit**

```bash
git add CONTEXT.md README.md llms.txt
git commit -m "$(cat <<'EOF'
docs: describe the peek of the focused herdr pane's task

Peek, Task panel, Task card and Action row in CONTEXT.md, the panel's
tucked-away card and the plugin's focus hooks in the README, and
`niritasks task peek` in llms.txt.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

## After landing: the hand check on the real desktop

herdr runs the plugin from the main checkout (`install.sh` links `~/Projects/niri-tasks/herdr/task-docs`), and the daemon is the installed `niritasks`, so the hooks and the new panel can only be seen together after `finish-worktree` lands the branch. Then, with the user:

1. `cargo install --path .` (or `install.sh`), `systemctl --user restart niri-tasks`, and `herdr server reload-config` so herdr reads the manifest's new `[[events]]`.
2. Select a task's Claude pane: its card alone peeks on that workspace's monitor. Hover: it slides out with notes and buttons; herdr still has the keyboard.
3. Select a plain shell pane off any task branch: the edge empties. Select another tab and another herdr workspace holding a task: each peeks at its task.
4. Complete the peeked task from its card: the edge empties. Mod+Alt+Ctrl+T: the full list with its tabs, as before.
5. If the peek does not follow: `herdr plugin log list --plugin task-docs`. A `task peek is for a pane of a named herdr session` line means herdr gives its event hooks no `HERDR_SESSION`; then pass the session from the hook (`HERDR_SOCKET_PATH` is the fallback `session_from_env` already reads) before anything else.
