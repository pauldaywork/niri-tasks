# Tab Key Numbers — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The task panel's tab bar shows the key that picks each tab: All (1), Active (2), Planned (3), To refine (4), Waiting (5), Finished (6), Ideas (7).

**Architecture:** The numbered labels need the whole 760px card width, so Clear all can't share the bar any more. Task 1 moves it to a right-aligned strip of its own under the bar, shown on Waiting alone, with its own blur rectangle. Task 2 adds `keys::tab_number(Tab)`, which gives each tab's fixed key next to the keymap. `surface.rs` gets a `tab_label(Tab)` that `tab_bar()` uses for the button faces. `Filter::label()` and `Tab::label()` keep the plain names.

**Tech Stack:** Rust 2021, GTK4 (gtk4-rs 0.11) and its CSS. No new crates.

**Spec:** Taskwarrior task `bff58397-2a6d-4137-8f7a-021b21fe7630`. Read it with `task rc.json.array=on bff58397-2a6d-4137-8f7a-021b21fe7630 export`. Its description and notes are the spec.

## Global Constraints

- Format: the name, a space, then the key in brackets: `"To refine (4)"`.
- A tab's number is fixed: `Filter::TABS[i]` is key `i+1`, Ideas is 7, hidden tabs keep theirs.
- `Filter::label()` and `Tab::label()` keep the plain names. `tab_bar()` in `src/panel/surface.rs` adds the number, so the label tests and the empty-tab text stay as they are.
- Done when: all seven tabs show their number, pressing each number picks the tab labelled with it, the bar still fits on one line at 760px with Clear all showing on Waiting, and `cargo test` passes.
- Out of scope: changing which key picks which tab.
- Decided with the user while planning: Clear all moves off the bar to a right-aligned strip of its own just under it, on Waiting only. Same key (Ctrl+Shift+Delete), same arming.
- House style: every new item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commits use Conventional Commits (`feat(panel): …`, imperative, lowercase, ≤72 chars) and end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Facts measured while planning (Pango, Iosevka Term Extended 10pt at 96dpi)

- The tab buttons have `6px 12px` padding (`ACTION_PADDING`) and a 1px line between them. The font is 8px a character, so the seven plain tabs take 534px and the numbered ones 758px. The bar is `CARD_WIDTH_PX` (760) wide inside its ring margins.
- Clear all (`🗑  Clear all`) is 125px with padding, or 189px once armed (`Confirm clear all`). On the bar with every tab showing, that comes to 883–947px, and trimming padding can't bring it under 760. That's why it moves.
- The trash can glyph comes from a fallback font. It makes Clear all's line 24px tall, against 17px for plain text, so the strip is about 36px, or 44px with the card gap over it.
- `gdk4` 0.11 has `gdk::Key::from_name`, which maps `"1"` to `Key::_1`. It has no `Key::from_unicode`.
- Baseline: `cargo test` passes, 439 unit tests plus the integration tests.

## File Structure

- `src/panel/surface.rs`: builds the tab bar (`tab_bar()`), the `Panel` fields, `Slide` heights, `fit()`, `update_tabs()` and `update_blur()`. Task 1 adds Clear all's strip and its blur. Task 2 adds `tab_label()` and uses it in `tab_bar()`.
- `src/panel/style.rs`: the CSS comment about where Clear all sits (Task 1).
- `src/panel/state.rs`: the doc comment on `shows_clear_all` (Task 1).
- `src/panel/keys.rs`: the keymap. Task 2 adds `tab_number()` next to it.
- `tests/e2e-panel.sh`: the Waiting-tab height check, which now has to allow for the strip (Task 1).
- `README.md`, `CONTEXT.md`, the `surface.rs` module doc: describe the strip (Task 1) and the numbers (Task 2).

---

### Task 1: Move Clear all to a strip of its own under the tab bar

