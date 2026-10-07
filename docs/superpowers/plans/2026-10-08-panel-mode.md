# Panel Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the task panel's state one `Mode` (Tasks, Ideas, Projects), put the project list in a module of its own with its ranking handed in, and make the panel's hint and the CLI's `project open` decide what a pick does with one function.

**Architecture:** Three moves, in dependency order. First `src/project.rs` gains a `Projects` value with `rows`, `move_destinations`, `choice` and `open`, and `main.rs` and the daemon use it; the `"  (github)"` row marker becomes unnecessary because `choice` already resolves a repo name to Clone. Then `src/panel/projects.rs` owns the `ProjectList` (typed `Row`s, query, ranking through an injected `Matcher`, pick, hint, labels), and the state, surface and daemon are wired to it, deleting the marker. Last, `PanelState` replaces `ideas: bool` and `projects: Option<ProjectList>` with `mode: Mode`, takes the raw key and picks the mapper itself, and the surface's `render` splits into one draw per mode. Docs follow.

**Tech Stack:** Rust (gtk4, gtk4-layer-shell, clap, anyhow), bash e2e scripts in a nested niri.

**Spec:** `docs/superpowers/specs/2026-10-08-panel-mode-design.md` (commit eb84c00). The spec is the authority; this plan argues from it.

## Global Constraints

- Commit messages are Conventional Commits, `<type>(<scope>): <summary>`, imperative, lowercase, no full stop, subject ≤ 72 chars, body wrapped at 72 saying what and why. End every commit message with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- `cargo test` passes and `cargo build` has no warnings after every task. Two tests in `src/main.rs` check README.md's `## Commands` block and llms.txt against clap; no subcommand changes here, so they stay green.
- Work in the `panel-mode` worktree branch (`/home/paul/.worktrees/niri-tasks/panel-mode`); `finish-worktree` lands it on `main` at the end.
- **Never run `install.sh`, `cargo install`, or `systemctl --user restart niri-tasks`** from the worktree: that swaps the user's live daemon onto this branch's build. Run the e2e scripts only as `NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh` after `cargo build`, so they test this worktree's binary and not the installed one. If the machine cannot run them (no Wayland session, no python3-pil, or they fail on an inherited `HERDR_SESSION`, which is a known harness problem, task a030807c), say so in the report rather than claiming they ran.
- The panel acts only by spawning `niritasks` commands (`Effect::Spawn`); it never calls into the library to change a task or open a project. Keep that.
- Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Comments are full sentences that say why, in the house style; docs on every new pub item.
- Names: the three modes are `Tasks`, `Ideas` and `Projects`. The user-facing tab stays "Ideas" everywhere.
- Line numbers below are anchors from `main` at eb84c00; match on the quoted text.

---

### Task 1: `project::Projects`, `Row` and `Choice`

**Files:**
- Modify: `src/project.rs` (after `move_destinations`, around line 78; tests at the end)

**Interfaces:**
- Consumes: `list()`, `resolve()`, `move_destinations()` in `project.rs`; `github::cache_path`, `read_cache`, `remote_only`.
- Produces: `pub struct Projects { pub dir: PathBuf, pub local: Vec<String>, pub remote: Vec<String> }`, `pub enum Row { Folder(String), Repo(String) }` with `fn name(&self) -> &str`, `pub enum Choice { Open(String), Create(String), Clone(String), Rejected(String), Nothing }`, and `impl Projects { fn load() -> Result<Projects>; fn rows(&self) -> Vec<Row>; fn move_destinations(&self, current_tag: &str) -> Vec<Row>; fn choice(&self, typed: &str) -> Choice }`. Nothing calls them yet.

- [ ] **Step 1: Write the failing tests**

Add to `src/project.rs`'s `mod tests`:

```rust
    fn projects(local: &[&str], remote: &[&str]) -> Projects {
        Projects {
            dir: std::path::PathBuf::from("/home/x/Projects"),
            local: local.iter().map(|s| s.to_string()).collect(),
            remote: remote.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// The list Mod+Alt+W shows: the folders in their order, then the repos.
    #[test]
    fn rows_are_folders_then_uncloned_repos() {
        let p = projects(&["alpha", "hansard"], &["convo"]);
        assert_eq!(
            p.rows(),
            vec![Row::Folder("alpha".into()), Row::Folder("hansard".into()), Row::Repo("convo".into())]
        );
        assert_eq!(p.rows()[2].name(), "convo");
    }

    /// A task can move to a folder, never to a repo that is not one yet, and
    /// never to the folder it is already on.
    #[test]
    fn move_destinations_are_other_folders_only() {
        let p = projects(&["alpha", "niri-tasks"], &["convo"]);
        assert_eq!(p.move_destinations("niri_tasks"), vec![Row::Folder("alpha".into())]);
    }

    #[test]
    fn a_folder_name_opens_it() {
        assert_eq!(projects(&["alpha"], &[]).choice("alpha"), Choice::Open("alpha".into()));
    }

    #[test]
    fn a_new_name_makes_a_folder() {
        assert_eq!(projects(&["alpha"], &["convo"]).choice("brand new thing"), Choice::Create("brand-new-thing".into()));
    }

    #[test]
    fn a_repo_name_clones_it() {
        assert_eq!(projects(&["alpha"], &["convo"]).choice("convo"), Choice::Clone("convo".into()));
    }

    /// The case the panel's hint got wrong: "my repo" normalises to a repo
    /// name, so Enter clones, and the hint has to say so.
    #[test]
    fn a_spaced_variant_of_a_repo_name_clones_it() {
        assert_eq!(projects(&["alpha"], &["my-repo"]).choice("my repo"), Choice::Clone("my-repo".into()));
    }

    #[test]
    fn a_local_folder_wins_over_a_same_named_repo() {
        assert_eq!(projects(&["alpha"], &["alpha"]).choice("alpha"), Choice::Open("alpha".into()));
    }

    #[test]
    fn nothing_and_rejections_pass_through() {
        assert_eq!(projects(&[], &[]).choice("  "), Choice::Nothing);
        assert!(matches!(projects(&[], &[]).choice("../etc"), Choice::Rejected(_)));
        assert!(matches!(projects(&[], &[]).choice(".hidden"), Choice::Rejected(_)));
    }
```

Run: `cargo test --lib project::`
Expected: FAIL to compile, `Projects`, `Row`, `Choice` not found.

- [ ] **Step 2: Implement**

Add to `src/project.rs`, after `move_destinations` (keep that free function; `destination` and `move_task` use it):

