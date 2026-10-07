# Ideas Notepad Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the task panel a last tab, Ideas: one large text area per workspace tag for jotting down ideas that are not tasks yet, saved to a plain file.

**Architecture:** A new `src/ideas.rs` owns where a tag's notepad file lives and reads and writes it whole. The pure panel state (`src/panel/state.rs`) learns a `Tab` (a filter tab, or Ideas) from `src/panel/model.rs`; on Ideas it shows no cards, has no card focus, and claims only Escape and the tab keys, so every other key falls through to GTK. A new `src/panel/notepad.rs` is the GTK text area with the autosave timer. `src/panel/surface.rs` puts it in the column in place of the cards, picks the key mapping by tab, and flushes the notepad when the keyboard is given back. The daemon passes each panel its workspace tag.

**Tech Stack:** Rust 2021, gtk4 0.11 (`gtk4::TextView`, `glib::timeout_add_local_once`), gtk4-layer-shell, the bash/niri e2e harness in `tests/e2e-panel.sh`.

**Spec:** Taskwarrior task `76bd1ee5-f3e0-4ee4-8bc0-5ac1ef3efdd3` ("feat: Add an Ideas notepad tab to the task panel"). Read it with `task rc.json.array=on 76bd1ee5-f3e0-4ee4-8bc0-5ac1ef3efdd3 export`; its description and annotations are the spec.

## Global Constraints

- One notepad per workspace tag, stored as plain text at `$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md` (`~/.local/share` fallback). A relative `XDG_DATA_HOME` is ignored, as `refine::refine_mod_dir` already does.
- Autosave shortly after typing stops (this plan: 1 second) and again when the panel gives the keyboard back; no save button.
- Ideas is not a filter tab and shows no cards; it is always shown, last after Waiting, reached with `6` or `]` from Waiting (from whichever filter tab is the last one shown).
- While the text area has focus it takes every key as typing; Escape still gives the keyboard back, and Ctrl+[ / Ctrl+] switch tabs.
- The other tabs behave as before.
- Out of scope: turning an idea into a task, Markdown rendering, syncing ideas across machines.
- Not changed: with no tasks on the workspace the panel still does not take the keyboard (`Mod+Alt+Ctrl+T` opens the fuzzel list instead, as today). The spec does not ask for that to change.
- Code style: match the surrounding files — doc comments in plain sentences that say why, tests named as sentences, no new crates.
- Commits: Conventional Commits with a scope (`feat(panel): …`), subject ≤ 72 chars, body wrapped at 72, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- The e2e test must never touch the real `~/.local/share`: the nested daemon gets `XDG_DATA_HOME="$SB/share"`.

---

## File Structure

- Create `src/ideas.rs` — where a tag's notepad file lives; read it (missing = empty) and write it whole through a renamed temp file. Pure path function plus two tiny fs functions; unit-tested.
- Modify `src/lib.rs` — `pub mod ideas;`.
- Modify `src/panel/model.rs` — `Tab` enum (`Filter(Filter)` | `Ideas`), `Tab::label`, `Tab::shown`.
- Modify `src/panel/keys.rs` — `KeyAction::Ideas` on `6`; `ideas_key_action` for keys while on Ideas.
- Modify `src/panel/state.rs` — `ideas: bool` field; `tab()`, `on_ideas()`; `on_tab(Tab)`; picking, stepping, rendering, focus and keys on Ideas.
- Create `src/panel/notepad.rs` — the GTK text area, its load/flush/autosave.
- Modify `src/panel/mod.rs` — `pub mod notepad;`.
- Modify `src/panel/style.rs` — the notepad's look.
- Modify `src/panel/surface.rs` — Ideas tab button, notepad in the column, key mapping by tab, flush on release/close, `show(tag, cards)`, module docs.
- Modify `src/daemon.rs` — pass each panel its workspace tag.
- Modify `tests/e2e-panel.sh` — sandbox `XDG_DATA_HOME`; type on Ideas, autosave, save on Escape, survive a daemon restart.
- Modify `README.md`, `CONTEXT.md` — document the Ideas tab.

---

### Task 1: Ideas file store

**Files:**
- Create: `src/ideas.rs`
- Modify: `src/lib.rs` (module list, between `pub mod herdr;` and `pub mod ipc;`)

**Interfaces:**
- Consumes: `crate::tag::workspace_tag(&str) -> String` (existing).
- Produces:
  - `pub fn file(home: &Path, xdg_data_home: Option<&Path>, tag: &str) -> Option<PathBuf>`
  - `pub fn file_for(tag: &str) -> Option<PathBuf>` (reads `HOME`, `XDG_DATA_HOME`)
  - `pub fn read(path: &Path) -> std::io::Result<String>` (missing file → `Ok(String::new())`)
  - `pub fn write(path: &Path, text: &str) -> std::io::Result<()>`

- [ ] **Step 1: Write the module with its tests and stub bodies**

Create `src/ideas.rs`:

