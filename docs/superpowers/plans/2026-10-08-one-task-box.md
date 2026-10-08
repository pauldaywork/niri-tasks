# One Task Box Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The daemon shows at most one task box: Mod+Alt+T, or Edit/Note on a card, while a box is open brings that box forward instead of opening a second.

**Architecture:** `src/taskbox.rs` remembers the open box in a `thread_local` holding a weak reference to its window and the `Subject` it is open on (adding, or a task's uuid), cleared when the window is destroyed. `open_in` presents the open box and drops the request when there is one, and returns an `Opened` (`New`, `Same`, `Other`) saying what it did. The daemon notifies "a task box is already open" only on `Other`. `tests/e2e-box.sh` gains a suite that proves it in a nested niri.

**Tech Stack:** Rust (gtk4, glib), bash e2e scripts in a nested niri, wtype.

**Spec:** Taskwarrior task `c8e364dd-828f-4a45-aaf6-5bd9832f742d` ("fix: Keep the daemon to one task box"); read it with `task rc.json.array=on c8e364dd-828f-4a45-aaf6-5bd9832f742d export`. Its notes are the spec:

- Goal: The daemon shows at most one task box. Pressing Mod+Alt+T, or Edit/Note on a card, while one is open must not open a second.
- Decided: While a box is open, a new request brings that box forward with `present()` and is dropped, so unsaved text is never lost.
- Decided: When the dropped request was for a different box (adding versus a task, or another task), a `notify::tasks` notice says a task box is already open.
- Decided: `taskbox.rs` tracks the open box in a `thread_local` weak reference to its window, cleared when it closes. `open_in` reports whether it opened a box.
- Done when: Pressing Mod+Alt+T twice leaves one task box in `niri msg windows`. After Esc or a save, the next press opens a new box. `e2e-box.sh` checks this and passes.
- Out of scope: The panel and the project list windows.

**One refinement of the spec:** "`open_in` reports whether it opened a box" is met by returning `Opened` rather than `bool`. A bool cannot tell the daemon whether the refused request was for the *same* box (stay silent) or a *different* one (notify), and the spec asks for exactly that split. `Opened::New` is the spec's `true`; `Same` and `Other` are its `false`.

## Global Constraints

- Commit messages are Conventional Commits, `<type>(<scope>): <summary>`, imperative, lowercase first word, no full stop, subject ≤ 72 chars (aim ~50), body wrapped at 72 saying what and why. This task is a `fix`, so its commits are `fix(...)` (docs and tests folded in). End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- `cargo test` passes and `cargo build` has no warnings after every task.
- Work in this worktree branch, `task/fix-keep-the-daemon-to-one-task-box-c8e364dd` (`/home/paul/.worktrees/niri-tasks/task-fix-keep-the-daemon-to-one-task-box-c8e364dd`). The `finish-worktree` skill lands it on `main` at the end — not `superpowers:finishing-a-development-branch`.
- **Never run `install.sh`, `cargo install`, or `systemctl --user restart niri-tasks`** from the worktree: that swaps the user's live daemon onto this branch's build. Run the e2e script only as `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-box.sh`, so it tests this worktree's binary. If the machine cannot run it (no Wayland session, no wtype, or it fails on an inherited `HERDR_SESSION`, a known harness problem, task a030807c), say so in the report rather than claiming it ran.
- Do not touch the panel or the project list windows (out of scope).
- Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Comments are full sentences that say why, in the house style of the surrounding code; docs on every new pub item.
- Every GTK touch stays on the main thread; the new `thread_local` is only ever read there, as `STYLE` and `PANELS` already are.

---

### Task 1: `taskbox.rs` tracks the open box and `open_in` refuses a second

**Files:**
- Modify: `src/taskbox.rs` — `BoxConfig` (around lines 80–124), `open_in` (line 126), the `thread_local!` (line 130), `build_window` (line 169 and its last lines, ~375–381), `mod tests` at the end.
- No change to `src/daemon.rs` yet: it ignores the new return value until Task 2, and `Opened` is not `#[must_use]`, so there is no warning.

**Interfaces:**
- Consumes: `crate::task::Task` (its `uuid: String`, `description`, `annotations`).
- Produces (Task 2 relies on these exact names):
  - `pub enum Subject { Add, Task(String) }` — `Debug, Clone, PartialEq, Eq`.
  - `pub enum Opened { New, Same, Other }` — `Debug, Clone, Copy, PartialEq, Eq`.
  - `BoxConfig` gains `pub subject: Subject`; `BoxConfig::add` sets `Subject::Add`, `BoxConfig::for_task` sets `Subject::Task(task.uuid)`.
  - `pub fn open_in(app: &Application, cfg: BoxConfig, on_submit: impl Fn(Submission) + 'static) -> Opened`.
  - private `fn opened(open: Option<&Subject>, asked: &Subject) -> Opened` (pure; the unit-tested decision).
  - private `fn build_window(...) -> ApplicationWindow` (was `()`).

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests` in `src/taskbox.rs` (before its closing `}`):

```rust
    /// A box remembers what it is open on: adding, in either add box, or one
    /// task by its uuid, in either Edit or Note.
    #[test]
    fn a_box_knows_what_it_is_open_on() {
        assert_eq!(BoxConfig::add("proj", false).subject, Subject::Add);
        assert_eq!(BoxConfig::add("proj", true).subject, Subject::Add);
        let t: crate::task::Task = serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap();
        assert_eq!(BoxConfig::for_task(Mode::Note, t).subject, Subject::Task("u".into()));
    }

    /// With no box open a request opens one. With one open, it is the same box
    /// only when it is open on the same thing; anything else is another box.
    #[test]
    fn a_second_request_finds_the_open_box() {
        let task = |uuid: &str| Subject::Task(uuid.into());
        assert_eq!(opened(None, &Subject::Add), Opened::New);
        assert_eq!(opened(None, &task("a")), Opened::New);
        assert_eq!(opened(Some(&Subject::Add), &Subject::Add), Opened::Same);
        assert_eq!(opened(Some(&task("a")), &task("a")), Opened::Same);
        assert_eq!(opened(Some(&Subject::Add), &task("a")), Opened::Other);
        assert_eq!(opened(Some(&task("a")), &Subject::Add), Opened::Other);
        assert_eq!(opened(Some(&task("a")), &task("b")), Opened::Other);
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib taskbox::tests`
Expected: compile errors — `cannot find type Subject`, `no field subject on BoxConfig`, `cannot find function opened`, `cannot find type Opened`.

- [ ] **Step 3: Add `Subject`, `Opened` and `opened`**

In `src/taskbox.rs`, directly after the `impl Mode { ... }` block (ends around line 53), add:

```rust
/// What a box is open on, so a second request can tell the box it asked for
/// from another one. Edit and Note on one task are the same box, as are the
/// two add boxes: only where the cursor starts, or the default button, differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A new task.
    Add,
    /// An existing task, by its full uuid.
    Task(String),
}

