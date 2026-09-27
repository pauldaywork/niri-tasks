//! How a task card looks: a mako notification, on purpose.
//!
//! These are mako's values rather than the theme's, so a task card and a
//! notification sit side by side on the right edge and read as one family. They
//! are copied from `config/mako/config` in the ubuntu-setup repo; change them
//! together.
//!
//! Every rule is scoped to `.task-panel`, because the provider is installed for
//! the whole display and the daemon also opens task boxes, which must not pick
//! up a transparent window or a card's border.

/// mako `background-color=#130f1acc`.
pub const BACKGROUND: &str = "#130f1acc";
/// mako `text-color`.
pub const TEXT: &str = "#f0f0f0";
/// mako `[urgency=normal] border-color`, for the active task.
pub const ACTIVE_BORDER: &str = "#eda792";
/// mako `[urgency=low] border-color`, for everything else.
pub const PENDING_BORDER: &str = "#d9bcb8";
/// mako `border-size`.
pub const BORDER_PX: i32 = 2;
/// mako `border-radius`.
pub const RADIUS_PX: i32 = 8;
/// mako `padding`.
pub const PADDING_PX: i32 = 12;
/// mako `font`.
pub const FONT: &str = "10pt \"Noto Sans\"";
/// mako `margin=0,0,8`: the gap between stacked notifications.
pub const GAP_PX: i32 = 8;
/// mako `width`.
pub const CARD_WIDTH_PX: i32 = 380;

pub fn css() -> String {
    format!(
        "
window.task-panel {{ background-color: transparent; }}
.task-panel .task-card {{
    background-color: {BACKGROUND};
    color: {TEXT};
    border: {BORDER_PX}px solid {PENDING_BORDER};
    border-radius: {RADIUS_PX}px;
    padding: {PADDING_PX}px;
    font: {FONT};
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
}}
.task-panel .task-card.active {{ border-color: {ACTIVE_BORDER}; }}
.task-panel .task-card.blocked,
.task-panel .task-card.more {{ color: alpha({TEXT}, 0.55); }}
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The card is a mako notification. If this fails, either mako changed and
    /// these should follow it, or they drifted apart by accident.
    #[test]
    fn cards_carry_makos_values() {
        let css = css();
        for want in [
            "background-color: #130f1acc",
            "border: 2px solid #d9bcb8",
            "border-color: #eda792",
            "border-radius: 8px",
            "padding: 12px",
            "font: 10pt \"Noto Sans\"",
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
