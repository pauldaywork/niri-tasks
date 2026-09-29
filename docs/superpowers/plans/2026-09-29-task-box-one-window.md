# Task Box: One Window, Editable Notes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One task box for Add, Edit and Note, whose notes are rows you can read, edit, delete and add to. Saving an existing task replaces its description and whole note list in a single `task import`.

**Architecture:** A new `task::replace_text` re-exports the task when you save, swaps in the description and annotations, and imports it back. It writes nothing when nothing changed. The box's pure logic goes into three new submodules of `src/taskbox.rs`: `keys` (keypress → action, by where the focus is), `form` (what is on screen → what to save) and `style` (CSS built from the task panel's constants). `build_window` is rebuilt around a list of note rows. `main.rs` and `daemon.rs` send Edit and Note to the same window.

**Tech Stack:** Rust 2021, gtk4-rs 0.11 (GTK 4.22 on this machine), serde_json, Taskwarrior 2.6.2, bash + wtype + niri for the e2e script.

**Spec:** Taskwarrior task `8394d91b-6edf-4c2d-901f-0eaa0967c01b`. Read it with `task rc.json.array=on 8394d91b-6edf-4c2d-901f-0eaa0967c01b export`; its description and notes are the spec. The "Decided:" notes are copied into the constraints below.

## Global Constraints

- Add, Edit and Note share one window. Add opens it empty, Edit opens it with the description and notes, and Note opens it with the cursor in a new empty note row.
- Notes are rows, one per note. Each row wraps, is editable in place and has an × that deletes it. A "+ Add note" button appends a row. Text is shown as stored, in stored order, and Goal:/Decided: prefixes get no special treatment.
- Saving an existing task replaces its description and full note list in one `task import` (export, replace description and annotations, import). Existing notes keep their original entry date, new notes are dated now, and a save that changed nothing writes nothing.
- Add still word-splits the description, so `due:`/`priority:` parse, and still files notes through `task::add_with_notes`. Edit keeps the description literal.
- Keys: Enter in the description moves to the first note. Enter in a note adds a row below and moves to it. Backspace in an empty row deletes it and moves up. Ctrl+Enter saves from anywhere. Esc discards everything without asking.
- Each note row shows its date, small and dim, at its right end. A new row shows none.
- The box is styled like the task panel: the terminal's font and colours, the same tint and outline (`src/panel/style.rs`), not the DMS theme from `theme.rs`.
- The window is a fixed 800×760 (equal min and max, so niri still floats it without a window rule). The description area is about three lines tall, and the notes list scrolls.
- Out of scope: reordering notes, grouping notes by prefix, a resizable window, and the fuzzel prompts.
- House style: every public item gets a doc comment that says *why*, in the voice of the surrounding code. Errors surface by returning `Err`; `main` already notifies.

## Facts established while planning (Taskwarrior 2.6.2, checked in a sandbox)

- `task import -` on an existing uuid **replaces the whole task**. Any field missing from the JSON is dropped: a payload of just uuid and description lost the task's tags. So the save must re-export the full task *at save time* and change only `description` and `annotations`.
- Annotations in the imported JSON **replace** the old list completely. A missing `annotations` key removes every note.
- An annotation with no `entry` is dated now. Colliding stamps are bumped forward one second each, in array order, so several new notes stay distinct and keep their order. `task annotate` three times in one second bumps the same way.
- Taskwarrior stores annotations keyed by entry stamp, so **stored order is date order**. A new note typed between two old ones therefore moves to the end once saved. That follows from "new notes are dated now" and needs no reordering feature (out of scope). The README says so.
- An import that changes nothing prints `skip` and adds nothing to `undo.data`. The description stays literal (`due:friday` in it does not set a due date).
- Import exits non-zero on malformed JSON.

## Decisions made while planning (flag any you disagree with)

1. **Tint and outline.** The cards fill with `rgba(0,0,0,0.375)` and draw a 4px `#00000020` ring because they are layer surfaces, which niri doesn't dress. The box is a window, and both of the user's window-rule profiles (`~/.config/niri/window-rules/{normal,focus}.kdl`) already give every window `opacity 0.75`, blur and that same 4px focus ring. So the box fills with ghostty's own `rgba(0,0,0,0.5)` (new `panel::style::WINDOW_BACKGROUND`) and lets niri add the rest, exactly as it does for a terminal. On screen that comes out the same as a card: 0.5 × 0.75 = 0.375. It draws no outline of its own.
2. Fields (the description and each note row) get a faint `rgba(255,255,255,0.06)` lift and the card's 1px white focus outline when the cursor is in them.
3. Edit and Note both use the title "Edit Task", because it is one window. Add stays "Add Task". Edit shows no subtitle line; Add keeps `+tag`.
4. The Cancel and Save buttons stay, so the box can still be used with the mouse. The hint line reads `Enter: next note · Ctrl+Enter: save · Esc: discard`.
5. Whitespace: text left exactly as loaded is saved verbatim. Text that was touched is collapsed with `text::collapse_whitespace`, as today. That way opening and saving an untouched task never counts as a change. Empty rows are dropped.
6. Known limitation, not fixed: a note that someone else adds to the task while the box is open (a refine session, say) is dropped by the save, since the box's list replaces the whole list. This is a single-user desktop tool and the spec asks for full replacement.
7. `theme.rs`, `text::note_lines` and `Task::notes_list` lose their last callers and are deleted, along with their tests. `task edit <uuid> <text>` and `task note <uuid> <text>` (the no-box CLI forms) are unchanged.

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `src/task.rs` | modify | `NoteEdit`, `replace_text`, pure `with_text`; `export` split so a raw `Value` export exists; `Annotation::date` made `pub`; `notes_list` removed |
| `tests/write_path.rs` | modify | `replace_text` against the sandboxed database |
| `src/taskbox/keys.rs` | create | `Place`, `KeyAction`, `key_action` (pure) |
| `src/taskbox/form.rs` | create | `Row`, `Submission`, `cleaned`, `submission` (pure) |
| `src/taskbox/style.rs` | create | The box's CSS, from `panel::style` constants |
| `src/panel/style.rs` | modify | Add `WINDOW_BACKGROUND` |
| `src/taskbox.rs` | rewrite | `Mode`, `BoxConfig`, the note-row window, `show`/`open_in` |
| `src/main.rs`, `src/daemon.rs` | modify | Route Add/Edit/Note to the new box |
| `src/theme.rs`, `src/lib.rs`, `src/text.rs` | delete / modify | Drop the DMS theme and `note_lines` |
| `tests/e2e-box.sh`, `README.md`, `CONTEXT.md` | modify | Drive the new keys; document them |

---

### Task 1: Replace a task's description and notes in one import

**Files:**
- Modify: `src/task.rs` (the `export` fn ~l.111–128; new items after `annotate` ~l.284; tests module)
- Test: `tests/write_path.rs` (new section after "add with notes", before "empty input is a no-op" ~l.162)

**Interfaces:**
- Produces:
  - `pub struct NoteEdit { pub entry: Option<String>, pub text: String }` (derive `Debug, Clone, PartialEq, Eq`)
  - `pub fn replace_text(uuid: &str, description: &str, notes: &[NoteEdit]) -> anyhow::Result<bool>`. `Ok(true)` means it wrote, `Ok(false)` means there was nothing to write (unchanged, or an empty description).
  - private `fn with_text(task: serde_json::Value, description: &str, notes: &[NoteEdit]) -> Option<serde_json::Value>`. `None` means unchanged.
  - private `fn export_values(filter: &[&str]) -> Result<Vec<serde_json::Value>>`

- [ ] **Step 1: Write the failing unit tests for `with_text`** (append inside `mod tests` in `src/task.rs`)

