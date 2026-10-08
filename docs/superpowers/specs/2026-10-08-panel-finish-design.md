# Panel finish design

Plan A of the 2026-10-08 (v3) architecture review: finish the panel state's
Mode so nothing sits beside it by discipline, put every fact about a task
action in one table, and clear the small tidies the review listed. No
behaviour the user sees changes, apart from two notifications and one hint
wording that now take the action's own label.

## 1. Finish the Mode (P7)

**`loaded` joins the variant.** `Mode::Projects(ProjectList)` becomes
`Mode::Projects { list: ProjectList, loaded: Projects }`, and the field
`PanelState.loaded` goes. The eight paired assignments go with it; the type
now says the lists a project list was built from travel with it.
`projects()` keeps returning `Option<&ProjectList>`; a new private
`listing() -> Option<(&ProjectList, &Projects)>` serves `hint` and
`on_folder`.

**The footer and the card hint come from the state.**

```rust
/// The footer under the list: the keys that act alike on every card, or the
/// project list's own. None on Ideas, where the text area has every key, and
/// none without the keyboard.
pub fn footer(&self) -> Option<&'static str>

/// The hint beside this card's buttons: the focused button's key and name,
/// or what Space does to its notes, then what Ctrl+Enter does to the task.
/// Empty on every card but the focused one, and on the focused one while its
/// Remove is armed: Confirm remove says what Enter does, and the hint would
/// not fit beside it.
pub fn card_hint(&self, uuid: &str) -> String
```

`Shown` gains `state: Option<TaskState>` (None on "+N more"), computed where
`visible()` already computes the row, so `card_hint` has the state without
the surface keeping a copy. The surface's `render` sets the footer from
`footer()`; `sync` sets each card's hint label from `card_hint(uuid)` and no
longer reads the notes widget's visibility or keeps `ActionRow.state`.

**One source for the Ctrl+Enter words.** `Action::hint` says
`Ctrl+Enter: <label>` ("Ctrl+Enter: Refine", "Ctrl+Enter: Start working")
and `advance`'s notification says `<label>: <task>`. The hand-written verbs
("refine"/"start working", "Refining"/"Starting") go.

Out of scope: `Armed::ClearAll` as a fourth `on_action` arm stays; it is a
pseudo-mode worth a later look, not this one.

## 2. One table per action (D3)

`src/actions.rs` gains:

```rust
/// What the panel draws and keys an action by: one row per action, so a new
/// action is one arm here and a CLI subcommand, nothing else.
pub struct Facts {
    pub icon: &'static str,
    pub class: &'static str,
    /// The key that presses the button, as the hint names it: a letter, or
    /// "Del" for Remove. None for a button only Enter presses.
    pub key: Option<&'static str>,
    pub colour: &'static str,
    pub keeps_keyboard: bool,
    pub leaves_the_list: bool,
}
impl Action { pub const fn facts(self) -> Facts }
```

`icon()`, `class()`, `letter()`, `keeps_keyboard()` and `leaves_the_list()`
become one-line readers of `facts()` in `actions.rs`; `panel/actions.rs`
loses its four matches and `ROW` (a pure alias of `ALL`; `row()` and
`for_letter()` read `ALL`); `style::colour(action)` becomes
`action.facts().colour`, the colour constants staying in `style.rs`. `label`,
`args` and `applies` stay as they are: they depend on state or on words, not
on a fixed fact. `panel/actions.rs` keeps the row policy (`row`, `advance`,
`hint`, `for_letter`), Clear all and the footer text, and its module doc
says so.

## 3. Tidies (S1)

`surface.rs:49`'s "Enter or a click on its body does nothing" becomes true;
`state.rs`'s module doc names the notes; `daemon::serve_box_request` becomes
`serve_request` with one `focused_panel()` lookup for the Projects and Panel
arms and one `notice_if_other_open(opened)` for Add and Edit/Note; the three
tests that assert a constant equals its own literal go; `PEEK_PX`,
`NAMESPACE`, `FOLDER_ICON` and `GITHUB_ICON` lose `pub`; e2e-panel presses
Space on a card's body where it presses Enter today, since the two are meant
to be alike.

## Tests

Existing: 115 PanelState tests, the `panel/actions.rs` row and hint tests,
`style::every_action_button_has_its_colour`, `every_button_has_its_own_class`.
New: `footer()` per mode and without the keyboard; `card_hint` on the focused
card, another card, an armed Remove, and a body with notes before and after
the toggle; `facts()` rows distinct by class, icon and key (replacing the
per-match tests); the e2e Space press.
