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
//! text are green. An up next task's icon and text are yellow, unless it is
//! active or waiting. Blocked, waiting and finished cards are dimmed.
//!
//! A card is a box holding a body button and, while the panel has the
//! keyboard, a row of action buttons along its bottom edge. The theme's button
//! look (border, gradient, minimum height, hover and press colours) is reset
//! away from both, so the body looks the same as a plain label would, and the
//! card's box carries the fill, the rounding and the outline.
//!
//! While the panel has the keyboard, the filter tabs above the cards are one
//! bar in a card's look, and a tab with nothing in it shows one line in a
//! card's look too. On the Waiting tab, Clear all sits on a strip of its own
//! under the bar, in Remove's red.
//!
//! Every rule is scoped to `.task-panel`, because the provider is installed for
//! the whole display and the daemon also opens task boxes, which must not pick
//! up a transparent window or a card's fill.

use crate::actions::Action;

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
/// An up next task's icon and text: mocha's `yellow`, the Edit button's
/// colour, and the Up next button's.
pub const UP_NEXT: &str = "#f9e2af";
/// The keyboard's action buttons, each its own colour from the same Catppuccin
/// palette as [`ACTIVE`] (mocha's): Go to session blue, Back to list lavender, Refine mauve, Edit
/// yellow, Complete green, Move sapphire, Speak pink, Stop peach, Waiting teal, Remove red. Start shares [`ACTIVE`]'s green, because
/// starting a task is what turns its card green, and Up next shares Edit's yellow, [`UP_NEXT`],
/// because it is what turns its card yellow.
pub const REFINE: &str = "#cba6f7";
/// Edit yellow.
pub const EDIT: &str = "#f9e2af";
/// Speak pink: mocha's `pink`, which no other button wears.
pub const SPEAK: &str = "#f5c2e7";
/// Stop peach.
pub const STOP: &str = "#fab387";
/// Waiting teal.
pub const WAIT: &str = "#94e2d5";
/// Remove red.
pub const REMOVE: &str = "#f38ba8";
/// Back to list lavender: mocha's `lavender`, the colour nearest Go to
/// session's blue, for the other button that takes you back to something.
pub const BACK: &str = "#b4befe";
/// Go to session blue: mocha's `blue`, apart from every other button's colour.
pub const SESSION: &str = "#89b4fa";
/// Complete green: mocha's own `green`, paler than [`ACTIVE`]'s, for the step
/// after the work rather than the work.
pub const COMPLETE: &str = "#a6e3a1";
/// Move to workspace sapphire: mocha's `sapphire`, between Go to session's
/// blue and Waiting's teal, apart from both.
pub const MOVE: &str = "#74c7ec";
/// Text on a solid colour fill, the armed Remove: mocha's `base`, so it reads
/// as dark on red.
pub const ON_FILL: &str = "#1e1e2e";
/// Each task action's colour, from its row of [`Action::facts`].
pub fn colour(action: Action) -> &'static str {
    action.facts().colour
}
/// mako `border-radius`.
pub const RADIUS_PX: i32 = 8;
/// mako `padding`.
pub const PADDING_PX: i32 = 12;
/// The action row's buttons: half a card's padding top and bottom, so the row
/// reads as a footer to the description rather than a second card.
pub const ACTION_PADDING: &str = "6px 12px";
/// ghostty `font-family` and `font-size`, which is in points too.
pub const FONT: &str = "10pt \"Iosevka Term Extended\"";
/// The keys' hints, beside the focused card's buttons and on the footer under
/// the list: a point under [`FONT`], so they read as captions and the widest
/// hint fits beside the widest action row.
pub const HINT_FONT: &str = "9pt \"Iosevka Term Extended\"";
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
/// The fill of the filter tab that is picked: a faint white, so it reads as
/// lit against the strip without taking a colour from the action buttons.
pub const CURRENT_TAB: &str = "rgba(255, 255, 255, 0.12)";

