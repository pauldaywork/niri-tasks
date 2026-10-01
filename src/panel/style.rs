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
//! A card is a box holding a body button and, while the panel has the
//! keyboard, a row of action buttons along its bottom edge. The theme's button
//! look (border, gradient, minimum height, hover and press colours) is reset
//! away from both, so the body looks the same as a plain label would, and the
//! card's box carries the fill, the rounding and the outline.
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
/// The keyboard's action buttons, each its own colour from the same Catppuccin
/// palette as [`ACTIVE`] (mocha's): Go to session blue, Refine mauve, Edit
/// yellow, Stop peach, Waiting teal, Remove red. Start shares [`ACTIVE`]'s green, because
/// starting a task is what turns its card green.
pub const REFINE: &str = "#cba6f7";
/// Edit yellow.
pub const EDIT: &str = "#f9e2af";
/// Stop peach.
pub const STOP: &str = "#fab387";
/// Waiting teal.
pub const WAIT: &str = "#94e2d5";
/// Remove red.
pub const REMOVE: &str = "#f38ba8";
/// Go to session blue: mocha's `blue`, apart from every other button's colour.
pub const SESSION: &str = "#89b4fa";
/// Text on a solid colour fill, the armed Remove: mocha's `base`, so it reads
/// as dark on red.
pub const ON_FILL: &str = "#1e1e2e";
/// Each button's class, as `Action::name()` gives it, and its colour.
pub const ACTION_COLOURS: [(&str, &str); 7] = [
    ("session", SESSION),
    ("start", ACTIVE),
    ("refine", REFINE),
    ("edit", EDIT),
    ("stop", STOP),
    ("wait", WAIT),
    ("remove", REMOVE),
];
/// mako `border-radius`.
pub const RADIUS_PX: i32 = 8;
/// mako `padding`.
pub const PADDING_PX: i32 = 12;
/// The action row's buttons: half a card's padding top and bottom, so the row
/// reads as a footer to the description rather than a second card.
pub const ACTION_PADDING: &str = "6px 12px";
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
/// The lines inside a keyboard card: one under the description, and one
/// between each pair of buttons. A faint white, so they part things as a
/// subtle line without competing with the buttons' colours.
pub const SEPARATOR: &str = "rgba(255, 255, 255, 0.12)";
/// The fill of the card the keyboard is on: the same black as [`BACKGROUND`],
/// stronger, so the card stands out without a border drawn round it.
pub const FOCUSED_BACKGROUND: &str = "rgba(0, 0, 0, 0.6)";

