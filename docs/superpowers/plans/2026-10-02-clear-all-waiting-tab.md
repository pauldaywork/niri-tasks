# Clear All on the Waiting Tab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On the keyboard panel's Waiting tab, a Clear all button (or Shift+Delete) deletes every waiting task on the workspace in one go, after a confirming second press, the way Remove deletes one.

**Architecture:** The pure parts sit beside the code they belong to: which uuids the Waiting tab lists (`Filter::uuids` in `src/panel/model.rs`), the button's words and the one shell command that runs Remove's own `task status <uuid> deleted --yes` for each uuid in turn (`src/panel/actions.rs`), and Shift+Delete as a new `KeyAction::ClearAll` (`src/panel/keys.rs`). `src/panel/surface.rs` adds the button at the far end of the tab bar, visible only on the Waiting tab and out of the focus chain like the tabs, with its own arming that disarms on any focus move and any re-render. The second press switches the panel to All and has niri spawn the delete command.

**Tech Stack:** Rust 2021, gtk4-rs 0.11 (GTK 4.22), gtk4-layer-shell 0.8, Taskwarrior 2.6.2, bash + Pillow + wtype for the e2e test in a nested niri.

**Spec:** Taskwarrior task `b1de2776-2cde-454f-b910-0e8160501927`. Read it with `task rc.json.array=on b1de2776-2cde-454f-b910-0e8160501927 export`. Its description and notes are the spec.

## Global Constraints

- Clear all **deletes** the tasks, the same as Remove on each one. It doesn't bring them back to the list or complete them.
- It lives only in the panel, on the Waiting tab, and shows only while that tab is picked. There's no new CLI subcommand and no fuzzel menu entry.
- It covers only the panel's workspace tag, meaning exactly the tasks the Waiting tab lists (`task::waiting_for_tag`).
- It confirms the way Remove does. The first press arms it as **"Confirm clear all"**, the second deletes, and moving away disarms it.
- **Shift+Delete** presses it from the keyboard, because the filter tabs are outside the focus chain. **Delete alone still removes the focused card.**
- Delete each waiting task through the same `task status … deleted --yes` path. Keep the keyboard afterwards, and fall back to All once the Waiting tab is empty.
- Add unit tests and a `tests/e2e-panel.sh` check. Document it in README's button table and key list and in CONTEXT.md.
- Done when: on a workspace with several waiting tasks, two presses of Clear all (or Shift+Delete twice) delete every one of them and no other task. The panel then returns to All with the Waiting tab hidden, and the tests and docs cover it.
- Out of scope: clearing waiting tasks across workspaces, a CLI or menu version, and undo.
- House style: every item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commit messages are one plain-English imperative sentence, like `git log` shows, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line.

## Decisions made while planning (flag any you disagree with)

1. **One process deletes them all, one after another.** The second press has niri spawn `sh -c 'for uuid; do "$0" task status $uuid deleted --yes; done' <niritasks> <uuid>…`. Each task goes through `niritasks task status … deleted --yes`, exactly Remove's path, notification and all (so N waiting tasks give N "Deleted: …" notifications, as N Removes would). Sequential rather than N parallel spawns because Taskwarrior 2.6 rewrites `pending.data` whole, and racing writers can lose a change. The script is built from `Action::Remove.args("$uuid")`, so it cannot drift from Remove. niritasks's path is the shell's `$0` and the uuids are its arguments, so neither is quoted into the script. `$uuid` goes in unquoted, which is safe because a uuid is hex and dashes. `;`, not `&&`: one failed delete doesn't stop the rest.
2. **The uuids are `Filter::Waiting.uuids(&all)`**: every card the Waiting tab lists, uncapped, including those folded into "+N more". The panel's `all` holds exactly what `daemon.rs` read from `task::waiting_for_tag` for this panel's tag.
3. **The button sits at the far end of the tab bar** (hexpand, halign End), in Remove's red, filled red once armed like the armed Remove. It reads "<trash icon>  Clear all", then "<trash icon>  Confirm clear all", with tooltip "Shift+Delete". It is not focusable and doesn't take focus on click, the same as the tabs.
4. **Disarming:** any change of the window's focus widget (Up/Down/Left/Right/Tab, a re-render's refocus) and any `render()` (a tab switch, a refresh that changed the cards) put it back. Arming itself doesn't move focus: a click doesn't take focus, and Shift+Delete moves nothing.
5. **The second press switches to All at once** (`pick(Filter::All)`) and doesn't wait for the deletes to land. Focus goes to All's first card through the existing render path, and the keyboard stays. Once the deletes land, the refresh hides the Waiting tab (`Filter::shown`). If every task on the workspace was waiting, All says "No tasks" until that refresh, and then `render()`'s existing no-tasks path hides the panel and gives the keyboard back. With no tasks left there is nothing to keep it for.
6. **Shift+Delete off the Waiting tab does nothing.** It doesn't fall through to Remove, because Remove takes plain Delete. `clear_all()` checks for the Waiting tab and the keyboard itself, so the key, `emit_clicked` and a click all take the one guarded path.
7. **`keys::key_action` gains a `shift: bool` argument.** Shift only changes Delete/KP_Delete. Shift+Tab already arrives as `ISO_Left_Tab` and Shift+letter as a capital, both of which keep their meaning.
8. **The nested niri now runs with the notify-send stub on its PATH.** A panel button's `niritasks` is spawned by niri, so it runs under the nested niri's environment, not the daemon's. Without this, the e2e's deletes would pop "Deleted: …" on the real desktop. The stub is written before the nested niri starts, and the nested niri gets `NOTIFY_LOG` and `PATH` as `NENV` does. The e2e then counts the "Tasks Deleted: " lines to show each delete went through `task status`.

