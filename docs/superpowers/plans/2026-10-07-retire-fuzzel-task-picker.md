# Retire the Fuzzel Task Picker and Menu — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The task panel does everything the fuzzel task picker and action menu did, so both go: a card's action row gains Grill me, Complete and Move to workspace, and Move picks its folder from a project list drawn like the task cards.

**Architecture:** The project folders and the retagging move into the lib (`project::list`, `project::destination`, `project::move_task`) behind a new `niritasks task move <uuid> <folder>` subcommand, so the panel still runs the CLI for every action. `src/rows.rs`, `src/menu.rs`, `task list`, `task menu` and the menu's fuzzel pickers are deleted; the two fallbacks that opened the picker become notifications, and a press on a card's body does nothing. `Action` gains `Complete` and `Move` and loses `Note` (no view shows it any more). Move is a mode of `PanelState` (pure, unit-tested): it asks the surface for the folders with a new `Effect::ListProjects`, then Up/Down walk them, Enter or a click spawns `task move`, and Escape goes back to the cards on the same card.

**Tech Stack:** Rust 2021, clap 4 (derive), GTK4 (gtk4-rs) and its CSS, Taskwarrior 2.6 CLI. No new crates.

**Spec:** Taskwarrior task `968be47e-f4dd-4569-9a56-6a038fcfd232`. Read it with `task rc.json.array=on 968be47e-f4dd-4569-9a56-6a038fcfd232 export`. Its description and notes are the spec.

## Global Constraints

- New row buttons: Grill me (already an `Action`), Complete (new `Action::Complete`, runs `task status <uuid> completed`, leaves the list) and Move to workspace (new `Action::Move`). Note and the Update status list go; Edit/Stop/Wait/Back/Remove cover the rest.
- Move swaps the panel's cards for a project list: the `~/Projects` folders from `project::move_destinations`, drawn like task cards. Up/Down walk it, Enter or a click spawns `niritasks task move <uuid> <folder>`, Escape goes back to the task list on the same card. Pure in `PanelState` so it is unit-tested.
- `projects()` and `task_move`'s retagging move into the lib (`project::list`). `task move <uuid> <folder>` is a CLI subcommand that refuses a folder not on the list.
- Both picker fallbacks become a notification: the `require_workspace_tag` error on an unnamed workspace, else "no tasks here, Mod+Alt+T adds one"; with no daemon, say the daemon is not running.
- Enter on a card's body and a click on it do nothing (task b96c1f78 later gives them the notes toggle).
- Keep `picker::Picker`, `clamp_lines`, `clamp_project_width` and `fuzzel/picker.ini` for the Mod+Alt+W project picker; `clamp_task_width` goes.
- Done when: `cargo test` passes (README and llms.txt every-subcommand tests included); `task list` and `task menu` are unknown subcommands; Grill me, Complete and Move work from a card, Move listing projects as cards and retagging the task; Enter or a click on a body does nothing; Mod+Alt+Ctrl+T on an empty or unnamed workspace shows a notification, not fuzzel.
- Out of scope: replacing the Mod+Alt+W project picker with the new list, the notes toggle (b96c1f78), workspace rename (fb4f5886) and `task active`.
- House style: every new item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commits use Conventional Commits (`refactor(panel): …`, imperative, lowercase, ≤72 chars) and end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions made while planning (flag any you disagree with)

- **`Action::Note` goes.** With the menu gone nothing shows it. `niritasks task note` stays, and Edit's box still edits every note.
- **Letters:** Grill me `i` (it interviews), Complete `c`, Move to workspace `m`. None was taken; the digits and brackets are the tabs'.
- **Icons:** Complete is Font Awesome's check `\u{f00c}`; Move to workspace its open folder `\u{f07c}`; a folder card on the project list its shut folder `\u{f07b}`.
- **Colours:** Grill me keeps Refine's mauve (it already does in `style::colour`). Complete is mocha's green `#a6e3a1`, paler than Start's `ACTIVE`. Move is mocha's sapphire `#74c7ec`.
- **Every task gets Complete and Move**, waiting ones too, as the menu's Update status and Move to workspace were on every task. Grill me goes where Refine goes (not on a waiting task).
- **The row is every action now.** `Action::ROW` becomes `Action::ALL`, in the order Session, Back, Start, Refine, Grill, Edit, Speak, UpNext, Complete, Move, Stop, Wait, Remove.
- **The hint goes blank while Remove is armed.** The widest row is now 11 buttons, and with Confirm remove's 128px the 9pt hint has room for 35 characters, short of `Del: Remove · Ctrl+Enter: start working` (39). Armed, the button says what Enter does, so the hint has nothing to add. Unarmed the hint has room for 53 characters on the widest row; the longest is 48.
- **The panel keeps the keyboard after a move**, as Back, Waiting and Remove do: the card leaves the list, so the focus goes to its neighbour, and the list stays up. Complete does the same.
- **The project list lists afresh on every Move press** (an `Effect::ListProjects` the surface answers by reading `~/Projects`), so a folder made since shows. A line over the folders names the task. The tab bar hides while it shows, and the footer names its keys. Every other key does nothing there.
- **`task move` takes the workspace the way `task add` does** (`caller_workspace_tag`: the herdr session's in a herdr pane, else the focused one), and refuses a task not on that workspace's tag, since dropping a tag it lacks would leave it on two workspaces. The panel's spawn focuses the panel's monitor first, so from a card it is that panel's workspace.
- **`task::Status::from_label` goes** (only the menu read a label back). `Status::ALL` and `label` stay: the CLI test and the notification use them.
- **`surface::open_menu` becomes the private `run_niritasks`**: the daemon no longer calls it.
- **The e2e test checks `c` and `m`/Escape, not Enter on a folder.** The nested niri's spawns run with the real `HOME`, so a move would act on the real `~/Projects`. `tests/write_path.rs` checks the retagging against the sandbox instead.

## File Structure

- `src/project.rs` — `list`, private `folders_in`, `destination`, `move_task`, and their tests.
- `src/main.rs` — `TaskCommand::Move` and its handler; `TaskCommand::List`, `TaskCommand::Menu`, `task_list`, `task_menu`, `task_status`, `task_move` and `projects` go; `task panel` without a daemon errors.
- `src/rows.rs`, `src/menu.rs` — deleted. `src/lib.rs` drops their `mod`s.
- `src/picker.rs` — `clamp_task_width` and its test go.
- `src/task.rs` — `Status::from_label` goes; comments naming the menu or picker change.
- `src/daemon.rs` — the no-cards fallback becomes a notification (`no_cards_text`).
- `src/actions.rs` — `Complete`, `Move` in; `Note` out.
- `src/panel/actions.rs` — `ROW`, letters, classes, `keeps_keyboard`, `leaves_the_list`, `CARD_KEYS`, `MOVE_KEYS`, `FOLDER_ICON`, the hint-room test.
- `src/panel/style.rs` — `COMPLETE`, `MOVE`, `colour`.
- `src/panel/state.rs` — body press a no-op; `Moving`, `Effect::ListProjects`, `Effect::FocusFolder`, `show_projects`, `on_folder` and the move mode's keys.
- `src/panel/surface.rs` — draws the project list, answers the two effects, blanks the hint while Remove is armed, `run_niritasks`.
- `src/link.rs`, `src/herdr.rs`, `src/work.rs`, `src/refine.rs`, `src/taskbox.rs`, `src/panel/model.rs`, `tests/write_path.rs`, `tests/e2e-box.sh` — comments only.
- `tests/write_path.rs` — a `move_task` section. `tests/e2e-panel.sh` — a Complete and Move block.
- `README.md`, `llms.txt`, `CONTEXT.md`, `.claude/skills/finish-worktree/SKILL.md` — docs.

Run `cargo test` from the worktree root. Build warnings count: finish each task with `cargo build 2>&1 | grep -c warning` at 0 (it is 0 now).

---

### Task 1: The project list in the lib, and `task move`

**Files:**
- Modify: `src/project.rs` (module doc; new items after `move_destinations`; tests)
- Modify: `src/main.rs` (`TaskCommand` enum, `task_command`, delete `projects()`, its two callers, tests)
- Modify: `tests/write_path.rs` (imports; a section before `// ---- tags scope the list`)
- Modify: `README.md` (`## Commands` block), `llms.txt` (command table)

**Interfaces:**
- Produces:
  - `pub fn project::list() -> anyhow::Result<(std::path::PathBuf, Vec<String>)>` — `~/Projects` and its folders, sorted, dotfiles left out
  - `pub fn project::destination(names: &[String], current_tag: &str, folder: &str) -> anyhow::Result<String>` — the folder's tag, refused off `move_destinations`
  - `pub fn project::move_task(task: &crate::task::Task, current_tag: &str, folder: &str, names: &[String]) -> anyhow::Result<String>` — retags and returns the new tag
  - CLI: `niritasks task move <uuid> <folder>`

- [ ] **Step 1: Write the failing unit tests**

Add to `mod tests` in `src/project.rs`:

```rust
    fn projects() -> Vec<String> {
        ["niri-tasks", "alp-theme", "keystone"].iter().map(|s| s.to_string()).collect()
    }

    /// A folder on the list moves the task to its folded tag, the one opening
    /// that project gives its workspace.
    #[test]
    fn a_destination_is_the_folders_tag() {
        assert_eq!(destination(&projects(), "niri_tasks", "alp-theme").unwrap(), "alp_theme");
    }

    /// The folder the task is already on is no move, and a folder that is not
    /// there would put the task on a tag no workspace shows.
    #[test]
    fn a_destination_off_the_list_is_refused() {
        assert!(destination(&projects(), "niri_tasks", "niri-tasks").is_err());
        assert!(destination(&projects(), "niri_tasks", "nowhere").is_err());
    }

    #[test]
    fn folders_are_sorted_dirs_without_dotfiles() {
        let dir = std::env::temp_dir().join(format!("niritasks-projects-{}", std::process::id()));
        for d in ["beta", "alpha", ".hidden"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        std::fs::write(dir.join("file"), "").unwrap();
        assert_eq!(folders_in(&dir).unwrap(), vec!["alpha".to_string(), "beta".to_string()]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_projects_folder_is_an_error() {
        assert!(folders_in(std::path::Path::new("/nonexistent/Projects")).is_err());
    }
```

Add to `mod tests` in `src/main.rs`:

```rust
    /// A card's Move to workspace runs this, with the folder picked on the
    /// project list; a folder is required.
    #[test]
    fn moving_a_task_is_a_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "move", "c53b6e3d", "alpha"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "move", "c53b6e3d"]).is_err());
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib project:: 2>&1 | tail -5; cargo test --bin niritasks moving_a_task 2>&1 | tail -5`
Expected: compile errors, `cannot find function destination` / `folders_in`, and the CLI test failing (no `move` subcommand).

- [ ] **Step 3: Write the lib functions**

In `src/project.rs`, change the module doc's first line to `//! Project folders: the `~/Projects` picker's name normalisation, the list a task can move to, and moving it.` and keep the rest. Add `use anyhow::{Context, Result};` under the module doc. After `move_destinations`, add:

```rust
/// `~/Projects` and the folders in it, sorted, dotfiles left out.
///
/// Shared by the project picker and a card's Move to workspace, so the two
/// always offer the same set: a project you can open is a project you can
/// move a task to.
pub fn list() -> Result<(std::path::PathBuf, Vec<String>)> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let dir = std::path::Path::new(&home).join("Projects");
    let names = folders_in(&dir)?;
    Ok((dir, names))
}

