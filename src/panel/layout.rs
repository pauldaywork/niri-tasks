//! Where the task panel's cards are on screen, as arithmetic: the surface's
//! size, the input rectangle the pointer is accepted in, and the rectangles
//! niri blurs behind the cards. `surface.rs` measures the widgets into a
//! [`Measured`] and applies what a [`Layout`] answers; nothing here touches
//! GTK or Wayland, so the one calculation that puts blur exactly under the
//! cards is tested without a compositor.

use super::blur::{self, Rect};
use super::style::{CARD_WIDTH_PX, GAP_PX, RADIUS_PX};

/// How much of each card shows while the panel is tucked away: the card's
/// 12px padding plus about one glyph, so the status icon peeks out and the
/// text stays hidden.
pub(crate) const PEEK_PX: i32 = 30;

/// Room around the cards for their CSS shadow, which the surface has to
/// contain or it is cut off square.
pub(crate) const SHADOW_PX: i32 = 16;

/// The spread of each card's outline ring (style.rs's `OUTLINE`), which is
/// drawn outside the card's box. The scroller clips its content, so the column
/// keeps this much margin inside it and the scroller is this much bigger than
/// the cards on every side; SHADOW_PX already leaves the surface room for it.
pub(crate) const RING_PX: i32 = 4;

/// The expanded cards' distance from the screen edge, matching mako's
/// `outer-margin`.
pub(crate) const EDGE_GAP_PX: i32 = 8;

/// The surface's fixed width: the shadow's room left of the expanded cards,
/// the cards, and the gap to the screen edge.
pub(crate) const SURFACE_WIDTH: i32 = SHADOW_PX + CARD_WIDTH_PX + EDGE_GAP_PX;

/// Where the cards sit when slid out on the right edge: just inside the
/// shadow's room.
pub(crate) const EXPANDED_X: f64 = SHADOW_PX as f64;

/// Where the cards sit when tucked away: far enough right that only the
/// peek is left on the surface.
pub(crate) const TUCKED_X: f64 = (SURFACE_WIDTH - PEEK_PX) as f64;

/// Where the cards sit while the panel has the keyboard and the surface is
/// pushed into the middle of the monitor by its right margin: the middle of
/// the surface, so the cards are in the middle of the screen. 784 less 760
/// leaves 12px each side, enough for the ring.
pub(crate) const CENTRED_X: f64 = ((SURFACE_WIDTH - CARD_WIDTH_PX) / 2) as f64;


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
    /// Work out where everything is from what the surface measured. The
    /// cards' height on screen is cut here once, since every answer below
    /// depends on it.
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
            // The tab bar, which does not scroll; not the card gap under it.
            rects.extend(blur::card_region((x, SHADOW_PX, width, m.bar_h), RADIUS_PX, on_screen));
            // Clear all's strip under it, when it shows; not the gap over it
            // or the room to its left.
            if m.clear.1 > 0 {
                let strip = clear_strip_rect(x, m.bar_h, m.clear);
                if strip.2 > 0 {
                    rects.extend(blur::card_region(strip, RADIUS_PX, on_screen));
                }
            }
        }
        // The cards as laid out in the column under the tabs, moved up by
        // however far it is scrolled, and cut to the part of the column on
        // screen.
        let top = SHADOW_PX + m.tabs_h;
        let mut y = top - scrolled;
        let mut cards = Vec::new();
        for &h in &m.card_heights {
            cards.extend(blur::card_region((x, y, width, h), RADIUS_PX, on_screen));
            y += h + GAP_PX;
        }
        rects.extend(blur::clip_rows(&cards, top, top + self.cards_h));
        if m.footer_h > 0 {
            // The footer, which does not scroll: under the cards on screen
            // and the card gap, not the gap itself.
            let y = top + self.cards_h + GAP_PX;
            rects.extend(blur::card_region((x, y, width, m.footer_h - GAP_PX), RADIUS_PX, on_screen));
        }
        rects
    }
}

/// The right margin that puts the surface in the middle of a monitor this
/// wide: half the room it leaves, or none on a monitor too narrow for it.
pub(crate) fn centre_margin(monitor_w: i32) -> i32 {
    ((monitor_w - SURFACE_WIDTH) / 2).max(0)
}

/// How wide the input region is from `x`: to the surface's edge, or, with the
/// panel centred, just the cards.
// Crate-visible only while surface.rs still calls it; Task 2 folds these callers into Layout and makes it private again.
pub(crate) fn region_width(x: i32, centred: bool) -> i32 {
    if centred { CARD_WIDTH_PX } else { SURFACE_WIDTH - x }
}

/// How tall the column of cards is on screen: all of it, or as much as fits
/// inside the screen's margins less the `bars_h` the filter tabs over them and
/// the keys footer under them take, with the rest scrolled. Only the
/// keyboard's wrapped cards, or "+N more" opened onto a long list, get that
/// tall.
// Crate-visible only while surface.rs still calls it; Task 2 folds these callers into Layout and makes it private again.
pub(crate) fn shown_height(cards_h: i32, bars_h: i32, screen_h: i32) -> i32 {
    cards_h.min(screen_h - 2 * (SHADOW_PX + EDGE_GAP_PX) - bars_h).max(0)
}

