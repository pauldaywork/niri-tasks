# Panel Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the task panel's geometry (surface size, input rectangle, blur rectangles) out of `surface.rs` into a pure, tested `panel/layout.rs`, and replace five hand-copied fit/region/blur tails with one call.

**Architecture:** `Measured` is what GTK reports; `Layout::new(measured)` answers `surface_height`, `visible_height`, `input_rect(x, centred)` and `blur_rects(x, scrolled)`. The surface fills `Measured` in `fit`, keeps the `Layout`, and applies its answers; `Slide` keeps only the animation. `blur.rs` stays the Wayland adapter. No behaviour change: the e2e panel measurements must be identical.

**Tech Stack:** Rust (gtk4, cairo), bash e2e in a nested niri.

**Spec:** `docs/superpowers/specs/2026-10-08-panel-layout-design.md` (same commit as this plan).

## Global Constraints

- Conventional Commits with an attribution trailer (`Co-Authored-By: <your model> <noreply@anthropic.com>`); subject ≤ 72, body wrapped at 72 saying what and why.
- `cargo test` passes and `cargo build` has no warnings after every task.
- Work in the `panel-layout` worktree (`/home/paul/.worktrees/niri-tasks/panel-layout`); `finish-worktree` lands it.
- **Never run `install.sh`, `cargo install` or `systemctl --user restart niri-tasks`.** Run e2e only as `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`. Two notes checks in that suite may fail with a stale baseline on `main` (a separate task fixes them); every geometry check (peek column, keyboard card span, heights) must pass, and any failure naming a column or row other than those two is this branch's to fix.
- No behaviour change: every number the e2e suite measures today comes out the same.
- Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Docs on every pub item; comments are full sentences that say why. Line numbers are anchors from `main` at 012cca5; match on quoted text.

---

### Task 1: `panel/layout.rs`, with the pure helpers and their tests

