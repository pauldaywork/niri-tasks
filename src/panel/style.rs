//! How a task card looks: shaped like a mako notification, coloured like a
//! terminal window.
//!
//! The shape — radius, padding, gap — is mako's, copied from
//! `config/mako/config` in the ubuntu-setup repo. The font, the fill and the
//! outline are a ghostty window's under niri, from the same repo:
//! `config/ghostty/` and `config/niri/window-rules/`. Change them together.
//!
//! No card has a border. The one the keyboard is on is filled a darker black
//! than the rest. The active task is marked by colour alone: its ▶ and its
//! text are green.
//!
//! The cards are buttons, so the keyboard can move between them, and the
//! theme's button look — border, gradient, minimum height, hover and press
//! colours — is reset away so a card looks the same as a plain label would.
//!
//! Every rule is scoped to `.task-panel`, because the provider is installed for
//! the whole display and the daemon also opens task boxes, which must not pick
//! up a transparent window or a card's fill.

/// A terminal window's fill: ghostty's black `background` at
/// `background-opacity = 0.5`, under niri's window `opacity 0.75`. The terminal
/// also gets niri's blur, which the cards cannot: niri blurs a layer surface's
/// whole rectangle, and the panel's is mostly empty space.
pub const BACKGROUND: &str = "rgba(0, 0, 0, 0.375)";
/// A terminal window's own fill, before niri touches it: ghostty's black
/// `background` at `background-opacity = 0.5`. For the task box, which is a
/// window rather than a layer surface, niri adds the rest itself — the window
/// `opacity 0.75` that turns this into [`BACKGROUND`], the blur, and the 4px
/// focus ring [`OUTLINE`] imitates — so the box fills with this and draws no
/// outline of its own.
pub const WINDOW_BACKGROUND: &str = "rgba(0, 0, 0, 0.5)";
/// A terminal window's outline: niri's 4px focus ring, `#00000020`, drawn as a
/// spread shadow so it sits outside the card the way the ring sits outside the
/// window.
pub const OUTLINE: &str = "0 0 0 4px rgba(0, 0, 0, 0.125)";
/// mako `text-color`.
pub const TEXT: &str = "#f0f0f0";
/// The active task's ▶ and text. A green between Catppuccin's two, nearer
/// mocha's `#a6e3a1` (which read too pale) than latte's `#40a02b` (too dark);
/// the palette the desktop's error and warning colours come from.
pub const ACTIVE: &str = "#8cd283";
/// mako `border-radius`.
pub const RADIUS_PX: i32 = 8;
/// mako `padding`.
pub const PADDING_PX: i32 = 12;
/// ghostty `font-family` and `font-size`, which is in points too.
pub const FONT: &str = "10pt \"Iosevka Term Extended\"";
/// mako `margin=0,0,8`: the gap between stacked notifications.
pub const GAP_PX: i32 = 8;
/// Twice mako `width`, so a long description fits on one line. The peek is
/// measured from the screen edge, so this does not change how much shows while
/// tucked away.
pub const CARD_WIDTH_PX: i32 = 760;
/// The border on whatever the keyboard is on in the task box: a field or a
/// button. The panel's cards used to wear it too; they are darkened instead,
/// with [`FOCUSED_BACKGROUND`].
pub const FOCUS: &str = "1px solid #ffffff";
/// The fill of the card the keyboard is on: the same black as [`BACKGROUND`],
/// stronger, so the card stands out without a border drawn round it.
pub const FOCUSED_BACKGROUND: &str = "rgba(0, 0, 0, 0.6)";

pub fn css() -> String {
    format!(
        "
window.task-panel {{ background-color: transparent; }}
.task-panel .task-card,
.task-panel .task-card:hover,
.task-panel .task-card:active {{
    background-color: {BACKGROUND};
    background-image: none;
    color: {TEXT};
    border: none;
    border-radius: {RADIUS_PX}px;
    padding: {PADDING_PX}px;
    min-height: 0;
    min-width: 0;
    font: {FONT};
    font-weight: normal;
    box-shadow: {OUTLINE};
    outline: none;
    transition: none;
}}
/* :focus, not :focus-visible: GTK clears focus-visible 3s after the last key
   press (VISIBLE_FOCUS_DURATION), and the darker fill would vanish mid-pick. */
.task-panel .task-card:focus {{ background-color: {FOCUSED_BACKGROUND}; }}
.task-panel .task-card.active {{ color: {ACTIVE}; }}
.task-panel .task-card.blocked,
.task-panel .task-card.more {{ color: alpha({TEXT}, 0.55); }}
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The card is a mako notification in a terminal's font and colours. If this fails,
    /// either those configs changed and these should follow, or they drifted
    /// apart by accident.
    #[test]
    fn cards_carry_makos_shape_and_the_terminals_font_and_colours() {
        let css = css();
        for want in [
            "background-color: rgba(0, 0, 0, 0.375)",
            "box-shadow: 0 0 0 4px rgba(0, 0, 0, 0.125)",
            "border-radius: 8px",
            ".task-card.active { color: #8cd283; }",
            "padding: 12px",
            "font: 10pt \"Iosevka Term Extended\"",
            ".task-card:focus { background-color: rgba(0, 0, 0, 0.6); }",
        ] {
            assert!(css.contains(want), "missing `{want}`");
        }
    }

    /// Unscoped, the transparent window rule would reach the task boxes the
    /// same daemon opens.
    #[test]
    fn every_rule_is_scoped_to_the_panel() {
        for line in css().lines().filter(|l| l.contains('{')) {
            assert!(
                line.contains("task-panel"),
                "rule escapes the panel: {line}"
            );
        }
    }

    /// The card the keyboard is on is picked out by its fill alone. A border
    /// or outline there is the look this replaced.
    #[test]
    fn the_focused_card_is_darker_and_unbordered() {
        let css = css();
        let focus = css
            .lines()
            .find(|l| l.contains(".task-card:focus"))
            .expect("no :focus rule");
        assert!(
            !focus.contains("outline"),
            "focused card has an outline: {focus}"
        );
        assert!(
            !focus.contains("border"),
            "focused card has a border: {focus}"
        );
        assert_ne!(
            FOCUSED_BACKGROUND, BACKGROUND,
            "focused card is not picked out"
        );
    }
}