## File map

| File | Change |
|---|---|
| `src/panel/model.rs` | `Filter::uuids`. |
| `src/panel/actions.rs` | `CLEAR_ALL`, `CONFIRM_CLEAR_ALL`, `clear_all_command`. |
| `src/panel/keys.rs` | `KeyAction::ClearAll`; `key_action(key, shift)`. |
| `src/panel/surface.rs` | Key call site passes Shift; `clear` button and `clear_armed`; `clear_all`, `disarm_clear`, `clear_label`, `delete_all`, `spawn_on`; module doc. |
| `src/panel/style.rs` | `.clear-all` and `.clear-all.confirm` CSS; module doc line. |
| `tests/lib/nested-niri.sh` | Stub before the nested niri starts; nested niri gets `NOTIFY_LOG` and `PATH`. |
| `tests/e2e-panel.sh` | Clear all section. |
| `README.md` | Keybind row, button table, tabs paragraph, hand checks, e2e description. |
| `CONTEXT.md` | Filter tab entry mentions Clear all. |

---

### Task 1: Clear all as data: its uuids, its command and its key

**Files:**
- Modify: `src/panel/model.rs` (impl `Filter`, after `shown`; tests module)
- Modify: `src/panel/actions.rs` (after `impl Action`; tests module)
- Modify: `src/panel/keys.rs` (`KeyAction`, `key_action`, tests module)
- Modify: `src/panel/surface.rs:378` (the key controller's call) and `src/panel/surface.rs:865-869` (the no-op arm in `key()`)

**Interfaces:**
- Consumes: `Filter::pick`, `Card.uuid`, `Action::Remove.args`, `Action::Remove.icon` (all exist).
- Produces (Task 2 uses these exact names):
  - `pub fn Filter::uuids(self, cards: &[Card]) -> Vec<String>`
  - `pub const actions::CLEAR_ALL: &str = "Clear all"`
  - `pub const actions::CONFIRM_CLEAR_ALL: &str = "Confirm clear all"`
  - `pub fn actions::clear_all_command(exe: &str, uuids: &[String]) -> Vec<String>`
  - `KeyAction::ClearAll`
  - `pub fn keys::key_action(key: gdk::Key, shift: bool) -> KeyAction`

- [ ] **Step 1: Write the failing tests**

In `src/panel/model.rs`, append inside `mod tests` (after `each_tab_picks_its_tasks_in_order`; the helpers `task`, `waiting` and `cards` already exist there):

```rust
    /// Clear all deletes what the Waiting tab lists, and nothing it does not.
    #[test]
    fn a_tabs_uuids_are_its_tasks_and_no_others() {
        let all = cards(
            &[task("plain", 9.0, false), waiting("parked"), task("started", 5.0, true), waiting("also-parked")],
            &[],
        );
        assert_eq!(Filter::Waiting.uuids(&all), vec!["parked", "also-parked"]);
        assert!(Filter::Waiting.uuids(&cards(&[task("plain", 9.0, false)], &[])).is_empty());
    }

    /// Every one the tab lists, not just those on screen before "+N more".
    #[test]
    fn a_tabs_uuids_are_uncapped() {
        let many: Vec<Task> = (0..CAP + 2).map(|i| waiting(&format!("w{i}"))).collect();
        assert_eq!(Filter::Waiting.uuids(&cards(&many, &[])).len(), CAP + 2);
    }
```

In `src/panel/actions.rs`, append inside `mod tests`:

```rust
    #[test]
    fn clear_all_reads_as_remove_does() {
        assert_eq!(CLEAR_ALL, "Clear all");
        assert_eq!(CONFIRM_CLEAR_ALL, "Confirm clear all");
    }

    /// Run for real, with `echo` standing in for niritasks: each uuid gets
    /// Remove's own command, in order.
    #[test]
    fn clear_all_runs_remove_on_each_task_in_turn() {
        let uuids = vec!["aaaa-1".to_string(), "bbbb-2".to_string()];
        let command = clear_all_command("echo", &uuids);
        assert_eq!(&command[..2], ["sh", "-c"]);
        let out = std::process::Command::new(&command[0]).args(&command[1..]).output().unwrap();
        assert!(out.status.success());
        let expected: String = uuids.iter().map(|u| Action::Remove.args(u).join(" ") + "\n").collect();
        assert_eq!(String::from_utf8(out.stdout).unwrap(), expected);
    }

    /// niritasks's path and the uuids are the shell's arguments, never spliced
    /// into its script, so a path with a space still works.
    #[test]
    fn clear_all_passes_niritasks_and_the_uuids_as_arguments() {
        let command = clear_all_command("/opt/my tools/niritasks", &["u1".into()]);
        assert_eq!(command[3], "/opt/my tools/niritasks");
        assert_eq!(command[4], "u1");
        assert!(!command[2].contains("my tools"));
    }
```

In `src/panel/keys.rs`, first give every existing test call the new argument. Run this, which touches only the tests module (it starts at `mod tests {`, line 74):

```bash
sed -i '/^mod tests {/,$ s/key_action(\([^()]*\))/key_action(\1, false)/g' src/panel/keys.rs
```

Then append inside `mod tests`:

```rust
    /// Shift+Delete is Clear all; Delete alone still removes the focused card.
    #[test]
    fn shift_delete_clears_all_and_delete_alone_removes() {
        assert_eq!(key_action(gdk::Key::Delete, true), KeyAction::ClearAll);
        assert_eq!(key_action(gdk::Key::KP_Delete, true), KeyAction::ClearAll);
        assert_eq!(key_action(gdk::Key::Delete, false), KeyAction::Run(Action::Remove));
        assert_eq!(key_action(gdk::Key::KP_Delete, false), KeyAction::Run(Action::Remove));
    }

    /// Shift changes nothing else: a capital and Shift+Tab already carry it.
    #[test]
    fn shift_leaves_every_other_key_alone() {
        for key in [gdk::Key::S, gdk::Key::Escape, gdk::Key::ISO_Left_Tab, gdk::Key::Down, gdk::Key::bracketright] {
            assert_eq!(key_action(key, true), key_action(key, false), "{key:?}");
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib panel 2>&1 | tail -20`
Expected: compile errors: no method `uuids`, cannot find `CLEAR_ALL` / `CONFIRM_CLEAR_ALL` / `clear_all_command`, no variant `ClearAll`, and `key_action` takes 1 argument but 2 were supplied.

- [ ] **Step 3: Implement**

In `src/panel/model.rs`, inside `impl Filter`, after `shown`:

```rust
    /// The tasks under this tab, by uuid and uncapped: what Clear all on the
    /// Waiting tab deletes, which is every task the tab lists, those folded
    /// into "+N more" too.
    pub fn uuids(self, cards: &[Card]) -> Vec<String> {
        self.pick(cards).into_iter().filter_map(|c| c.uuid).collect()
    }
```

In `src/panel/actions.rs`, between the closing `}` of `impl Action` and `#[cfg(test)]`:

```rust
/// What the Waiting tab's Clear all reads, and what it reads between its first
/// press and its second, as Remove asks before it deletes.
pub const CLEAR_ALL: &str = "Clear all";
pub const CONFIRM_CLEAR_ALL: &str = "Confirm clear all";

/// The command Clear all spawns: Remove's own `task status <uuid> deleted
/// --yes` for each of `uuids`, one after another in one shell. So every task
/// goes the way Remove takes it, notification and all, and no two
/// taskwarrior writes race over `pending.data`. `exe` is niritasks itself,
/// passed as the shell's `$0`, and the uuids as its arguments, so neither is
/// ever quoted into the script; `$uuid` goes in bare, a uuid being hex and
/// dashes. `;`, not `&&`: one task that fails to delete does not keep the
/// rest.
pub fn clear_all_command(exe: &str, uuids: &[String]) -> Vec<String> {
    let remove = Action::Remove.args("$uuid").join(" ");
    let mut command = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("for uuid; do \"$0\" {remove}; done"),
        exe.to_string(),
    ];
    command.extend(uuids.iter().cloned());
    command
}
```

In `src/panel/keys.rs`, add this variant to `KeyAction` after `NextFilter`:

```rust
    /// Shift+Delete: press the Waiting tab's Clear all, which sits with the
    /// tabs outside the focus chain, so no focused button can stand for it.
    ClearAll,
