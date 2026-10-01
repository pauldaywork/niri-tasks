# Keyboard Panel: Full Task Text and Action Buttons — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When Mod+Alt+Ctrl+T hands the task panel the keyboard, every card shows its full description, wrapped, above a row of coloured buttons (Start working, Refine, Edit, Stop, Remove). The buttons run the same `niritasks` commands the fuzzel menu runs, from a click or a single key.

**Architecture:** Each card changes from one `gtk::Button` to a vertical `gtk::Box` (still named after its task's uuid) that holds a *body* button, which still opens the fuzzel menu, and, only in keyboard mode, an *action row* of buttons. Which buttons a card gets, what each one spawns, and what each key means are plain data in two new pure modules, `src/panel/actions.rs` and `src/panel/keys.rs`, tested without a compositor, like `model.rs` and `taskbox/keys.rs`. `surface.rs` re-renders when the panel takes or gives back the keyboard, and wraps the column in a `ScrolledWindow` capped at the screen's height. It keeps the focused card in view and clips the blur region to what is on screen.

**Tech Stack:** Rust 2021, gtk4-rs 0.11 (GTK 4.22 on this machine), gtk4-layer-shell 0.8, clap 4, bash and Pillow for the e2e test.

**Spec:** Taskwarrior task `c53b6e3d-ca05-4aae-8588-4ee1abc25f5b`. Read it with `task rc.json.array=on c53b6e3d-ca05-4aae-8588-4ee1abc25f5b export`. Its description and notes are the spec.

## Global Constraints

- Only the keyboard panel expands. Hovering still shows today's one-line cards, and Escape collapses the cards again as it tucks the panel away.
- Every card expands at once to its full description, wrapped. The column's maximum height is the screen height minus margins, and it scrolls beyond that, keeping the focused card in view.
- Buttons, left to right: **Start working**, **Refine**, **Edit**, **Stop** (only when the task is active), **Remove**.
- Start working runs `niritasks task start <uuid>`. It must never only mark the task active.
- Refine runs `niritasks task refine <uuid>`. Edit runs `niritasks task edit <uuid>`. Stop runs `niritasks task status <uuid> stopped`. Each is spawned the way `open_menu` spawns the menu.
- Remove needs a second press. The first press turns the button into **Confirm remove**. The second runs `niritasks task status <uuid> deleted --yes`. Moving away resets it.
- Grill me, Note, Update status (apart from Stop and Remove) and Move to workspace stay in the fuzzel menu only. Enter or a click on the card body still opens the full menu.
- Colours are constants in `src/panel/style.rs`, scoped to `.task-panel`: Start green `#8cd283` (the existing `ACTIVE`), Refine mauve, Edit yellow, Stop peach, Remove red (Catppuccin).
- The button row sits flush along the card's bottom edge and shares its rounded bottom corners. If that can't be done cleanly, a padded row inside the card is fine.
- Keys: Up/Down move between cards. Left/Right or Tab move between the focused card's buttons. `s`, `r`, `e`, `t` and Delete trigger Start, Refine, Edit, Stop and Remove on the focused card. Every action except the first press of Remove gives the keyboard back first.
- Keep the card's uuid as the widget name, so `render()` can put focus back on it.
- Out of scope: changing the fuzzel menu, showing buttons on hover, and adding actions the menu doesn't have.
- Done when: Mod+Alt+Ctrl+T shows every card wrapped, with its coloured buttons and Stop only on active tasks. Start working opens the worktree and Claude exactly as the menu does. Remove deletes only on its second press. Hover and Escape give back today's one-line peek. `cargo test` and `tests/e2e-panel.sh` pass.
- House style: every item gets a doc comment that says *why*, in the voice of the surrounding code (plain sentences, no marketing, no "simply"). Commit messages are a plain-English sentence in the imperative, like `git log` shows, ending with the `Co-Authored-By` line.

## Decisions made while planning (flag any you disagree with)

1. **Catppuccin Mocha for the four new colours:** mauve `#cba6f7`, yellow `#f9e2af`, peach `#fab387`, red `#f38ba8`. `style.rs` already describes `ACTIVE` as nearer mocha's green, and these four are mocha's.
2. **A button shows its colour as text, and as a tinted fill (`alpha(colour, 0.2)`) when focused.** An armed Remove ("Confirm remove") is filled solid red with dark text (mocha `base`, `#1e1e2e`), so the second press is impossible to miss.
3. **The focus border moves from the button to the card.** The 1px white outline goes on `.task-card:focus-within`, so the card the keyboard is on stays outlined whether its body or one of its buttons has focus. The tinted fill shows which button.
4. **Up/Down land on the next card's body, and Left/Right/Tab stop at the ends instead of wrapping.** The slots along a card are its body, then its buttons. Shift+Tab (`ISO_Left_Tab`) goes back. Stopping at the ends keeps Left from jumping somewhere unexpected.
5. **A key for a button the card does not have does nothing.** For example, `t` on a task that isn't active, or any letter on "+N more". A key runs a button by emitting its `clicked`, so a key and a click take exactly one code path.
6. **The "+N more" cap stays in keyboard mode.** The spec doesn't change it, and Enter on "+N more" still shows the rest, which is how the column can grow past the screen and scroll.
7. **Scrolling uses a `gtk::Viewport` made by hand with `scroll-to-focus` off**, plus our own follow-focus. The viewport's built-in follow would scroll only far enough to show the focused *button*, which can leave the rest of the card off screen. Ours scrolls the whole card into view, the way `taskbox.rs`'s `scroll_focus_into_view` does for note rows. It reuses taskbox's `after_next_paint`, made `pub(crate)` instead of copied.
8. **"Screen height minus margins" is `monitor height − 2 × (SHADOW_PX + EDGE_GAP_PX)`.** That is the cards' shadow room plus mako's 8px edge gap, above and below.
9. **A test in `main.rs` parses every button's arguments with the real `Cli`.** A misspelt `stopped` or a dropped `--yes` then fails `cargo test`, not a click. `main.rs` has no test module yet, so this adds one.
10. **The e2e test drives the keyboard panel.** It runs `niritasks task panel` against its sandbox daemon and checks the panel slid out and grew taller. If `wtype` is installed, it then presses Escape and checks the one-line peek came back. Without `wtype` that one check is skipped, and the next section (no tasks pending) gives the keyboard back anyway.
11. **The README's "60px peek" becomes "30px".** That is the same paragraph this plan rewrites, and `PEEK_PX` is 30.
12. **Hand checks run against the worktree's own build, not an install.** Stop the service and run `./target/release/niritasks daemon`. The keybind's `niritasks task panel` reaches whichever daemon is listening, and the buttons spawn `current_exe()`, which is the worktree binary. Installing would put unlanded code on the user's `$PATH`.

## File map

| File | Change |
|---|---|
| `src/panel/actions.rs` | **New.** `Action`: which buttons a card gets, their labels, names and the `niritasks` arguments each spawns. |
| `src/panel/keys.rs` | **New.** `KeyAction` and `key_action()`: what a key means while the panel has the keyboard. `step()` moves along a list and stops at the ends. |
| `src/panel/mod.rs` | Declare the two modules. |
| `src/main.rs` | A test module that parses every button's arguments with `Cli`. |
| `src/panel/style.rs` | Card box, body and action-row CSS, the four new colour constants, and `:focus-within`. |
| `src/panel/blur.rs` | `clip_rows()`: the blur region trimmed to the visible part of a scrolled column. |
| `src/panel/surface.rs` | Card as box + body + action row, wrapping in keyboard mode, re-render on take and give back, Remove's two presses, keys, scrolling, blur and input region kept in step. |
| `src/taskbox.rs` | `after_next_paint` becomes `pub(crate)`. |
| `tests/e2e-panel.sh` | A keyboard section: slid out and taller, then Escape back to the peek when `wtype` is installed. |
| `README.md` | Keybind row, Task panel section, test table, hand checks. |
| `CONTEXT.md` | The Task card entry, plus a new Action row entry. |

---

### Task 1: The panel's actions and keys, as data

**Files:**
- Create: `src/panel/actions.rs`
- Create: `src/panel/keys.rs`
- Modify: `src/panel/mod.rs`
- Modify: `src/main.rs` (append a `#[cfg(test)] mod tests` at the end of the file)

**Interfaces:**
- Consumes: `super::model::Status` (`Active`, `Pending`, `Blocked`, `Planned`, `More`), from `src/panel/model.rs`, unchanged.
- Produces (Tasks 2 to 4 rely on these exact names):
  - `pub enum Action { Start, Refine, Edit, Stop, Remove }`: `Debug, Clone, Copy, PartialEq, Eq`.
  - `pub const Action::ALL: [Action; 5]`, in button order.
  - `pub const Action::CONFIRM_REMOVE: &str = "Confirm remove"`.
  - `pub fn Action::for_status(status: Status) -> Vec<Action>`
  - `pub fn Action::label(self) -> &'static str`
  - `pub fn Action::name(self) -> &'static str`: `"start" | "refine" | "edit" | "stop" | "remove"`. This is both the button's CSS class and its widget name.
  - `pub fn Action::args(self, uuid: &str) -> Vec<String>`: the `niritasks` arguments, without the program name.
  - `pub enum KeyAction { Release, PrevCard, NextCard, PrevSlot, NextSlot, Run(Action), Ignore }`: `Debug, Clone, Copy, PartialEq, Eq`.
  - `pub fn key_action(key: gdk::Key) -> KeyAction`
  - `pub fn step(index: usize, len: usize, forward: bool) -> usize`

- [ ] **Step 1: Write the failing tests for `actions.rs`**

Create `src/panel/actions.rs` with only the tests and a stub that does not compile yet:

```rust
//! The buttons on a task card while the panel has the keyboard, and what each
//! one runs.
//!
//! Plain data, like `model.rs`, so which card gets which buttons and what they
//! spawn is testable without a compositor. Each runs the same `niritasks`
//! command as its entry in the fuzzel menu, so a button cannot drift from the
//! menu entry it stands in for.

use super::model::Status;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_active_task_gets_every_button_in_order() {
        let names: Vec<&str> = Action::for_status(Status::Active).iter().map(|a| a.name()).collect();
        assert_eq!(names, vec!["start", "refine", "edit", "stop", "remove"]);
    }

    #[test]
    fn stop_is_only_on_an_active_task() {
        for status in [Status::Pending, Status::Blocked, Status::Planned] {
            let got = Action::for_status(status);
            assert_eq!(got, vec![Action::Start, Action::Refine, Action::Edit, Action::Remove], "{status:?}");
        }
    }

    #[test]
    fn more_is_no_one_task_and_gets_no_buttons() {
        assert!(Action::for_status(Status::More).is_empty());
    }

    #[test]
    fn labels_read_as_the_menu_does() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label()).collect();
        assert_eq!(labels, vec!["Start working", "Refine", "Edit", "Stop", "Remove"]);
        assert_eq!(Action::CONFIRM_REMOVE, "Confirm remove");
    }

    #[test]
    fn each_button_runs_its_menu_entrys_command() {
        let u = "c53b6e3d-ca05-4aae-8588-4ee1abc25f5b";
        assert_eq!(Action::Start.args(u), vec!["task", "start", u]);
        assert_eq!(Action::Refine.args(u), vec!["task", "refine", u]);
        assert_eq!(Action::Edit.args(u), vec!["task", "edit", u]);
        assert_eq!(Action::Stop.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Action::Remove.args(u), vec!["task", "status", u, "deleted", "--yes"]);
    }

    /// Start working is the menu's: worktree, herdr and Claude. Marking the
    /// task active alone would be `task status <uuid> active`, which is not it.
    #[test]
    fn start_never_only_marks_the_task_active() {
        assert!(!Action::Start.args("x").contains(&"active".to_string()));
    }
}
```

Add `pub mod actions;` and `pub mod keys;` to `src/panel/mod.rs`, keeping alphabetical order:

```rust
pub mod actions;
pub mod blur;
pub mod keys;
pub mod model;
pub mod style;
pub mod surface;
```

Create `src/panel/keys.rs` with its tests:

```rust
//! What a keypress on the task panel means while it has the keyboard.
//!
//! Pure, as the task box's keys are, so the mapping is testable without a
//! window. Acting on it, and the controller's propagation phase that lets it
//! see the arrows first, are `surface.rs`'s.

use super::actions::Action;
use gtk4::gdk;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_gives_the_keyboard_back() {
        assert_eq!(key_action(gdk::Key::Escape), KeyAction::Release);
    }

    #[test]
    fn up_and_down_move_between_cards() {
        assert_eq!(key_action(gdk::Key::Up), KeyAction::PrevCard);
        assert_eq!(key_action(gdk::Key::Down), KeyAction::NextCard);
    }

    #[test]
    fn left_right_and_tab_move_along_the_card() {
        assert_eq!(key_action(gdk::Key::Left), KeyAction::PrevSlot);
        assert_eq!(key_action(gdk::Key::Right), KeyAction::NextSlot);
        assert_eq!(key_action(gdk::Key::Tab), KeyAction::NextSlot);
        // Shift+Tab arrives as ISO_Left_Tab.
        assert_eq!(key_action(gdk::Key::ISO_Left_Tab), KeyAction::PrevSlot);
    }

    #[test]
    fn letters_and_delete_run_their_buttons() {
        assert_eq!(key_action(gdk::Key::s), KeyAction::Run(Action::Start));
        assert_eq!(key_action(gdk::Key::r), KeyAction::Run(Action::Refine));
        assert_eq!(key_action(gdk::Key::e), KeyAction::Run(Action::Edit));
        assert_eq!(key_action(gdk::Key::t), KeyAction::Run(Action::Stop));
        assert_eq!(key_action(gdk::Key::Delete), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::KP_Delete), KeyAction::Run(Action::Remove));
    }

    /// Enter and Space press the focused button, which GTK does itself.
    #[test]
    fn enter_and_space_pass_through() {
        for key in [gdk::Key::Return, gdk::Key::KP_Enter, gdk::Key::space, gdk::Key::x] {
            assert_eq!(key_action(key), KeyAction::Ignore, "{key:?}");
        }
    }

    #[test]
    fn step_moves_one_and_stops_at_the_ends() {
        assert_eq!(step(0, 3, true), 1);
        assert_eq!(step(2, 3, true), 2);
        assert_eq!(step(1, 3, false), 0);
        assert_eq!(step(0, 3, false), 0);
        assert_eq!(step(0, 0, true), 0);
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --lib panel::`
Expected: compile errors, ``cannot find type `Action` `` and ``cannot find function `key_action` ``.

- [ ] **Step 3: Implement `Action`**

Above the test module in `src/panel/actions.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Refine,
    Edit,
    Stop,
    Remove,
}

use Action::*;

impl Action {
    /// In the order the buttons sit, left to right.
    pub const ALL: [Action; 5] = [Start, Refine, Edit, Stop, Remove];

    /// What Remove reads between its first press and its second, the way the
    /// menu's delete asks "delete?" before it deletes.
    pub const CONFIRM_REMOVE: &'static str = "Confirm remove";

    /// The buttons a card gets, left to right. Stop only on an active task,
    /// since there is nothing to stop on the rest; none on "+N more", which
    /// stands for no one task.
    pub fn for_status(status: Status) -> Vec<Action> {
        match status {
            Status::More => Vec::new(),
            Status::Active => Self::ALL.to_vec(),
            Status::Pending | Status::Blocked | Status::Planned => {
                Self::ALL.into_iter().filter(|a| *a != Stop).collect()
            }
        }
    }

    /// The button's text: the menu's own word for it.
    pub fn label(self) -> &'static str {
        match self {
            Start => "Start working",
            Refine => "Refine",
            Edit => "Edit",
            Stop => "Stop",
            Remove => "Remove",
        }
    }

    /// The button's CSS class, which gives it its colour, and its widget name,
    /// which lets a re-render put focus back on the same button.
    pub fn name(self) -> &'static str {
        match self {
            Start => "start",
            Refine => "refine",
            Edit => "edit",
            Stop => "stop",
            Remove => "remove",
        }
    }

    /// The `niritasks` arguments the button runs, without the program: what
    /// its menu entry runs. Remove carries `--yes` because its second press is
    /// the confirmation.
    pub fn args(self, uuid: &str) -> Vec<String> {
        let words: &[&str] = match self {
            Start => &["task", "start", uuid],
            Refine => &["task", "refine", uuid],
            Edit => &["task", "edit", uuid],
            Stop => &["task", "status", uuid, "stopped"],
            Remove => &["task", "status", uuid, "deleted", "--yes"],
        };
        words.iter().map(|w| w.to_string()).collect()
    }
}
```

- [ ] **Step 4: Implement `KeyAction`, `key_action` and `step`**

Above the test module in `src/panel/keys.rs`:

```rust
/// What a keypress on the panel should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Give the keyboard back and tuck the panel away.
    Release,
    PrevCard,
    NextCard,
    /// Along the focused card: its body, then its buttons.
    PrevSlot,
    NextSlot,
    /// Press this button on the focused card, if it has one.
    Run(Action),
    /// Pass it through: Enter and Space press the focused button.
    Ignore,
}

/// Map a keypress to what it should do. The letters work without a modifier,
/// because nothing on the panel takes typing.
pub fn key_action(key: gdk::Key) -> KeyAction {
    match key {
        gdk::Key::Escape => KeyAction::Release,
        gdk::Key::Up => KeyAction::PrevCard,
        gdk::Key::Down => KeyAction::NextCard,
        gdk::Key::Left | gdk::Key::ISO_Left_Tab => KeyAction::PrevSlot,
        gdk::Key::Right | gdk::Key::Tab => KeyAction::NextSlot,
        gdk::Key::s => KeyAction::Run(Action::Start),
        gdk::Key::r => KeyAction::Run(Action::Refine),
        gdk::Key::e => KeyAction::Run(Action::Edit),
        gdk::Key::t => KeyAction::Run(Action::Stop),
        gdk::Key::Delete | gdk::Key::KP_Delete => KeyAction::Run(Action::Remove),
        _ => KeyAction::Ignore,
    }
}

/// One along a list of `len`, stopping at either end rather than wrapping: a
/// key held down settles on the last card instead of cycling past it.
pub fn step(index: usize, len: usize, forward: bool) -> usize {
    if forward {
        (index + 1).min(len.saturating_sub(1))
    } else {
        index.saturating_sub(1)
    }
}
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --lib panel::`
Expected: PASS, including every existing `panel::model` and `panel::blur` test.

- [ ] **Step 6: Write the CLI round-trip test in `main.rs`**

Append to the end of `src/main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use niri_tasks::panel::actions::Action;

    /// Every button on a task card spawns `niritasks` with these arguments, so
    /// each has to be a command the real CLI accepts. Otherwise a typo shows
    /// up as a click that does nothing.
    #[test]
    fn every_card_button_is_a_command_the_cli_accepts() {
        for action in Action::ALL {
            let mut argv = vec!["niritasks".to_string()];
            argv.extend(action.args("c53b6e3d"));
            if let Err(e) = Cli::try_parse_from(&argv) {
                panic!("{} runs {argv:?}, which the CLI rejects: {e}", action.label());
            }
        }
    }
}
```

- [ ] **Step 7: Run it**

Run: `cargo test --bin niritasks`
Expected: PASS, `every_card_button_is_a_command_the_cli_accepts`. To prove it bites, temporarily change `"stopped"` to `"stoped"` in `Action::args` and run it again: it should FAIL naming Stop. Then put `"stopped"` back.

- [ ] **Step 8: Run the full suite, then commit**

Run: `cargo test`
Expected: PASS.

```bash
git add src/panel/actions.rs src/panel/keys.rs src/panel/mod.rs src/main.rs
git commit -m "Name the task panel's card buttons and keys, and what each one runs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The card's box, body and coloured action row in the stylesheet

**Files:**
- Modify: `src/panel/style.rs` (module doc, constants, `css()`, tests)

**Interfaces:**
- Consumes: `crate::panel::actions::Action::{ALL, name}` from Task 1, in the tests only.
- Produces: CSS classes that Task 3 puts on widgets:
  - `.task-card` is now the card's `gtk::Box`. It keeps the status classes `.active`, `.blocked`, `.planned` and `.more`.
  - `.card-body` is the body button inside it.
  - `.card-actions` is the action row (a horizontal `gtk::Box`). Its buttons carry `Action::name()` as a class.
  - `.confirm` goes on an armed Remove button.
  - Constants `REFINE`, `EDIT`, `STOP`, `REMOVE`, `ON_FILL`, and `ACTION_COLOURS: [(&str, &str); 5]`.

- [ ] **Step 1: Write the failing tests**

In `src/panel/style.rs`'s test module, change the focus line in `cards_carry_makos_shape_and_the_terminals_font_and_colours` from
`".task-card:focus { outline: 1px solid #ffffff; outline-offset: -1px; }",`
to
`".task-card:focus-within { outline: 1px solid #ffffff; outline-offset: -1px; }",`
and add two tests:

```rust
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

    /// The row shares the card's rounded bottom corners.
    #[test]
    fn the_action_row_rounds_off_with_the_card() {
        let css = css();
        assert!(css.contains(".card-actions button:first-child { border-bottom-left-radius: 8px; }"));
        assert!(css.contains(".card-actions button:last-child { border-bottom-right-radius: 8px; }"));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib panel::style`
Expected: FAIL. `ACTION_COLOURS` is not found (a compile error).

- [ ] **Step 3: Add the constants**

After `ACTIVE` in `src/panel/style.rs`:

```rust
/// The keyboard's action buttons, each its own colour from the same Catppuccin
/// palette as [`ACTIVE`] (mocha's): Refine mauve, Edit yellow, Stop peach,
/// Remove red. Start shares [`ACTIVE`]'s green, because starting a task is what
/// turns its card green.
pub const REFINE: &str = "#cba6f7";
pub const EDIT: &str = "#f9e2af";
pub const STOP: &str = "#fab387";
pub const REMOVE: &str = "#f38ba8";
/// Text on a solid colour fill, the armed Remove: mocha's `base`, so it reads
/// as dark on red.
pub const ON_FILL: &str = "#1e1e2e";
/// Each button's class, as `Action::name()` gives it, and its colour.
pub const ACTION_COLOURS: [(&str, &str); 5] = [
    ("start", ACTIVE),
    ("refine", REFINE),
    ("edit", EDIT),
    ("stop", STOP),
    ("remove", REMOVE),
];
```

- [ ] **Step 4: Rewrite `css()`**

Replace the body of `css()` with:

```rust
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
.task-panel .card-body:hover,
.task-panel .card-body:active,
.task-panel .card-actions button,
.task-panel .card-actions button:hover,
.task-panel .card-actions button:active {{
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
/* The body keeps the card's whole rounding while it is the only thing in it. */
.task-panel .card-body {{ border-radius: {RADIUS_PX}px; }}
.task-panel .card-actions button {{ padding: {ACTION_PADDING}; }}
.task-panel .card-actions button:first-child {{ border-bottom-left-radius: {RADIUS_PX}px; }}
.task-panel .card-actions button:last-child {{ border-bottom-right-radius: {RADIUS_PX}px; }}
/* :focus-within, not :focus-visible: GTK clears focus-visible 3s after the
   last key press (VISIBLE_FOCUS_DURATION), and the border would vanish
   mid-pick. Within, so the card stays outlined while one of its buttons is
   focused. */
.task-panel .task-card:focus-within {{ outline: {FOCUS}; outline-offset: -1px; }}
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
```

Add this constant next to `PADDING_PX`:

```rust
/// The action row's buttons: half a card's padding top and bottom, so the row
/// reads as a footer to the description rather than a second card.
pub const ACTION_PADDING: &str = "6px 12px";
```

Then rewrite the module doc's paragraphs about the border and buttons, at the top of `style.rs`, so they say:

```rust
//! No card has a border, except the one the keyboard is on, which gets a 1px
//! white one. The active task is marked by colour alone: its ▶ and its text
//! are green.
//!
//! A card is a box holding a body button and, while the panel has the
//! keyboard, a row of action buttons along its bottom edge. The theme's button
//! look (border, gradient, minimum height, hover and press colours) is reset
//! away from both, so the body looks the same as a plain label would, and the
//! card's box carries the fill, the rounding and the outline.
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --lib panel::style`
Expected: PASS, all four tests. `every_rule_is_scoped_to_the_panel` checks that every line with a `{` contains `task-panel`. The `push_str` lines put the selector and the `{` on the same line, so they satisfy it.

- [ ] **Step 6: Commit**

The panel won't build a matching widget tree until Task 3. Until then a running build shows unstyled cards, so don't install between Task 2 and Task 3.

```bash
git add src/panel/style.rs
git commit -m "Style a task card as a box with a body and a coloured row of action buttons

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Wrapped cards with action buttons while the panel has the keyboard

**Files:**
- Modify: `src/panel/surface.rs` (module doc, `Panel` struct, `new`, `take_keyboard`, `release_keyboard`, `expand`, `render`, `card_button` → `card_widget`, new `press`/`disarm_unless_focused`/`card_of`/`focus_card`, free fn `slots`, `card_label`, `open_menu` doc)
- Modify: `tests/e2e-panel.sh` (a keyboard section)

**Interfaces:**
- Consumes: `Action::{for_status, label, name, args, CONFIRM_REMOVE}` (Task 1). CSS classes `task-card`, `card-body`, `card-actions`, `confirm` and `Action::name()` (Task 2).
- Produces (Task 4 and Task 5 rely on these):
  - `fn card_of(&self, widget: &gtk4::Widget) -> Option<gtk4::Widget>`: the column child (the card) holding `widget`.
  - `fn slots(card: &gtk4::Widget) -> Vec<gtk4::Widget>`: a free function returning the card's body, then its action buttons.
  - `fn focus_card(card: &gtk4::Widget, slot: Option<&str>)`: focuses the slot with that widget name, or the body.
  - `fn disarm_unless_focused(&self)`, called from the window's `focus-widget` notify.
  - `Panel.armed: RefCell<Option<gtk4::Button>>`.
  - The body button is named `"body"`. Action buttons are named `Action::name()`.

- [ ] **Step 1: Add the failing e2e check**

In `tests/e2e-panel.sh`, add a new section between `# ─── a task on another tag stays off this panel` and `# ─── nothing pending shows nothing`:

```bash
# ─── the keyboard: every card out, wrapped, with its buttons ─────────────────
# `task panel` is the keybind's command: it asks this sandbox's daemon to hand
# its panel the keyboard. The cards slide all the way out, so the measured
# strip is as wide as it can be, and each grows an action row, so the three of
# them stand taller than their one-line peek.
"$NIRITASKS" task panel >/dev/null 2>&1
settle
shot keyboard || exit 1
read -r key_w key_h < <(measure keyboard)
if [ "$key_w" -ge 250 ]; then
    ok "the keyboard slides the panel out (${key_w}px of the 300px strip)"
else
    bad "the panel is ${key_w}px wide with the keyboard, expected it slid out"
fi
if [ "$key_h" -ge $((three_h + 40)) ]; then
    ok "and each card grows its buttons (${key_h}px tall vs ${three_h}px)"
else
    bad "the keyboard's cards are ${key_h}px tall against ${three_h}px tucked away —
      the action rows are missing"
fi

# Escape needs a key pressed on the panel, which only wtype can do here. The
# next section gives the keyboard back regardless: a panel with no tasks lets
# it go.
if command -v wtype >/dev/null; then
    wtype -k Escape
    settle
    shot released || exit 1
    read -r rel_w rel_h < <(measure released)
    delta=$(( rel_h > three_h ? rel_h - three_h : three_h - rel_h ))
    if [ "$rel_w" -le $((PEEK + 30)) ] && [ "$delta" -le 6 ]; then
        ok "Escape tucks it back to the one-line peek (${rel_w}x${rel_h}px)"
    else
        bad "after Escape the panel is ${rel_w}x${rel_h}px, expected the
      ${three_w}x${three_h}px peek it started from"
    fi
else
    echo "  SKIP  Escape back to the peek (needs wtype)"
fi
```

In the header comment at the top of the script, replace the paragraph that starts `# What it cannot check is the hover:` with:

```bash
# What it cannot check is the hover: nothing on this machine can move the
# pointer, so the slide out, the slide back, and clicks passing beside the peek
# are checked by hand (README, "Testing"). The keyboard it can: `task panel`
# slides the panel out, and with wtype installed, Escape tucks it away again.
```

- [ ] **Step 2: Run the e2e check against the current code to see it fail**

Read the "Park the pointer away from the right edge" note at the top of `tests/e2e-panel.sh` first, and don't touch the machine while the script runs.

Run: `cargo build --release && NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh`
Expected: the slid-out check PASSes. `FAIL  the keyboard's cards are …px tall … the action rows are missing`, because today's keyboard panel only slides the one-line cards out. Every other check PASSes.

If the script stops at "the right edge is not static", that's the environment and not the code. Clear the right edge and run it again.

- [ ] **Step 3: Give `Panel` the armed Remove**

In the `Panel` struct, add after `keyboard`:

```rust
    /// The Remove button pressed once and waiting for its second press, which
    /// deletes. Moving the focus off it, or any re-render, puts it back.
    armed: RefCell<Option<gtk4::Button>>,
```

and in `Panel::new`'s struct literal, after `keyboard: Cell::new(false),`:

```rust
            armed: RefCell::new(None),
```

Add `use super::actions::Action;` to the imports.

- [ ] **Step 4: Replace `card_button` with `card_widget`, and add the helpers**

Delete `fn card_button` and add in its place, inside `impl Panel`:

```rust
    /// One card: a box named after its task's uuid, so render() can put focus
    /// back on it, holding the body, a button so the keyboard can focus and
    /// press it, and, while the panel has the keyboard, its action row along
    /// the bottom. The body opens the task's whole menu, or shows the rest in
    /// place of "+N more".
    fn card_widget(self: &Rc<Self>, card: &Card) -> gtk4::Box {
        let keyboard = self.keyboard.get();
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        widget.add_css_class("task-card");
        match card.status {
            Status::Active => widget.add_css_class("active"),
            Status::Blocked => widget.add_css_class("blocked"),
            Status::Planned => widget.add_css_class("planned"),
            Status::More => widget.add_css_class("more"),
            Status::Pending => {}
        }
        widget.set_size_request(CARD_WIDTH_PX, -1);
        // Clips the action row to the card's rounded bottom corners.
        widget.set_overflow(gtk4::Overflow::Hidden);
        widget.set_widget_name(card.uuid.as_deref().unwrap_or("more"));

        let body = gtk4::Button::builder().child(&card_label(card, keyboard)).build();
        body.add_css_class("card-body");
        body.set_widget_name("body");
        // A mouse click opens the menu without leaving the card bordered.
        body.set_focus_on_click(false);
        {
            let weak = Rc::downgrade(self);
            let uuid = card.uuid.clone();
            body.connect_clicked(move |_| {
                let Some(p) = weak.upgrade() else { return };
                match &uuid {
                    Some(uuid) => {
                        p.release_keyboard();
                        open_menu(&p.output, &["task".into(), "menu".into(), uuid.clone()]);
                    }
                    None => p.expand(),
                }
            });
        }
        widget.append(&body);

        let Some(uuid) = card.uuid.as_ref().filter(|_| keyboard) else {
            return widget;
        };
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        row.add_css_class("card-actions");
        row.set_homogeneous(true);
        for action in Action::for_status(card.status) {
            let button = gtk4::Button::with_label(action.label());
            button.add_css_class(action.name());
            button.set_widget_name(action.name());
            let weak = Rc::downgrade(self);
            let uuid = uuid.clone();
            button.connect_clicked(move |b| {
                if let Some(p) = weak.upgrade() {
                    p.press(action, &uuid, b);
                }
            });
            row.append(&button);
        }
        widget.append(&row);
        widget
    }

    /// Press one of a card's buttons. Remove only arms itself the first time,
    /// as the menu's delete asks "delete?" first; the second press runs it.
    /// Everything that runs gives the keyboard back first, so the box or
    /// terminal it opens can take it.
    fn press(self: &Rc<Self>, action: Action, uuid: &str, button: &gtk4::Button) {
        if action == Action::Remove && self.armed.borrow().as_ref() != Some(button) {
            // Focus first: the move disarms whatever was armed before, and
            // this one is armed only after it.
            button.grab_focus();
            button.set_label(Action::CONFIRM_REMOVE);
            button.add_css_class("confirm");
            *self.armed.borrow_mut() = Some(button.clone());
            return;
        }
        self.release_keyboard();
        open_menu(&self.output, &action.args(uuid));
    }

    /// Moving the focus away from an armed Remove puts it back to Remove, so a
    /// later press starts over at the first press.
    fn disarm_unless_focused(&self) {
        let focus = GtkWindowExt::focus(&self.window);
        let Some(button) = self.armed.take() else { return };
        if focus.as_ref() == Some(button.upcast_ref()) {
            *self.armed.borrow_mut() = Some(button);
            return;
        }
        button.set_label(Action::Remove.label());
        button.remove_css_class("confirm");
    }

    /// The card a widget is in: the one of the column's children it sits
    /// inside.
    fn card_of(&self, widget: &gtk4::Widget) -> Option<gtk4::Widget> {
        let column: &gtk4::Widget = self.column.upcast_ref();
        let mut w = widget.clone();
        loop {
            let parent = w.parent()?;
            if &parent == column {
                return Some(w);
            }
            w = parent;
        }
    }
```

Add these free functions next to `card_label`:

```rust
/// What the keyboard can stop on in a card, left to right: its body, then its
/// action buttons.
fn slots(card: &gtk4::Widget) -> Vec<gtk4::Widget> {
    let mut slots = Vec::new();
    let Some(body) = card.first_child() else { return slots };
    if let Some(row) = body.next_sibling() {
        let mut child = row.first_child();
        while let Some(button) = child {
            child = button.next_sibling();
            slots.push(button);
        }
    }
    slots.insert(0, body);
    slots
}

/// Focus the slot in `card` with this widget name, or its body when it has
/// none: a re-render that dropped the button, such as Stop on a task that
/// just stopped.
fn focus_card(card: &gtk4::Widget, slot: Option<&str>) {
    let slots = slots(card);
    let target = slot
        .and_then(|name| slots.iter().find(|s| s.widget_name() == name))
        .or_else(|| slots.first());
    if let Some(target) = target {
        target.grab_focus();
    }
}
```

- [ ] **Step 5: Wrap the body in keyboard mode**

Replace `card_label` with:

```rust
/// The card's icon and description: one line cut off with "…" for the peek
/// and the hover, or all of it, wrapped, while the panel has the keyboard.
fn card_label(card: &Card, wrap: bool) -> gtk4::Label {
    let text = glib::markup_escape_text(&card.text);
    let markup = match card.icon() {
        "" => text.to_string(),
        icon => format!("{icon}  {text}"),
    };
    let label = gtk4::Label::new(None);
    label.set_markup(&markup);
    label.set_xalign(0.0);
    if wrap {
        label.set_wrap(true);
        // WordChar: a long path or URL with no spaces still breaks.
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    } else {
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        label.set_single_line_mode(true);
    }
    label
}
```

- [ ] **Step 6: Re-render on taking and giving back the keyboard, and restore focus by card and slot**

Replace `take_keyboard` and `release_keyboard` with:

```rust
    /// Slide out and take the keyboard, every card wrapped with its buttons,
    /// focusing the first. False when there are no cards to take it for.
    pub fn take_keyboard(self: &Rc<Self>) -> bool {
        if self.all.borrow().is_empty() {
            return false;
        }
        self.keyboard.set(true);
        self.cancel_grace();
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.render();
        self.slide_to(EXPANDED_X);
        if let Some(first) = self.column.first_child() {
            focus_card(&first, None);
        }
        true
    }

    /// Give the keyboard back, folding every card to its one line again as
    /// the panel tucks away.
    fn release_keyboard(self: &Rc<Self>) {
        if !self.keyboard.replace(false) {
            return;
        }
        self.window.set_keyboard_mode(KeyboardMode::None);
        self.expanded.set(false);
        self.render();
        self.slide_to(TUCKED_X);
    }
```

In `expand`, change `c.grab_focus();` to `focus_card(&c, None);`.

In `render`, replace the line computing `focused` and the loop that rebuilds the column with:

```rust
        // A refresh while the keyboard is on the panel keeps focus on the same
        // task, and on the same button of it: each card is named after its
        // task's uuid, and each button after its action.
        let focused = GtkWindowExt::focus(&self.window)
            .and_then(|w| Some((self.card_of(&w)?.widget_name(), w.widget_name())));
        // The buttons go with the widgets they were armed on.
        self.armed.replace(None);
        while let Some(child) = self.column.first_child() {
            self.column.remove(&child);
        }
        for card in &cards {
            self.column.append(&self.card_widget(card));
        }
```

Replace the `if self.keyboard.get() { … }` block at the end of `render` with:

```rust
        if self.keyboard.get() {
            let mut child = self.column.first_child();
            while let Some(c) = &child {
                if Some(c.widget_name()) == focused.as_ref().map(|(card, _)| card.clone()) {
                    break;
                }
                child = c.next_sibling();
            }
            if let Some(c) = child.or_else(|| self.column.first_child()) {
                focus_card(&c, focused.as_ref().map(|(_, slot)| slot.as_str()));
            }
        }
```

`GtkWindowExt::focus` needs `use gtk4::prelude::*`, which is already imported. The old call spelled it `gtk4::prelude::GtkWindowExt::focus`. Either form works, but use the short one consistently.

- [ ] **Step 7: Disarm Remove when the focus moves**

In `Panel::new`, after `panel.window.add_controller(keys);`, add:

```rust
        // Moving off an armed Remove disarms it.
        {
            let weak = Rc::downgrade(&panel);
            panel.window.connect_notify_local(Some("focus-widget"), move |_, _| {
                if let Some(p) = weak.upgrade() {
                    p.disarm_unless_focused();
                }
            });
        }
```

- [ ] **Step 8: Update the module doc and `open_menu`'s doc**

Replace the `## The keyboard` section of the module doc at the top of `surface.rs` with:

```rust
//! ## The keyboard
//!
//! Mod+Alt+Ctrl+T slides the panel out and hands it the keyboard, and every
//! card lays itself out again: its whole description, wrapped, above a row of
//! buttons for the menu's most-used actions. A card is a box holding a body
//! button and that row, all buttons, so GTK focuses and presses them: Enter
//! clicks the focused one, and a click and Enter are the same `clicked`. The
//! body opens the whole menu. Escape, or anything that runs, hands the
//! keyboard back and folds the cards to one line again.
//!
//! It is held with exclusive keyboard mode throughout, as fuzzel does. niri
//! gives an on-demand layer surface focus only after a click on it, so
//! switching to on-demand once focused would drop the focus at once.
```

Change `open_menu`'s doc's first paragraph to:

```rust
/// Run a `niritasks` command on this monitor: a task's action menu, the
/// fuzzel list, or one of a card's buttons.
```

- [ ] **Step 9: Build and run the unit tests**

Run: `cargo build --release && cargo test`
Expected: builds with no warnings, and the tests PASS.

Fix any `unused` warning, for example a leftover import from `card_button`.

- [ ] **Step 10: Run the e2e test to see it pass**

Run: `NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh`
Expected: every check PASSes, including `each card grows its buttons`. With `wtype` installed, `Escape tucks it back to the one-line peek` also PASSes.

If the Escape check fails because the panel is still out, see whether `wtype` actually reached the panel. Give the key a moment after `settle`, as `tests/e2e-box.sh` does for a new box's keymap: add a `sleep 0.3` before `wtype -k Escape`. Then run it again.

- [ ] **Step 11: Commit**

```bash
git add src/panel/surface.rs tests/e2e-panel.sh
git commit -m "Wrap every task card and add its action buttons while the panel has the keyboard

Start working, Refine, Edit, Stop (on an active task) and Remove run the
menu's own commands, giving the keyboard back first. Remove arms on its
first press and deletes on its second; moving away disarms it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Keys: Up/Down between cards, Left/Right/Tab along one, letters for the buttons

**Files:**
- Modify: `src/panel/surface.rs` (the key controller in `new`, a new `Panel::key` method, the module doc's keyboard section)

**Interfaces:**
- Consumes: `keys::{key_action, KeyAction, step}` (Task 1). `card_of`, `slots`, `focus_card` and `press` via `emit_clicked` (Task 3).
- Produces: `fn key(self: &Rc<Self>, action: KeyAction)`, private to `surface.rs`.

- [ ] **Step 1: Replace the key controller**

In `Panel::new`, replace the whole `let keys = gtk4::EventControllerKey::new(); { … } panel.window.add_controller(keys);` block with:

```rust
        let key_controller = gtk4::EventControllerKey::new();
        // Capture, so the arrows and Tab reach this before the window's own
        // focus chain, which would walk every button on the panel in turn
        // rather than between cards, or along one.
        key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
        {
            let weak = Rc::downgrade(&panel);
            key_controller.connect_key_pressed(move |_, key, _, _| {
                let Some(p) = weak.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                match keys::key_action(key) {
                    KeyAction::Ignore => glib::Propagation::Proceed,
                    action => {
                        p.key(action);
                        glib::Propagation::Stop
                    }
                }
            });
        }
        panel.window.add_controller(key_controller);
```

Add to the imports:

```rust
use super::keys::{self, KeyAction};
```

The controller is named `key_controller`, not `keys` as before, so that `keys::key_action` names the module. Task 3, Step 7 put its `focus-widget` handler after `panel.window.add_controller(keys);`; it now follows `add_controller(key_controller)`.

- [ ] **Step 2: Add `Panel::key`**

Inside `impl Panel`, after `press`:

```rust
    /// Act on a key while the panel has the keyboard. Up and Down land on the
    /// next card's body, Left, Right and Tab move along the focused card, and
    /// a letter presses that card's button. If the card has no such button
    /// (Stop on a task that is not active, anything on "+N more"), nothing
    /// happens.
    fn key(self: &Rc<Self>, action: KeyAction) {
        if action == KeyAction::Release {
            self.release_keyboard();
            return;
        }
        let mut cards = Vec::new();
        let mut child = self.column.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            cards.push(c);
        }
        let focus = GtkWindowExt::focus(&self.window);
        let Some(card) = focus
            .as_ref()
            .and_then(|f| self.card_of(f))
            .or_else(|| cards.first().cloned())
        else {
            return;
        };
        let at = cards.iter().position(|c| *c == card).unwrap_or(0);
        let slots = slots(&card);
        let slot = focus
            .as_ref()
            .and_then(|f| slots.iter().position(|s| s == f))
            .unwrap_or(0);

        match action {
            KeyAction::PrevCard | KeyAction::NextCard => {
                let to = keys::step(at, cards.len(), action == KeyAction::NextCard);
                focus_card(&cards[to], None);
            }
            KeyAction::PrevSlot | KeyAction::NextSlot => {
                let to = keys::step(slot, slots.len(), action == KeyAction::NextSlot);
                slots[to].grab_focus();
            }
            KeyAction::Run(run) => {
                // Through the button, so a key does exactly what a click does.
                if let Some(button) = slots
                    .iter()
                    .find(|s| s.widget_name() == run.name())
                    .and_then(|s| s.downcast_ref::<gtk4::Button>())
                {
                    button.emit_clicked();
                }
            }
            KeyAction::Release | KeyAction::Ignore => {}
        }
    }
```

- [ ] **Step 3: Update the module doc**

In the `## The keyboard` section from Task 3, replace the sentence `A card is a box holding a body button and that row, all buttons, so GTK focuses and presses them: Enter clicks the focused one, and a click and Enter are the same `clicked`.` with:

```rust
//! A card is a box holding a body button and that row. Up and Down move
//! between cards, Left, Right and Tab along the focused one, and s, r, e, t
//! and Delete press its Start, Refine, Edit, Stop and Remove by emitting the
//! button's `clicked`, so a key, Enter on the focused button, and a click all
//! take one path. The controller runs in the capture phase, ahead of GTK's
//! own focus chain, which would otherwise walk every button on the panel.
```

- [ ] **Step 4: Build, test, and rerun the e2e test**

Run: `cargo build --release && cargo test && NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh`
Expected: no warnings, and every check PASSes. The Escape check now goes through `KeyAction::Release`.

- [ ] **Step 5: Check the keys by hand on the worktree's daemon**

Ask the human partner to do this, because it needs real keypresses on a real panel (Decision 12):

```bash
systemctl --user stop niri-tasks
./target/release/niritasks daemon     # leave running; Ctrl+C when done
# afterwards:
systemctl --user start niri-tasks
```

On a workspace with a few throwaway tasks, one of them active (`niritasks task status <uuid> active`), press Mod+Alt+Ctrl+T:
- Every card is wrapped and coloured. Stop appears only on the active card.
- Down and Up move the white outline card to card, landing on the body.
- Right and Tab walk the buttons, Left and Shift+Tab walk back, and both stop at the ends.
- `t` on a task that isn't active does nothing. `t` on the active one stops it.
- Delete turns Remove into a red "Confirm remove". Right or Up puts it back to "Remove". Delete, Delete removes the task.
- `e` opens the task box. `r` opens Refine. `s` opens the worktree and Claude, as the menu's Start working does.
- Escape folds the cards to one line and tucks them away.

- [ ] **Step 6: Commit**

```bash
git add src/panel/surface.rs
git commit -m "Move between task cards with Up and Down, along one with Left, Right and Tab, and press its buttons with s, r, e, t and Delete

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Cap the column at the screen's height and scroll, keeping the focused card in view

**Files:**
- Modify: `src/panel/blur.rs` (`clip_rows` and its tests)
- Modify: `src/panel/surface.rs` (`Panel` gains `scroller` and `monitor`, a viewport in `new`, `render` heights, `update_blur`, a `follow_focus` method, pure `shown_height` and `scroll_to_show` with a new `#[cfg(test)]` module, doc on `Slide::cards_h`)
- Modify: `src/taskbox.rs:523` (`fn after_next_paint` → `pub(crate) fn after_next_paint`)

**Interfaces:**
- Consumes: `card_of` and `disarm_unless_focused` (Task 3), and `crate::taskbox::after_next_paint(window: &ApplicationWindow, f: impl FnOnce() + 'static)`.
- Produces:
  - `pub fn blur::clip_rows(rects: &[Rect], top: i32, bottom: i32) -> Vec<Rect>`
  - `fn shown_height(cards_h: i32, screen_h: i32) -> i32` in `surface.rs`.
  - `fn scroll_to_show(value: f64, page: f64, top: f64, bottom: f64) -> f64` in `surface.rs`.

- [ ] **Step 1: Write the failing tests**

Add to `src/panel/blur.rs`'s test module:

```rust
    #[test]
    fn clipping_keeps_what_is_inside_whole() {
        assert_eq!(clip_rows(&[(0, 20, 10, 5)], 16, 100), vec![(0, 20, 10, 5)]);
    }

    #[test]
    fn clipping_trims_a_strip_across_either_edge() {
        assert_eq!(clip_rows(&[(0, 10, 10, 20)], 16, 100), vec![(0, 16, 10, 14)]);
        assert_eq!(clip_rows(&[(0, 90, 10, 20)], 16, 100), vec![(0, 90, 10, 10)]);
    }

    #[test]
    fn clipping_drops_what_is_scrolled_out_of_view() {
        assert!(clip_rows(&[(0, 0, 10, 16), (0, 100, 10, 5)], 16, 100).is_empty());
    }
```

At the end of `src/panel/surface.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_column_shows_whole() {
        assert_eq!(shown_height(400, 1080), 400);
    }

    #[test]
    fn a_tall_column_stops_at_the_screens_margins() {
        // 1080 less the shadow room and the edge gap, top and bottom.
        assert_eq!(shown_height(5000, 1080), 1080 - 2 * (SHADOW_PX + EDGE_GAP_PX));
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
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib panel::`
Expected: compile errors, ``cannot find function `clip_rows` ``, `shown_height` and `scroll_to_show`.

- [ ] **Step 3: Implement the three pure functions**

In `src/panel/blur.rs`, after `card_region`:

```rust
/// Only the parts of `rects` between rows `top` and `bottom`. A scrolled
/// column has cards above and below the view, and there is nothing on screen
/// to blur behind those.
pub fn clip_rows(rects: &[Rect], top: i32, bottom: i32) -> Vec<Rect> {
    rects
        .iter()
        .filter_map(|&(x, y, w, h)| {
            let (from, to) = (y.max(top), (y + h).min(bottom));
            (to > from).then_some((x, from, w, to - from))
        })
        .collect()
}
```

In `src/panel/surface.rs`, after `open_menu`:

```rust
/// How tall the column of cards is on screen: all of it, or as much as fits
/// inside the screen's margins, with the rest scrolled. Only the keyboard's
/// wrapped cards, or "+N more" opened onto a long list, get that tall.
fn shown_height(cards_h: i32, screen_h: i32) -> i32 {
    cards_h.min(screen_h - 2 * (SHADOW_PX + EDGE_GAP_PX)).max(0)
}

/// The scroll position that shows all of `top..bottom`, moving as little as it
/// can from `value` with `page` of the column in view. A card taller than the
/// page shows its top.
fn scroll_to_show(value: f64, page: f64, top: f64, bottom: f64) -> f64 {
    if top < value || bottom - top > page {
        top
    } else if bottom > value + page {
        bottom - page
    } else {
        value
    }
}
```

- [ ] **Step 4: Run them to see them pass**

Run: `cargo test --lib panel::`
Expected: PASS. Fix any `dead_code` warnings on `shown_height` and `scroll_to_show` in the next step, where they get used.

- [ ] **Step 5: Put the column in a scroller**

Change `fn after_next_paint` at `src/taskbox.rs:523` to `pub(crate) fn after_next_paint`. Leave its body and doc as they are.

In the `Panel` struct, after `base`:

```rust
    /// Scrolls the column once it is taller than the screen.
    scroller: gtk4::ScrolledWindow,
    /// For its height, which caps the column's: read at each render, since a
    /// scale change alters it.
    monitor: gdk::Monitor,
```

Change `Slide::cards_h`'s doc to:

```rust
    /// The cards' height on screen: all of them, or the scroller's when they
    /// run past the screen. The input region needs it.
```

In `Panel::new`, replace

```rust
        overlay.add_overlay(&column);
```

with

```rust
        // A viewport made by hand, to turn off its own scroll-to-focus. That
        // would show only the focused button, leaving the rest of its card
        // off screen; follow_focus scrolls the whole card into view instead.
        let viewport = gtk4::Viewport::new(None::<&gtk4::Adjustment>, None::<&gtk4::Adjustment>);
        viewport.set_scroll_to_focus(false);
        viewport.set_child(Some(&column));
        let scroller = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&viewport)
            .build();
        overlay.add_overlay(&scroller);
```

and in the struct literal add `scroller,` after `base,`, and `monitor: monitor.clone(),` after `output: …`.

After the struct literal (next to the other `connect_*` blocks), keep the blur in step with the scroll:

```rust
        // The blur region is in surface coordinates, so it moves with the scroll.
        {
            let weak = Rc::downgrade(&panel);
            panel.scroller.vadjustment().connect_value_changed(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.update_blur(p.slide.x.get());
                }
            });
        }
```

Extend the `focus-widget` handler from Task 3 so it also follows the focus:

```rust
                if let Some(p) = weak.upgrade() {
                    p.disarm_unless_focused();
                    p.follow_focus();
                }
```

- [ ] **Step 6: Size the surface by what is shown**

In `render`, replace

```rust
        let (_, cards_h, _, _) = self.column.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX);
        self.slide.cards_h.set(cards_h);
        let height = cards_h + 2 * SHADOW_PX;
```

with

```rust
        let (_, cards_h, _, _) = self.column.measure(gtk4::Orientation::Vertical, CARD_WIDTH_PX);
        let shown = shown_height(cards_h, self.monitor.geometry().height());
        self.slide.cards_h.set(shown);
        let height = shown + 2 * SHADOW_PX;
```

The overlay's `connect_get_child_position` and `set_region` already read `slide.cards_h`. That's now the height on screen, so the scroller is allocated that tall, and the input region covers only the visible column. Neither needs changing.

- [ ] **Step 7: Clip the blur to the view, offset by the scroll**

In `update_blur`, replace

```rust
        let mut y = SHADOW_PX;
        let mut rects = Vec::new();
        for &h in self.heights.borrow().iter() {
            rects.extend(blur::card_region((x, y, width, h), RADIUS_PX, on_screen));
            y += h + GAP_PX;
        }
        blur.set(&rects);
```

with

```rust
        // The cards as laid out in the column, moved up by however far it is
        // scrolled, and cut to the part of the column on screen.
        let scrolled = self.scroller.vadjustment().value().round() as i32;
        let mut y = SHADOW_PX - scrolled;
        let mut rects = Vec::new();
        for &h in self.heights.borrow().iter() {
            rects.extend(blur::card_region((x, y, width, h), RADIUS_PX, on_screen));
            y += h + GAP_PX;
        }
        let top = SHADOW_PX;
        blur.set(&blur::clip_rows(&rects, top, top + self.slide.cards_h.get()));
```

- [ ] **Step 8: Follow the focus**

Inside `impl Panel`, after `disarm_unless_focused`:

```rust
    /// Scroll the focused card wholly into view, body and buttons, once the
    /// frame after the move has laid it out. A card just rendered has no place
    /// in the column until then.
    fn follow_focus(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        crate::taskbox::after_next_paint(&self.window, move || {
            let Some(p) = weak.upgrade() else { return };
            let Some(focus) = GtkWindowExt::focus(&p.window) else { return };
            let Some(card) = p.card_of(&focus) else { return };
            let Some(bounds) = card.compute_bounds(&p.column) else { return };
            let adjustment = p.scroller.vadjustment();
            let top = bounds.y() as f64;
            let bottom = top + bounds.height() as f64;
            adjustment.set_value(scroll_to_show(adjustment.value(), adjustment.page_size(), top, bottom));
        });
    }
```

Then add this paragraph to the module doc, after the `## The keyboard` section's first paragraph:

```rust
//! Wrapped, the cards can stand taller than the screen, so the column sits in
//! a scroller capped at the screen's height less its margins. Moving the focus
//! scrolls the focused card wholly into view, and the blur region moves with
//! the scroll and stops at the view's edges.
```

- [ ] **Step 9: Build, test, and rerun the e2e test**

Run: `cargo build --release && cargo test && NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh`
Expected: no warnings, and every check PASSes. With few short cards nothing scrolls, so the frames match Task 4's.

- [ ] **Step 10: Check the scroll by hand**

Ask the human partner, using the worktree daemon as in Task 4, Step 5. On a throwaway workspace tag, add 12 tasks, each with a description of three or four sentences:

```bash
for i in $(seq 1 12); do niritasks task add "Task $i: $(printf 'a fairly long sentence about the work. %.0s' 1 2 3 4 5 6)"; done
```

Press Mod+Alt+Ctrl+T. Arrow down to "+4 more" and press Enter. Then hold Down to the last card. Check that:
- The column never runs off the top or bottom of the screen.
- Each focused card, with its buttons, comes wholly into view.
- The blur stays behind the visible cards only, with no blurred band above or below the column.
- Up scrolls back.
- Escape folds everything back to the one-line peek.

Remove the test tasks after (`task +<tag> delete`, or Delete twice on each card).

- [ ] **Step 11: Commit**

```bash
git add src/panel/blur.rs src/panel/surface.rs src/taskbox.rs
git commit -m "Scroll the task panel once its cards stand taller than the screen, keeping the focused card in view

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Document the keyboard panel

**Files:**
- Modify: `README.md` (the `Mod+Alt+Ctrl+T` keybind row near line 19, the `## Task panel` section near lines 24-55, the test table row for `tests/e2e-panel.sh` near line 233, and the hand-check paragraph near lines 246-252)
- Modify: `CONTEXT.md` (the **Task card** entry, plus a new **Action row** entry after it)

**Interfaces:** none (docs only).

- [ ] **Step 1: The keybind row**

Replace the `Mod+Alt+Ctrl+T` row with:

```markdown
| `Mod+Alt+Ctrl+T` | Hand the task panel the keyboard: every card shows its whole description, with buttons to start working on it in its own worktree, refine it into a plan with Claude, edit it, stop it, or remove it. Enter opens the full menu (note, grill me, update status, move to another workspace). With no tasks, or no daemon, the fuzzel list instead |
```

- [ ] **Step 2: The Task panel section**

In `## Task panel`, change `only a 60px peek` to `only a 30px peek`. Replace the paragraph that begins `` `Mod+Alt+Ctrl+T` does the same from the keyboard`` with:

```markdown
`Mod+Alt+Ctrl+T` slides the panel out and hands it the keyboard. Every card
opens up to its whole description, wrapped, above a row of buttons:

| Button | Key | Does |
|---|---|---|
| **Start working** (green) | `s` | The menu's Start working: the task's own worktree, herdr tab and Claude, or back to them |
| **Refine** (mauve) | `r` | Work it up into a plan with Claude |
| **Edit** (yellow) | `e` | Open it in the task box |
| **Stop** (peach) | `t` | Stop it — only on an active task |
| **Remove** (red) | `Delete` | Turns into **Confirm remove**; a second press deletes the task, moving away puts it back |

Up and Down move a 1px white border between cards; Left, Right and Tab move
along the focused card's buttons, and Enter presses the focused one. Enter on
the card itself opens its full menu, which also has Note, Grill me, Update
status and Move to workspace. Every button gives the keyboard back as it runs,
and Escape tucks the panel away, folding the cards back to one line. A list
taller than the screen scrolls, keeping the focused card in view. Hovering
never shows the buttons — only the keyboard does.
```

- [ ] **Step 3: The test table and the hand checks**

In the test table, change the `tests/e2e-panel.sh` **Needs** cell to:

```markdown
niri, `python3-pil`, taskwarrior; `wtype` for its Escape check, which is skipped without it
```

Replace the hand-check sentence that begins `And the keyboard: \`Mod+Alt+Ctrl+T\`` and runs to the end of that paragraph with:

```markdown
And the keyboard, past what the script measures: `Mod+Alt+Ctrl+T` slides the
panel out with every card wrapped and the first bordered, Stop only on active
tasks; Up and Down move between cards, Left, Right and Tab along the buttons;
`s` starts working exactly as the menu does, `r` refines, `e` opens the box,
`t` stops; Delete arms Remove and only a second Delete deletes, while moving
away disarms it; Enter on a card opens the menu, Enter on "+N more" shows the
rest, and a list taller than the screen scrolls with the focus; Escape tucks it
away to the one-line peek.
```

- [ ] **Step 4: CONTEXT.md**

Replace the **Task card** entry with:

```markdown
**Task card**:
One task in the task panel, drawn like a notification: a status icon and a
one-line description — or, while the task panel has the keyboard, the whole
description wrapped, above its action row.
_Avoid_: row (that is the picker's word), item, block

**Action row**:
The buttons along a task card's bottom edge while the task panel has the
keyboard — Start working, Refine, Edit, Stop (an active task only) and
Remove — each running what the same entry in the task's action menu runs.
_Avoid_: toolbar, button bar, quick actions
```

- [ ] **Step 5: Check the docs against the code**

Run: `grep -n "60px\|one-line description" README.md CONTEXT.md`
Expected: no output.

Run: `grep -n "Confirm remove" README.md src/panel/actions.rs`
Expected: a match in each file.

- [ ] **Step 6: Final verification**

Run: `cargo test && cargo build --release && NIRITASKS=./target/release/niritasks bash tests/e2e-panel.sh`
Expected: everything PASSes. This is the spec's "Done when", together with the hand checks in Task 4, Step 5 and Task 5, Step 10.

- [ ] **Step 7: Commit**

```bash
git add README.md CONTEXT.md
git commit -m "Document the keyboard task panel's wrapped cards and action buttons

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
