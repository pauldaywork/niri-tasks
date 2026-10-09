# Scroll a Card's Notes Past a Max Height Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On the keyboard task panel, a card's notes area (the id/uuid line and the notes) stops growing at `NOTES_MAX_PX` and anything longer scrolls inside it, so long notes no longer make a card taller than the panel.

**Architecture:** `notes_box` in `src/panel/surface.rs` still builds the column of labels. A new `notes_scroller` wraps it in a `gtk4::ScrolledWindow` with `propagate_natural_height(true)` and `max_content_height(NOTES_MAX_PX)`, so short notes stay their natural height and only long ones scroll. GTK works out the height itself, and `fit()` already reads it through `column.measure(...)`, so the surface, the input region and the blur fit the capped card with no change to `layout.rs` beyond the new const. The scroller, not the column, is what `card_widget` and `show_notes` show and hide, and it carries the `card-notes` class.

**Tech Stack:** Rust, gtk4-rs (GTK 4), gtk4-layer-shell, bash + Pillow e2e test in a nested niri.

**Spec:** Taskwarrior task `e7daec96-a497-447a-8f7b-e6f2c00dd11a`. Read it with `task rc.json.array=on e7daec96-a497-447a-8f7b-e6f2c00dd11a export`; its description and annotations are the spec.

## Global Constraints

- The id/uuid line and every note scroll together as one area.
- The cap is a new const `NOTES_MAX_PX`, about 240px (roughly 12 lines), next to `PROJECT_LIST_MAX_PX` in `src/panel/layout.rs`.
- Follow the notepad's and `scroller()`'s idiom: `gtk4::ScrolledWindow` with `hscrollbar_policy(Never)`, adding `propagate_natural_height(true)` and `max_content_height(NOTES_MAX_PX)`.
- Keep `card-notes` on the outer widget so the CSS still applies.
- Scrolling is by mouse wheel, touchpad or the scrollbar. A click on the notes still hides them. Dragging the scrollbar or scrolling must not toggle them.
- Short notes look exactly as before.
- Done when `cargo test` passes and `tests/e2e-panel.sh` passes against the debug build (`NIRITASKS=$PWD/target/debug/niritasks`).
- Out of scope: keyboard keys for scrolling the notes (Up/Down still move between cards), the peek and the hover, and editing notes from the panel.
- Commits: Conventional Commits, scope `panel`, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Why there is no `layout.rs` unit test

The spec asks for a layout test "if the height maths lives in `layout.rs`". It does not: the cap is applied by GTK's `ScrolledWindow` when `fit()` measures the column (`src/panel/surface.rs`, `self.column.measure(...)`), and `Layout` only receives the measured heights. There is no Rust function whose output changes, so the test that goes red first is a new check in `tests/e2e-panel.sh` that measures the pixels.

## Background for the implementer

- **How the panel sizes itself.** `Panel::fit()` measures each widget's natural height with `widget.measure(gtk4::Orientation::Vertical, width).1` and hands the numbers to `Layout::new`. `refit()` calls `fit()` and then moves the input region and the blur. `show_notes()` calls `refit()` when any card's notes showed or hid. A `ScrolledWindow` with `propagate_natural_height(true)` reports `min(child's natural height, max_content_height)` as its natural height, plus its own CSS padding, so `fit()` gets the capped height for free.
- **The cap's total.** `.card-notes` has `padding-top: PADDING_PX` (12px, `src/panel/style.rs:184`). The padding sits on the scroller, outside its content, so a capped notes area adds `12 + NOTES_MAX_PX` = 252px to a card.
- **Clicks.** The notes are inside the card's body `gtk4::Button`. Labels do not take clicks, so a click on the notes still reaches the button's click gesture and hides them. A `ScrolledWindow`'s own drag and long-press gestures are touch-only. Its scrollbar (a `GtkRange`) claims the pointer sequence on press, which cancels the button's click gesture, so dragging the scrollbar does not toggle the notes. The wheel goes to the scroller's scroll controller, which the button never sees. With `PolicyType::Automatic` and short notes the scroller has nothing to scroll and passes the wheel up to the panel's own scroller, so the list still scrolls over a short-noted card.
- **The e2e test.** `tests/e2e-panel.sh` runs its own daemon in a nested niri on a flat 1000px-tall screen, presses keys with `wtype`, takes screenshots and measures where they differ from a baseline. `measure <label>` prints `x0 x1 y0 y1` of the panel. `keyboard_h` (set at line 261) is the panel's height with the keyboard and no notes shown. `ok` and `bad` record a pass or a fail. Read the notes section, lines 322–386, before editing.

## File Structure

