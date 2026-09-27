//! How a task card looks: shaped like a mako notification, coloured like a
//! terminal window.
//!
//! The shape — radius, padding, width, gap — is mako's, copied from
//! `config/mako/config` in the ubuntu-setup repo. The font, the fill and the
//! outline are a ghostty window's under niri, from the same repo:
//! `config/ghostty/` and `config/niri/window-rules/`. Change them together.
//!
//! No card has a border, except the one the keyboard is on, which gets a 1px
//! white one. The active task is marked by colour alone: its ▶ and its text
//! are green.
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
/// mako `width`.
pub const CARD_WIDTH_PX: i32 = 380;
/// The border on the card the keyboard is on.
pub const FOCUS: &str = "1px solid #ffffff";

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
.task-panel .task-card:focus-visible {{ outline: {FOCUS}; outline-offset: -1px; }}
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
            ".task-card:focus-visible { outline: 1px solid #ffffff; outline-offset: -1px; }",
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
}
