# Open Projects from the Panel's Project List — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mod+Alt+W opens the task panel's project list (the one Move to workspace uses) instead of the fuzzel picker, and free workspace naming goes: a workspace's name is always a `~/Projects` folder or a GitHub repo cloned there.

**Architecture:** The panel's `Moving` mode becomes a `ProjectList` with a `Purpose`: `Move { uuid, text }` as today, or `Open`, which has no task and so shows with no cards and on an unnamed workspace. `niritasks project open` with no argument refreshes the GitHub cache and asks the daemon (new `ipc::Request::Projects`) to put the focused monitor's panel on the open list; picking a row spawns `niritasks project open <row>`, which runs the existing post-pick logic (clone or create, then focus or name a workspace and start its programs). `workspace rename`, `workspace new`, `prompt_for_name` and the Mod+Alt+Ctrl+W bind go. Everything new in the state is pure and unit-tested; the surface only draws it.

**Tech Stack:** Rust 2021, clap 4 (derive), GTK4 (gtk4-rs), niri IPC, Taskwarrior 2.6, `gh` for the GitHub cache. No new crates.

**Spec:** Taskwarrior task `46320b16-83f1-4d27-b987-fafa71cb71ee`. Read it with `task rc.json.array=on 46320b16-83f1-4d27-b987-fafa71cb71ee export`. Its description and notes are the spec.

## Global Constraints

- Mod+Alt+W opens the task panel's project list, with no task, over IPC on the focused monitor with the keyboard, and it opens even when the workspace has no tasks or no name.
- It keeps what the fuzzel picker had: local folders, then uncloned GitHub repos (cloned on pick), and a typed name matching nothing creates `~/Projects/<name>`.
- A pick goes through the existing post-pick logic: focus the project's workspace if open, else name the last workspace and run `startup_commands`.
- Remove `niritasks workspace rename`, its Mod+Alt+Ctrl+W bind, `workspace new` and the fuzzel `prompt_for_name`; `workspace default` stays.
- Update CONTEXT.md, README, llms.txt and the kdl comments.
- Done when: Mod+Alt+W shows the panel's project list and opens, clones or creates a project from it, rename/new no longer parse, and tests pass.
- Done when: the user gets a list of everything still using or naming fuzzel (expect `src/picker.rs`, `fuzzel/picker.ini`, the `install.sh` link and dep check, README, llms.txt, kdl comment, CONTEXT.md, code comments), filed as a follow-up `chore:` task to remove fuzzel.
- Out of scope: removing fuzzel itself; that waits for the follow-up plan.
- House style: every new item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commits use Conventional Commits (imperative, lowercase, ≤72 chars) and end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Build warnings count: finish each task with `cargo build 2>&1 | grep -c warning` printing `0`.

## Decisions made while planning (flag any you disagree with)

