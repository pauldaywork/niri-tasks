# Open the Task Panel on a herdr Pane's Task Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One herdr key opens the task panel with the keyboard, the focus already on the card of the task this pane's Claude is working.

**Architecture:** `PanelState` gains `take_keyboard_on(uuid, agents)`, which takes the keyboard as `take_keyboard` does and then focuses that task's card body (expanding "+N more" if the card is past the cap), or reports that the card is not there. The IPC request `panel` gains an optional uuid (`panel <uuid>`); `niritasks task panel [<uuid>]` resolves a uuid8 to the full uuid before sending, and the daemon notifies when the card is not on the panel. The herdr `task-docs` plugin gets a second action, `task-docs.panel`, that finds the pane's task with its existing `find_task` and runs `niritasks task panel <uuid8>`.

**Tech Stack:** Rust (gtk4, clap, anyhow), Python 3 stdlib (`unittest`), herdr plugin TOML.

**Spec:** Taskwarrior task `9e400fbc-d371-4da6-afe1-6320df840239` — its description and notes (`task rc.json.array=on 9e400fbc export`). No separate spec file.

## Global Constraints

- `niritasks task panel [<uuid>]` takes an optional uuid or uuid8 and resolves it to the full uuid. The IPC becomes `panel <uuid>`, and a bare `panel` works as it does now.
- The panel opens on All with the focus on that card's body and its notes hidden.
- If the task isn't among the focused panel's cards (another workspace, or not pending), the panel opens as usual on the first card, with a notification saying why.
- A pane with no task gets a notification and nothing opens, as Task docs does now.
- The action is `task-docs.panel` in the existing `herdr/task-docs` plugin, reusing `find_task` and `notify`: no new plugin, no `install.sh` change.
- The README's key binding is a `[[keys.command]]` on `prefix+shift+t`.
- A bare `niritasks task panel` and Mod+Alt+Ctrl+T still focus the first card.
- Out of scope: showing a finished task's card; switching niri to the task's workspace.
- Commits: Conventional Commits, `feat(panel): …` / `feat(herdr): …`, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## File Structure

- `src/panel/state.rs` — `Taken` enum and `PanelState::take_keyboard_on`, with state tests.
- `src/panel/surface.rs` — `Panel::take_keyboard_on`, the GTK wrapper that centres the panel.
- `src/ipc.rs` — `Request::Panel(Option<String>)`, encode/decode and tests.
- `src/daemon.rs` — serve `Request::Panel(uuid)`; a pure `panel_notice` choosing the notification, with tests.
- `src/main.rs` — `TaskCommand::Panel { uuid: Option<String> }`, resolving to the full uuid.
- `tests/typed_task_number.rs` — the CLI sends the full uuid for a uuid8.
- `herdr/task-docs/task_docs.py`, `herdr/task-docs/herdr-plugin.toml`, `herdr/task-docs/tests/test_task_docs.py` — the `panel` action.
- `README.md`, `llms.txt` — the command and the key binding.

---

### Task 1: `take_keyboard_on` in the panel state and surface

**Files:**
- Modify: `src/panel/state.rs` (add `Taken` near `Focus`, ~line 28; add the method after `take_keyboard`, ~line 427; tests in `mod tests`)
- Modify: `src/panel/surface.rs` (after `take_keyboard`, ~line 601)

**Interfaces:**
- Consumes: `PanelState::take_keyboard(&mut self, agents: Vec<String>) -> bool`, `Focus::body(&str)`, `Filter::All.pick(&[Card]) -> Vec<Card>`, `model::CAP` (8), `self.notes: Option<String>`, `self.expanded: bool`.
- Produces:
  - `pub enum Taken { Card, First, Not }` in `crate::panel::state` (derive `Debug, Clone, Copy, PartialEq, Eq`).
  - `PanelState::take_keyboard_on(&mut self, uuid: &str, agents: Vec<String>) -> Taken`
  - `Panel::take_keyboard_on(self: &Rc<Self>, uuid: &str, agents: Vec<String>) -> Taken`

