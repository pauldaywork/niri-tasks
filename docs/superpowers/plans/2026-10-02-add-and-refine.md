# Add & Refine from the Task Box — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the task box's add mode, an "Add & refine" button (and Ctrl+Shift+Enter) adds the task and then hands it straight to Claude's `refine-task` skill. A new keybind, Mod+Alt+Shift+T, opens the box with Add & refine as the default.

**Architecture:** `task::add_with_notes` returns the new task's uuid instead of dropping it. The box (`src/taskbox.rs`) gets a second button in add mode, and its `Submission` gets a `refine` flag that says which button was pressed. `BoxConfig.refine` says which button Ctrl+Enter presses. The CLI's `task add --refine` and the IPC `add refine` request carry that default to the box. After a refining add, both add paths (daemon and CLI fallback) call `refine::spawn_quick(uuid)`. It starts `niritasks task refine <uuid>` through niri's spawn, the same way the panel's Refine button does (`panel::surface::open_menu`). The refine runs in its own process, so the daemon's GTK loop never waits on herdr. If refine fails, the `main()` of that process reports the error with `notify::error`.

**Tech Stack:** Rust 2021, gtk4-rs, clap, taskwarrior CLI, niri IPC (`niri_ipc`). The e2e test runs in bash with wtype in a nested niri (`tests/lib/nested-niri.sh`).

**Spec:** Taskwarrior task `d9f76b94-e0ff-44df-85b4-060be4219169`. Read it with `task rc.json.array=on d9f76b94-e0ff-44df-85b4-060be4219169 export`. Its description and notes are the spec.

## Global Constraints

- In add mode the box has a second button, **"Add & refine"**, next to **"Add"**. Ctrl+Shift+Enter presses it, and the hint line says so.
- New keybind **Mod+Alt+Shift+T** runs `niritasks task add --refine`. It opens the same box with Add & refine as the default, so Ctrl+Enter presses it. The other button stays available. The bind uses the same `flock` guard (same lock file) as Mod+Alt+T.
- Quick refine only (`refine::Mode::Quick`). Grill me stays in the menu.
- After the add, spawn `niritasks task refine <uuid>` as its own process, the way the panel's Refine button does, so the daemon's GTK loop never blocks on herdr. If refine fails, the task stays added and a notification shows the error.
- Out of scope: Add & grill. Refining from Edit or Note mode. In Edit/Note mode, Ctrl+Shift+Enter keeps doing what it does today, which is save.
- Done when: both buttons and both shortcuts work from the daemon and from the CLI fallback. An Add & refine opens a "Refine: …" tab in the workspace's herdr session for the new task. `cargo test` passes. The README keybind table and CONTEXT.md's **Task box** entry mention the new option.
- House style: every item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commit messages are one plain-English imperative sentence, like `git log` shows, and end with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line.

## Decisions made while planning (flag any you disagree with)

1. **Spawn through niri, not `std::process::Command`.** The Mod+Alt+T bind holds its `flock` through a file descriptor that `niritasks task add` inherits. A child started with `Command` would inherit it too and keep the lock held for as long as Claude's terminal stays open. That is the exact trap the comment in `niri/niri-tasks.kdl` describes for the pickers. niri's spawn has no such inheritance, and it is what `panel::surface::open_menu` already uses for the Refine button.
2. **The spawned process reports its own failures.** `niritasks task refine` already sends every error to `notify::error` from `main()`. `spawn_quick` only has to notify when the spawn itself fails (no niri, no `current_exe`).
3. **Ctrl+Shift+Enter always means Add & refine in add mode, and Ctrl+Enter means the default button.** So in a `--refine` box both shortcuts refine, and plain Add is reached by Tab or a click. In Edit/Note, Ctrl+Shift+Enter saves, as it does today, because `key_action` checked only Ctrl.
4. **`Submission.refine` is set after `form::submission` builds it.** `form::submission` stays the pure "what is on screen" function and always returns `refine: false`. The button that was pressed sets the flag.
5. **`task add --refine <text>` also refines.** It is free once the flag exists, and a script can use it.
6. **IPC: `add` / `add refine`.** If an older daemon gets `add refine`, it decodes it as a plain `add` and opens a normal box. That is harmless, and installing restarts the daemon anyway.
7. **The hint label wraps.** The add-mode hint is longer than the room beside three buttons in an 800px window, so `set_wrap(true)` lets it take two lines instead of pushing the window wider.
8. **The e2e test observes refine by letting it fail on purpose.** The nested niri gets an `environment` block (opt-in, `NESTED_SPAWN_PATH`) whose PATH has the notify-send stub and no herdr. A spawned refine then fails with "herdr is not installed." and the stub records that. This proves the refine was spawned for a pending task, and it never touches your real herdr. The real "Refine: …" tab is the manual check in the last task.

## File map

| File | Change |
|---|---|
| `src/task.rs` | `add_with_notes` returns `Result<Option<String>>` |
| `tests/write_path.rs` | asserts the returned uuid |
| `src/taskbox/keys.rs` | `key_action` takes `shift`, new `KeyAction::Refine` |
| `src/taskbox/form.rs` | `Submission.refine` |
| `src/taskbox.rs` | `BoxConfig.refine`, `BoxConfig::add(tag, refine)`, second button, `hint()`, `do_submit(refine)` |
| `src/refine.rs` | `quick_command`, `spawn_quick` |
| `src/ipc.rs` | `Request::Add { refine }`, `add refine` on the wire |
| `src/daemon.rs` | serves `Add { refine }`, spawns refine after a refining add |
| `src/main.rs` | `task add --refine`, spawns refine after a refining add |
| `tests/lib/nested-niri.sh` | opt-in `NESTED_SPAWN_PATH` environment block |
| `tests/e2e-box.sh` | Add & refine suite, run in both halves |
| `niri/niri-tasks.kdl` | Mod+Alt+Shift+T bind, lock comment |
| `README.md`, `CONTEXT.md` | keybind table, box key table, CLI list, Task box entry |

