# Panel Finish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the panel state's `Mode` (the loaded projects travel inside the variant; the footer and card hint come from the state), put every drawn-and-keyed fact about a task action in one table, and clear the v3 review's small tidies.

**Architecture:** Three independent refactors with no user-visible change beyond two notification/hint wordings taking the action's label. Task 1 and 2 are `src/panel/state.rs` with `surface.rs` consuming two new accessors; Task 3 is `src/actions.rs` absorbing four matches from `panel/actions.rs` and one from `style.rs`; Task 4 is comments, a daemon rename with two small helpers, dead tests, visibility, and one e2e press.

**Tech Stack:** Rust (gtk4), bash e2e in a nested niri.

**Spec:** `docs/superpowers/specs/2026-10-08-panel-finish-design.md` (same commit as this plan).

## Global Constraints

- Conventional Commits, `<type>(<scope>): <summary>`, imperative, lowercase, subject ≤ 72, body wrapped at 72 saying what and why; end with an attribution trailer (`Co-Authored-By: <your model> <noreply@anthropic.com>`).
- `cargo test` passes and `cargo build` has no warnings after every task.
- Work in the `panel-finish` worktree (`/home/paul/.worktrees/niri-tasks/panel-finish`); `finish-worktree` lands it.
- **Never run `install.sh`, `cargo install` or `systemctl --user restart niri-tasks`** from the worktree. Run e2e only as `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh` (the harness now unsets HERDR_* itself); if the machine cannot run it, say so.
- The panel acts only by spawning `niritasks` commands; the state does no I/O.
- No user-visible change except: the Ctrl+Enter hint reads `Ctrl+Enter: Refine` / `Ctrl+Enter: Start working`, and the Ctrl+Enter notification reads `Refine: <task>` / `Start working: <task>`.
- Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Comments are full sentences that say why; docs on every pub item.
- Line numbers are anchors from `main` at b5a8bb6; match on quoted text.

---

### Task 1: `loaded` joins `Mode::Projects`

**Files:**
- Modify: `src/panel/state.rs` (`Mode` enum ~:108-119; field `loaded` :148-151; `projects()`/`projects_mut()` :203-215; `set_cards` :310-318; `take_keyboard` :338; `open_projects` :355-356; `show_projects` :580-582; `hint` :598-600; `on_folder` :608-615; `release` :644-645; `escape_projects` :711-712; tests matching `Mode::Projects(_)`)

**Interfaces:**
- Produces: `Mode::Projects { list: ProjectList, loaded: Projects }`; private `fn listing(&self) -> Option<(&ProjectList, &Projects)>`. `projects()`, `projects_mut()`, `hint()`, `mode()` keep their signatures.

- [ ] **Step 1: Write the failing test**

Add to `state.rs` tests:

```rust
    /// The lists a project list was built from travel with it: leaving the
    /// list, however it is left, leaves no loaded projects behind.
    #[test]
    fn the_loaded_projects_live_and_die_with_the_list() {
        let mut state = keyboard(pending(&["a", "b"]));
        state.show_projects("a", folders(&["x"]), projects(&["a", "x"], &[]));
        assert!(matches!(state.mode(), Mode::Projects { loaded, .. } if loaded.local == vec!["a", "x"]));
        key(&mut state, KeyAction::Release);
        assert!(matches!(state.mode(), Mode::Tasks));
        state.open_projects(projects(&["x"], &["y"]));
        assert!(matches!(state.mode(), Mode::Projects { loaded, .. } if loaded.remote == vec!["y"]));
        state.on_folder(0);
        assert!(matches!(state.mode(), Mode::Tasks));
    }
```

Run: `cargo test --lib panel::state::the_loaded`
Expected: FAIL to compile (`Mode::Projects` is a tuple variant).

- [ ] **Step 2: Implement**