/// What `open_in` did with a request. There is only ever one box, so a
/// request while one is open brings that box forward and is dropped: opening
/// a second over it, or replacing it, would lose whatever was typed there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// No box was open, so this one is now.
    New,
    /// The box asked for was the one already open.
    Same,
    /// Another box was open; it is the one brought forward.
    Other,
}

/// What a request for a box on `asked` does while `open` is the box on
/// screen, if any.
fn opened(open: Option<&Subject>, asked: &Subject) -> Opened {
    match open {
        None => Opened::New,
        Some(open) if open == asked => Opened::Same,
        Some(_) => Opened::Other,
    }
}
```

- [ ] **Step 4: Give `BoxConfig` its subject**

In `pub struct BoxConfig`, after `pub mode: Mode,`, add:

```rust
    /// What the box is open on, which is how a second request finds it.
    pub subject: Subject,
```

In `BoxConfig::add`, after `mode: Mode::Add,` add `subject: Subject::Add,`.

In `BoxConfig::for_task`, after `mode,` add `subject: Subject::Task(task.uuid),` (moving `uuid` out of `task` alongside `description` and `annotations` is fine — they are distinct fields).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --lib taskbox::tests`
Expected: all `taskbox::tests` PASS, including the two new ones.

- [ ] **Step 6: Track the open window**

