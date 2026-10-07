# Card Key Hints and Ctrl+Delete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On the keyboard's task panel, show new users the quick keys (a footer
under the list for the keys every card shares, a hint beside the focused card's
buttons for the ones that change per task) and let Ctrl+Delete delete the
focused task in one press without the list closing.

**Architecture:** Key mapping stays pure in `src/panel/keys.rs`, what a key
does stays pure in `src/panel/state.rs`, and the hint text is a pure function
next to `Action::advance` in `src/panel/actions.rs`. `src/panel/surface.rs`
only sets the hint label in `sync()` and adds one fixed footer label under the
scroller, counted into the surface's height, input region and blur like the
tab bar above it.

**Tech Stack:** Rust, gtk4-rs, gtk4-layer-shell; `cargo test`; the nested-niri
end-to-end suite in `tests/e2e-panel.sh` (run through `tests/all.sh`).

**Spec:** Taskwarrior task `a9c644c8-8d6f-4338-8038-ec4537a4e6cc` — read it
with `task rc.json.array=on a9c644c8-8d6f-4338-8038-ec4537a4e6cc export`; its
description and annotations are the spec. **This plan carries a revision the
user made while it was planned** (see Global Constraints): the spec's
"Decided: Body focused: 'Enter: menu · Ctrl+Enter: … · Ctrl+Del: delete'…"
wording measured up to 69 characters (552px) against about 345px of room
beside the widest action row, so the shared keys moved to a footer and the
hint got a smaller font. Where the spec and this plan differ on the hint's
wording or placement, this plan wins.

## Global Constraints