- `src/panel/layout.rs`: gains `NOTES_MAX_PX`, the cap, beside `PROJECT_LIST_MAX_PX`.
- `src/panel/surface.rs`: `notes_box` loses its CSS class; new `notes_scroller` wraps it; `CardWidgets::notes` becomes `Option<gtk4::ScrolledWindow>`; `card_widget` builds the scroller.
- `tests/e2e-panel.sh`: mirrors `NOTES_MAX_PX` and `PADDING_PX`; one new block checks a long-noted card is capped and hides again.
- `CONTEXT.md` (Task card), `README.md` (line 73 and the by-hand list under "Testing"): say the notes area is capped and scrolls.

---

### Task 1: Cap the notes area and scroll what is past it

**Files:**
- Modify: `tests/e2e-panel.sh:58-63` (mirrored constants) and after line 386 (new block, after the second Space)
- Modify: `src/panel/layout.rs:72-75` (new const after `PROJECT_LIST_MAX_PX`)
- Modify: `src/panel/surface.rs:147-150` (import), `:248-250` (`CardWidgets::notes`), `:1009-1021` (`card_widget`), `:1367-1383` (`notes_box`, new `notes_scroller`)
- Modify: `CONTEXT.md:79-83`, `README.md:73` and `README.md:395-397`

**Interfaces:**
- Consumes: `notes_box(id_line: &str, notes: &[String]) -> gtk4::Box` (existing), `PanelState::shows_notes(&self, uuid: &str) -> bool` (existing).
- Produces: `pub(crate) const NOTES_MAX_PX: i32 = 240;` in `src/panel/layout.rs`; `fn notes_scroller(column: &gtk4::Box) -> gtk4::ScrolledWindow` in `src/panel/surface.rs`; `CardWidgets::notes: Option<gtk4::ScrolledWindow>`.

- [ ] **Step 1: Mirror the cap and the padding in the e2e script**

In `tests/e2e-panel.sh`, replace lines 58–63:

```bash
# Mirror PEEK_PX and SURFACE_WIDTH in src/panel/layout.rs, and RING_PX and
# CARD_WIDTH_PX in src/panel/style.rs.
PEEK=30
RING=3
CARD=760
SURFACE=784
```

with:

```bash
# Mirror PEEK_PX, SURFACE_WIDTH and NOTES_MAX_PX in src/panel/layout.rs, and
# RING_PX, CARD_WIDTH_PX and PADDING_PX in src/panel/style.rs.
PEEK=30
RING=3
CARD=760
SURFACE=784
NOTES_MAX=240
PADDING=12
```

- [ ] **Step 2: Write the failing e2e check**

In `tests/e2e-panel.sh`, directly after the block that ends

```bash
    else
        bad "after a second Space the screen differs: $(whereabouts notes_space_hidden notes_hidden)"
    fi
```

(line 386) and before the `# Escape from a tab other than All:` comment, insert:

```bash

    # A card with more notes than fit shows them in an area capped at
    # NOTES_MAX tall, the padding over it, and scrolls the rest inside it:
    # thirty more notes on every task, so whichever card has the focus has
    # them, run far past the cap at a line apiece. Hidden again, the panel
    # is as it was with them hidden.
    for uuid in $(task "+$TAG" _uuids 2>/dev/null); do
        for n in $(seq 1 30); do
            task rc.verbose=nothing rc.confirmation=no "$uuid" annotate -- "long note $n" >/dev/null 2>&1
        done
    done
    settle
    shot long_noted || { summary; exit 1; }
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot long_notes_shown || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure long_notes_shown)
    grown=$((y1 - y0 - keyboard_h))
    if [ "$grown" -ge "$NOTES_MAX" ] && [ "$grown" -le $((PADDING + NOTES_MAX)) ]; then
        ok "thirty-one lines of notes stop at the cap (${grown}px, at most $((PADDING + NOTES_MAX))px)"
    else
        bad "a long-noted card grew ${grown}px, expected ${NOTES_MAX}-$((PADDING + NOTES_MAX))px —
      the notes area should stop at NOTES_MAX_PX and scroll the rest"
    fi
    "${NENV[@]}" wtype -k Return
    sleep 1
    shot long_notes_hidden || { summary; exit 1; }
    if same long_noted long_notes_hidden; then
        ok "and a second Enter hides them, the panel back as it was"
    else
        bad "after hiding the long notes the screen differs: $(whereabouts long_notes_hidden long_noted)"
    fi
```

- [ ] **Step 3: Run the e2e test against the debug build to see it fail**

Run:

```bash
cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh
```

Expected: every check passes except the new one, which prints `a long-noted card grew …px, expected 240-252px` with a number well over 252 (about 30 lines' worth, or the screen's height). The hide check after it passes. Keep off the nested niri's workspace while it runs (see the script's header).