```

Replace `key_action`'s doc and signature, and its Delete arm:

```rust
/// Map a keypress to what it should do. The letters work without a modifier,
/// because nothing on the panel takes typing. With Caps Lock on the keyval
/// arrives as a capital (`S`, not `s`), so the key is lowercased first and
/// capitals press the same buttons. `shift` matters to Delete alone, making it
/// Clear all: Shift+Tab already arrives as ISO_Left_Tab, and a Shift+letter as
/// its capital.
pub fn key_action(key: gdk::Key, shift: bool) -> KeyAction {
```

```rust
        gdk::Key::Delete | gdk::Key::KP_Delete if shift => KeyAction::ClearAll,
        gdk::Key::Delete | gdk::Key::KP_Delete => KeyAction::Run(Action::Remove),
```

In `src/panel/surface.rs`, the key controller (line 378) passes Shift:

```rust
                match keys::key_action(key, state.contains(gdk::ModifierType::SHIFT_MASK)) {
```

and the last arm of the second `match action` in `key()` (lines 865-869) gains `ClearAll`, so it still compiles. Task 2 handles it before that match:

```rust
            KeyAction::Release
            | KeyAction::Ignore
            | KeyAction::Filter(_)
            | KeyAction::PrevFilter
            | KeyAction::NextFilter
            | KeyAction::ClearAll => {}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: every `test result: ok`, no FAILED.

- [ ] **Step 5: Commit**

```bash
git add src/panel/model.rs src/panel/actions.rs src/panel/keys.rs src/panel/surface.rs
git commit -m "Give the panel Clear all's uuids, its delete command and Shift+Delete

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The Clear all button on the Waiting tab

**Files:**
- Modify: `src/panel/style.rs` (module doc, `css()`, tests module)
- Modify: `src/panel/surface.rs` (module doc, imports, `Panel` struct, `new`, `update_tabs`, `render`, `key`, new methods and free functions, tests module)

**Interfaces:**
- Consumes: from Task 1, `Filter::uuids`, `actions::CLEAR_ALL`, `actions::CONFIRM_CLEAR_ALL`, `actions::clear_all_command(exe, uuids)`, `KeyAction::ClearAll`. Also the existing `Panel::pick(filter)`, `render`, `update_tabs`, `open_menu`, and style's `REMOVE` and `ON_FILL`.
- Produces: `Panel.clear: gtk4::Button` (CSS class `clear-all`, plus `confirm` while armed), `Panel::clear_all`, `Panel::disarm_clear`, free fns `clear_label(armed: bool) -> String`, `delete_all(output: &str, uuids: &[String])`, `spawn_on(output: &str, build: impl FnOnce(&str) -> Vec<String>)`. `open_menu`'s signature is unchanged (`src/daemon.rs:207` calls it).

- [ ] **Step 1: Write the failing tests**

In `src/panel/style.rs`, append inside `mod tests`:

```rust
    /// Clear all wears Remove's red, and is filled with it once armed, as the
    /// armed Remove is.
    #[test]
    fn clear_all_is_removes_red() {
        let css = css();
        assert!(css.contains(&format!(".task-panel .filter-tabs .clear-all {{ color: {REMOVE}; }}")));
        assert!(css.contains(&format!(
            ".task-panel .filter-tabs .clear-all.confirm {{ background-color: {REMOVE}; color: {ON_FILL}; }}"
        )));
    }
```

In `src/panel/surface.rs`, append inside `mod tests`:

```rust
    /// Remove's trash can and its words, then, armed, what it asks.
    #[test]
    fn clear_all_wears_removes_trash_can_and_asks_once_armed() {
        assert_eq!(clear_label(false), format!("{}  Clear all", Action::Remove.icon()));
        assert_eq!(clear_label(true), format!("{}  Confirm clear all", Action::Remove.icon()));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib panel 2>&1 | tail -20`
Expected: `clear_all_is_removes_red` FAILS (assertion), and `clear_label` is not found (compile error).

- [ ] **Step 3: Style**

In `src/panel/style.rs`'s `css()`, right after the line
`.task-panel .filter-tabs button.current {{ color: {TEXT}; background-color: {CURRENT_TAB}; }}`
insert:

```
/* Clear all, at the Waiting tab bar's far end: Remove's red, and filled red
   once armed, as the armed Remove is. Its classes outrank the tabs' dimmed
   colour and the press reset's clear fill. */
.task-panel .filter-tabs .clear-all {{ color: {REMOVE}; }}
.task-panel .filter-tabs .clear-all.confirm {{ background-color: {REMOVE}; color: {ON_FILL}; }}
```

In the module doc, change the paragraph

```
//! While the panel has the keyboard, the filter tabs above the cards are one
//! bar in a card's look, and a tab with nothing in it shows one line in a
//! card's look too.
```

to

```
//! While the panel has the keyboard, the filter tabs above the cards are one
//! bar in a card's look, and a tab with nothing in it shows one line in a
//! card's look too. On the Waiting tab, Clear all ends the bar in Remove's
//! red.
```

- [ ] **Step 4: Surface: imports, field, construction**

Change the import `use super::actions::Action;` to:

```rust
use super::actions::{self, Action};
```

In `struct Panel`, after `tab_buttons`:

```rust
    /// Clear all, at the tab bar's far end, shown only on the Waiting tab:
    /// deletes every task the tab lists, on its second press.
    clear: gtk4::Button,
```

and after `armed`:

```rust
    /// Clear all pressed once and showing Confirm clear all. Moving the
    /// focus, or any re-render, puts it back, as for an armed Remove.
    clear_armed: Cell<bool>,
```

In `Panel::new`, right after the `tab_buttons` `.collect();` statement:

```rust
        // Clear all, at the bar's far end: hexpand takes the room the tabs
        // leave, and End keeps the button its own width at the end of it.
        // Out of the focus chain like the tabs, which is why Shift+Delete
        // presses it.
        let clear = gtk4::Button::with_label(&clear_label(false));
        clear.add_css_class("clear-all");
        clear.set_tooltip_text(Some("Shift+Delete"));
        clear.set_hexpand(true);
        clear.set_halign(gtk4::Align::End);
        clear.set_focusable(false);
        clear.set_focus_on_click(false);
        clear.set_visible(false);
        tabs.append(&clear);
```

In the `Rc::new(Panel { … })` literal, add `clear,` after `tab_buttons,` and `clear_armed: Cell::new(false),` after `armed: RefCell::new(None),`.

After the `for (button, filter) in panel.tab_buttons…` loop that connects the tabs' clicks:

```rust
        {
            let weak = Rc::downgrade(&panel);
            panel.clear.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.clear_all();
                }
            });
        }
