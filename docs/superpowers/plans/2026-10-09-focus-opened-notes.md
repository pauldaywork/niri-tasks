# Focus a Card When a Click Opens Its Notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a click on a card's body opens its notes, move the keyboard focus to that card's body, so the action row, the hint and the scroll follow the card just opened.

**Architecture:** The panel's state (`src/panel/state.rs`, `PanelState`) decides everything and hands `src/panel/surface.rs` a list of `Effect`s to run. A press on a card body goes `on_press(uuid, Slot::Body)` → `toggle_notes(uuid)`, which today returns only `[Effect::Notes]` and leaves the focus alone; the body button is `set_focus_on_click(false)` (`surface.rs:1025`), so GTK does not move it either. The change is state-only: when `toggle_notes` *opens* notes it also calls the existing `focus_on(Focus::body(uuid))`, which updates `self.focus` through `on_focus` (disarming as any focus move does) and returns `[Effect::Focus(..)]`; `Effect::Notes` is pushed after it. The surface already handles both: `Effect::Focus` shows the action row and grabs GTK focus, and `Effect::Notes` lays the notes out and runs `follow_focus`, which scrolls after the next paint — so the whole opened card ends up in view. No surface code changes.

**Tech Stack:** Rust, GTK4 (gtk4-rs), `cargo test`.

**Spec:** Taskwarrior task `abf4c017-ac8e-41ca-b300-a8af7a8f77eb` — read it with `task rc.json.array=on abf4c017-ac8e-41ca-b300-a8af7a8f77eb export`; its description and annotations are the spec.

## Global Constraints

- Only opening notes moves the focus. Closing them leaves the focus where it is.
- The focus goes to the opened card's body slot (`Focus::body(uuid)`), the same place Up/Down lands — also when the focus was on one of that card's own buttons.
- Change the state, not GTK: `toggle_notes` returns `focus_on(Focus::body(uuid))` together with `Effect::Notes`.
- Moving the focus disarms an armed Remove, as any other focus move does. Update `toggle_notes`'s doc comment to match.
- Out of scope: keyboard Enter/Space on the body (it is already the focused card) and the peek (it shows no notes). Both go through the same code; neither needs special handling.
- Commits follow Conventional Commits (`feat(panel): …`), subject ≤ 72 chars, with the `Co-Authored-By` trailer.

---

### Task 1: Move the focus to a card whose notes a press opens

