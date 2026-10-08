# Toggle a Card's Notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While the task panel has the keyboard, Enter or a click on a task card's body shows that task's notes on the card, dimmed under the description, and a second press hides them.

**Architecture:** `model::Card` gains `notes: Vec<String>`, the annotations' text, so the daemon's cards carry them and a changed note re-renders like any other change. `PanelState` keeps a `HashSet<String>` of the uuids whose notes show; `on_press(uuid, Slot::Body)` toggles it and returns a new `Effect::Notes`, and `release()` clears it. `surface.rs` builds each keyboard card's notes into its body button, hidden unless `PanelState::shows_notes` says otherwise, and on `Effect::Notes` sets their visibility and refits the surface, input region and blur exactly as `show_row` does.

**Tech Stack:** Rust, gtk4 (gtk4-rs), gtk4-layer-shell; bash e2e in a nested niri with wtype.

**Spec:** Taskwarrior task `b96c1f78-2995-4bc1-8272-bf863b777cb0` ("feat: Toggle a card's notes with Enter or a click"); read it with `task rc.json.array=on b96c1f78-2995-4bc1-8272-bf863b777cb0 export`. Its notes are the spec:

- Goal: On the keyboard task panel, Enter on a card's body or a click on it shows or hides that task's notes on the card, so they can be read without opening the task box. From the 2026-10-07 slim-down decision: a card click toggles its notes.
- Context: Today `on_press(uuid, Slot::Body)` in `src/panel/state.rs` returns no effects (test `a_press_on_the_body_does_nothing`), and CONTEXT.md:91 and README.md:72 say Enter or a click on the body does nothing.
- Context: Cards carry no notes, so pass the annotations' text through to them. Showing notes changes the card's height, so `fit`, `set_region`, `update_blur` and `follow_focus` must follow, as the action row's `show_row` does in `src/panel/surface.rs`.
- Context: The keys footer `actions::CARD_KEYS` reads `Enter: press the button · Ctrl+Del: delete the task` (asserted in actions.rs tests); reword its Enter part to cover the body.
- Decided: Only while the panel has the keyboard; the peek (no keyboard, one-line cards) is unchanged.
- Decided: Which cards show their notes is panel state (a set of uuids in `PanelState`), so a re-render or refresh tick keeps them open; giving the keyboard back clears it.
- Decided: Notes show under the description as dimmed wrapped lines, one per note, text only without dates. A card with no notes ignores the press.
- Done when: Enter or a click on a focused card's body shows its notes and a second press hides them, the panel, input region and blur fit the new height, a tick keeps them shown, `cargo test` passes, and the footer, CONTEXT.md and README describe it.
- Out of scope: Editing notes from the panel (the Edit button's task box does that), and folding Grill me, Complete and Move into the panel.

**How Enter reaches the body today (no key-map change needed):** on the Tasks mode, `PanelState::on_action(KeyAction::Enter)` returns `None`, so GTK keeps the key and activates the focused widget. On a card's body that is the body `gtk4::Button`, whose `connect_clicked` handler (`surface.rs`, in `card_widget`) calls `on_press(uuid, Slot::Body)`. So Enter and a click both land in `on_press`; only `on_press` and the surface's drawing change.

## Global Constraints

- Commit messages are Conventional Commits, `<type>(<scope>): <summary>`, imperative, lowercase first word, no full stop, subject ≤ 72 chars (aim ~50), body wrapped at 72 saying what and why. This task is a `feat`, so its commits are `feat(panel): …` (docs and tests folded in). End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- `cargo test` passes and `cargo build` has no warnings after every task.
- Work in this worktree branch, `task/feat-toggle-a-card-s-notes-with-enter-b96c1f78` (`/home/paul/.worktrees/niri-tasks/task-feat-toggle-a-card-s-notes-with-enter-b96c1f78`). The `finish-worktree` skill lands it on `main` at the end — not `superpowers:finishing-a-development-branch`.
- **Never run `install.sh`, `cargo install`, or `systemctl --user restart niri-tasks`** from the worktree: that swaps the user's live daemon onto this branch's build. Run the e2e script only as `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`. If the machine cannot run it (no Wayland session, no wtype, no Pillow), say so in the report rather than claiming it ran.
- The peek (no keyboard) never shows notes and draws exactly as before.
- Notes are text only: no dates, one wrapped dimmed line (or more, when it wraps) per note, under the description.
- Vocabulary (CONTEXT.md): the user-facing word is "notes" / "note"; "annotation" is Taskwarrior's storage word, to be avoided in docs and UI text. In code the `Task` field stays `annotations`.
- Comments are full sentences that say why, in the house style of the surrounding code (plain British English, no jargon); docs on every new pub item and every new field.
- Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`.

---

### Task 1: Cards carry their task's notes

**Files:**
- Modify: `src/panel/model.rs` — `Card` (lines 30–51), `cards()` (the `Card { … }` at ~line 257), `cap()` (the "+N more" `Card { … }` at ~line 290), test helper at ~line 673, `mod tests` (add a test).
- Modify: `src/panel/state.rs` — the test helper `card()` at ~line 845 (it builds a `Card` literal, so it must gain the field to compile).

**Interfaces:**
- Produces: `pub notes: Vec<String>` on `model::Card` — each note's text, in the task's order, empty on "+N more" and on a task with none. Task 2 reads it.

- [ ] **Step 1: Write the failing test**

In `src/panel/model.rs`'s `mod tests`, after `descriptions_are_one_line`, add:

```rust
    /// A card carries its task's notes, in order and text only, for the
    /// keyboard's panel to show under the description; "+N more" has none.
    #[test]
    fn a_card_carries_its_tasks_notes() {
        let mut noted = task("n", 2, false);
        noted.annotations = vec![
            crate::task::Annotation { entry: "20261001T120000Z".into(), description: "first note".into() },
            crate::task::Annotation { entry: "20261002T120000Z".into(), description: "second note".into() },
        ];
        let cards = cards(&[noted, task("plain", 1, false)], &[]);
        assert_eq!(cards[0].notes, vec!["first note".to_string(), "second note".to_string()]);
        assert!(cards[1].notes.is_empty());
        let capped = cap(&cards, 1);
        assert!(capped[1].notes.is_empty(), "+N more stands for no one task");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib panel::model::tests::a_card_carries_its_tasks_notes`
Expected: compile error, `no field `notes` on type `Card``.

- [ ] **Step 3: Add the field and fill it**

In `Card`, after `since`:

```rust
    /// The task's notes, oldest first, text only: what the keyboard's panel
    /// shows under the description once the card's body is pressed. Empty on
    /// "+N more" and on a task with none.
    pub notes: Vec<String>,
```

In `cards()`, inside the `Card { … }` literal after `since: …,`:

```rust
            notes: t.annotations.iter().map(|a| a.description.clone()).collect(),
```

In `cap()`'s "+N more" literal after `since: String::new(),`:

```rust
            notes: Vec::new(),
```

In the `model.rs` test closure at ~line 673, append `, notes: Vec::new()` inside the `Card { … }` after `since: String::new()`.

In `src/panel/state.rs`'s test helper `card()`:

```rust
    fn card(uuid: &str, status: Status) -> Card {
        Card {
            status,
            text: uuid.into(),
            uuid: Some(uuid.into()),
            planned: status == Status::Planned,
            up_next: false,
            since: String::new(),
            notes: Vec::new(),
        }
    }
```

Then `grep -rn "since: String::new()\|since: if" src tests` to confirm no other `Card` literal was missed (there should be none beyond these four).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: all pass, `a_card_carries_its_tasks_notes` among them; `cargo build` shows no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/panel/model.rs src/panel/state.rs
git commit -m "feat(panel): carry a task's notes on its card

The keyboard's panel is to show a task's notes on its card, so the
daemon's cards now hold their annotations' text, oldest first. A
changed note re-renders the panel like any other change to a card.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: A press on the body toggles the card's notes

**Files:**
- Modify: `src/panel/state.rs` — imports (top), `Effect` (lines ~74–101), `PanelState` fields (~118–144) and accessors, `on_press` (~466–501) and its doc comment, `release` (~593–608), `mod tests` (replace `a_press_on_the_body_does_nothing` at ~1262, add toggle tests).
- Modify: `src/panel/surface.rs` — `CardWidgets` (~262–273), `apply` (~669–702), `card_widget` (~999–1058) and its doc comment, a new `show_notes` beside `show_row` (~1248), a new free fn `notes_box` beside `card_label` (~1414).
- Modify: `src/panel/style.rs` — the CSS beside `.card-age` (~line 194) and a test beside `the_age_is_dimmed` (~line 510).

**Interfaces:**
- Consumes: `Card::notes: Vec<String>` (Task 1).
- Produces:
  - `Effect::Notes` (unit variant) — "show or hide each card's notes as `shows_notes` says, and refit".
  - `pub fn shows_notes(&self, uuid: &str) -> bool` on `PanelState` — true while the panel has the keyboard and that uuid's notes are toggled on.
  - CSS classes `card-notes` (the box under the description) and `card-note` (each note's label).

- [ ] **Step 1: Write the failing tests**

In `src/panel/state.rs`'s `mod tests`, add a helper beside `pending()`:

```rust
    /// A pending card whose task has one note.
    fn noted(uuid: &str) -> Card {
        Card { notes: vec![format!("a note on {uuid}")], ..card(uuid, Status::Pending) }
    }
```

Replace the whole `a_press_on_the_body_does_nothing` test (its `#[test]` attribute and body, ~lines 1262–1268) with:

```rust
    /// A press on a card's body, Enter or a click, shows its task's notes
    /// and a second hides them, keeping the keyboard and the focus.
    #[test]
    fn a_press_on_the_body_toggles_its_notes() {
        let mut state = keyboard(vec![noted("a"), noted("b")]);
        assert_eq!(state.on_press("a", Slot::Body), vec![Effect::Notes]);
        assert!(state.shows_notes("a"));
        assert!(!state.shows_notes("b"), "only the card pressed");
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(state.on_press("a", Slot::Body), vec![Effect::Notes]);
        assert!(!state.shows_notes("a"));
    }

    /// A card with no notes has nothing to show, so its body still does
    /// nothing.
    #[test]
    fn a_card_with_no_notes_ignores_the_press() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.on_press("a", Slot::Body), Vec::new());
        assert!(!state.shows_notes("a"));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    /// The peek's cards are one line each: a click on one shows nothing.
    #[test]
    fn the_peek_shows_no_notes() {
        let mut state = PanelState::default();
        state.set_cards(&[noted("a")]);
        assert_eq!(state.on_press("a", Slot::Body), Vec::new());
        assert!(!state.shows_notes("a"));
    }

    /// Shown notes are the panel's, not the widgets', so the daemon's tick
    /// keeps them: unchanged cards draw nothing, and changed ones re-render
    /// with the notes still shown.
    #[test]
    fn a_tick_keeps_the_notes_shown() {
        let mut state = keyboard(vec![noted("a"), noted("b")]);
        state.on_press("a", Slot::Body);
        assert_eq!(state.set_cards(&[noted("a"), noted("b")]), Vec::new());
        assert!(state.shows_notes("a"));
        assert_eq!(state.set_cards(&[noted("a"), noted("b"), noted("c")]), vec![Effect::Render]);
        assert!(state.shows_notes("a"));
    }

    /// Giving the keyboard back folds every card to one line again, notes
    /// and all, and the next time the keyboard is taken they start hidden.
    #[test]
    fn giving_the_keyboard_back_hides_the_notes() {
        let mut state = keyboard(vec![noted("a")]);
        state.on_press("a", Slot::Body);
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
        assert!(!state.shows_notes("a"));
        assert!(state.take_keyboard(Vec::new()));
        assert!(!state.shows_notes("a"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib panel::state`
Expected: compile errors, `no variant named `Notes`` and `no method named `shows_notes``.

- [ ] **Step 3: Implement the state**

At the top of `src/panel/state.rs`, add beside the other `use` lines:

```rust
use std::collections::HashSet;
```

In `enum Effect`, after `ClearQuery`:

```rust
    /// Show or hide each card's notes as [`PanelState::shows_notes`] says,
    /// and fit the panel to the cards' new heights.
    Notes,
```

In `struct PanelState`, after `focus`'s field (before `armed`):

```rust
    /// The tasks whose cards show their notes, by uuid: a press on a card's
    /// body adds it, a second takes it out. Kept across a re-render, so a
    /// tick leaves them open; emptied as the keyboard is given back.
    notes: HashSet<String>,
```

In `impl PanelState`, after `pub fn armed(…)`:

```rust
    /// This task's card shows its notes: only ever while the panel has the
    /// keyboard, the peek's cards being one line.
    pub fn shows_notes(&self, uuid: &str) -> bool {
        self.keyboard && self.notes.contains(uuid)
    }
```

Rewrite `on_press`'s doc comment's second sentence and its `let Slot::Button…` line. The doc comment becomes:

```rust
    /// A press on a task's card: a button, as a click or a key presses it,
    /// or the body, which shows the task's notes on the card or hides them
    /// again. Remove only arms itself the first time; the second press runs it.
```

(keep the rest of the existing comment, from "Everything that opens something…" on, unchanged), and in the body replace

```rust
        let Slot::Button(action) = slot else { return Vec::new() };
```

with

```rust
        let action = match slot {
            Slot::Body => return self.toggle_notes(uuid),
            Slot::Button(action) => action,
        };
```

Add a private method after `on_press`:

```rust
    /// Show this task's notes on its card, or hide them again. Only while
    /// the panel has the keyboard, and only on a card with notes to show:
    /// anywhere else the body still does nothing. The keyboard, the focus
    /// and anything armed stay as they are.
    fn toggle_notes(&mut self, uuid: &str) -> Vec<Effect> {
        let has_notes = self.all.iter().any(|c| c.uuid.as_deref() == Some(uuid) && !c.notes.is_empty());
        if !self.keyboard || !has_notes {
            return Vec::new();
        }
        if !self.notes.remove(uuid) {
            self.notes.insert(uuid.to_string());
        }
        vec![Effect::Notes]
    }
```

In `release()`, after `self.focus = None;`:

```rust
        self.notes.clear();
```

and change `release`'s doc comment first line to "Give the keyboard back: every card to one line again, its notes hidden, All for the next".

- [ ] **Step 4: Run the state tests to verify they pass**

Run: `cargo test --lib panel::state`
Expected: compile error in `src/panel/surface.rs`: `non-exhaustive patterns: `Effect::Notes` not covered`. That is the surface's half, next.

- [ ] **Step 5: Draw the notes in the surface**

In `struct CardWidgets`, after `row`:

```rust
    /// The task's notes under the description, shown while the state says.
    /// None on "+N more", on a task with no notes, and on every card off
    /// the keyboard.
    notes: Option<gtk4::Box>,
```

In `apply`, add an arm after `Effect::ClearQuery => …`:

```rust
                // The card grew or shrank: keep the whole of the focused one
                // in view, as a move of the focus does.
                Effect::Notes => {
                    self.show_notes();
                    self.follow_focus();
                }
```

In `card_widget`, replace

```rust
        let (label, age) = card_label(card, keyboard);
        let body = gtk4::Button::builder().child(&label).build();
```

with

```rust
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
```

and change the struct literal at the end of `card_widget` to

```rust
        CardWidgets { uuid: card.uuid.clone(), root, body, row, notes, age }
```

Update `card_widget`'s doc comment: replace "The body does nothing on a task's card, and shows the rest in place of "+N more"." with "The body shows or hides the task's notes on a task's card, and shows the rest in place of "+N more"."

After `show_row`, add:

```rust
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
```

After the free fn `card_label`, add:

```rust
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
```

- [ ] **Step 6: Write the failing style test**

In `src/panel/style.rs`'s `mod tests`, after `the_age_is_dimmed`:

```rust
    /// A card's notes are dimmed like its age, and stand a card's padding
    /// below the description.
    #[test]
    fn the_notes_are_dimmed_under_the_description() {
        let css = css();
        assert!(css.contains(&format!(".task-panel .card-notes {{ padding-top: {PADDING_PX}px; }}")));
        assert!(css.contains(&format!(".task-panel .card-note {{ color: alpha({TEXT}, 0.55); }}")));
    }
```

Run: `cargo test --lib panel::style::tests::the_notes_are_dimmed_under_the_description`
Expected: FAIL on the first assertion.

- [ ] **Step 7: Add the CSS**

In the CSS string in `src/panel/style.rs`, right after the `.task-panel .card-age { … }` line (~194), add:

```
/* A card's notes, once its body is pressed: captions to the description,
   so dimmed like its age, a padding's gap below it. */
.task-panel .card-notes {{ padding-top: {PADDING_PX}px; }}
.task-panel .card-note {{ color: alpha({TEXT}, 0.55); }}
```

- [ ] **Step 8: Run all tests and build**

Run: `cargo build 2>&1 | grep -c warning; cargo test`
Expected: `0` warnings; every test passes, the five new state tests and the style test among them.

- [ ] **Step 9: Commit**

```bash
git add src/panel/state.rs src/panel/surface.rs src/panel/style.rs
git commit -m "feat(panel): toggle a card's notes with Enter or a click

On the keyboard's panel, a press on a task card's body shows the
task's notes under its description, dimmed, and a second hides them,
so they can be read without opening the task box. Which cards show
them is the panel's state, kept across a tick and cleared as the
keyboard is given back; the peek never shows them. The surface, input
region and blur are refitted as the card grows, as for the action row.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The footer, the docs and the e2e say so

**Files:**
- Modify: `src/panel/actions.rs:152` (`CARD_KEYS`) and its assertion at ~line 511.
- Modify: `CONTEXT.md` — **Task card** (~lines 63–71) and **Action row** (~line 91).
- Modify: `README.md` — ~line 72 (keyboard section) and ~line 387 (Testing checklist). Leave line 41 and line 379 alone: they are about the peek and the hover, where a click on a card still does nothing.
- Modify: `tests/e2e-panel.sh` — after the "focused card shows its buttons" check (~line 303), before the "Escape from a tab other than All" block.

**Interfaces:**
- Consumes: the toggle from Task 2 (Enter on the focused body shows notes; a second hides them).
- Produces: `CARD_KEYS == "Enter: press the button or show notes · Ctrl+Del: delete the task"`.

- [ ] **Step 1: Write the failing test**

In `src/panel/actions.rs`'s test `the_hint_leaves_enter_and_ctrl_delete_to_the_footer`, change the last assertion to:

```rust
        assert_eq!(CARD_KEYS, "Enter: press the button or show notes · Ctrl+Del: delete the task");
```

Run: `cargo test --lib panel::actions`
Expected: FAIL, left `"Enter: press the button · Ctrl+Del: delete the task"`.

- [ ] **Step 2: Reword the footer**

```rust
/// The footer under the keyboard's list: the keys that act on whichever card
/// has the focus, the same on every one, so the card's own hint leaves them
/// out. Enter on a button presses it; on the card's body it shows the task's
/// notes, or hides them.
pub const CARD_KEYS: &str = "Enter: press the button or show notes · Ctrl+Del: delete the task";
```

Run: `cargo test --lib panel::actions`
Expected: PASS.

- [ ] **Step 3: Update CONTEXT.md**

In **Task card**, after "…counts its age from when the task was finished." add:

```
While the task panel has the keyboard, Enter or a click on a card's body shows
the task's notes under its description, dimmed, one per note, until a second
press or the keyboard is given back; a card with no notes ignores the press.
```

In **Action row**, replace the sentence `Enter or a click on the card's body does nothing.` with `Enter or a click on the card's body shows or hides the task's notes instead (see Task card).`

- [ ] **Step 4: Update README.md**

At ~line 72, replace `Enter on the card's description itself does nothing.` with:

```
Enter or a click on the card's description shows the task's notes under it, dimmed, one per note, and a second press hides them; a card with no notes ignores it, and giving the keyboard back hides them all.
```

At ~line 387 in the Testing checklist, replace `Enter on a card's description does nothing,` with:

```
Enter on a card's description shows its notes, dimmed under it, with the panel and its blur grown to fit, and a second Enter hides them,
```

- [ ] **Step 5: Add the e2e check**

In `tests/e2e-panel.sh`, after the `fi` that closes the "and the focused card shows its buttons" check and before `# Escape from a tab other than All`, insert:

```bash
    # Enter on the focused card's body shows its task's notes under the
    # description, and the panel grows to fit them; a second Enter hides
    # them. Every task gets a note first, so whichever card Down left the
    # focus on has one. Notes show only once pressed, so the re-render
    # the notes bring changes nothing on screen.
    for uuid in $(task "+$TAG" _uuids 2>/dev/null); do
        task rc.verbose=nothing rc.confirmation=no "$uuid" annotate -- "a note on this task" >/dev/null 2>&1
    done
    settle
    shot noted || { summary; exit 1; }
    if same keyboard_down noted; then
        ok "a task's notes stay hidden until its card is pressed"
    else
        read -r x0 x1 y0 y1 < <(measure noted keyboard_down)
        bad "adding notes changed columns ${x0}-${x1}, rows ${y0}-${y1} before any press"
    fi
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot notes_shown || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure notes_shown)
    if [ "$((y1 - y0))" -gt "$keyboard_h" ]; then
        ok "Enter on the card's body shows its notes, the panel grown to fit (${keyboard_h}px to $((y1 - y0))px)"
    else
        bad "after Enter the panel is $((y1 - y0))px tall against ${keyboard_h}px before —
      the focused card's notes should show and the panel grow to fit"
    fi
    settle
    shot notes_ticked || { summary; exit 1; }
    if same notes_shown notes_ticked; then
        ok "and the daemon's ticks keep them shown"
    else
        bad "the notes' frame changed over a tick with nothing changed"
    fi
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot notes_hidden || { summary; exit 1; }
    if same keyboard_down notes_hidden; then
        ok "and a second Enter hides them, the panel back as it was"
    else
        read -r x0 x1 y0 y1 < <(measure notes_hidden keyboard_down)
        bad "after a second Enter the screen differs in columns ${x0}-${x1}, rows ${y0}-${y1}"
    fi
```

Also add `Enter on a card's body (its notes),` to the header comment's list of keys pressed (~line 35–37), after `Ctrl+Enter,`.

- [ ] **Step 6: Run the tests and the e2e**

Run: `cargo test`
Expected: all pass.

Run: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`
Expected: every check `ok`, the four new ones among them. If the machine cannot run it (no Wayland session, no wtype, no Pillow), say so in the report; do not claim it ran.

- [ ] **Step 7: Commit**

```bash
git add src/panel/actions.rs CONTEXT.md README.md tests/e2e-panel.sh
git commit -m "feat(panel): say Enter on a card's body shows its notes

The keys footer, the glossary and the README said Enter or a click on
a card's body did nothing; it now shows the task's notes. The panel
e2e presses Enter on a noted card and checks the panel grows, a tick
keeps it so, and a second Enter puts it back.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
