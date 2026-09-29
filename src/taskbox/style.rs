//! How the task box looks: a terminal window, like the task panel's cards.
//!
//! The font, text colour, radius, padding and focus border are the panel's
//! (`src/panel/style.rs`), so a change there follows here. The fill is the
//! terminal's own, because niri dresses the box as a window — see
//! [`WINDOW_BACKGROUND`].
//!
//! Every rule is scoped to `.task-box`: the provider is installed for the whole
//! display, and in the daemon that display also holds the task panels.

use crate::panel::style::{FOCUS, FONT, PADDING_PX, RADIUS_PX, TEXT, WINDOW_BACKGROUND};

/// The description's and each note row's fill: a faint lift off the window,
/// so each reads as its own block without drawing a border.
pub const FIELD: &str = "rgba(255, 255, 255, 0.06)";

pub fn css() -> String {
    format!(
        "
window.task-box {{ background-color: {WINDOW_BACKGROUND}; color: {TEXT}; font: {FONT}; }}
.task-box label, .task-box textview, .task-box textview text {{ background-color: transparent; color: {TEXT}; font: {FONT}; }}
.task-box .field {{ background-color: {FIELD}; border-radius: {RADIUS_PX}px; padding: {PADDING_PX}px; }}
.task-box .field:focus-within {{ outline: {FOCUS}; outline-offset: -1px; }}
.task-box .dim {{ color: alpha({TEXT}, 0.55); }}
.task-box .date {{ color: alpha({TEXT}, 0.55); font-size: 80%; }}
.task-box button, .task-box button:hover, .task-box button:active {{
    background-image: none; background-color: {FIELD}; color: {TEXT};
    border: none; border-radius: {RADIUS_PX}px; padding: 4px 12px;
    min-height: 0; box-shadow: none; outline: none; font: {FONT}; }}
.task-box button:focus {{ outline: {FOCUS}; outline-offset: -1px; }}
.task-box button.delete {{ background-color: transparent; color: alpha({TEXT}, 0.55); padding: 0 6px; }}
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_wears_the_terminals_font_fill_and_focus_border() {
        let css = css();
        for want in [
            "window.task-box { background-color: rgba(0, 0, 0, 0.5);",
            "font: 10pt \"Iosevka Term Extended\"",
            "color: #f0f0f0",
            ".task-box .field:focus-within { outline: 1px solid #ffffff; outline-offset: -1px; }",
            "border-radius: 8px",
            "padding: 12px",
        ] {
            assert!(css.contains(want), "missing `{want}`");
        }
    }

    /// Unscoped, a `window` rule here would paint the panels' clear surface.
    #[test]
    fn every_rule_is_scoped_to_the_box() {
        let css = css();
        assert!(css.lines().any(|l| l.contains('{')), "no rules at all");
        for line in css.lines().filter(|l| l.contains('{')) {
            assert!(line.contains("task-box"), "rule escapes the box: {line}");
        }
    }
}