```

In the `focus-widget` notify handler, change the comment and body to:

```rust
        // Moving off an armed Remove disarms it, as moving at all disarms
        // Clear all, and the action row moves to the card the focus is on,
        // before follow_focus scrolls that card, row and all, into view.
        {
            let weak = Rc::downgrade(&panel);
            panel.window.connect_notify_local(Some("focus-widget"), move |_, _| {
                if let Some(p) = weak.upgrade() {
                    p.disarm_unless_focused();
                    p.disarm_clear();
                    p.show_focused_row();
                    p.follow_focus();
                }
            });
        }
```

- [ ] **Step 5: Surface: show, disarm, press**

In `update_tabs`, after the `for` loop:

```rust
        self.clear.set_visible(self.filter.get() == Filter::Waiting);
```

and change its doc to `/// Which tabs show, which one is picked, and whether Clear all shows.`

In `render`, replace

```rust
        // The buttons go with the widgets they were armed on.
        self.armed.replace(None);
```

with

```rust
        // The buttons go with the widgets they were armed on. Clear all
        // stays, but a re-render puts it back too, a tab switch included.
        self.armed.replace(None);
        self.disarm_clear();
```

In `key`, add an arm to the first `match action` after `KeyAction::Filter(filter) => return self.pick(filter),`:

```rust
            // Through the button, so the key does exactly what a click does.
            KeyAction::ClearAll => return self.clear.emit_clicked(),
```

and change `key`'s doc's last sentence to: `1 to 5, [ and ] pick a filter tab instead, and Shift+Delete presses Clear all, whatever has focus.`

Add these methods to `impl Panel`, after `disarm_unless_focused`:

```rust
    /// Press Clear all. The first press arms it, as Remove's does; the second
    /// deletes every task the Waiting tab lists, those past "+N more" too, and
    /// puts the panel on All at once rather than as each delete lands. The
    /// keyboard stays: the deletes open nothing. Nothing off the Waiting tab,
    /// where Clear all is hidden, so a stray Shift+Delete does nothing there.
    fn clear_all(self: &Rc<Self>) {
        if !self.keyboard.get() || self.filter.get() != Filter::Waiting {
            return;
        }
        if !self.clear_armed.replace(true) {
            self.clear.set_label(&clear_label(true));
            self.clear.add_css_class("confirm");
            return;
        }
        let uuids = Filter::Waiting.uuids(&self.all.borrow());
        // Its render disarms the button on the way.
        self.pick(Filter::All);
        if !uuids.is_empty() {
            delete_all(&self.output, &uuids);
        }
    }

    /// Put Clear all back from Confirm clear all, so a later press starts
    /// over at the first.
    fn disarm_clear(&self) {
        if self.clear_armed.replace(false) {
            self.clear.set_label(&clear_label(false));
            self.clear.remove_css_class("confirm");
        }
    }
```