- [ ] **Step 4: Add the cap to `layout.rs`**

In `src/panel/layout.rs`, after line 75 (`pub(crate) const PROJECT_LIST_MAX_PX: i32 = 800;`), add:

```rust

/// The tallest a card's notes area grows, the id line and every note
/// together, before they scroll inside it: about twelve lines. Without it a
/// task with long notes makes its card taller than the panel. The
/// `card-notes` padding over it comes on top.
pub(crate) const NOTES_MAX_PX: i32 = 240;
```

- [ ] **Step 5: Import it in `surface.rs`**

In `src/panel/surface.rs`, replace lines 147–150:

```rust
use super::layout::{
    centre_margin, scroll_to_show, Layout, Measured, CENTRED_X, EXPANDED_X, PROJECT_LIST_MAX_PX,
    SURFACE_WIDTH, TUCKED_X,
};
```

with:

```rust
use super::layout::{
    centre_margin, scroll_to_show, Layout, Measured, CENTRED_X, EXPANDED_X, NOTES_MAX_PX,
    PROJECT_LIST_MAX_PX, SURFACE_WIDTH, TUCKED_X,
};
```

- [ ] **Step 6: Wrap the notes column in a capped scroller**

In `src/panel/surface.rs`, replace `notes_box` (lines 1367–1383):

```rust
/// A task's notes, for under its card's description: first the line of its
/// id and uuid, to read off for a `task` or `niritasks` command, then one
/// label per note, all dimmed alike, wrapped, text only. The panel shows the
/// box once the card's body is pressed.
fn notes_box(id_line: &str, notes: &[String]) -> gtk4::Box {
    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    column.add_css_class("card-notes");
    for note in std::iter::once(id_line).chain(notes.iter().map(String::as_str)) {
```

with:

```rust
/// A task's notes, for under its card's description: first the line of its
/// id and uuid, to read off for a `task` or `niritasks` command, then one
/// label per note, all dimmed alike, wrapped, text only. `notes_scroller`
/// holds the column, and the panel shows it once the card's body is pressed.
fn notes_box(id_line: &str, notes: &[String]) -> gtk4::Box {
    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    for note in std::iter::once(id_line).chain(notes.iter().map(String::as_str)) {
```

and directly after `notes_box`'s closing `}` add:

```rust

/// The notes column's scroller: as tall as the notes up to `NOTES_MAX_PX`,
/// past which they scroll inside it, the id line with them, by wheel,
/// touchpad or the scrollbar. Short notes take their own height, as they
/// did unscrolled. A click on them still reaches the body and hides them;
/// the scrollbar takes its own presses, so dragging it does not. It wears
/// `card-notes`, so the padding stays over the area rather than scrolling.
fn notes_scroller(column: &gtk4::Box) -> gtk4::ScrolledWindow {
    let scroller = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .propagate_natural_height(true)
        .max_content_height(NOTES_MAX_PX)
        // Out of the focus chain: the state moves the focus between cards
        // and along their buttons, never onto the notes.
        .focusable(false)
        .child(column)
        .build();
    scroller.add_css_class("card-notes");
    scroller
}
```

- [ ] **Step 7: Show and hide the scroller instead of the column**

In `src/panel/surface.rs`, in `struct CardWidgets` (around line 248–250), change the field's type:

```rust
    notes: Option<gtk4::Box>,
```

to:

```rust
    notes: Option<gtk4::ScrolledWindow>,
```

Keep its doc comment; if it names "the box", say "the scroller" instead.

In `card_widget` (line 1017), replace:

```rust
            let notes = notes_box(&id_line, &card.notes);
```

with:

```rust
            let notes = notes_scroller(&notes_box(&id_line, &card.notes));
```

The next two lines (`notes.set_visible(...)`, `content.append(&notes)`) and `show_notes()` (line 1249) need no change: they already act on whatever `notes` is, now the scroller.

- [ ] **Step 8: Build and run the unit tests**

Run: `cargo build && cargo test`
Expected: builds with no warnings from these edits; all tests pass, including `style.rs`'s `.card-notes` CSS assertion (the CSS text is unchanged).

- [ ] **Step 9: Run the e2e test to see it pass**

Run: `NIRITASKS=$PWD/target/debug/niritasks NIRITASKS_E2E_KEEP=1 bash tests/e2e-panel.sh`
Expected: every check passes, the new one printing `thirty-one lines of notes stop at the cap (252px, at most 252px)` or within 240–252. The run ends `the run's files are kept in <dir>`; the frames are in `<dir>/shots/`, and Task 2 compares its `notes_shown.png`.