**Files:**
- Create: `src/panel/layout.rs`
- Modify: `src/panel/mod.rs` (add `pub mod layout;`), `src/panel/surface.rs` (delete the eight constants at :159-188 and import them from `layout`; delete `centre_margin` :1723, `region_width` :1729, `shown_height` :1738, `scroll_to_show` :1745, `clear_strip_rect` :1518 and import the two that stay in use; move their tests :1760-1872 out of `surface.rs`'s test module)

**Interfaces:**
- Consumes: `blur::Rect`, `blur::card_region`, `blur::clip_rows`; `style::{CARD_WIDTH_PX, GAP_PX, RADIUS_PX}`.
- Produces: `pub(crate)` constants `PEEK_PX`, `SHADOW_PX`, `RING_PX`, `EDGE_GAP_PX`, `SURFACE_WIDTH`, `EXPANDED_X`, `TUCKED_X`, `CENTRED_X`; `pub struct Measured { card_heights, column_h, tabs_h, bar_h, clear, footer_h, screen_h }`; `pub struct Layout` with `new`, `cards_h`, `visible_height`, `surface_height`, `input_rect(x: i32, centred: bool) -> blur::Rect`, `blur_rects(x: i32, scrolled: i32) -> Vec<blur::Rect>`; `pub(crate) fn centre_margin`, `pub(crate) fn scroll_to_show`. The surface does not use `Layout` until Task 2; `surface.rs` keeps compiling by importing the constants and the two helpers it still calls.

- [ ] **Step 1: Write the module with its tests**

Create `src/panel/layout.rs`. Move the eight constants with their doc comments from `surface.rs:159-188` (keep the comments; make each `pub(crate) const`). Then:

```rust
//! Where the task panel's cards are on screen, as arithmetic: the surface's
//! size, the input rectangle the pointer is accepted in, and the rectangles
//! niri blurs behind the cards. `surface.rs` measures the widgets into a
//! [`Measured`] and applies what a [`Layout`] answers; nothing here touches
//! GTK or Wayland, so the one calculation that puts blur exactly under the
//! cards is tested without a compositor.

use super::blur::{self, Rect};
use super::style::{CARD_WIDTH_PX, GAP_PX, RADIUS_PX};

// … the eight constants …

/// What the surface measured, once the widgets are in the window: the
/// heights the geometry works from. Filled by `surface::fit`, read by
/// [`Layout`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Measured {
    /// Each child of the card column, top to bottom: the cards, and the
    /// lines that stand in for them.
    pub card_heights: Vec<i32>,
    /// The column's own height less its rings: the cards and the gaps
    /// between them, before any of it is cut to the screen.
    pub column_h: i32,
    /// The tab bar, Clear all's strip when it shows, and the gap under them.
    /// 0 without the keyboard, which has no tabs.
    pub tabs_h: i32,
    /// The tab bar alone, without the strip or the gaps: the blur's.
    pub bar_h: i32,
    /// Clear all's strip's width and height, (0, 0) while it is hidden: the
    /// blur's.
    pub clear: (i32, i32),
    /// The footer's height, with the gap over it: what sits under the cards.
    /// 0 without the keyboard, and on Ideas.
    pub footer_h: i32,
    /// The monitor's height, which the cards are cut to.
    pub screen_h: i32,
}

/// Where everything is, worked out from what was measured: the surface's
/// size, and the rectangles the input region and the blur need.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    measured: Measured,
    /// The cards' height on screen: all of the column, or as much as fits
    /// inside the screen's margins once the tabs and the footer have theirs.
    cards_h: i32,
}

impl Layout {
    pub fn new(measured: Measured) -> Layout {
        let cards_h = shown_height(measured.column_h, measured.tabs_h + measured.footer_h, measured.screen_h);
        Layout { measured, cards_h }
    }

    /// The cards' height on screen.
    pub fn cards_h(&self) -> i32 {
        self.cards_h
    }

    /// From the top of the tabs, when there are any, to the bottom of the
    /// footer under the cards on screen: what the input region covers.
    pub fn visible_height(&self) -> i32 {
        self.measured.tabs_h + self.cards_h + self.measured.footer_h
    }

    /// The surface's height: what shows, and the shadow above and below it.
    pub fn surface_height(&self) -> i32 {
        self.visible_height() + 2 * SHADOW_PX
    }

    /// Where the pointer is accepted with the cards at `x`: to the surface's
    /// edge when tucked away or sliding, so the hover stays alive there, or
    /// just the cards when centred, so the strip beside them in the middle
    /// of the screen does not swallow clicks.
    pub fn input_rect(&self, x: i32, centred: bool) -> Rect {
        (x, SHADOW_PX, region_width(x, centred), self.visible_height())
    }

    /// Blur behind the cards at `x`, corners and all, with the column
    /// scrolled by `scrolled`: the tab bar, Clear all's strip under it when
    /// it shows, each card cut to the band on screen, and the footer. Not
    /// the gaps between them, and not the room left of the strip.
    pub fn blur_rects(&self, x: i32, scrolled: i32) -> Vec<Rect> {
        let m = &self.measured;
        let width = CARD_WIDTH_PX.min(SURFACE_WIDTH - x);
        let on_screen = x + CARD_WIDTH_PX <= SURFACE_WIDTH;
        let mut rects = Vec::new();
        if m.tabs_h > 0 {
            rects.extend(blur::card_region((x, SHADOW_PX, width, m.bar_h), RADIUS_PX, on_screen));
            if m.clear.1 > 0 {
                let strip = clear_strip_rect(x, m.bar_h, m.clear);
                if strip.2 > 0 {
                    rects.extend(blur::card_region(strip, RADIUS_PX, on_screen));
                }
            }
        }
        let top = SHADOW_PX + m.tabs_h;
        let mut y = top - scrolled;
        let mut cards = Vec::new();
        for &h in &m.card_heights {
            cards.extend(blur::card_region((x, y, width, h), RADIUS_PX, on_screen));
            y += h + GAP_PX;
        }
        rects.extend(blur::clip_rows(&cards, top, top + self.cards_h));
        if m.footer_h > 0 {
            let y = top + self.cards_h + GAP_PX;
            rects.extend(blur::card_region((x, y, width, m.footer_h - GAP_PX), RADIUS_PX, on_screen));
        }
        rects
    }
}
```

Carry the five pure helpers over verbatim with their docs (`centre_margin` and `scroll_to_show` as `pub(crate)`, the others private), and move their tests (`the_region_runs_to_the_edge_unless_centred` through `clear_alls_strip_has_no_width_at_the_peeks_x`, sixteen tests at `surface.rs:1760-1872`) into `layout.rs`'s test module unchanged. Then add:

```rust
    fn measured() -> Measured {
        Measured {
            card_heights: vec![60, 80, 40],
            column_h: 60 + 80 + 40 + 2 * GAP_PX,
            tabs_h: 48,
            bar_h: 32,
            clear: (0, 0),
            footer_h: 32,
            screen_h: 1000,
        }
    }

    /// The box the strips of a region cover: left, top, right, bottom.
    fn bounds(rects: &[Rect]) -> (i32, i32, i32, i32) {
        let left = rects.iter().map(|r| r.0).min().unwrap();
        let top = rects.iter().map(|r| r.1).min().unwrap();
        let right = rects.iter().map(|r| r.0 + r.2).max().unwrap();
        let bottom = rects.iter().map(|r| r.1 + r.3).max().unwrap();
        (left, top, right, bottom)
    }

    #[test]
    fn the_heights_add_up() {
        let layout = Layout::new(measured());
        assert_eq!(layout.cards_h(), 60 + 80 + 40 + 2 * GAP_PX, "a short column shows whole");
        assert_eq!(layout.visible_height(), 48 + layout.cards_h() + 32);
        assert_eq!(layout.surface_height(), layout.visible_height() + 2 * SHADOW_PX);
    }

    #[test]
    fn the_input_rect_runs_to_the_edge_tucked_and_stops_at_the_cards_centred() {
        let layout = Layout::new(measured());
        let x = TUCKED_X as i32;
        assert_eq!(layout.input_rect(x, false), (x, SHADOW_PX, SURFACE_WIDTH - x, layout.visible_height()));
        let x = CENTRED_X as i32;
        assert_eq!(layout.input_rect(x, true), (x, SHADOW_PX, CARD_WIDTH_PX, layout.visible_height()));
    }

    #[test]
    fn blur_covers_the_bar_the_cards_and_the_footer_and_not_the_gaps() {
        let layout = Layout::new(measured());
        let x = EXPANDED_X as i32;
        let rects = layout.blur_rects(x, 0);
        let (left, top, right, bottom) = bounds(&rects);
        assert_eq!((left, right), (x, x + CARD_WIDTH_PX));
        assert_eq!(top, SHADOW_PX, "the bar is the first thing blurred");
        let cards_top = SHADOW_PX + 48;
        assert_eq!(bottom, cards_top + layout.cards_h() + GAP_PX + (32 - GAP_PX), "down to the footer's bottom");
        // The gap under the bar is not blurred: no strip starts between the bar's bottom and the first card.
        assert!(rects.iter().all(|r| !(r.1 > SHADOW_PX + 32 && r.1 < cards_top)), "{rects:?}");
    }

    #[test]
    fn scrolling_moves_the_cards_up_and_cuts_them_to_the_band() {
        let mut m = measured();
        m.screen_h = 48 + 32 + 100 + 2 * (SHADOW_PX + EDGE_GAP_PX); // room for 100px of cards
        let layout = Layout::new(m);
        assert_eq!(layout.cards_h(), 100);
        let top = SHADOW_PX + 48;
        let x = EXPANDED_X as i32;
        let unscrolled: Vec<Rect> = layout.blur_rects(x, 0).into_iter().filter(|r| r.1 >= top && r.1 < top + 100).collect();
        let scrolled: Vec<Rect> = layout.blur_rects(x, 30).into_iter().filter(|r| r.1 >= top && r.1 < top + 100).collect();
        assert!(bounds(&unscrolled).1 == top);
        assert!(scrolled.iter().all(|r| r.1 >= top && r.1 + r.3 <= top + 100), "cards are cut to the band: {scrolled:?}");
        assert_ne!(unscrolled, scrolled, "scrolling by 30 moves the cards' strips");
    }

    #[test]
    fn tucked_cards_are_narrowed_to_the_peek_and_lose_their_right_corners() {
        let layout = Layout::new(measured());
        let x = TUCKED_X as i32;
        let rects = layout.blur_rects(x, 0);
        let (left, _, right, _) = bounds(&rects);
        assert_eq!((left, right), (x, SURFACE_WIDTH), "only the peek is on the surface");
        // Unrounded on the right: some strip reaches the surface's edge at every row of the bar.
        assert!(rects.iter().any(|r| r.0 + r.2 == SURFACE_WIDTH));
    }

    #[test]
    fn without_tabs_or_footer_only_the_cards_are_blurred() {
        let mut m = measured();
        m.tabs_h = 0;
        m.bar_h = 0;
        m.footer_h = 0;
        let layout = Layout::new(m);
        let (_, top, _, bottom) = bounds(&layout.blur_rects(EXPANDED_X as i32, 0));
        assert_eq!(top, SHADOW_PX);
        assert_eq!(bottom, SHADOW_PX + layout.cards_h());
    }

    #[test]
    fn clear_alls_strip_is_blurred_under_the_bar_when_it_shows() {
        let mut m = measured();
        m.clear = (120, 28);
        let layout = Layout::new(m);
        let x = EXPANDED_X as i32;
        let strip = clear_strip_rect(x, 32, (120, 28));
        let rects = layout.blur_rects(x, 0);
        assert!(rects.iter().any(|r| r.1 >= strip.1 && r.1 < strip.1 + strip.3 && r.0 >= strip.0), "{rects:?}");
    }
```

If a bounding-box expectation is off by a corner strip (card_region rounds corners by trimming rows), adjust the test to the real geometry after reading `blur::card_region` and say so in the report; do not change `blur_rects` to fit a test.

In `src/panel/surface.rs`: delete the eight constants and the five helpers with their sixteen tests; add `use super::layout::{self, Layout, Measured, PEEK_PX, SHADOW_PX, RING_PX, EDGE_GAP_PX, SURFACE_WIDTH, EXPANDED_X, TUCKED_X, CENTRED_X, centre_margin, scroll_to_show};` trimmed to what the file uses (`Layout`/`Measured` are for Task 2; leave them out until then so there is no unused-import warning); replace calls to `region_width`, `shown_height`, `clear_strip_rect` with `layout::region_width`… wait: those three are private to layout.rs in the design. For this task only, since `set_region`, `fit` and `update_blur` still call them, make them `pub(crate)` with a `// Task 2 folds these callers into Layout.` note, and Task 2 makes them private again. Add `pub mod layout;` to `src/panel/mod.rs`.

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test --lib panel:: 2>&1 | tail -3`
Expected: no warnings; pass (the moved 16 tests plus 7 new ones in `layout::tests`; `surface::tests` down to the five label tests).

- [ ] **Step 2: Commit**

```bash
git add src/panel/layout.rs src/panel/mod.rs src/panel/surface.rs
git commit -m "feat(panel): add a layout module for the panel's geometry

The surface's size, the input rectangle and the blur rectangles were
worked out in three places in surface.rs, and the blur arithmetic had no
test. Layout answers all three from one Measured, and the thirteen pure
helpers move in with their tests; the surface still does the measuring
and the applying, and switches to Layout in the next commit.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

### Task 2: The surface measures into `Measured` and applies the `Layout`

**Files:**
- Modify: `src/panel/surface.rs`: `Slide` :298-346 (drop `cards_h`, `tabs_h`, `bar_h`, `clear`, `footer_h` and their init), field `heights: RefCell<Vec<i32>>` :253 → `layout: RefCell<Layout>`; `fit` :807-866; `set_region` :1316-1325; `update_blur` :1328-1375; the hover check at :419; the tails at :885-887, :938-943, :1221-1222, :1270-1272, :1294-1296; module doc.
- Modify: `src/panel/layout.rs` (make `region_width`, `shown_height`, `clear_strip_rect` private again).

**Interfaces:**
- Consumes: Task 1's `Layout`, `Measured`.
- Produces: `fn refit(&self)` on `Panel`; `Slide` without heights.

- [ ] **Step 1: Replace the geometry**

- `Slide`: delete the five height fields, their docs and their `Cell::new(..)` lines in `tucked()`.
- Replace the field `heights: RefCell<Vec<i32>>` (and its init) with `/// Where the cards are, from the last fit. layout: RefCell<Layout>,` initialised `RefCell::new(Layout::default())`.
- `fit`: keep every GTK measurement as it is, then end with

```rust
        let (_, with_ring, _, _) =
            self.column.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX + 2 * RING_PX);
        let layout = Layout::new(Measured {
            card_heights: heights,
            column_h: with_ring - 2 * RING_PX,
            tabs_h,
            bar_h,
            clear,
            footer_h,
            screen_h: self.monitor.geometry().height(),
        });
        let height = layout.surface_height();
        *self.layout.borrow_mut() = layout;
        // Both calls: the size request lets the surface grow, the default size
        // lets it shrink back when the list gets shorter.
        self.base.set_size_request(SURFACE_WIDTH, height);
        self.window.set_size_request(SURFACE_WIDTH, height);
        self.window.set_default_size(SURFACE_WIDTH, height);
```
  (`heights` is the Vec the loop at the top of `fit` builds; it no longer goes into `self.heights`.)
- `set_region`:
```rust
    fn set_region(&self, x: f64) {
        let Some(surface) = self.window.surface() else { return };
        let (x, y, w, h) = self.layout.borrow().input_rect(x.round() as i32, self.state.borrow().keyboard());
        let rect = cairo::RectangleInt::new(x, y, w, h);
        surface.set_input_region(Some(&cairo::Region::create_rectangle(&rect)));
    }
```
  keeping its doc comment.
- `update_blur`:
```rust
    fn update_blur(&self, x: f64) {
        let mut blur = self.blur.borrow_mut();
        let Some(blur) = blur.as_mut() else { return };
        let scrolled = self.scroller.vadjustment().value().round() as i32;
        blur.set(&self.layout.borrow().blur_rects(x.round() as i32, scrolled));
    }
```
  keeping its doc comment.
- Add, near `fit`:
```rust
    /// Fit the surface to the cards again and put the input region and the
    /// blur where they now are: after anything that changes a card's height
    /// without a render (an action row showing, notes unfolding, an age
    /// gaining a line) and after every render.
    fn refit(&self) {
        self.fit();
        self.set_region(self.slide.x.get().min(self.slide.to.get()));
        self.update_blur(self.slide.x.get());
    }
```
  and replace the five tails (`refresh_ages` :885-887, `render` :938-943 — keep the `present()` call in between where it is: `self.fit(); … self.window.present(); self.set_region(..); self.update_blur(..)` becomes `self.fit(); … self.window.present(); self.set_region(..); self.update_blur(..)` only if `present()` must sit between them; read the comment there ("present(), not set_visible(true)") and keep the order if it matters, otherwise `refit()` after `present()` — say which you did and why in the report), `sync` :1221-1222 (today `fit(); update_blur(x)` with no `set_region`: use `refit()`; the extra `set_region` with unchanged x is harmless), `show_row` :1270-1272, `show_notes` :1294-1296.
- The hover check at :419 `slide.tabs_h.get() + slide.cards_h.get() + slide.footer_h.get() + 2 * RING_PX` → `p.layout.borrow().visible_height() + 2 * RING_PX` (match the variable names in that closure).
- `layout.rs`: make `region_width`, `shown_height` and `clear_strip_rect` private again and drop the Task 1 note.
- Module doc of `surface.rs`: where it describes the blur region and the slide, add one sentence that the geometry is `layout.rs`'s and the surface measures and applies.

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4; grep -n "slide\.\(cards_h\|tabs_h\|bar_h\|clear\|footer_h\)\|self\.heights" src/panel/surface.rs`
Expected: no warnings; all pass; the grep prints nothing.

- [ ] **Step 2: The e2e suite is the proof**

`cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -8`. Every geometry check (the peek at column 1566, the keyboard cards spanning 420-1180, the heights as cards are added, the Clear all strip, "+N more") must pass; the two notes checks with the stale baseline may fail as on `main`. Any other failure is this task's. If the machine cannot run it, say so; this task is not done without this run unless that is the case.

- [ ] **Step 3: Commit**

```bash
git add src/panel/surface.rs src/panel/layout.rs
git commit -m "refactor(panel): place the panel from one Layout

fit, set_region and update_blur each worked part of the geometry out
from heights kept on the Slide, and the fit-region-blur tail was copied
at five sites. The surface now measures into a Measured, keeps the
Layout it makes, applies its answers, and refits in one place; the
Slide is the animation alone.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

## Self-review notes

- Spec §1 → Task 1; §2 → Task 2. The spec's "no behaviour change" is checked by Task 2's e2e run.
- Type consistency: `Measured`'s fields in Task 1 are exactly what Task 2's `fit` fills; `Layout::input_rect` returns `blur::Rect` = `(i32, i32, i32, i32)`, destructured in Task 2's `set_region`; `blur_rects(x, scrolled)` takes `i32`s, so Task 2 rounds the `f64`s as the old code did.
- The intermediate state after Task 1 compiles because the surface imports the moved constants and helpers; the three helpers are temporarily `pub(crate)`.
- Placeholder scan: Task 2's `render` instruction leaves the `present()` ordering to the implementer with a reason to report; everything else is given.
