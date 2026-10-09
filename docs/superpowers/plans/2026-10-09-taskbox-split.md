# Taskbox Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `taskbox.rs` (826 lines) becomes the box alone; the one-box guard, the Notes widget and the frame-clock helper each get their own module; the panel stops importing the box module. No behaviour change.

**Architecture:** see `docs/superpowers/specs/2026-10-09-taskbox-split-design.md`. Taskwarrior task `08b2cc4f`.

**Tech Stack:** Rust, gtk4.

## Global Constraints

- Conventional Commits with an attribution trailer (`Co-Authored-By: <your model> <noreply@anthropic.com>`); subject ≤72, body wrapped at 72 saying what and why.
- `cargo build 2>&1 | grep -E "^(warning|error)"` prints nothing and `cargo test` passes in full after every task. Do not edit `tests/*`.
- **Move code verbatim**: bodies, docs and comments unchanged except visibility keywords and paths. A moved doc that referred to "this module" is reworded truthfully.
- Work in `/home/paul/.worktrees/niri-tasks/task-taskbox-split-08b2cc4f` (branch `task/taskbox-split-08b2cc4f`). **Never run `install.sh`, `cargo install`, `systemctl --user restart niri-tasks`, any `tests/e2e-*.sh`**, or start herdr sessions.
- Docs on every pub item; comments are full sentences that say why. Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Line numbers are anchors from main @ 9c5c6cf.

---

### Task 1: `paint.rs`

**Files:** Create `src/paint.rs`; modify `src/lib.rs` (`pub mod paint;`), `src/taskbox.rs` (delete `after_next_paint` ~:538-562; its two callers `settle_after_opening` ~:491 and `follow_focus` ~:517 call `crate::paint::after_next_paint`), `src/panel/surface.rs:1201` (`crate::taskbox::after_next_paint` → `crate::paint::after_next_paint`).

- [ ] Write `src/paint.rs` with a module doc ("Running something after GTK's next frame has been painted, when everything queued before it has been measured and placed. The task box settles its rows with it and the panel scrolls its focused card into view with it; it lives on its own so neither window imports the other.") and `pub fn after_next_paint(window: &gtk4::ApplicationWindow, f: impl FnOnce() + 'static)` moved verbatim (body, doc, the trailing `request_phase` comment). Imports: `gtk4::gdk`, `gtk4::prelude::*`, `std::cell::RefCell`, `std::rc::Rc`.
- [ ] Repoint the three callers; remove now-unused imports from taskbox.rs if any.
- [ ] `cargo build` warning grep empty; `cargo test` green. `grep -rn "after_next_paint" src` shows only paint.rs's definition and the three calls.
- [ ] Commit: `refactor(paint): move the after-paint helper out of the task box` with a body saying the panel imported the box module for one frame-clock helper, and now neither GUI module imports the other.

### Task 2: `taskbox/open.rs`

**Files:** Create `src/taskbox/open.rs`; modify `src/taskbox.rs`.

- [ ] Move to `open.rs`, verbatim: `Subject` (~:57-66), `Opened` (~:68-79), `opened()` (~:81-89), `open_in` (~:163-194), `focus_through_niri` (~:196-219), the `OPEN` thread-local (~:228-235; the `STYLE` one stays in taskbox.rs — split the `thread_local!` block), and the test `a_second_request_finds_the_open_box` (~:808-824) in its own `#[cfg(test)] mod tests`. Module doc: "One task box at a time: what a box is open on, what a second request does while one is open, and bringing the open box forward through niri. Pure decision (`opened`) apart from the GTK and niri calls, so it is tested on its own."
- [ ] In `taskbox.rs`: `pub mod open;` and `pub use open::{open_in, Opened, Subject};` beside the existing `pub use form::Submission;`; `build_window` becomes `pub(super)`; `BoxConfig.subject` stays `pub`. `open.rs` imports `super::{build_window, BoxConfig, Submission, APP_ID}` and `crate::niri`. Remove imports taskbox.rs no longer needs (`WeakRef`, `niri`, …).
- [ ] `daemon.rs` must not change (`git diff --stat` shows only taskbox files). `cargo build` warning grep empty; `cargo test` green (the moved test runs under `taskbox::open::tests`).
- [ ] Commit: `refactor(taskbox): put the one-box guard in its own module`.

### Task 3: `taskbox/notes.rs` and the module doc

**Files:** Create `src/taskbox/notes.rs`; modify `src/taskbox.rs`.

- [ ] Move `Notes`, `NoteRow` and `impl Notes` (~:591-718) verbatim to `notes.rs`, giving the struct, its fields and the methods `build_window` uses `pub(super)`. `notes.rs` imports `super::{text_view, focus_end}` (make those `pub(super)`), `crate::task::Annotation`, gtk4 bits. Module doc: "The notes list in the task box: one row per note, editable in place, with its date and an × to delete it; rows are inserted, deleted and read back as the form is submitted."
- [ ] `taskbox.rs`: `pub mod notes;` (or `mod notes;` if nothing outside needs it; prefer private), `use notes::Notes;`. Update the module doc's first paragraphs to name what lives where: `open` (one box at a time), `notes` (the notes list), `form`, `keys`, `style`, and `crate::paint` for the frame-clock helper. Target: `wc -l src/taskbox.rs` ≈ 550.
- [ ] `cargo build` warning grep empty; `cargo test` green; `cargo clippy --all-targets` adds no warning.
- [ ] Commit: `refactor(taskbox): move the notes list into its own module`.

## Self-review notes

Decision 1 → Task 1; 2 → Task 2; 3 and 4 → Task 3; 5 holds throughout (every move verbatim; daemon untouched). Each task leaves the tree building and tested.
