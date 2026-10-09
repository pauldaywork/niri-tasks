# Card Id and Uuid Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In keyboard mode, pressing any task card's body shows a dimmed `#<id> · <uuid>` line above the task's notes (only the uuid when the id is 0), so either can be read off and copied into a `task` or `niritasks` command.

**Architecture:** Taskwarrior's export already carries `id`. Deserialize it into `task::Task`, copy it onto `panel::model::Card` in `cards()`, and give `Card` a pure `id_line()` that formats the line. `surface.rs`'s `notes_box` writes that line first, and the notes box is built for every task card in keyboard mode, not only cards with notes. `PanelState` stops checking for notes when toggling, pruning and hinting, so Space toggles every task card's body.

**Tech Stack:** Rust, GTK4 (gtk4-rs), serde, Taskwarrior 3.5 `task export`.

**Spec:** Taskwarrior task `e44118a7-122d-4667-8300-0e7e98fd3131`. Read it with `task rc.json.array=on e44118a7-122d-4667-8300-0e7e98fd3131 export`. Its description and annotations are the spec.

## Global Constraints

- The line reads `#48 · e44118a7-122d-4667-8300-0e7e98fd3131`: `#`, the id, space, `·` (U+00B7), space, the full uuid.
- When the id is 0 (Taskwarrior exports 0 for completed and deleted tasks, so every Finished card), the line is only the uuid.
- The line is the first line inside the existing `.card-notes` box, dimmed and styled like a note (class `card-note`), with the task's notes under it. Do not change the CSS.
- Every task card's body now opens in keyboard mode, a card with no notes too, which then shows only the id/uuid line. "+N more" is unchanged: its press still shows the rest.
- The id comes from the export, through a new `id` field on `Task` and on `Card`. It is never stored anywhere else.
- Out of scope: showing the id/uuid in the peek or the hover, a copy-to-clipboard button, making the label selectable.
- Commits follow Conventional Commits with scope `panel` where it fits, imperative and lowercase, subject ≤ 72 characters. End each message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- `cargo test` must pass at the end of each task. The baseline is 571 unit tests plus the integration tests, all passing.

---

### Task 1: Carry the task's id onto its card