- The panel's hints show only while the panel has the keyboard, as plain dimmed text (`alpha(TEXT, 0.55)`).
- **Footer** (new, revised spec): one line under the task list, outside the scroller so it does not scroll, reading exactly `Enter: open the menu, or press the button · Ctrl+Del: delete the task`. These keys act on whichever card has the focus, the same on every card.
- **Card hint** (revised spec): the existing `card-hint` label at the end of the focused card's action row. It names only what changes per task and per button: the focused button's key and name (`g: Go to session`, `Del: Remove`, or the bare name for a button with no key, e.g. `Speak`), then `Ctrl+Enter: refine` or `Ctrl+Enter: start working`, joined by ` · `. Leave out Ctrl+Enter when `Action::advance` gives nothing or gives a button the card lacks. Body focused: the Ctrl+Enter part alone (or empty).
- **Font** (revised spec): the card hint and the footer use `9pt "Iosevka Term Extended"`, a point under the panel's 10pt `FONT`.
- The hint must fit beside the widest action row without wrapping, Confirm remove included: at most 49 characters (derivation in Task 2's test).
- Ctrl+Delete deletes the focused task in one press, whichever slot (body or any button) is focused: it runs Remove's own command (`task status <uuid> deleted --yes`), keeps the keyboard and moves the focus to the neighbour card, as Remove's second press does. Plain Delete keeps its two-press confirm.
- Clear all moves from Ctrl+Delete to Ctrl+Shift+Delete. While it is armed, Ctrl+Shift+Delete or Enter confirms it, and Ctrl+Delete does nothing.
- Out of scope: hints on the picker or action menu, panel-wide keys (Esc, 1-5, [ ]) in either hint, a setting to hide the hints, an undo for quick delete.
- Commits: Conventional Commits, `feat(panel): …`/`docs(panel): …`, imperative, lowercase, subject ≤ 72 chars, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Comment style: this repo writes doc comments in plain, full sentences that say *why*; match the surrounding density.

---

### Task 1: Ctrl+Delete deletes the focused task; Clear all moves to Ctrl+Shift+Delete

**Files:**
- Modify: `src/panel/keys.rs` (the `KeyAction` enum, `key_action`, tests)
- Modify: `src/panel/state.rs:264-290` (`on_key`), `:325` (`on_clear_all` doc), new `fn delete` after `fn run` (~`:469`), tests
- Modify: `src/panel/surface.rs:405-430` (`connect_keys`), `:1075-1080` (Clear all's comment and tooltip)
- Modify: `tests/e2e-panel.sh:36-37, 370-371, 387, 395, 397, 416`

**Interfaces:**
- Produces: `pub fn key_action(key: gdk::Key, ctrl: bool, shift: bool) -> KeyAction`; new variant `KeyAction::Delete` (Ctrl+Delete); `KeyAction::ClearAll` is now Ctrl+Shift+Delete.

- [ ] **Step 1: Write the failing keys tests**

In `src/panel/keys.rs`, replace the last test, `ctrl_delete_clears_all_and_delete_alone_removes`, with:

```rust
    /// Ctrl+Delete deletes the focused task, the stronger Delete as
    /// Ctrl+Enter is the stronger Enter; Ctrl+Shift+Delete, stronger again,
    /// is the Waiting tab's Clear all. Delete alone still arms Remove.
    #[test]
    fn ctrl_delete_deletes_and_ctrl_shift_delete_clears_all() {
        for key in [gdk::Key::Delete, gdk::Key::KP_Delete] {
            assert_eq!(key_action(key, true, false), KeyAction::Delete, "{key:?}");
            assert_eq!(key_action(key, true, true), KeyAction::ClearAll, "{key:?}");
            assert_eq!(key_action(key, false, false), KeyAction::Run(Action::Remove), "{key:?}");
        }
    }

    /// Shift only tells Ctrl+Delete from Ctrl+Shift+Delete: Shift+Delete is
    /// still Remove, and Ctrl+Shift+Enter still advances, as it always has.
    #[test]
    fn shift_alone_changes_nothing() {
        assert_eq!(key_action(gdk::Key::Delete, false, true), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::Return, true, true), KeyAction::Advance);
        assert_eq!(key_action(gdk::Key::Escape, false, true), KeyAction::Release);
    }
```

Then give every other existing call in the tests the new argument (the
function's own signature does not match this pattern):

```bash
sed -i -E 's/key_action\((gdk::Key::[A-Za-z_0-9]+|key), (true|false)\)/key_action(\1, \2, false)/g' src/panel/keys.rs
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib panel::keys`
Expected: compile error — `key_action` takes 2 arguments but 3 were supplied, and no variant `KeyAction::Delete`.

- [ ] **Step 3: Implement the mapping**

In `KeyAction`, replace the `ClearAll` variant and its doc with:

```rust
    /// Ctrl+Delete: delete the focused card's task at once, with no Confirm
    /// remove, from whichever of its slots has the focus, keeping the
    /// keyboard. The stronger Delete, as Ctrl+Enter is the stronger Enter.
    Delete,
    /// Ctrl+Shift+Delete: press the Waiting tab's Clear all, which sits with
    /// the tabs outside the focus chain, so no focused button can stand for
    /// it. Stronger again than Ctrl+Delete, as it deletes every waiting task.
    ClearAll,
```

Replace `key_action`'s doc tail and its Ctrl branch:

```rust
/// Map a keypress to what it should do. The letters work without a modifier,
/// because nothing on the panel takes typing. With Caps Lock on the keyval
/// arrives as a capital (`S`, not `s`), so the key is lowercased first and
/// capitals press the same buttons. Ctrl is the exception: Ctrl+Enter advances
/// the task, Ctrl+Delete deletes it and Ctrl+Shift+Delete clears the Waiting
/// tab, and every other Ctrl chord is left alone so Ctrl+T cannot Stop. Shift
/// matters only there.
pub fn key_action(key: gdk::Key, ctrl: bool, shift: bool) -> KeyAction {
    if ctrl {
        return match key {
            gdk::Key::Return | gdk::Key::KP_Enter => KeyAction::Advance,
            gdk::Key::Delete | gdk::Key::KP_Delete if shift => KeyAction::ClearAll,
            gdk::Key::Delete | gdk::Key::KP_Delete => KeyAction::Delete,
            _ => KeyAction::Ignore,
        };
    }
```

(the rest of the function is unchanged).

- [ ] **Step 4: Run the keys tests**

Run: `cargo test --lib panel::keys`
Expected: compile errors now come from `state.rs` (non-exhaustive match: `KeyAction::Delete` not covered) and `surface.rs` (2 arguments). That is the next step's work; the keys module itself is done.

- [ ] **Step 5: Write the failing state tests**

In `src/panel/state.rs` tests, add `KeyAction::Delete` to the list in
`a_cards_keys_do_nothing_while_clear_all_is_armed`:

```rust
        for k in [KeyAction::Run(Action::Edit), KeyAction::Run(Action::Remove), KeyAction::Advance, KeyAction::Delete, KeyAction::Ignore] {
```

Add after `a_tab_key_switches_tab_and_disarms_clear_all`:

```rust
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
```

And a new section before `// ─── arming ───`:

```rust
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
```

- [ ] **Step 6: Implement it in the state**

In `on_key`, the armed Clear all branch's last arm becomes:

```rust
                KeyAction::Run(_) | KeyAction::Advance | KeyAction::Delete | KeyAction::Ignore => Vec::new(),
```

and the unarmed match gains, after `KeyAction::Advance => self.advance(),`:

```rust
            KeyAction::Delete => self.delete(),
```

`on_clear_all`'s doc: change "Clear all, by its button or Ctrl+Delete." to
"Clear all, by its button or Ctrl+Shift+Delete."

Add after `fn run`:

```rust
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
```

- [ ] **Step 7: Pass Shift from the window, and fix Clear all's tooltip**

In `src/panel/surface.rs` `connect_keys`, replace the comment and the
`key_action` call:

```rust
            // Alt and Super chords belong to the compositor and the focused
            // widget, not to the letters. Ctrl chords do too, bar Ctrl+Enter,
            // Ctrl+Delete and Ctrl+Shift+Delete, which key_action picks out:
            // Ctrl+T must not Stop.
```

```rust
            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
            let action = keys::key_action(key, ctrl, shift);
```

In `tab_bar()`, the comment's "which is why Ctrl+Delete presses it." becomes
"which is why Ctrl+Shift+Delete presses it.", and:

```rust
    clear.set_tooltip_text(Some("Ctrl+Shift+Delete"));
```

- [ ] **Step 8: Run the unit tests**

Run: `cargo test`
Expected: all pass, including the new `ctrl_delete_*`, `a_second_clear_all_key_confirms_it` and `shift_alone_changes_nothing`.

- [ ] **Step 9: Move the end-to-end test to the new chord**

In `tests/e2e-panel.sh`:
- Both `"${NENV[@]}" wtype -M ctrl -k Delete -m ctrl` lines (Clear all's first press, ~:387, and the re-arm, ~:416) become
  `"${NENV[@]}" wtype -M ctrl -M shift -k Delete -m shift -m ctrl`
- In the header comment (~:37), "Ctrl+Delete and Enter for the Waiting tab's Clear all" → "Ctrl+Shift+Delete and Enter for the Waiting tab's Clear all".
- In the Clear all section comment (~:370), "and Ctrl+Delete presses it" → "and Ctrl+Shift+Delete presses it".
- In the `ok`/`bad` messages (~:395, :397), "the first Ctrl+Delete" → "the first Ctrl+Shift+Delete".

Check nothing is left: `grep -n "Ctrl+Delete\|-k Delete" tests/e2e-panel.sh` should show only the two new `-M shift` lines.

- [ ] **Step 10: Commit**

```bash
git add src/panel/keys.rs src/panel/state.rs src/panel/surface.rs tests/e2e-panel.sh
git commit -m "$(cat <<'EOF'
feat(panel): delete the focused task with Ctrl+Delete

Ctrl+Delete now deletes the focused card's task in one press, from any
of its buttons, keeping the list up on the next card; plain Delete still
asks first. Clear all moves to Ctrl+Shift+Delete to make room.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: The card hint's text, and the footer's

**Files:**
- Modify: `src/panel/actions.rs` (new `Action::hint` and private `Action::key` after `Action::advance`; new `CARD_KEYS` const by `CLEAR_ALL`; tests)

**Interfaces:**
- Consumes: `Action::advance(TaskState) -> Option<Action>`, `Action::letter`, `Action::label(up_next)`, `Action::row(TaskState)`.
- Produces: `pub fn Action::hint(state: TaskState, row: &[Action], focused: Option<Action>) -> String` (associated fn on `crate::actions::Action`, defined in `src/panel/actions.rs`); `pub const CARD_KEYS: &str` in `src/panel/actions.rs` (`super::actions::CARD_KEYS` from `surface.rs`).

- [ ] **Step 1: Write the failing tests**

Add to the tests module in `src/panel/actions.rs`:

```rust
    fn hint(state: TaskState, focused: Option<Action>) -> String {
        Action::hint(state, &Action::row(state), focused)
    }

    /// Up and Down land on the body: what Ctrl+Enter does to this task.
    #[test]
    fn the_body_hints_what_ctrl_enter_does() {
        assert_eq!(hint(on_list(false), None), "Ctrl+Enter: refine");
        assert_eq!(hint(active(false), None), "Ctrl+Enter: refine");
        assert_eq!(hint(on_list(true), None), "Ctrl+Enter: start working");
    }

    #[test]
    fn the_body_of_a_task_ctrl_enter_leaves_alone_hints_nothing() {
        assert_eq!(hint(active(true), None), "");
        assert_eq!(hint(waiting(false), None), "");
        assert_eq!(hint(with_claude(waiting(true)), None), "");
    }

    #[test]
    fn a_lettered_button_hints_its_letter_and_name() {
        assert_eq!(hint(with_claude(on_list(true)), Some(Session)), "g: Go to session · Ctrl+Enter: start working");
        assert_eq!(hint(on_list(true), Some(Start)), "s: Start working · Ctrl+Enter: start working");
        assert_eq!(hint(on_list(false), Some(Refine)), "r: Refine · Ctrl+Enter: refine");
        assert_eq!(hint(active(false), Some(Edit)), "e: Edit · Ctrl+Enter: refine");
        assert_eq!(hint(active(true), Some(Stop)), "t: Stop");
        assert_eq!(hint(waiting(false), Some(Back)), "b: Back to list");
    }

    /// Remove's key is Delete, not a letter.
    #[test]
    fn remove_hints_del() {
        assert_eq!(hint(on_list(false), Some(Remove)), "Del: Remove · Ctrl+Enter: refine");
        assert_eq!(hint(waiting(true), Some(Remove)), "Del: Remove");
    }

    /// Only Enter presses Speak, Up next and Waiting, and Enter is the
    /// footer's, so they hint their name alone. Up next reads as it does on
    /// its tooltip.
    #[test]
    fn a_button_with_no_key_hints_its_name_alone() {
        assert_eq!(hint(on_list(false), Some(Speak)), "Speak · Ctrl+Enter: refine");
        assert_eq!(hint(on_list(true), Some(Wait)), "Waiting · Ctrl+Enter: start working");
        let up_next = TaskState { up_next: true, ..on_list(true) };
        assert_eq!(hint(up_next, Some(UpNext)), "Not up next · Ctrl+Enter: start working");
    }

    /// Ctrl+Enter is left out when what it would press is not on the card.
    #[test]
    fn ctrl_enter_is_left_out_when_the_card_lacks_its_button() {
        assert_eq!(Action::hint(on_list(false), &[Edit, Remove], None), "");
        assert_eq!(Action::hint(on_list(true), &[Edit, Remove], Some(Edit)), "e: Edit");
    }

    /// Every state, up next or not, with the body and with each of its
    /// buttons focused.
    fn every_hint() -> Vec<(TaskState, Option<Action>, String)> {
        let mut all = Vec::new();
        for state in every_state() {
            for up_next in [false, true] {
                let state = TaskState { up_next, ..state };
                let row = Action::row(state);
                for focused in std::iter::once(None).chain(row.iter().copied().map(Some)) {
                    all.push((state, focused, Action::hint(state, &row, focused)));
                }
            }
        }
        all
    }

    /// Enter and Ctrl+Delete do the same on every card, so they are the
    /// footer's, and the card's hint leaves them out.
    #[test]
    fn the_hint_leaves_enter_and_ctrl_delete_to_the_footer() {
        for (state, focused, text) in every_hint() {
            assert!(!text.contains("Ctrl+Del"), "{state:?} {focused:?}: {text}");
            assert!(!text.split(" · ").any(|part| part.starts_with("Enter")), "{state:?} {focused:?}: {text}");
        }
        assert_eq!(CARD_KEYS, "Enter: open the menu, or press the button · Ctrl+Del: delete the task");
    }

    /// Room for the hint on the widest row, at the hint's 9pt, where
    /// Iosevka Term Extended is 7px a character: the 760px card, less eight
    /// buttons of one 8px glyph, 24px of padding and a 1px line between
    /// (263px), less the hint's own 24px of padding, less "  Confirm remove"
    /// (16 characters at the buttons' 10pt, 128px) while Remove is armed:
    /// 345px, so 49 characters.
    const HINT_ROOM: usize = 49;

    #[test]
    fn every_hint_fits_beside_the_widest_row() {
        for (state, focused, text) in every_hint() {
            assert!(text.chars().count() <= HINT_ROOM, "{state:?} {focused:?}: {text:?} is {} long", text.chars().count());
        }
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib panel::actions`
Expected: compile error — no function `hint` on `Action`, cannot find `CARD_KEYS`.

- [ ] **Step 3: Implement**

In `impl Action` in `src/panel/actions.rs`, right after `advance`:

```rust
    /// What the focused card's hint reads beside its buttons: the keys that
    /// change from card to card and button to button. The focused button's
    /// key and name ("g: Go to session", "Del: Remove", or just "Speak" for
    /// a button only Enter presses), then what Ctrl+Enter does to this task,
    /// when it presses a button `row` has. Empty on the body of a task
    /// Ctrl+Enter leaves alone. Enter and Ctrl+Delete do the same on every
    /// card, so they are the footer's, [`CARD_KEYS`]. Pure, so every card's
    /// hint is tested without a window; `surface.rs` only sets the label.
    pub fn hint(state: TaskState, row: &[Action], focused: Option<Action>) -> String {
        let mut parts = Vec::new();
        if let Some(action) = focused {
            let name = action.label(state.up_next);
            parts.push(match action.key() {
                Some(key) => format!("{key}: {name}"),
                None => name.to_string(),
            });
        }
        if let Some(step) = Self::advance(state).filter(|a| row.contains(a)) {
            let does = if step == Refine { "refine" } else { "start working" };
            parts.push(format!("Ctrl+Enter: {does}"));
        }
        parts.join(" · ")
    }

    /// The key that presses the button, as the hint names it: its letter, or
    /// Del for Remove. None for a button only Enter presses.
    fn key(self) -> Option<String> {
        match self {
            Remove => Some("Del".to_string()),
            _ => self.letter().map(String::from),
        }
    }
```

After `CONFIRM_CLEAR_ALL`:

```rust
/// The footer under the keyboard's list: the keys that act on whichever card
/// has the focus, the same on every one, so the card's own hint leaves them
/// out.
pub const CARD_KEYS: &str = "Enter: open the menu, or press the button · Ctrl+Del: delete the task";
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib panel::actions`
Expected: PASS. (`dead_code` warnings for `hint`/`CARD_KEYS` outside tests are expected until Task 3.)

- [ ] **Step 5: Commit**

```bash
git add src/panel/actions.rs
git commit -m "$(cat <<'EOF'
feat(panel): word the focused card's key hint and the keys footer

A pure function picks what the focused card's hint names: the focused
button's key and what Ctrl+Enter does to this task. Enter and Ctrl+Del,
the same on every card, go in a footer line instead, so the hint fits
beside the widest action row.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Draw the hint and the footer, a point smaller

**Files:**
- Modify: `src/panel/style.rs` (new `HINT_FONT`; the shared card rule, the `.card-hint` rule, a new `.keys-footer` rule; tests)
- Modify: `src/panel/surface.rs` (imports; `Panel` and `Slide` fields; `Panel::new`; `fit`; `render`; `card_widget`; `ActionRow`; `action_row`; `sync`; `set_region`; `update_blur`; `shown_height`; new `keys_footer`)

**Interfaces:**
- Consumes: `Action::hint(TaskState, &[Action], Option<Action>) -> String` and `actions::CARD_KEYS` from Task 2; `Card::state(has_session: bool) -> Option<TaskState>` (`src/panel/model.rs:73`).
- Produces: `pub const HINT_FONT: &str` in `src/panel/style.rs`; CSS class `keys-footer`.

- [ ] **Step 1: Write the failing style tests**

In `src/panel/style.rs` tests, add to `the_tabs_and_the_empty_line_look_like_cards`, after its last assert:

```rust
        assert!(rule.contains(".task-panel .keys-footer"), "the footer lacks the card look");
```

and add:

```rust
    /// The keys' hints are captions: dimmed, a point under the cards' text,
    /// so more fits beside the buttons. The footer's rule comes after the
    /// card look it shares, whose font would otherwise win.
    #[test]
    fn the_hints_are_dimmed_and_a_point_smaller() {
        let css = css();
        assert!(css.contains(&format!(
            ".task-panel .card-hint {{ color: alpha({TEXT}, 0.55); padding: {ACTION_PADDING}; font: {HINT_FONT}; }}"
        )));
        let footer = format!(
            ".task-panel .keys-footer {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); font: {HINT_FONT}; }}"
        );
        let at = css.find(&footer).unwrap_or_else(|| panic!("missing `{footer}`"));
        let shared = css.find(".task-panel .task-card,").expect("no shared card rule");
        assert!(at > shared, "the card look's font would win over the footer's");
        assert_eq!(HINT_FONT, "9pt \"Iosevka Term Extended\"");
        assert!(FONT.starts_with("10pt "), "the hint is a point under the cards' text");
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib panel::style`
Expected: compile error — cannot find `HINT_FONT`.

- [ ] **Step 3: Implement the style**

After `FONT` in `src/panel/style.rs`:

```rust
/// The keys' hints, beside the focused card's buttons and on the footer under
/// the list: a point under [`FONT`], so they read as captions and the widest
/// hint fits beside the widest action row.
pub const HINT_FONT: &str = "9pt \"Iosevka Term Extended\"";
```

In `css()`, the shared card rule's selector list becomes:

```
.task-panel .task-card,
.task-panel .filter-tabs,
.task-panel .filter-empty,
.task-panel .keys-footer {{
```

The `.card-hint` rule and its comment become:

```
/* The focused card's keys, dimmed and a point smaller so they read as a
   caption to the icons. */
.task-panel .card-hint {{ color: alpha({TEXT}, 0.55); padding: {ACTION_PADDING}; font: {HINT_FONT}; }}
```

After the `.filter-empty` rule:

```
/* The keys every card shares, under the list: a strip like the tab bar,
   its text dimmed and small like the card's hint. */
.task-panel .keys-footer {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); font: {HINT_FONT}; }}
```

Run: `cargo test --lib panel::style` — Expected: PASS.

- [ ] **Step 4: Set the card hint in `sync()`**

In `src/panel/surface.rs`:

Imports: `use crate::actions::Action;` → `use crate::actions::{Action, TaskState};`

`ActionRow`: replace the `hint` doc and the `up_next` field:

```rust
    /// The keys that change from card to card, after the icons: the focused
    /// button's, and what Ctrl+Enter does to this task.
    hint: gtk4::Label,
    buttons: Vec<(Action, gtk4::Button)>,
    /// The task's state, for the hint and for Up next reading Not up next.
    state: TaskState,
```

`card_widget`: the row is built for a card with a state:

```rust
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
```

(`has_session` is `false` here because the row's buttons are already
`shown.actions`; the state only feeds `advance` and `up_next`, which do not
read it.)

`action_row`: signature `fn action_row(self: &Rc<Self>, uuid: &str, state: TaskState, actions: &[Action]) -> ActionRow`; the doc's "then the focused one's name in words: the icons alone do not say what they do." becomes "then the keys that change per card and button: the icons alone do not say what they do, nor which key presses them."; the tooltip is `button.set_tooltip_text(Some(action.label(state.up_next)));`; the return is `ActionRow { separator, row, hint, buttons, state }`.

`sync`: its doc's last line "and the focused button's name." becomes "and its hint for the focused slot."; replace the `let hint = match &focus { … }; row.hint.set_label(hint);` block with:

```rust
            let hint = match &focus {
                Some(Focus { uuid, slot }) if Some(uuid) == card.uuid.as_ref() => {
                    let buttons: Vec<Action> = row.buttons.iter().map(|(a, _)| *a).collect();
                    let focused = match slot {
                        Slot::Button(action) => Some(*action),
                        Slot::Body => None,
                    };
                    Action::hint(row.state, &buttons, focused)
                }
                _ => String::new(),
            };
            // Only when it changed: every sync passes every card, and an
            // unchanged label should not cost a relayout.
            if row.hint.label().as_str() != hint.as_str() {
                row.hint.set_label(&hint);
            }
```

Run: `cargo build` — Expected: builds.

- [ ] **Step 5: Add the footer and count its height**

`Panel` struct, after `clear`:

```rust
    /// The keys every card shares, under the scroller so it stays put while
    /// the cards scroll, shown only while the panel has the keyboard.
    footer: gtk4::Label,
```

`Slide`, after `tabs_h` (and `footer_h: Cell::new(0),` in `Slide::tucked`):

```rust
    /// The footer's height, with the gap over it: what sits under the cards.
    /// 0 without the keyboard, which has no footer.
    footer_h: Cell<i32>,
```

`Panel::new`: after `front.append(&scroller);` add

```rust
        let footer = keys_footer();
        front.append(&footer);
```

add `footer,` to the `Panel { … }` initialiser after `clear,`, and the overlay
child's height becomes:

```rust
                    slide.tabs_h.get() + slide.cards_h.get() + slide.footer_h.get() + 2 * RING_PX,
```

`fit`: after `tabs_h` is measured add

```rust
        // The footer, measured the same way: margins included, so the gap
        // over it and the ring room under it.
        let footer_h = if keyboard {
            self.footer.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX).1
        } else {
            0
        };
```

and replace from `let shown = …` to `let height = …` with

```rust
        let shown = shown_height(cards_h, tabs_h + footer_h, self.monitor.geometry().height());
        self.slide.tabs_h.set(tabs_h);
        self.slide.footer_h.set(footer_h);
        self.slide.cards_h.set(shown);
        let height = tabs_h + shown + footer_h + 2 * SHADOW_PX;
```

The `fit` doc's "Measure the cards and the tabs" → "Measure the cards, the tabs and the footer".

`render`: after `self.tabs.set_visible(keyboard);` add `self.footer.set_visible(keyboard);`.

`set_region`: the comment and height become

```rust
        // From the top of the tabs, when there are any, to the bottom of the
        // footer under the cards on screen.
        let height = self.slide.tabs_h.get() + self.slide.cards_h.get() + self.slide.footer_h.get();
```

`update_blur`: after `rects.extend(blur::clip_rows(…));` add

```rust
        let footer_h = self.slide.footer_h.get();
        if footer_h > 0 {
            // The footer, which does not scroll: under the cards on screen
            // and the card gap, not the gap itself.
            let y = top + self.slide.cards_h.get() + GAP_PX;
            rects.extend(blur::card_region((x, y, width, footer_h - GAP_PX), RADIUS_PX, on_screen));
        }
```

`shown_height`: rename the parameter `tabs_h` to `bars_h` and its doc's
"under the `tabs_h` the filter tabs take" to "less the `bars_h` the filter tabs
over them and the keys footer under them take" (body: `- bars_h`). Its tests
are positional and stay as they are.

After `tab_bar()`, add:

```rust
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
```

- [ ] **Step 6: Run the unit tests and clippy**

Run: `cargo test && cargo clippy --all-targets 2>&1 | tail -20`
Expected: all tests pass; no new warnings (the Task 2 `dead_code` warnings are gone).

- [ ] **Step 7: Run the end-to-end suites**

Run: `cargo build --release && tests/all.sh`
Expected: every suite that runs passes. The panel suite measures heights
relative to one another, so the footer's extra height should not break it; if a
check that compares absolute heights fails, read the screenshot it names and fix
the check's expectation only if the footer is the whole difference. Report any
suite `tests/all.sh` SKIPs, with its reason, rather than calling it passed.

- [ ] **Step 8: Look at it**

Use the `run` skill to start this worktree's panel (or, with the daemon
installed from this branch, press Mod+Alt+Ctrl+T on a workspace with an
unplanned, a planned and an active planned task, and a waiting one). Check:
- The footer under the cards reads `Enter: open the menu, or press the button · Ctrl+Del: delete the task`, in smaller dimmed text, and stays put while a long list scrolls; the blur sits behind it.
- On the body: `Ctrl+Enter: refine` (unplanned), `Ctrl+Enter: start working` (planned), nothing (active planned, waiting).
- Right along the row, the hint follows the focus (`s: Start working · …`, `Speak · …`, `Del: Remove · …`), on one line, also with Remove armed as Confirm remove on a card with Go to session.
- Ctrl+Delete from any button deletes that task and the list stays up on the next card.
- On the Waiting tab, Ctrl+Shift+Delete arms Clear all and a second Ctrl+Shift+Delete (or Enter) confirms; Ctrl+Delete does nothing while it is armed.

- [ ] **Step 9: Commit**

```bash
git add src/panel/style.rs src/panel/surface.rs
git commit -m "$(cat <<'EOF'
feat(panel): show the focused card's keys and a keys footer

The focused card's hint now names the focused button's key and what
Ctrl+Enter does to the task, on the body too, and a footer under the
list names Enter and Ctrl+Del. Both are a point smaller than the cards'
text so the hint fits beside the widest action row.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Document the hints, Ctrl+Delete and Ctrl+Shift+Delete

**Files:**
- Modify: `README.md` (:20 the `Mod+Alt+Ctrl+T` row; :59-60 the Remove and Clear all rows; :62-66 the "Up and Down…" paragraph; :75-82 the Ctrl+Enter paragraph; :93-100 the tabs paragraph; :445 the e2e paragraph)
- Modify: `CONTEXT.md` (**Action row** and **Filter tab** entries)
- Modify: `src/panel/surface.rs` module doc (`//!` lines ~33-83)

- [ ] **Step 1: README key table**

Line 20, in the `Mod+Alt+Ctrl+T` row, replace
`and the Waiting tab's Clear all (Ctrl+Delete) deletes every waiting task.`
with
`and the Waiting tab's Clear all (Ctrl+Shift+Delete) deletes every waiting task. Ctrl+Delete deletes the focused task at once. A hint beside the focused card's buttons, and a line under the list, name the keys.`

The Remove row becomes:

```
| **Remove** (red) | `Delete` | Turns into **Confirm remove**; a second press deletes the task, moving away puts it back. `Ctrl+Delete` deletes the focused task at once, whichever of its buttons is focused, and moves on to the next card |
```

In the Clear all row, `` `Ctrl+Delete` `` → `` `Ctrl+Shift+Delete` ``.

- [ ] **Step 2: README keyboard paragraphs**

In "Up and Down move a darker fill…", replace
`Right and Tab move along the focused card's buttons, whose name shows beside them while one is focused, and Enter presses the focused one.`
(it wraps over lines; match it across the line breaks) with
`Right and Tab move along the focused card's buttons, and Enter presses the focused one. Beside the buttons, in smaller dimmed text, the focused card names its own keys: the focused button's letter and name (`g: Go to session`, `Del: Remove`), and what Ctrl+Enter would do to this task. A line under the list names the keys that act the same on every card: Enter, and Ctrl+Delete.`

Replace the whole "Ctrl+Enter is the one key that does not give the keyboard back. …" paragraph with:

```
Ctrl+Enter and Ctrl+Delete act on the focused card, whichever of its buttons is
focused, without giving the keyboard back. Ctrl+Enter refines the task, or
starts working on it once it is planned, and the panel stays up with the same
card focused, so you can carry on down the list. It does nothing on a task that
is already being worked, or on a waiting task. Ctrl+Delete deletes the task at
once, with no Confirm remove, and moves the focus to the next card (the one
above, from the last) with the list kept up; Delete alone still asks first.
Ctrl still held from the `Mod+Alt+Ctrl+T` chord counts, so let go of Ctrl
before pressing Enter if you only want the card's menu, or Delete if you want
Remove to ask. Apart from Ctrl+Shift+Delete, the Waiting tab's Clear all
(below), every other Ctrl chord passes through untouched.
```

In the tabs paragraph, replace
`which `Ctrl+Delete` presses (`Delete` alone is still the focused card's Remove).`
with
`which `Ctrl+Shift+Delete` presses (`Ctrl+Delete` and `Delete` are still the focused card's).`
and
`` `Enter` confirms as a second `Ctrl+Delete` does; a card's own keys do nothing ``
with
`` `Enter` confirms as a second `Ctrl+Shift+Delete` does; a card's own keys, `Ctrl+Delete` among them, do nothing ``.

Line ~445: `on its first Ctrl+Delete` → `on its first Ctrl+Shift+Delete`.

- [ ] **Step 3: CONTEXT.md**

**Action row**: after "…each running what the same entry in the task's action
menu runs." add (same paragraph):
`After the buttons, dimmed, the hint names the keys that change from card to card: the focused button's, and what Ctrl+Enter does to the task. The keys every card shares, Enter and Ctrl+Delete, are on a line under the list instead.`

**Filter tab**: "The Waiting tab alone ends in Clear all, which deletes every
task under it on a second press, as Remove does one." → "The Waiting tab alone
ends in Clear all (Ctrl+Shift+Delete), which deletes every task under it on a
second press, as Remove does one."

- [ ] **Step 4: `surface.rs` module doc**

In the "## The keyboard" section:
- After "…and g, b, s, r, e, t and Delete press its Go to session, Back to list, Start, Refine, Edit, Stop and Remove." add: "Ctrl+Delete deletes the focused card's task at once and keeps the keyboard. After the buttons a dimmed hint names the keys that change per card, the focused button's and what Ctrl+Enter does to the task (`Action::hint`); a footer under the scroller names Enter and Ctrl+Delete, which act alike on every card."
- In the Clear all paragraph, "so Ctrl+Delete presses it; Delete alone is still the focused card's Remove." → "so Ctrl+Shift+Delete presses it; Ctrl+Delete and Delete are still the focused card's."
- In the scroller paragraph, "capped at the screen's height less its margins and the tabs." → "capped at the screen's height less its margins, the tabs and the footer."

- [ ] **Step 5: Check nothing still names the old chord**

Run: `grep -rn "Ctrl+Delete" README.md CONTEXT.md src tests | grep -v "Ctrl+Shift+Delete"`
Expected: only lines where Ctrl+Delete means the one-press delete of the focused task. Then `cargo test` — Expected: PASS (docs only, but doc comments compile).

- [ ] **Step 6: Commit**

```bash
git add README.md CONTEXT.md src/panel/surface.rs
git commit -m "$(cat <<'EOF'
docs(panel): document the key hints and the Delete chords

Ctrl+Delete deletes the focused task at once, Clear all is now
Ctrl+Shift+Delete, and the panel names its keys beside the focused
card's buttons and under the list.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