```rust
    fn exported() -> serde_json::Value {
        serde_json::json!({
            "uuid": "u", "description": "old", "status": "pending", "tags": ["x"],
            "annotations": [
                {"entry": "20260801T000000Z", "description": "one"},
                {"entry": "20260802T000000Z", "description": "two"}
            ]
        })
    }

    fn kept(entry: &str, text: &str) -> NoteEdit {
        NoteEdit { entry: Some(entry.into()), text: text.into() }
    }

    /// `task import` replaces the whole task, so everything but the two
    /// fields the box edits has to go back exactly as it came out.
    #[test]
    fn with_text_changes_only_the_description_and_notes() {
        let out = with_text(
            exported(),
            "new",
            &[kept("20260802T000000Z", "two, edited"), NoteEdit { entry: None, text: "three".into() }],
        )
        .expect("changed");
        assert_eq!(out["description"], "new");
        assert_eq!(out["tags"], serde_json::json!(["x"]));
        assert_eq!(out["status"], "pending");
        assert_eq!(
            out["annotations"],
            serde_json::json!([
                {"entry": "20260802T000000Z", "description": "two, edited"},
                {"description": "three"}
            ]),
            "a kept note keeps its stamp; a new one has none, so taskwarrior dates it now"
        );
    }

    #[test]
    fn with_text_is_none_when_nothing_changed() {
        let same = [kept("20260801T000000Z", "one"), kept("20260802T000000Z", "two")];
        assert!(with_text(exported(), "old", &same).is_none());
    }

    /// A task with no notes exports no `annotations` key at all, so "no notes
    /// before, none now" must compare equal rather than `[]` vs missing.
    #[test]
    fn with_text_treats_a_missing_notes_key_as_no_notes() {
        let bare = serde_json::json!({"uuid": "u", "description": "old"});
        assert!(with_text(bare, "old", &[]).is_none());
    }

    /// Deleting every note has to remove the key: import keeps whatever
    /// list it is given, and an empty one is not what taskwarrior exports.
    #[test]
    fn with_text_drops_the_notes_key_when_every_note_is_deleted() {
        let out = with_text(exported(), "old", &[]).expect("changed");
        assert!(out.get("annotations").is_none());
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib task::tests::with_text`
Expected: compile error, `cannot find type NoteEdit` / `cannot find function with_text`.

- [ ] **Step 3: Implement.** In `src/task.rs`:

Change the imports at the top to:

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
```

Replace the body of `export` (keep its doc comment) with a raw export plus a parse:

```rust
/// Run a `task export` filter and parse the result.
///
/// `export` exits non-zero when the filter matches nothing, which is a normal
/// case rather than an error — an empty list is returned instead.
fn export(filter: &[&str]) -> Result<Vec<Task>> {
    export_values(filter)?
        .into_iter()
        .map(|v| serde_json::from_value(v).context("could not parse a task from `task export`"))
        .collect()
}

/// [`export`] without the parse: each task exactly as taskwarrior wrote it.
/// [`replace_text`] needs this form, because `task import` drops any field it
/// is not handed back, and [`Task`] only models the fields this tool reads.
fn export_values(filter: &[&str]) -> Result<Vec<Value>> {
    let out = base()
        .args(filter)
        .arg("export")
        .output()
        .context("could not run `task` — is taskwarrior installed?")?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(trimmed).context("could not parse `task export` output as JSON")
}
```

Add after `annotate`:

```rust
/// One note as the task box hands it back.
///
/// `entry` is the stamp of the note it was loaded from, which is what lets an
/// edited note keep the date it was first written. A note typed in the box has
/// none, and taskwarrior dates it when it is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEdit {
    pub entry: Option<String>,
    pub text: String,
}