/// The folders in `dir`, sorted, dotfiles left out. Split from [`list`] so
/// it is tested on a scratch directory, not the real `~/Projects`.
fn folders_in(dir: &std::path::Path) -> Result<Vec<String>> {
    anyhow::ensure!(dir.is_dir(), "No {} folder found.", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    Ok(names)
}

/// The tag a task on `current_tag` takes moving to `folder`: the folder's
/// own, folded as opening that project folds it. Refused for a folder not
/// among [`move_destinations`] — the one it is already on, or one that is not
/// there — because a task on a tag no workspace produces is invisible to
/// every list.
pub fn destination(names: &[String], current_tag: &str, folder: &str) -> Result<String> {
    anyhow::ensure!(
        move_destinations(names, current_tag).iter().any(|n| n == folder),
        "'{folder}' is not a ~/Projects folder this task can move to."
    );
    let tag = crate::tag::workspace_tag(folder);
    anyhow::ensure!(!tag.is_empty(), "'{folder}' has no usable tag characters.");
    Ok(tag)
}

/// Move `task` off `current_tag` onto `folder`'s workspace, and say which tag
/// it now carries. `names` are the `~/Projects` folders, from [`list`].
/// Refused for a task not on `current_tag`: dropping a tag it lacks would
/// leave it on two workspaces.
pub fn move_task(task: &crate::task::Task, current_tag: &str, folder: &str, names: &[String]) -> Result<String> {
    anyhow::ensure!(
        task.tags.iter().any(|t| t == current_tag),
        "That task is not on +{current_tag}, so it cannot move off it."
    );
    let to = destination(names, current_tag, folder)?;
    crate::task::move_to_tag(&task.uuid, current_tag, &to)?;
    Ok(to)
}
```

- [ ] **Step 4: Add `task move` and use `project::list` in main**

In `src/main.rs`, add to `enum TaskCommand`, right after `Status { … }`:

```rust
    /// Move a task to another project's workspace, as its card's Move to workspace does
    ///
    /// The folder is one of ~/Projects' folders, other than this workspace's;
    /// the task trades this workspace's tag for the folder's. In a pane of a
    /// named herdr session this workspace is the session's, else the focused
    /// one.
    Move {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// The ~/Projects folder to move it to, as named there
        folder: String,
    },
```

In `task_command`, after the `TaskCommand::Status` arm:

```rust
        TaskCommand::Move { uuid, folder } => {
            // From a herdr pane, the session's workspace, as for task add;
            // the panel's spawn has focused its own monitor first.
            let tag = caller_workspace_tag()?;
            let t = task::get(&uuid)?.context("task not found")?;
            let (_, names) = project::list()?;
            let to = project::move_task(&t, &tag, &folder, &names)?;
            notify::tasks(&format!("Moved to +{to}: {}", t.description));
        }
```

Delete `fn projects()` (its doc comment too). In `task_move` change `let (_, names) = projects()?;` to `let (_, names) = project::list()?;`, and in `project_open` change `let (projects_dir, names) = projects()?;` to `let (projects_dir, names) = project::list()?;`. (`task_move` itself goes in Task 2.)

- [ ] **Step 5: Run the unit tests**

Run: `cargo test --lib project:: && cargo test --bin niritasks moving_a_task`
Expected: PASS.

- [ ] **Step 6: Test the retagging against the sandbox**

In `tests/write_path.rs`, change the import to `use niri_tasks::{project, task, task::NoteEdit, text};` and insert before `// ---- tags scope the list`:

```rust
    // ---- move a task to another project's workspace ---------------------
    // What `task move` and a card's Move to workspace run: the task trades
    // this workspace's tag for the folder's, and only for a folder listed.
    task::add(TAG, &text::add_args("move me")).expect("add");
    let mv = task::pending_for_tag(TAG)
        .expect("list")
        .into_iter()
        .find(|t| t.description == "move me")
        .expect("find the task to move");
    let names: Vec<String> = ["sandbox", "other-proj"].iter().map(|s| s.to_string()).collect();
    assert!(project::move_task(&mv, TAG, "nowhere", &names).is_err(), "a folder off the list is refused");
    assert_eq!(project::move_task(&mv, TAG, "other-proj", &names).expect("move"), "other_proj");
    let moved = task::get(&mv.uuid).expect("get").expect("task");
    assert!(moved.tags.iter().any(|t| t == "other_proj"), "on the folder's workspace");
    assert!(!moved.tags.iter().any(|t| t == TAG), "and off this one");
    assert!(
        project::move_task(&moved, TAG, "other-proj", &names).is_err(),
        "a task not on this workspace is refused"
    );
```

Run: `cargo test --test write_path`
Expected: PASS.

- [ ] **Step 7: Document the command**

`cargo test --bin niritasks runs_every_subcommand` fails now: neither README nor llms.txt runs `task move`. In `README.md`'s `## Commands` block, after the `niritasks task status <uuid> deleted --yes` line, add:

```
niritasks task move <uuid> <folder>  # to another ~/Projects folder's workspace, as a card's Move to workspace does
```

In `llms.txt`'s command table, after the `niritasks task status <uuid> deleted --yes` row, add:

```
| `niritasks task move <uuid> <folder>` | Move it to another `~/Projects` folder's workspace: it trades this workspace's tag for the folder's. Refused for a folder not in `~/Projects`, this workspace's own, or a task not on this workspace | scriptable, follows the herdr session, else focus |
```

Run: `cargo test` and `cargo build 2>&1 | grep -c warning`
Expected: all PASS; `0`.

- [ ] **Step 8: Commit**

```bash
git add src/project.rs src/main.rs tests/write_path.rs README.md llms.txt
git commit -m "$(cat <<'EOF'
feat(task): move a task to another project with task move

The project list and the retagging move into the lib, so the task
panel can run the move through the CLI like every other action once
the fuzzel menu goes. A folder off the list, or a task not on this
workspace, is refused.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Retire the picker and the menu

**Files:**
- Delete: `src/rows.rs`, `src/menu.rs`
- Modify: `src/lib.rs` (the `pub mod menu;` and `pub mod rows;` lines)
- Modify: `src/main.rs` (imports, `TaskCommand`, `dispatch` doc, `task_command`, delete `task_list`, `task_menu`, `task_status`, `task_move`; tests)
- Modify: `src/picker.rs` (`clamp_task_width`, docs)
- Modify: `src/task.rs` (`Status::from_label`, comments)
- Modify: `src/daemon.rs` (`Request::Panel` arm, new `NO_TASKS` and `no_cards_text`, tests)
- Modify: `src/panel/state.rs` (`on_press`, `on_clear_all` doc, the body test)
- Modify: `src/panel/actions.rs` (`CARD_KEYS` and its test)
- Modify: `src/panel/surface.rs` (`open_menu` → `run_niritasks`, comments)
- Modify (comments only): `src/actions.rs`, `src/link.rs`, `src/herdr.rs`, `src/work.rs`, `src/refine.rs`, `src/taskbox.rs`, `src/panel/model.rs`, `src/panel/style.rs`, `tests/write_path.rs`, `tests/e2e-box.sh`
- Modify: `README.md` (Commands block), `llms.txt` (command rows)

**Interfaces:**
- Consumes: nothing from Task 1 beyond what is already in `main.rs`.
- Produces:
  - `pub const daemon::NO_TASKS: &str` and `fn daemon::no_cards_text(tag: anyhow::Result<String>) -> String`
  - `PanelState::on_press(uuid, Slot::Body)` returns `Vec::new()`
  - `panel::actions::CARD_KEYS == "Enter: press the button · Ctrl+Del: delete the task"`

- [ ] **Step 1: Write the failing tests**

In `src/panel/state.rs`, replace the test `the_body_gives_the_keyboard_back_and_opens_the_menu` with:

```rust
    /// Enter or a click on a card's body does nothing: its actions are its
    /// buttons. The keyboard and the focus stay where they were.
    #[test]
    fn a_press_on_the_body_does_nothing() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.on_press("a", Slot::Body), Vec::new());
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }
```

In `src/panel/actions.rs`, in `the_hint_leaves_enter_and_ctrl_delete_to_the_footer`, change the last assertion to:

```rust
        assert_eq!(CARD_KEYS, "Enter: press the button · Ctrl+Del: delete the task");
```

In `src/main.rs` tests, add:

```rust
    /// The fuzzel picker and menu are gone: the task panel does what they did.
    #[test]
    fn the_picker_and_the_menu_are_no_commands() {
        assert!(Cli::try_parse_from(["niritasks", "task", "list"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "list", "--dry-run"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "task", "menu", "c53b6e3d"]).is_err());
    }
