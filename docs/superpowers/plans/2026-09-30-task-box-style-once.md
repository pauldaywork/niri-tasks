# Task Box Stylesheet Once — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The long-running daemon holds one task-box `CssProvider` no matter how many boxes it has opened.

**Architecture:** `build_window` in `src/taskbox.rs` currently makes a new `CssProvider` from `style::css()` on every open and adds it to the whole display, and nothing ever removes it. Move that setup into a small `install_style()` that a `thread_local!` `OnceCell<CssProvider>` guards, so it runs once per process. `build_window` calls it. Both open paths still go through `build_window`: the daemon's `open_in` (called many times) and the CLI's one-shot `show`.

**Tech Stack:** Rust 2021 (rustc 1.95), gtk4-rs 0.11, `std::cell::OnceCell`.

**Spec:** Taskwarrior task `a54634c2-a30c-4105-a124-68f82bb7deb2`. Read it with `task rc.json.array=on a54634c2-a30c-4105-a124-68f82bb7deb2 export`. Its description and notes are the spec.

## Global Constraints

- Guard the install with a `thread_local` in `taskbox.rs` (a `OnceCell` holding the provider) so it runs once per process.
- Don't remove the provider when a box closes. Nothing ever needs a different stylesheet.
- Move the provider setup out of `build_window` into a small `install_style()` that returns early once it has run. Call it from `build_window`.
- Keep the `load_from_data` comment.
- Out of scope: the panel's stylesheet in `daemon.rs`, which is separate and already added once, and any change to the box's CSS itself.
- Done when: opening many boxes in the daemon adds the provider only once, boxes look the same from the daemon and the CLI, and `tests/all.sh` passes.
- House style: every item gets a doc comment that says *why*, in the voice of the surrounding code. Use `src/daemon.rs:36-40` (`PANELS`) as the model for a `thread_local!` and its comment.

## Decisions made while planning (flag any you disagree with)

1. **Mark it done only once the provider is on a display.** If `gdk::Display::default()` is `None`, `install_style` returns without filling the cell, so a later call tries again. Today's code builds the provider and quietly skips adding it when there is no display. Filling the cell anyway would make that one miss permanent. In practice `build_window` never runs without a display, so this costs nothing.
2. **No new cargo test.** `CssProvider::new()` needs GTK initialised, and GTK needs a display. `cargo test` has neither today: `tests/all.sh` runs it where only cargo and taskwarrior are needed, and no existing unit test touches GTK. Adding one would make `cargo test` fail over SSH. The guard is five lines you can check by reading them. What proves it works is a one-off count, made with temporary logging in a real daemon (Task 1, Steps 3 to 6) and removed before the commit, plus `tests/e2e-box.sh`, which opens several boxes from one daemon and then from the CLI.
3. **`std::cell::OnceCell`, not `once_cell` or `LazyCell`.** It is in std (since Rust 1.70), so there is no new dependency. `get`/`set` lets the display check happen before the cell is filled, which `LazyCell` can't do.

---

### Task 1: Install the box's stylesheet once per process

**Files:**
- Modify: `src/taskbox.rs:30-35` (imports), `src/taskbox.rs:236-260` (`build_window`), and add a `thread_local!` plus `install_style()` just above `build_window`.
- Test: `tests/e2e-box.sh` (unchanged; run it), plus a throwaway instrumented check that is not committed.

**Interfaces:**
- Consumes: `style::css() -> String` from `src/taskbox/style.rs` (unchanged).
- Produces: `fn install_style()`, private to `taskbox.rs`, which takes no arguments and returns nothing. Nothing outside this file calls it.

- [ ] **Step 1: Add the guard and `install_style()`**

In `src/taskbox.rs`, change the `std::cell` import (line 34) from:

```rust
use std::cell::RefCell;
```

to:

```rust
use std::cell::{OnceCell, RefCell};
```

Then insert this directly above the `/// Build and show the window.` doc comment on `build_window` (around line 233):

```rust
thread_local! {
    /// The box's stylesheet, once it is on the display. The daemon opens a box
    /// for every keypress over a process that runs for days, and a provider
    /// added per box would stay on the display for good, so every style
    /// lookup, the panels' included, would walk one more each time. Only ever
    /// touched on the GTK main thread.
    static STYLE: OnceCell<CssProvider> = const { OnceCell::new() };
}

/// Put the box's stylesheet on the display, unless it is already there.
///
/// It is never taken off again: `style::css()` is built from constants, so no
/// box ever needs a different one.
fn install_style() {
    STYLE.with(|installed| {
        if installed.get().is_some() {
            return;
        }
        // No display means nothing to style yet. The cell stays empty, so the
        // next box tries again.
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let provider = CssProvider::new();
        // load_from_data, not load_from_string: the latter is gated behind gtk4's
        // v4_12 feature, and this needs no minimum beyond what the crate requires.
        provider.load_from_data(&style::css());
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let _ = installed.set(provider);
    });
}
```