pub fn css() -> String {
    let mut css = format!(
        "
window.task-panel {{ background-color: transparent; }}
.task-panel .task-card,
.task-panel .filter-tabs,
.task-panel .filter-empty,
.task-panel .ideas,
.task-panel .keys-footer {{
    background-color: {BACKGROUND};
    color: {TEXT};
    border-radius: {RADIUS_PX}px;
    font: {FONT};
    box-shadow: {OUTLINE};
}}
.task-panel .card-body,
.task-panel .card-actions button,
.task-panel .filter-tabs button {{
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
.task-panel .card-actions button:active,
.task-panel .filter-tabs button:hover,
.task-panel .filter-tabs button:active {{
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
/* The focused card's keys, dimmed and a point smaller so they read as a
   caption to the icons. */
.task-panel .card-hint {{ color: alpha({TEXT}, 0.55); padding: {ACTION_PADDING}; font: {HINT_FONT}; }}
/* How long ago the task was added, at the card's right end: a caption, so
   dimmed like the hint, on a green or yellow card too. */
.task-panel .card-age {{ color: alpha({TEXT}, 0.55); padding-left: {PADDING_PX}px; }}
/* A card's notes, once its body is pressed: captions to the description,
   so dimmed like its age, a padding's gap below it. */
.task-panel .card-notes {{ padding-top: {PADDING_PX}px; }}
.task-panel .card-note {{ color: alpha({TEXT}, 0.55); }}
.task-panel .card-separator {{
    background-color: {SEPARATOR};
    background-image: none;
    min-height: 1px;
    margin: 0;
}}
/* Only between buttons: a line along the card's own edge would part nothing.
   After the hover and press reset, which clears borders, so it stays put. */
.task-panel .card-actions button:not(:first-child) {{ border-left: 1px solid {SEPARATOR}; }}
/* The filter tabs: dimmed, apart from the one that is picked, and parted
   like the action row's buttons. After the resets, which clear borders and
   fills. */
.task-panel .filter-tabs button {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); }}
.task-panel .filter-tabs button:not(:first-child) {{ border-left: 1px solid {SEPARATOR}; }}
.task-panel .filter-tabs button.current {{ color: {TEXT}; background-color: {CURRENT_TAB}; }}
/* Clear all, on its strip under the Waiting tab's bar: Remove's red, and
   filled red once armed, as the armed Remove is. Its classes outrank the
   tabs' dimmed colour and the press reset's clear fill. */
.task-panel .filter-tabs .clear-all {{ color: {REMOVE}; }}
.task-panel .filter-tabs .clear-all.confirm {{ background-color: {REMOVE}; color: {ON_FILL}; }}
/* The project list's text field, in the tab bar's place: the bar's own
   fill and rounding, so no frame, fill or focus ring of the theme's. */
.task-panel .filter-tabs entry.project-query {{
    background-color: transparent;
    background-image: none;
    color: {TEXT};
    caret-color: {TEXT};
    border: none;
    border-radius: 0;
    box-shadow: none;
    outline: none;
    min-height: 0;
    padding: {ACTION_PADDING};
    font: inherit;
}}
/* An empty tab's one line, padded like a card's text and dimmed like
   \"+N more\". */
.task-panel .filter-empty {{ padding: {PADDING_PX}px; color: alpha({TEXT}, 0.55); }}
/* The Ideas tab's text area: the card's tint shows through it, and its text
   and caret are the cards' colour. */
.task-panel .ideas textview,
.task-panel .ideas textview text {{ background-color: transparent; color: {TEXT}; caret-color: {TEXT}; }}
/* The keys every card shares, under the list: a strip like the tab bar,
   its text dimmed and small like the card's hint. */
.task-panel .keys-footer {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); font: {HINT_FONT}; }}
/* :focus-within, not :focus-visible: GTK clears focus-visible 3s after the
   last key press (VISIBLE_FOCUS_DURATION), and the darker fill would vanish
   mid-pick. Within, so the card stays darkened while one of its buttons is
   focused. */
.task-panel .task-card:focus-within,
.task-panel .task-card.picked {{ background-color: {FOCUSED_BACKGROUND}; }}
.task-panel .task-card.active {{ color: {ACTIVE}; }}
.task-panel .task-card.blocked,
.task-panel .task-card.waiting,
.task-panel .task-card.finished,
.task-panel .task-card.more {{ color: alpha({TEXT}, 0.55); }}
/* After the dimmed rule, which it outranks by coming later: an up next
   blocked card is yellow, lock and all. Active, waiting and finished cards
   never get the class (Card::shows_up_next). */
.task-panel .task-card.up-next {{ color: {UP_NEXT}; }}
"
    );
    for action in Action::ALL {
        let (name, colour) = (action.class(), colour(action));
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
            ".task-card.picked { background-color: rgba(0, 0, 0, 0.6); }",
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
    #[test]
    fn every_action_button_has_its_colour() {
        let css = css();
        for action in Action::ALL {
            let rule = format!(".card-actions .{} {{ color: {}; }}", action.class(), colour(action));
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

    /// The tab strip and an empty tab's line sit among the cards, so they
    /// wear a card's fill, rounding, font and outline.
    #[test]
    fn the_tabs_and_the_empty_line_look_like_cards() {
        let css = css();
        let rule = css
            .split('}')
            .find(|r| r.contains(".task-panel .task-card,") && r.contains("background-color"))
            .expect("no shared card rule");
        assert!(rule.contains(".task-panel .filter-tabs"), "the strip lacks the card look");
        assert!(rule.contains(".task-panel .filter-empty"), "the empty line lacks the card look");
        assert!(rule.contains(".task-panel .keys-footer"), "the footer lacks the card look");
    }

    /// The keys' hints are captions: dimmed, a point under the cards' text,
    /// so more fits beside the buttons. The footer's rule comes after the
    /// card look it shares, whose font would otherwise win.
    #[test]
    fn the_hints_are_dimmed_and_a_point_smaller() {
        let css = css();
        assert!(css.contains(&format!(
            ".task-panel .card-hint {{ color: alpha({TEXT}, 0.55); padding: {ACTION_PADDING}; font: {HINT_FONT}; }}"
        )));
        let footer = format!(
            ".task-panel .keys-footer {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); font: {HINT_FONT}; }}"
        );
        let at = css.find(&footer).unwrap_or_else(|| panic!("missing `{footer}`"));
        let shared = css.find(".task-panel .task-card,").expect("no shared card rule");
        assert!(at > shared, "the card look's font would win over the footer's");
        assert!(FONT.starts_with("10pt "), "the hint is a point under the cards' text");
    }

    /// The notepad is drawn as a card is, its text view see-through over it.
    #[test]
    fn the_ideas_notepad_wears_a_cards_look() {
        let css = css();
        assert!(css.contains(".task-panel .filter-empty,\n.task-panel .ideas,\n"));
        assert!(css.contains(".task-panel .ideas textview text { background-color: transparent;"));
    }

    /// The picked tab is full text on a faint fill; the rest are dimmed.
    #[test]
    fn the_picked_tab_stands_out() {
        let css = css();
        assert!(css.contains(&format!(
            ".task-panel .filter-tabs button.current {{ color: {TEXT}; background-color: {CURRENT_TAB}; }}"
        )));
        assert!(css.contains(&format!(
            ".task-panel .filter-tabs button {{ padding: {ACTION_PADDING}; color: alpha({TEXT}, 0.55); }}"
        )));
    }

    /// The tabs are buttons, so the theme's look is reset from them as from
    /// the action row's, hover and press included.
    #[test]
    fn the_tabs_lose_the_themes_button_look() {
        let css = css();
        assert!(css.contains(".task-panel .filter-tabs button {\n"));
        assert!(css.contains(".task-panel .filter-tabs button:hover"));
        assert!(css.contains(".task-panel .filter-tabs button:active"));
        let line = ".task-panel .filter-tabs button:not(:first-child) { border-left: 1px solid rgba(255, 255, 255, 0.12); }";
        let at = css.find(line).expect("missing the line between tabs");
        let reset = css.find(".filter-tabs button:active").unwrap();
        assert!(at > reset, "the press reset's `border: none` would win over the line");
    }

    /// Clear all wears Remove's red, and is filled with it once armed, as the
    /// armed Remove is.
    #[test]
    fn clear_all_is_removes_red() {
        let css = css();
        assert!(css.contains(&format!(".task-panel .filter-tabs .clear-all {{ color: {REMOVE}; }}")));
        assert!(css.contains(&format!(
            ".task-panel .filter-tabs .clear-all.confirm {{ background-color: {REMOVE}; color: {ON_FILL}; }}"
        )));
    }

    /// An up next card's icon and text are Edit's yellow, and the rule comes
    /// after the dimmed one, so an up next blocked card is yellow, lock and
    /// all, rather than dimmed.
    #[test]
    fn an_up_next_card_is_yellow_even_when_blocked() {
        let css = css();
        let rule = format!(".task-panel .task-card.up-next {{ color: {UP_NEXT}; }}");
        let at = css.find(&rule).unwrap_or_else(|| panic!("missing `{rule}`"));
        let dimmed = css.find(".task-panel .task-card.blocked,").expect("missing the dimmed rule");
        assert!(at > dimmed, "the dimmed rule would win over the yellow on a blocked card");
        assert_eq!(UP_NEXT, "#f9e2af");
        assert_eq!(UP_NEXT, EDIT, "Edit's yellow");
    }

    /// A finished card is dimmed, as a waiting one is: done, not to do.
    #[test]
    fn a_finished_card_is_dimmed() {
        assert!(css().contains(
            ".task-panel .task-card.waiting,\n.task-panel .task-card.finished,\n.task-panel .task-card.more"
        ));
    }

    /// The age is a caption to the description, dimmed like the action
    /// row's hint, and parted from the text by a card's padding.
    #[test]
    fn the_age_is_dimmed() {
        assert!(css().contains(&format!(
            ".task-panel .card-age {{ color: alpha({TEXT}, 0.55); padding-left: {PADDING_PX}px; }}"
        )));
    }

    /// A card's notes are dimmed like its age, and stand a card's padding
    /// below the description.
    #[test]
    fn the_notes_are_dimmed_under_the_description() {
        let css = css();
        assert!(css.contains(&format!(".task-panel .card-notes {{ padding-top: {PADDING_PX}px; }}")));
        assert!(css.contains(&format!(".task-panel .card-note {{ color: alpha({TEXT}, 0.55); }}")));
    }

    /// The project list's highlighted folder is darkened as the focused card
    /// is: the text field keeps the focus, so :focus-within cannot do it.
    #[test]
    fn the_picked_folder_is_darkened_like_the_focused_card() {
        assert!(css().contains(".task-panel .task-card:focus-within,\n.task-panel .task-card.picked { background-color: rgba(0, 0, 0, 0.6); }"));
    }

    /// The text field wears the bar's look, not the theme's frame.
    #[test]
    fn the_project_field_has_no_frame_of_its_own() {
        let css = css();
        let rule = css.split(".task-panel .filter-tabs entry.project-query {").nth(1).expect("a rule for the field");
        let rule = rule.split('}').next().unwrap();
        for want in ["background-color: transparent;", "border: none;", "box-shadow: none;", "outline: none;"] {
            assert!(rule.contains(want), "missing `{want}`");
        }
    }

}