```

In `src/daemon.rs`'s `mod tests`, add:

```rust
    /// Mod+Alt+Ctrl+T with no card to take the keyboard for: an unnamed
    /// workspace says what the tag refusal says, a named one that it is empty.
    #[test]
    fn no_cards_says_why() {
        assert_eq!(no_cards_text(Ok("web".into())), NO_TASKS);
        assert_eq!(
            no_cards_text(Err(anyhow::anyhow!("This workspace has no name — name it with Mod+Alt+Ctrl+W first."))),
            "This workspace has no name — name it with Mod+Alt+Ctrl+W first."
        );
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test 2>&1 | grep -E "^test .*FAILED|error\[" | head`
Expected: `a_press_on_the_body_does_nothing`, `the_hint_leaves_enter…`, `the_picker_and_the_menu_are_no_commands` fail; daemon tests fail to compile (`no_cards_text` not found).

- [ ] **Step 3: Delete the picker, the menu and their commands**

```bash
git rm src/rows.rs src/menu.rs
```

In `src/lib.rs` delete `pub mod menu;` and `pub mod rows;`.

In `src/main.rs`:
- The import becomes:
  ```rust
  use niri_tasks::{
      actions::Action,
      caller_workspace, caller_workspace_tag, github, ipc, link, niri, notify, picker::Picker, project, refine,
      require_workspace_tag, session,
      speak, task, taskbox, text, work,
  };
  ```
- Delete the `List { … }` and `Menu { … }` variants of `TaskCommand`.
- `Panel`'s doc becomes `/// Slide out the task panel and work its tasks with the keyboard (Mod+Alt+Ctrl+T)`.
- `Status`'s docs become:
  ```rust
      /// Move a task to a state, as a card's Stop, Waiting, Complete and Remove do
      ///
      /// The same states, code and notification as the card's buttons, for
      /// scripts and for finishing a task's worktree. Use it rather than `task
      /// <uuid> done`, `start` or `stop`: `active` also links the herdr pane it
      /// runs in to the task, and every change sends the buttons' notification.
  ```
  and its `yes` field's doc: `/// Confirm `deleted`, which a card's Remove asks about and a script cannot be asked`.
- `dispatch`'s doc becomes `/// Run one parsed command line.`
- In `task_command`, delete the `TaskCommand::List { dry_run } => …` arm and the `TaskCommand::Menu { uuid } => { … }` arm, and replace the `Panel` arm with:
  ```rust
          // The daemon draws the panel. Without one there is no panel to
          // hand the keyboard to, and saying so beats doing nothing.
          TaskCommand::Panel => {
              anyhow::ensure!(
                  delegate_to_daemon(ipc::Request::Panel),
                  "The niri-tasks daemon is not running, so there is no task panel. Start it with `systemctl --user start niri-tasks`."
              );
          }
  ```
- Delete the functions `task_list`, `task_menu`, `task_status` and `task_move`, with their doc comments. Keep `apply_status`; change its doc's first paragraph to `/// Move a task and say so. The one place it is done, so a task completed from a script looks exactly like one completed from its card.` and `findable from the task's menu` to `findable from the task's card`.
- In the tests, `every_task_action_is_a_command_the_cli_accepts`' doc becomes `/// The action row runs a task action as its own `niritasks` words, so each has to be a command the real CLI accepts. Otherwise a typo shows up as a click that does nothing.`; `going_to_a_tasks_session_is_a_command`'s doc `/// A card's Go to session runs this; a script can too.`; `speaking_a_task_is_a_command`'s doc `/// A card's Speak runs this.`

In `src/picker.rs`: delete `clamp_task_width` and the `task_width_is_clamped_both_ends` test; `picker_config`'s doc becomes `/// The stripped-down fuzzel theme the project picker and the workspace name prompt share.`; in `Picker::run`'s doc replace `picker create folders, and why the task list validates the result against its own rows before treating it as a uuid.` with `picker create folders.`

In `src/task.rs`: delete `Status::from_label`; in `a_status_is_the_same_word_in_the_menu_and_on_the_command_line` delete the two `from_label` assertions, rename it `a_status_is_its_label_lower_cased_on_the_command_line`, and make its doc `/// The CLI takes each state's label lower-cased, so `task status <uuid> completed` notifies "Completed".`. Change the comments:
- `:24` `picker and \`task next\`, which sort by urgency,` → `\`task next\`, which sorts by urgency,` (reflow the sentence).
- `:231` `so the picker agrees with the terminal.` → `so the daemon's list agrees with the terminal.`
- `:347` `what the picker's \`¶\` marker and the box's rows are` → `what the box's rows are`.
- `Status`'s doc: `/// Where a task can be moved to: \`niritasks task status\`'s argument, which a card's Back to list, Stop, Waiting, Complete and Remove run.` then keep the Active/Stopped paragraph, ending `From a card they are all just "where is this task now".`
- `ALL`'s doc `/// Every state, in the order \`task status --help\` lists them.`; `label`'s doc `/// The word its notification starts with.`

- [ ] **Step 4: Replace the daemon's fallback**

In `src/daemon.rs`, add above `serve_box_request`:

```rust
/// What Mod+Alt+Ctrl+T says on a workspace with no task to show.
pub const NO_TASKS: &str = "No tasks here — Mod+Alt+T adds one.";

/// What Mod+Alt+Ctrl+T says when there is no card to hand the keyboard to:
/// the tag's own refusal on an unnamed workspace, which says how to name it,
/// else that the workspace has no tasks. `tag` is
/// `crate::require_workspace_tag()`'s answer.
fn no_cards_text(tag: anyhow::Result<String>) -> String {
    match tag {
        Err(e) => e.to_string(),
        Ok(_) => NO_TASKS.to_string(),
    }
}
```

In the `Request::Panel` arm, change the comment inside the closure to `// One \`herdr agent list\` per slide-out, for every card at once; a session that is not running answers at once with none.` and replace the whole `{ // No cards to pick from: … crate::panel::surface::open_menu(…); }` block after `}) ` with:

```rust
            }) {
                notify::tasks(&no_cards_text(crate::require_workspace_tag()));
            }
```

- [ ] **Step 5: Make a body press do nothing**

In `src/panel/state.rs`, `on_press` becomes:

```rust
    /// A press on a task's card: a button, as a click or a key presses it.
    /// The body does nothing: the buttons are the card's actions. Remove
    /// only arms itself the first time; the second press runs it.
    /// Everything that opens something gives the keyboard back first, so the
    /// box or terminal it opens can take it. Back, Waiting and Remove take
    /// the card off the list, so the focus moves to the next card (the one
    /// above, from the last) to still be there when the next tick drops this
    /// one. Speak and Up next keep the card and the focus, so a second press
    /// undoes them.
    pub fn on_press(&mut self, uuid: &str, slot: Slot) -> Vec<Effect> {
        let Slot::Button(action) = slot else { return Vec::new() };
```

(the rest of the body unchanged). In `on_clear_all`'s doc, `so Enter confirms rather than opening a card's menu` → `so Enter confirms rather than pressing a card's button`.

In `src/panel/actions.rs`, `CARD_KEYS` becomes `"Enter: press the button · Ctrl+Del: delete the task"`.

In `src/panel/surface.rs`:
- Rename `pub fn open_menu` to `fn run_niritasks` and update its one caller in `apply` (`Effect::Spawn(args) => run_niritasks(&self.output, &args),`). Its doc: `/// Run a \`niritasks\` command on this monitor: one of a card's buttons. See \`spawn_on\`.`
- `spawn_on`'s doc, from its second paragraph: `/// This monitor is focused first, so the command files under the workspace the panel shows rather than whichever monitor had focus, and a box it opens comes up on the screen that was clicked. It runs as its own process, spawned by niri, so the daemon never waits on it.`
- Module doc: `a row of buttons for the menu's most-used actions under it` → `a row of buttons for the task's actions under it`; `The focused card is darkened. The body opens the whole menu.` → `The focused card is darkened. Enter or a click on its body does nothing.`; `Enter confirms rather than opening the card it was on` → `Enter confirms rather than pressing a button on the card it was on`.
- `card_widget`'s doc: `The body opens the task's whole menu, or shows the rest in place of "+N more".` → `The body does nothing on a task's card, and shows the rest in place of "+N more".`; the comment `// A mouse click opens the menu without leaving the card darkened.` → `// A click leaves the card as it was, not darkened.`

- [ ] **Step 6: Reword the other comments naming the picker or menu**

Run `grep -n -i -E "menu|picker" src tests --include=*.rs -r; grep -n -i menu tests/e2e-box.sh`. Leave `src/github.rs`, `src/project.rs`, `src/picker.rs`'s `Picker`, `prompt_for_name` and `project_open` (the Mod+Alt+W project picker stays), `src/actions.rs` (Task 3 rewrites it) and `src/panel/actions.rs`/`style.rs`'s Note and Grill me lines (Task 3). Reword the rest:

- `src/link.rs:3` `the menu finds a task's` → `the panel finds a task's`; `:27` `from the task's menu` → `from the task's card`; `:64` `the menu and the panel ask` → `the panel asks`; `:81` `it exited since the menu opened` → `it exited since the panel slid out`.
- `src/herdr.rs:4-5` `so this behaves the same from a fuzzel pick as from a terminal inside some other herdr session.` → `so this behaves the same from the task panel as from a terminal inside some other herdr session.`; `:35` `from a menu` → `from the panel`; `:51` `for the menu that opens on every card click.` → `for the panel's Go to session.`; `:62` `so the task's menu` → `so the task's card`.
- `src/work.rs:208` `from its menu` → `from its card`.
- `src/refine.rs:6` `the one the menu was opened on` → `the one the panel showed`; `:14` `Which of the menu's two entries opened Claude.` → `Which of a card's two buttons, Refine or Grill me, opened Claude.`; `:402` `menu's Refine and the panel's button run` → `the panel's Refine button runs`. (`:313` is the project picker's; leave it.)
- `src/taskbox.rs:14` `Add, Edit and the menu's Note all open` → `Add, Edit and \`task note\` all open`; `:121` `which a picker row shows a fraction of` → `which a one-line row shows a fraction of`; `:381` `refining from Edit or Note is the menu's job` → `refining from Edit or Note is the card's Refine's job`; `:799` `whichever menu row opened it` → `whichever command opened it`. Leave `:3-5` and `:20`: the project picker and the workspace name prompt are still fuzzel, so "every other prompt in this tool stays fuzzel" still holds.
- `src/panel/model.rs:1-5` → `//! What the task panel shows, as plain data, split from the drawing: the ordering, icons and cap are the parts worth testing, and none of them needs a compositor.`; `:218` drop `; the picker keeps the urgency order`.
- `tests/write_path.rs:432` `what the menu asks before it offers Start` → `what the panel asks before it offers Start`; `:470` `the menu's states` → `task status's states`; `:542` `lifts it in the picker` → `lifts it in \`task next\``.
- `tests/e2e-box.sh:23` `the menu's Note` → `\`task note\``.

- [ ] **Step 7: Drop the commands from the docs**

In `README.md`'s `## Commands` block delete the `task list`, `task list --dry-run` and `task menu <uuid>` lines, and change the two `task status` comments to `# what a card's Stop, Waiting, Complete and Remove run: active|stopped|waiting|completed|deleted` and `# deleted needs --yes, Remove's confirmation`.

In `llms.txt`'s table delete the `task list`, `task list --dry-run` and `task menu <uuid>` rows, and make the `task panel` row:

```
| `niritasks task panel` | Hand the task panel the keyboard (Mod+Alt+Ctrl+T); a notification instead on a workspace with no tasks or no name, and exit 1 with no daemon | GUI |
```

(Rule 1 and the rest of the prose are Task 5's.)

- [ ] **Step 8: Run everything**

Run: `cargo test && cargo build 2>&1 | grep -c warning && grep -rn "rows::\|menu::\|open_menu\|clamp_task_width\|from_label\|task_list\|task_menu" src tests`
Expected: all PASS; `0`; the grep prints nothing.

- [ ] **Step 9: Commit**

```bash
git add -A src tests README.md llms.txt
git commit -m "$(cat <<'EOF'
refactor: retire the fuzzel task picker and action menu

task list and task menu go, with rows.rs, menu.rs and the menu's
status and move pickers. Mod+Alt+Ctrl+T with no card now says why in
a notification, task panel without the daemon says it is not running,
and a press on a card's body does nothing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Grill me and Complete on the action row; Note goes

**Files:**
- Modify: `src/actions.rs` (enum, `ALL`, `applies`, `label`, `icon`, `args`, module doc, tests)
- Modify: `src/panel/actions.rs` (`ROW`, `row` doc, `keeps_keyboard`, `leaves_the_list`, `class`, `letter`, tests)
- Modify: `src/panel/style.rs` (`COMPLETE`, docs, `colour`)
- Modify: `src/panel/keys.rs` (letter tests)
- Modify: `src/panel/surface.rs` (`sync`: blank hint while Remove is armed; module doc's key list)

**Interfaces:**
- Consumes: Task 2's menu-free `actions.rs` users (nothing else matches on `Action::Note` once `menu.rs` is gone; `style::colour` and `class` do, and change here).
- Produces:
  - `Action::Complete` — label `"Complete"`, icon `"\u{f00c}"`, args `["task","status",uuid,"completed"]`, class `"complete"`, letter `'c'`
  - `Action::Grill` on the row — class `"grill"` (exists), letter `'i'`
  - `Action::ROW == Action::ALL`

- [ ] **Step 1: Write the failing tests**

In `src/actions.rs` tests:
- `a_task_on_the_list_gets_all_but_session_back_and_stop` expects `vec![Start, Refine, Grill, Edit, Speak, UpNext, Complete, Wait, Remove]`.
- `an_active_task_can_be_stopped_and_started_again` expects `vec![Start, Refine, Grill, Edit, Speak, UpNext, Complete, Stop, Wait, Remove]`.
- `a_waiting_task_gets_back_to_list_and_what_still_works_on_it` expects `vec![Back, Edit, Speak, Complete, Remove]`.
- `labels_read_as_the_menu_does` is renamed `labels_read_as_the_tooltips_do` and expects `"Go to session", "Back to list", "Start working", "Refine", "Grill me", "Edit", "Speak", "Up next", "Complete", "Stop", "Waiting", "Remove"`.
- `no_two_actions_share_a_label`'s doc becomes `/// Each is its button's tooltip, so no two may share one, either way up next reads.`
- In `each_action_runs_its_command`, delete the `Note` line and add `assert_eq!(Complete.args(u), vec!["task", "status", u, "completed"]);`.

In `src/panel/actions.rs` tests:
- `an_active_task_gets_stop_in_place_of_start`: `vec!["refine", "grill", "edit", "speak", "up-next", "complete", "stop", "wait", "remove"]`.
- `a_task_not_yet_active_gets_start_and_no_stop`: `vec![Start, Refine, Grill, Edit, Speak, UpNext, Complete, Wait, Remove]`.
- `a_task_with_a_live_claude_gets_go_to_session_first`: `vec![Session, Refine, Grill, Edit, Speak, UpNext, Complete, Stop, Wait, Remove]` and `vec![Session, Start, Refine, Grill, Edit, Speak, UpNext, Complete, Wait, Remove]`.
- `a_waiting_task_gets_back_to_list_edit_speak_and_remove` → rename `a_waiting_task_gets_back_to_list_and_what_still_works_on_it`, expecting `vec![Back, Edit, Speak, Complete, Remove]`.
- Replace `the_row_leaves_note_and_grill_me_to_the_menu` with:
  ```rust
      /// Grill me goes where Refine goes: every card on the list.
      #[test]
      fn every_card_but_a_waiting_one_gets_grill_me() {
          for state in every_state() {
              assert_eq!(Action::row(state).contains(&Grill), !state.waiting, "{state:?}");
          }
      }

      /// Every card can be completed, a waiting one too, as the menu's Update
      /// status offered.
      #[test]
      fn every_card_gets_complete() {
          for state in every_state() {
              assert!(Action::row(state).contains(&Complete), "{state:?}");
          }
      }
  ```
- `back_speak_up_next_wait_and_remove_keep_the_list_open` → rename `the_buttons_that_open_nothing_keep_the_list_open`, expecting `vec![Back, Speak, UpNext, Complete, Wait, Remove]`.
- `only_back_wait_and_remove_take_their_card_off_the_list` → rename `back_complete_wait_and_remove_take_their_card_off_the_list`, expecting `vec![Back, Complete, Wait, Remove]`.
- `letters_press_their_own_buttons`: doc `/// g b s r i e c t press Go to session, Back to list, Start working, Refine, Grill me, Edit, Complete and Stop, and each letter finds its own button.`, expecting `vec![('g', Session), ('b', Back), ('s', Start), ('r', Refine), ('i', Grill), ('e', Edit), ('c', Complete), ('t', Stop)]`.
- Add to `a_lettered_button_hints_its_letter_and_name`:
  ```rust
          assert_eq!(hint(on_list(false), Some(Grill)), "i: Grill me · Ctrl+Enter: refine");
          assert_eq!(hint(on_list(true), Some(Complete)), "c: Complete · Ctrl+Enter: start working");
  ```
- Replace `HINT_ROOM` and `every_hint_fits_beside_the_widest_row` with:
  ```rust
      /// Room for the hint beside a row of `buttons`, in characters at the
      /// hint's 9pt, where Iosevka Term Extended is 7px a character: the 760px
      /// card, less each button's one 8px glyph and 24px of padding, a 1px
      /// line between each two, and the hint's own 24px of padding.
      fn hint_room(buttons: usize) -> usize {
          (760 - buttons * 32 - (buttons - 1) - 24) / 7
      }

      /// The most buttons any card gets.
      fn widest_row() -> usize {
          every_state().into_iter().map(|s| Action::row(s).len()).max().unwrap()
      }

      #[test]
      fn every_hint_fits_beside_its_row() {
          for (state, focused, text) in every_hint() {
              let room = hint_room(Action::row(state).len());
              assert!(text.chars().count() <= room, "{state:?} {focused:?}: {text:?} is {} long, room {room}", text.chars().count());
          }
      }

      /// Armed, Remove reads "  Confirm remove" (16 characters at the
      /// buttons' 10pt, 128px), and the surface leaves the hint blank: the
      /// button says what Enter does. The widest row still fits the card.
      #[test]
      fn confirm_remove_fits_on_the_widest_row() {
          let n = widest_row();
          assert!(n * 32 + (n - 1) + 128 <= 760, "{n} buttons");
      }
  ```

In `src/panel/keys.rs`'s `letters_and_delete_run_their_buttons` add `gdk::Key::i` → `Grill`, `gdk::Key::c` → `Complete`, and in the Caps Lock test after it `gdk::Key::I` and `gdk::Key::C` the same.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: compile errors: no variant `Complete`.

- [ ] **Step 3: Change the actions**

In `src/actions.rs`:
- Module doc becomes:
  ```rust
  //! The task actions: everything that can be done to one task, each with its
  //! words, its icon and the `niritasks` command it runs, and which of them a
  //! task gets in each state.
  //!
  //! The task panel's action row (`panel/actions.rs`) is the view of them:
  //! which it shows, in what order, and their keys. A new action is a variant
  //! here and a place on the row.
  ```
- `Action`'s doc: `/// One thing that can be done to a task. Each is one \`niritasks\` command, which is what its button runs, so a script can do anything a click can.`
- `Back`'s and `Stop`'s docs: `Update status → Stopped` → `\`task status <uuid> stopped\``.
- Delete `Note` and its doc. `Edit`'s doc: `/// The task box on the description and every note. The box, not a one-line prompt: the descriptions you reach for it to fix are the long ones.`
- After `UpNext`, add:
  ```rust
      /// Mark it done: it leaves the list, as `task status <uuid> completed`.
      Complete,
  ```
- `ALL`: `pub const ALL: [Action; 12] = [Session, Back, Start, Refine, Grill, Edit, Speak, UpNext, Complete, Stop, Wait, Remove];`
- `applies`: `Edit | Speak | Complete | Remove => true,` (Note gone).
- `label`'s doc: `/// The action's words, on its button's tooltip and in the card's hint. …` (keep the Up next sentence); arms: delete `Note`, add `Complete => "Complete",` after the UpNext arms.
- `icon`'s doc: list `terminal, undo arrow, play, magic wand, comments, pencil, bookmark, check, stop, pause and trash can`; arms: delete `Note`, add `// Font Awesome's check: done.` and `Complete => "\u{f00c}",` after `UpNext`.
- `args`' doc: `/// The \`niritasks\` arguments the action runs, without the program: what its button spawns. Remove carries \`--yes\`, because its button asks first.`; arms: delete `Note`, add `Complete => &["task", "status", uuid, "completed"],`.

In `src/panel/actions.rs`:
- `ROW`:
  ```rust
      /// The row's buttons, left to right: every task action, the row being
      /// the only view of them.
      pub const ROW: [Action; 12] = Self::ALL;
  ```
- `CONFIRM_REMOVE`'s doc: `/// What Remove reads between its first press and its second.`
- `row`'s doc: `/// The buttons a card in \`state\` gets, left to right: the row's actions that apply. An active task gets Stop in the place of Start working, which would only go back to its worktree; Go to session goes back to its Claude.`
- `keeps_keyboard`: `matches!(self, Back | Speak | UpNext | Complete | Wait | Remove)`, doc `Back to list, Up next, Complete, Waiting and Remove only change the task, …` and `the rest open a box, a terminal or a herdr tab, which takes it.`
- `leaves_the_list`: `matches!(self, Back | Complete | Wait | Remove)`.
- `class`: delete `Note`, add `Complete => "complete",`.
- `letter`: add `Grill => Some('i'),` and `Complete => Some('c'),`; the `None` arm becomes `Speak | UpNext | Wait | Remove => None,`.

In `src/panel/style.rs`, after `SESSION`:

```rust
/// Complete green: mocha's own `green`, paler than [`ACTIVE`]'s, for the step
/// after the work rather than the work.
pub const COMPLETE: &str = "#a6e3a1";
```

In the doc on `REFINE` add `Complete green` to the list of colours. `colour`'s doc: `/// Each task action's colour. Exhaustive, so a new action cannot be a grey button. Grill me wears Refine's mauve, being a refine that interviews first.` Its arms: `Action::Edit => EDIT,` and add `Action::Complete => COMPLETE,`.

- [ ] **Step 4: Blank the hint while Remove is armed**

In `src/panel/surface.rs`'s `sync`, the `let hint = match &focus {` gains a first arm:

```rust
                // Armed, Remove reads Confirm remove, which says what Enter
                // does; on the widest row the hint would not fit beside it.
                _ if removing => String::new(),
```

In the module doc, `and g, b, s, r, e, t and Delete press its Go to session, Back to list, Start, Refine, Edit, Stop and Remove` → `and g, b, s, r, i, e, c, t and Delete press its Go to session, Back to list, Start, Refine, Grill me, Edit, Complete, Stop and Remove`.

- [ ] **Step 5: Run everything**

Run: `cargo test && cargo build 2>&1 | grep -c warning`
Expected: all PASS (main's `every_task_action_is_a_command_the_cli_accepts` covers Complete's args); `0`.

- [ ] **Step 6: Commit**

```bash
git add src/actions.rs src/panel/actions.rs src/panel/style.rs src/panel/keys.rs src/panel/surface.rs
git commit -m "$(cat <<'EOF'
feat(panel): add Grill me and Complete to the action row

With the menu gone the row is the only view of the task actions, so
Grill me (i) joins it and Complete (c) marks a task done, keeping the
list up. Note leaves the actions, nothing showing it now. The hint
blanks while Remove is armed, as the longer row has no room for both.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Move to workspace and its project list

**Files:**
- Modify: `src/actions.rs` (`Move`)
- Modify: `src/panel/actions.rs` (`ROW` length, `keeps_keyboard`, `class`, `letter`, `MOVE_KEYS`, `FOLDER_ICON`, tests)
- Modify: `src/panel/style.rs` (`MOVE`, `colour`)
- Modify: `src/panel/keys.rs` (letter tests)
- Modify: `src/panel/state.rs` (`Moving`, `NO_DESTINATIONS`, effects, the move mode, tests)
- Modify: `src/panel/surface.rs` (draw the list, answer the effects, follow its focus)
- Modify: `src/main.rs` (the every-action test)

**Interfaces:**
- Consumes: `project::list()` and `project::move_destinations(&[String], &str) -> Vec<String>` (Task 1); `task move <uuid> <folder>` (Task 1).
- Produces:
  - `Action::Move` — label `"Move to workspace"`, icon `"\u{f07c}"`, args `["task","move",uuid]` (the project list adds the folder), class `"move"`, letter `'m'`
  - `pub struct state::Moving { pub uuid: String, pub text: String, pub folders: Vec<String>, pub at: usize }`
  - `Effect::ListProjects(String)`, `Effect::FocusFolder(usize)`
  - `PanelState::moving(&self) -> Option<&Moving>`, `PanelState::show_projects(&mut self, uuid: &str, folders: Vec<String>) -> Vec<Effect>`, `PanelState::on_folder(&mut self, at: usize) -> Vec<Effect>`
  - `pub const state::NO_DESTINATIONS: &str = "No other project to move this to."`
  - `pub const panel::actions::MOVE_KEYS: &str`, `pub const panel::actions::FOLDER_ICON: &str = "\u{f07b}"`

- [ ] **Step 1: Write the failing action tests**

In `src/actions.rs` tests, add `Move` after `Complete` in the four expected lists (`…, Complete, Move, Wait, Remove`, `…, Complete, Move, Stop, Wait, Remove`, `vec![Back, Edit, Speak, Complete, Move, Remove]`), add `"Move to workspace"` after `"Complete"` in the labels list, and in `each_action_runs_its_command`:

```rust
        // The project list adds the folder.
        assert_eq!(Move.args(u), vec!["task", "move", u]);
```

In `src/panel/actions.rs` tests: add `"move"` after `"complete"` and `Move` after `Complete` in the five row expectations (the waiting one becomes `vec![Back, Edit, Speak, Complete, Move, Remove]`); rename `every_card_gets_complete` to `every_card_gets_complete_and_move` and assert both; `the_buttons_that_open_nothing_keep_the_list_open` expects `vec![Back, Speak, UpNext, Complete, Move, Wait, Remove]` with doc `/// Move opens the project list in the panel itself, so it keeps the keyboard with the buttons that only change the task.`; letters expect `…, ('c', Complete), ('m', Move), ('t', Stop)]` and the doc gains `m`/Move to workspace; add to `a_lettered_button_hints_its_letter_and_name`:

```rust
        assert_eq!(hint(on_list(true), Some(Move)), "m: Move to workspace · Ctrl+Enter: start working");
```

In `src/panel/keys.rs` add `gdk::Key::m` and `gdk::Key::M` → `Move`.

In `src/main.rs`, `every_task_action_is_a_command_the_cli_accepts` becomes:

```rust
    #[test]
    fn every_task_action_is_a_command_the_cli_accepts() {
        for action in Action::ALL {
            let mut argv = vec!["niritasks".to_string()];
            argv.extend(action.args("c53b6e3d"));
            // Move's words stop short of the folder the project list picks.
            if action == Action::Move {
                argv.push("alpha".to_string());
            }
            if let Err(e) = Cli::try_parse_from(&argv) {
                panic!("{} runs {argv:?}, which the CLI rejects: {e}", action.label(false));
            }
        }
    }
```

- [ ] **Step 2: Write the failing state tests**

In `src/panel/state.rs` tests, add a section after `// ─── the Ideas tab` tests:

```rust
    // ─── Move to workspace ───────────────────────────────────────────────

    fn folders(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    /// The panel on `a`'s project list, of folders x and y.
    fn moving_a() -> PanelState {
        let mut state = keyboard(pending(&["a", "b"]));
        assert_eq!(key(&mut state, KeyAction::Run(Action::Move)), vec![Effect::ListProjects("a".into())]);
        assert_eq!(state.show_projects("a", folders(&["x", "y"])), vec![Effect::Render]);
        state
    }

    /// m asks for the folders, keeping the keyboard and the focus; they come
    /// back as the project list, on the first.
    #[test]
    fn move_shows_the_project_list_in_place_of_the_cards() {
        let state = moving_a();
        assert!(state.keyboard());
        assert_eq!(
            state.moving(),
            Some(&Moving { uuid: "a".into(), text: "a".into(), folders: folders(&["x", "y"]), at: 0 })
        );
        assert!(state.visible().is_empty(), "no task cards");
        assert!(state.tabs().is_empty(), "no tabs");
        assert_eq!(state.empty_text(), None);
        assert!(!state.shows_clear_all());
        assert!(!state.hidden());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref(), "kept for Escape");
    }

    #[test]
    fn no_other_project_says_so_and_stays_on_the_cards() {
        let mut state = keyboard(pending(&["a"]));
        assert_eq!(state.show_projects("a", Vec::new()), vec![Effect::Notify(NO_DESTINATIONS.into())]);
        assert_eq!(state.moving(), None);
    }

    #[test]
    fn up_and_down_walk_the_folders_and_stop_at_the_ends() {
        let mut state = moving_a();
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::FocusFolder(1)]);
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::FocusFolder(1)]);
        assert_eq!(key(&mut state, KeyAction::PrevCard), vec![Effect::FocusFolder(0)]);
        assert_eq!(key(&mut state, KeyAction::PrevCard), vec![Effect::FocusFolder(0)]);
    }

    /// Enter moves the task to the folder the keyboard is on, back on the
    /// cards with the focus on the next one, as the task leaves this list.
    #[test]
    fn enter_moves_the_task_to_the_focused_folder() {
        let mut state = moving_a();
        key(&mut state, KeyAction::NextCard);
        assert_eq!(
            key(&mut state, KeyAction::Enter),
            vec![Effect::Render, Effect::Spawn(vec!["task".into(), "move".into(), "a".into(), "y".into()])],
        );
        assert_eq!(state.moving(), None);
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
    }

    #[test]
    fn a_click_on_a_folder_moves_the_task_there() {
        let mut state = moving_a();
        assert_eq!(
            state.on_folder(0),
            vec![Effect::Render, Effect::Spawn(vec!["task".into(), "move".into(), "a".into(), "x".into()])],
        );
        assert_eq!(state.on_folder(0), Vec::new(), "once the list has gone");
    }

    /// Escape goes back to the cards on the same card and slot; a second
    /// gives the keyboard back.
    #[test]
    fn escape_goes_back_to_the_same_card() {
        let mut state = moving_a();
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render]);
        assert_eq!(state.moving(), None);
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
    }

    /// Every other key is the list's and does nothing: no card is there to act on.
    #[test]
    fn the_project_list_takes_no_other_key() {
        let mut state = moving_a();
        for k in [
            KeyAction::Run(Action::Edit),
            KeyAction::Run(Action::Remove),
            KeyAction::PrevSlot,
            KeyAction::NextSlot,
            KeyAction::Filter(Filter::All),
            KeyAction::Ideas,
            KeyAction::PrevFilter,
            KeyAction::NextFilter,
            KeyAction::Advance,
            KeyAction::Delete,
            KeyAction::ClearAll,
            KeyAction::Ignore,
        ] {
            assert_eq!(state.on_key(k), Some(Vec::new()), "{k:?}");
        }
        assert!(state.moving().is_some());
        assert_eq!(state.on_tab(Tab::Ideas), Vec::new());
    }

    /// GTK's focus on a folder card is no card's: the task's focus is kept
    /// for Escape.
    #[test]
    fn focus_moves_on_the_project_list_keep_the_tasks_focus() {
        let mut state = moving_a();
        state.on_focus(None);
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    #[test]
    fn a_refresh_keeps_the_list_while_its_task_is_there() {
        let mut state = moving_a();
        key(&mut state, KeyAction::NextCard);
        assert_eq!(state.set_cards(&pending(&["a", "b", "c"])), vec![Effect::Render]);
        assert_eq!(state.moving().map(|m| m.at), Some(1));
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }

    /// The task done or moved elsewhere meanwhile: the list goes with it.
    #[test]
    fn a_refresh_without_its_task_closes_the_list() {
        let mut state = moving_a();
        assert_eq!(state.set_cards(&pending(&["b"])), vec![Effect::Render]);
        assert_eq!(state.moving(), None);
        assert_eq!(state.focus(), focused("b", Slot::Body).as_ref());
    }

    #[test]
    fn taking_the_keyboard_again_closes_the_list() {
        let mut state = moving_a();
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.moving(), None);
    }

    #[test]
    fn no_project_list_without_the_keyboard() {
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a"]));
        assert_eq!(state.show_projects("a", folders(&["x"])), Vec::new());
        assert_eq!(state.moving(), None);
    }
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --lib 2>&1 | tail -5`
Expected: compile errors: no variant `Move`, no `Moving`, `ListProjects`, `show_projects`.

- [ ] **Step 4: Add the action**

In `src/actions.rs`, after `Complete`:

```rust
    /// Move it to another `~/Projects` folder's workspace, picked on the
    /// panel's project list.
    Move,
```

`ALL` becomes `[Action; 13]` with `Move` after `Complete`; `applies`: `Edit | Speak | Complete | Move | Remove => true,`; `label`: `Move => "Move to workspace",`; `icon` (doc list gains `open folder`): `// Font Awesome's open folder: off to another project.` `Move => "\u{f07c}",`; `args`: `Move => &["task", "move", uuid],` and the doc gains `Move's stop short of the folder, which the project list adds.`

In `src/panel/actions.rs`: `ROW: [Action; 13]`; `keeps_keyboard` adds `Move` and its doc `Move opens the project list in the panel itself`; `class`: `Move => "move",`; `letter`: `Move => Some('m'),`. After `CARD_KEYS` add:

```rust
/// The footer while the project list shows: the keys it takes, which are
/// all it takes.
pub const MOVE_KEYS: &str = "Up, Down: pick a folder · Enter: move the task there · Esc: back";

/// A folder card's icon on the project list: Font Awesome's folder, Move to
/// workspace's open one shut.
pub const FOLDER_ICON: &str = "\u{f07b}";
```

In `src/panel/style.rs` after `COMPLETE`:

```rust
/// Move to workspace sapphire: mocha's `sapphire`, between Go to session's
/// blue and Waiting's teal, apart from both.
pub const MOVE: &str = "#74c7ec";
```

and `Action::Move => MOVE,` in `colour` (add `Move sapphire` to `REFINE`'s doc list).

- [ ] **Step 5: Add the move mode to the state**

In `src/panel/state.rs`, after `struct Shown`'s `impl`:

```rust
/// What the panel says when Move to workspace finds no folder to move to.
pub const NO_DESTINATIONS: &str = "No other project to move this to.";

/// The project list Move to workspace swaps in for the cards: the task being
/// moved, and the `~/Projects` folders it can go to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moving {
    /// The task being moved, by uuid.
    pub uuid: String,
    /// Its description, for the line over the folders.
    pub text: String,
    /// The folders, as `project::move_destinations` gives them.
    pub folders: Vec<String>,
    /// The folder the keyboard is on.
    pub at: usize,
}
```

Add to `enum Effect`:

```rust
    /// Read the `~/Projects` folders this task can move to and hand them to
    /// [`PanelState::show_projects`]: Move to workspace's first step, which
    /// the state cannot take, having no disk to read.
    ListProjects(String),
    /// Put the focus on the project list's folder card at this index.
    FocusFolder(usize),
```

Add a field to `PanelState`, after `armed`:

```rust
    /// The project list is up in place of the cards: Move to workspace was
    /// pressed. Only ever with the keyboard.
    moving: Option<Moving>,
```

and the accessor after `armed()`:

```rust
    pub fn moving(&self) -> Option<&Moving> {
        self.moving.as_ref()
    }
```

Change the existing methods:
- `tabs`: `if self.keyboard && self.moving.is_none() {`; doc gains `None on the project list either.`
- `shows_clear_all`: `self.keyboard && !self.ideas && self.moving.is_none() && self.filter == Filter::Waiting`.
- `visible`: the first check becomes `if self.ideas || self.moving.is_some() {`; doc gains `nor on the project list, which draws folders.`
- `empty_text`: `if self.ideas || self.moving.is_some() {`.
- `set_cards`: after `self.all = cards.to_vec();` add
  ```rust
          // The task being moved has gone (done elsewhere, say): its project
          // list goes with it.
          if self.moving.as_ref().is_some_and(|m| !self.all.iter().any(|c| c.uuid.as_deref() == Some(m.uuid.as_str()))) {
              self.moving = None;
          }
  ```
- `take_keyboard` and `release`: add `self.moving = None;` beside `self.ideas = false;`.
- `on_focus`: `if !self.keyboard || self.moving.is_some() || self.focus == focus {`; doc gains `On the project list too: a folder card is no task's, and the task's focus is kept for Escape.`
- `on_key`: after the `if self.ideas { … }` block add
  ```rust
          if self.moving.is_some() {
              // The project list's: Up and Down walk it, Enter moves the task,
              // Escape goes back to the cards. No card is there for the rest.
              return Some(match key {
                  KeyAction::Release => self.leave_projects(),
                  KeyAction::PrevCard | KeyAction::NextCard => self.move_folder(key == KeyAction::NextCard),
                  KeyAction::Enter => {
                      let at = self.moving.as_ref().map_or(0, |m| m.at);
                      self.on_folder(at)
                  }
                  _ => Vec::new(),
              });
          }
  ```
  and its doc gains `On the project list every key is the panel's: Up and Down, Enter and Escape act, and the rest do nothing.`
- `on_press`: first line `if self.moving.is_some() { return Vec::new(); }`, then after `let Slot::Button(action) = slot else …`:
  ```rust
          // The folders are on disk, which the surface reads: it answers
          // with show_projects. The keyboard and the focus stay put.
          if action == Action::Move {
              return vec![Effect::ListProjects(uuid.into())];
          }
  ```
  and the doc gains `Move to workspace asks for the project list.`
- `pick`: `if !self.keyboard || self.moving.is_some() || self.tab() == tab {`.
- `rerender`: after `self.armed = Armed::None;` add
  ```rust
          // The project list keeps the task's focus for Escape to go back to.
          if self.moving.is_some() {
              return vec![Effect::Render];
          }
  ```

Add the new methods after `on_clear_all`:

```rust
    /// The folders Move to workspace asked for: the project list in place of
    /// the cards, the keyboard on the first. With none, a notification and
    /// the cards stay. Nothing without the keyboard, or once the task has
    /// left the cards.
    pub fn show_projects(&mut self, uuid: &str, folders: Vec<String>) -> Vec<Effect> {
        if !self.keyboard {
            return Vec::new();
        }
        let Some(text) = self.all.iter().find(|c| c.uuid.as_deref() == Some(uuid)).map(|c| c.text.clone()) else {
            return Vec::new();
        };
        if folders.is_empty() {
            return vec![Effect::Notify(NO_DESTINATIONS.into())];
        }
        self.armed = Armed::None;
        self.moving = Some(Moving { uuid: uuid.into(), text, folders, at: 0 });
        vec![Effect::Render]
    }

    /// A folder on the project list, by Enter or a click: move the task
    /// there with `task move`, back on the cards with the focus on the next
    /// one, since the task leaves this workspace. The keyboard stays.
    pub fn on_folder(&mut self, at: usize) -> Vec<Effect> {
        let Some(moving) = self.moving.take() else { return Vec::new() };
        let Some(folder) = moving.folders.get(at).cloned() else {
            self.moving = Some(moving);
            return Vec::new();
        };
        if let Some(next) = self.neighbour(&moving.uuid) {
            self.focus = Some(next);
        }
        let mut effects = self.rerender();
        let mut args = Action::Move.args(&moving.uuid);
        args.push(folder);
        effects.push(Effect::Spawn(args));
        effects
    }
```

and after `leave_ideas`:

```rust
    /// Escape on the project list: back to the cards, on the card and slot
    /// the focus was on.
    fn leave_projects(&mut self) -> Vec<Effect> {
        self.moving = None;
        self.rerender()
    }

    /// Up and Down on the project list, stopping at the ends.
    fn move_folder(&mut self, forward: bool) -> Vec<Effect> {
        let Some(m) = self.moving.as_mut() else { return Vec::new() };
        m.at = keys::step(m.at, m.folders.len(), forward);
        vec![Effect::FocusFolder(m.at)]
    }
```

Update the module doc's first paragraph: `The cards, the filter tab, "+N more" opened or not, which card and button has the keyboard's focus, what a first press has armed, and the project list Move to workspace swaps in all live here, …`.

- [ ] **Step 6: Run the state and action tests**

Run: `cargo test --lib panel:: actions:: && cargo test --bin niritasks every_task_action`
Expected: PASS. (`surface.rs` does not compile yet if it matches `Effect` exhaustively: do Step 7 first if `cargo test` stops on it, then rerun.)

- [ ] **Step 7: Draw the project list**

In `src/panel/surface.rs`:

- Import `Moving` is not needed; the render takes `state.moving().cloned()`.
- Add a field to `Panel`, after `cards`:
  ```rust
      /// The project list's folder cards, top to bottom, while a task is
      /// being moved: each card's box, to scroll to, and its body, to focus.
      folders: RefCell<Vec<(gtk4::Box, gtk4::Button)>>,
  ```
  and `folders: RefCell::new(Vec::new()),` in `Panel::new`.
- In `apply`, add:
  ```rust
                  Effect::ListProjects(uuid) => {
                      // Read on every press, so a folder made since shows.
                      let folders = crate::project::list()
                          .map(|(_, names)| crate::project::move_destinations(&names, &self.tag.borrow()));
                      let effects = match folders {
                          Ok(folders) => self.state.borrow_mut().show_projects(&uuid, folders),
                          Err(e) => vec![Effect::Notify(e.to_string())],
                      };
                      self.apply(effects);
                  }
                  Effect::FocusFolder(at) => self.focus_folder(at),
  ```
- In `render`, take the list with the rest:
  ```rust
          let (shown, keyboard, empty, focus, ideas, moving) = {
              let state = self.state.borrow();
              (
                  state.visible(),
                  state.keyboard(),
                  state.empty_text(),
                  state.focus().cloned(),
                  state.on_ideas(),
                  state.moving().cloned(),
              )
          };
  ```
  Inside the `while_drawing` closure, after `*self.cards.borrow_mut() = cards;`:
  ```rust
              let mut folders = Vec::new();
              if let Some(m) = &moving {
                  // Which task is moving, over the folders.
                  self.column.append(&empty_line(&format!("{}: {}", Action::Move.label(false), m.text)));
                  for (at, folder) in m.folders.iter().enumerate() {
                      let card = self.folder_card(at, folder);
                      self.column.append(&card.0);
                      folders.push(card);
                  }
              }
              *self.folders.borrow_mut() = folders;
  ```
  After the closure, replace `self.tabs.set_visible(keyboard);` and the footer lines with:
  ```rust
          // The project list has no tabs: they filter task cards.
          self.tabs.set_visible(keyboard && moving.is_none());
          // The footer names the cards' keys, so not on Ideas, where the
          // text area takes Enter and Delete as typing; on the project list,
          // its own.
          self.footer.set_visible(keyboard && !ideas);
          self.footer.set_label(if moving.is_some() { actions::MOVE_KEYS } else { actions::CARD_KEYS });
  ```
  and the focus at the end:
  ```rust
          if ideas {
              self.notepad.focus();
          } else if let Some(m) = &moving {
              self.focus_folder(m.at);
          } else if keyboard {
              self.focus_on(focus.as_ref());
          }
  ```
- In `fit`, `let tabs_h = if keyboard {` → `let tabs_h = if self.tabs.is_visible() {` and delete the now-unused `let keyboard = …` line if nothing else in `fit` reads it.
- `empty_line` gains wrapping, for a long description over the folders: after `line.set_xalign(0.0);` add `line.set_wrap(true);` and `line.set_wrap_mode(gtk4::pango::WrapMode::WordChar);`, and its doc gains `Also the line naming the task over the project list.`
- After `action_row`, add:
  ```rust
      /// One folder on the project list, drawn as a task card is: a box
      /// holding a body button with the folder's icon and name. Enter or a
      /// click on it moves the task there.
      fn folder_card(self: &Rc<Self>, at: usize, folder: &str) -> (gtk4::Box, gtk4::Button) {
          let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
          root.add_css_class("task-card");
          root.set_size_request(CARD_WIDTH_PX, -1);
          root.set_overflow(gtk4::Overflow::Hidden);
          // The two spaces are the gap a task card leaves after its icon.
          let label = gtk4::Label::new(Some(&format!("{}  {folder}", actions::FOLDER_ICON)));
          label.set_xalign(0.0);
          let body = gtk4::Button::builder().child(&label).build();
          body.add_css_class("card-body");
          body.set_focus_on_click(false);
          let weak = Rc::downgrade(self);
          body.connect_clicked(move |_| {
              if let Some(p) = weak.upgrade() {
                  let effects = p.state.borrow_mut().on_folder(at);
                  p.apply(effects);
              }
          });
          root.append(&body);
          (root, body)
      }

      /// GTK's focus on the project list's folder card `at`; connect_focus
      /// then scrolls it into view. The button is cloned out first: grabbing
      /// the focus runs connect_focus there and then.
      fn focus_folder(&self, at: usize) {
          let body = self.folders.borrow().get(at).map(|(_, body)| body.clone());
          if let Some(body) = body {
              body.grab_focus();
          }
      }
  ```
- In `follow_focus`, find the card to scroll to from the project list too. Replace
  ```rust
              let Some(uuid) = p.state.borrow().focus().map(|f| f.uuid.clone()) else { return };
              let root = p.cards.borrow().iter().find(|c| c.uuid.as_ref() == Some(&uuid)).map(|c| c.root.clone());
  ```
  with
  ```rust
              let folder = p.state.borrow().moving().map(|m| m.at);
              let root = match folder {
                  Some(at) => p.folders.borrow().get(at).map(|(root, _)| root.clone()),
                  None => {
                      let Some(uuid) = p.state.borrow().focus().map(|f| f.uuid.clone()) else { return };
                      p.cards.borrow().iter().find(|c| c.uuid.as_ref() == Some(&uuid)).map(|c| c.root.clone())
                  }
              };
  ```
- Module doc: after the paragraph on Ctrl+Enter and Ctrl+Delete's keys add `Move to workspace (m) swaps the cards for the project list: the \`~/Projects\` folders the task can move to, drawn as cards under a line naming it, with no tabs and a footer of its own. Up and Down walk it, Enter or a click runs \`task move\` and goes back to the cards on the next one, and Escape goes back to the cards on the same one.`

- [ ] **Step 8: Run everything and look at it**

Run: `cargo test && cargo build 2>&1 | grep -c warning`
Expected: all PASS; `0`.

Then, if the user's daemon may be restarted (ask first — it is their live panel): `cargo build --release && systemctl --user restart niri-tasks`, press Mod+Alt+Ctrl+T on a workspace with tasks, press `m` on a card, and check the folders show as cards with this workspace's folder left out, Up/Down darken them in turn, Escape goes back to the same card, and Enter on a folder moves the task (it leaves the list, a notification says `Moved to +<tag>: …`, and `task +<tag> export` shows it there). Move it back with `niritasks task move <uuid8> <this-folder>` from that workspace's terminal.

- [ ] **Step 9: Commit**

```bash
git add src/actions.rs src/panel/actions.rs src/panel/style.rs src/panel/keys.rs src/panel/state.rs src/panel/surface.rs src/main.rs
git commit -m "$(cat <<'EOF'
feat(panel): move a task to another project from its card

Move to workspace (m) swaps the cards for the ~/Projects folders the
task can go to, drawn as cards. Up and Down walk them, Enter or a
click runs task move, and Escape goes back to the same card. The mode
lives in PanelState, so it is tested without a window.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Docs and the e2e test

**Files:**
- Modify: `README.md`, `llms.txt`, `CONTEXT.md`, `.claude/skills/finish-worktree/SKILL.md`
- Modify: `tests/e2e-panel.sh`

**Interfaces:**
- Consumes: the finished behaviour of Tasks 1–4. Nothing produced.

- [ ] **Step 1: Add the e2e block**

In `tests/e2e-panel.sh`, insert before `# ─── nothing pending shows nothing`:

```bash
# ─── a card's Complete and Move to workspace ─────────────────────────────────
# c completes the focused task and keeps the list up. m swaps the cards for
# the ~/Projects folders the task could move to, and Escape puts the cards
# back as they were. Enter on a folder is not pressed: the move would run in
# the nested niri's spawn, with the real HOME and ~/Projects. write_path.rs
# checks the retagging, against its sandbox.
if command -v wtype >/dev/null; then
    add "complete me"
    add "move me"
    settle
    completed() { task rc.verbose=nothing "+$TAG" status:completed count 2>/dev/null; }
    before=$(completed)
    "${NENV[@]}" "$NIRITASKS" task panel >/dev/null 2>&1
    settle
    "${NENV[@]}" wtype c
    for _ in $(seq 1 50); do [ "$(completed)" -gt "$before" ] && break; sleep 0.1; done
    if [ "$(completed)" -eq $((before + 1)) ]; then
        ok "c completes the focused task"
    else
        bad "c completed $(($(completed) - before)) tasks, expected 1"
    fi
    settle
    if find "$HOME/Projects" -mindepth 1 -maxdepth 1 -type d ! -name '.*' ! -name e2e 2>/dev/null | grep -q .; then
        shot move_before || { summary; exit 1; }
        "${NENV[@]}" wtype m
        sleep 1
        shot move_list || { summary; exit 1; }
        if same move_before move_list; then
            bad "m left the panel as it was — no project list"
        else
            ok "m swaps the cards for the project list"
        fi
        "${NENV[@]}" wtype -k Escape
        sleep 1
        shot move_back || { summary; exit 1; }
        if same move_before move_back; then
            ok "Escape from the project list puts the cards back as they were"
        else
            read -r x0 x1 y0 y1 < <(measure move_back move_before)
            bad "after Escape from the project list the panel differs in columns ${x0}-${x1}, rows ${y0}-${y1}"
        fi
    else
        skip "the project list (no ~/Projects folder to move a task to)"
    fi
    "${NENV[@]}" wtype -k Escape
    settle
else
    skip "a card's Complete and Move to workspace (needs wtype)"
fi
```

In the header comment's list of keys pressed (`… Ctrl+Shift+Delete and Enter for the Waiting tab's Clear all, and Escape are pressed …`) add `c and m on a card,` before `and Escape`.

Run: `bash tests/e2e-panel.sh` (it starts a nested niri in a window on the current desktop; keep off its workspace while it runs).
Expected: every line `ok` or `skip`, the run exits 0. If it cannot run here (no niri session), say so in the report rather than skipping silently.

- [ ] **Step 2: README**

Make these edits in `README.md` (each quoted phrase is the current text):

- The `Mod+Alt+Ctrl+T` key row: `… and the focused one has buttons to start working on it in its own worktree, refine it into a plan with Claude, edit it, read it aloud, stop it, park it as waiting, or remove it. Enter opens the full menu (note, grill me, update status, move to another workspace).` → `… and the focused one has buttons to start working on it in its own worktree, refine it into a plan with Claude (or be grilled about it first), edit it, read it aloud, complete it, move it to another project's workspace, stop it, park it as waiting, or remove it.`; and `With no tasks, or no daemon, the fuzzel list instead` → `With no tasks, or no name, a notification says so instead`.
- `Claude from the menu's **Refine** or **Grill me**` → `Claude from a card's **Refine** or **Grill me**`.
- The paragraph `Click a card to open that task's actions in fuzzel — the same menu the picker shows once you pick a task — on the monitor you clicked. The "+N more" card shows the rest of the tasks in the panel.` → `Clicking a card does nothing: its actions are the buttons the keyboard shows (below). The "+N more" card shows the rest of the tasks in the panel.`
- Button table: `The menu's Go to session: back to` → `Back to`; `The menu's Start working: the task's own` → `The task's own`; after the Refine row add `| **Grill me** (mauve) | \`i\` | Refine, with Claude interviewing you about it first |`; after the Up next row add `| **Complete** (green) | \`c\` | Mark it done: it leaves the panel, which keeps the keyboard |` and `| **Move to workspace** (sapphire) | \`m\` | Pick another \`~/Projects\` folder from a list in the panel, and move the task to that folder's workspace |`; in the Back to list row `which gets just this, Edit, Speak and Remove` → `which gets just this, Edit, Speak, Complete, Move to workspace and Remove`.
- In the paragraph after the table, replace `Enter on the card itself opens its full menu, which also has Note, Grill me, Update status and Move to workspace — led by Go to session while a Claude is working on the task, and on a waiting task just Edit, Note, Speak, Update status and Move to workspace. Every button that opens something gives the keyboard back as it runs; Speak, Up next, Waiting, Back to list and Remove keep the list up,` with `Enter on the card's description itself does nothing. Every button that opens something gives the keyboard back as it runs; Speak, Up next, Complete, Move to workspace, Waiting, Back to list and Remove keep the list up,`. Then add a paragraph after it:

  ```
  **Move to workspace** swaps the cards for a list of your `~/Projects`
  folders, drawn like cards under a line naming the task: every folder but
  this workspace's own. Up and Down walk it, and Enter or a click moves the
  task there, retagging it to that folder's workspace, and goes back to the
  cards with the next one focused. Escape goes back to the cards on the same
  one without moving it.
  ```
- `let go of Ctrl before pressing Enter if you only want the card's menu, or Delete if you want Remove to ask` → `let go of Ctrl before pressing Enter if you only want the focused button, or Delete if you want Remove to ask`.
- `The menu's **Refine** and **Grill me** need both herdr` → `A card's **Refine** and **Grill me** need both herdr`.
- `so the task's menu can find it` → `so the task's card can offer Go to session`.
- `and the menu's Note opens it the same way with the cursor in a new empty row at the end` → `and \`niritasks task note <uuid>\` opens it the same way with the cursor in a new empty row at the end`.
- `as the menu's Refine does` → `as a card's Refine does`.
- `The pickers use a stripped-down fuzzel theme` → `The project picker (Mod+Alt+W) uses a stripped-down fuzzel theme`; `Without it the picker still works` → `Without it the project picker still works`.
- Testing paragraph: `while a click on a card opens its actions on that monitor` → `while a click on a card does nothing`; `` `s` starts working exactly as the menu does `` → `` `s` starts working ``; `Enter on a card opens the menu,` → `Enter on a card's description does nothing, \`c\` completes the task, \`m\` shows the project list and Escape comes back from it while Enter on a folder moves the task there,`.

Then run `grep -n -i -E "menu|picker|fuzzel" README.md` and check each hit left is about the Mod+Alt+W project picker, `workspace new`/`rename`'s prompt, or fuzzel as a requirement.

- [ ] **Step 3: llms.txt**

- Rule 1: replace `` `task active`, `task list` and `task menu` follow focus too. `task add`, `task refine`, `task start` and `task session` do not in a pane of a named herdr session: there they take the session's workspace, so `niritasks task add "<type>: <description>"` files under it (see rule 8) and the other three open in its herdr session; `` with `` `task active` follows focus too. `task add`, `task move`, `task refine`, `task start` and `task session` do not in a pane of a named herdr session: there they take the session's workspace, so `niritasks task add "<type>: <description>"` files under it (see rule 8), `task move` moves a task off it, and the other three open in its herdr session; ``.
- Rule 4: `It is the same path the task menu uses. \`active\` also links the herdr pane you run it in to the task, so its menu can find you, and every change sends the menu's notification.` → `It is the same path a task card's buttons use. \`active\` also links the herdr pane you run it in to the task, so its card's Go to session can find you, and every change sends the same notification a button does.`
- Rule 5: `is the menu's confirmation, given up front` → `is Remove's confirmation, given up front`.
- The CONTEXT.md link line: `task panel, task card, picker, task box` → `task panel, task card, action row, project list, task box`.

Run: `cargo test --bin niritasks llms_txt` — Expected: PASS.

- [ ] **Step 4: CONTEXT.md**

- Planned task: `on its task card and picker row` → `on its task card`.
- Up next task: `from Up next in its action menu or on its action row` → `from Up next on its action row`; `the tag adds urgency, so the picker lifts it too` → `the tag adds urgency, so \`task next\` lifts it too`.
- Task card's `_Avoid_: row (that is the picker's word), item, block` → `_Avoid_: row (the action row is part of a card), item, block`.
- Task action: the list becomes `Go to session, Back to list, Start working, Refine, Grill me, Edit, Speak, Up next, Complete, Move to workspace, Stop, Waiting or Remove`; `a waiting task gets only Back to list, Edit, Note, Speak and Remove. The action row and the action menu each show their own share of them, in their own order.` → `a waiting task gets only Back to list, Edit, Speak, Complete, Move to workspace and Remove. The action row shows them.`; `_Avoid_: command (the CLI's word), button, menu entry, verb` → `_Avoid_: command (the CLI's word), button, verb`.
- Action row: the list becomes `Go to session, Start working, Refine, Grill me, Edit, Speak, Up next, Complete, Move to workspace, Stop, Waiting and Remove`; `a waiting task getting just Back to list, Edit, Speak and Remove — each running what the same entry in the task's action menu runs.` → `a waiting task getting just Back to list, Edit, Speak, Complete, Move to workspace and Remove — each running its task action's \`niritasks\` command. Enter or a click on the card's body does nothing.`
- Delete the **Picker** and **Action menu** entries. In their place:

  ```
  **Project list**:
  What Move to workspace shows in the task panel in place of the task cards:
  the `~/Projects` folders the task can move to, every one but its own
  workspace's, drawn like task cards under a line naming the task. Up and
  Down walk it, Enter or a click moves the task to that folder's workspace,
  and Escape goes back to the task cards on the same card.
  _Avoid_: picker (the fuzzel list Mod+Alt+W opens), menu, folder list
  ```
- The session entry's `_Avoid_: project (the picker's word)` → `_Avoid_: project (the project picker's word)`.

Run `grep -n -i -E "menu|picker" CONTEXT.md` and check each hit left is the project picker or the new entry's `_Avoid_`.

- [ ] **Step 5: The finish-worktree skill**

In `.claude/skills/finish-worktree/SKILL.md`, `niritasks is the one path the task menu also uses, and it sends the same notification.` → `niritasks is the one path a task card's Complete also uses, and it sends the same notification.`

- [ ] **Step 6: Run everything**

Run: `cargo test && cargo build 2>&1 | grep -c warning && grep -rn -i "task list\|task menu\|action menu" README.md llms.txt CONTEXT.md src tests .claude/skills`
Expected: all PASS; `0`; the grep prints nothing.

- [ ] **Step 7: Commit**

```bash
git add README.md llms.txt CONTEXT.md .claude/skills/finish-worktree/SKILL.md tests/e2e-panel.sh
git commit -m "$(cat <<'EOF'
docs: describe the panel without the fuzzel picker and menu

The README, llms.txt and CONTEXT.md describe Grill me, Complete and
Move to workspace on the action row, the project list, and a card's
body doing nothing. The e2e test presses c and m on a card.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