- [ ] **Step 6: Surface: spawning**

Add after `empty_line`:

```rust
/// Clear all's face: Remove's trash can and its words, or, armed, what it asks.
fn clear_label(armed: bool) -> String {
    let words = if armed { actions::CONFIRM_CLEAR_ALL } else { actions::CLEAR_ALL };
    format!("{}  {words}", Action::Remove.icon())
}
```

Replace `open_menu` (doc and body) with these three functions:

```rust
/// Run a `niritasks` command on this monitor: a task's action menu, the
/// fuzzel list, or one of a card's buttons. See `spawn_on`.
pub fn open_menu(output: &str, args: &[String]) {
    spawn_on(output, |exe| {
        let mut command = vec![exe.to_string()];
        command.extend_from_slice(args);
        command
    });
}

/// Clear all's deletes, one after another in one process: see
/// `actions::clear_all_command`.
fn delete_all(output: &str, uuids: &[String]) {
    spawn_on(output, |exe| actions::clear_all_command(exe, uuids));
}

/// Have niri spawn the command `build` makes from this program's own path.
///
/// This monitor is focused first. The menu then files under the workspace the
/// panel shows, rather than whichever monitor had focus, and fuzzel opens on
/// the screen that was clicked. It runs as its own process, spawned by niri:
/// fuzzel blocks until it closes, and the daemon must not.
fn spawn_on(output: &str, build: impl FnOnce(&str) -> Vec<String>) {
    let result = std::env::current_exe()
        .map_err(anyhow::Error::from)
        .and_then(|exe| {
            crate::niri::focus_monitor(output)?;
            crate::niri::spawn(build(&exe.to_string_lossy()))
        });
    if let Err(e) = result {
        crate::notify::tasks(&e.to_string());
    }
}
```

- [ ] **Step 7: Surface: module doc**

In the module doc, after the paragraph that ends `…and focus falls back to the first card when the`
`//! one it was on has left it.`, insert a new paragraph:

```
//!
//! On the Waiting tab alone, Clear all ends the tab bar. Like Remove, its
//! first press arms it as Confirm clear all, and its second deletes: every
//! task the tab lists, each through Remove's own `task status <uuid> deleted
//! --yes`, one after another. The panel goes to All at once and keeps the
//! keyboard. Moving the focus, or any re-render, a tab switch included,
//! disarms it. It is out of the focus chain with the tabs, so Shift+Delete
//! presses it; Delete alone is still the focused card's Remove.
```

- [ ] **Step 8: Run the tests and lints**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked"; cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | sort | uniq -c`
Expected: every `test result: ok`, no FAILED. clippy shows no warning or error pointing at a line this task changed (check against `git diff`; leave unrelated warnings alone).

- [ ] **Step 9: Build and try it by hand**

Run: `cargo build --release 2>&1 | tail -2`
Expected: `Finished`. The real check is the e2e in Task 3. Don't restart the user's own daemon to try it.

- [ ] **Step 10: Commit**

```bash
git add src/panel/style.rs src/panel/surface.rs
git commit -m "Add Clear all to the panel's Waiting tab, deleting every waiting task on its second press

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The e2e check, and the docs

**Files:**
- Modify: `tests/lib/nested-niri.sh` (the notify-send stub block and the nested niri's launch, around lines 196-229)
- Modify: `tests/e2e-panel.sh` (header comment; new section after the `parked` check, before `else` / `skip`)
- Modify: `README.md` (lines 19, 56, 67-77, 317-319, 380-384)
- Modify: `CONTEXT.md` (the **Filter tab** entry)

**Interfaces:**
- Consumes: the built binary from Task 2. The e2e drives Clear all only with `niritasks task panel`, `wtype 1`/`5` and `wtype -M shift -k Delete -m shift`. The stub logs each notification as `$*`, so a delete reads `Tasks Deleted: <description>` (`notify::tasks` → `notify-send Tasks "Deleted: …"`).
- Produces: nothing code depends on.

- [ ] **Step 1: Give the nested niri the notify-send stub**

In `tests/lib/nested-niri.sh`, cut this whole block (it currently sits after the `die "the nested niri gave its programs no NIRI_SOCKET or WAYLAND_DISPLAY"` line):

```bash
    # Notifications: every task added through the box or the CLI runs
    # notify-send (src/notify.rs) over the session bus, which is shared with
    # the real desktop and would pop up there. A stub first on PATH records
    # them in $SB/notifications instead.
    mkdir -p "$SB/bin"
    cat > "$SB/bin/notify-send" <<'STUB'
#!/bin/sh
echo "$*" >> "${NOTIFY_LOG:?}"
exit 0
STUB
    chmod +x "$SB/bin/notify-send"