```rust
//! The Ideas tab's notepads: one plain-text file per workspace tag, for ideas
//! that are not tasks yet.
//!
//! Plain files under the user's data directory rather than Taskwarrior, which
//! has nowhere to keep text that is not a task. Read and written whole: a
//! notepad is small, and the panel saves it only once typing stops.

use std::io;
use std::path::{Path, PathBuf};

/// Where a workspace tag's ideas live: `$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md`,
/// else under `~/.local/share`. A relative `XDG_DATA_HOME` is ignored, as the
/// XDG spec says.
///
/// None for anything but a workspace tag: an empty one, which no workspace
/// with tasks has, or a string `tag::workspace_tag` would have changed. A tag
/// is only lowercase letters, digits and `_`, so it is always a plain file
/// name and never a path.
pub fn file(home: &Path, xdg_data_home: Option<&Path>, tag: &str) -> Option<PathBuf> {
    todo!()
}

/// [`file`], from this process's `HOME` and `XDG_DATA_HOME`. None without a
/// `HOME`.
pub fn file_for(tag: &str) -> Option<PathBuf> {
    todo!()
}

/// A notepad's text. A file not there yet is a notepad with nothing in it.
pub fn read(path: &Path) -> io::Result<String> {
    todo!()
}

/// Save `text` as the whole notepad, making its folder first. Written beside
/// it and renamed into place, so a daemon stopped mid-write leaves the ideas
/// as they were rather than half of the new ones.
pub fn write(path: &Path, text: &str) -> io::Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideas_live_under_xdg_data_home() {
        assert_eq!(
            file(Path::new("/home/x"), Some(Path::new("/data")), "niri_tasks"),
            Some(PathBuf::from("/data/niri-tasks/ideas/niri_tasks.md"))
        );
    }

    #[test]
    fn without_xdg_data_home_they_live_under_local_share() {
        assert_eq!(
            file(Path::new("/home/x"), None, "web"),
            Some(PathBuf::from("/home/x/.local/share/niri-tasks/ideas/web.md"))
        );
    }

    #[test]
    fn a_relative_xdg_data_home_is_ignored() {
        assert_eq!(
            file(Path::new("/home/x"), Some(Path::new("data")), "web"),
            Some(PathBuf::from("/home/x/.local/share/niri-tasks/ideas/web.md"))
        );
    }

    /// One notepad per workspace: two tags never share a file.
    #[test]
    fn each_tag_has_its_own_file() {
        let home = Path::new("/home/x");
        assert_ne!(file(home, None, "web"), file(home, None, "docs"));
    }

    #[test]
    fn only_a_workspace_tag_names_a_file() {
        for tag in ["", "../escape", "a/b", "Web", "has space", "_web"] {
            assert_eq!(file(Path::new("/home/x"), None, tag), None, "{tag:?}");
        }
    }

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("niritasks-ideas-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_missing_file_reads_as_no_ideas() {
        let dir = scratch("missing");
        assert_eq!(read(&dir.join("web.md")).unwrap(), "");
    }

    #[test]
    fn written_ideas_read_back_making_their_folder() {
        let dir = scratch("roundtrip");
        let path = dir.join("niri-tasks/ideas/web.md");
        write(&path, "one\ntwo\n").unwrap();
        assert_eq!(read(&path).unwrap(), "one\ntwo\n");
        write(&path, "shorter").unwrap();
        assert_eq!(read(&path).unwrap(), "shorter", "a save replaces the file whole");
        assert!(!path.with_extension("md.tmp").exists(), "no temporary file left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

In `src/lib.rs`, add the module between `pub mod herdr;` and `pub mod ipc;`:

```rust
pub mod herdr;
pub mod ideas;
pub mod ipc;
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --lib ideas::`
Expected: compiles (with unused-variable warnings), every `ideas::tests::*` test FAILS with `not yet implemented`.

- [ ] **Step 3: Implement the four functions**

Replace the four `todo!()` bodies:

```rust
pub fn file(home: &Path, xdg_data_home: Option<&Path>, tag: &str) -> Option<PathBuf> {
    if tag.is_empty() || crate::tag::workspace_tag(tag) != tag {
        return None;
    }
    let data = xdg_data_home
        .filter(|p| p.is_absolute())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".local/share"));
    Some(data.join("niri-tasks/ideas").join(format!("{tag}.md")))
}

pub fn file_for(tag: &str) -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    file(&home, xdg.as_deref(), tag)
}

pub fn read(path: &Path) -> io::Result<String> {
    match std::fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        other => other,
    }
}