- Change the variant to
```rust
    /// The project list, for Move to workspace or Mod+Alt+W, with the
    /// folders and repos it was built from, so its hint and its pick mean
    /// what `project open` will.
    Projects { list: ProjectList, loaded: Projects },
```
- Delete the field `loaded` and its doc from `PanelState`.
- `projects()`: `Mode::Projects { list, .. } => Some(list)`; `projects_mut()` likewise with `&mut`.
- Add after `projects_mut`:
```rust
    /// The project list and the lists it was built from, while it is up.
    fn listing(&self) -> Option<(&ProjectList, &Projects)> {
        match &self.mode {
            Mode::Projects { list, loaded } => Some((list, loaded)),
            _ => None,
        }
    }
```
- `open_projects`: `self.mode = Mode::Projects { list: ProjectList::new(Purpose::Open, projects.rows()), loaded: projects };` (compute `rows()` before moving `projects`: `let list = ProjectList::new(Purpose::Open, projects.rows());`).
- `show_projects`: `self.mode = Mode::Projects { list: ProjectList::new(Purpose::Move { uuid: uuid.into(), text }, rows), loaded: projects };`.
- `hint`: `let (list, loaded) = self.listing()?; list.hint(loaded)`.
- `on_folder`: `let Some((list, loaded)) = self.listing() else { return Vec::new() };` and drop `self.loaded = None;`.
- Delete every `self.loaded = None;` (`set_cards`, `take_keyboard`, `release`, `escape_projects`).
- `escape_projects` closes with `self.mode = Mode::Tasks;` only; note that `list` is borrowed from `self.mode` there, so copy `list.purpose() == &Purpose::Open` into a local before assigning the mode.
- In tests, every `Mode::Projects(_)` pattern becomes `Mode::Projects { .. }`.

Run: `cargo test --lib panel::state:: && cargo build 2>&1 | grep -E "^(warning|error)"`
Expected: PASS; no output.

- [ ] **Step 3: Commit**

```bash
git add src/panel/state.rs
git commit -m "refactor(panel): carry the loaded projects inside Mode::Projects

The lists a project list was built from must be present exactly while
the list is up; eight paired assignments kept that true by hand. As a
field of the variant the type says it.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

### Task 2: The footer and the card hint come from the state

**Files:**
- Modify: `src/panel/state.rs` (`Shown` :56-61 and `visible()` :230-250; new `footer`, `card_hint`; `advance` :845-855; module doc :1-13), `src/panel/actions.rs` (`hint` :61-83: the Ctrl+Enter words), `src/panel/surface.rs` (`render` :931-935; `sync` :1189-1230; `ActionRow.state` :285-294 and `action_row` :1084-1115; module doc :49)
- Tests: `state.rs`, `panel/actions.rs` (hint tests :447-565)

**Interfaces:**
- Consumes: `Action::hint(state, row, focused, notes) -> String`, `actions::CARD_KEYS`, `ProjectList::keys()`.
- Produces: `Shown.state: Option<TaskState>`; `PanelState::footer(&self) -> Option<&'static str>`; `PanelState::card_hint(&self, uuid: &str) -> String`.

- [ ] **Step 1: Write the failing tests**

In `state.rs` tests:

```rust
    #[test]
    fn the_footer_follows_the_mode() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a"]));
        assert_eq!(state.footer(), None, "no footer without the keyboard");
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.footer(), Some(actions::CARD_KEYS));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(state.footer(), None, "the text area has every key");
        key(&mut state, KeyAction::Release);
        state.show_projects("a", folders(&["x"]), projects(&["a", "x"], &[]));
        assert_eq!(state.footer(), Some(super::super::projects::MOVE_KEYS));
    }

    #[test]
    fn only_the_focused_card_has_a_hint_and_an_armed_remove_blanks_it() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert!(!state.card_hint("a").is_empty());
        assert_eq!(state.card_hint("b"), "");
        key(&mut state, KeyAction::NextSlot);
        assert!(state.card_hint("a").starts_with("s: Start working"), "{}", state.card_hint("a"));
        state.on_press("a", Slot::Button(Action::Remove));
        assert_eq!(state.card_hint("a"), "", "Confirm remove says what Enter does");
    }

    #[test]
    fn the_body_hint_follows_the_notes_toggle() {
        let mut with_notes = card("a", Status::Pending);
        with_notes.notes = vec!["a note".into()];
        let mut state = keyboard(vec![with_notes]);
        assert!(state.card_hint("a").starts_with("Space: view notes"), "{}", state.card_hint("a"));
        state.on_press("a", Slot::Body);
        assert!(state.card_hint("a").starts_with("Space: hide notes"), "{}", state.card_hint("a"));
    }
```
(`card`, `pending`, `keyboard`, `key`, `folders`, `projects` are the module's existing test helpers; check `card()` builds a `Card` whose `notes` field is settable, else build the `Card` literal as `model.rs`'s tests do.)

In `panel/actions.rs` tests, change the expectations that read `Ctrl+Enter: refine` to `Ctrl+Enter: Refine` and `Ctrl+Enter: start working` to `Ctrl+Enter: Start working` (tests `the_body_hints_what_ctrl_enter_does`, `a_lettered_button_hints_its_letter_and_name`, `every_hint` and any other that spells them).