If the new check fails with a number between 252 and the uncapped height, look at `long_notes_shown.png`: if the notes area is capped but the card is taller, something other than the scroller grew (for example the body's padding) — measure it before changing the expectation.

- [ ] **Step 10: Say so in CONTEXT.md and README.md**

In `CONTEXT.md`, Task card (lines 79–83), replace:

```
While the task panel has the keyboard, Enter or a click on a card's body shows,
dimmed under its description, the task's id and uuid on one line (`#48 ·
<uuid>`, the uuid alone on a finished task, which has no id) and then its
notes, one per note, until a second press or the keyboard is given back.
```

with:

```
While the task panel has the keyboard, Enter or a click on a card's body shows,
dimmed under its description, the task's id and uuid on one line (`#48 ·
<uuid>`, the uuid alone on a finished task, which has no id) and then its
notes, one per note, until a second press or the keyboard is given back. The
id line and notes together grow to about twelve lines (`NOTES_MAX_PX`), past
which they scroll inside that height by wheel, touchpad or scrollbar.
```

In `README.md` line 73, replace:

```
and then its notes, one per note, and a second press hides them;
```

with:

```
and then its notes, one per note, scrolling once they pass about twelve lines, and a second press hides them;
```

In `README.md`'s by-hand list under "Testing" (line ~397), replace:

```
Enter on a card's description shows its notes, dimmed under it, with the panel and its blur grown to fit, and a second Enter hides them,
```

with:

```
Enter on a card's description shows its notes, dimmed under it, with the panel and its blur grown to fit, and a second Enter hides them; a card with more notes than fit stops at about twelve lines and scrolls the rest by wheel and by dragging the scrollbar, neither of which hides them, while a click on the notes does,
```

- [ ] **Step 11: Commit**

```bash
git add src/panel/layout.rs src/panel/surface.rs tests/e2e-panel.sh CONTEXT.md README.md
git commit -m "$(cat <<'EOF'
feat(panel): scroll a card's notes past a max height

A task with long notes made its card taller than the panel. The id
line and notes now grow to NOTES_MAX_PX (240px, about twelve lines)
and scroll inside it by wheel, touchpad or scrollbar; short notes
keep their own height. A click on the notes still hides them.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Check short notes are unchanged and the pointer half by hand

Nothing here is committed unless it finds a fault; a fault goes back to Task 1's code with a new commit.

**Files:**
- None modified.

**Interfaces:**
- Consumes: Task 1's build and its kept e2e shots.
- Produces: nothing.

- [ ] **Step 1: Compare a short-noted card with main's**

The spec says short notes look exactly as before. The e2e's `notes_shown` frame is a card with one short note. Build main in a scratch worktree, run the e2e there with its shots kept, and compare the two `notes_shown.png` pixel for pixel:

```bash
git worktree add --detach $HOME/.worktrees/niri-tasks/notes-main-check main
(cd $HOME/.worktrees/niri-tasks/notes-main-check && cargo build && NIRITASKS=$PWD/target/debug/niritasks NIRITASKS_E2E_KEEP=1 bash tests/e2e-panel.sh)
python3 -c 'import sys; from PIL import Image, ImageChops; a,b=(Image.open(p).convert("RGB") for p in sys.argv[1:]); print("same" if a.size==b.size and ImageChops.difference(a,b).getbbox() is None else "DIFFERENT")' <main-shots>/notes_shown.png <branch-shots>/notes_shown.png
git worktree remove $HOME/.worktrees/niri-tasks/notes-main-check
```

Replace `<main-shots>` and `<branch-shots>` with `<dir>/shots` from each run's last line, `the run's files are kept in <dir>` (Task 1 Step 9 for the branch). Delete both `<dir>`s afterwards. Expected: `same`. If `DIFFERENT`, open both and find what moved (an undershoot line, a width change, a padding) before going on.

- [ ] **Step 2: Check the pointer by hand**

Install the build into the running daemon: `bash install.sh` (it restarts the daemon). Give a task 30 notes (`for n in $(seq 1 30); do task <id> annotate -- "note $n"; done`) and one note long enough to wrap. Press `Mod+Alt+Ctrl+T`, move to that card, press Enter. Check:

- The notes area stops at about twelve lines; the panel, its outline and its blur fit the capped card.
- The wheel and the touchpad over the notes scroll them to the last note, and do not hide them.
- Dragging the scrollbar scrolls them and does not hide them.
- A click on the notes hides them, as before.
- A card with one short note looks as it did, and the wheel over it still scrolls the list when the list is taller than the screen.
- The long wrapped note wraps at the card's width, no horizontal scrollbar.

Report each result. A failure is a fault in Task 1: fix it there, rerun Task 1's Steps 8–9, and commit with `fix(panel): …`.
