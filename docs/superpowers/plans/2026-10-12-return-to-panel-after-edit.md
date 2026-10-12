# Return to the Task Panel after Edit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Closing a task box opened by the task panel's Edit hands the keyboard back to that panel, on the same tab, with the focus on the edited card.

**Architecture:** `PanelState` remembers where Edit left it (a `Return`: the edited card's focus, the filter tab, "+N more" opened or not) as it gives the keyboard back, and forgets it whenever the keyboard is taken or given back again. The task box's destroy handler, which every way out ends in, calls a new `on_close` callback that `open_in` takes; the daemon's callback finds the panel whose state `returns_to` that uuid and calls `take_keyboard_back`, which takes the keyboard as `take_keyboard` does, then puts the tab and the focus back through `rerender`'s existing rules: All if the tab emptied, the tab's first card if the edited one left it.

**Tech Stack:** Rust (`cargo test`, `cargo build`), GTK4 panel (layer shell), niri IPC, the bash e2e suite in a nested niri (`tests/e2e-panel.sh`, needs wtype and Pillow).

**Spec:** Taskwarrior task `da6fa61d-80e7-4974-837e-dbd4967bfe15` (read it with `task rc.json.array=on da6fa61d-80e7-4974-837e-dbd4967bfe15 export`); its description and notes are the spec.

## Global Constraints

- **Every way out returns:** Save, Cancel, Esc and niri's close-window all end in the window's destroy, so the hook is the destroy handler in `src/taskbox/open.rs`, never close-request.
- **Only a box the panel's Edit opened returns.** A box from the CLI (`niritasks task edit <uuid>`, `task note`) or a keybind (Mod+Alt+T) leaves the keyboard alone. Taking the keyboard (Mod+Alt+Ctrl+T, Mod+Alt+W) or giving it back on that panel meanwhile drops the return.
- **Where it comes back:** the tab Edit was pressed on, the focus on the edited card's Edit button; the tab's first card if the edited card is gone from it; All if the tab itself has nothing under it; no keyboard taken at all with no cards.
- **Out of scope:** Note and Add boxes, and returning from other actions that give up the keyboard (Start working, Refine, the Go to list's rows).
- **Done when:** Edit from the panel then Save or Esc puts the keyboard back on the panel with that card and tab focused, and a box opened from outside the panel still closes with the panel tucked away.
- **Commits:** Conventional Commits, subject at most 72 characters (aim ~50), imperative, lowercase first word, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. The task's type is `feat`, scope `panel`.
- **Tests stay green at every commit**, and `cargo build 2>&1 | grep -c warning` prints `0` before each commit.
- **Comments match the code around them:** this repo writes doc comments in plain prose about behaviour (what the user sees), British spelling, no "we". Read a few neighbouring doc comments before writing one.

## File Structure

- `src/panel/state.rs` (modify): `Return` struct; `PanelState.returning`; `on_press` records it for Edit; `take_keyboard`, `open_projects` and `release` drop it; new `returns_to` and `take_keyboard_back`; unit tests beside the existing Edit press test.
- `src/panel/surface.rs` (modify): `Panel::returns_to`, `Panel::take_keyboard_back`.
- `src/taskbox/open.rs` (modify): `open_in` takes `on_close`, called from the destroy handler.
- `src/daemon.rs` (modify): `live_agents` factored out of `Request::Panel`; `return_keyboard(uuid)` passed as Edit/Note boxes' `on_close`; the Add box passes a no-op.
- `tests/e2e-panel.sh` (modify): one section pressing Edit, Escaping the box, and checking the panel came back exactly as it was; and one checking a CLI box leaves it tucked.
- `README.md`, `CONTEXT.md` (modify): the Edit row and the task box say so.

---

### Task 1: The panel state remembers where Edit left it and goes back there

**Files:**
- Modify: `src/panel/state.rs` — the `PanelState` struct (~line 137-176), `take_keyboard` (~line 400), `open_projects` (~line 419), `on_press` (~line 560-604), `release` (~line 787), plus a new `Return` struct after `Armed` (~line 58)
- Test: `src/panel/state.rs`, the `tests` module, directly after `a_letter_presses_only_a_button_the_card_has` (~line 1495)

**Interfaces:**
- Consumes: `Focus`, `Slot`, `Filter`, `Effect`, `PanelState::rerender` (private; keeps the focus on the same card and slot, falls back to the card's body, then the first card, and to All when the tab is gone), `PanelState::take_keyboard(&mut self, agents: Vec<String>) -> bool`.
- Produces:
  - `pub fn returns_to(&self, uuid: &str) -> bool` — true while the panel does not hold the keyboard and its Edit on `uuid` gave it up.
  - `pub fn take_keyboard_back(&mut self, uuid: &str, agents: Vec<String>) -> Vec<Effect>` — empty when nothing was done; otherwise the render effects to run (Task 2 hands them to `Panel::centre`).

- [ ] **Step 1: Write the failing tests**

In `src/panel/state.rs`'s `tests` module, directly after `a_letter_presses_only_a_button_the_card_has`, add:

```rust
    // ─── back from Edit's task box ───────────────────────────────────────

    /// An active card, which is off To refine, and two to refine: Edit on
    /// the second of those, pressed on the To refine tab.
    fn edited_on_to_refine() -> PanelState {
        let mut state = keyboard(vec![card("x", Status::Active), card("b", Status::Pending), card("c", Status::Pending)]);
        key(&mut state, KeyAction::Filter(Filter::ToRefine));
        assert_eq!(uuids(&state), vec!["b", "c"]);
        assert_eq!(
            state.on_press("c", Slot::Button(Action::Edit)),
            vec![Effect::Render, Effect::Release, Effect::Spawn(Action::Edit.args("c"))],
        );
        state
    }

    /// Edit gives the keyboard back for the box, and the box on that task
    /// closing takes it again: on the tab Edit was pressed on, the focus on
    /// the edited card's Edit. Once only.
    #[test]
    fn the_box_edit_opened_closing_takes_the_keyboard_back_where_it_was() {
        let mut state = edited_on_to_refine();
        assert!(!state.keyboard());
        assert!(state.returns_to("c"));
        assert!(!state.returns_to("b"), "another task's box");
        assert_eq!(state.take_keyboard_back("c", Vec::new()), vec![Effect::Render]);
        assert!(state.keyboard());
        assert_eq!(state.tab(), Tab::Filter(Filter::ToRefine));
        assert_eq!(state.focus(), focused("c", Slot::Button(Action::Edit)).as_ref());
        assert!(!state.returns_to("c"), "the return is spent");
        key(&mut state, KeyAction::Release);
        assert_eq!(state.take_keyboard_back("c", Vec::new()), Vec::new(), "a second close of that task's box");
        assert!(!state.keyboard());
    }

    /// A box Edit did not open leaves the keyboard alone: one on another
    /// task, and one on a panel whose Edit was never pressed.
    #[test]
    fn a_box_edit_did_not_open_leaves_the_keyboard_alone() {
        let mut state = edited_on_to_refine();
        assert_eq!(state.take_keyboard_back("b", Vec::new()), Vec::new());
        assert!(!state.keyboard());
        assert!(state.returns_to("c"), "another task's box spends nothing");

        let mut untouched = PanelState::default();
        untouched.set_cards(&pending(&["a"]));
        assert!(!untouched.returns_to("a"));
        assert_eq!(untouched.take_keyboard_back("a", Vec::new()), Vec::new());
        assert!(!untouched.keyboard());
    }

    /// Taking the keyboard, by Mod+Alt+Ctrl+T or Mod+Alt+W, or giving it
    /// back, while the box is open drops the return.
    #[test]
    fn taking_or_giving_back_the_keyboard_meanwhile_drops_the_return() {
        let mut state = edited_on_to_refine();
        assert!(state.take_keyboard(Vec::new()));
        assert!(!state.returns_to("c"));
        key(&mut state, KeyAction::Release);
        assert_eq!(state.take_keyboard_back("c", Vec::new()), Vec::new());
        assert!(!state.keyboard());

        let mut state = edited_on_to_refine();
        state.open_projects(projects(&["x"], &[]));
        key(&mut state, KeyAction::Release);
        assert_eq!(state.take_keyboard_back("c", Vec::new()), Vec::new());
        assert!(!state.keyboard());
    }

    /// The edited card gone from the tab: the tab's first card. The tab
    /// gone too: All's first card.
    #[test]
    fn back_from_edit_falls_back_to_the_tabs_first_card() {
        let mut state = edited_on_to_refine();
        state.set_cards(&[card("x", Status::Active), card("b", Status::Pending)]);
        assert_eq!(state.take_keyboard_back("c", Vec::new()), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Filter(Filter::ToRefine));
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());

        let mut state = edited_on_to_refine();
        state.set_cards(&[card("x", Status::Active)]);
        assert_eq!(state.take_keyboard_back("c", Vec::new()), vec![Effect::Render]);
        assert_eq!(state.tab(), Tab::Filter(Filter::All));
        assert_eq!(state.focus(), focused("x", Slot::Body).as_ref());
    }

    /// The edited card past "+N more" comes back with the list still
    /// opened, so the focus can land on it.
    #[test]
    fn back_from_edit_keeps_the_list_opened_past_more() {
        let names: Vec<String> = (0..model::CAP + 2).map(|i| format!("t{i}")).collect();
        let last = names.last().unwrap().clone();
        let mut state = keyboard(pending(&names.iter().map(String::as_str).collect::<Vec<_>>()));
        state.on_more();
        state.on_press(&last, Slot::Button(Action::Edit));
        assert_eq!(state.take_keyboard_back(&last, Vec::new()), vec![Effect::Render]);
        assert_eq!(state.focus(), focused(&last, Slot::Button(Action::Edit)).as_ref());
    }

    /// With no card left there is nothing to take the keyboard for.
    #[test]
    fn back_from_edit_takes_nothing_with_no_cards() {
        let mut state = edited_on_to_refine();
        state.set_cards(&[]);
        assert_eq!(state.take_keyboard_back("c", Vec::new()), Vec::new());
        assert!(!state.keyboard());
    }
```

Before running, check two assumptions these tests make and adjust the test (not the code) if either is wrong: `Status::Active` cards are not under `Filter::ToRefine` and `Status::Pending` ones are (`Filter::matches` in `src/panel/model.rs`), and `model::CAP` is public (it is used as `model::CAP` in `visible`).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib panel::state::tests::back_from_edit 2>&1 | tail -20`
Expected: FAIL to compile, "no method named `returns_to` found" / "no method named `take_keyboard_back` found".

- [ ] **Step 3: Add the `Return` struct**

In `src/panel/state.rs`, directly after the `Armed` enum, add:

```rust
/// Where Edit left the panel as it gave the keyboard back for the task box:
/// the edited card's Edit, the tab it was pressed on and whether "+N more"
/// was opened. The box on that task closing takes the keyboard back to it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Return {
    focus: Focus,
    filter: Filter,
    expanded: bool,
}
```

- [ ] **Step 4: Add the field**

In `pub struct PanelState`, directly after `armed: Armed,`, add:

```rust
    /// Where Edit left the panel, until the box it opened closes. Dropped
    /// whenever the keyboard is taken or given back meanwhile, so a box
    /// closing later than that, or one opened from anywhere else, leaves the
    /// keyboard alone.
    returning: Option<Return>,
```

(`PanelState` derives `Default`, and `Option` defaults to `None`, so nothing else needs it.)

- [ ] **Step 5: Record it on Edit**

In `on_press`, replace:

```rust
        } else if !action.keeps_keyboard() {
            effects = self.release();
        }
```

with:

```rust
        } else if !action.keeps_keyboard() {
            // Read before the release, which puts the panel back on All.
            let returning = (action == Action::Edit && self.keyboard).then(|| Return {
                focus: Focus { uuid: uuid.into(), slot },
                filter: self.filter,
                expanded: self.expanded,
            });
            effects = self.release();
            self.returning = returning;
        }