Run: `cargo test --lib panel::`
Expected: FAIL (`footer`, `card_hint` not found; the two hint wordings).

- [ ] **Step 2: Implement in the state**

- `Shown`: add `/// The task's state, for its hint; None on "+N more".` `pub state: Option<TaskState>,` and in `visible()` keep the computed `TaskState` (today `card.state(has_session).map(Action::row)`) as `let state = card.state(has_session); let actions = state.map(Action::row).unwrap_or_default(); Shown { card, actions, state }`. Off the keyboard `state` is `None` as `actions` is empty. Import `crate::actions::TaskState`.
- Add:
```rust
    /// The footer under the list: the keys that act alike on every card, or
    /// the project list's own. None on Ideas, where the text area has every
    /// key, and none without the keyboard.
    pub fn footer(&self) -> Option<&'static str> {
        if !self.keyboard {
            return None;
        }
        match &self.mode {
            Mode::Ideas => None,
            Mode::Projects { list, .. } => Some(list.keys()),
            Mode::Tasks => Some(actions::CARD_KEYS),
        }
    }

    /// The hint beside this card's buttons: the focused button's key and
    /// name, or what Space does to its notes, then what Ctrl+Enter does to
    /// the task. Empty on every card but the focused one, and on the focused
    /// one while its Remove is armed: Confirm remove says what Enter does,
    /// and the hint would not fit beside it.
    pub fn card_hint(&self, uuid: &str) -> String {
        if !self.keyboard || !self.on_tasks() {
            return String::new();
        }
        let Some(focus) = self.focus.as_ref().filter(|f| f.uuid == uuid) else { return String::new() };
        if self.armed == Armed::Remove(uuid.to_string()) {
            return String::new();
        }
        let shown = self.visible();
        let Some(card) = shown.iter().find(|s| s.card.uuid.as_deref() == Some(uuid)) else { return String::new() };
        let Some(state) = card.state else { return String::new() };
        let focused = match focus.slot {
            Slot::Button(action) => Some(action),
            Slot::Body => None,
        };
        let notes = (!card.card.notes.is_empty()).then(|| self.shows_notes(uuid));
        Action::hint(state, &card.actions, focused, notes)
    }
```
- `advance`: replace `let verb = …; Effect::Notify(format!("{verb}: {}", …))` with `Effect::Notify(format!("{}: {}", action.label(false), card.card.text))`.
- Module doc: add "which cards show their notes" to the list of what lives here.

In `panel/actions.rs` `hint`: replace `let does = if step == Refine { "refine" } else { "start working" }; parts.push(format!("Ctrl+Enter: {does}"));` with `parts.push(format!("Ctrl+Enter: {}", step.label(false)));` and reword its doc's example.

- [ ] **Step 3: The surface consumes them**

`surface.rs`:
- `render`: replace `self.footer.set_visible(keyboard && !ideas); self.footer.set_label(list.map_or(actions::CARD_KEYS, |l| l.keys()));` with
```rust
        let footer = self.state.borrow().footer();
        self.footer.set_visible(footer.is_some());
        if let Some(text) = footer {
            self.footer.set_label(text);
        }
```
  and delete the `ideas`/`list` locals if nothing else uses them (the focus tail at the end still needs `ideas` and `list.is_some()`; keep what it needs).
- `sync`: replace the `let hint = match &focus { … }` block with `let hint = card.uuid.as_deref().map_or(String::new(), |u| self.state.borrow().card_hint(u));` (take the state borrow per card or once above the loop into a local closure; do not hold it across `set_label`, which is fine since `card_hint` returns an owned String). Remove the now-unused `focus` destructuring if only the hint used it (`show_row` still uses `focus`; keep it).
- `ActionRow`: delete the field `state` and its doc; `action_row` still takes `state: TaskState` for the tooltip's `label(state.up_next)` but no longer stores it.
- Module doc line 49: "The focused card is darkened. Enter or a click on its body does nothing." → "The focused card is darkened. Enter, Space or a click on its body shows the task's notes, or hides them."

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4; grep -n "is_visible()" src/panel/surface.rs`
Expected: no warnings; all pass; the grep no longer shows a `notes` visibility read feeding a hint (other `is_visible` uses may remain).

- [ ] **Step 4: e2e, if the machine allows**

`cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -4`. Expected `failed: 0`.

- [ ] **Step 5: Commit**

```bash
git add src/panel/state.rs src/panel/actions.rs src/panel/surface.rs
git commit -m "refactor(panel): say the footer and the card hint from the state

