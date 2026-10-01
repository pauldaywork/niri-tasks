# Action Row on the Focused Card Only — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While the task panel has the keyboard, only the focused task card shows its separator and action row; the other cards show just their wrapped description.

**Architecture:** Every card still gets its separator and row in `card_widget` (`src/panel/surface.rs`), but they start hidden. A new `Panel::show_row(card)` makes them visible on that one card, hides them on the rest, and, when anything changed, measures the cards again and refits the surface, input region and blur. The `focus-widget` notify handler calls it through `show_focused_row()` on every focus move, and `render()` calls it for the card it is about to focus. Focus moves still never re-render. The measure/size block comes out of `render()` as `Panel::fit()` so it can run from both places.

**Tech Stack:** Rust 2021, gtk4-rs 0.11 (GTK 4.22), gtk4-layer-shell 0.8, bash + Pillow + wtype for the e2e test in a nested niri.

**Spec:** Taskwarrior task `ad8eb3bd-dee6-4c36-9a53-429deeea173b`. Read it with `task rc.json.array=on ad8eb3bd-dee6-4c36-9a53-429deeea173b export`. Its description and notes are the spec.

## Global Constraints

- Every card still gets its row. Only the card that holds focus shows its separator and row, toggled with `set_visible` from the focus-widget handler and again at the end of `render()`. Focus moves don't re-render (that keeps the agents check and the focused button).
- Descriptions stay wrapped on every card. Only the action row follows focus.
- Hovering never shows a row, as now. A mouse click on a card's body still opens its menu.
- After each toggle, measure the card heights again so the blur region and the scroll fit the new layout. `follow_focus` then scrolls the focused card and its row into view.
- Update the `surface.rs` module doc, README (keyboard section), CONTEXT.md's **Action row** entry (and the **Task card** entry that mentions it), and the "each card grows its buttons" check in `tests/e2e-panel.sh`.
- Done when: after Mod+Alt+Ctrl+T only the focused card shows buttons; Up and Down move them with the fill; Left, Right, Tab and the letter keys still work; blur and scrolling match the changed heights; `cargo test` and `tests/e2e-panel.sh` pass.
- Out of scope: the peek, hover behaviour, and which buttons a card gets.
- House style: every item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commit messages are one plain-English imperative sentence, like `git log` shows, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line.

## Decisions made while planning (flag any you disagree with)

1. **Show before focusing.** `render()` shows the target card's row *before* `focus_card` moves focus onto a button in it (a refresh that keeps focus on, say, Refine). Grabbing focus on a hidden button is at best undefined in GTK, so the row is made visible first. The focus-widget handler then finds that card already showing and changes nothing.
2. **`show_row` refits only when a visibility changed.** Re-rendering removes the focused card, which fires the focus-widget handler mid-teardown with no focus; the guard keeps that from refitting a half-built column more than once, and makes repeat calls on the same card free.
3. **No focus, no row.** If focus is on nothing in the column (focus left it, or mid-render), every row is hidden.
4. **`fit()` does measure and size only.** `present()` stays in `render()` between `fit()` and the region/blur calls, as now: the surface may not exist before `present()`, and calling `present()` on every focus move is not wanted. `show_row` calls `fit()`, then `set_region` and `update_blur` itself, since the surface is up by then.
5. **No new unit test.** The change is GTK widget visibility and layout; the existing `#[cfg(test)]` tests in `surface.rs` are for pure geometry functions and none of those change. The behaviour is checked by `tests/e2e-panel.sh` in the nested niri, which is where the failing test goes first.
6. **The new e2e check** compares heights, not the frame diff (the darker fill moving changes both cards whatever the rows do):
   - After Down, the keyboard panel's own extent is the same height as before Down: the row moved, it was not added.
   - On the Waiting tab (one card, with its row) against All (three cards, one row): the difference is exactly two tucked one-line cards and their gaps, `three_h - one_h`, ±2px. With a row on every card it would be two rows taller.

---

### Task 1: Pull the measure-and-size block out of `render()`

A pure refactor: no behaviour changes. It gives Task 2 one call that refits the surface to the cards.

**Files:**
- Modify: `src/panel/surface.rs` — `render()` (the block from `// Measured only once they are in the window` to `self.window.set_default_size(SURFACE_WIDTH, height);`, about lines 594–628), and a new method `fit()` beside `update_tabs()`.

**Interfaces:**
- Produces: `fn fit(&self)` on `Panel` — measures every column child's height into `self.heights`, the tab bar into `self.slide.tabs_h`, the column's shown height into `self.slide.cards_h`, and sets the base and window size requests and default size. Does not call `present()`, `set_region` or `update_blur`. Assumes `self.tabs` visibility was already set for this render.