**Files:**
- Modify: `src/task.rs:27-71` (`struct Task`), and its tests module (near `parses_end`, around line 840)
- Modify: `src/panel/model.rs:31-55` (`struct Card`), `:57-104` (`impl Card`), `:275-293` (the `Card` built in `cards()`), `:317-325` (the "+N more" `Card` in `cap()`), the test helper `task()` around line 336, the `card` closure in `a_cards_state_is_its_tasks` around line 769, and new tests next to `a_card_carries_its_tasks_notes` around line 447
- Modify: `src/panel/state.rs:928-930` (the test helper `card()`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `task::Task { pub id: u64, .. }`: Taskwarrior's working-set number, 0 when absent from the export.
  - `panel::model::Card { pub id: u64, .. }`: copied from `Task::id` in `cards()`, 0 on "+N more".
  - `panel::model::Card::id_line(&self) -> Option<String>`: `Some("#48 · <uuid>")`, `Some("<uuid>")` when `id == 0`, and `None` on "+N more" (no uuid).

- [ ] **Step 1: Write the failing tests**

In `src/task.rs`'s tests module, right after `parses_end`, add:

```rust
    /// `id` is read from the export, and absent means 0, as taskwarrior
    /// writes it on a completed or deleted task.
    #[test]
    fn parses_id() {
        let json = r#"[{"id":48,"uuid":"a","description":"d"},
                       {"id":0,"uuid":"b","description":"d","status":"completed"},
                       {"uuid":"c","description":"d"}]"#;
        let tasks: Vec<Task> = serde_json::from_str(json).unwrap();
        assert_eq!(tasks[0].id, 48);
        assert_eq!(tasks[1].id, 0);
        assert_eq!(tasks[2].id, 0);
    }
```

In `src/panel/model.rs`'s tests module, right after `a_card_carries_its_tasks_notes`, add:

```rust
    /// A card carries its task's id from the export; "+N more" has none.
    #[test]
    fn a_card_carries_its_tasks_id() {
        let mut numbered = task("n", 2, false);
        numbered.id = 48;
        let cards = cards(&todo(vec![numbered, task("plain", 1, false)]), &[]);
        assert_eq!(cards[0].id, 48);
        assert_eq!(cards[1].id, 0);
        assert_eq!(cap(&cards, 1)[1].id, 0, "+N more stands for no one task");
    }

    /// The line a pressed card shows above its notes: the id and the uuid,
    /// or the uuid alone when taskwarrior numbers the task 0, as it does a
    /// finished one. "+N more" has no line.
    #[test]
    fn the_id_line_is_the_id_and_the_uuid() {
        let mut numbered = task("e44118a7-122d-4667-8300-0e7e98fd3131", 2, false);
        numbered.id = 48;
        let got = cards(&todo(vec![numbered]), &[]);
        assert_eq!(got[0].id_line().as_deref(), Some("#48 · e44118a7-122d-4667-8300-0e7e98fd3131"));

        let got = cards(&done(vec![finished("f", 3)]), &[]);
        assert_eq!(got[0].id_line().as_deref(), Some("f"), "a finished task's id is 0");

        let capped = cap(&cards(&todo(vec![task("a", 1, false), task("b", 2, false)]), &[]), 1);
        assert_eq!(capped[1].id_line(), None);
    }
```

(`finished(uuid, day)` and `done(..)` already exist in this tests module. `finished` builds a `Task` through `task()`, so its id is 0.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib id 2>&1 | tail -20`
Expected: compile errors, `no field 'id' on type 'Task'` / `'Card'` and `no method named 'id_line'`.

- [ ] **Step 3: Add `id` to `Task`**

In `src/task.rs`, make `id` the first field of `struct Task`, above `uuid`:

```rust
pub struct Task {
    /// Taskwarrior's working-set number, what `task 48 …` takes: 0 on a
    /// completed or deleted task, which has none. Absent only on a
    /// hand-built task in a test, hence the default.
    #[serde(default)]
    pub id: u64,
    pub uuid: String,
```

- [ ] **Step 4: Add `id` to `Card` and `Card::id_line`**

In `src/panel/model.rs`, add the field to `struct Card` right after `uuid`:

```rust
    /// The task's id, read off the export: what the line above its notes
    /// shows. 0 on a finished task, which taskwarrior numbers 0, and on
    /// "+N more".
    pub id: u64,
```

In `impl Card`, after `shows_up_next`, add:

```rust
    /// The first line of the pressed card's notes: `#48 · <uuid>`, or the
    /// uuid alone when the id is 0, as on a finished task. Either can be
    /// read off for a `task` or `niritasks` command. None on "+N more".
    pub fn id_line(&self) -> Option<String> {
        let uuid = self.uuid.as_deref()?;
        Some(if self.id == 0 { uuid.to_string() } else { format!("#{} · {uuid}", self.id) })
    }
```

In `cards()`, in the `Card { .. }` literal, add after `uuid: Some(t.uuid.clone()),`:

```rust
            id: t.id,
```

In `cap()`, in the "+N more" `Card { .. }` literal, add after `uuid: None,`:

```rust
            id: 0,
```

- [ ] **Step 5: Fix the test-only `Task` and `Card` literals**

`src/panel/model.rs`, test helper `task()`: add `id: 0,` as the first field of the `Task { .. }` literal.

`src/panel/model.rs`, `a_cards_state_is_its_tasks`: the `card` closure's literal becomes

```rust
        let card = |status, planned, up_next| Card { status, text: "t".into(), uuid: Some("u".into()), id: 0, planned, up_next, since: String::new(), notes: Vec::new() };
```

`src/panel/state.rs`, test helper `card()`:

```rust
    fn card(uuid: &str, status: Status) -> Card {
        Card { status, text: uuid.into(), uuid: Some(uuid.into()), id: 0, planned: status == Status::Planned, up_next: false, since: String::new(), notes: Vec::new() }
    }
```

Then check nothing else builds one: `grep -rn "Card {\|Task {" src tests --include=*.rs | grep -v "struct\|impl\|fn \|\.\.card("`. Every hit must now have `id`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: every `test result: ok`, with 3 more tests than the baseline (574 in the lib).

- [ ] **Step 7: Commit**

```bash
git add src/task.rs src/panel/model.rs src/panel/state.rs
git commit -m "feat(panel): carry each task's id onto its card

Read Taskwarrior's id from the export and give the card a line of
its id and uuid, for the notes to show. A finished task's id is 0,
so its line is the uuid alone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Open every task card's body on the id/uuid line

**Files:**
- Modify: `src/panel/actions.rs:47-78` (`Action::hint` and its doc comment), its tests `hint` helper and the hint tests around lines 332-425
- Modify: `src/panel/state.rs:294-317` (`card_hint`), `:340-349` (the `notes.retain` in `set_cards`), `:570-583` (`toggle_notes`), the tests `the_body_hint_follows_the_notes_toggle` (~987), `a_card_with_no_notes_ignores_the_press` (~1412) and `a_card_that_loses_its_notes_does_not_reopen` (~1444)
- Modify: `src/panel/surface.rs:44-51` (module doc), `:248-251` (`CardWidgets::notes` doc), `:1012-1021` (`card_widget`), `:1367-1382` (`notes_box`)
- Modify: `CONTEXT.md:79-81` (Task card) and `:104-106` (Action row)
- Modify: `README.md:69-73`
- Modify: `tests/e2e-panel.sh:322-337` (comment and `ok` message only)

**Interfaces:**
- Consumes: `Card::id_line(&self) -> Option<String>` and `Card.id` from Task 1.
- Produces: `Action::hint(state: TaskState, row: &[Action], focused: Option<Action>, notes: bool) -> String`. `notes` is now whether the card's notes show (was `Option<bool>`, None on a card with no notes). Its only caller is `PanelState::card_hint`.

The surface has no unit tests for widgets (GTK needs a display). The state and the hint carry the testable rules, and Step 9 checks the drawing by hand.

- [ ] **Step 1: Write the failing state tests**

In `src/panel/state.rs`'s tests, replace `a_card_with_no_notes_ignores_the_press` with:

```rust
    /// A card with no notes still opens, on the line of its id and uuid
    /// alone, and a second press closes it.
    #[test]
    fn a_card_with_no_notes_opens_on_its_id_line() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.on_press("a", Slot::Body), vec![Effect::Notes]);
        assert!(state.shows_notes("a"));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(state.on_press("a", Slot::Body), vec![Effect::Notes]);
        assert!(!state.shows_notes("a"));
    }
```

Replace `a_card_that_loses_its_notes_does_not_reopen` with:

```rust
    /// A card that loses its notes stays open, on its id line. One that
    /// leaves the list stops being shown: when it comes back it does not
    /// reopen by itself.
    #[test]
    fn a_card_that_leaves_the_list_does_not_reopen() {
        let mut state = keyboard(vec![noted("a"), noted("b")]);
        state.on_press("a", Slot::Body);
        state.on_press("b", Slot::Body);
        state.set_cards(&[card("a", Status::Pending), noted("b")]);
        assert!(state.shows_notes("a"), "still open, on its id line");
        state.set_cards(&[noted("a")]);
        state.set_cards(&[noted("a"), noted("b")]);
        assert!(!state.shows_notes("b"));
    }
```

Replace `the_body_hint_follows_the_notes_toggle` with:

```rust
    /// The body's hint offers Space on every task card, notes or none, and
    /// follows the toggle.
    #[test]
    fn the_body_hint_follows_the_notes_toggle() {
        let mut state = keyboard(pending(&["a"]));
        assert!(state.card_hint("a").starts_with("Space: view notes"), "{}", state.card_hint("a"));
        state.on_press("a", Slot::Body);
        assert!(state.card_hint("a").starts_with("Space: hide notes"), "{}", state.card_hint("a"));
    }
```

- [ ] **Step 2: Rewrite the hint tests for a `bool`**

In `src/panel/actions.rs`'s tests:

The helper:

```rust
    fn hint(state: TaskState, focused: Option<Action>) -> String {
        Action::hint(state, &Action::row(state), focused, false)
    }
```

Replace `the_body_of_a_card_with_notes_hints_space` with:

```rust
    /// On a card's body, the hint names what Space does to its notes,
    /// before what Ctrl+Enter does; a button's hint leaves them out.
    #[test]
    fn the_body_hints_space() {
        let row = Action::row(on_list(false));
        assert_eq!(Action::hint(on_list(false), &row, None, false), "Space: view notes · Ctrl+Enter: Refine");
        assert_eq!(Action::hint(on_list(false), &row, None, true), "Space: hide notes · Ctrl+Enter: Refine");
        assert_eq!(Action::hint(active(true), &Action::row(active(true)), None, false), "Space: view notes");
        assert_eq!(Action::hint(on_list(false), &row, Some(Edit), true), "e: Edit · Ctrl+Enter: Refine");
    }
```

Replace `the_body_hints_what_ctrl_enter_does` and `the_body_of_a_task_ctrl_enter_leaves_alone_hints_nothing` with:

```rust
    /// Up and Down land on the body: Space for the notes, then what
    /// Ctrl+Enter does to this task.
    #[test]
    fn the_body_hints_what_ctrl_enter_does() {
        assert_eq!(hint(on_list(false), None), "Space: view notes · Ctrl+Enter: Refine");
        assert_eq!(hint(active(false), None), "Space: view notes · Ctrl+Enter: Refine");
        assert_eq!(hint(on_list(true), None), "Space: view notes · Ctrl+Enter: Start working");
    }

    #[test]
    fn the_body_of_a_task_ctrl_enter_leaves_alone_hints_only_space() {
        assert_eq!(hint(active(true), None), "Space: view notes");
        assert_eq!(hint(waiting(false), None), "Space: view notes");
        assert_eq!(hint(with_claude(waiting(true)), None), "Space: view notes");
    }
```

In `ctrl_enter_is_left_out_when_the_card_lacks_its_button`:

```rust
        assert_eq!(Action::hint(on_list(false), &[Edit, Remove], None, false), "Space: view notes");
        assert_eq!(Action::hint(on_list(true), &[Edit, Remove], Some(Edit), false), "e: Edit");
```

In `every_hint`, the doc comment's "(with no notes, and with notes hidden and shown)" becomes "(with its notes hidden and shown)", and the loop becomes `for notes in [false, true] {`.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib panel 2>&1 | tail -20`
Expected: compile errors in `actions.rs` tests, `expected 'Option<bool>', found 'bool'`. (The state tests would fail on `assert_eq!(.., vec![Effect::Notes])` once it compiles.)

- [ ] **Step 4: Change `Action::hint` to take `notes: bool`**

In `src/panel/actions.rs`, the doc comment and body of `hint`:

```rust
    /// What the focused card's hint reads beside its buttons: the keys that
    /// change from card to card and button to button. The focused button's
    /// key and name ("g: Go to session", "Del: Remove", or just "Speak" for
    /// a button only Enter presses), or, on the card's body, what Space does
    /// to its notes; then what Ctrl+Enter does to this task, in the words of
    /// the button it presses ("Ctrl+Enter: Refine"), when `row` has that
    /// button. `notes` is whether the card's notes show. Enter and
    /// Ctrl+Delete do the same on every card, so they are the footer's,
    /// [`CARD_KEYS`]. Pure, so every card's hint is tested without a window;
    /// `PanelState::card_hint` picks the card and `surface.rs` only sets the
    /// label.
    pub fn hint(state: TaskState, row: &[Action], focused: Option<Action>, notes: bool) -> String {
        let mut parts = Vec::new();
        match focused {
            Some(action) => {
                let name = action.label(state.up_next);
                parts.push(match action.facts().key {
                    Some(key) => format!("{key}: {name}"),
                    None => name.to_string(),
                });
            }
            // Space, not Enter: Enter is the footer's, and either presses
            // the body.
            None => parts.push(format!("Space: {} notes", if notes { "hide" } else { "view" })),
        }
        if let Some(step) = Self::advance(state).filter(|a| row.contains(a)) {
            parts.push(format!("Ctrl+Enter: {}", step.label(false)));
        }
        parts.join(" · ")
    }
```

- [ ] **Step 5: Drop the has-notes checks in `PanelState`**

In `src/panel/state.rs`, `card_hint`: replace

```rust
        let notes = (!card.card.notes.is_empty()).then(|| self.shows_notes(uuid));
        Action::hint(state, &card.actions, focused, notes)
```

with

```rust
        Action::hint(state, &card.actions, focused, self.shows_notes(uuid))
```

and in its doc comment, "or what Space does to its notes" stays as is.

In `set_cards`, replace the comment and `retain`:

```rust
        // Notes stay shown only for a card still there: otherwise a stale
        // uuid would reopen the card unasked when it came back, with the
        // toggle unable to reach it meanwhile.
        let all = &self.all;
        self.notes.retain(|uuid| all.iter().any(|c| c.uuid.as_deref() == Some(uuid.as_str())));
```

`toggle_notes` becomes:

```rust
    /// Show this task's notes on its card, under the line of its id and
    /// uuid, or hide them again. Only while the panel has the keyboard, and
    /// only on a card still there: anywhere else the body still does
    /// nothing. The keyboard, the focus and anything armed stay as they are.
    fn toggle_notes(&mut self, uuid: &str) -> Vec<Effect> {
        if !self.keyboard || !self.all.iter().any(|c| c.uuid.as_deref() == Some(uuid)) {
            return Vec::new();
        }
        if !self.notes.remove(uuid) {
            self.notes.insert(uuid.to_string());
        }
        vec![Effect::Notes]
    }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: every `test result: ok`. `every_hint_fits_beside_its_row` still passes: the longest body hint, `Space: hide notes · Ctrl+Enter: Start working`, was already in `every_hint` as `Some(true)`.

- [ ] **Step 7: Build the notes box for every task card, id line first**

In `src/panel/surface.rs`, `notes_box` becomes:

```rust
/// A task's notes, for under its card's description: first the line of its
/// id and uuid, to read off for a `task` or `niritasks` command, then one
/// label per note, all dimmed alike, wrapped, text only. The panel shows the
/// box once the card's body is pressed.
fn notes_box(id_line: &str, notes: &[String]) -> gtk4::Box {
    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    column.add_css_class("card-notes");
    for note in std::iter::once(id_line).chain(notes.iter().map(String::as_str)) {
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

In `card_widget`, replace the `let notes = …` block with:

```rust
        // The notes go inside the body, under the description, so a click
        // on them hides them as a click on the description does. Every task
        // card has the box, a card with no notes for its id line alone.
        let notes = card.uuid.as_deref().zip(card.id_line()).filter(|_| keyboard).map(|(uuid, id_line)| {
            let notes = notes_box(&id_line, &card.notes);
            notes.set_visible(self.state.borrow().shows_notes(uuid));
            content.append(&notes);
            notes
        });
```

The `CardWidgets::notes` doc comment becomes:

```rust
    /// The task's id line and notes under the description, shown while the
    /// state says. None on "+N more" and on every card off the keyboard.
```

In the module doc at the top of the file, change "Enter, Space or a click on its body shows the task's notes, or hides them." to "Enter, Space or a click on its body shows the task's id and uuid and its notes, or hides them."

- [ ] **Step 8: Build and run the tests**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: no warnings or errors, every `test result: ok`.

- [ ] **Step 9: Check it in the real panel**

Install the build with `./install.sh`, which also restarts `niri-tasks.service` on the new binary. Then, on a workspace with tasks:

1. Mod+Alt+Ctrl+T, Down onto a card with notes, Space: a dimmed `#<id> · <uuid>` line above its notes. Compare the id with `task <uuid8> _id`.
2. Down onto a card with no notes: its hint reads `Space: view notes …`. Space: the id/uuid line alone. Space again: it closes.
3. Press `6` for Finished (if shown), Space on a card: the uuid alone, no `#0`.
4. Down to "+N more" (if shown) and Space: it still shows the rest.

If you cannot drive the panel (no niri session), say so in the task report instead of claiming this step. `tests/e2e-panel.sh` runs the panel in a nested niri and can stand in for 1 if it passes.

- [ ] **Step 10: Update the docs**

`CONTEXT.md`, Task card, replace lines 79-81:

```
While the task panel has the keyboard, Enter or a click on a card's body shows,
dimmed under its description, the task's id and uuid on one line (`#48 ·
<uuid>`, the uuid alone on a finished task, which has no id) and then its
notes, one per note, until a second press or the keyboard is given back.
```

`CONTEXT.md`, Action row: "Enter or a click on the card's body shows or hides the task's notes instead (see Task card)." becomes "Enter or a click on the card's body shows or hides the task's id, uuid and notes instead (see Task card)." and "the focused button's, or on the body of a card with notes, `Space: view notes` or `Space: hide notes`" becomes "the focused button's, or on the card's body, `Space: view notes` or `Space: hide notes`".

`README.md`: "or on the description of a card with notes `Space: view notes`" becomes "or on a card's description `Space: view notes`", and "Enter or a click on the card's description shows the task's notes under it, dimmed, one per note, and a second press hides them; a card with no notes ignores it, and giving the keyboard back hides them all." becomes "Enter or a click on the card's description shows, dimmed under it, the task's id and uuid (`#48 · <uuid>`, the uuid alone on a finished task) and then its notes, one per note, and a second press hides them; giving the keyboard back hides them all."

`tests/e2e-panel.sh` lines 322-328, the comment: replace "so the re-render the notes bring changes only the focused card's hint, which now offers Space, and leaves the panel's size and place alone." with "so the re-render the notes bring leaves the panel's size and place alone." Line 337's message: `ok "adding notes leaves the panel as it was"`. The check itself is unchanged.

Run: `cargo test 2>&1 | grep -E "test result|FAILED"` (unchanged), and `bash -n tests/e2e-panel.sh`.

- [ ] **Step 11: Commit**

```bash
git add src/panel/actions.rs src/panel/state.rs src/panel/surface.rs CONTEXT.md README.md tests/e2e-panel.sh
git commit -m "feat(panel): show id and uuid above a card's notes

A press on any task card's body in keyboard mode now shows a dimmed
#<id> · <uuid> line above the task's notes, so either can be copied
into a task or niritasks command. A card with no notes opens on that
line alone, and Space is offered on every task card's body. A
finished card shows the uuid alone, as its id is 0.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
