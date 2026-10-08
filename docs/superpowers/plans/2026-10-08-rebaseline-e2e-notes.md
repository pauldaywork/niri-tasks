# Rebaseline the E2E Notes Checks Plan

**Goal:** Taskwarrior task `abda1b55` ("test: Rebaseline the e2e notes checks
that a card hint now changes"). The notes block of `tests/e2e-panel.sh` fails
two checks on every run; make it assert what it means again.

**Cause:** Commit 378d7a5 made a card with notes say "Space: view notes" in
its hint. Adding a note to every task now changes the focused card's hint
from "Ctrl+Enter: Refine" to "Space: view notes · Ctrl+Enter: Refine". Two
checks assumed adding notes changes nothing on screen:

- "a task's notes stay hidden until its card is pressed" holds `noted`
  against `keyboard_down`, the frame from before the notes.
- "a second Enter hides them, the panel back as it was" holds
  `notes_hidden` against `keyboard_down` too.

Both differ by a few pixels of hint text. Fewer than 20 pixels change in any
line, under `measure`'s `MIN_RUN`, so `measure` prints `0 0 0 0` and the bad
line reports "columns 0-0, rows 0-0", which says nothing useful.

**Change (tests/e2e-panel.sh only):**

1. The first check asserts that adding notes leaves the panel's size and
   place alone: `measure noted` equals `measure keyboard_down`, both against
   the baseline. Its bad line reports both measurements.
2. The second Enter check holds `notes_hidden` against `noted`, the frame
   after the notes were added and before any press.
3. A helper next to `measure`, `whereabouts label against`, says where two
   frames differ, or that the change is too small for `measure` to place.
   Every bad line in the notes block (both Enter checks and both Space
   checks) uses it, so a tiny change is never reported as 0-0 again.
4. The block's comments say the re-render the notes bring changes only the
   focused card's hint, so the frames after it are compared among
   themselves.

No Rust changes.

**Verification:**

- `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh`
  ends with `failed: 0`.
- `cargo test --bin niritasks` still passes, as a sanity check.
- Never run `install.sh`, `cargo install` or `systemctl` from the worktree.