```

and paste it, with its comment extended as below, immediately before the line `    # Vblank waits are off for the nested niri alone: parked out of sight, it`:

```bash
    # Notifications: every task added through the box or the CLI runs
    # notify-send (src/notify.rs) over the session bus, which is shared with
    # the real desktop and would pop up there. A stub first on PATH records
    # them in $SB/notifications instead. The nested niri gets it too: what a
    # panel button runs, niri spawns, under niri's environment and not the
    # daemon's.
    mkdir -p "$SB/bin"
    cat > "$SB/bin/notify-send" <<'STUB'
#!/bin/sh
echo "$*" >> "${NOTIFY_LOG:?}"
exit 0
STUB
    chmod +x "$SB/bin/notify-send"
```

Then change the nested niri's launch from

```bash
    env -u NIRI_SOCKET XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$PARENT_WAYLAND" \
        vblank_mode=0 __GL_SYNC_TO_VBLANK=0 niri -c "$SB/niri.kdl" >"$SB/niri.log" 2>&1 &
```

to

```bash
    env -u NIRI_SOCKET XDG_RUNTIME_DIR="$RT" WAYLAND_DISPLAY="$PARENT_WAYLAND" \
        NOTIFY_LOG="$SB/notifications" PATH="$SB/bin:$PATH" \
        vblank_mode=0 __GL_SYNC_TO_VBLANK=0 niri -c "$SB/niri.kdl" >"$SB/niri.log" 2>&1 &
```

- [ ] **Step 2: Write the e2e section**

In `tests/e2e-panel.sh`, insert this right after the `parked` check's closing `fi` and before the `else` whose body is `skip "the panel's cards in the middle of the screen, …"`:

```bash

    # ─── Clear all on the Waiting tab ────────────────────────────────────────
    # Park a second task, so the Waiting tab has two and one task stays on
    # All. Clear all shows on the Waiting tab alone, and Shift+Delete presses
    # it, the tabs being outside the focus chain.
    count() {  # filter…
        task rc.verbose=nothing "$@" count 2>/dev/null
    }
    task rc.verbose=nothing rc.confirmation=no "+$TAG" "description.is:write the glossary" \
        modify wait:someday </dev/null >/dev/null 2>&1
    settle
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype 5
    sleep 1
    shot clear_before || { summary; exit 1; }

    # The first press only arms it, as Remove's does: Confirm clear all, in
    # the tab bar alone, and nothing deleted.
    "${NENV[@]}" wtype -M shift -k Delete -m shift
    sleep 1
    shot clear_armed || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure clear_armed clear_before)
    if [ "$y1" -gt 0 ] && [ "$((y1 - y0))" -lt 60 ] && [ "$(count "+$TAG" status:waiting)" = 2 ]; then
        ok "the first Shift+Delete arms Clear all in the tab bar and deletes nothing (rows ${y0}-${y1})"
    else
        bad "the first Shift+Delete changed rows ${y0}-${y1} with $(count "+$TAG" status:waiting) task(s)
      still waiting, expected the tab bar alone and 2 — 0-0 means Clear all is not there"
    fi

    # Moving away disarms it: off the tab and back is the frame from before.
    "${NENV[@]}" wtype 1
    "${NENV[@]}" wtype 5
    sleep 1
    shot clear_disarmed || { summary; exit 1; }
    if same clear_before clear_disarmed; then
        ok "switching tab puts Clear all back"
    else
        bad "Clear all is still armed after switching tab and back"
    fi

    # Twice: both waiting tasks deleted, one after the other, each through
    # task status; the task still on All and the other tag's are left alone.
    "${NENV[@]}" wtype -M shift -k Delete -m shift
    sleep 0.5
    "${NENV[@]}" wtype -M shift -k Delete -m shift
    for _ in $(seq 1 20); do
        [ "$(count "+$TAG" status:waiting)" = 0 ] && break
        sleep 0.5
    done
    settle
    deleted=$(count "+$TAG" status:deleted)
    left=$(count "+$TAG" status:pending)
    elsewhere=$(count +niritasks_e2e_elsewhere status:pending)
    if [ "$deleted" = 2 ] && [ "$left" = 1 ] && [ "$elsewhere" = 1 ]; then
        ok "the second press deletes both waiting tasks and no other"
    else
        bad "after Clear all: ${deleted} deleted, ${left} pending here, ${elsewhere} on the
      other tag — expected 2, 1 and 1"
    fi
    notified=$(grep -c '^Tasks Deleted: ' "$SB/notifications" 2>/dev/null)
    if [ "$notified" = 2 ]; then
        ok "each went through task status deleted, notification and all"
    else
        bad "${notified:-0} 'Deleted' notifications for two tasks cleared — Clear all did not
      run task status on each"
    fi

    # The panel keeps the keyboard, back on All with the one task left, and
    # the Waiting tab is gone, so 5 does nothing.
    shot clear_after || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure clear_after)
    if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ] &&
        [ "$((y1 - y0))" -lt "$keyboard_h" ]; then
        ok "the panel keeps the keyboard in the middle, on All with one card (rows ${y0}-${y1})"
    else
        bad "after Clear all the panel covers columns ${x0}-${x1}, rows ${y0}-${y1} —
      reaching ${OUT_W} means it gave up the keyboard; 0-0 means it is gone"
    fi
    "${NENV[@]}" wtype 5
    sleep 1
    shot clear_no_tab || { summary; exit 1; }
    if same clear_after clear_no_tab; then
        ok "and the Waiting tab is gone: 5 does nothing"
    else
        bad "5 changed the panel after Clear all; the Waiting tab should be hidden"
    fi
    "${NENV[@]}" wtype -k Escape
    settle
```