`RefCell` is already imported (`use std::cell::{OnceCell, RefCell};`). Add to the existing `thread_local!` block in `src/taskbox.rs`, after `STYLE`:

```rust
    /// The box on screen and what it is open on, so a second request brings it
    /// forward rather than opening another box over it. Weak, so this never
    /// keeps a closed box alive; the window's destroy handler empties it, and
    /// a reference that no longer upgrades counts as no box either. Only ever
    /// touched on the GTK main thread.
    static OPEN: RefCell<Option<(gtk4::glib::WeakRef<ApplicationWindow>, Subject)>> =
        const { RefCell::new(None) };
```

Make `build_window` return its window. Change its signature to:

```rust
fn build_window(app: &Application, cfg: &BoxConfig, on_submit: Rc<dyn Fn(Submission)>) -> ApplicationWindow {
```

and after its final `match cfg.mode { ... }` (the "Where the cursor starts" block, last statement of the function), add the tail expression:

```rust
    window
```

- [ ] **Step 7: Make `open_in` present the open box and refuse**

Replace `open_in` and its doc comment with:

```rust
/// Open the box inside the daemon's running Application, calling `on_submit`
/// with what was saved. The window is built once, in `build_window`.
///
/// While a box is open this opens nothing: it brings the open box forward and
/// drops the request, saying whether that box was the one asked for, so the
/// daemon can tell you when it was not.
pub fn open_in(app: &Application, cfg: BoxConfig, on_submit: impl Fn(Submission) + 'static) -> Opened {
    let open = OPEN.with(|o| {
        o.borrow()
            .as_ref()
            .and_then(|(window, subject)| Some((window.upgrade()?, subject.clone())))
    });
    if let Some((window, subject)) = open {
        window.present();
        return opened(Some(&subject), &cfg.subject);
    }
    let window = build_window(app, &cfg, Rc::new(on_submit));
    // Destroy, not close-request: every way out — Esc, Cancel, a save, niri's
    // close-window — ends in it, and it comes after the window is gone.
    window.connect_destroy(|_| {
        OPEN.with(|o| {
            o.borrow_mut().take();
        });
    });
    OPEN.with(|o| *o.borrow_mut() = Some((window.downgrade(), cfg.subject)));
    Opened::New
}
```

Note: the destroy handler captures nothing, so it adds no reference cycle (see the "Every handler below holds the window weakly" comment in `build_window`). `OPEN` is never borrowed while a window is closing — `window.close()` runs from key and button handlers, not from inside an `OPEN.with` — so the `borrow_mut` there cannot panic.

- [ ] **Step 8: Build and test**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test`
Expected: no warnings or errors from the build; every test passes. `daemon.rs` compiles unchanged because it discards `open_in`'s return value as a statement expression.

- [ ] **Step 9: Commit**

```bash
git add src/taskbox.rs
git commit -m "$(cat <<'EOF'
fix(taskbox): keep one box open and bring it forward

A request for a task box while one was open built a second window
over it. The box now remembers the open window, weakly, and what it
is open on; a second request presents that box and is dropped, so
nothing typed into it is lost. open_in says whether it opened a box,
the same one was open, or another one was.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: The daemon says another box is open; e2e proves one box; docs