- [ ] **Step 1: Run the existing tests to get a baseline**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok.` with no failures.

- [ ] **Step 2: Add `fit()`**

Add this method in `impl Panel`, directly after `update_tabs()`:

```rust
    /// Measure the cards and the tabs, and size the surface to them: the
    /// heights the blur region and the input region work from. Run by every
    /// render, and again whenever a card's action row shows or hides, which
    /// changes its height without a render.
    fn fit(&self) {
        let keyboard = self.keyboard.get();
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
```

- [ ] **Step 3: Replace the block in `render()` with a call to it**

In `render()`, replace everything from the line `// Measured only once they are in the window: a label outside it has no` down to and including `self.window.set_default_size(SURFACE_WIDTH, height);` with:

```rust
        self.tabs.set_visible(keyboard);
        self.update_tabs();
        self.fit();
```

The lines just after stay as they are:

```rust
        // present(), not set_visible(true): see new().
        self.window.present();
        self.set_region(self.slide.x.get().min(self.slide.to.get()));
        self.update_blur(self.slide.x.get());
```

Note the order kept from before: the tab bar's visibility is set before anything is measured, since `fit()` measures it. The card heights used to be measured before `self.tabs.set_visible`; measuring them after changes nothing, as the cards sit in the scroller, not the tab bar.

- [ ] **Step 4: Build and run the tests**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test 2>&1 | tail -5`
Expected: no warnings or errors (an unused `cards.len()` capacity hint or variable would show here — remove it), and `test result: ok.`

- [ ] **Step 5: Run the e2e test**

Run: `bash tests/e2e-panel.sh`
Expected: every check `ok` (none `bad`), exactly as before the change. Keep off the nested niri's workspace while it runs (see the script's header).

- [ ] **Step 6: Commit**

```bash
git add src/panel/surface.rs
git commit -m "Pull measuring and sizing the panel out of render so it can run again without one

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Show the action row only on the focused card

**Files:**
- Modify: `tests/e2e-panel.sh` — the Down check (about lines 285–303) and the Waiting-tab check (about lines 330–345), and the header comment's mention of the keyboard's checks.
- Modify: `src/panel/surface.rs` — module doc (`## The keyboard`, about lines 26–39), the `focus-widget` handler in `new()` (about line 396), `render()`'s focus restore (end of `render()`), `card_widget` (its doc comment and the separator/row lines at its end), new methods `show_row` and `show_focused_row`, new free function `row_parts`.
- Modify: `README.md` — keyboard section (about lines 43–67) and the `Mod+Alt+Ctrl+T` table row (line 19).
- Modify: `CONTEXT.md` — **Task card** (about line 48) and **Action row** (about line 54).

**Interfaces:**
- Consumes: `fn fit(&self)` from Task 1; existing `set_region(&self, x: f64)`, `update_blur(&self, x: f64)`, `card_of(&self, &gtk4::Widget) -> Option<gtk4::Widget>`, `focus_card(&gtk4::Widget, Option<&str>)`.
- Produces: `fn show_row(&self, card: Option<&gtk4::Widget>)`, `fn show_focused_row(&self)`, free `fn row_parts(card: &gtk4::Widget) -> Vec<gtk4::Widget>`.

- [ ] **Step 1: Write the failing e2e checks**

In `tests/e2e-panel.sh`, replace the comment above `if command -v wtype >/dev/null; then` in the keyboard section:

```bash
# Down moves the darker fill from the first card to the second, and nothing
# else in the frame changes, so what differs between the two frames is exactly
# two cards: their columns, and with their action rows, well over twice the
# height of one card's one-line peek.
```

with:

```bash
# Down moves the darker fill from the first card to the second, and the action
# row with it: only the focused card has one. So what differs between the two
# frames is those two cards' columns, and the panel is as tall as it was.
```

Then replace the check that ends `the action rows are missing"` (the `if [ "$key_h" -ge $((one_h * 2 + 40)) ]; then … fi` block) with:

```bash
    read -r x0 x1 y0 y1 < <(measure keyboard_down)
    if [ "$((y1 - y0))" -eq "$keyboard_h" ]; then
        ok "and Down moves the buttons to the next card, the panel as tall as before (${keyboard_h}px)"
    else
        bad "after Down the panel is $((y1 - y0))px tall against ${keyboard_h}px before —
      the action row should move to the focused card, not be added or lost"
    fi
```

`key_h` is no longer used; delete the line `key_h=$((y1 - y0))` above it.

Then, in the Waiting-tab check (after `shot tab_waiting` and the `read -r x0 x1 y0 y1 < <(measure tab_waiting)` line), after its closing `fi`, add:

```bash
    # The Waiting tab's one card has its row, as the focused card on All did;
    # All's two other cards have none. So All is taller by exactly two
    # one-line cards and their gaps, as three tucked cards are than one.
    waiting_h=$((y1 - y0))
    extra=$((keyboard_h - waiting_h - (three_h - one_h)))
    if [ "$extra" -ge -2 ] && [ "$extra" -le 2 ]; then
        ok "only the focused card shows its buttons (All ${keyboard_h}px, Waiting ${waiting_h}px)"
    else
        bad "All's three cards are ${keyboard_h}px against ${waiting_h}px for the Waiting
      tab's one, ${extra}px off two one-line cards ($((three_h - one_h))px) — the unfocused
      cards still show their action rows"
    fi
```

Also in the header comment, change `with wtype, Down, the filter tabs' keys and Escape are pressed in the` so it reads `with wtype, Down (which moves the action row), the filter tabs' keys and Escape are pressed in the`, rewrapping the paragraph to the surrounding width.

- [ ] **Step 2: Run the e2e test and see the new checks fail**

Run: `bash tests/e2e-panel.sh`
Expected: `bad` on "only the focused card shows its buttons" (it is off by two rows' height), and the "Down moves the buttons" check passes already (three rows before and after Down) — that is fine; it guards against a row being added or lost by the change. Every other check `ok`.

- [ ] **Step 3: Add `row_parts`**

Add this free function in `src/panel/surface.rs`, directly before `fn slots(`:

```rust
/// A card's separator and action row: what shows only while it has focus.
/// Nothing for a card without them ("+N more", or any card off the keyboard).
fn row_parts(card: &gtk4::Widget) -> Vec<gtk4::Widget> {
    let mut parts = Vec::new();
    let mut child = card.first_child();
    while let Some(c) = child {
        child = c.next_sibling();
        if c.has_css_class("card-separator") || c.has_css_class("card-actions") {
            parts.push(c);
        }
    }
    parts
}
```

- [ ] **Step 4: Add `show_row` and `show_focused_row`**

Add these methods in `impl Panel`, directly after `follow_focus()`:

```rust
    /// Show this card's action row and hide every other card's, so the list
    /// stays one line a card bar the one being worked on. The cards change
    /// height without a render, so the surface, the input region and the blur
    /// are fitted to them again, but only when a row actually showed or hid:
    /// a render tearing the column down moves the focus too.
    fn show_row(&self, card: Option<&gtk4::Widget>) {
        let mut changed = false;
        let mut child = self.column.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            let show = Some(&c) == card;
            for part in row_parts(&c) {
                if part.is_visible() != show {
                    part.set_visible(show);
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

    /// Show the action row of the card that has focus, and no other.
    fn show_focused_row(&self) {
        let card = GtkWindowExt::focus(&self.window).and_then(|w| self.card_of(&w));
        self.show_row(card.as_ref());
    }
```

- [ ] **Step 5: Start every card's row hidden**

At the end of `card_widget`, replace:

```rust
        row.append(&hint);
        // A line between the description and the buttons.
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        separator.add_css_class("card-separator");
        widget.append(&separator);
        widget.append(&row);
        widget
```

with:

```rust
        row.append(&hint);
        // A line between the description and the buttons.
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        separator.add_css_class("card-separator");
        // Hidden until the card has focus: show_row() shows them.
        separator.set_visible(false);
        row.set_visible(false);
        widget.append(&separator);
        widget.append(&row);
        widget
```

And in `card_widget`'s doc comment, change `and, while the panel has the keyboard, its action row along the bottom.` to `and, while the panel has the keyboard, its action row along the bottom, hidden until the card has focus.` (rewrap to the comment's width).

- [ ] **Step 6: Call it from the focus-widget handler**

In `new()`, replace:

```rust
        // Moving off an armed Remove disarms it.
        {
            let weak = Rc::downgrade(&panel);
            panel.window.connect_notify_local(Some("focus-widget"), move |_, _| {
                if let Some(p) = weak.upgrade() {
                    p.disarm_unless_focused();
                    p.follow_focus();
                }
            });
        }
```

with:

```rust
        // Moving off an armed Remove disarms it, and the action row moves to
        // the card the focus is on, before follow_focus scrolls that card,
        // row and all, into view.
        {
            let weak = Rc::downgrade(&panel);
            panel.window.connect_notify_local(Some("focus-widget"), move |_, _| {
                if let Some(p) = weak.upgrade() {
                    p.disarm_unless_focused();
                    p.show_focused_row();
                    p.follow_focus();
                }
            });
        }
```

- [ ] **Step 7: Show the row in `render()` before focusing its card**

At the end of `render()`, replace:

```rust
            match child {
                Some(c) => focus_card(&c, focused.as_ref().map(|(_, slot)| slot.as_str())),
                None => {
                    if let Some(first) = self.column.first_child() {
                        focus_card(&first, None);
                    }
                }
            }
```

with:

```rust
            // Its row shows before the focus goes in, which may be onto one of
            // its buttons: a hidden button is no place for the focus.
            let target = child.map(|c| (c, focused.as_ref().map(|(_, slot)| slot.as_str())));
            let target = target.or_else(|| self.column.first_child().map(|c| (c, None)));
            if let Some((card, slot)) = target {
                self.show_row(Some(&card));
                focus_card(&card, slot);
            }
```

(The empty line "No tasks" can be the first child; it has no row parts, so `show_row` hides every row and `focus_card` finds no slots, as before.)

- [ ] **Step 8: Build and run the unit tests**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test 2>&1 | tail -5`
Expected: no warnings or errors, and `test result: ok.`

- [ ] **Step 9: Run the e2e test and see it pass**

Run: `bash tests/e2e-panel.sh`
Expected: every check `ok`, including "and Down moves the buttons to the next card, the panel as tall as before" and "only the focused card shows its buttons".

- [ ] **Step 10: Check by hand what the e2e test cannot**

On your own desktop with the daemon rebuilt (`cargo build --release` then restart it as the README's install section says) and a workspace with at least three tasks, one of them active:
- Mod+Alt+Ctrl+T: only the first card shows buttons.
- Down and Up: the buttons follow the darker fill; the panel does not jump.
- Right, Left and Tab: walk the focused card's buttons, the hint naming each.
- `e` on a card: opens the task box for that card. `Delete` arms Remove; Down disarms it.
- With enough tasks to scroll (or a tall description): moving to the bottom card scrolls it and its row wholly into view, and the blur behind the cards matches their outlines at every step.
- A mouse click on another card's body opens its menu. Hovering the peek shows no buttons.

- [ ] **Step 11: Update the module doc**

In the `//! ## The keyboard` section of `src/panel/surface.rs`, replace:

```rust
//! Mod+Alt+Ctrl+T hands the panel the keyboard in the middle of the screen,
//! and every card lays itself out again: its whole description, wrapped, above
//! a row of buttons for the menu's most-used actions. A card is a box holding
//! a body button and that row. Up and Down move between cards, Left, Right and
```

with:

```rust
//! Mod+Alt+Ctrl+T hands the panel the keyboard in the middle of the screen,
//! and every card lays itself out again: its whole description, wrapped, and,
//! on the focused card alone, a row of buttons for the menu's most-used
//! actions under it, so the list stays short enough to scan. A card is a box
//! holding a body button and that row; every card has its row, shown and
//! hidden as the focus moves rather than built again, since a render would
//! lose the focused button. Up and Down move between cards, Left, Right and
```

and rewrap the rest of that paragraph to the same width if needed. In the paragraph starting `Wrapped, the cards can stand taller than the screen`, change `Moving the focus scrolls the focused card wholly into view,` to `Moving the focus measures the cards again, its row having moved, and scrolls the focused card wholly into view,` and rewrap.

- [ ] **Step 12: Update README and CONTEXT.md**

`README.md`, line 19 (`Mod+Alt+Ctrl+T` table row): change `every card shows its whole description, with buttons to start working on it` to `every card shows its whole description, and the focused one buttons to start working on it`.

`README.md`, the keyboard section: replace

```markdown
screen, leaving no peek on the right edge while it is there. Every card opens up
to its whole description, wrapped, above a row of buttons:
```

with

```markdown
screen, leaving no peek on the right edge while it is there. Every card opens up
to its whole description, wrapped, and the focused card shows a row of buttons
under it:
```

and replace `Up and Down move a darker fill between cards; Left, Right and Tab move` with `Up and Down move a darker fill and the buttons between cards; Left, Right and Tab move`.

`CONTEXT.md`, **Task card**: change `description wrapped, above its action row.` to `description wrapped, above its action row while it has focus.`

`CONTEXT.md`, **Action row**: change `The buttons along a task card's bottom edge while the task panel has the keyboard —` to `The buttons along the focused task card's bottom edge while the task panel has the keyboard; the other cards hide theirs —` and rewrap the entry.

- [ ] **Step 13: Commit**

```bash
git add src/panel/surface.rs tests/e2e-panel.sh README.md CONTEXT.md
git commit -m "Show the action row only on the focused card while the panel has the keyboard

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