Also in the header comment, change

```
# middle of the screen, and with wtype, Down (which moves the action row), the
# filter tabs' keys and Escape are pressed in the nested niri, never on your
# desktop.
```

to

```
# middle of the screen, and with wtype, Down (which moves the action row), the
# filter tabs' keys, Shift+Delete for the Waiting tab's Clear all, and Escape
# are pressed in the nested niri, never on your desktop.
```

and the `skip` message to `"the panel's cards in the middle of the screen, their buttons, the filter tabs, Clear all, and Escape back (needs wtype)"`.

- [ ] **Step 3: Run the e2e**

Run: `bash tests/e2e-panel.sh 2>&1 | tail -40`
Expected: every new line `ok`, no `bad`, summary with 0 failures. Keep off the workspace the nested niri is parked on while it runs. If a frame check fails, rerun with `NIRITASKS_E2E_KEEP=1` and look at the PNGs it names. Also run `bash tests/e2e-box.sh 2>&1 | tail -5` to show the moved stub broke nothing there.

- [ ] **Step 4: README**

Line 19 (the `Mod+Alt+Ctrl+T` keybind row): replace
`narrow them to All, Active, Planned, To refine or Waiting. With no tasks`
with
`narrow them to All, Active, Planned, To refine or Waiting, and the Waiting tab's Clear all (Shift+Delete) deletes every waiting task. With no tasks`

Button table: after the `| **Remove** (red) | …` row, add

```
| **Clear all** (red) | `Shift+Delete` | On the Waiting tab's bar, not a card: turns into **Confirm clear all**; a second press deletes every waiting task on the workspace and goes back to All, moving away puts it back |
```

Tabs paragraph: replace
`The panel opens on All every time, and goes back to All when the`
`tab it is on runs out of tasks.`
with

```
The panel opens on All every time, and goes back to All when the
tab it is on runs out of tasks. The Waiting tab alone ends in **Clear all**,
which `Shift+Delete` presses (`Delete` alone is still the focused card's
Remove). Like Remove it asks first, as **Confirm clear all**, and its second
press deletes every waiting task on this workspace, one `task status <uuid>
deleted --yes` after another, putting the panel back on All with the keyboard
kept. Switching tab or moving the focus puts it back.
```

Hand checks (around line 319): replace
`and Back to list on a waiting task brings it back to All; Escape tucks it away to the one-line peek.`
with
`and Back to list on a waiting task brings it back to All; a click on Clear all arms it, a second click deletes the waiting tasks, and Up or Down in between puts it back; Escape tucks it away to the one-line peek.`

e2e description (around line 383): replace
`All, a waiting task on the Waiting tab and off the tucked panel), and Escape`
with
`All, a waiting task on the Waiting tab and off the tucked panel, Clear all arming on its first Shift+Delete, disarming on a tab switch, and deleting both waiting tasks and nothing else on its second), and Escape`

- [ ] **Step 5: CONTEXT.md**

In the **Filter tab** entry, replace
`task that is not a planned task, and a waiting task is under Waiting alone.`
with

```
task that is not a planned task, and a waiting task is under Waiting alone.
The Waiting tab alone ends in Clear all, which deletes every task under it on
a second press, as Remove does one.
```

- [ ] **Step 6: Run the whole suite**

Run: `cargo test 2>&1 | grep -E "test result|FAILED"`
Expected: every `test result: ok`. (The README and llms.txt tests check the CLI's commands, which this doesn't change.)

- [ ] **Step 7: Commit**

```bash
git add tests/lib/nested-niri.sh tests/e2e-panel.sh README.md CONTEXT.md
git commit -m "Check Clear all end to end and document it in README and CONTEXT.md

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