The closure parameter is `installed`, not `style`, so it doesn't read like the `style` module that `style::css()` names.

- [ ] **Step 2: Call it from `build_window` in place of the inline setup**

In `build_window`, replace this block (currently lines 250-260):

```rust
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
```

with:

```rust
    install_style();
```

It stays in the same place, right after `window.add_css_class("task-box");`.

Run: `cargo build --release 2>&1 | tail -3 && cargo clippy --release 2>&1 | grep -E "^(warning|error)" | sort | uniq -c`
Expected: it builds, and clippy shows no warning in `src/taskbox.rs`. `CssProvider` and `gdk` are both still used, so neither import becomes unused.

- [ ] **Step 3: Temporarily log each install (do not commit)**

To count installs, add this as the line just before `let _ = installed.set(provider);` in `install_style`:

```rust
        eprintln!("task-box: stylesheet installed");
```

Then rebuild: `cargo build --release 2>&1 | tail -1`

- [ ] **Step 4: Open four boxes in one daemon and count the installs**

Save this as a scratch script (in the session scratchpad, not the repo) and run it with `bash`. It stops the user's `niri-tasks.service` while it runs, then starts it again only if it had been running. Don't touch the keyboard while it runs: `wtype` types into whatever has focus.

```bash
#!/usr/bin/env bash
set -u
BIN="$PWD/target/release/niritasks"
LOG="$(mktemp)"
SB="$(mktemp -d)"; mkdir -p "$SB/data"
printf 'data.location=%s/data\n' "$SB" > "$SB/taskrc"
export TASKRC="$SB/taskrc" TASKDATA="$SB/data"

WAS=$(systemctl --user is-active niri-tasks.service 2>/dev/null || echo inactive)
systemctl --user stop niri-tasks.service 2>/dev/null; sleep 1
"$BIN" daemon 2>"$LOG" & D=$!; sleep 3

box_up() {
    niri msg -j windows | python3 -c "
import json,sys
sys.exit(0 if any((w.get('app_id') or '')=='dev.niri-tasks.box' for w in json.load(sys.stdin)) else 1)"
}
for i in 1 2 3 4; do
    "$BIN" task add >/dev/null 2>&1 &
    for _ in $(seq 1 40); do box_up && break; sleep 0.1; done
    sleep 0.6; wtype -k Escape; sleep 0.8
done

kill "$D"; wait "$D" 2>/dev/null
[ "$WAS" = active ] && systemctl --user start niri-tasks.service
echo "installs: $(grep -c 'task-box: stylesheet installed' "$LOG")"
rm -rf "$SB" "$LOG"
```

Expected: `installs: 1`. (Before this change the same run would have added four providers.) If it prints `0`, no box opened: check that niri is running and that `niri msg -j windows` shows a `dev.niri-tasks.box` window while a box is up.

- [ ] **Step 5: Check the CLI path still installs it**

With no daemon running, `task add` builds its own box through `show`. Run:

```bash
WAS=$(systemctl --user is-active niri-tasks.service 2>/dev/null || echo inactive)
systemctl --user stop niri-tasks.service 2>/dev/null; sleep 1
./target/release/niritasks task add & sleep 2; wtype -k Escape; wait
[ "$WAS" = active ] && systemctl --user start niri-tasks.service
```

Expected: `task-box: stylesheet installed` is printed once, and while it was up the box looked like the daemon's box: the terminal's font, a dark translucent fill, and a white outline on the focused field.

- [ ] **Step 6: Remove the temporary log line**

Delete the `eprintln!("task-box: stylesheet installed");` line from `install_style`.

Run: `git diff src/taskbox.rs | grep -c eprintln`
Expected: `0`

- [ ] **Step 7: Run every suite against the build**

Run: `cargo build --release 2>&1 | tail -1 && NIRITASKS=./target/release/niritasks bash tests/all.sh`
Expected: the last line reads `N passed   0 failed   M skipped`, and `tests/e2e-box.sh` is among the passes if this machine has wtype and a Wayland session. Its "served by the daemon" half opens several boxes from one daemon and its "fallback" half opens them from the CLI, so between them they cover both open paths through `build_window`. Leave the keyboard alone while the box suite runs.

- [ ] **Step 8: Commit**

```bash
git add src/taskbox.rs
git commit -m "$(cat <<'EOF'
Add the task box's stylesheet to the display once per process

Every box the daemon opened left another CssProvider on the display, and
every style lookup walked all of them. A thread_local OnceCell now guards
the install, and the provider is never removed: the stylesheet is built
from constants, so no box ever needs a different one.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