`Taken::Card`: the keyboard is taken with the focus on that card's body. `Taken::First`: taken, but the card is not under All, so the focus is on the first card as `take_keyboard` leaves it. `Taken::Not`: no cards, nothing taken.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/panel/state.rs` (helpers `pending`, `noted`, `card`, `uuids`, `focused`, `keyboard`, `key` already exist there):

```rust
    /// herdr's task-docs.panel: the keyboard on All with the focus on that
    /// task's card body, not the first card.
    #[test]
    fn taking_the_keyboard_on_a_task_focuses_its_card() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a", "b", "c"]));
        assert_eq!(state.take_keyboard_on("b", Vec::new()), Taken::Card);
        assert_eq!(state.focus, focused("b", Slot::Body));
        assert_eq!(state.filter, Filter::All);
    }

    /// From another tab, with another card's notes open, it still lands on
    /// All with every card's notes hidden.
    #[test]
    fn taking_the_keyboard_on_a_task_picks_all_and_hides_notes() {
        let mut state = keyboard(vec![noted("a"), card("b", Status::Planned)]);
        state.on_press("a", Slot::Body);
        assert!(state.shows_notes("a"));
        key(&mut state, KeyAction::Filter(Filter::Planned));
        assert_eq!(state.take_keyboard_on("a", Vec::new()), Taken::Card);
        assert_eq!(state.filter, Filter::All);
        assert_eq!(state.focus, focused("a", Slot::Body));
        assert!(!state.shows_notes("a"));
    }

    /// A card past "+N more" is shown: the list opens expanded.
    #[test]
    fn taking_the_keyboard_on_a_hidden_task_expands_the_list() {
        let names: Vec<String> = (0..10).map(|i| format!("t{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut state = PanelState::default();
        state.set_cards(&pending(&refs));
        assert_eq!(state.take_keyboard_on("t9", Vec::new()), Taken::Card);
        assert_eq!(state.focus, focused("t9", Slot::Body));
        assert!(uuids(&state).contains(&"t9".to_string()), "{:?}", uuids(&state));
    }

    /// Another workspace's task, or one finished or waiting, is not under
    /// All: the panel opens as usual on the first card.
    #[test]
    fn taking_the_keyboard_on_a_task_not_here_focuses_the_first_card() {
        let mut state = PanelState::default();
        state.set_cards(&[card("a", Status::Pending), card("w", Status::Waiting), card("f", Status::Finished)]);
        assert_eq!(state.take_keyboard_on("elsewhere", Vec::new()), Taken::First);
        assert_eq!(state.focus, focused("a", Slot::Body));
        assert_eq!(state.take_keyboard_on("w", Vec::new()), Taken::First);
        assert_eq!(state.focus, focused("a", Slot::Body));
        assert_eq!(state.take_keyboard_on("f", Vec::new()), Taken::First);
        assert_eq!(state.focus, focused("a", Slot::Body));
    }

    #[test]
    fn taking_the_keyboard_on_a_task_with_no_cards_takes_nothing() {
        let mut state = PanelState::default();
        assert_eq!(state.take_keyboard_on("a", Vec::new()), Taken::Not);
        assert_eq!(state.footer(), None, "no keyboard");
    }
```

`KeyAction::Filter(Filter)` is the 1–6 tab key (`src/panel/keys.rs`). `state.focus` and `state.filter` are private fields, readable from the in-file `mod tests`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib panel::state::tests::taking_the_keyboard_on`
Expected: compile error, `no method named take_keyboard_on` / `cannot find type Taken`.

- [ ] **Step 3: Implement**

In `src/panel/state.rs`, after `impl Focus { … }`:

```rust
/// What taking the keyboard on one task's card came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken {
    /// The focus is on that card's body.
    Card,
    /// The card is not under All, so the focus is on the first card.
    First,
    /// No cards to take the keyboard for.
    Not,
}
```

After `take_keyboard`:

```rust
    /// Take the keyboard as `take_keyboard` does, with the focus on `uuid`'s
    /// card body instead of the first card and every card's notes hidden:
    /// herdr's task-docs.panel, from that task's pane. A card past "+N more"
    /// opens the list expanded. A task that is not under All, on another
    /// workspace or waiting or finished, leaves the focus on the first card.
    pub fn take_keyboard_on(&mut self, uuid: &str, agents: Vec<String>) -> Taken {
        if !self.take_keyboard(agents) {
            return Taken::Not;
        }
        self.notes = None;
        let Some(at) = Filter::All.pick(&self.all).iter().position(|c| c.uuid.as_deref() == Some(uuid)) else {
            return Taken::First;
        };
        if at >= model::CAP {
            self.expanded = true;
        }
        self.focus = Some(Focus::body(uuid));
        Taken::Card
    }