/// The scroll position that shows all of `top..bottom`, moving as little as it
/// can from `value` with `page` of the column in view. A card taller than the
/// page shows its top.
pub(crate) fn scroll_to_show(value: f64, page: f64, top: f64, bottom: f64) -> f64 {
    if top < value || bottom - top > page {
        top
    } else if bottom > value + page {
        bottom - page
    } else {
        value
    }
}

/// Where Clear all's strip is, for the blur behind it: a card gap under the
/// bar, its right edge on the cards', and cut at the surface's edge as the
/// cards are, to no width at all once it is wholly past it.
// Crate-visible only while surface.rs still calls it; Task 2 folds these callers into Layout and makes it private again.
pub(crate) fn clear_strip_rect(x: i32, bar_h: i32, (w, h): (i32, i32)) -> blur::Rect {
    let left = x + CARD_WIDTH_PX - w;
    (left, SHADOW_PX + bar_h + GAP_PX, w.min(SURFACE_WIDTH - left).max(0), h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_region_runs_to_the_edge_unless_centred() {
        assert_eq!(region_width(100, false), SURFACE_WIDTH - 100);
        assert_eq!(region_width(CENTRED_X as i32, true), CARD_WIDTH_PX);
    }

    #[test]
    fn the_ring_margin_is_the_outlines_spread() {
        // A mismatch would clip the ring against the scroller again.
        assert!(super::super::style::OUTLINE.contains(&format!(" {RING_PX}px ")));
    }

    #[test]
    fn centred_cards_have_equal_margins_in_the_surface() {
        // The compositor centres the surface, so the cards are centred on the
        // screen only if they are centred inside it.
        let left = CENTRED_X as i32;
        let right = SURFACE_WIDTH - CARD_WIDTH_PX - left;
        assert_eq!(left, right);
    }

    #[test]
    fn the_centre_margin_leaves_equal_room_either_side() {
        let margin = centre_margin(3440);
        assert_eq!(3440 - margin - SURFACE_WIDTH, margin);
    }

    #[test]
    fn a_monitor_narrower_than_the_surface_gets_no_margin() {
        assert_eq!(centre_margin(600), 0);
    }

    #[test]
    fn centred_cards_leave_room_for_their_ring() {
        // Less than this and the surface cuts the outline ring off square.
        assert!(CENTRED_X as i32 >= RING_PX);
    }

    #[test]
    fn a_short_column_shows_whole() {
        assert_eq!(shown_height(400, 0, 1080), 400);
    }

    #[test]
    fn a_tall_column_stops_at_the_screens_margins() {
        // 1080 less the shadow room and the edge gap, top and bottom.
        assert_eq!(shown_height(5000, 0, 1080), 1080 - 2 * (SHADOW_PX + EDGE_GAP_PX));
    }

    /// The tab strip does not scroll, so the cards get the screen less it.
    #[test]
    fn the_tabs_take_their_height_off_the_cards() {
        assert_eq!(shown_height(400, 50, 1080), 400, "a short column still shows whole");
        assert_eq!(shown_height(5000, 50, 1080), 1080 - 2 * (SHADOW_PX + EDGE_GAP_PX) - 50);
    }

    /// The strip's bottom margin is the card gap less the ring the column
    /// keeps above the first card; less than none would not build.
    #[test]
    fn the_gap_under_the_tabs_leaves_room_for_the_ring() {
        const _: () = assert!(GAP_PX >= RING_PX);
    }

    #[test]
    fn a_card_in_view_does_not_scroll() {
        assert_eq!(scroll_to_show(100.0, 500.0, 150.0, 300.0), 100.0);
    }

    #[test]
    fn a_card_below_scrolls_up_until_its_bottom_shows() {
        assert_eq!(scroll_to_show(0.0, 500.0, 450.0, 600.0), 100.0);
    }

    #[test]
    fn a_card_above_scrolls_down_to_its_top() {
        assert_eq!(scroll_to_show(300.0, 500.0, 100.0, 250.0), 100.0);
    }

    /// Taller than the screen: its top, where the icon and the start of the
    /// description are.
    #[test]
    fn a_card_taller_than_the_page_shows_its_top() {
        assert_eq!(scroll_to_show(0.0, 500.0, 200.0, 900.0), 200.0);
    }

    /// Clear all's strip sits a card gap under the bar, its right edge on
    /// the cards' right edge, its own size.
    #[test]
    fn clear_alls_strip_sits_under_the_bar_at_the_cards_right_edge() {
        let x = SURFACE_WIDTH - CARD_WIDTH_PX;
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

    /// At the peek's x, as when 5 is pressed during the slide-in, the cards
    /// are far past the edge: the strip's width is 0, never negative, which
    /// the Wayland region would take.
    #[test]
    fn clear_alls_strip_has_no_width_at_the_peeks_x() {
        assert_eq!(clear_strip_rect(TUCKED_X as i32, 30, (120, 36)).2, 0);
    }

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
}