/// Replace a task's description and its whole list of notes, in one write.
///
/// Taskwarrior has no command to edit or delete one annotation by position,
/// but `task import` of a uuid that already exists replaces that task. So this
/// exports the task as it stands *now* — not as it was when the box opened,
/// because import drops any field it is not given, and a stale copy would undo
/// whatever changed in between — swaps in the two fields the box edits, and
/// imports it back.
///
/// The description goes through as JSON, so it stays literal the way
/// [`modify_description`] keeps it: a typed `due:` is text, not a due date.
///
/// Returns whether anything was written. Nothing is when the description is
/// empty — a task has to be called something — or when nothing changed, so
/// opening the box and saving leaves no trace in the undo log.
pub fn replace_text(uuid: &str, description: &str, notes: &[NoteEdit]) -> Result<bool> {
    if description.is_empty() {
        return Ok(false);
    }
    let current = export_values(&[uuid])?
        .into_iter()
        .next()
        .context("task not found")?;
    let Some(updated) = with_text(current, description, notes) else {
        return Ok(false);
    };

    let mut child = base()
        .arg("import")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("could not run `task import`")?;
    // Taken, so it is dropped — and the pipe closed — before the wait: import
    // reads to end of input.
    child
        .stdin
        .take()
        .context("`task import` has no stdin")?
        .write_all(updated.to_string().as_bytes())
        .context("could not write to `task import`")?;
    let out = child.wait_with_output().context("`task import` did not finish")?;
    anyhow::ensure!(
        out.status.success(),
        "`task import` failed: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(true)
}

/// `task` with its description and notes swapped for these, or `None` when
/// that would change nothing.
///
/// Split out from [`replace_text`] so the one part that decides what reaches
/// the database can be tested without one. A task with no notes exports no
/// `annotations` key rather than an empty list, so the key is removed, not
/// emptied, when every note is deleted, and a missing key compares as none.
fn with_text(mut task: Value, description: &str, notes: &[NoteEdit]) -> Option<Value> {
    let annotations: Vec<Value> = notes
        .iter()
        .map(|n| match &n.entry {
            Some(entry) => serde_json::json!({ "entry": entry, "description": n.text }),
            None => serde_json::json!({ "description": n.text }),
        })
        .collect();

    let same_description = task.get("description").and_then(Value::as_str) == Some(description);
    let current = task
        .get("annotations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if same_description && current == annotations {
        return None;
    }

    // `task export` always emits objects; anything else is not a task this
    // could write back, and "nothing to do" is the safe reading of it.
    let fields = task.as_object_mut()?;
    fields.insert("description".into(), description.into());
    if annotations.is_empty() {
        fields.remove("annotations");
    } else {
        fields.insert("annotations".into(), Value::Array(annotations));
    }
    Some(task)
}
```

- [ ] **Step 4: Run the unit tests**

Run: `cargo test --lib task::tests`
Expected: all pass, including the four new ones.

- [ ] **Step 5: Write the failing sandbox test.** In `tests/write_path.rs`, change the import line to `use niri_tasks::{task, task::NoteEdit, text};` and insert this section just before `// ---- empty input is a no-op, not an error`:

```rust
    // ---- replace description and notes in one import ---------------------
    // The task box's save for an existing task. Taskwarrior cannot edit or
    // delete one annotation in place, so the whole list goes back through
    // `task import` — which makes it the write most able to lose something:
    // an unchanged note's date, a tag, or the literal-description rule.
    task::add(TAG, &text::add_args("to be rewritten")).expect("add");
    let target = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description == "to be rewritten")
        .expect("find the task to rewrite")
        .uuid;
    for note in ["one", "two", "three"] {
        task::annotate(&target, note).expect("annotate");
    }
    let stamps: Vec<String> = task::get(&target)
        .expect("get")
        .expect("exists")
        .annotations
        .iter()
        .map(|a| a.entry.clone())
        .collect();
    assert_eq!(stamps.len(), 3, "three notes, each with its own stamp");

    let wrote = task::replace_text(
        &target,
        "rewritten due:2026-09-01",
        &[
            NoteEdit { entry: Some(stamps[0].clone()), text: "one, edited".into() },
            NoteEdit { entry: Some(stamps[2].clone()), text: "three".into() },
            NoteEdit { entry: None, text: "four".into() },
            NoteEdit { entry: None, text: "five".into() },
        ],
    )
    .expect("replace");
    assert!(wrote, "a changed task is written");

    let rewritten = task::get(&target).expect("get").expect("exists");
    assert_eq!(rewritten.description, "rewritten due:2026-09-01");
    assert!(
        raw(&target, "due").is_empty(),
        "the description goes through import literally, like an edit"
    );
    assert!(rewritten.tags.iter().any(|t| t == TAG), "import must not drop the tag");
    assert_eq!(rewritten.status, "pending");

    let notes: Vec<(String, String)> = rewritten
        .annotations
        .iter()
        .map(|a| (a.entry.clone(), a.description.clone()))
        .collect();
    assert_eq!(notes.len(), 4, "note two is gone, four and five are new");
    assert_eq!(notes[0], (stamps[0].clone(), "one, edited".into()), "an edited note keeps its date");
    assert_eq!(notes[1], (stamps[2].clone(), "three".into()), "an untouched note keeps its date");
    assert_eq!((notes[2].1.as_str(), notes[3].1.as_str()), ("four", "five"));
    assert!(
        notes[2].0 > stamps[2] && notes[3].0 > notes[2].0,
        "new notes are dated now, after the old ones, in the order they were typed"
    );

    // Saving what is already there writes nothing — not even an undo entry.
    let undo = sandbox.dir.join("data").join("undo.data");
    let undo_lines = || std::fs::read_to_string(&undo).unwrap_or_default().lines().count();
    let before_noop = undo_lines();
    let unchanged: Vec<NoteEdit> = rewritten
        .annotations
        .iter()
        .map(|a| NoteEdit { entry: Some(a.entry.clone()), text: a.description.clone() })
        .collect();
    assert!(
        !task::replace_text(&target, &rewritten.description, &unchanged).expect("no-op"),
        "an unchanged save reports that it wrote nothing"
    );
    assert_eq!(undo_lines(), before_noop, "and leaves nothing in the undo log");

    // An empty description is not a task: nothing is written.
    assert!(!task::replace_text(&target, "", &[]).expect("empty description"));
    assert_eq!(task::get(&target).expect("get").expect("exists").annotations.len(), 4);

    // Deleting every note leaves a task with none.
    assert!(task::replace_text(&target, "rewritten due:2026-09-01", &[]).expect("clear notes"));
    assert!(!task::get(&target).expect("get").expect("exists").has_notes());
```

- [ ] **Step 6: Run the sandbox test**

Run: `cargo test --test write_path`
Expected: PASS. (Step 3 already implemented `replace_text`. If this fails, the failure is real behaviour against Taskwarrior; debug it, don't loosen the assertion.)

- [ ] **Step 7: Run the whole suite and commit**

Run: `cargo test`
Expected: all pass.

```bash
git add src/task.rs tests/write_path.rs
git commit -m "Replace a task's description and notes in one import, writing nothing when unchanged

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The box's keys and form, as pure logic

**Files:**
- Create: `src/taskbox/keys.rs`, `src/taskbox/form.rs`
- Modify: `src/taskbox.rs` (only add `pub mod form; pub mod keys;` below the module doc comment, after the `//!` block and before the `use` lines)

**Interfaces:**
- Consumes: `crate::task::NoteEdit` (Task 1), `crate::text::collapse_whitespace`.
- Produces:
  - `keys::Place` = `Description | Note { index: usize, empty: bool }` (`Debug, Clone, Copy, PartialEq, Eq`)
  - `keys::KeyAction` = `Save | Cancel | ToFirstNote | NewNoteBelow(usize) | DeleteNote(usize) | Ignore`
  - `keys::key_action(key: gdk::Key, ctrl: bool, place: Option<Place>) -> KeyAction`
  - `form::Row { pub entry: Option<String>, pub loaded: String, pub text: String }`
  - `form::Submission { pub description: String, pub notes: Vec<NoteEdit> }` with `pub fn note_texts(&self) -> Vec<String>`
  - `form::cleaned(loaded: &str, text: &str) -> String`
  - `form::submission(loaded_description: &str, description: &str, rows: &[Row]) -> Option<Submission>`

- [ ] **Step 1: Write `src/taskbox/keys.rs` with tests and a stub that fails them**

```rust
//! What a keypress in the task box means, by where the cursor is.
//!
//! Pure, so the mapping is testable without a window. The other half of
//! making these keys work is the controller's propagation phase, which only a
//! real compositor exercises (`tests/e2e-box.sh`).

use gtk4::gdk;

/// Where the keyboard focus is, as far as the box's keys care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Description,
    /// The note row at `index`, and whether it has any text — Backspace only
    /// deletes a row there is nothing left in.
    Note { index: usize, empty: bool },
}

/// What a keypress in the box should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Save,
    Cancel,
    /// From the description to the first note, making one if there are none.
    ToFirstNote,
    /// A new empty row below the one at this index, with the cursor in it.
    NewNoteBelow(usize),
    /// Delete the row at this index, and move up.
    DeleteNote(usize),
    /// Pass it through: ordinary typing, and Enter on a button, which presses
    /// it.
    Ignore,
}

/// Map a keypress to what it should do. `place` is `None` when the focus is
/// on something other than a text field — a button.
pub fn key_action(key: gdk::Key, ctrl: bool, place: Option<Place>) -> KeyAction {
    let _ = (key, ctrl, place);
    KeyAction::Ignore
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERYWHERE: [Option<Place>; 4] = [
        None,
        Some(Place::Description),
        Some(Place::Note { index: 0, empty: true }),
        Some(Place::Note { index: 2, empty: false }),
    ];

    #[test]
    fn escape_discards_from_anywhere() {
        for place in EVERYWHERE {
            assert_eq!(key_action(gdk::Key::Escape, false, place), KeyAction::Cancel);
            assert_eq!(key_action(gdk::Key::Escape, true, place), KeyAction::Cancel);
        }
    }

    #[test]
    fn ctrl_enter_saves_from_anywhere() {
        for place in EVERYWHERE {
            assert_eq!(key_action(gdk::Key::Return, true, place), KeyAction::Save);
            assert_eq!(key_action(gdk::Key::KP_Enter, true, place), KeyAction::Save);
        }
    }

    #[test]
    fn enter_in_the_description_moves_to_the_first_note() {
        assert_eq!(
            key_action(gdk::Key::Return, false, Some(Place::Description)),
            KeyAction::ToFirstNote
        );
        assert_eq!(
            key_action(gdk::Key::KP_Enter, false, Some(Place::Description)),
            KeyAction::ToFirstNote
        );
    }

    #[test]
    fn enter_in_a_note_adds_a_row_below_it() {
        for empty in [true, false] {
            assert_eq!(
                key_action(gdk::Key::Return, false, Some(Place::Note { index: 3, empty })),
                KeyAction::NewNoteBelow(3)
            );
        }
    }

    /// A focused button is pressed by Enter; taking the key would break it.
    #[test]
    fn enter_on_a_button_is_passed_through() {
        assert_eq!(key_action(gdk::Key::Return, false, None), KeyAction::Ignore);
    }

    #[test]
    fn backspace_deletes_only_an_empty_row() {
        assert_eq!(
            key_action(gdk::Key::BackSpace, false, Some(Place::Note { index: 1, empty: true })),
            KeyAction::DeleteNote(1)
        );
        assert_eq!(
            key_action(gdk::Key::BackSpace, false, Some(Place::Note { index: 1, empty: false })),
            KeyAction::Ignore,
            "in a row with text, Backspace deletes a character"
        );
        assert_eq!(
            key_action(gdk::Key::BackSpace, false, Some(Place::Description)),
            KeyAction::Ignore,
            "the description is never deleted"
        );
    }

    /// Ctrl+A and friends must still reach the text view, or select-all and
    /// the usual editing keys stop working inside the box.
    #[test]
    fn ordinary_typing_is_passed_through() {
        for key in [gdk::Key::a, gdk::Key::space, gdk::Key::Tab, gdk::Key::Up] {
            for place in EVERYWHERE {
                assert_eq!(key_action(key, false, place), KeyAction::Ignore);
                assert_eq!(key_action(key, true, place), KeyAction::Ignore);
            }
        }
    }
}
```

Add `pub mod form;` and `pub mod keys;` to `src/taskbox.rs` (create `form.rs` in Step 3; for now add only `pub mod keys;`).

- [ ] **Step 2: Run to see the key tests fail**

Run: `cargo test --lib taskbox::keys`
Expected: FAIL. `escape_discards_from_anywhere`, `ctrl_enter_saves_from_anywhere`, `enter_in_*` and `backspace_deletes_only_an_empty_row` fail with `left: Ignore`.

- [ ] **Step 3: Implement `key_action`**

```rust
pub fn key_action(key: gdk::Key, ctrl: bool, place: Option<Place>) -> KeyAction {
    let enter = matches!(key, gdk::Key::Return | gdk::Key::KP_Enter);
    match (key, place) {
        (gdk::Key::Escape, _) => KeyAction::Cancel,
        _ if enter && ctrl => KeyAction::Save,
        (_, Some(Place::Description)) if enter => KeyAction::ToFirstNote,
        (_, Some(Place::Note { index, .. })) if enter => KeyAction::NewNoteBelow(index),
        (gdk::Key::BackSpace, Some(Place::Note { index, empty: true })) => KeyAction::DeleteNote(index),
        _ => KeyAction::Ignore,
    }
}
```

Run: `cargo test --lib taskbox::keys`
Expected: PASS.

- [ ] **Step 4: Write `src/taskbox/form.rs` with tests and a stub**

```rust
//! What is on screen in the task box, turned into what to save.
//!
//! Pure, so the rules that decide what reaches taskwarrior are tested without
//! typing into a window: which rows count, which notes keep their date, and
//! when there is nothing to do at all.

use crate::task::NoteEdit;
use crate::text::collapse_whitespace;

/// One note row as it stands when the box is saved.
pub struct Row {
    /// The stamp of the note this row was loaded from; `None` for a row added
    /// in the box.
    pub entry: Option<String>,
    /// The text it was loaded with — empty for a new row.
    pub loaded: String,
    /// The text in it now.
    pub text: String,
}

/// What the box hands back when it is saved.
pub struct Submission {
    pub description: String,
    /// In the order the rows are on screen, empty rows left out.
    pub notes: Vec<NoteEdit>,
}

impl Submission {
    /// The notes' text alone, for adding a task — a new task's notes are all
    /// new, so there is no date to keep.
    pub fn note_texts(&self) -> Vec<String> {
        self.notes.iter().map(|n| n.text.clone()).collect()
    }
}

/// The text to save for a field that was loaded with `loaded`.
///
/// Typed text has its whitespace collapsed: taskwarrior descriptions and notes
/// are single-line, and a pasted newline has to go somewhere. Text left exactly
/// as it was loaded is kept verbatim instead, so opening a task and saving it
/// untouched never rewrites a stored double space into a change.
pub fn cleaned(loaded: &str, text: &str) -> String {
    let _ = loaded;
    text.to_string()
}

/// What to save, or `None` when there is nothing to do — an empty description,
/// which gives a task nothing to be called and its notes nothing to hang off.
pub fn submission(loaded_description: &str, description: &str, rows: &[Row]) -> Option<Submission> {
    let _ = (loaded_description, rows);
    Some(Submission { description: description.to_string(), notes: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(entry: Option<&str>, loaded: &str, text: &str) -> Row {
        Row { entry: entry.map(Into::into), loaded: loaded.into(), text: text.into() }
    }

    #[test]
    fn an_empty_description_saves_nothing() {
        let notes = [row(None, "", "a note")];
        assert!(submission("", "", &notes).is_none());
        assert!(submission("", "  \n ", &notes).is_none());
        assert!(submission("was here", "", &[]).is_none(), "clearing it is not deleting the task");
    }

    #[test]
    fn typed_text_is_collapsed_and_untouched_text_is_kept() {
        assert_eq!(cleaned("", "  two\nlines  "), "two lines");
        assert_eq!(cleaned("stored  as is", "stored  as is"), "stored  as is");
        assert_eq!(cleaned("stored  as is", "stored  as  was"), "stored as was");
    }

    #[test]
    fn rows_become_notes_in_screen_order() {
        let s = submission(
            "d",
            "d",
            &[
                row(Some("20260802T000000Z"), "two", "two, edited"),
                row(None, "", "new one"),
                row(Some("20260801T000000Z"), "one", "one"),
            ],
        )
        .expect("something to save");
        assert_eq!(
            s.notes,
            vec![
                NoteEdit { entry: Some("20260802T000000Z".into()), text: "two, edited".into() },
                NoteEdit { entry: None, text: "new one".into() },
                NoteEdit { entry: Some("20260801T000000Z".into()), text: "one".into() },
            ],
            "an edited note keeps its stamp; a new row has none"
        );
    }

    /// An empty row is a row nobody typed into — Note opens one, Enter makes
    /// one — and an existing note emptied out is a note deleted.
    #[test]
    fn empty_rows_are_left_out() {
        let s = submission(
            "d",
            "d",
            &[row(None, "", ""), row(None, "", "   "), row(Some("20260801T000000Z"), "one", "")],
        )
        .expect("the description alone is worth saving");
        assert!(s.notes.is_empty());
    }

    #[test]
    fn note_texts_are_the_notes_in_order() {
        let s = submission("", "d", &[row(None, "", "a"), row(None, "", "b")]).unwrap();
        assert_eq!(s.note_texts(), vec!["a".to_string(), "b".to_string()]);
    }
}
```

Add `pub mod form;` to `src/taskbox.rs` next to `pub mod keys;`.

- [ ] **Step 5: Run to see the form tests fail**

Run: `cargo test --lib taskbox::form`
Expected: FAIL. `an_empty_description_saves_nothing`, `typed_text_is_collapsed_and_untouched_text_is_kept`, `rows_become_notes_in_screen_order`, `note_texts_are_the_notes_in_order`.

- [ ] **Step 6: Implement `cleaned` and `submission`**

```rust
pub fn cleaned(loaded: &str, text: &str) -> String {
    if text == loaded {
        text.to_string()
    } else {
        collapse_whitespace(text)
    }
}

pub fn submission(loaded_description: &str, description: &str, rows: &[Row]) -> Option<Submission> {
    let description = cleaned(loaded_description, description);
    if description.trim().is_empty() {
        return None;
    }
    let notes = rows
        .iter()
        .filter_map(|r| {
            let text = cleaned(&r.loaded, &r.text);
            (!text.trim().is_empty()).then(|| NoteEdit { entry: r.entry.clone(), text })
        })
        .collect();
    Some(Submission { description, notes })
}
```

- [ ] **Step 7: Run and commit**

Run: `cargo test --lib taskbox`
Expected: PASS. The old `taskbox::tests` still pass, since nothing in `taskbox.rs` changed except the two `pub mod` lines. Dead-code warnings are fine at this point.

```bash
git add src/taskbox.rs src/taskbox/keys.rs src/taskbox/form.rs
git commit -m "Give the task box's keys and saved form their own tested modules

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Style the box like the task panel

**Files:**
- Modify: `src/panel/style.rs` (add a constant after `BACKGROUND`, ~l.26)
- Create: `src/taskbox/style.rs`
- Modify: `src/taskbox.rs` (add `pub mod style;`)

**Interfaces:**
- Produces: `panel::style::WINDOW_BACKGROUND: &str = "rgba(0, 0, 0, 0.5)"`; `taskbox::style::css() -> String`, where every rule is scoped to `.task-box`. It uses the CSS classes `field`, `dim`, `date` and `delete`, and Task 4 puts those same classes on its widgets.

- [ ] **Step 1: Add the constant to `src/panel/style.rs`** directly after `BACKGROUND`:

```rust
/// A terminal window's own fill, before niri touches it: ghostty's black
/// `background` at `background-opacity = 0.5`. For the task box, which is a
/// window rather than a layer surface, niri adds the rest itself — the window
/// `opacity 0.75` that turns this into [`BACKGROUND`], the blur, and the 4px
/// focus ring [`OUTLINE`] imitates — so the box fills with this and draws no
/// outline of its own.
pub const WINDOW_BACKGROUND: &str = "rgba(0, 0, 0, 0.5)";
```

- [ ] **Step 2: Write `src/taskbox/style.rs` with its tests, and an empty `css()`**

```rust
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
    String::new()
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
```

Add `pub mod style;` to `src/taskbox.rs`.

- [ ] **Step 3: Run to see them fail**

Run: `cargo test --lib taskbox::style`
Expected: FAIL, with `missing ...` and `no rules at all`.

- [ ] **Step 4: Implement `css()`**

```rust
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
```

The rule lines that follow a `{` line (the button block's continuation lines) contain no `{`, so the scoping test only checks selector lines, which is what it is meant to check.

- [ ] **Step 5: Run and commit**

Run: `cargo test --lib style`
Expected: PASS for both `panel::style::tests` and `taskbox::style::tests`.

```bash
git add src/panel/style.rs src/taskbox/style.rs src/taskbox.rs
git commit -m "Style the task box in the terminal's font and fill, from the panel's constants

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Rebuild the window around note rows, and route Add, Edit and Note to it

These go in one commit: changing `BoxConfig` and `Submission` breaks every caller until the callers change too.

**Files:**
- Rewrite: `src/taskbox.rs`
- Modify: `src/task.rs` (`Annotation::date` → `pub`; delete `Task::notes_list` and its two tests `notes_list_is_one_line_per_note`, `a_task_with_no_notes_lists_nothing`; reword `add_with_notes`'s doc)
- Modify: `src/main.rs` (Add arm ~l.193–221, Edit arm ~l.229–255, Note arm ~l.264–292; new helper fn)
- Modify: `src/daemon.rs` (`serve_box_request` Add/Edit/Note arms ~l.200–276; new helper fn)
- Delete: `src/theme.rs`; Modify `src/lib.rs` (remove `pub mod theme;`)
- Modify: `src/text.rs` (delete `note_lines` and its tests; reword the module doc's third bullet if present), `tests/write_path.rs` (l.131 stops using `note_lines`)

**Interfaces:**
- Consumes: `task::{NoteEdit, replace_text, Annotation, Task}` (Task 1), `taskbox::{keys, form}` (Task 2), `taskbox::style::css` (Task 3).
- Produces:
  - `taskbox::Mode = Add | Edit | Note`; `Mode::title()` gives `"Add Task"` or `"Edit Task"`.
  - `taskbox::BoxConfig { pub mode: Mode, pub subtitle: String, pub description: String, pub notes: Vec<Annotation> }`, with `BoxConfig::add(tag: &str)` and `BoxConfig::for_task(mode: Mode, task: Task)`.
  - `taskbox::Submission` (re-export of `form::Submission`).
  - `taskbox::show(cfg: BoxConfig) -> Option<Submission>` and `taskbox::open_in(app: &Application, cfg: BoxConfig, on_submit: impl Fn(Submission) + 'static)`, with the same names as today.

- [ ] **Step 1: Update the Mode tests first.** In `src/taskbox.rs` `mod tests`, replace `titles_match_the_mode`, `submit_labels_are_distinct`, the four key tests (`ctrl_enter_submits_and_bare_enter_does_not`, `escape_cancels_with_or_without_ctrl`, `ordinary_typing_is_passed_through`), the three `is_worth_submitting` tests and `both_variants_share_one_width` with:

```rust
    /// One window for an existing task, whichever menu row opened it; only
    /// adding reads differently.
    #[test]
    fn titles_match_the_mode() {
        assert_eq!(Mode::Add.title(), "Add Task");
        assert_eq!(Mode::Edit.title(), "Edit Task");
        assert_eq!(Mode::Note.title(), "Edit Task");
    }

    #[test]
    fn a_task_opens_with_its_description_and_every_note() {
        let t: crate::task::Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20260801T000000Z","description":"Goal: one"},
                {"entry":"20260802T000000Z","description":"Decided: two"}]}"#,
        )
        .unwrap();
        let cfg = BoxConfig::for_task(Mode::Note, t);
        assert_eq!(cfg.description, "d");
        let notes: Vec<&str> = cfg.notes.iter().map(|a| a.description.as_str()).collect();
        assert_eq!(notes, ["Goal: one", "Decided: two"], "as stored, in stored order");
    }
```

Keep the three `fast_startup_*` / `a_running_screen_reader_*` tests exactly as they are.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib taskbox::tests`
Expected: compile error (`Mode::Note`, `BoxConfig::for_task` don't exist).

- [ ] **Step 3: Rewrite `src/taskbox.rs`.** The new file, top to bottom:

1. Module doc. Keep the existing three paragraphs ("This is the one surface fuzzel cannot be…", "It is a plain fixed-size window…", "Replaces `TaskBoxDaemon.qml`…"), but change the first sentence to `//! The task box: one window for adding a task and for editing one.` and add this paragraph after the second:

```rust
//! Its description is a wrapping text area and its notes are a list of rows,
//! one per note, each editable in place with its date at its right end and an
//! × to delete it. Add, Edit and the menu's Note all open this same window;
//! they differ only in what is filled in and where the cursor starts.
```

2. Modules, imports, and types:

```rust
pub mod form;
pub mod keys;
pub mod style;

pub use form::Submission;

use crate::task::{Annotation, Task};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider};
use keys::{KeyAction, Place};
use std::cell::RefCell;
use std::rc::Rc;

/// Which job the box is doing. Edit and Note are the same window on the same
/// task; Note only starts the cursor in a new empty note row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Add,
    Edit,
    Note,
}

impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Mode::Add => "Add Task",
            Mode::Edit | Mode::Note => "Edit Task",
        }
    }

    fn submit_label(self) -> &'static str {
        match self {
            Mode::Add => "Add",
            Mode::Edit | Mode::Note => "Save",
        }
    }
}

/// Fixed, because a resizable window here would be a decision to make every
/// time rather than a box that is always the same shape. Big enough that a
/// planned task's ten long notes read as a list rather than a keyhole.
const WIDTH: i32 = 800;
const HEIGHT: i32 = 760;
/// About three lines of the terminal font plus the field's padding: room to
/// see a long description whole, while the notes keep the rest.
const DESCRIPTION_HEIGHT: i32 = 84;

/// Stable, so `window-rule { match app-id="dev.niri-tasks.box" }` works.
pub const APP_ID: &str = "dev.niri-tasks.box";

pub struct BoxConfig {
    pub mode: Mode,
    /// A dim line above the description — the tag a new task goes to. Hidden
    /// when empty.
    pub subtitle: String,
    pub description: String,
    /// The task's notes, shown as stored and in stored order.
    pub notes: Vec<Annotation>,
}

impl BoxConfig {
    /// An empty box for a new task on `tag`.
    pub fn add(tag: &str) -> Self {
        Self {
            mode: Mode::Add,
            subtitle: format!("+{tag}"),
            description: String::new(),
            notes: Vec::new(),
        }
    }

    /// The box for an existing task. It fetches the description and notes
    /// itself, rather than taking them as arguments: the ones you open the box
    /// to fix are the long ones, which a picker row shows a fraction of.
    pub fn for_task(mode: Mode, task: Task) -> Self {
        Self {
            mode,
            subtitle: String::new(),
            description: task.description,
            notes: task.annotations,
        }
    }
}
```

3. `open_in`, `show`, and the fast-startup functions. Keep `open_in`'s doc comment, and change its body to `build_window(app, &cfg, Rc::new(on_submit));`. Keep `show` and its doc comment, but drop `let theme = Theme::load();` and the `&theme` argument, and fix its doc's first paragraph to say `None` means "discarded, or nothing worth saving". Keep `prefer_fast_startup`, `fast_startup_overrides`, `screen_reader_enabled` and `A11Y_SCHEMA` verbatim. In the long `GSK_RENDERER=cairo` comment, change "at 560x440" to "at 800x760".

4. The window. Replace the old `build_window` entirely with:

```rust
/// Build and show the window. `on_submit` fires only with something worth
/// saving: never on Esc, never with an empty description.
fn build_window(app: &Application, cfg: &BoxConfig, on_submit: Rc<dyn Fn(Submission)>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(cfg.mode.title())
        .default_width(WIDTH)
        .default_height(HEIGHT)
        .resizable(false)
        .build();
    // Equal minimum and maximum is what makes niri float this rather than tile
    // it into the column layout.
    window.set_size_request(WIDTH, HEIGHT);
    // What every rule in the box's stylesheet is scoped to (style.rs).
    window.add_css_class("task-box");

    let provider = CssProvider::new();
    // load_from_data, not load_from_string: the latter is gated behind gtk4's
    // v4_12 feature, and this needs no minimum beyond what the crate requires.
    provider.load_from_data(&style::css());
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(20);
    root.set_margin_end(20);

    // ─── header ───────────────────────────────────────────────────────────
    let header = gtk4::Label::new(Some(&cfg.subtitle));
    header.add_css_class("dim");
    header.set_xalign(0.0);
    header.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    header.set_visible(!cfg.subtitle.is_empty());
    root.append(&header);

    // ─── description ──────────────────────────────────────────────────────
    let description = text_view(&cfg.description);
    let description_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .height_request(DESCRIPTION_HEIGHT)
        .child(&description)
        .build();
    description_scroll.add_css_class("field");
    root.append(&description_scroll);

    // ─── notes ────────────────────────────────────────────────────────────
    let notes_label = gtk4::Label::new(Some("Notes"));
    notes_label.add_css_class("dim");
    notes_label.set_xalign(0.0);
    root.append(&notes_label);

    let list = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    // Rows wrap, so there is never anything to scroll to sideways. Scrolling to
    // the focus is what keeps a row made by Enter at the bottom in view.
    let viewport = gtk4::Viewport::builder()
        .scroll_to_focus(true)
        .child(&list)
        .build();
    let notes_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&viewport)
        .build();
    root.append(&notes_scroll);

    let notes = Rc::new(Notes {
        list,
        description: description.clone(),
        rows: RefCell::new(Vec::new()),
    });
    for note in &cfg.notes {
        notes.insert(notes.len(), Some(note));
    }

    let add_note = gtk4::Button::with_label("+ Add note");
    add_note.set_halign(gtk4::Align::Start);
    root.append(&add_note);

    // ─── footer ───────────────────────────────────────────────────────────
    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let hint = gtk4::Label::new(Some("Enter: next note · Ctrl+Enter: save · Esc: discard"));
    hint.add_css_class("dim");
    hint.set_xalign(0.0);
    hint.set_hexpand(true);
    let cancel = gtk4::Button::with_label("Cancel");
    let submit = gtk4::Button::with_label(cfg.mode.submit_label());
    footer.append(&hint);
    footer.append(&cancel);
    footer.append(&submit);
    root.append(&footer);

    window.set_child(Some(&root));

    // ─── save / discard ───────────────────────────────────────────────────
    let do_submit = {
        let loaded_description = cfg.description.clone();
        let window = window.clone();
        let notes = notes.clone();
        move || {
            let submission = form::submission(
                &loaded_description,
                &buffer_text(&notes.description),
                &notes.rows(),
            );
            window.close();
            if let Some(submission) = submission {
                on_submit(submission);
            }
        }
    };

    {
        let do_submit = do_submit.clone();
        submit.connect_clicked(move |_| do_submit());
    }
    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    {
        let notes = notes.clone();
        add_note.connect_clicked(move |_| focus_end(&notes.insert(notes.len(), None)));
    }

    // Capture phase, so the window sees a key before the focused text view
    // does. Defaulting to bubble was a bug once: the view took Return,
    // inserted a newline and stopped it there, so Ctrl+Enter did nothing while
    // Escape — which a text view does not consume — kept working. Everything
    // `key_action` does not claim still returns Proceed and reaches the view.
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let window = window.clone();
        let notes = notes.clone();
        keys.connect_key_pressed(move |_, key, _, state| {
            let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
            // GtkWindowExt and RootExt both have a `focus()`; either answers.
            let place = GtkWindowExt::focus(&window).and_then(|w| notes.place_of(&w));
            match keys::key_action(key, ctrl, place) {
                KeyAction::Cancel => window.close(),
                KeyAction::Save => do_submit(),
                KeyAction::ToFirstNote => focus_end(&notes.first_or_new()),
                KeyAction::NewNoteBelow(i) => focus_end(&notes.insert(i + 1, None)),
                KeyAction::DeleteNote(i) => notes.remove(i),
                KeyAction::Ignore => return gtk4::glib::Propagation::Proceed,
            }
            gtk4::glib::Propagation::Stop
        });
    }
    window.add_controller(keys);

    window.present();

    // The Wayland app_id comes from the GtkApplication, and inside the daemon
    // that application is the daemon's — so a box opened there arrived as
    // dev.niri-tasks.daemon while one opened by the CLI was dev.niri-tasks.box.
    // It is set per-toplevel rather than per-application, so set it here once
    // the surface exists.
    if let Some(surface) = window.surface() {
        if let Ok(toplevel) = surface.downcast::<gdk4_wayland::WaylandToplevel>() {
            toplevel.set_application_id(APP_ID);
        }
    }

    // Where the cursor starts is what tells Note from Edit. At the end of the
    // text, so typing carries on rather than landing in front of it.
    match cfg.mode {
        Mode::Note => focus_end(&notes.insert(notes.len(), None)),
        Mode::Add | Mode::Edit => focus_end(&description),
    }
}

/// The note rows on screen, in order, and the box that holds them.
struct Notes {
    list: gtk4::Box,
    /// Where the cursor goes when the first row is deleted.
    description: gtk4::TextView,
    rows: RefCell<Vec<NoteRow>>,
}

struct NoteRow {
    entry: Option<String>,
    loaded: String,
    container: gtk4::Box,
    view: gtk4::TextView,
}

impl Notes {
    fn len(&self) -> usize {
        self.rows.borrow().len()
    }

    /// Put a row at `index` — `note`'s, or an empty new one — and return its
    /// text view, for the caller to focus if it should.
    fn insert(self: &Rc<Self>, index: usize, note: Option<&Annotation>) -> gtk4::TextView {
        let text = note.map(|n| n.description.clone()).unwrap_or_default();
        let view = text_view(&text);
        view.set_hexpand(true);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.add_css_class("field");
        container.append(&view);

        // Small and dim at the right end. A new row has no date until
        // taskwarrior gives it one on save.
        if let Some(note) = note {
            let date = gtk4::Label::new(Some(&note.date()));
            date.add_css_class("date");
            date.set_valign(gtk4::Align::Start);
            container.append(&date);
        }

        let delete = gtk4::Button::with_label("×");
        delete.add_css_class("delete");
        delete.set_valign(gtk4::Align::Start);
        delete.set_tooltip_text(Some("Delete this note"));
        container.append(&delete);
        {
            // Weak, both of them: the row's own button holding the list that
            // holds the row is a cycle, and in the daemon it would keep every
            // closed box's rows alive.
            let notes = Rc::downgrade(self);
            let view = view.downgrade();
            delete.connect_clicked(move |_| {
                if let (Some(notes), Some(view)) = (notes.upgrade(), view.upgrade()) {
                    if let Some(i) = notes.index_of(&view) {
                        notes.remove(i);
                    }
                }
            });
        }

        let mut rows = self.rows.borrow_mut();
        match index.checked_sub(1).and_then(|above| rows.get(above)) {
            Some(above) => self.list.insert_child_after(&container, Some(&above.container)),
            None => self.list.prepend(&container),
        }
        rows.insert(
            index,
            NoteRow {
                entry: note.map(|n| n.entry.clone()),
                loaded: text,
                container,
                view: view.clone(),
            },
        );
        view
    }

    /// Delete the row at `index` and put the cursor at the end of the row
    /// above it — or of the description, when it was the first.
    fn remove(&self, index: usize) {
        let row = self.rows.borrow_mut().remove(index);
        self.list.remove(&row.container);
        let above = match index.checked_sub(1) {
            Some(i) => self.rows.borrow()[i].view.clone(),
            None => self.description.clone(),
        };
        focus_end(&above);
    }

    /// The first row's text view, making an empty row when there is none, so
    /// Enter in the description always has somewhere to go.
    fn first_or_new(self: &Rc<Self>) -> gtk4::TextView {
        let first = self.rows.borrow().first().map(|r| r.view.clone());
        first.unwrap_or_else(|| self.insert(0, None))
    }

    fn index_of(&self, view: &gtk4::TextView) -> Option<usize> {
        self.rows.borrow().iter().position(|r| &r.view == view)
    }

    /// Where `focus` is, in the terms `keys::key_action` asks about.
    fn place_of(&self, focus: &gtk4::Widget) -> Option<Place> {
        if focus == self.description.upcast_ref::<gtk4::Widget>() {
            return Some(Place::Description);
        }
        self.rows
            .borrow()
            .iter()
            .enumerate()
            .find(|(_, r)| r.view.upcast_ref::<gtk4::Widget>() == focus)
            .map(|(index, r)| Place::Note {
                index,
                empty: r.view.buffer().char_count() == 0,
            })
    }

    fn rows(&self) -> Vec<form::Row> {
        self.rows
            .borrow()
            .iter()
            .map(|r| form::Row {
                entry: r.entry.clone(),
                loaded: r.loaded.clone(),
                text: buffer_text(&r.view),
            })
            .collect()
    }
}

/// A wrapping text field. Tab is left to move the focus, so the keyboard can
/// reach a row's × and the buttons.
fn text_view(text: &str) -> gtk4::TextView {
    let view = gtk4::TextView::new();
    view.set_wrap_mode(gtk4::WrapMode::WordChar);
    view.set_accepts_tab(false);
    view.buffer().set_text(text);
    view
}

fn buffer_text(view: &gtk4::TextView) -> String {
    let buffer = view.buffer();
    buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
}

fn focus_end(view: &gtk4::TextView) {
    view.grab_focus();
    let buffer = view.buffer();
    buffer.place_cursor(&buffer.end_iter());
}
```

5. The `#[cfg(test)] mod tests` from Step 1.

- [ ] **Step 4: Update `src/task.rs`.** Change `fn date(&self)` to `pub fn date(&self)`, and change its doc's first line to `/// The \`entry\` stamp as a date a person reads — a task box row's date, and the front of [\`Annotation::line\`].`. Delete `Task::notes_list` and the tests `notes_list_is_one_line_per_note` and `a_task_with_no_notes_lists_nothing`. Change `Annotation::line`'s doc to `/// One note as \`niritasks task get-notes\` prints it: the date it was added, then the text.`. Replace the first paragraph of `add_with_notes`'s doc with:

```rust
/// Add a task and attach one annotation per note.
///
/// The notes are the add box's rows, in order, empty ones already dropped.
/// They are attached one at a time rather than joined, because separate
/// annotations are what the picker's `¶` marker and the box's rows are
/// counting.
```

- [ ] **Step 5: Delete the theme and `note_lines`.**

```bash
git rm src/theme.rs
```

Remove `pub mod theme;` from `src/lib.rs`. In `src/text.rs`, delete `note_lines` and every test that calls it (the tests around l.100–130 that call `note_lines(`). In `tests/write_path.rs` (~l.131), replace

```rust
    let notes = text::note_lines("first note\n\nsecond note with due:2026-09-01\n");
```

with

```rust
    let notes = vec!["first note".to_string(), "second note with due:2026-09-01".to_string()];
```

and change that section's assertion message `"each line is its own annotation, and the blank line is not one"` to `"each note is its own annotation"`. Also update the comment above it from "The add box's second text area: one annotation per line" to "The add box's note rows: one annotation each".

- [ ] **Step 6: Route the CLI (`src/main.rs`).** Add this fn next to the other private helpers (e.g. just after `run`'s closing brace):

```rust
/// Open the task box on an existing task and save what comes back. Edit and
/// Note are one window; the mode only says where the cursor starts.
fn edit_in_box(uuid: &str, mode: taskbox::Mode) -> Result<()> {
    let t = task::get(uuid)?.context("task not found")?;
    if let Some(s) = taskbox::show(taskbox::BoxConfig::for_task(mode, t)) {
        task::replace_text(uuid, &s.description, &s.notes)?;
    }
    Ok(())
}
```

In the Add arm, replace the `match taskbox::show(taskbox::BoxConfig { … })` block with:

```rust
                match taskbox::show(taskbox::BoxConfig::add(&tag)) {
                    Some(s) => {
                        let notes = s.note_texts();
                        (s.description, notes)
                    }
                    None => return Ok(()),
                }
```

Replace the whole Edit arm with:

```rust
        TaskCommand::Edit { uuid, text: words } => {
            if words.is_empty() {
                if delegate_to_daemon(ipc::Request::Edit(uuid.clone())) {
                    return Ok(());
                }
                return edit_in_box(&uuid, taskbox::Mode::Edit);
            }
            let description = text::collapse_whitespace(&words.join(" "));
            if description.is_empty() {
                return Ok(());
            }
            task::modify_description(&uuid, &description)?;
        }
```

Replace the whole Note arm with:

```rust
        TaskCommand::Note { uuid, text: words } => {
            if words.is_empty() {
                if delegate_to_daemon(ipc::Request::Note(uuid.clone())) {
                    return Ok(());
                }
                return edit_in_box(&uuid, taskbox::Mode::Note);
            }
            let note = text::collapse_whitespace(&words.join(" "));
            if note.is_empty() {
                return Ok(());
            }
            task::annotate(&uuid, &note)?;
        }
```

- [ ] **Step 7: Route the daemon (`src/daemon.rs`).** In `serve_box_request`'s Add arm, replace the `taskbox::open_in(…)` call with:

```rust
            taskbox::open_in(
                app,
                taskbox::BoxConfig::add(&tag),
                move |sub: taskbox::Submission| {
                    if let Err(e) = task::add_with_notes(
                        &tag_for_submit,
                        &text::add_args(&sub.description),
                        &sub.note_texts(),
                    ) {
                        notify::tasks(&e.to_string());
                    } else {
                        notify::tasks(&format!("Added to +{tag_for_submit}: {}", sub.description));
                    }
                },
            );
```

Replace the whole `Request::Edit(uuid) => { … }` and `Request::Note(uuid) => { … }` arms with:

```rust
        Request::Edit(uuid) => open_task_box(app, uuid, taskbox::Mode::Edit),
        Request::Note(uuid) => open_task_box(app, uuid, taskbox::Mode::Note),
```

and add after `serve_box_request`:

```rust
/// Open the task box on an existing task, and save what comes back. Edit and
/// Note are one window; the mode only says where the cursor starts.
fn open_task_box(app: &Application, uuid: String, mode: crate::taskbox::Mode) {
    use crate::{notify, task, taskbox};

    let Ok(Some(t)) = task::get(&uuid) else {
        notify::tasks("Task not found");
        return;
    };
    taskbox::open_in(
        app,
        taskbox::BoxConfig::for_task(mode, t),
        move |sub: taskbox::Submission| {
            if let Err(e) = task::replace_text(&uuid, &sub.description, &sub.notes) {
                notify::tasks(&e.to_string());
            }
        },
    );
}
```

- [ ] **Step 8: Build and run every cargo test**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" | sort | uniq -c; cargo test`
Expected: no errors and no new warnings (in particular no unused `theme`, `note_lines` or `notes_list`), and every test passes. Also run `grep -rn "theme::\|note_lines\|notes_list\|Mode::Annotate\|HEIGHT_WITH_NOTES" src tests` and expect no matches.

- [ ] **Step 9: Look at it by hand** against a sandbox, with the daemon out of the way so the CLI builds the box itself:

```bash
cargo build --release
SB=$(mktemp -d); mkdir -p $SB/data; echo "data.location=$SB/data" > $SB/taskrc
export TASKRC=$SB/taskrc TASKDATA=$SB/data
systemctl --user stop niri-tasks.service
task rc.verbose=nothing add "A planned task with a long description that should wrap onto a second line inside the description field" +demo
U=$(task rc.verbose=nothing rc.json.array=on +demo export | python3 -c "import json,sys;print(json.load(sys.stdin)[0]['uuid'])")
for n in $(seq 1 10); do task rc.verbose=nothing $U annotate -- "Decided: note $n — long enough to wrap: lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."; done
./target/release/niritasks task edit $U
```

Check: the window floats at 800×760 in the terminal's font, with the same tinted, blurred look as a terminal. The description wraps within about three lines. All ten notes are rows that wrap, each with a dim date at its right end and an ×, and the list scrolls. Clicking × removes a row, and "+ Add note" adds an empty, undated row and focuses it. The focused field has a 1px white outline. Close with Esc and check that `task $U export` is unchanged. Then run `./target/release/niritasks task note $U` and check that the cursor is in a new empty row at the bottom, scrolled into view. Finish with `systemctl --user start niri-tasks.service`.

- [ ] **Step 10: Commit**

```bash
git add -A src tests/write_path.rs
git commit -m "Make the task box one window whose notes are editable rows

Add, Edit and Note open the same window. Each note is a row that wraps,
edits in place, shows its date and deletes with ×; saving an existing task
replaces its description and notes in one import. Styled from the task
panel's constants, so the DMS theme reader goes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Drive the new box end to end, and document it

**Files:**
- Modify: `tests/e2e-box.sh` (header comment, `pkill` pattern, `run_suite`)
- Modify: `README.md` (keybind table l.18; "Two rules" section l.128–143; e2e paragraphs ~l.187, 226–260)
- Modify: `CONTEXT.md` (Task box entry; add Note row)

**Interfaces:**
- Consumes: the box from Task 4 (titles `Add Task` / `Edit Task`; the keys; `app_id` `dev.niri-tasks.box`).

- [ ] **Step 1: Add helpers to `tests/e2e-box.sh`** after `notes_of()`:

```bash
# A task's notes with their stamps, one "<entry> <text>" per line.
stamped_notes_of() {
    task rc.verbose=nothing rc.json.array=on "$1" export 2>/dev/null \
      | python3 -c "
import json,sys
ts=json.load(sys.stdin)
print('\n'.join(a['entry']+' '+a['description'] for a in (ts[0].get('annotations') or []))) if ts else None
"
}
# How many changes taskwarrior has logged — an unchanged save must add none.
undo_count() { wc -l < "$TASKDATA/undo.data" 2>/dev/null || echo 0; }
field_of() {
    task rc.verbose=nothing rc.json.array=on "$1" export 2>/dev/null \
      | python3 -c "import json,sys; ts=json.load(sys.stdin); print(ts[0].get(sys.argv[1],'') if ts else '')" "$2"
}
```

Change the clean-slate `pkill` line to `pkill -f "$NIRITASKS task (add|edit|note)" 2>/dev/null`.

- [ ] **Step 2: Replace `run_suite`'s body after the "Submitting an empty box is a no-op" block** (that is, from the `# The notes area. Tab moves to it` comment through the "Escape in the note box" block, keeping the final `close_any_box`) with:

```bash
    # Add still word-splits, so taskwarrior attributes parse.
    local dmark="due-$RANDOM" uuid
    if open_box add; then
        wtype "$dmark due:friday"
        sleep 0.4
        wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        uuid=$(uuid_of "$dmark")
        if [ -n "$uuid" ]; then
            [ -n "$(field_of "$uuid" due)" ] && ok "Add filed due:friday as a due date" \
                || bad "Add left due:friday without a due date"
            [ "$(field_of "$uuid" description)" = "$dmark" ] && ok "and took it out of the description" \
                || bad "description was \"$(field_of "$uuid" description)\""
        else
            bad "Add with due:friday wrote no task"
        fi
    else
        bad "box did not reopen"
    fi

    # Notes while adding: Enter in the description moves to the first note,
    # Enter in a note adds a row below. An empty row is not a note.
    local marker="notes-$RANDOM" notes
    if open_box add; then
        wtype "$marker"
        wtype -k Return
        wtype "first note"
        wtype -k Return
        wtype -k Return                      # an empty row, left empty
        wtype "second note"
        sleep 0.4
        wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        uuid=$(uuid_of "$marker")
        if [ -n "$uuid" ]; then
            notes=$(notes_of "$uuid")
            [ "$notes" = "$(printf 'first note\nsecond note')" ] \
                && ok "Enter moved from description to notes; one annotation per row, empty row dropped" \
                || bad "expected first/second note, got: $(printf '%s' "$notes" | tr '\n' '|')"
        else
            bad "add box with notes wrote no task"
        fi
    else
        bad "box did not reopen"
    fi

    # Edit an existing task's notes in place. Three notes, seeded from the CLI.
    local emark="edit-$RANDOM" stamps a_stamp c_stamp
    "$NIRITASKS" task add "$emark" >/dev/null 2>&1
    uuid=$(uuid_of "$emark")
    for n in "note A" "note B" "note C"; do "$NIRITASKS" task note "$uuid" "$n" >/dev/null 2>&1; done
    stamps=$(stamped_notes_of "$uuid")
    a_stamp=$(printf '%s\n' "$stamps" | sed -n 1p | cut -d' ' -f1)
    c_stamp=$(printf '%s\n' "$stamps" | sed -n 3p | cut -d' ' -f1)

    if [ -n "$uuid" ] && open_box edit "$uuid"; then
        [ "$(box_title)" = "Edit Task" ] && ok "edit opens the one box" \
            || bad "edit box title was \"$(box_title)\""
        wtype -k Return                      # description -> note A, cursor at end
        wtype " edited"
        wtype -k Tab; wtype -k Tab           # note A's ×, then note B
        wtype -M ctrl a -m ctrl              # select all of note B
        wtype -k BackSpace                   # ...and clear it
        wtype -k BackSpace                   # an empty row: delete it, move up
        sleep 0.4
        wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        [ "$(notes_of "$uuid")" = "$(printf 'note A edited\nnote C')" ] \
            && ok "a note edited in place and another deleted, in one save" \
            || bad "notes after edit: $(notes_of "$uuid" | tr '\n' '|')"
        [ "$(stamped_notes_of "$uuid" | cut -d' ' -f1 | tr '\n' ' ')" = "$a_stamp $c_stamp " ] \
            && ok "edited and untouched notes kept their dates" \
            || bad "stamps changed: $(stamped_notes_of "$uuid" | tr '\n' '|')"
    else
        bad "edit box did not open"
    fi

    # Saving without changing anything writes nothing at all.
    local undo_before
    undo_before=$(undo_count)
    if [ -n "$uuid" ] && open_box edit "$uuid"; then
        wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        [ "$(undo_count)" -eq "$undo_before" ] && ok "an unchanged save wrote nothing" \
            || bad "an unchanged save wrote to the task database"
    else
        bad "edit box did not reopen"
    fi

    # Note opens the same box with the cursor in a new empty row at the end.
    if [ -n "$uuid" ] && open_box note "$uuid"; then
        [ "$(box_title)" = "Edit Task" ] && ok "note opens the same box" \
            || bad "note box title was \"$(box_title)\""
        wtype "note D"
        wtype -k Return
        wtype "note E"
        sleep 0.4
        wtype -M ctrl -k Return -m ctrl
        sleep 1.8
        [ "$(notes_of "$uuid")" = "$(printf 'note A edited\nnote C\nnote D\nnote E')" ] \
            && ok "note added rows after the existing ones" \
            || bad "notes after note box: $(notes_of "$uuid" | tr '\n' '|')"
        [ "$(stamped_notes_of "$uuid" | sed -n 2p | cut -d' ' -f1)" = "$c_stamp" ] \
            && ok "adding notes left the old ones' dates alone" \
            || bad "note C's date changed"
    else
        bad "note box did not open"
    fi

    # Escape discards everything, without asking.
    if [ -n "$uuid" ] && open_box note "$uuid"; then
        wtype "this note should never be saved"; sleep 0.3
        wtype -k Escape; sleep 1.2
        [ "$(notes_of "$uuid" | grep -c .)" -eq 4 ] \
            && ok "Escape in the box wrote nothing" \
            || bad "Escape changed the notes"
        [ -z "$(box_id)" ] && ok "box closed on Escape" || bad "box still open after Escape"
    else
        bad "note box did not reopen"
    fi
```

In the earlier "A bare Return" block, the multi-line description test no longer holds, because Enter now moves to a note. Replace that whole block (from `# A bare Return has to reach the text view` to its closing `fi`) with:

```bash
    # A bare Return in the description does not submit: it moves to the notes.
    before=$(pending)
    if open_box add; then
        wtype "first line"; wtype -k Return; wtype "a note"
        sleep 0.4
        [ "$(pending)" -eq "$before" ] && ok "bare Enter did not submit" \
            || bad "bare Enter submitted"
        wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        desc=$(task rc.verbose=nothing rc.json.array=on status:pending export 2>/dev/null \
            | python3 -c "
import json,sys
ts=json.load(sys.stdin)
print(next((t['description'] for t in ts if 'first line' in t['description']), ''))")
        [ "$desc" = "first line" ] && ok "Enter left the description on its line" \
            || bad "expected 'first line', got \"$desc\""
    else
        bad "box did not reopen"
    fi
```

Rewrite the header comment's paragraph "It covers both boxes: …" as:

```bash
# It covers the one box in all three modes: adding (with due:friday parsed and
# notes typed as rows), editing an existing task's notes in place — changed,
# deleted with Backspace, dates kept — and the menu's Note, which opens the same
# box with the cursor in a new row. Driven the only way that proves anything
# here: real keypresses. The ×, "+ Add note" and scrolling take a pointer or
# eyes, which wtype has neither of — README.md lists them as the manual check.
```

- [ ] **Step 3: Run it against the new build**

```bash
cargo build --release
NIRITASKS=./target/release/niritasks bash tests/e2e-box.sh
```

Expected: `passed: N   failed: 0`. Leave the keyboard alone while it runs; it types into whatever has focus. If the Tab-to-note-B step fails, check the focus order before changing the test: a row is `[text][date][×]`, so from note A's text, Tab should land on A's × and then B's text.

- [ ] **Step 4: Update the README.**

Line 18 becomes:

```markdown
| `Mod+Alt+T` | Add a task to this workspace, with notes — Enter starts each one |
```

Replace the paragraph "There is a third rule, in the add box's notes area: …" (l.139–143) with:

```markdown
The task box follows the same split. Adding word-splits the description, so
`due:friday` in it is a due date; editing an existing task in the box keeps it
literal.

### The task box

One window adds a task and edits one. Edit opens it on the task's description
and every note, and the menu's Note opens it the same way with the cursor in a
new empty row at the end. Each note is a row: it wraps, you edit it in place,
its date sits small at its right end, and × deletes it. "+ Add note" appends a
row.

| Key | Does |
|---|---|
| Enter in the description | Moves to the first note (making one if there are none) |
| Enter in a note | Adds a row below and moves to it |
| Backspace in an empty row | Deletes it and moves up |
| Ctrl+Enter | Saves, from anywhere |
| Esc | Discards everything, without asking |

Saving an existing task writes its description and whole note list back in
one `task import`. Untouched and edited notes keep the date they were first
written; new ones are dated when saved, and a save that changed nothing writes
nothing. Taskwarrior keeps notes in date order, so a note typed between two
old ones moves to the end once saved. Empty rows are dropped.
```

In the e2e section (l.226 onward), after the paragraph starting "Two things about `e2e-box.sh`", add:

```markdown
It cannot click, so after a change to `src/taskbox.rs` check the pointer half by
hand: × deletes its row, "+ Add note" appends an empty row with the cursor in
it, and a long note list scrolls, keeping a row made by Enter at the bottom in
view.
```

- [ ] **Step 5: Update `CONTEXT.md`.** Replace the Task box entry with:

```markdown
**Task box**:
The GTK window for adding a task or editing one — its description and its
note rows. Edit and Note open the same box.
_Avoid_: dialog, prompt

**Note row**:
One note in the task box: its text, wrapping and editable in place, its date,
and an × to delete it.
_Avoid_: line, annotation (Taskwarrior's word for what a note row is saved as)
```

- [ ] **Step 6: Run everything and commit**

```bash
cargo test
NIRITASKS=./target/release/niritasks bash tests/all.sh
```

Expected: `cargo test` passes. `tests/all.sh` shows 0 failed; suites whose prerequisites are missing may be skipped.

```bash
git add tests/e2e-box.sh README.md CONTEXT.md
git commit -m "Drive the one-window task box end to end, and document its keys

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Done when (from the spec)

- Edit on a planned task shows its description and every note as readable rows. *(Task 4 Step 9, by eye; Task 5 e2e)*
- Notes can be changed, deleted and added, and after Ctrl+Enter `task export` shows exactly that list, with unchanged notes' dates intact. *(Task 1 write_path; Task 5 e2e)*
- Add still files `due:friday` as a due date. *(Task 5 e2e; write_path's existing attribute test)*
- `cargo test` and `tests/e2e-box.sh` pass. *(Task 5 Step 6)*