```

Also extend `on_press`'s doc comment: after the sentence "Everything that opens something gives the keyboard back first, so the box or terminal it opens can take it." add " Edit remembers where it was, for the box closing to come back to."

- [ ] **Step 6: Drop it whenever the keyboard is taken or given back**

In `take_keyboard`, directly after `self.armed = Armed::None;`, add `self.returning = None;`.
In `open_projects`, directly after `self.armed = Armed::None;`, add `self.returning = None;`.
In `release`, directly after `self.armed = Armed::None;`, add `self.returning = None;`.

(`release` returns early without the keyboard, which is right: a release that does nothing drops nothing. Edit's own `release` runs before Step 5 sets the field, so it does not drop the one being made.)

- [ ] **Step 7: Add `returns_to` and `take_keyboard_back`**

In `src/panel/state.rs`, directly after `take_keyboard`, add:

```rust
    /// The task box on `uuid` closing takes the keyboard back here: this
    /// panel's Edit opened it, and the keyboard has been neither taken nor
    /// given back since.
    pub fn returns_to(&self, uuid: &str) -> bool {
        !self.keyboard && self.returning.as_ref().is_some_and(|r| r.focus.uuid == uuid)
    }

    /// The task box on `uuid` closed. When this panel's Edit opened it, take
    /// the keyboard back as Edit left the panel: on that tab, or All once
    /// the tab has nothing under it, with the focus on the edited card's
    /// Edit, or the tab's first card once the edited card has left it.
    /// Nothing for any other box, and nothing with no cards to take the
    /// keyboard for. `agents` are as `take_keyboard`'s.
    pub fn take_keyboard_back(&mut self, uuid: &str, agents: Vec<String>) -> Vec<Effect> {
        if !self.returns_to(uuid) {
            return Vec::new();
        }
        let Some(back) = self.returning.take() else { return Vec::new() };
        if !self.take_keyboard(agents) {
            return Vec::new();
        }
        // The re-render keeps the focus where it can and falls back as
        // anywhere else: the card's body, the tab's first card, All.
        self.filter = back.filter;
        self.expanded = back.expanded;
        self.focus = Some(back.focus);
        self.rerender()
    }
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test --lib panel::state:: 2>&1 | tail -20`
Expected: PASS, the six new tests among them, and `a_letter_presses_only_a_button_the_card_has` unchanged.

- [ ] **Step 9: Run everything and check for warnings**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked" ; cargo build 2>&1 | grep -c warning`
Expected: every `test result: ok`, and `0`. (`returns_to` and `take_keyboard_back` are `pub`, so no dead-code warning before Task 2 uses them.)