**Files:**
- Modify: `src/panel/surface.rs` (module doc ~line 69; `Panel` fields ~183–193; `Slide` ~257–296; `Panel::new` ~326–346; `update_tabs` ~664–676; `fit` ~682–724; `update_blur` ~1062–1072; `tab_bar` ~1220–1264; tests ~1394+)
- Modify: `src/panel/style.rs:203-206`
- Modify: `src/panel/state.rs:170`
- Modify: `tests/e2e-panel.sh` (~lines 350–361)
- Modify: `README.md:62`, `README.md:105`, `CONTEXT.md:103-104`

**Interfaces:**
- Consumes: nothing new.
- Produces: `fn tab_bar() -> TabBar`, where `struct TabBar { head, bar, tab_buttons, ideas, clear_strip, clear }`. `Panel` gains `bar: gtk4::Box` and `clear_strip: gtk4::Box`. `Panel.tabs` is now the head box (the bar plus the strip). `fn clear_strip_rect(x: i32, bar_h: i32, size: (i32, i32)) -> blur::Rect`. Task 2 edits `tab_bar()`'s loop, which still appends each tab button to `bar`.

- [ ] **Step 1: Write the failing tests for the strip's blur rectangle**

Add to `mod tests` in `src/panel/surface.rs`, after `a_card_taller_than_the_page_shows_its_top`:

```rust
    /// Clear all's strip sits a card gap under the bar, its right edge on
    /// the cards' right edge, its own size.
    #[test]
    fn clear_alls_strip_sits_under_the_bar_at_the_cards_right_edge() {
        let x = 100;
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
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test clear_alls_strip 2>&1 | tail -20`
Expected: compile error, `cannot find function clear_strip_rect`.

- [ ] **Step 3: Add `clear_strip_rect`**

In `src/panel/surface.rs`, put it right before `fn tab_bar()` (with its doc comment, ~line 1220):

```rust
/// Where Clear all's strip is, for the blur behind it: a card gap under the
/// bar, its right edge on the cards', and cut at the surface's edge as the
/// cards are.
fn clear_strip_rect(x: i32, bar_h: i32, (w, h): (i32, i32)) -> blur::Rect {
    let left = x + CARD_WIDTH_PX - w;
    (left, SHADOW_PX + bar_h + GAP_PX, w.min(SURFACE_WIDTH - left), h)
}
```

Run: `cargo test clear_alls_strip 2>&1 | tail -5`
Expected: both PASS. There may be a dead-code warning until Step 6 uses the function.

- [ ] **Step 4: Rebuild `tab_bar()` as the bar plus Clear all's strip**

Replace the whole of `tab_bar()` and its doc comment (`src/panel/surface.rs` ~1220–1264) with:

```rust
/// What [`tab_bar`] builds, for the panel to keep.
struct TabBar {
    /// The bar and Clear all's strip, one over the other, with the ring
    /// and gap margins: what the panel shows, hides and measures.
    head: gtk4::Box,
    /// The tabs' strip alone, for the blur behind it.
    bar: gtk4::Box,
    tab_buttons: Vec<gtk4::Button>,
    ideas: gtk4::Button,
    /// Clear all's own strip, at the right under the bar, shown on the
    /// Waiting tab alone.
    clear_strip: gtk4::Box,
    clear: gtk4::Button,
}

/// The filter tabs and Ideas on a bar, and under it Clear all on a strip of
/// its own, over the scroller rather than in it, so they stay put while the
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
            let button = gtk4::Button::with_label(filter.label());
            // Out of the focus chain: the arrows and Tab stay between the
            // cards and their buttons, and a click does not take the focus.
            button.set_focusable(false);
            button.set_focus_on_click(false);
            bar.append(&button);
            button
        })
        .collect();
    // Ideas, last of the tabs: not a filter, and always shown.
    let ideas = gtk4::Button::with_label(Tab::Ideas.label());
    ideas.set_focusable(false);
    ideas.set_focus_on_click(false);
    bar.append(&ideas);
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
    TabBar { head, bar, tab_buttons, ideas, clear_strip, clear }
}
```