The footer's text was chosen in render and the card's hint in sync,
which read a widget's visibility to learn whether notes showed. Both
now come from PanelState, where the mode, the focus, the arming and the
shown notes already are, and the Ctrl+Enter words take the action's own
label instead of two hand-written verbs.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

### Task 3: One table per action

**Files:**
- Modify: `src/actions.rs` (add `Facts` and `facts()`; `icon()` :157-180 becomes a reader; add `class`, `letter`, `keeps_keyboard`, `leaves_the_list` readers), `src/panel/actions.rs` (delete `ROW` :16, `keeps_keyboard` :99, `leaves_the_list` :107, `class` :111-128, `letter` :131-144, `key` :86-91; `row()` and `for_letter()` read `ALL`; module doc), `src/panel/style.rs` (`colour()` :89-104 becomes `action.facts().colour`; constants stay), tests in all three.

**Interfaces:**
- Produces: `pub struct Facts { icon, class, key: Option<&'static str>, colour, keeps_keyboard, leaves_the_list }` and `pub const fn facts(self) -> Facts` on `Action`; `icon()`, `class()`, `letter()`, `keeps_keyboard()`, `leaves_the_list()` keep their names and signatures but live in `actions.rs`. `Action::ROW` is gone.

- [ ] **Step 1: Write the failing tests**

In `actions.rs` tests, add:

```rust
    /// One row per action, and no two rows alike where the panel tells
    /// buttons apart: by glyph, by class, and by key.
    #[test]
    fn every_action_has_its_own_glyph_class_and_key() {
        let mut icons: Vec<&str> = Action::ALL.iter().map(|a| a.facts().icon).collect();
        let mut classes: Vec<&str> = Action::ALL.iter().map(|a| a.facts().class).collect();
        let mut keys: Vec<&str> = Action::ALL.iter().filter_map(|a| a.facts().key).collect();
        for list in [&mut icons, &mut classes, &mut keys] {
            let before = list.len();
            list.sort();
            list.dedup();
            assert_eq!(list.len(), before);
        }
        assert_eq!(Action::Remove.facts().key, Some("Del"));
        assert_eq!(Action::Speak.facts().key, None);
    }

    /// What a press does to the panel, read off the table.
    #[test]
    fn the_table_says_what_a_press_does_to_the_panel() {
        let keeps: Vec<Action> = Action::ALL.into_iter().filter(|a| a.keeps_keyboard()).collect();
        assert_eq!(keeps, vec![Back, Speak, UpNext, Complete, Move, Wait, Remove]);
        let leaves: Vec<Action> = Action::ALL.into_iter().filter(|a| a.leaves_the_list()).collect();
        assert_eq!(leaves, vec![Back, Complete, Wait, Remove]);
    }
```

Run: `cargo test --lib actions::`
Expected: FAIL to compile (`facts` not found).

- [ ] **Step 2: Implement the table**

In `src/actions.rs`, after `TaskState`'s impl:

```rust
/// What the panel draws and keys an action by: its glyph, its CSS class, the
/// key that presses it, its colour, and what pressing it does to the panel.
/// One row per action, so a new action is one arm of [`Action::facts`] and
/// a CLI subcommand, nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    /// The glyph on the action row, from the Nerd Font the cards use.
    pub icon: &'static str,
    /// The button's CSS class, which the stylesheet colours.
    pub class: &'static str,
    /// The key that presses the button while the panel has the keyboard, as
    /// the hint names it: a letter, or "Del" for Remove. None for a button
    /// only Enter presses.
    pub key: Option<&'static str>,
    /// The button's colour, from `panel::style`'s palette.
    pub colour: &'static str,
    /// The panel keeps the keyboard after the button runs: nothing opens that
    /// would need it, and the list stays up.
    pub keeps_keyboard: bool,
    /// The button takes its card off the list, so the focus moves to a
    /// neighbour first.
    pub leaves_the_list: bool,
}

impl Action {
    /// The row of the table for this action.
    pub const fn facts(self) -> Facts {
        use crate::panel::style as c;
        match self {
            Session => Facts { icon: "\u{f120}", class: "session", key: Some("g"), colour: c::SESSION, keeps_keyboard: false, leaves_the_list: false },
            Back => Facts { icon: "\u{f0e2}", class: "back", key: Some("b"), colour: c::BACK, keeps_keyboard: true, leaves_the_list: true },
            Start => Facts { icon: "\u{f04b}", class: "start", key: Some("s"), colour: c::ACTIVE, keeps_keyboard: false, leaves_the_list: false },
            Refine => Facts { icon: "\u{f0d0}", class: "refine", key: Some("r"), colour: c::REFINE, keeps_keyboard: false, leaves_the_list: false },
            Grill => Facts { icon: "\u{f086}", class: "grill", key: Some("i"), colour: c::REFINE, keeps_keyboard: false, leaves_the_list: false },
            Edit => Facts { icon: "\u{f040}", class: "edit", key: Some("e"), colour: c::EDIT, keeps_keyboard: false, leaves_the_list: false },
            Speak => Facts { icon: "\u{f0580}", class: "speak", key: None, colour: c::SPEAK, keeps_keyboard: true, leaves_the_list: false },
            UpNext => Facts { icon: "\u{f02e}", class: "up-next", key: None, colour: c::UP_NEXT, keeps_keyboard: true, leaves_the_list: false },
            Complete => Facts { icon: "\u{f00c}", class: "complete", key: Some("c"), colour: c::COMPLETE, keeps_keyboard: true, leaves_the_list: true },
            Move => Facts { icon: "\u{f07c}", class: "move", key: Some("m"), colour: c::MOVE, keeps_keyboard: true, leaves_the_list: false },
            Stop => Facts { icon: "\u{f04d}", class: "stop", key: Some("t"), colour: c::STOP, keeps_keyboard: false, leaves_the_list: false },
            Wait => Facts { icon: "\u{f04c}", class: "wait", key: None, colour: c::WAIT, keeps_keyboard: true, leaves_the_list: true },
            Remove => Facts { icon: "\u{f1f8}", class: "remove", key: Some("Del"), colour: c::REMOVE, keeps_keyboard: true, leaves_the_list: true },
        }
    }

    pub fn icon(self) -> &'static str { self.facts().icon }
    pub fn class(self) -> &'static str { self.facts().class }
    /// The letter that presses the button; None for Remove, whose key is
    /// Delete and is `keys.rs`'s, and for a button only Enter presses.
    pub fn letter(self) -> Option<char> {
        let key = self.facts().key?;
        let mut chars = key.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    }
    pub fn keeps_keyboard(self) -> bool { self.facts().keeps_keyboard }
    pub fn leaves_the_list(self) -> bool { self.facts().leaves_the_list }
}
```

Carry the existing doc comments for `icon`, `keeps_keyboard` and `leaves_the_list` over from their old homes (the glyph names, the reasoning about which buttons keep the keyboard). Delete the old `icon()` match. Keep the Speak-glyph comment on the Speak row.

In `src/panel/actions.rs`: delete `ROW`, `key()`, `keeps_keyboard`, `leaves_the_list`, `class`, `letter`; `row()` iterates `Self::ALL`; `for_letter` searches `Self::ALL`; `hint` uses `action.facts().key` where it called `action.key()`. Rewrite the module doc: "The action row: which of the task actions a card shows and in what order, the key hints beside them, and the Waiting tab's Clear all. The facts about each action it draws by (glyph, class, key, colour, what a press does to the panel) are `Action::facts`' in `crate::actions`." Delete the tests `every_button_has_its_own_class` and `letters_press_their_own_buttons` only if they duplicate the new table test; otherwise keep them pointing at the readers.

In `src/panel/style.rs`: `colour()` becomes `pub fn colour(action: Action) -> &'static str { action.facts().colour }`, its doc kept; `every_action_button_has_its_colour` keeps passing.

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4; grep -rn "Action::ROW\|\.key()" src/panel`
Expected: no warnings; pass; the grep prints nothing.

- [ ] **Step 3: Commit**

```bash
git add src/actions.rs src/panel/actions.rs src/panel/style.rs
git commit -m "refactor(actions): put each action's drawn facts in one table

Adding Complete touched five files and six lists: the glyph in one
match, the class and the letter in two more, the colour in a third
file, and what a press does to the panel in two predicates. One Facts
row per action holds them; the readers keep their names so no caller
changes.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

### Task 4: Tidies