```

`Filter::pick(self, cards: &[Card]) -> Vec<Card>` is in `src/panel/model.rs:224`.

In `src/panel/surface.rs`, after `take_keyboard`:

```rust
    /// Take the keyboard as `take_keyboard` does, focused on `uuid`'s card:
    /// herdr's task-docs.panel. Centred unless there were no cards.
    pub fn take_keyboard_on(self: &Rc<Self>, uuid: &str, agents: Vec<String>) -> Taken {
        let taken = self.state.borrow_mut().take_keyboard_on(uuid, agents);
        if taken != Taken::Not {
            self.centre(vec![Effect::Render]);
        }
        taken
    }
```

and add `Taken` to surface.rs's import at line 168: `use super::state::{Armed, Effect, Focus, Mode, PanelState, Shown, Slot, Taken};`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib panel::state`
Expected: all pass, the five new ones included.

- [ ] **Step 5: Commit**

```bash
git add src/panel/state.rs src/panel/surface.rs
git commit -m "feat(panel): take the keyboard on a given task's card

The panel can now open with the focus on one task's card body, on All
with notes hidden, expanding past +N more when it must; a task not under
All leaves the focus on the first card, as before.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `panel <uuid>` from the CLI through the daemon

**Files:**
- Modify: `src/ipc.rs` (enum ~line 37, encode ~51, decode ~60, module doc ~21, tests)
- Modify: `src/daemon.rs` (`serve_request` ~line 297; new const near `NO_TASKS` ~229; `panel_notice` near `no_cards_text` ~262; tests ~503)
- Modify: `src/main.rs` (`TaskCommand::Panel` ~line 57, dispatch ~339)
- Modify: `tests/typed_task_number.rs` (end of the one test function)
- Modify: `README.md:338`, `llms.txt:26`

**Interfaces:**
- Consumes: `Panel::take_keyboard_on(&Rc<Self>, &str, Vec<String>) -> Taken`, `crate::panel::state::Taken { Card, First, Not }` (Task 1); `task::get(&str) -> Result<Option<Task>>` whose `Task.uuid` is the full uuid.
- Produces: `ipc::Request::Panel(Option<String>)`; `daemon::NOT_ON_PANEL: &str`; `fn panel_notice(asked: bool, taken: Taken, tag: impl FnOnce() -> anyhow::Result<String>) -> Option<String>`; CLI `niritasks task panel [<uuid>]`.

- [ ] **Step 1: Write the failing IPC tests**

In `src/ipc.rs` tests, change `round_trips_every_request`'s list: replace `Request::Panel,` with

```rust
            Request::Panel(None),
            Request::Panel(Some("9e400fbc-d371-4da6-afe1-6320df840239".into())),
```

replace `send(&Request::Panel)` in `no_daemon_is_one_sentence` with `send(&Request::Panel(None))`, and add:

```rust
    /// A bare `panel`, from Mod+Alt+Ctrl+T or an older CLI, still opens on
    /// the first card; `panel <uuid>` names the card to open on.
    #[test]
    fn panel_carries_an_optional_uuid() {
        assert_eq!(Request::Panel(None).encode(), "panel");
        assert_eq!(Request::Panel(Some("abc".into())).encode(), "panel abc");
        assert_eq!(Request::decode("panel\n"), Some(Request::Panel(None)));
        assert_eq!(Request::decode("panel \n"), Some(Request::Panel(None)));
        assert_eq!(Request::decode("panel abc\n"), Some(Request::Panel(Some("abc".into()))));
    }