- [ ] **Step 10: Commit**

```bash
git add src/panel/state.rs
git commit -m "$(cat <<'EOF'
feat(panel): remember where Edit left the panel

Edit gives the keyboard back so its task box can take it, and the
panel forgot its tab and focus as it did. The state now keeps them
until the box on that task closes, and takes the keyboard back to
them, falling back to the tab's first card or All as a refresh does.
Taking or giving back the keyboard meanwhile drops the return.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: The box closing hands the keyboard back to the panel

**Files:**
- Modify: `src/taskbox/open.rs:50-78` (`open_in`)
- Modify: `src/daemon.rs` — `serve_request`'s `Request::Panel` (~line 280-296) and `Request::Add` (~line 315), `open_task_box` (~line 350-370), plus two new functions after `open_task_box`
- Modify: `src/panel/surface.rs` — after `take_keyboard` (~line 590-597)
- Modify: `tests/e2e-panel.sh` — a new section after the "Escape puts it back exactly as it was before the keyboard took it" check (~line 446), before the `task panel` that follows it
- Modify: `README.md:54` (the Edit row), `README.md:418-421` (the one-box paragraph), `CONTEXT.md:242-249` (**Task box**)

**Interfaces:**
- Consumes: `PanelState::returns_to(&self, uuid: &str) -> bool`, `PanelState::take_keyboard_back(&mut self, uuid: &str, agents: Vec<String>) -> Vec<Effect>` (Task 1); `Panel::centre(self: &Rc<Self>, effects: Vec<Effect>)` (private, in `surface.rs`); `niri::workspaces() -> Result<Vec<niri_ipc::Workspace>>`; `niri::active_workspace_by_output(&[Workspace]) -> BTreeMap<String, Option<String>>` (connector → workspace name, the same keys as `PANELS`); `crate::link::live_agent_names(&Workspace) -> Vec<String>`.
- Produces: `taskbox::open_in(app, cfg, on_submit: impl Fn(Submission) + 'static, on_close: impl Fn() + 'static) -> Opened`; `Panel::returns_to(&self, uuid: &str) -> bool`; `Panel::take_keyboard_back(self: &Rc<Self>, uuid: &str, agents: Vec<String>)`.

This task is GTK and niri glue with no unit-testable logic of its own (the rules are Task 1's, tested there); its test is the e2e section in Steps 1-2 and 8.

- [ ] **Step 1: Write the e2e check**

In `tests/e2e-panel.sh`, find this block (around line 434-446):

```bash
    "${NENV[@]}" wtype 4
    sleep 1
    "${NENV[@]}" wtype -k Escape
    settle
    shot released || { summary; exit 1; }
    if same three released; then
        ok "Escape puts it back exactly as it was before the keyboard took it"
    else
        read -r x0 x1 y0 y1 < <(measure released three)
        bad "after Escape the screen differs from the tucked panel in columns
      ${x0}-${x1}, rows ${y0}-${y1}"
    fi