- **`project open <name>` is the pick.** The panel runs the CLI for everything it does, so a picked row spawns `niritasks project open <row>` on the panel's monitor (`spawn_on` focuses it first, which is the output the post-pick logic names a workspace on). The row is passed as the list shows it, GitHub marker and all; `github::choose` already turns `name  (github)` into a clone. A new name is passed normalised.
- **Bare `project open` refreshes the GitHub cache, not the daemon.** `github::spawn_refresh` spawns `sh` and never waits on it. In the CLI, which exits at once, init reaps it; in the daemon it would sit as a zombie. The daemon only reads the cache.
- **No daemon is an error**, as `task panel` is: "The niri-tasks daemon is not running, so there is no project list. Start it with `systemctl --user start niri-tasks`." There is no fuzzel fallback.
- **The open list takes over the panel.** Opening it drops the focus, the tab and any armed button, as `take_keyboard` does. Escape with text typed clears it; with none it gives the keyboard back (there may be no cards to go back to, and Mod+Alt+W was not asked from the cards). A pick gives the keyboard back too, then spawns: the user is going to another workspace.
- **A new name only when nothing matches**, as fuzzel did: Enter on an empty list with text typed spawns `project open <normalised>`. A line under the title says what Enter will make (`Enter makes ~/Projects/my-thing`), or why it cannot (`project::resolve`'s Rejected message), or, with nothing typed and no rows at all, `Type a name to make a new project`. Move's list says nothing there, as today.
- **GitHub rows wear Font Awesome's GitHub mark** (`\u{f09b}`) and the bare repo name; folders keep `FOLDER_ICON`. Ranking still runs on the full row, so typing `github` finds the repos.
- **Title and footer:** the line over the open list reads `Open a project`; its footer `Type to filter · Up, Down: pick · Enter: open it · Esc: clear, then close`.
- **The unnamed-workspace refusal** now reads `This workspace has no name — Mod+Alt+W opens a project on a named one.` (it named Mod+Alt+Ctrl+W, which goes).
- **README's fuzzel theme note goes now**, since it describes the Mod+Alt+W picker. `fuzzel` in the requirements, `install.sh`'s link and dep check, `fuzzel/picker.ini`, `src/picker.rs` and llms.txt's "wraps … fuzzel" stay for the follow-up task, which this plan files.
- **The e2e test opens the list on an empty workspace and closes it with Escape**, never picking: a pick would run in the nested niri's spawn, with the real `HOME` and `~/Projects`. The CLI call gets its own `XDG_CACHE_HOME`, so its `gh` refresh does not write the real cache.

## File Structure

- `src/panel/state.rs` — `Moving` becomes `ProjectList` + `Purpose`; `open_projects`, `ProjectList::{moving, title, keys, new_folder, no_match_text}`; `hidden`, `set_cards`, `escape_projects`, `on_folder` learn the open list.
- `src/panel/actions.rs` — `OPEN_KEYS`, `OPEN_TITLE`, `TYPE_A_NAME`, `GITHUB_ICON`, `folder_label`.
- `src/panel/surface.rs` — renames; draws title, footer and no-match line from the state; `open_projects`; `take_keyboard`'s tail becomes `centre`.
- `src/project.rs` — `open_args`, `open_rows`, `rows_to_open`.
- `src/ipc.rs` — `Request::Projects`.
- `src/daemon.rs` — serves `Request::Projects`; refusal test strings.
- `src/main.rs` — `project open [NAME]`; `open_project`; `WorkspaceCommand::{New, Rename}` and `prompt_for_name` go.
- `src/github.rs` — comments that said fuzzel echoes the row back.
- `src/lib.rs` — the unnamed-workspace refusal text.
- `niri/niri-tasks.kdl`, `install.sh`, `tests/e2e-tag.sh`, `.claude/skills/workspace-tasks/SKILL.md` — the bind and the text naming it.
- `tests/e2e-panel.sh` — an open-list block.
- `README.md`, `llms.txt`, `CONTEXT.md` — docs.

Run every command from the worktree root.

---

### Task 1: Rename the panel's `Moving` to `ProjectList` with a `Purpose`

A pure refactor: no behaviour changes, and every existing test passes after it. It gives the next task a place to put the open list.

**Files:**
- Modify: `src/panel/state.rs` (types at ~68-89, `PanelState` field ~145 and accessor ~180, `set_cards` ~262, `show_projects` ~471-485, `on_folder` ~501-516, every `self.moving`, tests ~1597-1850)
- Modify: `src/panel/surface.rs` (`connect_tab_bar` ~447, `connect_keys` ~543-549, `update_tabs` ~753-756, `render` ~885-950, `sync` ~1133, `follow_focus` ~1183)

**Interfaces:**
- Produces: `pub enum Purpose { Move { uuid: String, text: String } }`; `pub struct ProjectList { pub purpose: Purpose, pub folders: Vec<String>, pub query: String, pub shown: Vec<String>, pub at: usize }`; `impl ProjectList { pub fn moving(&self) -> Option<&str>; pub fn title(&self) -> String; pub fn keys(&self) -> &'static str }`; `PanelState::projects(&self) -> Option<&ProjectList>` (replaces `moving()`). The field is `PanelState::projects: Option<ProjectList>`.

- [ ] **Step 1: Write the failing test**

Add to the `// ─── Move to workspace ───` section of `src/panel/state.rs`'s tests, after `move_shows_the_project_list_in_place_of_the_cards`:

```rust
    /// The line over the folders names the task, and the footer Move's keys.
    #[test]
    fn the_move_list_names_its_task_and_its_keys() {
        let state = moving_a();
        let list = state.projects().unwrap();
        assert_eq!(list.title(), format!("{}: a", Action::Move.label(false)));
        assert_eq!(list.keys(), actions::MOVE_KEYS);
        assert_eq!(list.moving(), Some("a"));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --lib panel::state::tests::the_move_list_names_its_task_and_its_keys`
Expected: compile error, `no method named projects`.

- [ ] **Step 3: Rename mechanically**

```bash
sed -i 's/\.moving()/.projects()/g; s/self\.moving\b/self.projects/g' src/panel/state.rs src/panel/surface.rs
```

Then in `src/panel/state.rs`:

1. Add `use super::actions;` under `use super::keys::{self, KeyAction};`.
2. Replace the `Moving` struct and its doc comment with:

```rust
/// What the project list is for, and so what picking a folder does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    /// Move to workspace: the task being moved, by uuid, and its
    /// description, for the line over the folders.
    Move { uuid: String, text: String },
}

/// The project list the panel swaps in for the cards: the `~/Projects`
/// folders, narrowed by what is typed, and what picking one is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectList {
    pub purpose: Purpose,
    /// The folders, as the surface listed them for this purpose.
    pub folders: Vec<String>,
    /// What is typed in the field over the list.
    pub query: String,
    /// The folders on show: those matching `query`, best first, or every
    /// folder with nothing typed. Ranked by the surface, which can run fzf.
    pub shown: Vec<String>,
    /// The folder in `shown` the highlight is on: Up, Down and Enter's.
    pub at: usize,
}

impl ProjectList {
    /// The task being moved, on Move to workspace's list.
    pub fn moving(&self) -> Option<&str> {
        match &self.purpose {
            Purpose::Move { uuid, .. } => Some(uuid.as_str()),
        }
    }

    /// The line over the folders: which task is moving.
    pub fn title(&self) -> String {
        match &self.purpose {
            Purpose::Move { text, .. } => format!("{}: {text}", Action::Move.label(false)),
        }
    }

    /// The footer under the list: the keys that act on it.
    pub fn keys(&self) -> &'static str {
        match self.purpose {
            Purpose::Move { .. } => actions::MOVE_KEYS,
        }
    }
}
```

3. The field: `moving: Option<Moving>,` becomes `projects: Option<ProjectList>,` (its doc comment stays: "The project list is up in place of the cards…").
4. The accessor: `pub fn projects(&self) -> Option<&Moving>` (after sed) returns `Option<&ProjectList>`, body `self.projects.as_ref()`.
5. In `set_cards`, replace the `if self.projects.as_ref().is_some_and(|m| …m.uuid…)` block with:

```rust
        // The task being moved has gone (done elsewhere, say): its project
        // list goes with it.
        let gone = self
            .projects
            .as_ref()
            .and_then(ProjectList::moving)
            .is_some_and(|uuid| !self.all.iter().any(|c| c.uuid.as_deref() == Some(uuid)));
        if gone {
            self.projects = None;
        }
```

6. In `show_projects`, the constructor becomes:

```rust
        self.projects = Some(ProjectList {
            purpose: Purpose::Move { uuid: uuid.into(), text },
            folders,
            query: String::new(),
            shown,
            at: 0,
        });
```

7. `on_folder` becomes:

```rust
    pub fn on_folder(&mut self, at: usize) -> Vec<Effect> {
        let Some(list) = self.projects.take() else { return Vec::new() };
        let Some(folder) = list.shown.get(at).cloned() else {
            self.projects = Some(list);
            return Vec::new();
        };
        let Purpose::Move { uuid, .. } = &list.purpose;
        if let Some(next) = self.neighbour(uuid) {
            self.focus = Some(next);
        }
        let mut effects = self.rerender();
        let mut args = Action::Move.args(uuid);
        args.push(folder);
        effects.push(Effect::Spawn(args));
        effects
    }
```

8. In the tests, `move_shows_the_project_list_in_place_of_the_cards` builds its expected value as:

```rust
            Some(&ProjectList {
                purpose: Purpose::Move { uuid: "a".into(), text: "a".into() },
                folders: folders(&["x", "y"]),
                query: String::new(),
                shown: folders(&["x", "y"]),
                at: 0,
            })
```

In `src/panel/surface.rs`:

1. Rename the local `moving` variables in `connect_keys`, `update_tabs` and `render` to `list`, so they read as what they hold.
2. In `render`, the header line becomes `self.column.append(&empty_line(&m.title()));` (keep the comment "Which task is moving, over the folders." → "What the list is for, over the folders.").
3. The footer becomes `self.footer.set_label(list.as_ref().map_or(actions::CARD_KEYS, |l| l.keys()));`.
4. Update the module doc and comments that say "moving" only where they name the old type; the prose about Move to workspace stays.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib panel::`
Expected: PASS, the new test included. Then `cargo build 2>&1 | grep -c warning` prints `0`.

- [ ] **Step 5: Commit**

```bash
git add src/panel/state.rs src/panel/surface.rs
git commit -m "refactor(panel): make Move's project list a ProjectList with a purpose

So the same list can serve another purpose than moving one task.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The open list in `PanelState`, and how its rows are drawn

**Files:**
- Modify: `src/project.rs` (add `open_args` after `move_task`)
- Modify: `src/panel/actions.rs` (constants after `FOLDER_ICON` ~160, a test in its tests module)
- Modify: `src/panel/state.rs` (module doc, `Purpose`, `ProjectList`, `hidden`, `set_cards`, new `open_projects`, `on_key` comment, `escape_projects`, `on_folder`, tests)
- Modify: `src/panel/surface.rs` (`render`, `folder_card`)

**Interfaces:**
- Consumes: Task 1's `ProjectList`, `Purpose`, `PanelState::projects()`.
- Produces: `Purpose::Open`; `PanelState::open_projects(&mut self, rows: Vec<String>) -> Vec<Effect>`; `ProjectList::new_folder(&self) -> Option<String>`; `ProjectList::no_match_text(&self) -> Option<String>`; `project::open_args(row: &str) -> Vec<String>`; `panel::actions::{OPEN_KEYS, OPEN_TITLE, TYPE_A_NAME, GITHUB_ICON}: &str` and `panel::actions::folder_label(row: &str) -> String`.

- [ ] **Step 1: Write the failing tests**

In `src/project.rs`'s tests:

```rust
    /// Picking a row runs `project open` with the row as the list shows it.
    #[test]
    fn opening_a_row_is_project_open_with_the_row() {
        assert_eq!(open_args("alpha  (github)"), vec!["project", "open", "alpha  (github)"]);
    }
```

In `src/panel/actions.rs`'s tests:

```rust
    /// A GitHub row shows the repo's bare name beside GitHub's mark; a
    /// folder its name beside the folder.
    #[test]
    fn a_rows_label_says_whether_it_is_a_repo() {
        assert_eq!(folder_label("alpha"), format!("{FOLDER_ICON}  alpha"));
        assert_eq!(folder_label(&crate::github::mark("convo")), format!("{GITHUB_ICON}  convo"));
    }
```

In `src/panel/state.rs`'s tests, a new section after the Move to workspace one:

```rust
    // ─── Open a project (Mod+Alt+W) ──────────────────────────────────────

    /// The panel on the open list of folder x and GitHub repo y.
    fn opening(cards: &[Card]) -> PanelState {
        let mut state = PanelState::default();
        state.set_cards(cards);
        assert_eq!(state.open_projects(folders(&["x", "y  (github)"])), vec![Effect::Render]);
        state
    }

    fn open(row: &str) -> Effect {
        Effect::Spawn(crate::project::open_args(row))
    }

    /// With no task on the workspace, the list still takes the keyboard and
    /// shows: no cards, no tabs, no focus.
    #[test]
    fn the_open_list_shows_with_no_cards() {
        let state = opening(&[]);
        assert!(state.keyboard());
        assert!(!state.hidden());
        let list = state.projects().unwrap();
        assert_eq!(list.purpose, Purpose::Open);
        assert_eq!(list.shown, folders(&["x", "y  (github)"]));
        assert_eq!((list.title(), list.keys()), (actions::OPEN_TITLE.to_string(), actions::OPEN_KEYS));
        assert_eq!(list.moving(), None);
        assert!(state.visible().is_empty());
        assert!(state.tabs().is_empty());
        assert_eq!(state.empty_text(), None);
        assert_eq!(state.focus(), None);
    }

    /// Over cards it takes their place, dropping the focus and anything armed.
    #[test]
    fn the_open_list_takes_the_place_of_the_cards() {
        let mut state = keyboard(pending(&["a", "b"]));
        key(&mut state, KeyAction::Run(Action::Remove));
        assert_eq!(state.open_projects(folders(&["x"])), vec![Effect::Render]);
        assert!(state.visible().is_empty());
        assert_eq!(state.focus(), None);
        assert_eq!(state.armed(), &Armed::None);
    }

    /// Enter opens the highlighted row, GitHub marker and all, giving the
    /// keyboard back first: the user is off to that workspace.
    #[test]
    fn enter_opens_the_highlighted_row_and_gives_the_keyboard_back() {
        let mut state = opening(&[]);
        assert_eq!(key(&mut state, KeyAction::NextCard), vec![Effect::ShowFolder]);
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("y  (github)")]);
        assert!(!state.keyboard());
        assert_eq!(state.projects(), None);
        assert!(state.hidden(), "no cards to peek");
    }

    #[test]
    fn a_click_opens_that_row() {
        let mut state = opening(&pending(&["a"]));
        assert_eq!(state.on_folder(0), vec![Effect::Render, Effect::Release, open("x")]);
        assert!(!state.hidden(), "the card peeks again");
    }

    /// A name matching nothing makes a folder of it, normalised as
    /// `project open` will, and the line under the title says so first.
    #[test]
    fn a_name_matching_nothing_makes_a_new_folder() {
        let mut state = opening(&[]);
        state.on_query("my thing", Vec::new());
        let list = state.projects().unwrap();
        assert_eq!(list.new_folder(), Some("my-thing".to_string()));
        assert_eq!(list.no_match_text(), Some("Enter makes ~/Projects/my-thing".to_string()));
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("my-thing")]);
    }

    /// While anything matches, Enter takes the match, as fuzzel did.
    #[test]
    fn a_match_wins_over_a_new_name() {
        let mut state = opening(&[]);
        state.on_query("xx", folders(&["x"]));
        assert_eq!(state.projects().unwrap().new_folder(), None);
        assert_eq!(state.projects().unwrap().no_match_text(), None);
        assert_eq!(key(&mut state, KeyAction::Enter), vec![Effect::Render, Effect::Release, open("x")]);
    }

    /// A name that cannot be a folder says why, and Enter does nothing.
    #[test]
    fn a_name_that_cannot_be_a_folder_says_why() {
        let mut state = opening(&[]);
        state.on_query("a/b", Vec::new());
        let text = state.projects().unwrap().no_match_text().unwrap();
        assert!(text.contains("can't contain '/'"), "{text}");
        assert_eq!(key(&mut state, KeyAction::Enter), Vec::new());
        assert!(state.projects().is_some());
    }

    /// An empty ~/Projects with nothing typed asks for a name.
    #[test]
    fn no_folder_and_nothing_typed_asks_for_a_name() {
        let mut state = PanelState::default();
        state.open_projects(Vec::new());
        assert_eq!(state.projects().unwrap().no_match_text(), Some(actions::TYPE_A_NAME.to_string()));
        assert_eq!(key(&mut state, KeyAction::Enter), Vec::new());
    }

    /// Move's list never makes a folder: with nothing matching, Enter does
    /// nothing and no line says otherwise.
    #[test]
    fn the_move_list_makes_no_new_folder() {
        let mut state = moving_a();
        state.on_query("zz", Vec::new());
        assert_eq!(state.projects().unwrap().new_folder(), None);
        assert_eq!(state.projects().unwrap().no_match_text(), None);
    }

    /// Escape clears the text, then gives the keyboard back: there may be
    /// no cards to go back to.
    #[test]
    fn escape_clears_the_text_then_closes_the_open_list() {
        let mut state = opening(&pending(&["a"]));
        state.on_query("y", folders(&["y  (github)"]));
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::ClearQuery, Effect::Render]);
        assert_eq!(key(&mut state, KeyAction::Release), vec![Effect::Render, Effect::Release]);
        assert!(!state.keyboard());
        assert_eq!(state.projects(), None);
    }

    /// The daemon's ticks with no task, or with tasks coming and going,
    /// leave the open list up and the keyboard held.
    #[test]
    fn ticks_keep_the_open_list() {
        let mut state = opening(&[]);
        assert_eq!(state.set_cards(&[]), Vec::new());
        assert_eq!(state.set_cards(&pending(&["a"])), vec![Effect::Render]);
        assert_eq!(state.set_cards(&[]), vec![Effect::Render]);
        assert!(state.keyboard());
        assert_eq!(state.projects().map(|l| l.purpose.clone()), Some(Purpose::Open));
    }

    /// Mod+Alt+Ctrl+T over the open list puts the cards back.
    #[test]
    fn taking_the_keyboard_closes_the_open_list() {
        let mut state = opening(&pending(&["a"]));
        assert!(state.take_keyboard(Vec::new()));
        assert_eq!(state.projects(), None);
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib`
Expected: compile errors: `open_args`, `folder_label`, `GITHUB_ICON`, `open_projects`, `Purpose::Open`, `new_folder`, `no_match_text`, `OPEN_TITLE` not found.

- [ ] **Step 3: Implement**

`src/project.rs`, after `move_task`:

```rust
/// The `niritasks` words that open `row`, a row of the project list or a
/// new folder's name: what picking one on the panel spawns. The row goes as
/// the list shows it, so a GitHub row's marker is what makes it a clone.
pub fn open_args(row: &str) -> Vec<String> {
    vec!["project".to_string(), "open".to_string(), row.to_string()]
}
```

`src/panel/actions.rs`, after `FOLDER_ICON`:

```rust
/// The line over the project list Mod+Alt+W opens.
pub const OPEN_TITLE: &str = "Open a project";

/// The footer while that list shows: the same keys as Move's, but Enter
/// opens the project and Escape, with nothing typed, closes the panel, there
/// being no cards it was opened from.
pub const OPEN_KEYS: &str = "Type to filter · Up, Down: pick · Enter: open it · Esc: clear, then close";

/// What the open list says with no folder to show and nothing typed: an
/// empty `~/Projects`, where the first project has to be made.
pub const TYPE_A_NAME: &str = "Type a name to make a new project";

/// A GitHub repo's card on the open list: Font Awesome's GitHub mark, so a
/// repo not cloned yet reads apart from a folder.
pub const GITHUB_ICON: &str = "\u{f09b}";

/// A project list row as its card reads: the icon, then the name. A GitHub
/// row shows the bare repo name, its marker being for `project open`, not
/// for reading.
pub fn folder_label(row: &str) -> String {
    // The two spaces are the gap a task card leaves after its icon.
    match crate::github::unmark(row) {
        Some(repo) => format!("{GITHUB_ICON}  {repo}"),
        None => format!("{FOLDER_ICON}  {row}"),
    }
}
```

`src/panel/state.rs`:

1. Module doc: "and the project list Move to workspace swaps in all live here" → "and the project list Move to workspace and Mod+Alt+W swap in all live here".
2. `Purpose` gains:

```rust
    /// Mod+Alt+W: open the project picked on its own workspace, cloning a
    /// GitHub row or making a folder of a new name, as `project open` does.
    /// It has no task, so it shows with no cards and on an unnamed
    /// workspace too: opening a project is how a workspace gets a name.
    Open,
```

3. `ProjectList`'s methods gain the `Open` arms, `moving` → `Purpose::Open => None`, `title` → `Purpose::Open => actions::OPEN_TITLE.to_string()`, `keys` → `Purpose::Open => actions::OPEN_KEYS`, and two new methods:

```rust
    /// What Enter makes a new `~/Projects` folder of on the open list: the
    /// text typed, normalised as `project open` will, once nothing matches
    /// it, as fuzzel echoed text that matched no row. None on Move's list,
    /// while anything matches, and for text no folder can be named.
    pub fn new_folder(&self) -> Option<String> {
        if self.purpose != Purpose::Open || !self.shown.is_empty() {
            return None;
        }
        match crate::project::resolve(&self.query, &self.folders) {
            crate::project::Resolved::Create(name) => Some(name),
            _ => None,
        }
    }

    /// The line under the title when the open list shows nothing: what
    /// Enter will make, why the text cannot be a folder, or, with nothing
    /// typed and no folder at all, to type a name. None on Move's list.
    pub fn no_match_text(&self) -> Option<String> {
        if self.purpose != Purpose::Open || !self.shown.is_empty() {
            return None;
        }
        match crate::project::resolve(&self.query, &self.folders) {
            crate::project::Resolved::Create(name) => Some(format!("Enter makes ~/Projects/{name}")),
            crate::project::Resolved::Rejected(why) => Some(why),
            crate::project::Resolved::Nothing => Some(actions::TYPE_A_NAME.to_string()),
            crate::project::Resolved::Existing(_) => None,
        }
    }
```

4. `hidden()`: the keyboard arm becomes `self.all.is_empty() && !self.ideas && self.projects.is_none()`, and its doc comment gains "Nor on the project list, which Mod+Alt+W opens with no task."
5. `set_cards`: the release line becomes `if self.keyboard && self.all.is_empty() && !self.ideas && self.projects.is_none() {` and the doc comment gains "Nor on the project list, which needs no task."
6. After `take_keyboard`, add:

```rust
    /// Mod+Alt+W: take the keyboard on the project list for opening a
    /// project, `rows` being the `~/Projects` folders and then the GitHub
    /// repos not cloned yet. With no cards too, and on an unnamed workspace.
    /// It takes the panel over as `take_keyboard` does: All, nothing
    /// expanded, armed or focused.
    pub fn open_projects(&mut self, rows: Vec<String>) -> Vec<Effect> {
        self.keyboard = true;
        self.filter = Filter::All;
        self.ideas = false;
        self.expanded = false;
        self.armed = Armed::None;
        self.focus = None;
        let shown = rows.clone();
        self.projects = Some(ProjectList { purpose: Purpose::Open, folders: rows, query: String::new(), shown, at: 0 });
        vec![Effect::Render]
    }
```

7. `escape_projects`: its `None` arm becomes

```rust
            None => {
                // The open list was asked for from anywhere, not from the
                // cards, which there may be none of: it closes the panel.
                if self.projects.as_ref().is_some_and(|l| l.purpose == Purpose::Open) {
                    return self.release();
                }
                self.projects = None;
                self.rerender()
            }
```

and its doc comment: "with none, back to the cards, on the card and slot the focus was on; or, on the open list, the keyboard given back."

8. `on_folder` becomes (doc comment included):

```rust
    /// A folder on the project list, by Enter or a click. Moving, `task
    /// move` takes the task there, back on the cards with the focus on the
    /// next one, since the task leaves this workspace; the keyboard stays.
    /// Opening, `project open` opens it, or makes a folder of a new name
    /// with nothing matching, and the keyboard goes back first: the user is
    /// off to that workspace. Nothing with no row and no new name.
    pub fn on_folder(&mut self, at: usize) -> Vec<Effect> {
        let Some(list) = self.projects.take() else { return Vec::new() };
        let Some(row) = list.shown.get(at).cloned().or_else(|| list.new_folder()) else {
            self.projects = Some(list);
            return Vec::new();
        };
        match &list.purpose {
            Purpose::Open => {
                let mut effects = self.release();
                effects.push(Effect::Spawn(crate::project::open_args(&row)));
                effects
            }
            Purpose::Move { uuid, .. } => {
                if let Some(next) = self.neighbour(uuid) {
                    self.focus = Some(next);
                }
                let mut effects = self.rerender();
                let mut args = Action::Move.args(uuid);
                args.push(row);
                effects.push(Effect::Spawn(args));
                effects
            }
        }
    }
```

9. In `on_key`'s project-list branch, the comment "Enter moves the task" → "Enter picks the folder".

`src/panel/surface.rs`:

1. In `render`, after the loop that appends the folder cards:

```rust
                // Nothing matches on the open list: what Enter makes instead.
                if let Some(text) = m.no_match_text() {
                    self.column.append(&empty_line(&text));
                }
```

2. In `folder_card`, the label becomes `gtk4::Label::new(Some(&actions::folder_label(folder)))` (drop the two-spaces comment there; it moved to `folder_label`), and its doc comment's "A click on it moves the task there." → "A click on it picks it."

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS. Then `cargo build 2>&1 | grep -c warning` prints `0`.

- [ ] **Step 5: Commit**

```bash
git add src/project.rs src/panel/actions.rs src/panel/state.rs src/panel/surface.rs
git commit -m "feat(panel): add a project list for opening a project

It has no task, so it shows with no cards and on an unnamed workspace.
Picking a row, or a name matching none, runs project open with it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The daemon shows the open list over IPC

**Files:**
- Modify: `src/project.rs` (add `open_rows`, `rows_to_open` after `open_args`; a test)
- Modify: `src/ipc.rs` (module doc protocol block, `Request`, `encode`, `decode`, tests)
- Modify: `src/daemon.rs` (`serve_box_request` and its doc comment, a `NO_PANEL` const)
- Modify: `src/panel/surface.rs` (`take_keyboard` → `take_keyboard` + `centre`, new `open_projects`, module doc)

**Interfaces:**
- Consumes: Task 2's `PanelState::open_projects(Vec<String>) -> Vec<Effect>`.
- Produces: `ipc::Request::Projects` (wire form `projects`); `project::open_rows() -> anyhow::Result<Vec<String>>`; `project::rows_to_open(names: &[String], cached: &[String]) -> Vec<String>`; `Panel::open_projects(self: &Rc<Self>, rows: Vec<String>)`.

- [ ] **Step 1: Write the failing tests**

`src/project.rs` tests:

```rust
    /// The folders first, then each cached repo that is not one already,
    /// marked as GitHub's.
    #[test]
    fn rows_to_open_are_folders_then_uncloned_repos() {
        let names: Vec<String> = ["alpha", "hansard"].iter().map(|s| s.to_string()).collect();
        let cached: Vec<String> = ["convo", "hansard"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            rows_to_open(&names, &cached),
            vec!["alpha".to_string(), "hansard".to_string(), crate::github::mark("convo")]
        );
    }
```

`src/ipc.rs`: add `Request::Projects,` to the array in `round_trips_every_request`, and:

```rust
    /// Mod+Alt+W's project list is a request of its own, with no uuid.
    #[test]
    fn projects_is_one_word() {
        assert_eq!(Request::Projects.encode(), "projects");
        assert_eq!(Request::decode("projects\n"), Some(Request::Projects));
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib project:: ipc::`
Expected: compile errors, `rows_to_open` and `Request::Projects` not found.

- [ ] **Step 3: Implement**

`src/project.rs`, after `open_args`:

```rust
/// The project list Mod+Alt+W shows: every `~/Projects` folder, then the
/// account's GitHub repos not cloned yet, from the cache `project open`
/// refreshes. Read on every open, so a folder made since shows.
pub fn open_rows() -> Result<Vec<String>> {
    let (_, names) = list()?;
    let cached = crate::github::cache_path()
        .map(|cache| crate::github::read_cache(&cache))
        .unwrap_or_default();
    Ok(rows_to_open(&names, &cached))
}

/// [`open_rows`] from the folders and the cached repo names: the folders,
/// then each repo that is not one already, marked as GitHub's. Split out so
/// it is tested without the real `~/Projects` or cache.
pub fn rows_to_open(names: &[String], cached: &[String]) -> Vec<String> {
    let mut rows = names.to_vec();
    rows.extend(crate::github::remote_only(cached, names).iter().map(|n| crate::github::mark(n)));
    rows
}
```

`src/ipc.rs`:
- Module doc: "or to hand the task panel the keyboard." → "to hand the task panel the keyboard, or to show its project list."; the protocol block gains a `projects` line after `panel`, and "it only ever carries a mode, a uuid and the add box's refine flag" stays true.
- `Request` gains, after `Panel`:

```rust
    /// Show the focused monitor's project list, for opening a project, with
    /// the keyboard: Mod+Alt+W.
    Projects,
```

- `encode`: `Request::Projects => "projects".into(),`
- `decode`: `("projects", _) => Some(Request::Projects),`

`src/panel/surface.rs`: replace `take_keyboard` (keep its first doc paragraph on it, move the paragraph about the right anchor and margin to `centre`) with:

```rust
    pub fn take_keyboard(self: &Rc<Self>, agents: Vec<String>) -> bool {
        if !self.state.borrow_mut().take_keyboard(agents) {
            return false;
        }
        self.centre(vec![Effect::Render]);
        true
    }

    /// Mod+Alt+W: the project list for opening a project, in the middle of
    /// the monitor with the keyboard, as `take_keyboard` puts the cards
    /// there. With no cards too: the panel shows for the list alone.
    pub fn open_projects(self: &Rc<Self>, rows: Vec<String>) {
        let effects = self.state.borrow_mut().open_projects(rows);
        self.centre(effects);
    }

    /// Hold the keyboard in the middle of the monitor, running `effects`,
    /// the state's render, on the way.
    ///
    /// (the moved paragraph: "The right anchor alone already centres the
    /// surface vertically; …")
    fn centre(self: &Rc<Self>, effects: Vec<Effect>) {
        self.cancel_grace();
        // Afresh every time: another panel may have saved this tag's ideas
        // since, its workspace having moved monitor.
        self.notepad.load(&self.tag.borrow());
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.apply(effects);
        // After the render, which measures the cards the region needs.
        self.slide_to(CENTRED_X, centre_margin(self.monitor.geometry().width()));
        self.start_spinner();
    }
```

Add a paragraph to the module doc after the Move to workspace paragraph:

```rust
//! Mod+Alt+W shows the same list for opening a project (`state::Purpose::Open`):
//! every `~/Projects` folder, then the GitHub repos not cloned yet, under
//! "Open a project", with or without cards, on a named workspace or not.
//! Enter or a click gives the keyboard back and runs `project open` with the
//! row; text matching no row makes a folder of it, and a line says so
//! first. Escape clears the text, then gives the keyboard back.
```

`src/daemon.rs`:
- After `NO_TASKS`:

```rust
/// What Mod+Alt+W says when the focused monitor has no panel to show the
/// project list on: a monitor GTK has not named yet (see `sync_monitors`).
pub const NO_PANEL: &str = "No task panel on this monitor yet to show the project list on.";
```

- In `serve_box_request`, after the `Request::Panel` arm:

```rust
        Request::Projects => {
            let output = niri::focused_workspace().ok().flatten().and_then(|w| w.output);
            let Some(panel) = output.and_then(|o| PANELS.with(|p| p.borrow().get(&o).cloned())) else {
                notify::tasks(NO_PANEL);
                return;
            };
            match crate::project::open_rows() {
                Ok(rows) => panel.open_projects(rows),
                Err(e) => notify::tasks(&e.to_string()),
            }
        }
```

- Its doc comment's last sentence: "Or hand the focused monitor's panel the keyboard, on its cards or on the project list."

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS. Then `cargo build 2>&1 | grep -c warning` prints `0`.

- [ ] **Step 5: Commit**

```bash
git add src/project.rs src/ipc.rs src/daemon.rs src/panel/surface.rs
git commit -m "feat(panel): show the project list for opening one over IPC

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `project open` asks the daemon, and `project open <name>` opens

**Files:**
- Modify: `src/main.rs` (`ProjectCommand` ~231-234, `dispatch` ~264, `project_open` ~524-597, tests)
- Modify: `src/github.rs` (module doc ~1-13, `MARKER` doc ~18-20, `read_cache` and `spawn_refresh` docs saying "picker")
- Modify: `tests/e2e-panel.sh` (a block after "nothing pending shows nothing")

**Interfaces:**
- Consumes: `ipc::Request::Projects` (Task 3); `project::open_args` (Task 2).
- Produces: `niritasks project open [NAME]`.

- [ ] **Step 1: Write the failing test**

In `src/main.rs`'s tests, after `moving_a_task_is_a_command`:

```rust
    /// Mod+Alt+W runs `project open` bare, for the panel's project list,
    /// and picking a row runs it again with the row, GitHub marker and all.
    #[test]
    fn opening_a_project_takes_an_optional_name() {
        assert!(Cli::try_parse_from(["niritasks", "project", "open"]).is_ok());
        let mut argv = vec!["niritasks".to_string()];
        argv.extend(project::open_args(&github::mark("convo")));
        assert!(Cli::try_parse_from(&argv).is_ok(), "{argv:?}");
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --bin niritasks opening_a_project_takes_an_optional_name`
Expected: FAIL, the CLI rejects the extra argument.

- [ ] **Step 3: Implement**

`ProjectCommand` becomes:

```rust
#[derive(Subcommand)]
enum ProjectCommand {
    /// Show the task panel's project list (Mod+Alt+W); with a name, put that project on its own named workspace
    Open {
        /// A ~/Projects folder, a GitHub repo as the list shows it, or a new name to make a folder of
        name: Option<String>,
    },
}
```

`dispatch`: `Command::Project(ProjectCommand::Open { name }) => return project_open(name),`

Replace `project_open` with the two functions below. `open_project`'s body from `let dir = projects_dir.join(&name);` to the end is the old `project_open`'s tail, unchanged.

```rust
/// `project open`: with no name, the daemon's project list, which runs this
/// again with the row picked; with one, open that project.
///
/// The GitHub rows come from a cache, refreshed here for the *next* open so
/// the list never waits on the network. Here and not in the daemon: the
/// refresh is a child never waited on, which init reaps once this process
/// exits, and which the daemon would keep as a zombie.
fn project_open(name: Option<String>) -> Result<()> {
    let Some(name) = name else {
        if let Some(cache) = github::cache_path() {
            github::spawn_refresh(&cache);
        }
        anyhow::ensure!(
            delegate_to_daemon(ipc::Request::Projects),
            "The niri-tasks daemon is not running, so there is no project list. Start it with `systemctl --user start niri-tasks`."
        );
        return Ok(());
    };
    open_project(&name)
}

/// Open `selected`, a row of the project list or a name typed there: a
/// folder as it is, a GitHub row cloned first, a new name made a folder
/// first. Then back to the project's workspace if it has one, else onto the
/// last workspace on this output, named for it, with its programs started.
fn open_project(selected: &str) -> Result<()> {
    let (projects_dir, names) = project::list()?;
    let remote = github::cache_path()
        .map(|cache| github::remote_only(&github::read_cache(&cache), &names))
        .unwrap_or_default();

    let name = match github::choose(selected, &names, &remote) {
        // … the old match, unchanged …
    };

    let dir = projects_dir.join(&name);
    // … the rest of the old project_open, unchanged …
}
```

The `Picker` import stays until Task 5 (`prompt_for_name` still uses it).

`src/github.rs` docs:
- Module doc first paragraph: "`niritasks project open` lists the folders in `~/Projects`; underneath them it also offers…" → "The task panel's project list (Mod+Alt+W) lists the folders in `~/Projects`; underneath them it also offers…". "while the popup waits" → "while the list waits". "Each open reads the cache and fires a background refresh" → "Each `project open` fires a background refresh and the list reads the cache".
- `MARKER`: "The suffix that tells a GitHub row apart from a local folder in the picker. fuzzel echoes the whole accepted row back, so the suffix is also how the selection is recognised as a repo to clone." → "The suffix that tells a GitHub row apart from a local folder on the project list. Picking a row runs `project open` with the whole row, so the suffix is also how the selection is recognised as a repo to clone."
- `mark` / `unmark`: "picker row" → "project list row". `read_cache`, `spawn_refresh`, `remote_only`: "the picker" → "the project list".

`tests/e2e-panel.sh`, after the `# ─── nothing pending shows nothing ───` block's closing `fi`:

```bash
# ─── Mod+Alt+W's project list, with no task ──────────────────────────────────
# `project open` shows the project list on a workspace with no tasks, where
# the panel had nothing to show, and Escape hides it again. Nothing is
# picked: the open would run in the nested niri's spawn, on the real
# ~/Projects. Its own XDG_CACHE_HOME keeps its gh refresh off the real cache.
if command -v wtype >/dev/null && [ -d "$HOME/Projects" ]; then
    "${NENV[@]}" XDG_CACHE_HOME="$SB/cache" "$NIRITASKS" project open >/dev/null 2>&1
    sleep 1
    shot open_list || { summary; exit 1; }
    if same baseline open_list; then
        bad "project open drew nothing on a workspace with no tasks"
    else
        ok "project open shows the project list with no task on the workspace"
    fi
    "${NENV[@]}" wtype -k Escape
    sleep 1
    shot open_closed || { summary; exit 1; }
    if same baseline open_closed; then
        ok "Escape closes the project list, leaving nothing drawn"
    else
        read -r x0 x1 y0 y1 < <(measure open_closed)
        bad "after Escape from the project list something is still drawn (columns ${x0}-${x1}, rows ${y0}-${y1})"
    fi
else
    skip "Mod+Alt+W's project list (needs wtype and ~/Projects)"
fi
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS (README and llms.txt still mention `niritasks project open`, so their coverage tests pass).
Run: `bash tests/e2e-panel.sh`
Expected: the two new lines `ok`, every other line as before. If no display can run the nested niri, say so in the report rather than claiming it passed.
Then `cargo build 2>&1 | grep -c warning` prints `0`.

Check by hand, with the build installed (`bash install.sh`): Mod+Alt+W on a workspace with no tasks shows "Open a project" in the middle of the screen; typing narrows it; Enter on a folder goes to that project's workspace (or names the last one and opens the terminal); a GitHub row clones; a new name makes `~/Projects/<name>`. If you can't reach a display, say so in the report.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/github.rs tests/e2e-panel.sh
git commit -m "feat: open projects from the panel's project list

project open with no name shows the panel's list in place of fuzzel;
with one, it opens that project as a pick did.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Remove free workspace naming

**Files:**
- Modify: `src/main.rs` (imports ~12-17, `Command::Workspace` doc ~57, `WorkspaceCommand` ~220-228, `workspace_command` ~481-508, `prompt_for_name` ~510-522, tests)
- Modify: `src/lib.rs:46`, `src/daemon.rs` (test `no_cards_says_why`)
- Modify: `niri/niri-tasks.kdl`, `install.sh:142`, `tests/e2e-tag.sh:140`, `.claude/skills/workspace-tasks/SKILL.md:47`

**Interfaces:**
- Consumes: nothing new.
- Produces: `niritasks workspace` has only `default`.

- [ ] **Step 1: Write the failing test**

In `src/main.rs`'s tests:

```rust
    /// A workspace gets its name by opening a project on it, so there is no
    /// naming one freely; workspace 1's startup name stays.
    #[test]
    fn workspaces_are_named_only_by_opening_a_project() {
        assert!(Cli::try_parse_from(["niritasks", "workspace", "rename"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "workspace", "new"]).is_err());
        assert!(Cli::try_parse_from(["niritasks", "workspace", "default"]).is_ok());
    }
```

In `src/daemon.rs`, `no_cards_says_why` uses the new refusal:

```rust
        assert_eq!(
            no_cards_text(Err(anyhow::anyhow!("This workspace has no name — Mod+Alt+W opens a project on a named one."))),
            "This workspace has no name — Mod+Alt+W opens a project on a named one."
        );
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --bin niritasks workspaces_are_named_only_by_opening_a_project`
Expected: FAIL, `workspace rename` parses.

- [ ] **Step 3: Implement**

`src/main.rs`:
- `WorkspaceCommand` keeps only `Default` and its doc line.
- The `Command::Workspace` doc `/// Workspace naming` → `/// Name workspace 1 at startup; every other name comes from opening a project`.
- `workspace_command` becomes:

```rust
fn workspace_command(cmd: WorkspaceCommand) -> Result<()> {
    match cmd {
        WorkspaceCommand::Default => niri_tasks::workspace_default(),
    }
}
```

- Delete `prompt_for_name` and its doc comment, and `picker::Picker` from the `use niri_tasks::{…}` list. Leave `text` if the compiler still sees it used (it is, in `task_command`).

`src/lib.rs:46`: `"This workspace has no name — name it with Mod+Alt+Ctrl+W first."` → `"This workspace has no name — Mod+Alt+W opens a project on a named one."`

`niri/niri-tasks.kdl`:
- Delete the `Mod+Alt+Ctrl+W` bind and its three comment lines.
- The Mod+Alt+W comment becomes:

```kdl
    // The task panel's project list, in the middle of the screen: the
    // ~/Projects folders, then the GitHub repos not cloned yet. Picking one
    // (or typing a new name to make one) puts it on its own named workspace,
    // and opens a terminal on that workspace's herdr session
    // (`herdr --session <workspace>`) and VS Code in it. herdr reattaches the
    // session if it exists and creates it in the project folder if not. The
    // editor is skipped when `code` is not on $PATH — it is a nicety here,
    // not a requirement. Opening a project is the only way a workspace gets
    // a name, bar workspace 1's at startup. W for workspace.
```

- The paragraph before `binds {` ("One of the binds below is guarded…" through "…not safe for the pickers.") becomes:

```kdl
// One of the binds below is guarded against opening twice, and only one needs
// to be.
//
// The task panel and its project list are the daemon's own surfaces, one per
// monitor, so pressing their keys again shows the same one again rather than a
// second. The task box is the exception: it is the GTK window in
// src/taskbox.rs, a new one each time, so pressing Mod+Alt+T twice stacked two
// boxes. It spawns nothing that outlives it — Add & refine's refine goes
// through niri's spawn, not as its child — so a flock held across it is
// released when it closes.
```

`install.sh:142`: `Mod+Alt+W open a project workspace, Mod+Alt+Ctrl+W rename it.` → `Mod+Alt+W open a project workspace.`

`tests/e2e-tag.sh:140`: the echo becomes `"the focused workspace has no name — Mod+Alt+W opens a project on a named one" >&2`.

`.claude/skills/workspace-tasks/SKILL.md:47`: `Tell the user to name it with \`Mod+Alt+Ctrl+W\`` → `Tell the user to open a project with \`Mod+Alt+W\`, which gives its workspace a name`.

Then `grep -rn "Ctrl+W\|workspace rename\|workspace new\|prompt_for_name" --exclude-dir=target --exclude-dir=.git --exclude-dir=docs .` must list only README.md, llms.txt and CONTEXT.md lines, which Task 6 rewrites.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS. Then `cargo build 2>&1 | grep -c warning` prints `0`.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/lib.rs src/daemon.rs niri/niri-tasks.kdl install.sh tests/e2e-tag.sh .claude/skills/workspace-tasks/SKILL.md
git commit -m "feat!: name workspaces only by opening a project

workspace rename, workspace new and Mod+Alt+Ctrl+W go: a workspace's
name is always a ~/Projects folder, which keeps every tag a project.

BREAKING CHANGE: niritasks workspace rename and workspace new are gone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Docs, and the fuzzel follow-up

**Files:**
- Modify: `README.md` (intro ~10-12, keybinds ~21-22, Requirements ~168-172, Commands block ~260-263, Notes ~320-335, Testing ~403)
- Modify: `llms.txt` (rows ~48-51)
- Modify: `CONTEXT.md` (Project list ~126-134)

**Interfaces:** none.

- [ ] **Step 1: README**

- Intro: "Which tasks you see goes purely off the workspace name, which `niritasks project open` sets to the folder name. `~/Projects` is only ever read to offer folders to pick from — opening one, or moving a task to another workspace." stays true; add a sentence after it: "Opening a project is the only way a workspace gets a name, bar workspace 1, which starts as `general`."
- Keybinds: the `Mod+Alt+W` row becomes `| \`Mod+Alt+W\` | The task panel's project list, in the middle of the screen: the \`~/Projects\` folders, then your GitHub repos not cloned yet. Type to narrow it; Enter or a click puts the project on its own named workspace (cloning a repo first) and opens a terminal and an editor in it. A name that matches nothing makes \`~/Projects/<name>\`. Escape clears what you typed, then closes it |`. Delete the `Mod+Alt+Ctrl+W` row.
- In the Mod+Alt+Ctrl+T row: "With no tasks at all, finished ones included, or no name, a notification says so instead" stays.
- Requirements: "`fzf` ranks what you type on Move to workspace's project list" → "`fzf` ranks what you type on the project list".
- Commands block: delete the `workspace new` and `workspace rename` lines; the `project open` line becomes two:

```
niritasks project open             # the panel's project list: pick a ~/Projects folder or GitHub repo
                                   #   onto its own named workspace, or type a new name (Mod+Alt+W)
niritasks project open <name>      # open that one, cloning or making it first: what a pick runs
```

- Notes: delete the paragraph "The project picker (Mod+Alt+W) uses a stripped-down fuzzel theme…" and the `layer-rule` kdl block that belongs to it, down to the end of that block.
- Testing (the e2e-panel paragraph near line 403): after "…while Enter on a folder moves the task there," add "`project open` on a workspace with no tasks shows the project list and Escape closes it,".

- [ ] **Step 2: llms.txt**

Delete the `workspace new` and `workspace rename` rows. The `project open` row becomes two:

```
| `niritasks project open` | Show the task panel's project list with the keyboard (Mod+Alt+W): the `~/Projects` folders, then the account's GitHub repos not cloned yet. Picking a row runs `project open <row>`; a typed name matching nothing makes `~/Projects/<name>`. Exit 1 with no daemon | GUI |
| `niritasks project open <name>` | Open that `~/Projects` folder on its own named workspace (cloning a GitHub row, or making a new folder, first): back to its workspace if it has one, else the last workspace on the focused monitor, named for it, with a terminal there, and VS Code too when `code` is on `$PATH` | GUI (windows), follows focus |
```

- [ ] **Step 3: CONTEXT.md**

The **Project list** entry becomes:

```markdown
**Project list**:
What the task panel shows in place of the task cards for picking a
`~/Projects` folder, drawn like task cards under a line saying what it is
for, with a text field above them that narrows them to fzf's matches as you
type. Up and Down move the highlight and Enter or a click picks. Move to
workspace lists every folder but the task's own workspace's, and moves the
task there; Escape clears the text, then goes back to the cards on the same
card. Mod+Alt+W lists every folder and then the GitHub repos not cloned
yet, and opens the one picked on its own named workspace, cloning a repo
first; a typed name matching nothing makes a new folder. It shows with no
task and on an unnamed workspace, and Escape clears the text, then closes
the panel.
_Avoid_: picker, menu, folder list
```

Then grep CONTEXT.md for "rename" and "name it": any line saying a workspace can be named freely changes to say a workspace is named by opening a project.

- [ ] **Step 4: Run the tests**

Run: `cargo test`
Expected: PASS, `readme_commands_block_runs_every_subcommand_and_flag` and `llms_txt_runs_every_subcommand_and_flag` included.

- [ ] **Step 5: Commit**

```bash
git add README.md llms.txt CONTEXT.md
git commit -m "docs: describe opening projects from the panel's project list

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 6: List what still uses or names fuzzel, and file the follow-up**

Run:

```bash
grep -rni fuzzel --exclude-dir=target --exclude-dir=.git --exclude-dir=docs . | sort
```

Expected, at least: `src/picker.rs` (the whole module, now unused by the CLI), `src/lib.rs` (`pub mod picker`), `fuzzel/picker.ini`, `install.sh` (the `picker.ini` link and `fuzzel` in the dep check), `README.md` (Requirements and the install paragraph's "the fuzzel picker theme"), `llms.txt` (line 5, "wraps … fuzzel"), `niri/niri-tasks.kdl` (any comment left), `CONTEXT.md` (if any), and code comments such as `src/panel/surface.rs` ("as fuzzel does") and `src/panel/state.rs` ("as fuzzel echoed"). Also `grep -rn "picker" src/ | grep -v "^src/picker.rs"` for comments that still say "the picker" for the Mod+Alt+W list.

File it, from this worktree's herdr pane so it lands on this workspace's tag:

```bash
niritasks task add "chore: Remove fuzzel and its picker leftovers"
```

Take the uuid it prints, then add one note per place found, as `file: what to do`, for example:

```bash
niritasks task note <uuid> "src/picker.rs and lib.rs pub mod picker: delete the module, nothing calls Picker"
niritasks task note <uuid> "fuzzel/picker.ini and install.sh: drop the theme link and fuzzel from the dep check"
```

…and one note per remaining README, llms.txt, kdl, CONTEXT.md and code-comment hit. Tell the user the task's uuid and paste them the list.

---

## Self-review notes

- Spec coverage: IPC request and no-task mode (Tasks 2–3); local, then GitHub, then a new name (Task 2's `new_folder`, Task 3's `rows_to_open`, Task 4's `open_project`); the post-pick logic unchanged behind `project open <name>` (Task 4); rename/new/bind/`prompt_for_name` gone, `default` kept (Task 5); CONTEXT.md, README, llms.txt, kdl comments (Tasks 5–6); tests (every task); fuzzel inventory filed as a `chore:` task (Task 6, Step 6); fuzzel itself kept.
- Names used across tasks: `ProjectList`, `Purpose::{Move, Open}`, `PanelState::{projects, open_projects}`, `ProjectList::{moving, title, keys, new_folder, no_match_text}`, `project::{open_args, open_rows, rows_to_open}`, `ipc::Request::Projects`, `Panel::{open_projects, centre}`, `panel::actions::{OPEN_TITLE, OPEN_KEYS, TYPE_A_NAME, GITHUB_ICON, folder_label}`, `daemon::NO_PANEL`.