- [ ] **Step 5: Wire the new fields into `Panel`, `Slide`, `update_tabs` and `fit`**

`Panel` struct (~183–193): replace the `tabs` and `clear` field docs and add the two new fields:

```rust
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
    /// Clear all, on its strip: deletes every task the tab lists, on its
    /// second press.
    clear: gtk4::Button,
```

`Slide` struct (~267–272): extend `tabs_h`'s doc and add two fields after it:

```rust
    /// The filter tabs' height, Clear all's strip included when it shows,
    /// with the gap under them: what the cards sit below. 0 without the
    /// keyboard, which has no tabs.
    tabs_h: Cell<i32>,
    /// The tab bar's own height, without the strip or the gaps: the blur's.
    bar_h: Cell<i32>,
    /// Clear all's strip's width and height, (0, 0) while it is hidden: the
    /// blur's.
    clear: Cell<(i32, i32)>,
```

`Slide::tucked()` (~290): after `tabs_h: Cell::new(0),` add:

```rust
            bar_h: Cell::new(0),
            clear: Cell::new((0, 0)),
```

`Panel::new` (~326): replace `let (tabs, tab_buttons, ideas_button, clear) = tab_bar();` with:

```rust
        let TabBar { head: tabs, bar, tab_buttons, ideas: ideas_button, clear_strip, clear } = tab_bar();
```

and in the `Panel { … }` literal (~342–345) list `bar` after `tabs`, and `clear_strip` before `clear`:

```rust
            tabs,
            bar,
            tab_buttons,
            ideas_button,
            clear_strip,
            clear,
```

`update_tabs` (~664–676): rewrite the doc's last clause and show the strip, not the button:

```rust
    /// Which filter tabs show, which tab is picked, and whether Clear all's
    /// strip shows. Ideas is always shown: the bar itself hides without the
    /// keyboard.
```

and replace `self.clear.set_visible(state.shows_clear_all());` with:

```rust
        self.clear_strip.set_visible(state.shows_clear_all());
```

`fit` (~696–701): after the `tabs_h` block, add:

```rust
        // The bar and Clear all's strip apart, for the blur, which leaves
        // out the gap between them and the room left of the strip.
        let bar_h = if keyboard { self.bar.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX).1 } else { 0 };
        let clear = if keyboard && self.clear_strip.is_visible() {
            (
                self.clear_strip.measure(gtk4::Orientation::Horizontal, -1).1,
                self.clear_strip.measure(gtk4::Orientation::Vertical, -1).1,
            )
        } else {
            (0, 0)
        };
```

and next to `self.slide.tabs_h.set(tabs_h);` add:

```rust
        self.slide.bar_h.set(bar_h);
        self.slide.clear.set(clear);
```

- [ ] **Step 6: Blur the bar and the strip apart**

In `update_blur` (~1068–1072), replace:

```rust
        if tabs_h > 0 {
            // The tab bar, which does not scroll; not the card gap under it.
            rects.extend(blur::card_region((x, SHADOW_PX, width, tabs_h - GAP_PX), RADIUS_PX, on_screen));
        }
```

with:

```rust
        if tabs_h > 0 {
            // The tab bar, which does not scroll; not the card gap under it.
            let bar_h = self.slide.bar_h.get();
            rects.extend(blur::card_region((x, SHADOW_PX, width, bar_h), RADIUS_PX, on_screen));
            // Clear all's strip under it, when it shows; not the gap over it
            // or the room to its left.
            let clear = self.slide.clear.get();
            if clear.1 > 0 {
                rects.extend(blur::card_region(clear_strip_rect(x, bar_h, clear), RADIUS_PX, on_screen));
            }
        }
```

- [ ] **Step 7: Update the comments that put Clear all on the bar**

`src/panel/style.rs:203-205`, the comment over `.clear-all`:

```css
/* Clear all, on its strip under the Waiting tab's bar: Remove's red, and
   filled red once armed, as the armed Remove is. Its classes outrank the
   tabs' dimmed colour and the press reset's clear fill. */
```

(The selectors `.task-panel .filter-tabs .clear-all` still match, because the strip has the `filter-tabs` class.)

`src/panel/state.rs:170`:

```rust
    /// Clear all shows, on its strip under the tab bar, on the Waiting tab alone.
```

`src/panel/surface.rs` module doc, line 69, `//! On the Waiting tab alone, Clear all ends the tab bar. Like Remove, its` becomes:

```rust
//! On the Waiting tab alone, Clear all sits under the tab bar, at the right
//! on a strip of its own, the numbered tabs leaving no room on the bar. Like
//! Remove, its
```

Then reflow that paragraph so the lines stay under 80 columns.

- [ ] **Step 8: Build, run the unit tests and clippy**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: no warnings or errors, and every `test result: ok`, the two new tests among them.

- [ ] **Step 9: Let the e2e allow for the strip on Waiting**

The Waiting tab is now taller by the strip and the gap over it, about 44px (see the measured facts above). The check at `tests/e2e-panel.sh` ~350–361 compares All against Waiting, so give it that allowance. Replace:

```bash
    # The Waiting tab's one card has its row, as the focused card on All did;
    # All's two other cards have none. So All is taller by exactly two
    # one-line cards and their gaps, as three tucked cards are than one.
    waiting_h=$((y1 - y0))
    extra=$((keyboard_h - waiting_h - (three_h - one_h)))
```

with:

```bash
    # The Waiting tab's one card has its row, as the focused card on All did;
    # All's two other cards have none. So All is taller by exactly two
    # one-line cards and their gaps, as three tucked cards are than one, less
    # Clear all's strip under the Waiting tab's bar and the card gap over it:
    # its trash can's 24px line, 12px of padding and 8px of gap.
    clear_strip_h=44
    waiting_h=$((y1 - y0))
    extra=$((keyboard_h - waiting_h - (three_h - one_h) + clear_strip_h))
```

Run: `bash tests/e2e-panel.sh 2>&1 | tail -40` (about 30s. It needs niri and `wtype`, as described in the README's test table.)
Expected: every check `ok`, "only the focused card shows its buttons" among them. If that one check fails, read the `extra` it reports. It should be within a few pixels of 0. Then open the run's `tab_waiting` shot (the script prints where the shots are, or look under its sandbox's `shots/`) and check that Clear all sits on its own strip under the bar's right end. If it does, set `clear_strip_h` to 44 minus the reported `extra` and update the comment. If the strip looks wrong, fix the layout and don't touch the number. If the e2e can't run on this machine, say so in the task report. Don't skip it silently.

- [ ] **Step 10: Update README and CONTEXT**

`README.md:62`, the Clear all row's third cell: `On the Waiting tab's bar, not a card:` becomes `On the Waiting tab, on a strip under the tabs, not a card:`.

`README.md:105-106`: `The Waiting tab alone ends in **Clear all**,` becomes `On the Waiting tab alone, **Clear all** sits under the tabs on a strip of its own,`. Reflow the paragraph.

`CONTEXT.md:103-104`: `The Waiting tab alone ends in Clear all (Ctrl+Shift+Delete), which deletes every task under it` becomes `The Waiting tab alone has Clear all (Ctrl+Shift+Delete), on a strip under the tab bar, which deletes every task under it`. Reflow.

- [ ] **Step 11: Commit**

```bash
git add src/panel/surface.rs src/panel/style.rs src/panel/state.rs tests/e2e-panel.sh README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
feat(panel): move Clear all to a strip under the tab bar

The tabs are about to show their key numbers, which takes the bar's
whole 760px, so Clear all can no longer end it. On the Waiting tab it
now sits on a right-aligned strip of its own under the bar, with the
same key, arming and colours, and its own blur.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Show each tab's key number