```rust
/// The projects a name can mean, read once per use: the `~/Projects`
/// folders, and the account's GitHub repos not cloned yet, from the cache
/// `project open` refreshes. One value, so the panel's list, its hint and
/// `project open` all read the same two lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projects {
    /// `~/Projects`.
    pub dir: std::path::PathBuf,
    /// Its folders, sorted, dotfiles left out.
    pub local: Vec<String>,
    /// The repos worth offering: not a folder already, not a dotfile, in
    /// `gh`'s most-recently-pushed order.
    pub remote: Vec<String>,
}

/// One row of the project list: a folder in `~/Projects`, or a GitHub repo
/// not cloned yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Folder(String),
    Repo(String),
}

impl Row {
    /// The name a pick spawns, bare: `project open` tells a repo from a
    /// folder by [`Projects::choice`], not by how the row read.
    pub fn name(&self) -> &str {
        match self {
            Row::Folder(name) | Row::Repo(name) => name,
        }
    }
}

/// What `project open` does with a name, and so what Enter on the project
/// list will do with what is typed: one decision for the hint and the act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// A folder that exists: open its workspace.
    Open(String),
    /// A new name, normalised and allowed: make the folder, then open it.
    Create(String),
    /// A GitHub repo not cloned yet: clone it, then open it.
    Clone(String),
    /// Normalised into something that must not become a directory.
    Rejected(String),
    /// Nothing usable was typed.
    Nothing,
}

impl Projects {
    /// Read `~/Projects` and the GitHub cache. The cache missing is an empty
    /// repo list, never an error: the list works without GitHub.
    pub fn load() -> Result<Projects> {
        let (dir, local) = list()?;
        let cached = crate::github::cache_path()
            .map(|cache| crate::github::read_cache(&cache))
            .unwrap_or_default();
        let remote = crate::github::remote_only(&cached, &local);
        Ok(Projects { dir, local, remote })
    }

    /// The project list Mod+Alt+W shows: the folders, then the repos.
    pub fn rows(&self) -> Vec<Row> {
        self.local
            .iter()
            .cloned()
            .map(Row::Folder)
            .chain(self.remote.iter().cloned().map(Row::Repo))
            .collect()
    }

    /// The folders a task on `current_tag` can move to: every folder but the
    /// one it is on. Never a repo: a repo is not a workspace until it is
    /// cloned.
    pub fn move_destinations(&self, current_tag: &str) -> Vec<Row> {
        move_destinations(&self.local, current_tag).into_iter().map(Row::Folder).collect()
    }

    /// What `typed` means, a picked row's name or text: [`resolve`] against
    /// the folders, then a creatable name that is one of the repos clones
    /// instead, because an empty folder shadowing your own repo is never
    /// what you want.
    pub fn choice(&self, typed: &str) -> Choice {
        match resolve(typed, &self.local) {
            Resolved::Existing(name) => Choice::Open(name),
            Resolved::Create(name) if self.remote.contains(&name) => Choice::Clone(name),
            Resolved::Create(name) => Choice::Create(name),
            Resolved::Nothing => Choice::Nothing,
            Resolved::Rejected(why) => Choice::Rejected(why),
        }
    }
}
```

Run: `cargo test --lib project::`
Expected: PASS (the new tests and the existing ones). `cargo build` may warn that nothing uses the new items; that is expected until Task 2 and is why this task's commit is allowed a dead-code warning. Silence it for now with `#[allow(dead_code)]` on nothing: instead, confirm the warning text names only `Projects`, `Row`, `Choice` and their methods.

- [ ] **Step 3: Commit**

```bash
git add src/project.rs
git commit -m "feat(project): add Projects, Row and Choice for one definition of a pick

The panel's hint and project open each decided what a typed name
means; Projects::choice is the one decision both will use, and a Row
carries whether a list entry is a folder or a repo as a type rather
than a string suffix.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: `Projects::open`, and `project open` on it

**Files:**
- Modify: `src/project.rs` (add `open` and a private `start`), `src/main.rs` (`project_open` 413-424, delete `open_project` 426-485 and `spawn_startup` 487-492, the `WorkspaceReferenceArg` import if unused)

**Interfaces:**
- Consumes: Task 1's `Projects`, `Choice`; `github::unmark` (temporarily, removed in Task 4), `github::clone`, `notify::project`, `niri::{workspaces, find_workspace_by_name, focus_workspace, window_count, last_workspace_idx, set_workspace_name, spawn}`, `startup_commands`.
- Produces: `impl Projects { pub fn open(&self, choice: Choice) -> Result<()> }`.

- [ ] **Step 1: Move `open_project`'s body into the library**

Add to `impl Projects` in `src/project.rs`:

```rust
    /// Open what `choice` names: make the folder or clone the repo first,
    /// then back to the project's workspace if it has one, else the last
    /// workspace on the focused output, named for it, with its programs
    /// started. Nothing for [`Choice::Nothing`]; [`Choice::Rejected`] is the
    /// error it carries.
    pub fn open(&self, choice: Choice) -> Result<()> {
        let name = match choice {
            Choice::Nothing => return Ok(()),
            Choice::Rejected(why) => anyhow::bail!(why),
            Choice::Open(name) => name,
            Choice::Create(name) => {
                std::fs::create_dir(self.dir.join(&name))
                    .with_context(|| format!("Could not create {}/{name}", self.dir.display()))?;
                crate::notify::project(&format!("Created {}/{name}", self.dir.display()));
                name
            }
            Choice::Clone(name) => {
                // The clone blocks this command, not the compositor; the
                // notifications are what says it started and finished.
                crate::notify::project(&format!("Cloning {name}…"));
                crate::github::clone(&name, &self.dir.join(&name))?;
                crate::notify::project(&format!("Cloned {}/{name}", self.dir.display()));
                name
            }
        };
        let dir = self.dir.join(&name);

        let all = crate::niri::workspaces()?;
        if let Some(ws) = crate::niri::find_workspace_by_name(&all, &name) {
            // Already opened once: go back to its workspace rather than end
            // up with two sharing a name. Re-picking means "take me back",
            // not "another terminal", so start things only if it is empty.
            let id = ws.id;
            crate::niri::focus_workspace(niri_ipc::WorkspaceReferenceArg::Name(name.clone()))?;
            if crate::niri::window_count(id)? == 0 {
                start(&dir, &name)?;
            }
        } else {
            let focused = all.iter().find(|w| w.is_focused).context("no focused workspace")?;
            let output = focused.output.clone().unwrap_or_default();
            let last = crate::niri::last_workspace_idx(&all, &output).context("no workspaces on output")?;
            crate::niri::focus_workspace(niri_ipc::WorkspaceReferenceArg::Index(last))?;
            crate::niri::set_workspace_name(&name, None)?;
            start(&dir, &name)?;
        }
        Ok(())
    }
