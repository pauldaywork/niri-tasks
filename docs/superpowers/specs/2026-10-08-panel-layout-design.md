# Panel layout design

P6 of the 2026-10-08 (v3) architecture review. `src/panel/surface.rs` is
1,920 lines behind a five-item interface, and the geometry that places the
panel on screen is spread through it: the surface's size in `fit`, the input
rectangle in `set_region`, the blur rectangles in `update_blur`, the slide
position and the measured heights together in one `Slide` struct. The
sequence `fit(); set_region(x.min(to)); update_blur(x)` is hand-copied at
five sites, which is the deletion-test signal: delete it and it reappears at
every caller. The one piece of arithmetic that puts blur exactly under the
cards (`update_blur`, 40 lines) has no test, while the small helpers around
it have thirteen.

This design moves the geometry into `src/panel/layout.rs`: a `Measured`
value the surface fills from GTK, and a pure `Layout` that answers the three
questions the surface asks (how tall is the surface, where is the input
rectangle, where are the blur rectangles). `blur.rs` stays the Wayland
adapter beneath it. No behaviour changes: the e2e panel measurements (peek
at column 1566 of 1600, keyboard cards spanning 420-1180) must come out the
same.

## 1. `src/panel/layout.rs`

Constants move here from `surface.rs`: `PEEK_PX`, `SHADOW_PX`, `RING_PX`,
`EDGE_GAP_PX`, `SURFACE_WIDTH`, `EXPANDED_X`, `TUCKED_X`, `CENTRED_X`
(all `pub(crate)`). `CARD_WIDTH_PX`, `GAP_PX` and `RADIUS_PX` stay in
`style.rs`, where the stylesheet uses them.

```rust
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
    /// 0 without the keyboard.
    pub tabs_h: i32,
    /// The tab bar alone, for the blur.
    pub bar_h: i32,
    /// Clear all's strip's width and height; (0, 0) while hidden.
    pub clear: (i32, i32),
    /// The footer with the gap over it; 0 without the keyboard or on Ideas.
    pub footer_h: i32,
    /// The monitor's height.
    pub screen_h: i32,
}

/// Where everything is, worked out from what was measured: the surface's
/// size, and the rectangles the input region and the blur need. Pure, so the
/// arithmetic that puts blur under the cards is tested without a compositor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    measured: Measured,
    /// The cards' height on screen: the column, or as much as fits.
    cards_h: i32,
}

impl Layout {
    pub fn new(measured: Measured) -> Layout;        // cards_h = shown_height(column_h, tabs_h + footer_h, screen_h)
    pub fn cards_h(&self) -> i32;
    /// From the top of the tabs to the bottom of the footer: what the input region covers.
    pub fn visible_height(&self) -> i32;              // tabs_h + cards_h + footer_h
    /// The surface, shadow included.
    pub fn surface_height(&self) -> i32;              // visible_height + 2 * SHADOW_PX
    /// The input rectangle for cards at `x`: to the surface's edge, or, centred, just the cards.
    pub fn input_rect(&self, x: i32, centred: bool) -> blur::Rect;   // (x, SHADOW_PX, region_width(x, centred), visible_height)
    /// The blur rectangles for cards at `x` with the column scrolled by `scrolled`: the bar, Clear all's strip, each card cut to the band on screen, the footer.
    pub fn blur_rects(&self, x: i32, scrolled: i32) -> Vec<blur::Rect>;   // today's update_blur body
}

pub(crate) fn centre_margin(monitor_w: i32) -> i32;
pub(crate) fn scroll_to_show(value: f64, page: f64, top: f64, bottom: f64) -> f64;
fn region_width(x: i32, centred: bool) -> i32;
fn shown_height(cards_h: i32, bars_h: i32, screen_h: i32) -> i32;
fn clear_strip_rect(x: i32, bar_h: i32, (w, h): (i32, i32)) -> blur::Rect;
```

The five pure helpers move with their sixteen tests. New tests pin
`blur_rects` and `input_rect` on a fixture `Measured` by bounding boxes
(a `bounds(rects) -> (left, top, right, bottom)` helper over the corner
strips `blur::card_region` makes): the bar's band sits at `SHADOW_PX` and
is `bar_h` tall; the first card starts at `SHADOW_PX + tabs_h`; scrolling by
`s` moves the cards up by `s` and clips them to the band
`top..top + cards_h`; the footer starts at `top + cards_h + GAP_PX` and is
`footer_h - GAP_PX` tall; at the tucked `x` the widths narrow to
`SURFACE_WIDTH - x` and no right corners are rounded; with `tabs_h == 0`
and `footer_h == 0` only cards appear; `surface_height` and `visible_height`
agree with the sums; `input_rect` is the whole width tucked and
`CARD_WIDTH_PX` wide centred.

## 2. `surface.rs` on top of it

- `Slide` keeps the animation (`x`, `from`, `to`, `start_us`, `ticking`,
  `grace`, `margin*`) and loses the five measured heights; the field
  `heights: RefCell<Vec<i32>>` goes. A new field `layout: RefCell<Layout>`
  holds the last `fit`.
- `fit` measures as today into a `Measured` (`card_heights`, `column_h`,
  `tabs_h`, `bar_h`, `clear`, `footer_h`, the monitor's height), stores
  `Layout::new(measured)`, and sets the three size requests from
  `layout.surface_height()`.
- `set_region(x)` builds the cairo region from `layout.input_rect(x,
  state.keyboard())`; `update_blur(x)` calls `blur.set(&layout.blur_rects(x,
  scrolled))` with the scroller's value. Both become a few lines.
- One `refit(&self)` does `fit(); set_region(x.min(to)); update_blur(x)` and
  replaces the hand-copied tails in `refresh_ages`, `render`, `sync`
  (the Clear all relabel), `show_row` and `show_notes`. `jump_to` and
  `slide_to` keep calling `set_region`/`update_blur` directly, since they move
  `x` without re-measuring.
- The one other reader of the heights (`surface.rs:419`, the hover's
  region check) uses `layout.visible_height()`.
- The module doc names `layout.rs` as where the geometry lives.

## 3. Out of scope

`blur.rs`'s `card_region` and `clip_rows` stay where they are; `Blur` stays
the protocol adapter. The card widget factory and the footer text, the other
two modules the review named inside `surface.rs`, are later work. ADR 0001's
reasoning (blur per card over `ext-background-effect-v1`) is untouched; only
where the rectangles are computed moves.

## Tests

`cargo test` green and warning-free after each task; the e2e panel suite's
measurements unchanged (it is the proof that the geometry did not move).