---

### Task 1: `add_with_notes` hands back the new task's uuid

**Files:**
- Modify: `src/task.rs:239-257` (`add_with_notes`)
- Test: `tests/write_path.rs` (inside `write_path_lifecycle`, right after the `---- add parses taskwarrior attributes ----` block, around line 100)

**Interfaces:**
- Produces: `pub fn add_with_notes(tag: &str, description_args: &[&str], notes: &[String]) -> Result<Option<String>>`. `Some(uuid)` is the full 36-character uuid of the task created. `None` means nothing was added because the description was empty.

- [ ] **Step 1: Write the failing test**

In `tests/write_path.rs`, add this right after the attributes block, before `// ---- edit does NOT split ----`. It uses a tag of its own so the `pending_for_tag(TAG)` counts later in the test are unchanged.

```rust
    // ---- add with notes hands back the uuid -------------------------------
    // Add & refine needs it: the refine tab opens on the task just made.
    // A tag of its own, so the counts on TAG below are unchanged.
    let added = task::add_with_notes("refinesandbox", &text::add_args("refine me next"), &["a first note".into()])
        .expect("add with notes")
        .expect("a description was given, so a task was made");
    let fresh = task::get(&added).expect("get").expect("the uuid names the new task");
    assert_eq!(fresh.uuid, added, "the full uuid, not a prefix");
    assert_eq!(fresh.description, "refine me next");
    assert_eq!(fresh.annotations.len(), 1);
    assert_eq!(
        task::add_with_notes("refinesandbox", &[], &[]).expect("empty add"),
        None,
        "nothing added, so no uuid"
    );
```

- [ ] **Step 2: Run the test to check it fails**

Run: `cargo test --test write_path`
Expected: compile error: `add_with_notes` returns `()`, which has no `expect`.

- [ ] **Step 3: Make it return the uuid**

Replace `add_with_notes` in `src/task.rs` with this, and extend its doc comment:

```rust
/// Add a task and attach one annotation per note, returning the new task's
/// uuid — `None` when the description was empty and nothing was added.
///
/// The notes are the add box's rows, in order, empty ones already dropped.
/// They are attached one at a time rather than joined, because separate
/// annotations are what the picker's `¶` marker and the box's rows are
/// counting.
///
/// A note that cannot be attached fails the whole call: the task is already
/// added by then, so the caller is told rather than left believing the notes
/// went with it.
///
/// The uuid is what Add & refine hands to `niritasks task refine`.
pub fn add_with_notes(tag: &str, description_args: &[&str], notes: &[String]) -> Result<Option<String>> {
    let Some(uuid) = add(tag, description_args)? else {
        return Ok(None);
    };
    for note in notes {
        annotate(&uuid, note)?;
    }
    Ok(Some(uuid))
}
```

The two callers (`src/daemon.rs:227` in `if let Err(e) = …`, and `src/main.rs:255` as `…?;`) still compile unchanged. Task 4 starts using the value.

- [ ] **Step 4: Run the tests to check they pass**

Run: `cargo test`
Expected: PASS, with no new warnings from `cargo build`.

- [ ] **Step 5: Commit**

```bash
git add src/task.rs tests/write_path.rs
git commit -m "Return the new task's uuid from add_with_notes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Ctrl+Shift+Enter maps to Add & refine

**Files:**
- Modify: `src/taskbox/keys.rs` (enum, `key_action`, tests)
- Modify: `src/taskbox.rs:419-435` (the key handler passes Shift)

**Interfaces:**
- Produces: `pub fn key_action(key: gdk::Key, ctrl: bool, shift: bool, place: Option<Place>) -> KeyAction`, and the new variant `KeyAction::Refine`. Task 3 relies on both names.

- [ ] **Step 1: Write the failing tests**

In `src/taskbox/keys.rs` tests, add `false` as the new third argument to every existing `key_action(…)` call. For example, `key_action(gdk::Key::Escape, false, place)` becomes `key_action(gdk::Key::Escape, false, false, place)`. In `ordinary_typing_is_passed_through`, also loop over shift:

```rust
    #[test]
    fn ordinary_typing_is_passed_through() {
        for key in [gdk::Key::a, gdk::Key::space, gdk::Key::Tab, gdk::Key::Up] {
            for place in EVERYWHERE {
                for (ctrl, shift) in [(false, false), (true, false), (false, true), (true, true)] {
                    assert_eq!(key_action(key, ctrl, shift, place), KeyAction::Ignore);
                }
            }
        }
    }