```

and, as a free function in `project.rs`:

```rust
/// Start a project workspace's programs: its herdr session's terminal, and
/// the editor if installed. The name is set before this runs, which is what
/// puts the windows on the right workspace: niri spawns onto the focused one.
fn start(dir: &std::path::Path, workspace: &str) -> Result<()> {
    for command in startup_commands(dir, workspace) {
        crate::niri::spawn(command)?;
    }
    Ok(())
}
```

(`project.rs` already imports `anyhow::{Context, Result}`; `niri_ipc` is a dependency of the crate.)

- [ ] **Step 2: The CLI uses it**

In `src/main.rs`, replace the tail of `project_open` (the line `open_project(&name)`) and delete `open_project` and `spawn_startup` entirely. `project_open` becomes:

```rust
fn project_open(name: Option<String>) -> Result<()> {
    let Some(name) = name else {
        if let Some(cache) = github::cache_path() {
            github::spawn_refresh(&cache);
        }
        return ipc::send(&ipc::Request::Projects);
    };
    let projects = project::Projects::load()?;
    // Until the panel sends bare names (the next tasks), a GitHub row still
    // arrives with its "  (github)" marker on.
    let choice = match github::unmark(&name) {
        Some(repo) => project::Choice::Clone(repo.to_string()),
        None => projects.choice(&name),
    };
    projects.open(choice)
}
```

Keep `project_open`'s existing doc comment. Remove `use niri_ipc::WorkspaceReferenceArg;` and anything else `cargo build` reports unused (`notify` may still be used elsewhere in main.rs; check).

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4`
Expected: no warnings (Task 1's dead-code warning is gone now that `Projects` has callers); tests pass.

- [ ] **Step 3: Check the open path by hand if the machine allows**

With niri running: `cargo build && ./target/debug/niritasks project open -- <an existing ~/Projects folder>` should focus that folder's workspace (or name one and open the terminal). Do not run it for a Create or Clone name. Report what happened or why it was skipped.

- [ ] **Step 4: Commit**

```bash
git add src/project.rs src/main.rs
git commit -m "refactor(project): move opening a project into the library

The create, clone, name-the-workspace and spawn half of project open
lived in main.rs beside the CLI arm. As Projects::open it is one
function any caller can use, and project open becomes load, choice,
open.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: The project list module, `src/panel/projects.rs`

**Files:**
- Create: `src/panel/projects.rs`
- Modify: `src/panel/mod.rs` (add `pub mod projects;`)

**Interfaces:**
- Consumes: `crate::project::{Choice, Projects, Row, open_args, substring_matches}`, `crate::actions::Action`, `super::keys::step`.
- Produces (all `pub`): `enum Purpose { Move { uuid: String, text: String }, Open }`; `type Matcher<'a> = &'a dyn Fn(&[String], &str) -> Vec<String>`; `enum Picked { Spawn(Vec<String>), Nothing }`; `enum Escaped { Cleared, Closed }`; consts `NO_DESTINATIONS`, `MOVE_KEYS`, `OPEN_TITLE`, `OPEN_KEYS`, `TYPE_A_NAME`, `FOLDER_ICON`, `GITHUB_ICON`; `struct ProjectList` with `new(purpose, rows)`, `purpose()`, `moving()`, `shown() -> &[Row]`, `at()`, `query()`, `narrow(&mut self, query, matcher) -> bool`, `step(&mut self, forward) -> bool`, `escape(&mut self) -> Escaped`, `pick(&self, at, &Projects) -> Picked`, `hint(&self, &Projects) -> Option<String>`, `title()`, `keys()`, and `fn label(row: &Row) -> String`. Nothing uses the module until Task 4, so this task's build has dead-code warnings; confirm they name only this module's items.

- [ ] **Step 1: Write the module with its tests**

Create `src/panel/projects.rs`:

```rust
//! The project list: the `~/Projects` folders, and for Mod+Alt+W the GitHub
//! repos not cloned yet, that the task panel shows in place of the task
//! cards, narrowed by what is typed, and what picking one does. Plain data,
//! like the rest of the panel state: the ranking is handed in, so the daemon
//! ranks with fzf and a test with a substring match, and `surface.rs` only
//! draws what this says.

use crate::actions::Action;
use crate::project::{self, Choice, Projects, Row};

/// What the project list is for, and so what picking a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    /// Move to workspace: the task being moved, by uuid, and its
    /// description, for the line over the folders.
    Move { uuid: String, text: String },
    /// Mod+Alt+W: open the project picked on its own workspace, cloning a
    /// repo or making a folder of a new name, as `project open` does. It has
    /// no task, so it shows with no cards and on an unnamed workspace too:
    /// opening a project is how a workspace gets a name.
    Open,
}

/// Ranks `names` for `query`, best first. The daemon hands in fzf with its
/// substring fallback; tests hand in [`project::substring_matches`].
pub type Matcher<'a> = &'a dyn Fn(&[String], &str) -> Vec<String>;

/// What Enter or a click on the list does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picked {
    /// Run `niritasks` with these arguments: `project open -- <name>`, or
    /// `task move <uuid> <name>`.
    Spawn(Vec<String>),
    /// No row is shown and nothing typed could be one.
    Nothing,
}

/// What Escape did to the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escaped {
    /// Text was typed: it is cleared and every row shows again.
    Cleared,
    /// Nothing was typed: the list is done with.
    Closed,
}

/// What the panel says when Move to workspace finds no folder to move to.
pub const NO_DESTINATIONS: &str = "No other project to move this to.";

/// The footer while Move's list shows: typing narrows it, and these keys act.
pub const MOVE_KEYS: &str = "Type to filter · Up, Down: pick · Enter: move the task there · Esc: clear, then back";

/// The line over the list Mod+Alt+W opens.
pub const OPEN_TITLE: &str = "Open a project";

/// The footer while that list shows: Enter opens, and Escape with nothing
/// typed closes the panel, there being no cards it was opened from.
pub const OPEN_KEYS: &str = "Type to filter · Up, Down: pick · Enter: open it · Esc: clear, then close";

/// What the open list says with no folder to show and nothing typed: an
/// empty `~/Projects`, where the first project has to be made.
pub const TYPE_A_NAME: &str = "Type a name to make a new project";

/// A folder row's icon: Font Awesome's folder, Move to workspace's open one
/// shut.
pub const FOLDER_ICON: &str = "\u{f07b}";

/// A GitHub repo's icon: Font Awesome's GitHub mark, so a repo not cloned
/// yet reads apart from a folder.
pub const GITHUB_ICON: &str = "\u{f09b}";

/// The project list: every row, what is typed, the rows that match it and
/// the one the highlight is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectList {
    purpose: Purpose,
    /// Every row, in list order: folders, then repos.
    rows: Vec<Row>,
    /// What is typed in the field over the list.
    query: String,
    /// The rows matching `query`, best first, or every row with nothing
    /// typed.
    shown: Vec<Row>,
    /// The row in `shown` the highlight is on: Up, Down and Enter's.
    at: usize,
}

impl ProjectList {
    /// A list of `rows` for `purpose`, every row shown and the first
    /// highlighted.
    pub fn new(purpose: Purpose, rows: Vec<Row>) -> ProjectList {
        ProjectList { purpose, shown: rows.clone(), rows, query: String::new(), at: 0 }
    }

    pub fn purpose(&self) -> &Purpose {
        &self.purpose
    }

    /// The task being moved, on Move to workspace's list.
    pub fn moving(&self) -> Option<&str> {
        match &self.purpose {
            Purpose::Move { uuid, .. } => Some(uuid),
            Purpose::Open => None,
        }
    }

    pub fn shown(&self) -> &[Row] {
        &self.shown
    }