**Files:**
- Modify: `src/daemon.rs` (`serve_box_request` :224-298 and its call site; `already_open_notice` :206-208; `open_task_box`), `src/panel/actions.rs` (tests `remove_asks_before_it_deletes` :406, `clear_all_reads_as_remove_does` :412), `src/panel/style.rs` (test asserting `HINT_FONT` literal :442), `src/panel/surface.rs` (`pub const NAMESPACE`, `pub const PEEK_PX` → private), `src/panel/projects.rs` (`FOLDER_ICON`, `GITHUB_ICON` → private), `tests/e2e-panel.sh` (:309-350 the notes block)

- [ ] **Step 1: The daemon's request dispatch**

Rename `serve_box_request` to `serve_request` (and its doc: "Serve one request from the CLI: a task box, the panel's keyboard, or the project list."). Add:

```rust
/// The panel on the monitor showing the focused workspace, and that
/// workspace, for a request that acts on what the user is looking at. None
/// before GTK has named the monitor (see `sync_monitors`).
fn focused_panel() -> Option<(Rc<Panel>, niri_ipc::Workspace)> {
    let focused = niri::focused_workspace().ok().flatten()?;
    let output = focused.output.clone()?;
    let panel = PANELS.with(|p| p.borrow().get(&output).cloned())?;
    Some((panel, focused))
}
```

and use it in the `Projects` arm (`let Some((panel, _)) = focused_panel() else { notify::tasks(NO_PANEL); return; };`) and the `Panel` arm (`focused_panel()` giving the panel and the workspace name for `live_agent_names`; the `None` case notifies `no_cards_text(...)` as the missing-panel case did). Keep behaviour identical: today a missing panel on `Panel` falls into the `no_cards_text` notification; keep that.

Replace the two `if let Some(notice) = already_open_notice(opened) { notify::tasks(notice); }` sites with one `fn notice_if_other_open(opened: taskbox::Opened)` that does both, called from the Add arm and `open_task_box`.

- [ ] **Step 2: Tests that assert a literal, and visibility**

Delete `remove_asks_before_it_deletes` and `clear_all_reads_as_remove_does` in `panel/actions.rs` and the `assert_eq!(HINT_FONT, "9pt …")` line (or the whole test if that is all it does) in `style.rs`. Make `NAMESPACE` and `PEEK_PX` in `surface.rs` and `FOLDER_ICON`/`GITHUB_ICON` in `projects.rs` non-pub (`const`), confirming with `cargo build` that nothing outside used them; if the e2e script's comments cite them by name, leave the comments.

- [ ] **Step 3: Space on a card's body in e2e**

In `tests/e2e-panel.sh`, the notes block (~:309-350) presses Return to show the notes, shoots `notes_shown`, ticks, presses Return again and shoots `notes_hidden`. After `notes_hidden` is checked, add: press `wtype -k space`, shoot `notes_space`, and assert `same notes_shown notes_space` with `ok "Space on the body shows the notes as Enter does"` / `bad …`; then press space again and assert `same keyboard_down notes_space_hidden` with a matching ok/bad, so the panel is back where the following checks expect it. Use the file's existing `shot`, `same`, `measure`, `ok`, `bad` helpers exactly as the Return block does, including the `|| { summary; exit 1; }` after each `shot`.

Run: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -6`
Expected: `failed: 0`, the two new PASS lines present. If the machine cannot run it, say so.

- [ ] **Step 4: Verify and commit**

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4; grep -n "serve_box_request\|already_open_notice(" src/daemon.rs`
Expected: no warnings; pass; the grep prints nothing.

```bash
git add src/daemon.rs src/panel/actions.rs src/panel/style.rs src/panel/surface.rs src/panel/projects.rs tests/e2e-panel.sh
git commit -m "chore(panel): tidy the daemon dispatch, dead tests and visibility

serve_request names what it does and looks the focused panel up once;
three tests that asserted a constant against its own literal go; four
constants only their file used stop being pub; and e2e presses Space on
a card's body, which is meant to do what Enter does there.

Co-Authored-By: <model> <noreply@anthropic.com>"
```

---

## Self-review notes

- Spec §1 → Tasks 1 and 2; §2 → Task 3; §3 → Task 4. Nothing in the spec is unaddressed.
- Type consistency: `Mode::Projects { list, loaded }` (Task 1) is what Task 2's `footer()` matches on; `Shown.state: Option<TaskState>` (Task 2) is what `card_hint` reads; `Action::facts().key` (Task 3) is what `hint` reads where Task 2 left `action.key()` (Task 3 changes that call).
- Task order matters only between 1 and 2 (same file, same variant); 3 and 4 are independent of them.
- Known wording change: the Ctrl+Enter hint and notification take the action's label (Global Constraints).