```

- [ ] **Step 2: Write the failing daemon test**

In `src/daemon.rs` tests, next to `no_cards_says_why`:

```rust
    /// A bare panel request says nothing once it has the keyboard; one for a
    /// task whose card is not here says so; no cards says why, either way.
    #[test]
    fn the_panel_says_why_it_is_not_on_the_task() {
        use crate::panel::state::Taken;
        let web = || Ok("web".to_string());
        let unasked = || -> anyhow::Result<String> { panic!("the tag is read only for no cards") };
        assert_eq!(panel_notice(false, Taken::First, unasked), None);
        assert_eq!(panel_notice(true, Taken::Card, unasked), None);
        assert_eq!(panel_notice(true, Taken::First, unasked), Some(NOT_ON_PANEL.to_string()));
        assert_eq!(panel_notice(false, Taken::Not, web), Some(NO_TASKS.to_string()));
        assert_eq!(panel_notice(true, Taken::Not, web), Some(NO_TASKS.to_string()));
    }
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test --lib ipc daemon::tests`
Expected: compile errors — `Request::Panel` takes no argument, `panel_notice` / `NOT_ON_PANEL` not found.

- [ ] **Step 4: Implement the IPC change**

In `src/ipc.rs`, the module doc's protocol block: replace the `panel` line with

```text
//! panel
//! panel <uuid>
```

The variant:

```rust
    /// Slide out the focused monitor's task panel and give it the keyboard,
    /// focused on this task's card when one is named, else the first card.
    Panel(Option<String>),
```

encode:

```rust
            Request::Panel(None) => "panel".into(),
            Request::Panel(Some(uuid)) => format!("panel {uuid}"),
```

decode, replacing `("panel", _) => Some(Request::Panel),`:

```rust
            ("panel", rest) => Some(Request::Panel(rest.map(str::trim).filter(|u| !u.is_empty()).map(String::from))),
```

- [ ] **Step 5: Implement the daemon change**

In `src/daemon.rs`, after `NO_TASKS`:

```rust
/// What herdr's task-docs.panel says when the pane's task has no card under
/// All on the focused workspace's panel, which then opens on its first card.
pub const NOT_ON_PANEL: &str =
    "That task's card is not on this panel: it is on another workspace, or waiting or finished. The panel opened on the first card.";
```

After `no_cards_text`:

```rust
/// What a panel request says, if anything: why there was no card to take
/// the keyboard for, or, when `asked` named a task, that its card is not on
/// the panel. `tag` reads the focused workspace's tag, as `no_cards_text`
/// takes it; it asks niri again, so it is called only when there were no
/// cards.
fn panel_notice(asked: bool, taken: Taken, tag: impl FnOnce() -> anyhow::Result<String>) -> Option<String> {
    match taken {
        Taken::Not => Some(no_cards_text(tag())),
        Taken::First if asked => Some(NOT_ON_PANEL.to_string()),
        _ => None,
    }
}
```

Add `use crate::panel::state::Taken;` to the daemon's imports (next to `use crate::panel::{model, style, Panel};`). Replace the `Request::Panel => { … }` arm in `serve_request` with:

```rust
        Request::Panel(uuid) => {
            // No panel reads as no cards: either way there is nothing to
            // hand the keyboard to, and the tag says why.
            let taken = focused_panel().map_or(Taken::Not, |(panel, focused)| {
                let agents = live_agents(focused.name.as_deref());
                match uuid.as_deref() {
                    Some(uuid) => panel.take_keyboard_on(uuid, agents),
                    None if panel.take_keyboard(agents) => Taken::First,
                    None => Taken::Not,
                }
            });
            let tag = || Workspace::focused().map(|w| w.tag().to_string());
            if let Some(text) = panel_notice(uuid.is_some(), taken, tag) {
                notify::tasks(&text);
            }
        }
```

- [ ] **Step 6: Implement the CLI change**

In `src/main.rs`, replace the variant:

```rust
    /// Slide out the task panel and work its tasks with the keyboard (Mod+Alt+Ctrl+T)
    ///
    /// With a task, the focus starts on its card, as herdr's task-docs.panel
    /// opens it from the task's pane; a task with no card on the focused
    /// workspace's panel opens it on the first card, with a notification.
    Panel {
        /// The task's uuid, or its first 8 characters, as in a
        /// `task/<slug>-<uuid8>` branch
        uuid: Option<String>,
    },