```

Then add:

```rust
    /// The second button's shortcut. Shift is what tells it from Ctrl+Enter,
    /// which presses whichever button is the default.
    #[test]
    fn ctrl_shift_enter_refines_from_anywhere() {
        for place in EVERYWHERE {
            assert_eq!(key_action(gdk::Key::Return, true, true, place), KeyAction::Refine);
            assert_eq!(key_action(gdk::Key::KP_Enter, true, true, place), KeyAction::Refine);
        }
    }

    /// Shift alone changes nothing: Shift+Enter is still Enter, so a slip on
    /// Shift while typing never adds the task.
    #[test]
    fn shift_enter_without_ctrl_is_plain_enter() {
        assert_eq!(
            key_action(gdk::Key::Return, false, true, Some(Place::Description)),
            KeyAction::ToFirstNote
        );
        assert_eq!(
            key_action(gdk::Key::Return, false, true, Some(Place::Note { index: 2, empty: false })),
            KeyAction::NewNoteBelow(2)
        );
        assert_eq!(key_action(gdk::Key::Return, false, true, None), KeyAction::Ignore);
    }
```

- [ ] **Step 2: Run the tests to check they fail**

Run: `cargo test --lib taskbox::keys`
Expected: compile error: `key_action` takes 3 arguments and `KeyAction::Refine` does not exist.

- [ ] **Step 3: Implement it**

In `src/taskbox/keys.rs`, add the variant after `Save`:

```rust
pub enum KeyAction {
    /// Press the default button: Add or Save, or Add & refine in a box opened
    /// to refine.
    Save,
    /// Press Add & refine. Only add mode has that button; elsewhere the box
    /// treats this as Save, which is what Ctrl+Shift+Enter did before it.
    Refine,
    Cancel,
    // … the rest unchanged
```

Change `key_action`:

```rust
/// Map a keypress to what it should do. `place` is `None` when the focus is
/// on something other than a text field — a button.
pub fn key_action(key: gdk::Key, ctrl: bool, shift: bool, place: Option<Place>) -> KeyAction {
    let enter = matches!(key, gdk::Key::Return | gdk::Key::KP_Enter);
    match (key, place) {
        (gdk::Key::Escape, _) => KeyAction::Cancel,
        _ if enter && ctrl && shift => KeyAction::Refine,
        _ if enter && ctrl => KeyAction::Save,
        (_, Some(Place::Description)) if enter => KeyAction::ToFirstNote,
        (_, Some(Place::Note { index, .. })) if enter => KeyAction::NewNoteBelow(index),
        (gdk::Key::BackSpace, Some(Place::Note { index, empty: true })) => KeyAction::DeleteNote(index),
        _ => KeyAction::Ignore,
    }
}
```

In `src/taskbox.rs`'s key handler, read Shift and pass it on. Until Task 3 adds the button, treat `Refine` as `Save`:

```rust
            let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
            let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
            // GtkWindowExt and RootExt both have a `focus()`; either answers.
            let place = GtkWindowExt::focus(&window).and_then(|w| notes.place_of(&w));
            match keys::key_action(key, ctrl, shift, place) {
                KeyAction::Cancel => window.close(),
                KeyAction::Save | KeyAction::Refine => do_submit(),
```

- [ ] **Step 4: Run the tests to check they pass**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/taskbox/keys.rs src/taskbox.rs
git commit -m "Map Ctrl+Shift+Enter in the task box to its own key action

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The Add & refine button, and which button is the default

**Files:**
- Modify: `src/taskbox/form.rs` (`Submission`, `submission`, one test)
- Modify: `src/taskbox.rs` (`BoxConfig`, `BoxConfig::add`, `BoxConfig::for_task`, `build_window` footer/submit/keys, new `hint` fn, tests)
- Modify: `src/daemon.rs:222` and `src/main.rs:243` (callers of `BoxConfig::add` pass `false` for now)

**Interfaces:**
- Consumes: `KeyAction::Refine` and `key_action(key, ctrl, shift, place)` from Task 2.
- Produces:
  - `pub struct Submission { pub description: String, pub notes: Vec<NoteEdit>, pub refine: bool }`. `refine` is true when Add & refine was pressed.
  - `pub struct BoxConfig { pub mode, pub subtitle, pub description, pub notes, pub refine: bool }`, where `refine` means Ctrl+Enter presses Add & refine.
  - `pub fn BoxConfig::add(tag: &str, refine: bool) -> BoxConfig`
  - Task 4 uses all three.

- [ ] **Step 1: Write the failing tests**

In `src/taskbox/form.rs` tests, extend `note_texts_are_the_notes_in_order`:

```rust
    #[test]
    fn note_texts_are_the_notes_in_order() {
        let s = submission("", "d", &[row(None, "", "a"), row(None, "", "b")]).unwrap();
        assert_eq!(s.note_texts(), vec!["a".to_string(), "b".to_string()]);
        assert!(!s.refine, "what is on screen says nothing about which button was pressed");
    }
```

In `src/taskbox.rs` tests, add:

```rust
    /// Mod+Alt+Shift+T opens the same empty box; only the default button
    /// differs.
    #[test]
    fn an_add_box_says_which_button_ctrl_enter_presses() {
        let plain = BoxConfig::add("proj", false);
        let refining = BoxConfig::add("proj", true);
        assert_eq!((plain.mode, refining.mode), (Mode::Add, Mode::Add));
        assert_eq!(refining.subtitle, "+proj");
        assert!(!plain.refine);
        assert!(refining.refine);
    }

    #[test]
    fn an_existing_task_never_opens_to_refine() {
        let t: crate::task::Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert!(!BoxConfig::for_task(Mode::Edit, t).refine);
    }

    /// The hint names what Ctrl+Enter does in this box, and offers
    /// Ctrl+Shift+Enter only where it does something different.
    #[test]
    fn the_hint_names_the_default_button() {
        assert_eq!(
            hint(Mode::Add, false),
            "Enter: next note · Ctrl+Enter: add · Ctrl+Shift+Enter: add & refine · Esc: discard"
        );
        assert_eq!(
            hint(Mode::Add, true),
            "Enter: next note · Ctrl+Enter: add & refine · Esc: discard"
        );
        for mode in [Mode::Edit, Mode::Note] {
            assert_eq!(hint(mode, false), "Enter: next note · Ctrl+Enter: save · Esc: discard");
        }
    }
```

- [ ] **Step 2: Run the tests to check they fail**

Run: `cargo test --lib taskbox`
Expected: compile errors: no field `refine`, `BoxConfig::add` takes 1 argument, no function `hint`.

- [ ] **Step 3: Add `refine` to `Submission`**

In `src/taskbox/form.rs`:

```rust
/// What the box hands back when it is saved.
pub struct Submission {
    pub description: String,
    /// In the order the rows are on screen, empty rows left out.
    pub notes: Vec<NoteEdit>,
    /// Add & refine was pressed: once added, the task goes straight to
    /// `niritasks task refine`. Set by the button, not by what is on screen,
    /// so [`submission`] always leaves it false.
    pub refine: bool,
}
```

At the end of `submission(…)`: `Some(Submission { description, notes, refine: false })`.

- [ ] **Step 4: Add `refine` to `BoxConfig`, plus the `hint` function**

In `src/taskbox.rs`:

```rust
pub struct BoxConfig {
    pub mode: Mode,
    /// A dim line above the description — the tag a new task goes to. Hidden
    /// when empty.
    pub subtitle: String,
    pub description: String,
    /// The task's notes, shown as stored and in stored order.
    pub notes: Vec<Annotation>,
    /// Ctrl+Enter presses Add & refine rather than Add — the box
    /// Mod+Alt+Shift+T opens. Only add mode has that button, so it is false
    /// for an existing task.
    pub refine: bool,
}

impl BoxConfig {
    /// An empty box for a new task on `tag`, with Add & refine as the default
    /// button when `refine` is set.
    pub fn add(tag: &str, refine: bool) -> Self {
        Self {
            mode: Mode::Add,
            subtitle: format!("+{tag}"),
            description: String::new(),
            notes: Vec::new(),
            refine,
        }
    }
```

`for_task` gets `refine: false,`.

Add `hint` below `impl Mode`. It is a free function rather than a `Mode` method because it also depends on `refine`:

```rust
/// The line of keys beside the buttons. It names what Ctrl+Enter presses in
/// this box, and offers Ctrl+Shift+Enter only where that is a different
/// button — in a box opened to refine, both refine.
fn hint(mode: Mode, refine: bool) -> &'static str {
    match (mode, refine) {
        (Mode::Add, false) => {
            "Enter: next note · Ctrl+Enter: add · Ctrl+Shift+Enter: add & refine · Esc: discard"
        }
        (Mode::Add, true) => "Enter: next note · Ctrl+Enter: add & refine · Esc: discard",
        (Mode::Edit | Mode::Note, _) => "Enter: next note · Ctrl+Enter: save · Esc: discard",
    }
}
```

Fix the two callers so the crate builds. In `src/daemon.rs`: `taskbox::BoxConfig::add(&tag, false)`. In `src/main.rs`: `taskbox::show(taskbox::BoxConfig::add(&tag, false))`. Task 4 replaces both `false`s.

- [ ] **Step 5: Build the button and wire the submit**

In `build_window`, replace the footer block:

```rust
    // ─── footer ───────────────────────────────────────────────────────────
    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let hint_label = gtk4::Label::new(Some(hint(cfg.mode, cfg.refine)));
    hint_label.add_css_class("dim");
    hint_label.set_xalign(0.0);
    hint_label.set_hexpand(true);
    // The add hint is longer than the room beside three buttons; wrapping
    // keeps it whole rather than pushing a fixed-size window wider.
    hint_label.set_wrap(true);
    let cancel = gtk4::Button::with_label("Cancel");
    let submit = gtk4::Button::with_label(cfg.mode.submit_label());
    footer.append(&hint_label);
    footer.append(&cancel);
    footer.append(&submit);
    // Add mode only: refining from Edit or Note is the menu's job.
    let refine = (cfg.mode == Mode::Add).then(|| gtk4::Button::with_label("Add & refine"));
    if let Some(refine) = &refine {
        footer.append(refine);
    }
    root.append(&footer);
```

The label is `hint_label` so it does not shadow the `hint` function.

Make `do_submit` take which button was pressed:

```rust
    let do_submit = {
        let loaded_description = cfg.description.clone();
        let window = window.downgrade();
        let notes = notes.clone();
        move |refine: bool| {
            let submission = form::submission(
                &loaded_description,
                &buffer_text(&notes.description),
                &notes.rows(),
            );
            // An empty description saves nothing, and closing anyway would
            // throw the note edits away without a word: stay open, with the
            // cursor where the fix is. Esc and Cancel are what discard.
            let Some(mut submission) = submission else {
                focus_end(&notes.description);
                return;
            };
            submission.refine = refine;
            if let Some(window) = window.upgrade() {
                window.close();
            }
            on_submit(submission);
        }
    };

    {
        let do_submit = do_submit.clone();
        submit.connect_clicked(move |_| do_submit(false));
    }
    if let Some(refine) = &refine {
        let do_submit = do_submit.clone();
        refine.connect_clicked(move |_| do_submit(true));
    }
```

In the key handler, capture the two booleans before the closure and replace the Task 2 placeholder arm:

```rust
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let window = window.downgrade();
        let notes = notes.clone();
        // Which button each shortcut presses. Ctrl+Enter presses the default;
        // Ctrl+Shift+Enter presses Add & refine where there is one, and saves
        // where there is not, as it always did.
        let default_refines = cfg.refine;
        let can_refine = cfg.mode == Mode::Add;
        keys.connect_key_pressed(move |_, key, _, state| {
            // … unchanged down to the match …
            match keys::key_action(key, ctrl, shift, place) {
                KeyAction::Cancel => window.close(),
                KeyAction::Save => do_submit(default_refines),
                KeyAction::Refine => do_submit(can_refine),
                // … the rest unchanged
```

(`do_submit` is moved into this closure as it is today. The two `clone()`s above came first, so that still works.)

- [ ] **Step 6: Run the tests and build**

Run: `cargo test && cargo build`
Expected: PASS, no warnings.

- [ ] **Step 7: Look at it**

Run: `cargo run -- task add` on a named workspace with the daemon stopped for the moment, or with `XDG_RUNTIME_DIR` pointed at an empty dir so no daemon is found. Expected: buttons Cancel / Add / Add & refine, with the hint wrapped on the left and readable. Press Esc. Then `cargo run -- task edit <some uuid>` should show only Cancel / Save and the save hint. Neither run writes anything if you press Esc.

- [ ] **Step 8: Commit**

```bash
git add src/taskbox.rs src/taskbox/form.rs src/daemon.rs src/main.rs
git commit -m "Give the add box an Add & refine button and say which button Ctrl+Enter presses

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Refine after the add, from the daemon and the CLI

**Files:**
- Modify: `src/refine.rs` (new `quick_command`, `spawn_quick`, tests)
- Modify: `src/ipc.rs` (`Request::Add { refine }`, module doc, tests)
- Modify: `src/daemon.rs:215-238` (`Request::Add` arm)
- Modify: `src/main.rs` (`TaskCommand::Add`, its arm at 238-258)

**Interfaces:**
- Consumes: `task::add_with_notes(…) -> Result<Option<String>>` (Task 1); `Submission.refine`, `BoxConfig::add(tag, refine)` (Task 3).
- Produces:
  - `pub fn refine::quick_command(exe: &str, uuid: &str) -> Vec<String>`
  - `pub fn refine::spawn_quick(uuid: &str)`
  - `ipc::Request::Add { refine: bool }`, encoded `add` / `add refine`
  - CLI `niritasks task add [--refine] [TEXT]...`

- [ ] **Step 1: Write the failing tests**

In `src/refine.rs`'s `mod tests`, add:

```rust
    /// Add & refine runs exactly what the panel's Refine button runs, so the
    /// two cannot drift apart.
    #[test]
    fn quick_command_is_the_refine_buttons_command() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        assert_eq!(
            quick_command("/usr/bin/niritasks", u),
            ["/usr/bin/niritasks", "task", "refine", u]
        );
    }
```

In `src/ipc.rs` tests, replace `Request::Add` in `round_trips_every_request` with both forms, and add the wire-format test:

```rust
        for req in [
            Request::Add { refine: false },
            Request::Add { refine: true },
            Request::Edit("abc-123".into()),
            Request::Note("def-456".into()),
            Request::Panel,
        ] {
```

```rust
    /// `add` alone stays a plain add, so a box asked for by an older CLI
    /// opens as it always did.
    #[test]
    fn add_carries_whether_to_refine() {
        assert_eq!(Request::Add { refine: false }.encode(), "add");
        assert_eq!(Request::Add { refine: true }.encode(), "add refine");
        assert_eq!(Request::decode("add\n"), Some(Request::Add { refine: false }));
        assert_eq!(Request::decode("add refine\n"), Some(Request::Add { refine: true }));
        assert_eq!(Request::decode("add sideways"), Some(Request::Add { refine: false }));
    }
```

In `tolerates_the_trailing_newline_writeln_adds`, change `Some(Request::Add)` to `Some(Request::Add { refine: false })`.

- [ ] **Step 2: Run the tests to check they fail**

Run: `cargo test --lib refine ipc`
(If cargo rejects two filters, run `cargo test --lib refine` and then `cargo test --lib ipc`.)
Expected: compile errors: no `quick_command`, and `Request::Add` has no field `refine`.

- [ ] **Step 3: Implement `quick_command` and `spawn_quick` in `src/refine.rs`**

Put these after `launch`:

```rust
/// The command the panel's Refine button runs on `uuid`, with `exe` as the
/// `niritasks` binary — built from the button's own arguments, so the two
/// cannot drift apart.
pub fn quick_command(exe: &str, uuid: &str) -> Vec<String> {
    let mut command = vec![exe.to_string()];
    command.extend(crate::panel::actions::Action::Refine.args(uuid));
    command
}

/// Hand a task just added to the refine-task skill — Add & refine.
///
/// In a `niritasks task refine` process of its own, spawned by niri as the
/// panel's Refine button is: the daemon's GTK loop must never wait on herdr,
/// and a child of `niritasks task add` would inherit the `flock` the
/// Mod+Alt+T binds hold and keep it for as long as Claude's terminal stayed
/// open. That process reports its own failures; this only reports failing to
/// start it. Either way the task is already added.
pub fn spawn_quick(uuid: &str) {
    let result = std::env::current_exe()
        .map_err(anyhow::Error::from)
        .and_then(|exe| niri::spawn(quick_command(&exe.to_string_lossy(), uuid)));
    if let Err(e) = result {
        notify::tasks(&format!("Added, but could not start refining it: {e}"));
    }
}
```

(`niri` and `notify` are already imported at the top of `refine.rs`.)

- [ ] **Step 4: Carry `refine` over IPC in `src/ipc.rs`**

Update the module doc's protocol block:

```text
//! add
//! add refine
//! edit <uuid>
//! note <uuid>
//! panel
```

Change the enum and the codec:

```rust
pub enum Request {
    /// The add box; `refine` makes Add & refine its default button.
    Add { refine: bool },
    Edit(String),
    Note(String),
    /// Slide out the focused monitor's task panel and give it the keyboard.
    Panel,
}

impl Request {
    pub fn encode(&self) -> String {
        match self {
            Request::Add { refine: false } => "add".into(),
            Request::Add { refine: true } => "add refine".into(),
            Request::Edit(uuid) => format!("edit {uuid}"),
            Request::Note(uuid) => format!("note {uuid}"),
            Request::Panel => "panel".into(),
        }
    }

    pub fn decode(line: &str) -> Option<Self> {
        let mut parts = line.trim().splitn(2, ' ');
        match (parts.next()?, parts.next()) {
            ("add", rest) => Some(Request::Add { refine: rest == Some("refine") }),
            ("panel", _) => Some(Request::Panel),
            ("edit", Some(uuid)) if !uuid.is_empty() => Some(Request::Edit(uuid.to_string())),
            ("note", Some(uuid)) if !uuid.is_empty() => Some(Request::Note(uuid.to_string())),
            _ => None,
        }
    }
}
```

- [ ] **Step 5: Wire the daemon (`src/daemon.rs`)**

Replace the `Request::Add` arm in `serve_box_request`:

```rust
        Request::Add { refine } => {
            let tag = match crate::require_workspace_tag() {
                Ok(t) => t,
                Err(e) => {
                    notify::tasks(&e.to_string());
                    return;
                }
            };
            let tag_for_submit = tag.clone();
            taskbox::open_in(
                app,
                taskbox::BoxConfig::add(&tag, refine),
                move |sub: taskbox::Submission| {
                    match task::add_with_notes(
                        &tag_for_submit,
                        &text::add_args(&sub.description),
                        &sub.note_texts(),
                    ) {
                        Err(e) => notify::tasks(&e.to_string()),
                        Ok(uuid) => {
                            notify::tasks(&format!("Added to +{tag_for_submit}: {}", sub.description));
                            // Spawned, not run: herdr must not hold up the
                            // panels' main loop.
                            if let (true, Some(uuid)) = (sub.refine, uuid) {
                                crate::refine::spawn_quick(&uuid);
                            }
                        }
                    }
                },
            );
        }
```

- [ ] **Step 6: Wire the CLI (`src/main.rs`)**

In `TaskCommand`, change `Add`:

```rust
    /// Add a task
    Add {
        /// Then refine it straight away with Claude, as the box's Add & refine
        /// does; with no text, the box opens with that as its default (Mod+Alt+Shift+T)
        #[arg(long)]
        refine: bool,
        text: Vec<String>,
    },
```

Replace the `TaskCommand::Add` arm:

```rust
        // With no text, open the box. With text, add straight away — which is
        // what makes `niritasks task add ship it due:friday` work from a shell.
        // The binding is `and_refine` because `refine` is the module, imported
        // at the top of this file.
        TaskCommand::Add { text: words, refine: and_refine } => {
            let tag = require_workspace_tag()?;
            // The box can return notes with the description; the shell form has
            // nowhere to type them, so it never does. Which button was pressed
            // decides whether to refine, not how the box was opened.
            let (description, notes, and_refine) = if words.is_empty() {
                if delegate_to_daemon(ipc::Request::Add { refine: and_refine }) {
                    return Ok(());
                }
                match taskbox::show(taskbox::BoxConfig::add(&tag, and_refine)) {
                    Some(s) => {
                        let notes = s.note_texts();
                        (s.description, notes, s.refine)
                    }
                    None => return Ok(()),
                }
            } else {
                (text::collapse_whitespace(&words.join(" ")), Vec::new(), and_refine)
            };

            if description.is_empty() {
                return Ok(());
            }
            let uuid = task::add_with_notes(&tag, &text::add_args(&description), &notes)?;
            notify::tasks(&format!("Added to +{tag}: {description}"));
            if let (true, Some(uuid)) = (and_refine, uuid) {
                refine::spawn_quick(&uuid);
            }
        }
```

(A pattern binding with the same name as a module would not actually break the path `refine::spawn_quick`, since paths and values live in different namespaces. The rename is there for whoever reads the code next.)

- [ ] **Step 7: Run the tests and build**

Run: `cargo test && cargo build`
Expected: PASS, no warnings. Also check the flag parses: `cargo run -- task add --help` lists `--refine`.

- [ ] **Step 8: Commit**

```bash
git add src/refine.rs src/ipc.rs src/daemon.rs src/main.rs
git commit -m "Hand a task to refine once Add & refine has added it, from the daemon and the CLI

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: End-to-end check in the nested niri

**Files:**
- Modify: `tests/lib/nested-niri.sh` (`nested_start`'s kdl heredoc, around line 183, plus a note near the top where the other knobs are described)
- Modify: `tests/e2e-box.sh` (header comment, new `run_refine_suite`, calls at the bottom)

**Interfaces:**
- Consumes: the CLI and daemon from Task 4; the box shortcuts from Tasks 2–3.
- Produces: `NESTED_SPAWN_PATH`, an opt-in knob in `nested-niri.sh`. When it is set, programs the nested niri spawns get that `PATH` and `NOTIFY_LOG="$SB/notifications"`.

- [ ] **Step 1: Give the nested niri an opt-in spawn environment**

In `nested_start`, just before `cat > "$SB/niri.kdl" <<EOF`, add:

```bash
    # What niri's own spawns run with — a refine Add & refine starts, say. Off
    # unless a test asks: by default the nested niri's children see your PATH,
    # as before. When set, PATH is this and notifications go to the stub's log
    # rather than your desktop.
    local spawn_env=""
    if [ -n "${NESTED_SPAWN_PATH:-}" ]; then
        spawn_env="environment { PATH \"$NESTED_SPAWN_PATH\"; NOTIFY_LOG \"$SB/notifications\"; }"
    fi
```

Inside the heredoc, add a line `$spawn_env` after `workspace "e2e"`. An empty line is fine in KDL. Next to the `NIRITASKS=` default near line 36, add a one-line comment documenting `NESTED_SPAWN_PATH`.

- [ ] **Step 2: Write the e2e suite**

In `tests/e2e-box.sh`, after sourcing the lib (it defines `$SB`), set the knob. Its PATH holds the notify-send stub and the system dirs, and leaves out wherever herdr is installed:

```bash
# A refine spawned by Add & refine runs in the nested niri with this PATH: the
# notify-send stub, the system tools, and no herdr — so it stops at "herdr is
# not installed." and the stub logs that, which proves a refine was started on
# the new task without opening anything in your real herdr.
NESTED_SPAWN_PATH="$SB/bin:/usr/bin:/bin"
```

Add helpers next to `pending()`:

```bash
# How many refines Add & refine has started — each stops at the missing herdr.
refine_tries() { cat "$SB/notifications" 2>/dev/null | grep -c "herdr is not installed"; }
# Wait up to 5s for refine_tries to reach $1.
wait_refine_tries() {
    for _ in $(seq 1 50); do
        [ "$(refine_tries)" -ge "$1" ] && return 0
        sleep 0.1
    done
    return 1
}
```

Add the suite before `nested_start`:

```bash
run_refine_suite() {
    local label="$1" before mark uuid
    echo
    echo "=== $label: Add & refine ==="

    if PATH="$NESTED_SPAWN_PATH" command -v herdr >/dev/null; then
        echo "  skip: herdr is in $NESTED_SPAWN_PATH, so a refine here would really open"
        return
    fi

    # Ctrl+Shift+Enter in the plain add box presses Add & refine.
    mark="refine-$RANDOM"; before=$(refine_tries)
    if open_box add; then
        nested wtype "$mark"; sleep 0.4
        nested wtype -M ctrl -M shift -k Return -m shift -m ctrl
        sleep 1.5
        uuid=$(uuid_of "$mark")
        [ -n "$uuid" ] && ok "Ctrl+Shift+Enter added the task" \
            || bad "Ctrl+Shift+Enter wrote no task"
        wait_refine_tries $((before + 1)) && ok "and started refining it" \
            || bad "no refine was started (notifications: $(tail -n 3 "$SB/notifications" 2>/dev/null | tr '\n' '|'))"
        [ -n "$(uuid_of "$mark")" ] && ok "a failed refine left the task added" \
            || bad "the task went missing after refine failed"
    else
        bad "add box never opened"
    fi

    # `task add --refine` makes Add & refine the default: Ctrl+Enter presses it.
    mark="refine-$RANDOM"; before=$(refine_tries)
    if open_box add --refine; then
        [ "$(box_title)" = "Add Task" ] && ok "--refine opens the add box" \
            || bad "--refine box title was \"$(box_title)\""
        nested wtype "$mark"; sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        [ -n "$(uuid_of "$mark")" ] && ok "Ctrl+Enter in a --refine box added the task" \
            || bad "Ctrl+Enter in a --refine box wrote no task"
        wait_refine_tries $((before + 1)) && ok "and started refining it" \
            || bad "Ctrl+Enter in a --refine box started no refine"
    else
        bad "--refine box never opened"
    fi

    # Plain Ctrl+Enter in the plain box adds and does not refine.
    mark="plain-$RANDOM"; before=$(refine_tries)
    if open_box add; then
        nested wtype "$mark"; sleep 0.4
        nested wtype -M ctrl -k Return -m ctrl
        sleep 3
        [ -n "$(uuid_of "$mark")" ] && ok "plain Ctrl+Enter added the task" \
            || bad "plain Ctrl+Enter wrote no task"
        [ "$(refine_tries)" -eq "$before" ] && ok "and did not refine it" \
            || bad "plain Ctrl+Enter started a refine"
    else
        bad "add box never opened"
    fi

    close_any_box
    guard
}
```

Call it in both halves at the bottom:

```bash
nested_daemon_start "$SB/daemon.err"
run_suite "served by the daemon"
run_refine_suite "served by the daemon"
nested_daemon_stop
sleep 1

run_suite "fallback, no daemon running"
run_refine_suite "fallback, no daemon running"
```

Add a line to the header comment's coverage paragraph: "…and Add & refine: Ctrl+Shift+Enter, and `task add --refine`'s Ctrl+Enter, add the task and start a refine on it, which is let to fail on a missing herdr so nothing opens in yours."

- [ ] **Step 3: Check the suite would catch a regression**

Temporarily change `KeyAction::Refine => do_submit(can_refine)` in `src/taskbox.rs` to `do_submit(false)`, then run `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-box.sh`.
Expected: the "Ctrl+Shift+Enter … started refining it" checks FAIL in both halves. Revert the change.

- [ ] **Step 4: Run it for real**

Run: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-box.sh`
Expected: every check `ok`, in both halves, including the existing suite. If the refine checks fail with a different notification, such as "Refine would leave these sockets…", the refine *was* spawned. Read the logged notification, and check whether `ensure_no_exposed_sockets` sees a socket from the nested run. Do not loosen the assertion until you know why.

- [ ] **Step 5: Commit**

```bash
git add tests/lib/nested-niri.sh tests/e2e-box.sh
git commit -m "Check Add & refine end to end in the nested niri, from the daemon and the fallback

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The keybind and the docs

**Files:**
- Modify: `niri/niri-tasks.kdl:36-80`
- Modify: `README.md` (keybind table ~line 18, CLI list ~line 163, "The task box" section ~lines 190-210)
- Modify: `CONTEXT.md` (**Task box** entry ~line 80)

- [ ] **Step 1: Add the bind**

In `niri/niri-tasks.kdl`, after the `Mod+Alt+T` bind:

```kdl
    // Mod+Alt+Shift+T is the same box with Add & refine as the default: once
    // added, the task opens in a "Refine: …" tab of the workspace's herdr
    // session. Same lock as Mod+Alt+T, so the two share one box at a time.
    Mod+Alt+Shift+T hotkey-overlay-title="Add Task and Refine" { spawn-sh "flock -n \"${XDG_RUNTIME_DIR:-/tmp}/niritasks-taskbox.lock\" niritasks task add --refine"; }
```

The lock comment above `binds` says the box "spawns nothing that outlives it". Amend it, keeping the voice:

```kdl
// The task box is the exception, and the reason any of this exists: it is the
// GTK window in src/taskbox.rs, it has no lock, and fuzzel's lock does not know
// it exists, so pressing Mod+Alt+T twice stacked two boxes. It spawns nothing
// that outlives it — Add & refine's refine goes through niri's spawn, not as
// its child — which is what makes holding a lock across it safe where the
// same trick was not safe for the pickers.
```

Check that it parses: `niri validate -c niri/niri-tasks.kdl`. If `validate` rejects a file that is only binds, check the user config that includes it instead: `niri validate` with no `-c`, after `install.sh` has linked it, or by eye against the Mod+Alt+T line.

- [ ] **Step 2: README**

Keybind table, below the `Mod+Alt+T` row:

```markdown
| `Mod+Alt+Shift+T` | The same, with Add & refine as the default: once added, the task opens in a "Refine: …" tab of the workspace's herdr session, where Claude works it up into a plan |
```

CLI list, after the `task add <text>` line:

```
niritasks task add --refine        # the add box, with Add & refine as its default (Mod+Alt+Shift+T)
                                   #   (with text, adds it and refines it straight away)
```

In "The task box", add a sentence to the opening paragraph: "Adding has a second button, **Add & refine**: it adds the task and hands it straight to Claude's refine-task skill, in a new tab of the workspace's herdr session, as the menu's Refine does. If the refine fails, the task stays added and a notification says why." In the key table, after the Ctrl+Enter row:

```markdown
| Ctrl+Shift+Enter | Add & refine, when adding |
```

Change the Ctrl+Enter row to `| Ctrl+Enter | Saves, from anywhere — Add & refine in the box Mod+Alt+Shift+T opens |`.

- [ ] **Step 3: CONTEXT.md**

Replace the **Task box** entry:

```markdown
**Task box**:
The GTK window for adding a task or editing one — its description and its
note rows. Edit and Note open the same box. When adding, a second button,
Add & refine (Ctrl+Shift+Enter), adds the task and refines it straight away;
Mod+Alt+Shift+T opens the box with that as the default.
_Avoid_: dialog, prompt
```

- [ ] **Step 4: Full verification**

Run: `cargo test && bash tests/all.sh`
Expected: `cargo test` PASS. `all.sh` reports every suite that can run here as passing. Report SKIPs as SKIPs, not as passes.

- [ ] **Step 5: Commit**

```bash
git add niri/niri-tasks.kdl README.md CONTEXT.md
git commit -m "Bind Mod+Alt+Shift+T to the add box with Add & refine as its default, and document both

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 6: Manual check on the real desktop (the spec's "Done when")**

This needs the user's session, so ask the user before installing:

1. `bash install.sh`. It runs `cargo install` and restarts the daemon. Then `niri msg action load-config-file`, or rely on niri's auto-reload, so the new bind loads.
2. On a project workspace, press Mod+Alt+T, type a description, and click **Add & refine**. A "Refine: …" tab should open in the workspace's herdr session for the new task.
3. Mod+Alt+T, then Ctrl+Shift+Enter: the same.
4. Mod+Alt+Shift+T, then Ctrl+Enter: the same. Open it again and click **Add**: the task is added with no refine.
5. Repeat 2–4 with the daemon stopped (`systemctl --user stop niri-tasks`), then start it again.
6. Delete the test tasks with `niritasks task status <uuid> deleted --yes`.