pub fn write(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --lib ideas::`
Expected: all 7 `ideas::tests::*` tests PASS, no warnings from `src/ideas.rs`.

- [ ] **Step 5: Commit**

```bash
git add src/ideas.rs src/lib.rs
git commit -m "$(cat <<'EOF'
feat(ideas): keep a notepad file per workspace tag

The Ideas tab needs somewhere to keep text that is not a task. Each
workspace tag gets one plain file under $XDG_DATA_HOME/niri-tasks/ideas,
read whole and written whole through a renamed temporary file.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: The Ideas tab in the panel's state and keys

**Files:**
- Modify: `src/panel/model.rs` (add `Tab` after `impl Filter`, ~line 171; tests at the end of `mod tests`)
- Modify: `src/panel/keys.rs` (`KeyAction`, `key_action`, new `ideas_key_action`, tests)
- Modify: `src/panel/state.rs` (struct, getters, `visible`, `empty_text`, `shows_clear_all`, `take_keyboard`, `on_tab`, `on_key`, `on_clear_all`, `release`, `pick`, `filter_key`, `rerender`, tests)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces (used by Task 3's `surface.rs`):
  - `model::Tab` — `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Tab { Filter(Filter), Ideas }`, `Tab::label(self) -> &'static str`, `Tab::shown(cards: &[Card]) -> Vec<Tab>`
  - `keys::KeyAction::Ideas`; `keys::ideas_key_action(key: gdk::Key, ctrl: bool) -> KeyAction`
  - `PanelState::tab(&self) -> Tab`, `PanelState::on_ideas(&self) -> bool`, `PanelState::on_tab(&mut self, tab: Tab) -> Vec<Effect>` (was `on_tab(Filter)`)
  - `PanelState::tabs()` is unchanged: still the filter tabs on show, `Vec<Filter>`; Ideas is always shown after them.

- [ ] **Step 1: Write the failing model tests**

Append inside `mod tests` in `src/panel/model.rs`, before its closing `}`:

```rust
    /// Ideas is a tab, not a filter: always on show, after the filter tabs.
    #[test]
    fn ideas_is_always_shown_last() {
        let all = cards(&[task("plain", 9.0, false), waiting("parked")], &[]);
        assert_eq!(
            Tab::shown(&all),
            vec![Tab::Filter(Filter::All), Tab::Filter(Filter::ToRefine), Tab::Filter(Filter::Waiting), Tab::Ideas]
        );
        assert_eq!(Tab::shown(&[]), vec![Tab::Filter(Filter::All), Tab::Ideas]);
    }

    #[test]
    fn a_tab_reads_as_its_filter_or_ideas() {
        assert_eq!(Tab::Filter(Filter::ToRefine).label(), "To refine");
        assert_eq!(Tab::Ideas.label(), "Ideas");
    }
```

- [ ] **Step 2: Write the failing key tests**

In `src/panel/keys.rs` `mod tests`, replace the `other_numbers_pass_through` test (doc comment included) with:

```rust
    /// 6 is the Ideas tab, after the five filter tabs.
    #[test]
    fn six_picks_ideas() {
        assert_eq!(key_action(gdk::Key::_6, false), KeyAction::Ideas);
    }

    /// Past the six tabs a number is nothing, not a seventh tab.
    #[test]
    fn other_numbers_pass_through() {
        for key in [gdk::Key::_0, gdk::Key::_7, gdk::Key::_9] {
            assert_eq!(key_action(key, false), KeyAction::Ignore, "{key:?}");
        }
    }

    /// On Ideas the text area takes every key as typing: the letters,
    /// digits and brackets that press buttons and pick tabs elsewhere, Enter,
    /// Tab, the arrows, Delete, and the Ctrl chords the cards use.
    #[test]
    fn on_ideas_every_other_key_is_typing() {
        for key in [
            gdk::Key::s,
            gdk::Key::S,
            gdk::Key::_1,
            gdk::Key::_6,
            gdk::Key::bracketleft,
            gdk::Key::bracketright,
            gdk::Key::Return,
            gdk::Key::Tab,
            gdk::Key::Up,
            gdk::Key::Down,
            gdk::Key::Left,
            gdk::Key::Delete,
            gdk::Key::space,
        ] {
            assert_eq!(ideas_key_action(key, false), KeyAction::Ignore, "{key:?}");
        }
        for key in [gdk::Key::Return, gdk::Key::Delete, gdk::Key::t, gdk::Key::a] {
            assert_eq!(ideas_key_action(key, true), KeyAction::Ignore, "Ctrl+{key:?}");
        }
    }

    /// Escape still gives the keyboard back, and Ctrl+[ and Ctrl+] switch
    /// tab, as [ and ] do off Ideas.
    #[test]
    fn on_ideas_escape_gives_back_and_ctrl_brackets_switch_tab() {
        assert_eq!(ideas_key_action(gdk::Key::Escape, false), KeyAction::Release);
        assert_eq!(ideas_key_action(gdk::Key::bracketleft, true), KeyAction::PrevFilter);
        assert_eq!(ideas_key_action(gdk::Key::bracketright, true), KeyAction::NextFilter);
    }
```

- [ ] **Step 3: Write the failing state tests**

In `src/panel/state.rs` `mod tests`, replace the whole `the_brackets_skip_hidden_tabs_and_stop_at_the_ends` test with:

```rust
    #[test]
    fn the_brackets_skip_hidden_tabs_and_stop_at_the_ends() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert_eq!(key(&mut state, KeyAction::NextFilter), vec![Effect::Render]);
        assert_eq!(state.filter(), Filter::ToRefine, "past the hidden Active and Planned");
        assert_eq!(key(&mut state, KeyAction::NextFilter), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Ideas, "after the last filter tab shown");
        assert_eq!(key(&mut state, KeyAction::NextFilter), Vec::new(), "Ideas is the last");
        key(&mut state, KeyAction::PrevFilter);
        key(&mut state, KeyAction::PrevFilter);
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
        assert_eq!(key(&mut state, KeyAction::PrevFilter), Vec::new());
    }
```

Then append a new section at the end of `mod tests`, before its closing `}`:

```rust
    // ─── the Ideas tab ───────────────────────────────────────────────────

    #[test]
    fn six_opens_ideas_with_no_cards_no_focus_and_no_clear_all() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        assert_eq!(key(&mut state, KeyAction::Ideas), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Ideas);
        assert!(state.on_ideas());
        assert!(state.visible().is_empty(), "Ideas shows no cards");
        assert_eq!(state.empty_text(), None, "nor says it has none");
        assert_eq!(state.focus(), None);
        assert!(!state.shows_clear_all());
        assert_eq!(key(&mut state, KeyAction::Ideas), Vec::new(), "already there");
    }

    /// ] from Waiting, the last filter tab, reaches Ideas; [ comes back, onto
    /// the first card.
    #[test]
    fn the_bracket_past_waiting_is_ideas() {
        let mut state = keyboard(vec![card("p", Status::Pending), card("w", Status::Waiting)]);
        key(&mut state, KeyAction::Filter(Filter::Waiting));
        assert_eq!(key(&mut state, KeyAction::NextFilter), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Ideas);
        assert_eq!(key(&mut state, KeyAction::NextFilter), Vec::new(), "Ideas is the last");
        assert_eq!(key(&mut state, KeyAction::PrevFilter), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Filter(Filter::Waiting));
        assert_eq!(state.focus(), focused("w", Slot::Body).as_ref());
    }

    /// The text area takes every key as typing: on Ideas a card's keys, the
    /// arrows, Enter and the rest are not the panel's. Escape still gives the
    /// keyboard back, onto All for next time.
    #[test]
    fn on_ideas_only_escape_and_the_tab_keys_are_the_panels() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        for k in [
            KeyAction::Run(Action::Edit),
            KeyAction::Run(Action::Remove),
            KeyAction::PrevCard,
            KeyAction::NextCard,
            KeyAction::PrevSlot,
            KeyAction::NextSlot,
            KeyAction::Enter,
            KeyAction::Advance,
            KeyAction::ClearAll,
            KeyAction::Ignore,
        ] {
            assert_eq!(state.on_key(k), None, "{k:?}");
        }
        assert!(state.on_ideas());
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
        assert!(!state.on_ideas());
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
    }

    /// Leaving Ideas works for the filter tab picked before it too: All here.
    #[test]
    fn a_tab_key_or_click_leaves_ideas_for_its_tab() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::All)), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(state.on_tab(Tab::Ideas), vec![Effect::Render]);
        assert_eq!(state.on_tab(Tab::Filter(Filter::ToRefine)), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Filter(Filter::ToRefine));
    }

    #[test]
    fn a_hidden_tabs_key_on_ideas_stays_on_ideas() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(key(&mut state, KeyAction::Filter(Filter::Planned)), Vec::new());
        assert!(state.on_ideas());
    }

    /// A tick mid-typing draws again and stays on Ideas, no card focused.
    #[test]
    fn a_refresh_keeps_ideas() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(state.set_cards(&pending(&["a", "b"])), vec![Effect::Render]);
        assert!(state.on_ideas());
        assert_eq!(state.focus(), None);
    }

    #[test]
    fn the_keyboard_opens_on_all_even_from_ideas() {
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn six_disarms_clear_all_on_the_way_to_ideas() {
        let mut state = waiting_tab();
        key(&mut state, KeyAction::ClearAll);
        assert_eq!(key(&mut state, KeyAction::Ideas), vec![Effect::Render]);
        assert_eq!(state.armed(), &Armed::None);
        assert!(state.on_ideas());
    }
```

- [ ] **Step 4: Run the tests to see them fail**

Run: `cargo test --lib panel::`
Expected: FAIL to compile — `cannot find type Tab`, `no variant KeyAction::Ideas`, `cannot find function ideas_key_action`, `no method tab / on_ideas`.

- [ ] **Step 5: Add `Tab` to the model**

In `src/panel/model.rs`, directly after the closing `}` of `impl Filter` (before the `/// The task cards for one workspace tag…` doc of `pub fn cards`), add:

```rust
/// A tab on the keyboard's panel: a filter tab over the cards, or Ideas, the
/// workspace's notepad, which is not a filter and shows no cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Filter(Filter),
    Ideas,
}

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::Filter(filter) => filter.label(),
            Tab::Ideas => "Ideas",
        }
    }

    /// The tabs on show, left to right: the filter tabs [`Filter::shown`]
    /// keeps, then Ideas, always and last.
    pub fn shown(cards: &[Card]) -> Vec<Tab> {
        Filter::shown(cards).into_iter().map(Tab::Filter).chain([Tab::Ideas]).collect()
    }
}
```

- [ ] **Step 6: Add the keys**

In `src/panel/keys.rs`:

1. In `enum KeyAction`, after the `Filter(Filter),` variant and its doc, add:

```rust
    /// 6: the Ideas tab, after the filter tabs.
    Ideas,
```

2. Leave the doc on `Filter(Filter)` as it is (1 to 5 is still right). Replace the doc above `PrevFilter` (`/// [ and ]: the tab either side, stopping at the ends as the cards do.`) with:

```rust
    /// [ and ]: the tab either side, Ideas the last, stopping at the ends as
    /// the cards do. Ctrl+[ and Ctrl+] on Ideas.
```

3. In `key_action`, after `gdk::Key::_5 => KeyAction::Filter(Filter::TABS[4]),` add:

```rust
        gdk::Key::_6 => KeyAction::Ideas,
```

4. Replace the first sentences of `key_action`'s doc comment — `/// Map a keypress to what it should do. The letters work without a modifier,\n/// because nothing on the panel takes typing.` — with:

```rust
/// Map a keypress to what it should do, off the Ideas tab (on it,
/// [`ideas_key_action`]). The letters work without a modifier, because
/// nothing on the cards takes typing.
```

(leave the rest of that doc comment as it is).

5. After `key_action` (before `pub fn step`), add:

```rust
/// Map a keypress while the Ideas tab is picked, its text area having the
/// focus. Every key is typing, the letters, digits and brackets that press
/// buttons and pick tabs elsewhere included, but Escape, which gives the
/// keyboard back, and Ctrl+[ and Ctrl+], which switch tab as [ and ] do off
/// it.
pub fn ideas_key_action(key: gdk::Key, ctrl: bool) -> KeyAction {
    match (key, ctrl) {
        (gdk::Key::Escape, _) => KeyAction::Release,
        (gdk::Key::bracketleft, true) => KeyAction::PrevFilter,
        (gdk::Key::bracketright, true) => KeyAction::NextFilter,
        _ => KeyAction::Ignore,
    }
}
```

- [ ] **Step 7: Teach the state the Ideas tab**

In `src/panel/state.rs`:

1. Imports: `use super::model::{self, Card, Filter};` becomes:

```rust
use super::model::{self, Card, Filter, Tab};
```

2. In `pub struct PanelState`, after the `filter: Filter,` field add:

```rust
    /// The Ideas tab is picked: the notepad in place of the cards. Only ever
    /// with the keyboard, and off whenever the panel takes it.
    ideas: bool,
```

and change the `filter` field's doc to:

```rust
    /// The filter tab picked, or, on Ideas, the one picked before it. All
    /// whenever the panel takes the keyboard; kept across a refresh while it
    /// has it.
```

3. After `pub fn filter(&self) -> Filter { … }` add:

```rust
    /// The tab picked: a filter tab, or Ideas.
    pub fn tab(&self) -> Tab {
        if self.ideas {
            Tab::Ideas
        } else {
            Tab::Filter(self.filter)
        }
    }

    /// On the Ideas tab, whose text area takes every key but Escape and the
    /// tab keys.
    pub fn on_ideas(&self) -> bool {
        self.ideas
    }
```

4. Replace the doc and body of `tabs()`:

```rust
    /// The filter tabs on show, in `Filter::TABS` order: All, and every
    /// other tab with a task under it. None without the keyboard. Ideas,
    /// always shown, follows them.
    pub fn tabs(&self) -> Vec<Filter> {
```

(body unchanged).

5. `shows_clear_all`:

```rust
    /// Clear all ends the tab bar on the Waiting tab alone.
    pub fn shows_clear_all(&self) -> bool {
        self.keyboard && !self.ideas && self.filter == Filter::Waiting
    }
```

6. At the top of `visible()`'s body, before `let picked = …`, add:

```rust
        if self.ideas {
            return Vec::new();
        }
```

and append to its doc comment: `/// None on Ideas, which is a notepad, not cards.`

7. `empty_text()` becomes:

```rust
    /// The line shown in place of cards when the tab has none: only All, on
    /// a workspace whose tasks are all waiting. Never on Ideas, which has no
    /// cards to have none of.
    pub fn empty_text(&self) -> Option<&'static str> {
        if self.ideas {
            return None;
        }
        let filter = self.shown_filter();
        filter.pick(&self.all).is_empty().then(|| filter.empty_text())
    }
```

8. In `take_keyboard`, after `self.filter = Filter::All;` add `self.ideas = false;`. In `release`, after `self.filter = Filter::All;` add `self.ideas = false;`.

9. `on_tab` becomes:

```rust
    /// A click on a tab: a filter tab, or Ideas.
    pub fn on_tab(&mut self, tab: Tab) -> Vec<Effect> {
        self.pick(tab)
    }
```

10. In `on_key`, right after the `if !self.keyboard { return None; }` block, add:

```rust
        if self.ideas {
            // The text area's: every key is typing but these.
            return match key {
                KeyAction::Release => Some(self.release()),
                KeyAction::Filter(_) | KeyAction::Ideas | KeyAction::PrevFilter | KeyAction::NextFilter => {
                    Some(self.filter_key(key))
                }
                _ => None,
            };
        }
```

In the Clear-all-armed `match` change `KeyAction::Filter(_) | KeyAction::PrevFilter | KeyAction::NextFilter => self.filter_key(key),` to:

```rust
                KeyAction::Filter(_) | KeyAction::Ideas | KeyAction::PrevFilter | KeyAction::NextFilter => {
                    self.filter_key(key)
                }
```

and make the same change in the main `match` below it. Extend `on_key`'s doc comment with a final sentence: `/// On Ideas only Escape and the tab keys are the panel's; the rest are typing, for the text area.`

11. In `on_clear_all`, change `let mut effects = self.pick(Filter::All);` to:

```rust
        let mut effects = self.pick(Tab::Filter(Filter::All));
```

12. Replace `pick` and `filter_key` (docs included) with:

```rust
    /// Show this tab: a filter tab's cards, or Ideas. Nothing without the
    /// keyboard, for a filter tab hidden for having no tasks, or for the tab
    /// already picked.
    fn pick(&mut self, tab: Tab) -> Vec<Effect> {
        if !self.keyboard || self.tab() == tab {
            return Vec::new();
        }
        match tab {
            Tab::Filter(filter) if !Filter::shown(&self.all).contains(&filter) => return Vec::new(),
            Tab::Filter(filter) => {
                self.filter = filter;
                self.ideas = false;
            }
            Tab::Ideas => self.ideas = true,
        }
        self.rerender()
    }

    /// 1 to 5 pick a filter tab and 6 Ideas; [ and ] step along the tabs on
    /// show, Ideas the last, stopping at the ends.
    fn filter_key(&mut self, key: KeyAction) -> Vec<Effect> {
        let to = match key {
            KeyAction::Filter(filter) => Tab::Filter(filter),
            KeyAction::Ideas => Tab::Ideas,
            _ => {
                let shown = Tab::shown(&self.all);
                let at = shown.iter().position(|t| *t == self.tab()).unwrap_or(0);
                shown[keys::step(at, shown.len(), key == KeyAction::NextFilter)]
            }
        };
        self.pick(to)
    }
```

13. In `rerender`, change `self.focus = if self.keyboard {` to:

```rust
        self.focus = if self.keyboard && !self.ideas {
```

and append to its doc: `/// On Ideas no card has the focus: the text area has it.`

- [ ] **Step 8: Run the tests to see them pass**

Run: `cargo test --lib panel::`
Expected: PASS, including the new model, keys and state tests and every existing one. `cargo build` then FAILS in `src/panel/surface.rs` at `on_tab(filter)` (expected `Tab`, found `Filter`) — fix that one call now so the crate builds, in `connect_tab_bar`:

```rust
                    let effects = p.state.borrow_mut().on_tab(Tab::Filter(filter));
```

and add `Tab` to its model import: `use super::model::{Card, Filter, Status, Tab};`

Run: `cargo build && cargo test`
Expected: builds; all tests PASS.

- [ ] **Step 9: Commit**

```bash
git add src/panel/model.rs src/panel/keys.rs src/panel/state.rs src/panel/surface.rs
git commit -m "$(cat <<'EOF'
feat(panel): add the Ideas tab to the panel state and keys

Ideas is a tab but not a filter: always shown after the filter tabs,
picked with 6 or ] from the last of them, showing no cards and focusing
none. On it only Escape and the tab keys are the panel's, Ctrl+[ and
Ctrl+] standing for [ and ], so every other key can be typing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: The notepad on screen, saved per workspace

**Files:**
- Create: `src/panel/notepad.rs`
- Modify: `src/panel/mod.rs`
- Modify: `src/panel/style.rs` (`css()` card-look selector and a new rule; a test)
- Modify: `src/panel/surface.rs` (module docs; `Panel` fields; `new`; `connect_tab_bar`; `connect_keys`; `show`; `take_keyboard`; `apply`; `update_tabs`; `render`; `close`; `tab_bar`)
- Modify: `src/daemon.rs` (`tick`)
- Modify: `tests/e2e-panel.sh`
- Modify: `README.md`, `CONTEXT.md`

**Interfaces:**
- Consumes: `crate::ideas::{file_for, read, write}` (Task 1); `Tab`, `KeyAction`, `keys::ideas_key_action`, `PanelState::{tab, on_ideas, on_tab}` (Task 2).
- Produces:
  - `notepad::Notepad` with `pub root: gtk4::ScrolledWindow`, `pub fn new() -> Rc<Notepad>`, `pub fn load(&self, tag: &str)`, `pub fn flush(&self)`, `pub fn focus(&self)`
  - `Panel::show(self: &Rc<Self>, tag: &str, cards: &[Card])` (was `show(cards)`)

- [ ] **Step 1: Write the failing e2e checks**

In `tests/e2e-panel.sh`:

1. Directly after the line `nested_start` (before `TAG=$(…)`), add:

```bash
# The Ideas tab saves under XDG_DATA_HOME: the sandbox's, never the real
# ~/.local/share. Set before the daemon starts, so it has it.
NENV+=(XDG_DATA_HOME="$SB/share")
```

2. Directly after the `TAG` check block (after its closing `fi`), add:

```bash
IDEAS="$SB/share/niri-tasks/ideas/$TAG.md"
ideas() { cat "$IDEAS" 2>/dev/null; }
```

3. Inside the `if command -v wtype >/dev/null; then` block, after the past-the-cap section's final

```bash
    "${NENV[@]}" wtype -k Escape
    settle
```

and before the `else` that leads to `skip "the panel's cards in the middle of the screen…`, add:

```bash

    # ─── the Ideas tab ───────────────────────────────────────────────────────
    # 6 opens Ideas: no cards, one text area in the middle, taking every key
    # as typing, the s, 1 and ] that press a button or pick a tab elsewhere
    # included. It saves a second after typing stops, and again at once when
    # Escape gives the keyboard back.
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype 6
    sleep 1
    shot ideas_tab || { summary; exit 1; }
    read -r x0 x1 y0 y1 < <(measure ideas_tab)
    if [ "$x1" -gt 0 ] && [ "$x0" -ge "$SURFACE_LEFT" ] && [ "$x1" -le "$SURFACE_RIGHT" ]; then
        ok "6 opens the Ideas tab in the middle (columns ${x0}-${x1}, rows ${y0}-${y1})"
    else
        bad "on the Ideas tab the panel covers columns ${x0}-${x1}, rows ${y0}-${y1}, expected
      within ${SURFACE_LEFT}-${SURFACE_RIGHT}"
    fi
    "${NENV[@]}" wtype 'first idea'
    "${NENV[@]}" wtype -k Return
    "${NENV[@]}" wtype 'second: s, 1 and ] are typing'
    sleep 2.5
    want=$'first idea\nsecond: s, 1 and ] are typing'
    if [ "$(ideas)" = "$want" ]; then
        ok "what is typed on Ideas is saved once typing stops, the panel still up"
    else
        bad "a second after typing stopped, $IDEAS holds '$(ideas)', expected '$want'"
    fi
    "${NENV[@]}" wtype -k Return
    "${NENV[@]}" wtype 'third'
    "${NENV[@]}" wtype -k Escape
    settle
    want=$'first idea\nsecond: s, 1 and ] are typing\nthird'
    if [ "$(ideas)" = "$want" ]; then
        ok "Escape saves what was typed since, as it gives the keyboard back"
    else
        bad "after Escape, $IDEAS holds '$(ideas)', expected '$want'"
    fi
    shot ideas_released || { summary; exit 1; }
    read -r x0 x1 _ _ < <(measure ideas_released)
    if [ "$x0" -eq "$PEEK_X" ] && [ "$x1" -eq "$OUT_W" ]; then
        ok "Escape from Ideas tucks the panel back to its peek (columns ${x0}-${x1})"
    else
        bad "after Escape from Ideas the panel drew columns ${x0}-${x1}, expected ${PEEK_X}-${OUT_W}"
    fi
```

4. Change that block's skip message from `skip "the panel's cards in the middle of the screen, their buttons, the filter tabs, Clear all, and Escape back (needs wtype)"` to:

```bash
    skip "the panel's cards in the middle of the screen, their buttons, the filter tabs, Clear all, the Ideas tab, and Escape back (needs wtype)"
```

5. After the cold-start check (its closing `fi`) and before `# ─── the real daemon, untouched ───`, add:

```bash

# ─── the Ideas tab survives a restart ────────────────────────────────────────
# The daemon above is a new one: Ideas opens on what the last one saved, the
# cursor at its end, so a line typed now lands after it.
if command -v wtype >/dev/null; then
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype 6
    sleep 1
    "${NENV[@]}" wtype -k Return
    "${NENV[@]}" wtype 'after the restart'
    "${NENV[@]}" wtype -k Escape
    settle
    want=$'first idea\nsecond: s, 1 and ] are typing\nthird\nafter the restart'
    if [ "$(ideas)" = "$want" ]; then
        ok "a restarted daemon's Ideas tab opens on the saved ideas and adds to them"
    else
        bad "after a restart, $IDEAS holds '$(ideas)', expected '$want' — the
      first three lines missing means the new daemon did not load them"
    fi
else
    skip "the Ideas tab after a daemon restart (needs wtype)"
fi
```

- [ ] **Step 2: Run the e2e test to see the Ideas checks fail**

Run: `bash tests/e2e-panel.sh 2>&1 | tail -30`
Expected: the existing checks PASS; the new ones FAIL — `6` picks Ideas in the state but nothing draws a text area, so typing reaches no notepad and `$IDEAS` stays empty ("… holds '', expected …"). (If `wtype` is missing they SKIP; install it with `sudo apt install wtype` and rerun.)

- [ ] **Step 3: Write the notepad widget**

Create `src/panel/notepad.rs`:

```rust
//! The Ideas tab's text area: the workspace's notepad, typed into while the
//! task panel has the keyboard, and saved to `ideas.rs`'s file for its tag a
//! second after typing stops and again when the keyboard is given back.
//!
//! One per panel, kept across renders: the column is torn down on every
//! render, but the buffer, its cursor and what is waiting to be saved are
//! this, not the column's.

use super::style::{CARD_WIDTH_PX, PADDING_PX};
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// How long typing has to stop before the notepad saves.
const SAVE_AFTER: Duration = Duration::from_secs(1);

/// The text area's height: a large page, scrolling inside itself past that,
/// which keeps the cursor in view however long the ideas run.
const HEIGHT_PX: i32 = 480;

pub struct Notepad {
    /// What the panel puts in its column on the Ideas tab: the text view in
    /// a scroller of its own.
    pub root: gtk4::ScrolledWindow,
    view: gtk4::TextView,
    /// The tag whose file the buffer holds. None before the first load, for
    /// a workspace with no tag, and after a file that would not read.
    tag: RefCell<Option<String>>,
    /// Typed into since the last save.
    dirty: Cell<bool>,
    /// The save waiting for typing to stop.
    pending: RefCell<Option<glib::SourceId>>,
    /// The buffer is being filled from the file, which is not typing.
    loading: Cell<bool>,
}

impl Notepad {
    pub fn new() -> Rc<Notepad> {
        let view = gtk4::TextView::new();
        // WordChar, as the cards wrap: a long path or URL still breaks.
        view.set_wrap_mode(gtk4::WrapMode::WordChar);
        view.set_top_margin(PADDING_PX);
        view.set_bottom_margin(PADDING_PX);
        view.set_left_margin(PADDING_PX);
        view.set_right_margin(PADDING_PX);
        let root = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&view)
            .build();
        root.add_css_class("ideas");
        root.set_size_request(CARD_WIDTH_PX, HEIGHT_PX);
        // Clips the text to the rounded corners, as a card's action row is.
        root.set_overflow(gtk4::Overflow::Hidden);
        let notepad = Rc::new(Notepad {
            root,
            view,
            tag: RefCell::new(None),
            dirty: Cell::new(false),
            pending: RefCell::new(None),
            loading: Cell::new(false),
        });
        let weak = Rc::downgrade(&notepad);
        notepad.view.buffer().connect_changed(move |_| {
            if let Some(n) = weak.upgrade() {
                n.typed();
            }
        });
        notepad
    }

    /// Fill the buffer from `tag`'s file, with the cursor at the end to carry
    /// on from. What was typed for the tag before is saved to it first, so a
    /// workspace's ideas never land in another's file.
    ///
    /// A file that will not read leaves the notepad empty and read-only:
    /// saving it would write over ideas that are still there.
    pub fn load(&self, tag: &str) {
        self.flush();
        let (tag, text) = match crate::ideas::file_for(tag).map(|path| crate::ideas::read(&path)) {
            Some(Ok(text)) => (Some(tag.to_string()), text),
            Some(Err(e)) => {
                crate::notify::tasks(&format!("Could not read your ideas: {e}"));
                (None, String::new())
            }
            None => (None, String::new()),
        };
        self.view.set_editable(tag.is_some());
        *self.tag.borrow_mut() = tag;
        let buffer = self.view.buffer();
        self.loading.set(true);
        buffer.set_text(&text);
        buffer.place_cursor(&buffer.end_iter());
        self.loading.set(false);
        self.dirty.set(false);
    }

    /// Save now what was typed since the last save, if anything, to the tag
    /// it was typed for; the save waiting for typing to stop is not needed
    /// after this. A save that fails says so and stays owed.
    pub fn flush(&self) {
        if let Some(id) = self.pending.borrow_mut().take() {
            id.remove();
        }
        if !self.dirty.get() {
            return;
        }
        let Some(path) = self.tag.borrow().as_deref().and_then(crate::ideas::file_for) else { return };
        let buffer = self.view.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        match crate::ideas::write(&path, &text) {
            Ok(()) => self.dirty.set(false),
            Err(e) => crate::notify::tasks(&format!("Could not save your ideas: {e}")),
        }
    }

    /// Give the text area the keyboard's focus.
    pub fn focus(&self) {
        self.view.grab_focus();
    }

    /// The buffer changed: unless it was a load, owe a save, and put it off
    /// until typing has stopped for [`SAVE_AFTER`].
    fn typed(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        self.dirty.set(true);
        if let Some(id) = self.pending.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(SAVE_AFTER, move || {
            if let Some(n) = weak.upgrade() {
                // Fired, so gone: removing it again would be an error.
                n.pending.borrow_mut().take();
                n.flush();
            }
        });
        *self.pending.borrow_mut() = Some(id);
    }
}
```

In `src/panel/mod.rs`, add `pub mod notepad;` between `pub mod model;` and `pub mod state;`.

- [ ] **Step 4: Give the notepad a card's look**

In `src/panel/style.rs` `css()`:

1. Change the opening card-look selector group from

```
.task-panel .task-card,
.task-panel .filter-tabs,
.task-panel .filter-empty {{
```

to

```
.task-panel .task-card,
.task-panel .filter-tabs,
.task-panel .filter-empty,
.task-panel .ideas {{
```

2. After the `.task-panel .filter-empty {{ padding: … }}` rule (and its comment), add:

```
/* The Ideas tab's text area: the card's tint shows through it, and its text
   and caret are the cards' colour. */
.task-panel .ideas textview,
.task-panel .ideas textview text {{ background-color: transparent; color: {TEXT}; caret-color: {TEXT}; }}
```

3. In `mod tests`, add:

```rust
    /// The notepad is drawn as a card is, its text view see-through over it.
    #[test]
    fn the_ideas_notepad_wears_a_cards_look() {
        let css = css();
        assert!(css.contains(".task-panel .filter-empty,\n.task-panel .ideas {"));
        assert!(css.contains(".task-panel .ideas textview text { background-color: transparent;"));
    }
```

Run: `cargo test --lib panel::style`
Expected: PASS (including `every_rule_is_scoped_to_the_panel`).

- [ ] **Step 5: Wire the notepad into the panel**

In `src/panel/surface.rs`:

1. Imports: add `use super::notepad::Notepad;` after `use super::keys;`.

2. `Panel` struct: after the `tab_buttons` field add

```rust
    /// The Ideas tab, after the filter tabs and always shown with them.
    ideas_button: gtk4::Button,
```

and after the `blur` field add

```rust
    /// The Ideas tab's text area, shown in the column in place of the cards.
    notepad: Rc<Notepad>,
    /// The workspace tag this panel shows, whose notepad Ideas opens.
    tag: RefCell<String>,
```

3. `tab_bar()` returns the Ideas button too. Change its signature and doc, and add the button after the filter buttons, before Clear all:

```rust
/// The filter tabs, Ideas and Clear all, over the scroller rather than in it,
/// so they stay put while the cards scroll. Ring room on three sides, as the
/// column keeps, and under them the card gap less the ring the column keeps
/// above the first card: the first card then sits a card gap below.
fn tab_bar() -> (gtk4::Box, Vec<gtk4::Button>, gtk4::Button, gtk4::Button) {
```

After the `let tab_buttons: Vec<gtk4::Button> = … .collect();` statement add:

```rust
    // Ideas, last of the tabs: not a filter, and always shown.
    let ideas = gtk4::Button::with_label(Tab::Ideas.label());
    ideas.set_focusable(false);
    ideas.set_focus_on_click(false);
    tabs.append(&ideas);
```

and change its last line to `(tabs, tab_buttons, ideas, clear)`.

4. In `Panel::new`, change `let (tabs, tab_buttons, clear) = tab_bar();` to `let (tabs, tab_buttons, ideas_button, clear) = tab_bar();`, and in the `Rc::new(Panel { … })` literal add `ideas_button,` after `tab_buttons,` and, after `blur: RefCell::new(None),`:

```rust
            notepad: Notepad::new(),
            tag: RefCell::new(String::new()),
```

5. `connect_tab_bar`: change its doc to `/// A click on a tab picks it; a click on Clear all presses it.` and, after the filter-button loop, add:

```rust
        let weak = Rc::downgrade(self);
        self.ideas_button.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                let effects = p.state.borrow_mut().on_tab(Tab::Ideas);
                p.apply(effects);
            }
        });
```

6. `connect_keys`: replace

```rust
            let action = keys::key_action(key, modifiers.contains(gdk::ModifierType::CONTROL_MASK));
```

with

```rust
            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
            // On Ideas the text area takes every key as typing, bar Escape
            // and the Ctrl+[ and Ctrl+] that switch tab.
            let action = if p.state.borrow().on_ideas() {
                keys::ideas_key_action(key, ctrl)
            } else {
                keys::key_action(key, ctrl)
            };
```

7. Replace `show`:

```rust
    /// Show the cards of the workspace this tag is for, capped, or hide the
    /// panel when there are none. A new tag moves the notepad: what was typed
    /// is saved to the tag it was typed for, and, with the keyboard still
    /// held, the new tag's ideas load at once; without it they load when the
    /// panel next takes it.
    pub fn show(self: &Rc<Self>, tag: &str, cards: &[Card]) {
        if self.tag.borrow().as_str() != tag {
            *self.tag.borrow_mut() = tag.to_string();
            if self.state.borrow().keyboard() {
                self.notepad.load(tag);
            } else {
                self.notepad.flush();
            }
        }
        let effects = self.state.borrow_mut().set_cards(cards);
        self.apply(effects);
    }
```

8. In `take_keyboard`, after `self.cancel_grace();` add:

```rust
        // Afresh every time: another panel may have saved this tag's ideas
        // since, its workspace having moved monitor.
        self.notepad.load(&self.tag.borrow());
```

9. In `apply`, at the start of the `Effect::Release => {` arm (before the existing comment), add:

```rust
                    // What was typed on Ideas, saved as the keyboard goes.
                    self.notepad.flush();
```

10. Replace `update_tabs`:

```rust
    /// Which filter tabs show, which tab is picked, and whether Clear all
    /// shows. Ideas is always shown: the bar itself hides without the
    /// keyboard.
    fn update_tabs(&self) {
        let state = self.state.borrow();
        let shown = state.tabs();
        for (button, filter) in self.tab_buttons.iter().zip(Filter::TABS) {
            button.set_visible(shown.contains(&filter));
            set_class(button, "current", state.tab() == Tab::Filter(filter));
        }
        set_class(&self.ideas_button, "current", state.on_ideas());
        self.clear.set_visible(state.shows_clear_all());
    }
```

11. In `render`, replace the `let (shown, keyboard, empty, focus) = { … };` statement, the `self.while_drawing(|| { … });` call and the final `if keyboard { self.focus_on(focus.as_ref()); }` with:

```rust
        let (shown, keyboard, empty, focus, ideas) = {
            let state = self.state.borrow();
            (state.visible(), state.keyboard(), state.empty_text(), state.focus().cloned(), state.on_ideas())
        };

        self.while_drawing(|| {
            // Mid-typing, a tick's render leaves the notepad where it is:
            // taking it out of the column would take its focus with it.
            let column: &gtk4::Widget = self.column.upcast_ref();
            if ideas && self.notepad.root.parent().as_ref() == Some(column) {
                return;
            }
            while let Some(child) = self.column.first_child() {
                self.column.remove(&child);
            }
            let cards: Vec<CardWidgets> = shown.iter().map(|s| self.card_widget(s, keyboard)).collect();
            for card in &cards {
                self.column.append(&card.root);
            }
            *self.cards.borrow_mut() = cards;
            if ideas {
                self.column.append(&self.notepad.root);
            }
            if let Some(text) = empty {
                // Only All, with every task waiting: it says so, and the
                // Waiting tab beside it has them.
                self.column.append(&empty_line(text));
            }
        });
```

and, at the end of `render` (where `if keyboard { self.focus_on(focus.as_ref()); }` was):

```rust
        if ideas {
            self.notepad.focus();
        } else if keyboard {
            self.focus_on(focus.as_ref());
        }
```

(The lines between — `self.tabs.set_visible(keyboard);`, `self.update_tabs();`, `self.fit();`, `self.window.present();`, `self.set_region(…)`, `self.update_blur(…)` — stay as they are.)

12. `close`: add `self.notepad.flush();` as its first line.

13. Module docs: after the `//! On the Waiting tab alone, Clear all ends the tab bar. …` paragraph (ending `//! with the focus where it was, and a card's keys do nothing.`), add:

```rust
//!
//! After the filter tabs, Ideas is always shown: not a filter but the
//! workspace's notepad, `notepad.rs`'s text area in the column in place of the
//! cards, one per workspace tag. 6 picks it, and ] from the last filter tab.
//! While it is picked the text area takes every key but Escape, Ctrl+[ and
//! Ctrl+] (`keys::ideas_key_action`), and a tick's render leaves it in place
//! so typing keeps its focus. What was typed is saved a second after typing
//! stops and again as the keyboard is given back, to the tag it was typed for.
```

- [ ] **Step 6: Pass each panel its tag**

In `src/daemon.rs` `tick`, replace the loop body:

```rust
    for (connector, panel) in panels.borrow().iter() {
        let name = outputs.get(connector).cloned().flatten();
        let cards = by_name
            .entry(name.clone())
            .or_insert_with(|| cards_for_workspace(name.as_deref()));
        panel.show(cards);
    }
```

with:

```rust
    for (connector, panel) in panels.borrow().iter() {
        let name = outputs.get(connector).cloned().flatten();
        // The panel's tag, for the notepad its Ideas tab opens.
        let t = tag::workspace_tag(name.as_deref().unwrap_or_default());
        let cards = by_name
            .entry(name.clone())
            .or_insert_with(|| cards_for_workspace(name.as_deref()));
        panel.show(&t, cards);
    }
```

- [ ] **Step 7: Build and run the unit tests**

Run: `cargo build && cargo test`
Expected: builds with no warnings; all tests PASS.

- [ ] **Step 8: Run the e2e test to see it pass**

Run: `bash tests/e2e-panel.sh 2>&1 | tail -40`
Expected: every check PASSES, the new ones included:
- `6 opens the Ideas tab in the middle …`
- `what is typed on Ideas is saved once typing stops, the panel still up`
- `Escape saves what was typed since, as it gives the keyboard back`
- `Escape from Ideas tucks the panel back to its peek …`
- `a restarted daemon's Ideas tab opens on the saved ideas and adds to them`
and the run ends with the real daemon untouched. Confirm the real `~/.local/share/niri-tasks/ideas/` gained no `e2e.md`: `ls ~/.local/share/niri-tasks/ideas/e2e.md` → `No such file or directory`.

- [ ] **Step 9: Document the Ideas tab**

`CONTEXT.md`: in the **Filter tab** entry, after the sentence `The Waiting tab alone ends in Clear all, which deletes every task under it on a second press, as Remove does one.` add ` The Ideas tab after them is not one.`; then, directly after the Filter tab entry's `_Avoid_:` line and its blank line, add:

```markdown
**Ideas tab**:
The last tab above the task cards while the task panel has the keyboard,
after Waiting and always shown: in place of the cards, one large text area,
the workspace's notepad for ideas that are not tasks yet, kept per workspace
tag in `$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md`. Not a filter tab: it shows
no cards, and while it is picked every key is typing but Escape, which gives
the keyboard back, and Ctrl+[ and Ctrl+], which switch tab. 6 picks it, as ]
does from the last filter tab. Saved a second after typing stops, and again
when the keyboard is given back.
_Avoid_: notes (a note row is a task's), scratchpad, memo
```

`README.md`:

1. In the `Mod+Alt+Ctrl+T` table row, replace `and the Waiting tab's Clear all (Ctrl+Delete) deletes every waiting task.` with `and the Waiting tab's Clear all (Ctrl+Delete) deletes every waiting task. The last tab, Ideas (6), is a notepad for the workspace's ideas that are not tasks yet.`

2. After the paragraph ending `the card it was on, and the panel stays up; switching tab puts it back too.` add a blank line and:

```markdown
The last tab, **Ideas**, is not a filter: in place of the cards it shows one
large text area, a notepad for this workspace's ideas that are not tasks yet.
It is always there, after Waiting; `6` picks it, as `]` does from the last tab
before it. While it is picked every key is typing, the letters, digits and
brackets included: `Escape` still gives the keyboard back, and `Ctrl+[` and
`Ctrl+]` switch tab. Each workspace tag has its own notepad, plain text in
`$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md` (under `~/.local/share` without
it), saved a second after typing stops and again when the panel gives the
keyboard back.
```

- [ ] **Step 10: Commit**

```bash
git add src/panel/notepad.rs src/panel/mod.rs src/panel/style.rs src/panel/surface.rs src/daemon.rs tests/e2e-panel.sh README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
feat(panel): type ideas into a notepad on the Ideas tab

The Ideas tab shows a large text area in place of the cards: the
workspace's notepad, loaded from its tag's file each time the panel takes
the keyboard, saved a second after typing stops and again when the
keyboard is given back. The daemon now hands each panel its tag, so a
workspace switch saves what was typed to the tag it was typed for.

The e2e run sandboxes XDG_DATA_HOME and checks typing, both saves and a
daemon restart.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