**Files:**
- Modify: `src/panel/keys.rs` (imports line 8; new fn after `key_action` ~line 94; tests after `seven_picks_ideas` ~line 223)
- Modify: `src/panel/surface.rs` (`tab_bar()` from Task 1; new `tab_label` before it; module doc ~lines 59–62; tests)
- Modify: `README.md:20`, `README.md:98-99`

**Interfaces:**
- Consumes: `tab_bar()` from Task 1, whose loop appends each filter button to `bar`, then appends `ideas`. `Tab` and `Filter` from `src/panel/model.rs`.
- Produces: `pub fn keys::tab_number(tab: Tab) -> u32` (1–6 for `Filter::TABS`, 7 for Ideas) and `fn surface::tab_label(tab: Tab) -> String` (`"To refine (4)"`).

- [ ] **Step 1: Write the failing keymap test**

In `src/panel/keys.rs`, change the import at line 8 to `use super::model::{Filter, Tab};`. Then add this after `seven_picks_ideas` in `mod tests`:

```rust
    /// The number on a tab's label is the key that picks it: 1 to 6 the
    /// filter tabs left to right, 7 Ideas.
    #[test]
    fn each_tabs_number_is_the_key_that_picks_it() {
        let numbers: Vec<u32> = Filter::TABS.into_iter().map(|f| tab_number(Tab::Filter(f))).collect();
        assert_eq!(numbers, [1, 2, 3, 4, 5, 6]);
        assert_eq!(tab_number(Tab::Ideas), 7);
        for filter in Filter::TABS {
            let key = gdk::Key::from_name(tab_number(Tab::Filter(filter)).to_string()).unwrap();
            assert_eq!(key_action(key, false, false), KeyAction::Filter(filter), "{filter:?}");
        }
        let key = gdk::Key::from_name(tab_number(Tab::Ideas).to_string()).unwrap();
        assert_eq!(key_action(key, false, false), KeyAction::Ideas);
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test each_tabs_number 2>&1 | tail -10`
Expected: compile error, `cannot find function tab_number`.

- [ ] **Step 3: Add `tab_number` next to the keymap**

In `src/panel/keys.rs`, right after `key_action` (before `ideas_key_action`):

```rust
/// The number key that picks this tab, which its label shows: 1 to 6 the
/// filter tabs in `Filter::TABS` order, 7 Ideas after them, as
/// [`key_action`] maps them. Fixed, so a hidden tab keeps its number and
/// the shown ones do not renumber.
pub fn tab_number(tab: Tab) -> u32 {
    match tab {
        Tab::Filter(filter) => {
            let index = Filter::TABS.iter().position(|f| *f == filter).expect("every filter is a tab");
            index as u32 + 1
        }
        Tab::Ideas => Filter::TABS.len() as u32 + 1,
    }
}
```

Also update the doc on `KeyAction::Filter` (line 26), `/// 1 to 6: show this filter tab's cards, when it is shown.`, to:

```rust
    /// 1 to 6: show this filter tab's cards, when it is shown. Its label
    /// says which ([`tab_number`]).
```

Run: `cargo test each_tabs_number 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 4: Write the failing label tests**

In `src/panel/surface.rs` `mod tests`, after the Task 1 tests:

```rust
    /// A tab wears its name, a space, and the key that picks it in brackets.
    #[test]
    fn a_tab_wears_its_key_in_brackets() {
        assert_eq!(tab_label(Tab::Filter(Filter::All)), "All (1)");
        assert_eq!(tab_label(Tab::Filter(Filter::ToRefine)), "To refine (4)");
        assert_eq!(tab_label(Tab::Filter(Filter::Finished)), "Finished (6)");
        assert_eq!(tab_label(Tab::Ideas), "Ideas (7)");
    }

    /// Every tab shown at once, numbered, still fits the bar's one line:
    /// Iosevka Term Extended at 10pt is 8px a character, and each tab adds
    /// 24px of padding and, after the first, a 1px line. 758px of 760.
    #[test]
    fn the_numbered_tabs_fit_the_bar() {
        let tabs: Vec<Tab> = Filter::TABS.into_iter().map(Tab::Filter).chain([Tab::Ideas]).collect();
        let width: usize =
            tabs.iter().map(|t| tab_label(*t).chars().count() * 8 + 24).sum::<usize>() + tabs.len() - 1;
        assert!(width <= CARD_WIDTH_PX as usize, "{width}px");
    }