```

and the dispatch arm:

```rust
        // The daemon draws the panel; with none, say so. A uuid8 is resolved
        // here, since the panel's cards carry full uuids.
        TaskCommand::Panel { uuid } => {
            let uuid = match uuid {
                Some(u) => Some(task::get(&u)?.context("task not found")?.uuid),
                None => None,
            };
            ipc::send(&ipc::Request::Panel(uuid))?
        }
```

- [ ] **Step 7: Add the CLI integration check**

At the end of `a_task_number_names_everything_after_the_tasks_uuid` in `tests/typed_task_number.rs`:

```rust
    // ---- task panel hands the daemon the full uuid for a uuid8 ----------
    // The panel's cards carry full uuids, so a uuid8 from herdr's
    // task-docs.panel must be resolved before it is sent. A listener stands
    // in for the daemon in the sandbox's runtime folder.
    let daemon = UnixListener::bind(s.dir.join("run/niri-tasks.sock")).expect("bind the fake daemon socket");
    let out = s.niritasks(&["task", "panel", uuid8], &home, "", NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let (stream, _) = daemon.accept().expect("the CLI connects");
    let mut sent = String::new();
    BufReader::new(stream).read_line(&mut sent).unwrap();
    assert_eq!(sent, format!("panel {}\n", s.uuid));

    let out = s.niritasks(&["task", "panel", "ffffffff"], &home, "", NO_WORKTREES);
    assert!(!out.status.success(), "an unknown task is refused");
    assert!(String::from_utf8_lossy(&out.stderr).contains("task not found"));
```

Also add one line to the file's top doc comment: "`task panel` likewise sends the daemon the full uuid."

- [ ] **Step 8: Run the tests**

Run: `cargo test`
Expected: all pass, including `ipc::tests::panel_carries_an_optional_uuid`, `daemon::tests::the_panel_says_why_it_is_not_on_the_task` and `a_task_number_names_everything_after_the_tasks_uuid`. If `typed_task_number` fails at Refine with "Refine would leave these sockets in reach", that is the machine's real socket, as its comment says — not this change; report it.

- [ ] **Step 9: Document the command**

`README.md:338`, replace the `task panel` line with:

```
niritasks task panel [<uuid>]      # hand the task panel the keyboard (Mod+Alt+Ctrl+T); with a uuid or
                                   #   uuid8, focused on that task's card (herdr's task-docs.panel)
```

`llms.txt:26`, replace the row with:

```
| `niritasks task panel [<uuid>]` | Hand the task panel the keyboard (Mod+Alt+Ctrl+T), on All. With a uuid or uuid8, the focus starts on that task's card body, notes hidden; a task with no card under All on the focused workspace's panel opens it on the first card with a notification. A notification instead on a workspace with no tasks or no name; exit 1 with no daemon or an unknown task | GUI |
```

- [ ] **Step 10: Commit**

```bash
git add src/ipc.rs src/daemon.rs src/main.rs tests/typed_task_number.rs README.md llms.txt
git commit -m "feat(panel): open the panel on a task with task panel <uuid>

niritasks task panel takes an optional uuid or uuid8, resolves it to the
full uuid and asks the daemon for panel <uuid>, which focuses that
task's card. A task with no card on the panel opens it on the first card
with a notification; a bare panel request works as before.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The herdr `task-docs.panel` action

**Files:**
- Modify: `herdr/task-docs/task_docs.py`
- Modify: `herdr/task-docs/herdr-plugin.toml`
- Modify: `herdr/task-docs/tests/test_task_docs.py`
- Modify: `README.md` (new subsection after "Annotating a task's documents in herdr", before `## Commands`), `llms.txt:85`

**Interfaces:**
- Consumes: `niritasks task panel <uuid8>` (Task 2) — exit 0 on success, non-zero with an `Error: …` on stderr; `task_docs.find_task(pane, cwd) -> str | None`, `task_docs.notify(body)`, `task_docs.context()`.
- Produces: `task_docs.panel_action()`; `task_docs.py panel`; plugin action id `panel` (`task-docs.panel`).

- [ ] **Step 1: Write the failing tests**

In `herdr/task-docs/tests/test_task_docs.py`, add a fake after `FAKE_NIRITASKS`:

```python
# niritasks: logs its arguments; exits $FAKE_NIRITASKS_EXIT with an error, as
# niritasks does with no daemon.
FAKE_NIRITASKS_PANEL = """#!/bin/sh
printf '%s\\n' "$@" > "$FAKE_NIRITASKS_LOG"
if [ -n "$FAKE_NIRITASKS_EXIT" ]; then
    echo "Error: The niri-tasks daemon is not running" >&2
    exit "$FAKE_NIRITASKS_EXIT"
fi
"""
```

and a class after `TheAction`:

```python
class ThePanelAction(PluginTest):
    def setUp(self):
        super().setUp()
        fake(self.bin, "niritasks", FAKE_NIRITASKS_PANEL)
        self.niritasks_log = self.tmp / "niritasks.log"
        patcher = mock.patch.dict(os.environ, {"FAKE_NIRITASKS_LOG": str(self.niritasks_log)})
        patcher.start()
        self.addCleanup(patcher.stop)

    def asked(self):
        return self.niritasks_log.read_text().split("\n")[:-1] if self.niritasks_log.exists() else None

    def test_a_tasks_pane_opens_the_panel_on_its_task(self):
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        task_docs.panel_action()
        self.assertEqual(self.asked(), ["task", "panel", "1234abcd"])
        self.assertEqual(self.notified(), "")

    def test_a_shell_on_the_tasks_branch_opens_it_too(self):
        self.on_branch("task/fix-1234abcd")
        task_docs.panel_action()
        self.assertEqual(self.asked(), ["task", "panel", "1234abcd"])

    def test_it_needs_no_claude_or_plannotator(self):
        self.on_branch("task/fix-1234abcd")
        task_docs.panel_action()
        self.assertNotIn("plannotator", self.notified())
        self.assertNotIn("no Claude", self.notified())

    def test_a_pane_with_no_task_says_so_and_opens_nothing(self):
        task_docs.panel_action()
        self.assertIn("Task panel: no task here", self.notified())
        self.assertIsNone(self.asked())

    def test_a_failing_niritasks_says_why(self):
        self.on_branch("task/fix-1234abcd")
        with mock.patch.dict(os.environ, {"FAKE_NIRITASKS_EXIT": "1"}):
            task_docs.panel_action()
        self.assertIn("Task panel: could not open the panel", self.notified())
        self.assertIn("daemon is not running", self.notified())

    def test_without_niritasks_it_says_so(self):
        self.on_branch("task/fix-1234abcd")
        (self.bin / "niritasks").unlink()
        task_docs.panel_action()
        self.assertIn("Task panel: niritasks not found", self.notified())
```

And a test that `main` routes `panel`:

```python
class TheCommandLine(unittest.TestCase):
    def test_panel_is_a_command(self):
        with mock.patch.object(task_docs, "panel_action") as panel:
            self.assertEqual(task_docs.main(["task_docs.py", "panel"]), 0)
        panel.assert_called_once_with()
```

Note: `main` builds its `commands` dict at call time from module globals, so `mock.patch.object` takes effect — keep it that way in Step 3.

- [ ] **Step 2: Run them to verify they fail**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs`
Expected: errors, `module 'task_docs' has no attribute 'panel_action'`.

- [ ] **Step 3: Implement**

In `herdr/task-docs/task_docs.py`, factor the no-task message so both actions say it:

```python
def no_task(title):
    """What an action says in a pane with no task, under its own title."""
    notify(f"{title}: no task here. This pane runs no task's Claude (work-/task-/report-<uuid8>) "
           "and is not on a task/<slug>-<uuid8> branch.")
```

and in `open_action` replace the inline `notify("Task docs: no task here. …")` with `no_task("Task docs")`. Then add after `open_action`:

```python
def panel_action():
    """The task-docs.panel action: slide out the task panel with the keyboard,
    focused on the pane's task's card, or say why not. The daemon says when
    the card is not on the panel."""
    ctx = context()
    pane = os.environ.get("HERDR_PANE_ID") or ctx.get("focused_pane_id")
    cwd = ctx.get("focused_pane_cwd")
    uuid8 = find_task(pane, cwd)
    if not uuid8:
        no_task("Task panel")
        return
    try:
        result = subprocess.run(["niritasks", "task", "panel", uuid8],
                                capture_output=True, text=True, check=False)
    except OSError:
        notify("Task panel: niritasks not found on PATH.")
        return
    if result.returncode != 0:
        notify(f"Task panel: could not open the panel. {result.stderr.strip()[-300:]}")
```

Update `main`:

```python
def main(argv):
    commands = {"open": open_action, "panel": panel_action, "pick": pick, "review": review}
    if len(argv) != 2 or argv[1] not in commands:
        print("usage: task_docs.py open|panel|pick|review", file=sys.stderr)
        return 2
    commands[argv[1]]()
    return 0
```

Update the module docstring's first lines to:

```python
"""herdr's task-docs plugin: annotate one of a task's documents in plannotator-tui,
or open the task panel on the task.

`task_docs.py open` is the annotate action. … (keep the existing text)
`task_docs.py panel` is the panel action: it finds the pane's task the same
way and runs `niritasks task panel <uuid8>`, which slides out the task panel
with the keyboard on that task's card.
…
"""
```

In `herdr/task-docs/herdr-plugin.toml`, extend the header comment's first sentence with ", or open the task panel on it", set `description = "Annotate a niri-tasks task's reports, notes and plans in plannotator-tui beside its Claude, or open the task panel on it"`, and add after the `open` action:

```toml
# Press in a task's pane: finds the task as `open` does and runs
# `niritasks task panel <uuid8>`, sliding out the task panel with the
# keyboard on that task's card.
[[actions]]
id = "panel"
title = "Task panel: open it on this task's card"
contexts = ["pane"]
command = ["python3", "task_docs.py", "panel"]
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs`
Expected: OK, all tests including `ThePanelAction` and `TheCommandLine`.

- [ ] **Step 5: Document the key**

In `README.md`, insert before `## Commands` (after the paragraph ending "The Markdown is what plannotator-tui opens."):

````markdown
### Opening the task panel on a herdr pane's task

The same plugin has a second action, `task-docs.panel`. Bind it too:

```toml
[[keys.command]]
key = "prefix+shift+t"
type = "plugin_action"
command = "task-docs.panel"
description = "task panel: open it on this task's card"
```

Press it in a task's pane, found as for Task docs: the task panel slides
out with the keyboard, on All, the focus on that task's card with its notes
hidden. A task with no card on the focused workspace's panel, one on
another workspace or waiting or finished, opens the panel on its first card
with a notification saying why. A pane with no task gets a notification and
nothing opens. It runs `niritasks task panel <uuid8>`.
````

`llms.txt:85`, extend the task-docs line's description by appending: `; its second action, task-docs.panel, runs \`niritasks task panel <uuid8>\` to open the task panel on the pane's task`.

- [ ] **Step 6: Validate the manifest**

Run: `python3 -c "import tomllib; d=tomllib.load(open('herdr/task-docs/herdr-plugin.toml','rb')); print([a['id'] for a in d['actions']])"`
Expected: `['open', 'panel']`.

If herdr is installed (`command -v herdr`), also run `herdr config check` after adding the key binding to `~/.config/herdr/config.toml` only if the user asks; do not edit their config unasked.

- [ ] **Step 7: Commit**

```bash
git add herdr/task-docs README.md llms.txt
git commit -m "feat(herdr): add task-docs.panel to open the panel on a task

A second action in the task-docs plugin finds the pane's task as Task
docs does and runs niritasks task panel <uuid8>, so one key opens the
task panel focused on the card of the task that pane is working.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Final check (done when)

- [ ] `cargo test` passes.
- [ ] `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs` passes.
- [ ] `bash tests/e2e-panel.sh` still passes (bare `task panel` unchanged), if a nested niri is available here; otherwise say it was not run.
- [ ] Manual, after landing (the plugin is linked from `~/Projects/niri-tasks` by `install.sh`, so a worktree's copy is not the one herdr runs): bind `prefix+shift+t`, `herdr server reload-config`, press it in a task's Claude pane — the panel slides out focused on that task's card. Mod+Alt+Ctrl+T still focuses the first card.