**Files:**
- Modify: `src/panel/state.rs:524-578` (`on_press`'s doc comment and `toggle_notes`)
- Test: `src/panel/state.rs` tests module — `a_press_on_the_body_toggles_its_notes` (~line 1396), `a_card_with_no_notes_opens_on_its_id_line` (~line 1410), plus two new tests after them
- Modify: `CONTEXT.md:79-82` (the **Task card** entry's notes sentence)

**Interfaces:**
- Consumes: existing `PanelState::focus_on(&mut self, focus: Focus) -> Vec<Effect>` (`state.rs:810`, returns `vec![Effect::Focus(Some(focus))]` after calling `on_focus`), `Focus::body(uuid: &str) -> Focus`, `Armed::Remove(String)`, test helpers `keyboard(cards)`, `noted(uuid)`, `pending(uuids)`, `focused(uuid, slot)`.
- Produces: `toggle_notes` returns `vec![Effect::Focus(Some(Focus::body(uuid))), Effect::Notes]` when it opens notes and `vec![Effect::Notes]` when it closes them. Nothing else depends on it.

Background for the tester: `keyboard(...)` (the test helper) takes the keyboard the way Mod+Alt+Ctrl+T does, which puts the focus on the **first** card's body. So in `keyboard(vec![noted("a"), noted("b")])` the focus starts on `a`'s body. Opening `a` therefore still yields an `Effect::Focus` for `a` (a no-op move: `on_focus` returns early on an unchanged focus, and GTK's `grab_focus` on the already-focused widget fires nothing) — the existing tests' expected effects change for that reason.

- [ ] **Step 1: Write the failing tests**

In `src/panel/state.rs`'s tests module, replace `a_press_on_the_body_toggles_its_notes` and `a_card_with_no_notes_opens_on_its_id_line` with these, and add the two new tests directly after them:

```rust
    /// A press on a card's body, Enter or a click, shows its task's notes
    /// and puts the focus on it; a second hides them, keeping the keyboard
    /// and the focus.
    #[test]
    fn a_press_on_the_body_toggles_its_notes() {
        let mut state = keyboard(vec![noted("a"), noted("b")]);
        assert_eq!(
            state.on_press("a", Slot::Body),
            vec![Effect::Focus(focused("a", Slot::Body)), Effect::Notes],
        );
        assert!(state.shows_notes("a"));
        assert!(!state.shows_notes("b"), "only the card pressed");
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(state.on_press("a", Slot::Body), vec![Effect::Notes]);
        assert!(!state.shows_notes("a"));
    }

    /// A card with no notes still opens, on the line of its id and uuid
    /// alone, and a second press closes it.
    #[test]
    fn a_card_with_no_notes_opens_on_its_id_line() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(
            state.on_press("a", Slot::Body),
            vec![Effect::Focus(focused("a", Slot::Body)), Effect::Notes],
        );
        assert!(state.shows_notes("a"));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(state.on_press("a", Slot::Body), vec![Effect::Notes]);
        assert!(!state.shows_notes("a"));
    }

    /// A click on another card's body opens its notes and brings the focus,
    /// and with it the action row, the hint and the scroll, to that card.
    /// Closing them again leaves the focus where it is.
    #[test]
    fn opening_another_cards_notes_moves_the_focus_to_it() {
        let mut state = keyboard(vec![noted("a"), noted("b")]);
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(
            state.on_press("b", Slot::Body),
            vec![Effect::Focus(focused("b", Slot::Body)), Effect::Notes],
        );
        assert!(state.shows_notes("b"));
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
        assert!(state.card_hint("b").starts_with("Space: hide notes"), "{}", state.card_hint("b"));
        assert_eq!(state.on_press("b", Slot::Body), vec![Effect::Notes]);
        assert!(!state.shows_notes("b"));
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref(), "closing does not move it");
    }

    /// Opening a card's notes from one of its own buttons puts the focus on
    /// its body, as Up and Down would, and the move disarms its Remove.
    #[test]
    fn opening_notes_from_an_armed_remove_moves_to_the_body_and_disarms() {
        let mut state = keyboard(vec![noted("a"), noted("b")]);
        state.on_press("b", Slot::Button(Action::Remove));
        assert_eq!(state.armed(), &Armed::Remove("b".into()));
        assert_eq!(
            state.on_press("b", Slot::Body),
            vec![Effect::Focus(focused("b", Slot::Body)), Effect::Notes],
        );
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
        assert_eq!(state.armed(), &Armed::None);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib notes`
Expected: FAIL — `a_press_on_the_body_toggles_its_notes`, `a_card_with_no_notes_opens_on_its_id_line` and `opening_another_cards_notes_moves_the_focus_to_it` fail on `left: [Notes]` vs `right: [Focus(Some(Focus { uuid: …, slot: Body })), Notes]`; `opening_notes_from_an_armed_remove_moves_to_the_body_and_disarms` fails the same way. (If the filter matches nothing, check the module path with `cargo test --lib -- --list | grep notes`.)

- [ ] **Step 3: Implement**

In `src/panel/state.rs`, replace `toggle_notes` (currently lines 566-578) with:

```rust
    /// Show this task's notes on its card, under the line of its id and
    /// uuid, or hide them again. Only while the panel has the keyboard, and
    /// only on a card still there: anywhere else the body still does
    /// nothing. Showing them moves the focus to the card's body, so its
    /// action row, its hint and the scroll follow the card just opened, and
    /// that move disarms as any other does. Hiding them leaves the focus and
    /// anything armed as they are. The keyboard stays either way.
    fn toggle_notes(&mut self, uuid: &str) -> Vec<Effect> {
        if !self.keyboard || !self.all.iter().any(|c| c.uuid.as_deref() == Some(uuid)) {
            return Vec::new();
        }
        if self.notes.remove(uuid) {
            return vec![Effect::Notes];
        }
        self.notes.insert(uuid.to_string());
        // Focus first, so the Notes effect's scroll follows this card.
        let mut effects = self.focus_on(Focus::body(uuid));
        effects.push(Effect::Notes);
        effects
    }
```

And in `on_press`'s doc comment (line 524-525), change

```rust
    /// or the body, which shows the task's notes on the card or hides them
    /// again. Remove only arms itself the first time; the second press runs it.
```

to

```rust
    /// or the body, which shows the task's notes on the card, focusing it, or
    /// hides them again. Remove only arms itself the first time; the second
    /// press runs it.
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib notes`
Expected: PASS for all four, plus the other notes tests (`the_body_hint_follows_the_notes_toggle`, `a_tick_keeps_the_notes_shown`, `a_card_that_leaves_the_list_does_not_reopen`, `giving_the_keyboard_back_hides_the_notes`, `the_peek_shows_no_notes`) unchanged.

Then the whole suite: `cargo test`
Expected: all tests pass. Also `cargo clippy --all-targets` shows no new warnings.

- [ ] **Step 5: Update CONTEXT.md**

In `CONTEXT.md`, the **Task card** entry (lines 79-82) reads:

```
While the task panel has the keyboard, Enter or a click on a card's body shows,
dimmed under its description, the task's id and uuid on one line (`#48 ·
<uuid>`, the uuid alone on a finished task, which has no id) and then its
notes, one per note, until a second press or the keyboard is given back.
```

Append one sentence after it, before the `_Avoid_:` line:

```
Showing them moves the focus to that card's body, its action row and hint
with it; hiding them leaves the focus where it is.
```

- [ ] **Step 6: Check it in the panel**

Installing replaces the user's running daemon with this branch's binary, so ask the user first. With their go-ahead, run `bash install.sh` (it restarts the daemon), press Mod+Alt+Ctrl+T, then click the body of a card other than the focused one. Expected: its notes open, its action row and hint appear on it (the previous card's row disappears), and the whole card scrolls into view. Click it again: notes close, the focus and row stay on it. If the user declines or no niri session is available, say so in the report rather than claiming it was checked.

- [ ] **Step 7: Commit**

```bash
git add src/panel/state.rs CONTEXT.md
git commit -m "$(cat <<'EOF'
feat(panel): focus a card when a click opens its notes

A click on a card's body opened its notes but left the focus, the
action row, the hint and the scroll on the card that had it before.
Opening notes now moves the focus to the opened card's body; closing
them leaves it where it is.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