```

`Filter`, `Tab` and `CARD_WIDTH_PX` reach the test module through `use super::*;`, because `surface.rs` imports them at lines 123 and 125.

- [ ] **Step 5: Run them to see them fail**

Run: `cargo test tab_wears_its_key 2>&1 | tail -10`
Expected: compile error, `cannot find function tab_label`.

- [ ] **Step 6: Add `tab_label` and use it in `tab_bar()`**

In `src/panel/surface.rs`, before the `TabBar` struct from Task 1:

```rust
/// A tab button's face: its name, then the key that picks it in brackets,
/// so the keys can be learnt off the bar. The plain name stays
/// `Tab::label`'s, for the tests and the empty tab's line.
fn tab_label(tab: Tab) -> String {
    format!("{} ({})", tab.label(), keys::tab_number(tab))
}
```

`surface.rs` already has `use super::keys;` (line 121).

In `tab_bar()`, change the filter buttons' line:

```rust
            let button = gtk4::Button::with_label(&tab_label(Tab::Filter(*filter)));
```

and the Ideas button's line:

```rust
    let ideas = gtk4::Button::with_label(&tab_label(Tab::Ideas));
```

Run: `cargo test tab_wears_its_key 2>&1 | tail -5; cargo test numbered_tabs_fit 2>&1 | tail -5`
Expected: both PASS.

- [ ] **Step 7: Full test run, with no warnings**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: no warnings, and every `test result: ok`. The `model.rs` label tests (`Filter::label`, `Tab::Ideas.label() == "Ideas"`) pass unchanged.

- [ ] **Step 8: See it on screen**

Run: `bash tests/e2e-panel.sh 2>&1 | tail -40`
Expected: every check `ok`. On the `tab_waiting` shot, the bar reads `All (1) │ Waiting (5) │ Ideas (7)` (only the tabs with tasks show) on one line, with Clear all on its strip under the bar's right end. If the e2e can't run here, say so in the task report.

- [ ] **Step 9: Update the docs**

`README.md:20`: in the `Mod+Alt+Ctrl+T` row, `Tabs above the cards, picked with 1–6 or [ and ], narrow them to` becomes `Tabs above the cards, each labelled with the key that picks it, as To refine (4), and picked with 1–6 or [ and ], narrow them to`.

`README.md:98-99`: `` All apart. `1` to `6` pick one, each always the same tab and doing nothing while it is hidden; `` becomes `` All apart. `1` to `6` pick one, each always the same tab and doing nothing while it is hidden, and each tab's label ends in its key, as **To refine (4)**; ``. Reflow the paragraph.

`src/panel/surface.rs` module doc, lines 59–62: `//! a task under it, All apart. 1 to 6 pick one, always the same one, and do` becomes `//! a task under it, All apart. 1 to 6 pick one, always the same one, and each wears its key in brackets after its name, as To refine (4); they do`. Reflow to under 80 columns. The Ideas paragraph (~line 82) already says "7 picks it", so leave it.

- [ ] **Step 10: Commit**

```bash
git add src/panel/keys.rs src/panel/surface.rs README.md
git commit -m "$(cat <<'EOF'
feat(panel): show each tab's key number, as All (1)

Each tab button now ends in the number key that picks it, so the
keys can be learnt off the bar. keys::tab_number holds the numbers
next to the keymap, and a test ties each one to the key that picks
its tab. Filter::label and Tab::label keep the plain names.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