    pub fn at(&self) -> usize {
        self.at
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Narrow the list to `query`, ranked by `matcher`, the highlight back
    /// on the top match. False when the text is what it was, as when Escape
    /// has just emptied the field.
    pub fn narrow(&mut self, query: &str, matcher: Matcher) -> bool {
        if self.query == query {
            return false;
        }
        self.query = query.to_string();
        self.shown = if query.is_empty() {
            self.rows.clone()
        } else {
            let names: Vec<String> = self.rows.iter().map(|r| r.name().to_string()).collect();
            matcher(&names, query)
                .iter()
                .filter_map(|name| self.rows.iter().find(|r| r.name() == name).cloned())
                .collect()
        };
        self.at = 0;
        true
    }

    /// Up or Down, stopping at the ends. False with nothing shown.
    pub fn step(&mut self, forward: bool) -> bool {
        if self.shown.is_empty() {
            return false;
        }
        self.at = super::keys::step(self.at, self.shown.len(), forward);
        true
    }

    /// Escape: with text typed, clear it and show every row again; with
    /// none, the list is done with.
    pub fn escape(&mut self) -> Escaped {
        if self.query.is_empty() {
            return Escaped::Closed;
        }
        self.query.clear();
        self.shown = self.rows.clone();
        self.at = 0;
        Escaped::Cleared
    }

    /// What the typed text means when the open list shows nothing: a new
    /// name, a folder the plain-text ranking missed ("my project" for
    /// "my-project"), or a repo. None on Move, and while anything shows.
    fn typed(&self, projects: &Projects) -> Option<Choice> {
        if self.purpose != Purpose::Open || !self.shown.is_empty() {
            return None;
        }
        Some(projects.choice(&self.query))
    }

    /// The name a pick at `at` means: the shown row there, or, on the open
    /// list with nothing shown, what the typed text names.
    fn picked_name(&self, at: usize, projects: &Projects) -> Option<String> {
        if let Some(row) = self.shown.get(at) {
            return Some(row.name().to_string());
        }
        match self.typed(projects)? {
            Choice::Open(name) | Choice::Create(name) | Choice::Clone(name) => Some(name),
            Choice::Rejected(_) | Choice::Nothing => None,
        }
    }

    /// Enter or a click at `at`: the `niritasks` words that open the project
    /// or move the task, by purpose. `projects` is what the rows came from,
    /// so a typed name means here what `project open` will take it to mean.
    pub fn pick(&self, at: usize, projects: &Projects) -> Picked {
        let Some(name) = self.picked_name(at, projects) else {
            return Picked::Nothing;
        };
        Picked::Spawn(match &self.purpose {
            Purpose::Open => project::open_args(&name),
            Purpose::Move { uuid, .. } => {
                let mut args = Action::Move.args(uuid);
                args.push(name);
                args
            }
        })
    }

    /// The line under the title when the open list shows nothing: what
    /// Enter will do with the text, decided by the same [`Projects::choice`]
    /// that `project open` will run, or, with nothing typed and no row at
    /// all, to type a name. None on Move, and while anything shows.
    pub fn hint(&self, projects: &Projects) -> Option<String> {
        Some(match self.typed(projects)? {
            Choice::Clone(name) => format!("Enter clones {name} from GitHub"),
            Choice::Create(name) => format!("Enter makes ~/Projects/{name}"),
            Choice::Open(name) => format!("Enter opens ~/Projects/{name}"),
            Choice::Rejected(why) => why,
            Choice::Nothing => TYPE_A_NAME.to_string(),
        })
    }

    /// The line over the rows: which task is moving, or that a project is
    /// being opened.
    pub fn title(&self) -> String {
        match &self.purpose {
            Purpose::Move { text, .. } => format!("{}: {text}", Action::Move.label(false)),
            Purpose::Open => OPEN_TITLE.to_string(),
        }
    }

    /// The footer under the list: the keys that act on it.
    pub fn keys(&self) -> &'static str {
        match self.purpose {
            Purpose::Move { .. } => MOVE_KEYS,
            Purpose::Open => OPEN_KEYS,
        }
    }