```

Directly after its closing `fi` (and before the `"${NENV[@]}" "$NIRITASKS" task panel` that follows), add:

```bash

    # Edit's box closing hands the keyboard back: on the To refine tab
    # (4), the second card, its Edit focused, exactly as before e opened
    # the box. Right four times from the body is Go to…, Start working,
    # Refine, Edit, a task on the list's row (Action::row).
    box_up() {
        nested niri msg -j windows 2>/dev/null | python3 -c "
import json,sys
print(any((w.get('app_id') or '')=='dev.niri-tasks.box' for w in json.load(sys.stdin)))
"
    }
    wait_box() {  # True|False
        for _ in $(seq 1 50); do [ "$(box_up)" = "$1" ] && return 0; sleep 0.1; done
        return 1
    }
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype 4
    sleep 1
    "${NENV[@]}" wtype -k Down
    for _ in 1 2 3 4; do "${NENV[@]}" wtype -k Right; done
    sleep 1
    shot edit_before || { summary; exit 1; }
    "${NENV[@]}" wtype e
    if wait_box True; then
        sleep 1
        "${NENV[@]}" wtype -k Escape
        wait_box False || bad "Escape did not close the box Edit opened"
        settle
        shot edit_back || { summary; exit 1; }
        if same edit_before edit_back; then
            ok "closing the box Edit opened puts the keyboard back on that tab, card and button"
        else
            read -r x0 x1 y0 y1 < <(measure edit_back edit_before)
            bad "after the box closed the screen differs from before Edit in columns
      ${x0}-${x1}, rows ${y0}-${y1} — the whole surface means the panel
      stayed tucked; a band means the tab or the focus moved"
        fi
        "${NENV[@]}" wtype -k Escape
        settle
    else
        bad "e on a card opened no task box"
    fi

    # A box opened from the CLI closes with the panel left tucked.
    glossary=$(task "+$TAG" "description.is:write the glossary" _uuids 2>/dev/null | head -n 1)
    "${NENV[@]}" "$NIRITASKS" task edit "$glossary" >/dev/null 2>&1
    if wait_box True; then
        sleep 1
        "${NENV[@]}" wtype -k Escape
        wait_box False || bad "Escape did not close the box task edit opened"
        settle
        shot outside_closed || { summary; exit 1; }
        if same three outside_closed; then
            ok "a box opened from outside the panel closes with the panel tucked away"
        else
            read -r x0 x1 y0 y1 < <(measure outside_closed three)
            bad "after a CLI box closed the screen differs from the tucked panel in
      columns ${x0}-${x1}, rows ${y0}-${y1}"
        fi
    else
        bad "task edit opened no task box"
    fi
