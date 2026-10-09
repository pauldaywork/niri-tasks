# Split taskbox.rs: the box, its one-box guard, its notes, and the paint helper

Date: 2026-10-09. Taskwarrior task `08b2cc4f`. Closes review card T1 (v3/v4).

## Problem

`src/taskbox.rs` is 826 lines holding four concerns: the one-box bookkeeping (`Subject`, `Opened`, `opened()`, the `OPEN` thread-local, `open_in`, `focus_through_niri`), the window itself (`Mode`, `BoxConfig`, the stylesheet, `build_window` and its settle-and-scroll helpers), the Notes widget (`Notes`, `NoteRow`), and a general GTK frame-clock helper, `after_next_paint`, which the panel imports from the box module (`src/panel/surface.rs:1201`). The module already has a folder with `form`, `keys` and `style`; the split follows it.

## Decisions

1. **`src/paint.rs`** takes `after_next_paint` verbatim, `pub`, with its doc. The box and the panel both call `crate::paint::after_next_paint`; neither GUI module imports the other.
2. **`src/taskbox/open.rs`** takes the one-box guard: `Subject`, `Opened`, `opened()`, the `OPEN` thread-local, `open_in`, `focus_through_niri`, and the test `a_second_request_finds_the_open_box`. It calls `super::build_window` (which becomes `pub(super)`). `taskbox.rs` re-exports `pub use open::{open_in, Opened, Subject};` so `daemon.rs` does not change.
3. **`src/taskbox/notes.rs`** takes `Notes`, `NoteRow` and `impl Notes`, with whatever visibility (`pub(super)`) `build_window` needs on their fields and methods. The text-view helpers `text_view`, `buffer_text`, `focus_end` stay in `taskbox.rs` as `pub(super)`; `notes.rs` calls `super::text_view`/`super::focus_end`.
4. **`taskbox.rs` keeps** `Mode`, `hint`, the size constants, `APP_ID`, `BoxConfig`, `STYLE`/`install_style`, `build_window`, `settle_after_opening`, `follow_focus`, `scroll_focus_into_view`, the text-view helpers, and the remaining tests. Its module doc names the submodules and what each holds.
5. **No behaviour change.** Same window, same keys, same one-box decision, same focus-through-niri. `tests/e2e-box.sh` is not run (it drives the screen); the unit tests move with their code.

## Out of scope

Any change to the box's look or keys; splitting `build_window` further; the daemon's calls (unchanged by the re-exports).

## Tests

Moved: `a_second_request_finds_the_open_box` → `open.rs`. Stay: `titles_match_the_mode`, `a_task_opens_with_its_description_and_every_note`, `an_add_box_says_which_button_ctrl_enter_presses`, `an_existing_task_never_opens_to_refine`, `the_hint_names_the_default_button`, `a_box_knows_what_it_is_open_on`. `cargo test` green; `cargo build` warning-free.