**Files:**
- Modify: `src/daemon.rs` — consts after `NO_PANEL` (~line 190), the `Request::Add` arm of `serve_box_request` (~lines 245–278), `open_task_box` (~lines 285–305), `mod tests`.
- Modify: `tests/e2e-box.sh` — helpers after `close_any_box` (~line 140), a new `run_one_box_suite` before `nested_start` (~line 410), and its call after `run_refine_suite "served by the daemon"`.
- Modify: `README.md` — `### The task box` paragraph (~lines 289–297).
- Modify: `CONTEXT.md` — the **Task box** entry (~lines 157–161).

**Interfaces:**
- Consumes: `taskbox::Opened { New, Same, Other }` and `taskbox::open_in(...) -> Opened` from Task 1.
- Produces: `pub const BOX_ALREADY_OPEN: &str` and private `fn already_open_notice(opened: crate::taskbox::Opened) -> Option<&'static str>` in `daemon.rs`.

- [ ] **Step 1: Write the failing unit test**

Append inside `mod tests` in `src/daemon.rs`:

```rust
    /// Only a request for a different box hears that one is already open: the
    /// same box asked for twice is simply brought forward, which says enough.
    #[test]
    fn only_another_box_says_one_is_open() {
        use crate::taskbox::Opened;
        assert_eq!(already_open_notice(Opened::New), None);
        assert_eq!(already_open_notice(Opened::Same), None);
        assert_eq!(already_open_notice(Opened::Other), Some(BOX_ALREADY_OPEN));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --lib daemon::tests`
Expected: compile errors — `cannot find function already_open_notice`, `cannot find value BOX_ALREADY_OPEN`.

- [ ] **Step 3: Add the notice**

In `src/daemon.rs`, after the `NO_PANEL` const, add:

```rust
/// What asking for a task box says while a different one is open. That box is
/// brought forward instead and the request dropped, so this says why the box
/// asked for did not appear.
pub const BOX_ALREADY_OPEN: &str = "A task box is already open — save or discard it first.";

/// The notice for what `taskbox::open_in` did, if it needs one.
fn already_open_notice(opened: crate::taskbox::Opened) -> Option<&'static str> {
    (opened == crate::taskbox::Opened::Other).then_some(BOX_ALREADY_OPEN)
}
```

In the `Request::Add { refine }` arm, change `taskbox::open_in(` to `let opened = taskbox::open_in(`, and after that call's closing `);` add:

```rust
            if let Some(notice) = already_open_notice(opened) {
                notify::tasks(notice);
            }
```

In `open_task_box`, do the same: `let opened = taskbox::open_in(`, and after its closing `);`:

```rust
    if let Some(notice) = already_open_notice(opened) {
        notify::tasks(notice);
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test`
Expected: no warnings or errors; every test passes, including `daemon::tests::only_another_box_says_one_is_open`.

- [ ] **Step 5: Add the e2e helpers**

In `tests/e2e-box.sh`, after the `close_any_box() { ... }` function, add:

```bash
# How many task boxes niri has.
box_count() {
    nested niri msg -j windows 2>/dev/null | python3 -c "
import json,sys
print(sum(1 for w in json.load(sys.stdin) if (w.get('app_id') or '')=='dev.niri-tasks.box'))
" 2>/dev/null || echo 0
}
# Ask for a box while one is already open. Unlike open_box this waits for no
# window — none should come — only for the CLI to hand the request over and
# the daemon to act on it.
ask_again() {
    "${NENV[@]}" "$NIRITASKS" task "$@" >/dev/null 2>&1
    sleep 1
}
# How many times the daemon has said a task box is already open.
already_open_notices() { cat "$SB/notifications" 2>/dev/null | grep -c "already open"; }
```

- [ ] **Step 6: Add the suite**

Before the `nested_start` line near the end of `tests/e2e-box.sh`, add:

```bash
run_one_box_suite() {
    local label="$1" mark other uuid notices
    echo
    echo "=== $label: one box ==="

    # A task to ask for a note box on, made from the CLI before any box is up
    # so no keys are in flight.
    other="other-$RANDOM"
    nested "$NIRITASKS" task add "$other" >/dev/null 2>&1
    uuid=$(uuid_of "$other")
    [ -n "$uuid" ] || { bad "could not seed a task for the note request"; return; }

    mark="one-$RANDOM"
    if open_box add; then
        settle_keys
        nested wtype "$mark"; sleep 0.3
        notices=$(already_open_notices)

        # Mod+Alt+T again: the same box, brought forward, and nothing said.
        ask_again add
        [ "$(box_count)" -eq 1 ] && ok "a second Mod+Alt+T left one box" \
            || bad "a second Mod+Alt+T left $(box_count) boxes"
        [ "$(already_open_notices)" -eq "$notices" ] && ok "and said nothing, being the same box" \
            || bad "the same box asked for twice sent a notice"

        # Note on a card: a different box, so it is dropped with a notice.
        ask_again note "$uuid"
        [ "$(box_count)" -eq 1 ] && ok "a note request left one box" \
            || bad "a note request left $(box_count) boxes"
        [ "$(box_title)" = "Add Task" ] && ok "and the add box stayed in front" \
            || bad "focused box was \"$(box_title)\""
        [ "$(already_open_notices)" -gt "$notices" ] && ok "and said a task box is already open" \
            || bad "no notice (notifications: $(tail -n 3 "$SB/notifications" 2>/dev/null | tr '\n' '|'))"

        # What was typed before both requests is still there to save.
        nested wtype -M ctrl -k Return -m ctrl
        sleep 1.5
        [ -n "$(uuid_of "$mark")" ] && ok "the text typed before them was saved" \
            || bad "the text typed before the second request was lost"
        [ "$(box_count)" -eq 0 ] && ok "and the save closed the only box" \
            || bad "$(box_count) boxes open after the save"
    else
        bad "add box never opened"; return
    fi

    # After a save, and after Esc, the next press opens a box again.
    if open_box add; then
        ok "after a save the next Mod+Alt+T opens a box"
        nested wtype -k Escape; sleep 1.2
        [ "$(box_count)" -eq 0 ] || bad "Escape left a box open"
        if open_box add; then
            ok "after Esc the next Mod+Alt+T opens a box"
        else
            bad "no box opened after Esc"
        fi
    else
        bad "no box opened after a save"
    fi

    close_any_box
    guard
}

```

And after the existing line `run_refine_suite "served by the daemon"`, add:

```bash
run_one_box_suite "served by the daemon"
```

- [ ] **Step 7: Run the e2e script**

Run: `cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-box.sh`
Expected: every line `PASS`, including the new `one box` checks, and exit 0. If the machine cannot run it (see Global Constraints), report that rather than claiming a pass. Do not use `git stash` to try the suite against the old code: the stash stack is shared with other worktrees.

- [ ] **Step 8: Update the docs**

In `README.md`, under `### The task box`, after the paragraph ending "…and a notification says why.", add a new paragraph:

```markdown
There is only ever one box. Mod+Alt+T, or Edit or Note on a card, while a box
is open brings that box forward and opens nothing, so nothing typed into it is
lost; if you asked for a different box, a notification says one is already
open. Save or discard it, then ask again.
```

In `CONTEXT.md`, in the **Task box** entry, after "Mod+Alt+Shift+T opens the box with that as the default.", add on the next line:

```markdown
Only one is ever open: asking for another brings the open one forward.
```

- [ ] **Step 9: Run the full checks**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo test`
Expected: no warnings or errors; every test passes (the README and llms.txt checks in `src/main.rs` are untouched by these edits).

- [ ] **Step 10: Commit**

```bash
git add src/daemon.rs tests/e2e-box.sh README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
fix(daemon): say when a different task box is already open

Asking for a box while another is open now brings that one forward
and drops the request; the daemon says so, unless the box asked for
is the one already open. e2e-box.sh checks that two presses leave
one box, that the typed text survives, and that a box opens again
after a save or Esc.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