```

Before running, check: `nested` is the helper `tests/lib/nested-niri.sh` defines (line ~129) and `tests/e2e-box.sh` uses it the same way for `niri msg -j windows`; the box's app id is `dev.niri-tasks.box` (`APP_ID` in `src/taskbox.rs`); "write the glossary" is one of the three tasks added earlier in the script, still pending and unplanned here.

- [ ] **Step 2: Run the e2e to verify it fails**

Run: `bash tests/e2e-panel.sh 2>&1 | grep -E "ok|bad|FAIL|skip" | grep -iE "edit|outside|box" `
Expected: `bad … after the box closed the screen differs from before Edit …` (the panel stays tucked today); the CLI check already passes. Keep off the nested niri's workspace while it runs (see the script's header). If wtype is missing the whole keyboard block is skipped: install it (`sudo apt install wtype`) rather than skipping the check.

- [ ] **Step 3: Give `open_in` an `on_close`**

In `src/taskbox/open.rs`, change `open_in`'s doc comment and signature, and the destroy handler. Replace:

```rust
/// Open the box inside the daemon's running Application, calling `on_submit`
/// with what was saved. The window is built once, in `build_window`.
```

with:

```rust
/// Open the box inside the daemon's running Application, calling `on_submit`
/// with what was saved, and `on_close` once the box is gone, however it
/// went. The window is built once, in `build_window`.
```

Replace:

```rust
pub fn open_in(app: &Application, cfg: BoxConfig, on_submit: impl Fn(Submission) + 'static) -> Opened {
```

with:

```rust
pub fn open_in(
    app: &Application,
    cfg: BoxConfig,
    on_submit: impl Fn(Submission) + 'static,
    on_close: impl Fn() + 'static,
) -> Opened {
```

Replace:

```rust
    window.connect_destroy(|_| {
        OPEN.with(|o| {
            o.borrow_mut().take();
        });
    });
```

with:

```rust
    window.connect_destroy(move |_| {
        OPEN.with(|o| {
            o.borrow_mut().take();
        });
        on_close();
    });
```

A box brought forward rather than opened (`Opened::Same`/`Other`) never calls its `on_close`: the open box's own handler fires when it goes.

- [ ] **Step 4: Give the panel the two calls**

In `src/panel/surface.rs`, directly after `take_keyboard`, add:

```rust
    /// The task box on `uuid` closing takes the keyboard back here: this
    /// panel's Edit opened it.
    pub fn returns_to(&self, uuid: &str) -> bool {
        self.state.borrow().returns_to(uuid)
    }

    /// The task box on `uuid` closed: when this panel's Edit opened it, take
    /// the keyboard back in the middle of the monitor, on the tab and card
    /// Edit was pressed on. `agents` are as `take_keyboard`'s.
    pub fn take_keyboard_back(self: &Rc<Self>, uuid: &str, agents: Vec<String>) {
        let effects = self.state.borrow_mut().take_keyboard_back(uuid, agents);
        if !effects.is_empty() {
            self.centre(effects);
        }
    }
```

- [ ] **Step 5: Factor out the agent lookup**

In `src/daemon.rs`, directly before `serve_request`, add:

```rust
/// The live herdr agents in the session of the workspace named `name`, for
/// a panel taking the keyboard: one `herdr agent list` for every card at
/// once. None for an unnamed workspace, or a session that is not running,
/// which answers at once with none.
fn live_agents(name: Option<&str>) -> Vec<String> {
    name.and_then(|n| Workspace::named(n).ok())
        .map(|w| crate::link::live_agent_names(&w))
        .unwrap_or_default()
}
```

and in `Request::Panel`, replace:

```rust
            if !focused_panel().is_some_and(|(panel, focused)| {
                // One `herdr agent list` per slide-out, for every card at
                // once; a session that is not running answers at once with
                // none.
                let agents = focused
                    .name
                    .as_deref()
                    .and_then(|n| Workspace::named(n).ok())
                    .map(|w| crate::link::live_agent_names(&w))
                    .unwrap_or_default();
                panel.take_keyboard(agents)
            }) {
```

with:

```rust
            if !focused_panel().is_some_and(|(panel, focused)| panel.take_keyboard(live_agents(focused.name.as_deref()))) {
```

- [ ] **Step 6: Hand the keyboard back when a box closes**

In `src/daemon.rs`, directly after `open_task_box`, add:

```rust
/// A task box on `uuid` closed, however it closed: the panel whose Edit
/// opened it takes the keyboard back, asking the session of the workspace
/// its monitor shows for its agents. A box from the CLI or a keybind finds
/// no such panel and asks niri and herdr nothing.
fn return_keyboard(uuid: &str) {
    let returning: Vec<(String, Rc<Panel>)> = PANELS.with(|p| {
        p.borrow()
            .iter()
            .filter(|(_, panel)| panel.returns_to(uuid))
            .map(|(connector, panel)| (connector.clone(), panel.clone()))
            .collect()
    });
    if returning.is_empty() {
        return;
    }
    let names = niri::workspaces()
        .map(|ws| niri::active_workspace_by_output(&ws))
        .unwrap_or_default();
    for (connector, panel) in returning {
        let name = names.get(&connector).cloned().flatten();
        panel.take_keyboard_back(uuid, live_agents(name.as_deref()));
    }
}
```

In `open_task_box`, replace:

```rust
    // The full uuid, not the one asked for, which may be a prefix.
    let uuid = t.uuid.clone();
    let opened = taskbox::open_in(
        app,
        taskbox::BoxConfig::for_task(mode, t),
        move |sub: taskbox::Submission| {
            if let Err(e) = task::replace_text(&uuid, &sub.description, &sub.notes) {
                notify::tasks(&e.to_string());
            }
        },
    );
```

with:

```rust
    // The full uuid, not the one asked for, which may be a prefix: the
    // panel's Edit remembers the card's, which is full.
    let uuid = t.uuid.clone();
    let closed = uuid.clone();
    let opened = taskbox::open_in(
        app,
        taskbox::BoxConfig::for_task(mode, t),
        move |sub: taskbox::Submission| {
            if let Err(e) = task::replace_text(&uuid, &sub.description, &sub.notes) {
                notify::tasks(&e.to_string());
            }
        },
        move || return_keyboard(&closed),
    );
```

and in `Request::Add`, add a fourth argument after the submit closure's closing `},` so the call ends:

```rust
                },
                || {},
            );
```

(A Save closes the window before `on_submit` saves, so the panel takes the keyboard back on the cards as they were; the next tick brings the edit in and its re-render keeps the focus on the card. Nothing to do about that.)

- [ ] **Step 7: Build and run the unit tests**

Run: `cargo build 2>&1 | grep -E "^(warning|error)" ; cargo build 2>&1 | grep -c warning ; cargo test 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: no errors, `0` warnings, every `test result: ok`.

- [ ] **Step 8: Run the e2e to verify it passes**

Run: `bash tests/e2e-panel.sh 2>&1 | tail -40`
Expected: `ok closing the box Edit opened puts the keyboard back on that tab, card and button`, `ok a box opened from outside the panel closes with the panel tucked away`, and no `bad` lines anywhere in the run (every later check still compares against frames this section must leave untouched). If a later check now fails, the section did not leave the panel tucked: check that its last `wtype -k Escape` ran.

- [ ] **Step 9: Check Save by hand**

The e2e covers Escape; Save is the same destroy, but check it once in a nested niri: `bash try.sh`, then in the nested window press Alt+Ctrl+T (Mod is plain Alt there), pick a tab with 2-6, move to a card that is not the first, press `e`, change the description, Ctrl+Enter. Expected: the panel slides back to the middle on that tab with that card's Edit focused, and the card shows the new description within a second. Then run `niritasks task edit <some uuid>` in the try shell and Esc the box: the panel stays tucked. `exit` the try shell when done. Report what you saw; if you cannot run a nested niri, say so rather than claiming it.

- [ ] **Step 10: Update the docs**

In `README.md`, the Edit row (line ~54), replace:

```markdown
| **Edit** (yellow) | `e` | Open it in the task box |
```

with:

```markdown
| **Edit** (yellow) | `e` | Open it in the task box. Closing the box, saved or not, hands the keyboard back to the panel on the same tab, on this card |
```

In `README.md`'s task box section, after the paragraph that starts "There is only ever one box." (ends "Save or discard it, then ask again."), add a paragraph:

```markdown
A box opened by a card's Edit hands the keyboard back to the task panel
when it closes, however it closes, on the tab Edit was pressed on with
that card focused (or the tab's first card, if that one has gone). A box
opened any other way, by Mod+Alt+T or `niritasks task edit`, leaves the
panel tucked away, as does taking or giving back the panel's keyboard
while the box is open.
```

In `CONTEXT.md`'s **Task box** entry, after the line `Only one is ever open: asking for another brings the open one forward.`, add:

```markdown
One a task card's Edit opened hands the keyboard back to that task panel as
it closes, on the same tab and card.
```

Run: `cargo test --bin niritasks readme 2>&1 | grep -E "test result|FAILED"` (the README's Commands block is tested against clap; this change does not touch it, but confirm).
Expected: `test result: ok`.

- [ ] **Step 11: Commit**

```bash
git add src/taskbox/open.rs src/daemon.rs src/panel/surface.rs tests/e2e-panel.sh README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
feat(panel): return to the task panel after edit

The task box's destroy, which Save, Cancel, Esc and niri's close all
end in, now tells the daemon, and the panel whose Edit opened that box
takes the keyboard back on the tab and card it left. A box opened from
the CLI or a keybind leaves the panel tucked away as before.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