    /// A row as its card reads: the icon, then the name. The two spaces are
    /// the gap a task card leaves after its icon.
    pub fn label(row: &Row) -> String {
        match row {
            Row::Repo(name) => format!("{GITHUB_ICON}  {name}"),
            Row::Folder(name) => format!("{FOLDER_ICON}  {name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projects(local: &[&str], remote: &[&str]) -> Projects {
        Projects {
            dir: std::path::PathBuf::from("/home/x/Projects"),
            local: local.iter().map(|s| s.to_string()).collect(),
            remote: remote.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn substring(names: &[String], query: &str) -> Vec<String> {
        project::substring_matches(names, query)
    }

    fn folders(names: &[&str]) -> Vec<Row> {
        names.iter().map(|n| Row::Folder(n.to_string())).collect()
    }

    fn moving(rows: &[&str]) -> ProjectList {
        ProjectList::new(Purpose::Move { uuid: "a".into(), text: "task a".into() }, folders(rows))
    }

    fn opening(p: &Projects) -> ProjectList {
        ProjectList::new(Purpose::Open, p.rows())
    }

    #[test]
    fn a_new_list_shows_every_row_with_the_first_highlighted() {
        let list = moving(&["x", "y"]);
        assert_eq!(list.shown(), folders(&["x", "y"]));
        assert_eq!(list.at(), 0);
        assert_eq!(list.query(), "");
    }

    #[test]
    fn narrowing_ranks_the_matches_and_goes_back_to_the_top() {
        let mut list = moving(&["alpha", "beta", "gamma"]);
        assert!(list.step(true));
        assert!(list.narrow("a", &substring));
        assert_eq!(list.shown(), folders(&["alpha", "beta", "gamma"]));
        assert!(list.narrow("et", &substring));
        assert_eq!(list.shown(), folders(&["beta"]));
        assert_eq!(list.at(), 0);
    }

    #[test]
    fn the_same_text_again_is_no_change() {
        let mut list = moving(&["x", "y"]);
        assert!(!list.narrow("", &substring));
        assert!(list.narrow("y", &substring));
        assert!(!list.narrow("y", &substring));
    }

    #[test]
    fn an_empty_query_shows_every_row_without_asking_the_matcher() {
        let mut list = moving(&["x", "y"]);
        list.narrow("x", &substring);
        let never = |_: &[String], _: &str| -> Vec<String> { panic!("the matcher is not asked for an empty query") };
        assert!(list.narrow("", &never));
        assert_eq!(list.shown(), folders(&["x", "y"]));
    }

    #[test]
    fn stepping_stops_at_the_ends_and_does_nothing_on_nothing() {
        let mut list = moving(&["x", "y"]);
        assert!(!list.step(false));
        assert_eq!(list.at(), 0);
        assert!(list.step(true));
        assert!(list.step(true));
        assert_eq!(list.at(), 1);
        list.narrow("zz", &substring);
        assert!(!list.step(true));
    }

    #[test]
    fn escape_clears_typed_text_first_and_closes_second() {
        let mut list = moving(&["x", "y"]);
        list.narrow("y", &substring);
        assert_eq!(list.escape(), Escaped::Cleared);
        assert_eq!(list.shown(), folders(&["x", "y"]));
        assert_eq!(list.query(), "");
        assert_eq!(list.escape(), Escaped::Closed);
    }

    #[test]
    fn a_pick_on_move_moves_the_task_to_the_shown_row() {
        let p = projects(&["x", "y"], &[]);
        let mut list = moving(&["x", "y"]);
        list.step(true);
        assert_eq!(
            list.pick(list.at(), &p),
            Picked::Spawn(vec!["task".into(), "move".into(), "a".into(), "y".into()])
        );
    }

    #[test]
    fn typed_text_is_never_a_pick_on_move() {
        let p = projects(&["x"], &[]);
        let mut list = moving(&["x"]);
        list.narrow("brand new", &substring);
        assert_eq!(list.pick(0, &p), Picked::Nothing);
        assert_eq!(list.hint(&p), None);
    }

    #[test]
    fn a_pick_on_open_opens_the_shown_row_by_its_bare_name() {
        let p = projects(&["alpha"], &["convo"]);
        let list = opening(&p);
        assert_eq!(list.pick(1, &p), Picked::Spawn(project::open_args("convo")));
    }

    #[test]
    fn a_typed_new_name_opens_once_nothing_shows() {
        let p = projects(&["alpha"], &[]);
        let mut list = opening(&p);
        list.narrow("my thing", &substring);
        assert_eq!(list.hint(&p), Some("Enter makes ~/Projects/my-thing".into()));
        assert_eq!(list.pick(0, &p), Picked::Spawn(project::open_args("my-thing")));
    }

    /// The hint and the pick agree with project open: a typed name that is
    /// one of the repos clones, and says so.
    #[test]
    fn a_typed_repo_name_says_it_clones() {
        let p = projects(&["alpha"], &["my-repo"]);
        let mut list = opening(&p);
        list.narrow("my repo", &substring);
        assert_eq!(list.hint(&p), Some("Enter clones my-repo from GitHub".into()));
        assert_eq!(list.pick(0, &p), Picked::Spawn(project::open_args("my-repo")));
    }

    #[test]
    fn a_folder_the_plain_ranking_missed_still_opens() {
        let p = projects(&["my-project"], &[]);
        let mut list = opening(&p);
        list.narrow("my project", &substring);
        assert_eq!(list.hint(&p), Some("Enter opens ~/Projects/my-project".into()));
        assert_eq!(list.pick(0, &p), Picked::Spawn(project::open_args("my-project")));
    }

    #[test]
    fn rejected_text_is_no_pick_and_says_why() {
        let p = projects(&["alpha"], &[]);
        let mut list = opening(&p);
        list.narrow("a/b", &substring);
        assert_eq!(list.pick(0, &p), Picked::Nothing);
        assert!(list.hint(&p).is_some_and(|h| h.contains('/')));
    }

    #[test]
    fn an_empty_projects_folder_asks_for_a_name() {
        let p = projects(&[], &[]);
        let list = opening(&p);
        assert_eq!(list.hint(&p), Some(TYPE_A_NAME.into()));
        assert_eq!(list.pick(0, &p), Picked::Nothing);
    }

    #[test]
    fn no_hint_while_anything_shows() {
        let p = projects(&["alpha"], &[]);
        let list = opening(&p);
        assert_eq!(list.hint(&p), None);
    }

    #[test]
    fn titles_keys_and_labels() {
        let p = projects(&["alpha"], &["convo"]);
        assert_eq!(moving(&["x"]).title(), "Move to workspace: task a");
        assert_eq!(moving(&["x"]).keys(), MOVE_KEYS);
        assert_eq!(opening(&p).title(), OPEN_TITLE);
        assert_eq!(opening(&p).keys(), OPEN_KEYS);
        assert_eq!(ProjectList::label(&Row::Repo("convo".into())), format!("{GITHUB_ICON}  convo"));
        assert_eq!(ProjectList::label(&Row::Folder("alpha".into())), format!("{FOLDER_ICON}  alpha"));
        assert_eq!(moving(&["x"]).moving(), Some("a"));
        assert_eq!(opening(&p).moving(), None);
    }
}
```

Add `pub mod projects;` to `src/panel/mod.rs`.

- [ ] **Step 2: Run the module's tests**

Run: `cargo test --lib panel::projects::`
Expected: PASS, 16 tests. `cargo build` warns about dead code in this module only; confirm nothing else.

- [ ] **Step 3: Commit**

```bash
git add src/panel/projects.rs src/panel/mod.rs
git commit -m "feat(panel): add the project list as a module of its own

The list's rows, what is typed, its ranking and what a pick does were
spread over state.rs, surface.rs and the action-row module. This holds
them together, with the ranking handed in so the daemon can rank with
fzf and a test with a substring match, and with the hint and the pick
both decided by Projects::choice, as project open decides.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: Wire the state, surface and daemon to the module; drop the marker

**Files:**
- Modify: `src/panel/state.rs` (delete `Purpose`, `ProjectList`, `NO_DESTINATIONS` at 70-160; the field `projects`; `open_projects` 372-383, `show_projects` 567-590, `on_query` 592-604, `on_folder` 606-631, `escape_projects` 697-717, `move_folder` 719-725; tests 1716-2160), `src/panel/surface.rs` (`open_projects` 651-654, `rank` 786-803, the `query.connect_changed` handler in `connect_tab_bar` 447-458, `apply`'s `ListProjects` arm 696-705, `render`'s list block 931-942, `folder_card` 1084-1106), `src/panel/actions.rs` (delete 146-190: `MOVE_KEYS`, `FOLDER_ICON`, `OPEN_TITLE`, `OPEN_KEYS`, `TYPE_A_NAME`, `GITHUB_ICON`, `folder_label`, and the `folder_label` test at ~584), `src/daemon.rs` (`Request::Projects` 216-226), `src/github.rs` (delete `MARKER`, `mark`, `unmark`, `Choice`, `choose` and their tests; keep `remote_only`, the cache fns, `clone`), `src/project.rs` (delete `open_rows`, `rows_to_open` and the test `rows_to_open_are_folders_then_uncloned_repos`), `src/main.rs` (the `unmark` adapter from Task 2; the test `opening_a_project_takes_an_optional_name`)

**Interfaces:**
- Consumes: Task 3's module; Task 1's `Projects`, `Row`.
- Produces: `PanelState` API changes: `open_projects(&mut self, projects: Projects) -> Vec<Effect>`; `show_projects(&mut self, uuid: &str, rows: Vec<Row>, projects: Projects) -> Vec<Effect>`; `on_query(&mut self, query: &str, matcher: Matcher) -> Vec<Effect>`; new `pub fn hint(&self) -> Option<String>`; `projects()` still returns `Option<&ProjectList>` (the new type). `Panel::open_projects(self: &Rc<Self>, projects: Projects)`.

- [ ] **Step 1: The state holds the new list**

In `src/panel/state.rs`:
- Delete the `NO_DESTINATIONS` const, `enum Purpose`, `struct ProjectList` and its `impl` (lines 70-160). Add `use super::projects::{Escaped, Matcher, Picked, ProjectList, Purpose, NO_DESTINATIONS};` and `use crate::project::{Projects, Row};`. Re-export for the surface and tests: `pub use super::projects::{ProjectList, Purpose};` is not needed if the surface imports from `projects` directly; prefer the direct import.
- Add a field after `projects`:
```rust
    /// The folders and repos the project list was built from, for its hint
    /// and its pick to mean what `project open` will. Some only while the
    /// list is up.
    loaded: Option<Projects>,
```
- `open_projects`:
```rust
    /// Mod+Alt+W: take the keyboard on the project list for opening a
    /// project, `projects` being the `~/Projects` folders and the GitHub
    /// repos not cloned yet. With no cards too, and on an unnamed workspace.
    /// It takes the panel over as `take_keyboard` does: All, nothing
    /// expanded, armed or focused.
    pub fn open_projects(&mut self, projects: Projects) -> Vec<Effect> {
        self.keyboard = true;
        self.filter = Filter::All;
        self.ideas = false;
        self.expanded = false;
        self.armed = Armed::None;
        self.focus = None;
        self.projects = Some(ProjectList::new(Purpose::Open, projects.rows()));
        self.loaded = Some(projects);
        vec![Effect::Render]
    }
```
- `show_projects(&mut self, uuid: &str, rows: Vec<Row>, projects: Projects)`: as today, but `if rows.is_empty() { return vec![Effect::Notify(NO_DESTINATIONS.into())]; }` and set `self.projects = Some(ProjectList::new(Purpose::Move { uuid: uuid.into(), text }, rows)); self.loaded = Some(projects);`.
- `on_query`:
```rust
    /// The project list's text field changed: narrow the list to `query`,
    /// ranked by `matcher`. Nothing when the text is what it was, as when
    /// Escape empties the field.
    pub fn on_query(&mut self, query: &str, matcher: Matcher) -> Vec<Effect> {
        match self.projects.as_mut() {
            Some(list) if list.narrow(query, matcher) => vec![Effect::Render],
            _ => Vec::new(),
        }
    }
```
- `hint`:
```rust
    /// The project list's line under its title, when it has one: what Enter
    /// will do with the text typed.
    pub fn hint(&self) -> Option<String> {
        self.projects.as_ref()?.hint(self.loaded.as_ref()?)
    }
```
- `on_folder`:
```rust
    pub fn on_folder(&mut self, at: usize) -> Vec<Effect> {
        let (Some(list), Some(loaded)) = (self.projects.as_ref(), self.loaded.as_ref()) else {
            return Vec::new();
        };
        let Picked::Spawn(args) = list.pick(at, loaded) else { return Vec::new() };
        let purpose = list.purpose().clone();
        self.projects = None;
        self.loaded = None;
        match purpose {
            Purpose::Open => {
                let mut effects = self.release();
                effects.push(Effect::Spawn(args));
                effects
            }
            Purpose::Move { uuid, .. } => {
                if let Some(next) = self.neighbour(&uuid) {
                    self.focus = Some(next);
                }
                let mut effects = self.rerender();
                effects.push(Effect::Spawn(args));
                effects
            }
        }
    }
```
(keep its doc comment.)
- `escape_projects`:
```rust
    fn escape_projects(&mut self) -> Vec<Effect> {
        let Some(list) = self.projects.as_mut() else { return Vec::new() };
        match list.escape() {
            Escaped::Cleared => vec![Effect::ClearQuery, Effect::Render],
            Escaped::Closed => {
                // The open list was asked for from anywhere, not from the
                // cards, which there may be none of: it closes the panel.
                if list.purpose() == &Purpose::Open {
                    return self.release();
                }
                self.projects = None;
                self.loaded = None;
                self.rerender()
            }
        }
    }
```
- `move_folder`: `match self.projects.as_mut() { Some(list) if list.step(forward) => vec![Effect::ShowFolder], _ => Vec::new() }`.
- `release()` and `set_cards()`'s `gone` branch also clear `self.loaded = None;` wherever they clear `self.projects`.
- `set_cards`'s `gone` check uses `ProjectList::moving` as before (now a method on the new type, same name).

- [ ] **Step 2: Adapt the state tests**

In `state.rs`'s tests: replace the helper `fn folders(names: &[&str]) -> Vec<String>` with

```rust
    fn projects(local: &[&str], remote: &[&str]) -> crate::project::Projects {
        crate::project::Projects {
            dir: std::path::PathBuf::from("/home/x/Projects"),
            local: local.iter().map(|s| s.to_string()).collect(),
            remote: remote.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn folders(names: &[&str]) -> Vec<Row> {
        names.iter().map(|n| Row::Folder(n.to_string())).collect()
    }

    /// The matcher the tests rank with: fzf is the daemon's.
    fn substring(names: &[String], query: &str) -> Vec<String> {
        crate::project::substring_matches(names, query)
    }
```

and change every call:
- `state.show_projects("a", folders(&["x", "y"]))` → `state.show_projects("a", folders(&["x", "y"]), projects(&["a", "x", "y"], &[]))`; `show_projects("a", Vec::new())` → `show_projects("a", Vec::new(), projects(&[], &[]))`.
- `state.on_query("y", folders(&["y"]))` → `state.on_query("y", &substring)`; any `on_query("zz", Vec::new())` → `on_query("zz", &substring)` (the substring matcher yields nothing for "zz" on these fixtures: check each fixture's names and pick a query that matches nothing, "zz" does). `on_query("", …)` → `on_query("", &substring)`.
- `state.open_projects(folders(&["x", "y  (github)"]))` → `state.open_projects(projects(&["x"], &["y"]))`; `open_projects(folders(&["x"]))` → `open_projects(projects(&["x"], &[]))`; `open_projects(Vec::new())` → `open_projects(projects(&[], &[]))`; `open_projects(folders(&["my-project"]))` → `open_projects(projects(&["my-project"], &[]))`.
- Assertions on `Some(&ProjectList { purpose: …, folders: …, … })` struct literals (around 1736) become assertions through the accessors: `list.purpose()`, `list.shown()`, `list.at()`, `list.query()`.
- The test around 2137 that types `"y"` against `folders(&["y  (github)"])` and expects an `open("y  (github)")` spawn becomes: `open_projects(projects(&[], &["y"]))`, `on_query("y", &substring)`, expect `open("y")`.
- `fn open(row: &str) -> Effect` stays (it wraps `project::open_args`).

Run: `cargo test --lib panel::state::`
Expected: compile errors until the surface is updated too (same crate); go on to step 3 and run both together.

- [ ] **Step 3: The surface and the daemon**

`src/panel/surface.rs`:
- `use super::projects::ProjectList;` and `use crate::project::Projects;`.
- `pub fn open_projects(self: &Rc<Self>, projects: Projects)` passes it straight to the state.
- `rank` keeps its body minus the empty-query shortcut (narrow handles that), and the `query.connect_changed` handler becomes:
```rust
            self.query.connect_changed(move |entry| {
                let Some(p) = weak.upgrade() else { return };
                let query = entry.text().to_string();
                let matcher = |names: &[String], q: &str| p.rank(names, q);
                let effects = p.state.borrow_mut().on_query(&query, &matcher);
                p.apply(effects);
            });
```
(The state borrow and `p.rank`'s `fzf_missing` Cell are different cells, so the closure may run while the state is borrowed.)
- `apply`'s `Effect::ListProjects(uuid)` arm:
```rust
                Effect::ListProjects(uuid) => {
                    // Read on every press, so a folder made since shows.
                    let effects = match Projects::load() {
                        Ok(projects) => {
                            let rows = projects.move_destinations(&self.tag.borrow());
                            self.state.borrow_mut().show_projects(&uuid, rows, projects)
                        }
                        Err(e) => vec![Effect::Notify(e.to_string())],
                    };
                    self.apply(effects);
                }
```
- `render`: read `state.hint()` into the destructured tuple alongside `projects().cloned()`, iterate `l.shown()` (rows) and call `self.folder_card(at, row)`, and append `empty_line(&hint)` when `Some`.
- `folder_card(self, at: usize, row: &Row)` uses `ProjectList::label(row)`.
- The footer: `list.as_ref().map_or(actions::CARD_KEYS, |l| l.keys())` is unchanged.

`src/panel/actions.rs`: delete `MOVE_KEYS`, `FOLDER_ICON`, `OPEN_TITLE`, `OPEN_KEYS`, `TYPE_A_NAME`, `GITHUB_ICON`, `folder_label` and the `folder_label` test; keep `CLEAR_ALL`, `CONFIRM_CLEAR_ALL`, `CARD_KEYS`, `clear_all_command`. Update the module doc's first sentence if it mentions the project list.

`src/daemon.rs`, `Request::Projects`:
```rust
            match crate::project::Projects::load() {
                Ok(projects) => panel.open_projects(projects),
                Err(e) => notify::tasks(&e.to_string()),
            }
```

`src/github.rs`: delete `MARKER`, `mark`, `unmark`, `enum Choice`, `choose`, and the tests `a_marked_row_round_trips_to_its_repo_name`, `plain_text_does_not_unmark`, `only_the_outer_marker_comes_off`, `picking_a_marked_row_clones`, `typing_a_repo_name_clones_it`, `a_spaced_variant_of_a_repo_name_clones_it`, `a_local_folder_wins_over_a_same_named_repo`, `new_names_still_create_folders`, `nothing_and_rejections_pass_through` (Task 1's `project.rs` tests cover them). Keep `strings` if other tests use it. Rewrite the module doc so it describes the cache, the refresh and the clone.

`src/project.rs`: delete `open_rows`, `rows_to_open` and the test `rows_to_open_are_folders_then_uncloned_repos`.

`src/main.rs`: `project_open` loses the `unmark` branch: `projects.open(projects.choice(&name))`. The test `opening_a_project_takes_an_optional_name` uses `project::open_args("convo")` instead of `github::mark("convo")`; update its doc comment ("picking a row runs it again with the row's bare name").

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4; grep -rn "github)\|unmark\|mark(\|folder_label\|open_rows\|rows_to_open" src tests`
Expected: no warnings; all tests pass; the grep prints nothing.

- [ ] **Step 4: The panel end to end, if the machine allows**

`cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -6`. Expected: the Move list (`m`, Escape) and the Open list (bare `project open`, Escape) checks pass. If it cannot run, or fails only on the known HERDR_SESSION leak, say so.

- [ ] **Step 5: Commit**

```bash
git add src/panel src/daemon.rs src/github.rs src/project.rs src/main.rs
git commit -m "refactor(panel): run the project list through its module

The state holds a ProjectList of typed rows and the Projects they came
from, the surface hands the ranking in as a matcher and draws what the
list says, and the daemon loads Projects once per open. The hint under
an unmatched name now comes from the same choice project open acts on,
so a typed repo name says it clones. The \"  (github)\" row marker goes:
a bare name is enough, since a name that is a repo and not a folder
already means clone.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: A `Mode` for the panel state, and the key map behind it

**Files:**
- Modify: `src/panel/state.rs` (fields, `tab`, `on_ideas`, `projects`, `hidden`, `tabs`, `shows_clear_all`, `visible`, `empty_text`, `set_cards`, `take_keyboard`, `open_projects`, `on_focus`, `on_key`, `on_press`, `show_projects`, `on_folder`, `release`, `pick`, `escape_projects`, `rerender`; tests), `src/panel/surface.rs` (`connect_keys` 531-573; `render` 895-982 split), `src/panel/keys.rs` (doc only)

**Interfaces:**
- Produces: `pub enum Mode { Tasks, Ideas, Projects(ProjectList) }`; `PanelState::mode(&self) -> &Mode`; `PanelState::on_key(&mut self, key: gdk::Key, ctrl: bool, shift: bool) -> Option<Vec<Effect>>` (new signature); `pub(crate) fn on_action(&mut self, action: KeyAction) -> Option<Vec<Effect>>` (today's `on_key` body, the tests' seam). `on_ideas()` and `projects()` keep their signatures.

- [ ] **Step 1: Write the failing tests**

In `state.rs`'s tests, change the helper `fn key(state: &mut PanelState, key: KeyAction) -> Vec<Effect>` to call `state.on_action(key)` instead of `state.on_key(key)`, and add:

```rust
    /// The three screens, and the way between them.
    #[test]
    fn the_mode_follows_the_keyboard_the_tabs_and_the_lists() {
        let mut state = keyboard(pending(&["a", "b"]));
        assert!(matches!(state.mode(), Mode::Tasks));
        key(&mut state, KeyAction::Ideas);
        assert!(matches!(state.mode(), Mode::Ideas));
        key(&mut state, KeyAction::Release);
        assert!(matches!(state.mode(), Mode::Tasks), "Escape on Ideas is back to the list");
        state.show_projects("a", folders(&["x"]), projects(&["a", "x"], &[]));
        assert!(matches!(state.mode(), Mode::Projects(_)));
        key(&mut state, KeyAction::Release);
        assert!(matches!(state.mode(), Mode::Tasks), "Escape with nothing typed closes Move's list");
        assert!(state.keyboard());
        assert_eq!(state.focus(), focused("a", Slot::Body).as_ref());
        state.open_projects(projects(&["x"], &[]));
        assert!(matches!(state.mode(), Mode::Projects(_)));
        key(&mut state, KeyAction::Release);
        assert!(!state.keyboard(), "Escape on the open list gives the keyboard back");
        assert!(matches!(state.mode(), Mode::Tasks));
    }

    #[test]
    fn the_moving_task_leaving_closes_its_list() {
        let mut state = keyboard(pending(&["a", "b"]));
        state.show_projects("a", folders(&["x"]), projects(&["a", "x"], &[]));
        state.set_cards(&pending(&["b"]));
        assert!(matches!(state.mode(), Mode::Tasks));
    }

    /// The state picks the key map by mode: a letter presses a button on
    /// Tasks and is typing on Ideas and on the project list.
    #[test]
    fn a_raw_key_is_mapped_by_the_mode() {
        use gtk4::gdk::Key;
        let mut state = keyboard(pending(&["a"]));
        assert!(state.on_key(Key::s, false, false).is_some_and(|e| e.iter().any(|x| matches!(x, Effect::Spawn(_)))));
        let mut state = keyboard(pending(&["a"]));
        key(&mut state, KeyAction::Ideas);
        assert_eq!(state.on_key(Key::s, false, false), None, "typing on Ideas");
        assert!(state.on_key(Key::Escape, false, false).is_some());
        let mut state = keyboard(pending(&["a"]));
        state.show_projects("a", folders(&["x"]), projects(&["a", "x"], &[]));
        assert_eq!(state.on_key(Key::s, false, false), None, "typing on the project list");
        assert_eq!(state.on_key(Key::Down, false, false), Some(vec![Effect::ShowFolder]));
        let mut state = PanelState::default();
        state.set_cards(&pending(&["a"]));
        assert_eq!(state.on_key(Key::s, false, false), None, "no key is the panel's without the keyboard");
    }
```

Run: `cargo test --lib panel::state::`
Expected: FAIL to compile (`Mode`, `mode()`, `on_action` not found).

- [ ] **Step 2: Implement the mode**

In `src/panel/state.rs`:
- Add, after `Shown`:
```rust
/// Which of the three screens the panel shows while it has the keyboard.
/// Without the keyboard it is always on Tasks: the hover and the peek are
/// the task cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// The task cards under their filter tabs.
    Tasks,
    /// The Ideas tab: the workspace's notepad in place of the cards.
    Ideas,
    /// The project list, for Move to workspace or Mod+Alt+W.
    Projects(ProjectList),
}
```
and `impl Default for Mode { fn default() -> Mode { Mode::Tasks } }` (or `#[derive(Default)]` with `#[default]` on `Tasks`).
- Replace the fields `ideas: bool` and `projects: Option<ProjectList>` with `mode: Mode` (doc: "Which screen is up. Only ever Tasks without the keyboard."). Keep `loaded: Option<Projects>`.
- Add:
```rust
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// On the task cards, where the tabs, Clear all and the cards' keys
    /// apply; not on Ideas or the project list.
    fn on_tasks(&self) -> bool {
        matches!(self.mode, Mode::Tasks)
    }

    pub fn on_ideas(&self) -> bool {
        matches!(self.mode, Mode::Ideas)
    }

    pub fn projects(&self) -> Option<&ProjectList> {
        match &self.mode {
            Mode::Projects(list) => Some(list),
            _ => None,
        }
    }

    fn projects_mut(&mut self) -> Option<&mut ProjectList> {
        match &mut self.mode {
            Mode::Projects(list) => Some(list),
            _ => None,
        }
    }
```
- `tab()`: `if self.on_ideas() { Tab::Ideas } else { Tab::Filter(self.filter) }`.
- Every `!self.ideas && self.projects.is_none()` → `self.on_tasks()`; every `self.ideas || self.projects.is_some()` → `!self.on_tasks()`; `self.projects.is_some()` alone → `self.projects().is_some()`; `self.ideas` alone → `self.on_ideas()`.
- Every place that set `self.ideas = false; self.projects = None;` (`take_keyboard`, `release`, `open_projects`) sets `self.mode = Mode::Tasks;` (and `open_projects` then sets `self.mode = Mode::Projects(ProjectList::new(Purpose::Open, projects.rows()))`). `pick(Tab::Ideas)` sets `self.mode = Mode::Ideas`; `pick(Tab::Filter(f))` sets `self.filter = f; self.mode = Mode::Tasks;`. `show_projects` sets `self.mode = Mode::Projects(ProjectList::new(Purpose::Move { .. }, rows))`. `on_folder` and `escape_projects`'s `Closed` on Move set `self.mode = Mode::Tasks` where they set `self.projects = None`. `set_cards`'s `gone` check reads `self.projects().and_then(ProjectList::moving)` and sets `self.mode = Mode::Tasks` when gone.
- `on_query`, `move_folder`, `escape_projects` use `self.projects_mut()`; `on_folder` and `hint` use `self.projects()`.
- Replace `pub fn on_key(&mut self, key: KeyAction)` with:
```rust
    /// A key while the panel has the keyboard, mapped by the mode it is in:
    /// on Tasks the letters press buttons and the digits pick tabs; on Ideas
    /// and on the project list every key is typing but the few the mode
    /// keeps. None when the key is not the panel's and GTK should have it,
    /// and always None without the keyboard: a key landing after it was
    /// given back is not for the panel.
    pub fn on_key(&mut self, key: gdk::Key, ctrl: bool, shift: bool) -> Option<Vec<Effect>> {
        if !self.keyboard {
            return None;
        }
        let action = match &self.mode {
            Mode::Ideas => keys::ideas_key_action(key, ctrl),
            Mode::Projects(_) => keys::project_key_action(key),
            Mode::Tasks => keys::key_action(key, ctrl, shift),
        };
        self.on_action(action)
    }

    /// What a mapped key does, by mode. Pure, and the tests' way in: they
    /// drive the state with `KeyAction`s rather than key codes.
    pub(crate) fn on_action(&mut self, action: KeyAction) -> Option<Vec<Effect>> {
        if !self.keyboard {
            return None;
        }
        match &self.mode {
            Mode::Ideas => match action { /* today's Ideas branch */ },
            Mode::Projects(_) => match action { /* today's project-list branch */ },
            Mode::Tasks => { /* today's Clear-all-armed branch, then the main match */ }
        }
    }
```
with the three bodies moved verbatim from today's `on_key`. Add `use gtk4::gdk;` to `state.rs`.

- [ ] **Step 3: The surface stops choosing the mapper, and draws by mode**

`src/panel/surface.rs`:
- `connect_keys`: delete the `(ideas, listing)` lookup and the three-way `if`; call `p.state.borrow_mut().on_key(key, ctrl, shift)`. Remove `use super::keys` if nothing else in the file uses it (check `keys::step` and `tab_number` uses; keep the import if so).
- `render`: keep the `hidden()` early return and the destructuring, then replace the `while_drawing` closure's body with a `match` on the mode, each arm a method:
```rust
        self.while_drawing(|| match &mode {
            Mode::Ideas => self.draw_ideas(),
            Mode::Projects(list) => self.draw_projects(list, hint.as_deref()),
            Mode::Tasks => self.draw_tasks(&shown, keyboard, empty),
        });
```
where `mode` is `state.mode().clone()` read in the destructuring (it is `Clone`; the list is small), `hint` is `state.hint()`, and:
  - `draw_ideas(&self)`: the "notepad already parented, so leave it" check, else clear the column and append `self.notepad.root`.
  - `draw_projects(&self, list: &ProjectList, hint: Option<&str>)`: clear the column, clear `self.cards`, append the title line, one `folder_card` per `list.shown()` row into `self.folders`, then the hint line if any.
  - `draw_tasks(&self, shown: &[Shown], keyboard: bool, empty: Option<&str>)`: clear the column, build the card widgets into `self.cards`, clear `self.folders`, append the empty line if any.
  The code after `while_drawing` (tabs, footer, `update_tabs`, `fit`, `present`, region, blur, focus) stays as it is, using `ideas`/`list.is_some()` as today or `matches!` on the mode.
- Remove the `ideas` and `list` locals that the split makes unused.

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | tail -4`
Expected: no warnings; all tests pass, the three new ones included; `grep -n "ideas_key_action\|project_key_action" src/panel/surface.rs` prints nothing.

- [ ] **Step 4: Docs in code**

`src/panel/keys.rs` module doc: "What the key then does is `state.rs`'s" stays; add that the state picks which of the three mappers to run by its mode. `src/panel/state.rs` module doc: mention the mode in the list of what lives there ("which of the task cards, the Ideas notepad or the project list is up").

- [ ] **Step 5: e2e, if the machine allows**

`cargo build && NIRITASKS=$PWD/target/debug/niritasks bash tests/e2e-panel.sh 2>&1 | tail -6`: tabs, Ideas typing, the Move and Open lists and Escape all exercise the mode. Report the summary or the reason it was skipped.

- [ ] **Step 6: Commit**

```bash
git add src/panel
git commit -m "refactor(panel): give the panel state a mode and the key map

The state tracked the three screens it can show as a bool and an
Option, and ten of its methods re-derived which one was up from both;
the surface read the same two to pick a key map. One Mode names the
screen, on_key takes the raw key and maps it by mode, and render draws
one function per mode.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: Docs

**Files:**
- Modify: `CONTEXT.md` (the **Project list** entry at 126; add **Mode** and **Choice**), `README.md:22` (the Mod+Alt+W row), `docs/adr/0003-the-daemon-draws-and-the-cli-asks.md` (no change needed; verify its "typed rows instead of a string echoed back" sentence is now true)

- [ ] **Step 1: CONTEXT.md**

In the **Project list** entry, after the sentence ending "a typed name matching nothing makes a new folder.", add: "The line under the title says what Enter will do with the text, opening, making or cloning, decided by the same rule `project open` acts on." Then add two entries after it:

```markdown
**Mode**:
Which of three screens the task panel is showing while it has the keyboard:
Tasks (the task cards under their filter tabs), Ideas (the notepad) or
Projects (the project list). Without the keyboard it is always on Tasks. In
the code, `PanelState::mode`.
_Avoid_: view, screen (in code), state

**Choice**:
What `project open` does with a name, and so what Enter on the project list
will do with what is typed: open a folder that exists, make a new one, clone
a GitHub repo not cloned yet, refuse it, or nothing. One function decides it
(`Projects::choice`), for the panel's hint and the CLI alike; a name that is
one of your repos and not a folder clones.
_Avoid_: resolution, selection, pick (the act of choosing a row)
```

- [ ] **Step 2: README**

In the `Mod+Alt+W` row (line 22), after "A name that matches nothing makes `~/Projects/<name>`.", add " The line under the list says what Enter will do: open, make, or clone a repo of that name."

- [ ] **Step 3: Verify and commit**

Run: `cargo test 2>&1 | tail -3` (the README/llms tests only check the Commands block and llms.txt, which did not change).

```bash
git add CONTEXT.md README.md
git commit -m "docs: name the panel's mode and a pick's choice

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

## Self-review notes

- Spec coverage: §1 → Tasks 1, 2 and the deletions in Task 4; §2 → Tasks 3, 4; §3 → Task 5; the spec's "Order of work" step 4 → Task 6. `Projects::open` takes the directory from `self.dir` as the spec requires.
- Type consistency: `Row::name(&self) -> &str`, `Projects::rows() -> Vec<Row>`, `Projects::move_destinations(&self, &str) -> Vec<Row>`, `Projects::choice(&self, &str) -> Choice`, `Projects::open(&self, Choice) -> Result<()>` (Tasks 1, 2) are what Tasks 3, 4 call. `ProjectList::new(Purpose, Vec<Row>)`, `narrow(&mut self, &str, Matcher) -> bool`, `step(&mut self, bool) -> bool`, `escape(&mut self) -> Escaped`, `pick(&self, usize, &Projects) -> Picked`, `hint(&self, &Projects) -> Option<String>` (Task 3) are what Task 4's state methods call. `PanelState::on_action(KeyAction)` (Task 5) is what the test helper `key` calls.
- Intermediate states: after Task 2 the panel still sends marked rows and the CLI still understands them (the `unmark` adapter); after Task 3 the module exists unused (dead-code warnings allowed for that one commit); Task 4 removes both. Every other commit builds warning-free.
- Placeholders: the `on_action` sketch in Task 5 step 2 says "today's branch" three times; the branches are the three blocks of the current `on_key` (state.rs 444-499), moved without change. The implementer has them in the file.