pub fn css() -> String {
    let mut css = format!(
        "
window.task-panel {{ background-color: transparent; }}
.task-panel .task-card {{
    background-color: {BACKGROUND};
    color: {TEXT};
    border-radius: {RADIUS_PX}px;
    font: {FONT};
    box-shadow: {OUTLINE};
}}
.task-panel .card-body,
.task-panel .card-actions button {{
    background-color: transparent;
    background-image: none;
    color: inherit;
    border: none;
    border-radius: 0;
    padding: {PADDING_PX}px;
    min-height: 0;
    min-width: 0;
    font: inherit;
    font-weight: normal;
    box-shadow: none;
    outline: none;
    transition: none;
}}
/* Clear the theme's hover and press look so the button keeps its own colour,
   padding and rounding. */
.task-panel .card-body:hover,
.task-panel .card-body:active,
.task-panel .card-actions button:hover,
.task-panel .card-actions button:active {{
    background-color: transparent;
    background-image: none;
    border: none;
    box-shadow: none;
    outline: none;
}}
/* The body keeps the card's whole rounding while it is the only thing in it. */
.task-panel .card-body {{ border-radius: {RADIUS_PX}px; }}
.task-panel .card-actions button {{ padding: {ACTION_PADDING}; }}
.task-panel .card-actions button:first-child {{ border-bottom-left-radius: {RADIUS_PX}px; }}
/* The focused button's name, dimmed so it reads as a caption to the icons. */
.task-panel .card-hint {{ color: alpha({TEXT}, 0.55); padding: {ACTION_PADDING}; }}
.task-panel .card-separator {{
    background-color: {SEPARATOR};
    background-image: none;
    min-height: 1px;
    margin: 0;
}}
/* Only between buttons: a line along the card's own edge would part nothing.
   After the hover and press reset, which clears borders, so it stays put. */
.task-panel .card-actions button:not(:first-child) {{ border-left: 1px solid {SEPARATOR}; }}
/* :focus-within, not :focus-visible: GTK clears focus-visible 3s after the
   last key press (VISIBLE_FOCUS_DURATION), and the darker fill would vanish
   mid-pick. Within, so the card stays darkened while one of its buttons is
   focused. */
.task-panel .task-card:focus-within {{ background-color: {FOCUSED_BACKGROUND}; }}
.task-panel .task-card.active {{ color: {ACTIVE}; }}
.task-panel .task-card.blocked,
.task-panel .task-card.more {{ color: alpha({TEXT}, 0.55); }}
"
    );
    for (name, colour) in ACTION_COLOURS {
        css.push_str(&format!(
            ".task-panel .card-actions .{name} {{ color: {colour}; }}\n\
             .task-panel .card-actions .{name}:focus {{ background-color: alpha({colour}, 0.2); }}\n"
        ));
    }
    css.push_str(&format!(
        ".task-panel .card-actions .remove.confirm,\n\
         .task-panel .card-actions .remove.confirm:focus {{ background-color: {REMOVE}; color: {ON_FILL}; }}\n"
    ));
    css
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
            ".task-card:focus-within { background-color: rgba(0, 0, 0, 0.6); }",
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

    /// Each button in its own colour, Start sharing the active task's green.
    /// A new Action without a colour fails here, not as a grey button.
    #[test]
    fn every_action_button_has_its_colour() {
        let css = css();
        for action in crate::panel::actions::Action::ALL {
            let (_, colour) = ACTION_COLOURS
                .iter()
                .find(|(name, _)| *name == action.name())
                .unwrap_or_else(|| panic!("no colour for {}", action.name()));
            let rule = format!(".card-actions .{} {{ color: {colour}; }}", action.name());
            assert!(css.contains(&rule), "missing `{rule}`");
        }
        assert!(css.contains(".card-actions .start { color: #8cd283; }"), "Start is ACTIVE's green");
    }

    /// A faint line parts the description from the buttons.
    #[test]
    fn a_solid_line_parts_the_text_from_the_buttons() {
        let css = css();
        assert!(css.contains(".task-panel .card-separator {"));
        assert!(css.contains("background-color: rgba(255, 255, 255, 0.12);"));
        assert!(css.contains("min-height: 1px;"));
    }

    /// A line between each pair of buttons, and none before the first, which
    /// would sit on the card's edge.
    #[test]
    fn a_line_parts_each_button_from_the_next() {
        let css = css();
        let rule = ".card-actions button:not(:first-child) { border-left: 1px solid rgba(255, 255, 255, 0.12); }";
        let at = css.find(rule).expect("missing the line between buttons");
        let reset = css.find(".card-actions button:active").expect("missing the press reset");
        assert!(at > reset, "the press reset's `border: none` would win over the line");
    }

    /// The row sits in the card's bottom-left corner and shares its rounding
    /// there. Its last button ends mid-card, so nothing rounds it off.
    #[test]
    fn the_action_row_rounds_off_with_the_card() {
        let css = css();
        assert!(css.contains(".card-actions button:first-child { border-bottom-left-radius: 8px; }"));
        assert!(!css.contains(".card-actions button:last-child"));
    }

    /// Hover and press must not override the button's own colour, padding or rounding,
    /// or the pointer action would dim the button's colour and jump its size.
    #[test]
    fn hover_and_press_keep_each_buttons_own_look() {
        let css = css();
        // Parse CSS rules: selector { body }.
        // Split on '}' to get individual rules, then split on '{' to separate selector from body.
        for rule in css.split('}') {
            if let Some(brace_pos) = rule.find('{') {
                let selector = &rule[..brace_pos];
                let body = &rule[brace_pos + 1..];
                // If selector contains :hover or :active, check that body doesn't set
                // colour, padding, or border-radius.
                if selector.contains(":hover") || selector.contains(":active") {
                    for line in body.lines() {
                        let trimmed = line.trim_start();
                        if trimmed.starts_with("color:") {
                            panic!(
                                "hover/active rule must not set color: selector=[{}] body=[{}]",
                                selector, body
                            );
                        }
                        if trimmed.starts_with("padding:") {
                            panic!(
                                "hover/active rule must not set padding: selector=[{}] body=[{}]",
                                selector, body
                            );
                        }
                        if trimmed.starts_with("border-radius:") {
                            panic!(
                                "hover/active rule must not set border-radius: selector=[{}] body=[{}]",
                                selector, body
                            );
                        }
                    }
                }
            }
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
