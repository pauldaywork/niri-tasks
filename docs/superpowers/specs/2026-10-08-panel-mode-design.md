# Panel mode design

The task panel can show three things while it has the keyboard: the task
cards under their filter tabs, the Ideas notepad, or the project list. The
panel state (`src/panel/state.rs`) tracks which with two fields, `ideas: bool`
and `projects: Option<ProjectList>`, and ten of its methods re-derive the
answer from both. The surface picks which key map to use from the same two
fields. The project list's data lives in the state, its ranking in the
surface, its words in the action-row module and its keys in a third mapper.
And what Enter does to a typed name is computed twice: the panel's hint uses
`project::resolve`, the CLI acts on `github::choose`, so a typed name that is
also an uncloned GitHub repo is announced as "Enter makes ~/Projects/x" and
then cloned.

This design gives the state one `Mode`, the project list one module with its
matcher injected, and a pick one definition that the hint and the CLI share.
It is the second half of the 2026-10-07 architecture review's top
recommendation (P3, P1, P2, with P4 falling out). Nothing the user sees
changes except the hint line, which becomes correct.

## Vocabulary

From CONTEXT.md: task panel, task card, action row, task action, filter tab,
Ideas tab, project list, session. New here:

**Mode**: which of the three screens the panel is showing while it has the
keyboard: `Tasks` (the cards under the filter tabs), `Ideas` (the notepad) or
`Projects` (the project list). Without the keyboard the panel is always on
Tasks, as the hover and the peek. _Avoid_: view, screen (in code), state.

**Choice**: what `project open` will do with a name: open a folder that
exists, make a new one, clone a GitHub repo, refuse it, or nothing. One
function decides it, for the panel's hint and for the CLI's act alike.

## 1. One definition of a pick (`src/project.rs`)

### Today

`project::open_rows` reads the folders and the GitHub cache and marks each
remote repo with the suffix `"  (github)"`; the panel shows the marked
string with a GitHub icon and spawns `project open -- <row>` with the suffix
on; `main::open_project` reads the folders and the cache again and calls
`github::choose`, which strips the suffix to decide Clone, else runs
`project::resolve` and promotes a creatable name that is a remote repo to
Clone. The panel's `ProjectList::typed()` runs only `project::resolve`, so
its hint never says clone.

### Design

`project.rs` gains a `Projects` value, the one place the two lists are read:

```rust
/// The projects a name can mean: the `~/Projects` folders and the GitHub
/// repos not cloned yet, read once per use.
pub struct Projects {
    pub dir: PathBuf,
    pub local: Vec<String>,
    pub remote: Vec<String>,   // github::remote_only(cache, local)
}

/// One row of the project list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Folder(String),
    Repo(String),
}

impl Row {
    /// The name a pick spawns: `project open -- <name>` or `task move <uuid> <name>`.
    pub fn name(&self) -> &str;
}

/// What `project open` does with a name, or what Enter on the project
/// list will do with what is typed.
#[derive(Debug, PartialEq, Eq)]
pub enum Choice {
    Open(String),
    Create(String),
    Clone(String),
    Rejected(String),
    Nothing,
}

impl Projects {
    pub fn load() -> Result<Projects>;                 // list() + github cache
    pub fn rows(&self) -> Vec<Row>;                    // folders, then repos
    pub fn move_destinations(&self, current_tag: &str) -> Vec<Row>;  // folders only, minus the task's own
    pub fn choice(&self, typed: &str) -> Choice;       // resolve, then Create→Clone when remote has it
    pub fn open(&self, choice: Choice) -> Result<()>;  // create or clone, then name/focus the workspace and spawn its programs
}
```

`choice` is `github::choose` moved here with the marker branch removed: a
picked `Row::Repo(name)` is spawned as its bare name, and `choice(name)`
resolves it to `Clone` because the name is in `remote` and not in `local`.
That is the rule the CLI already applies to typed text, so the marker was
only ever a second way to say the same thing. `github::mark`, `unmark`,
`Choice` and `choose` go; `github.rs` keeps the cache, the refresh and
`clone`.

`open` is `main::open_project`'s body from the match on `Choice` down: make
the folder or clone the repo (with the two notifications), then go back to
the project's workspace if it has one, else name the last workspace on the
focused output and spawn its programs (`startup_commands` through
`niri::spawn`). `Choice::Nothing` returns `Ok(())`; `Rejected(why)` is an
error. `main::project_open(Some(name))` becomes:

```rust
let projects = project::Projects::load()?;
projects.open(projects.choice(&name))
```

`project::open_args(name)` stays (`["project", "open", "--", name]`).
`open_rows` and `rows_to_open` go; the daemon's `Request::Projects` handler
calls `Projects::load()?.rows()`.

### Tests

- `choice`: a folder name opens; a new name creates; a repo name clones; a
  new name that is also a remote repo clones (the divergence case); `/` and
  a leading `.` are rejected; empty is nothing. Built on a `Projects` literal,
  no disk.
- `rows` and `move_destinations`: order, the task's own folder left out,
  repos never offered as a move destination.
- `open` touches the filesystem and niri, so it is covered by the existing
  e2e-panel Open check and by its callers' tests; its create branch can be
  tested against a scratch directory if `open` takes the directory from
  `self.dir`, which it does.

## 2. The project list as a module (`src/panel/projects.rs`)

### Today

`ProjectList` and `Purpose` are in `state.rs` (lines 74-160); `on_query`,
`on_folder`, `escape_projects`, `move_folder`, `show_projects` and
`open_projects` are `PanelState` methods; the folder labels, icons, titles
and footers are constants in `panel/actions.rs` (140-207); ranking is
`surface::rank`, which runs `project::fzf_matches` and falls back to
`substring_matches` with a one-time notification, then calls
`state.on_query(query, shown)`.

### Design

A new module owns the list and everything about it that is not drawing:

```rust
pub enum Purpose { Move { uuid: String, text: String }, Open }

/// Ranks `names` for `query`, best first. The daemon passes fzf with its
/// substring fallback; tests pass `project::substring_matches`.
pub type Matcher<'a> = &'a dyn Fn(&[String], &str) -> Vec<String>;

pub struct ProjectList {
    purpose: Purpose,
    rows: Vec<Row>,          // every row, in list order
    query: String,
    shown: Vec<Row>,         // rows matching query, ranked
    at: usize,
}

pub enum Picked {
    /// `project open -- name` or `task move <uuid> name`, by purpose.
    Spawn(Vec<String>),
    /// Nothing to pick: no row shown and no typed name that could be one.
    Nothing,
}

pub enum Escaped { Cleared, Closed }

impl ProjectList {
    pub fn new(purpose: Purpose, rows: Vec<Row>) -> ProjectList;
    pub fn purpose(&self) -> &Purpose;
    pub fn moving(&self) -> Option<&str>;
    pub fn shown(&self) -> &[Row];
    pub fn at(&self) -> usize;
    pub fn query(&self) -> &str;
    /// Re-rank for `query`; false when the text is what it was.
    pub fn narrow(&mut self, query: &str, matcher: Matcher) -> bool;
    pub fn step(&mut self, forward: bool) -> bool;     // false with nothing shown
    pub fn escape(&mut self) -> Escaped;               // Cleared with text typed, else Closed
    pub fn pick(&self, at: usize, projects: &Projects) -> Picked;
    pub fn hint(&self, projects: &Projects) -> Option<String>;  // the line under the title when nothing matches on Open
    pub fn title(&self) -> String;
    pub fn keys(&self) -> &'static str;
    pub fn label(row: &Row) -> String;                 // icon + name
}
```

`pick` and `hint` take the `Projects` the rows came from so both run
`projects.choice(typed)`: the hint says "Enter clones <repo>", "Enter makes
~/Projects/<name>", "Enter opens ~/Projects/<name>", the rejection reason, or
"Type a name to make a new project"; the pick spawns `open_args(name)` for
the same name. On Move the typed text is never a pick (only a shown row is),
as today.

The constants `MOVE_KEYS`, `OPEN_TITLE`, `OPEN_KEYS`, `TYPE_A_NAME`,
`FOLDER_ICON`, `GITHUB_ICON`, `NO_DESTINATIONS` and `folder_label` move here
from `panel/actions.rs` and `state.rs`; `panel/actions.rs` is about the
action row again. `keys::project_key_action` stays in `keys.rs` beside the
other two mappers.

The `PanelState` keeps the list as the payload of `Mode::Projects` (section 3)
and holds the `Projects` it was built from, so `pick` and `hint` have it.
`Effect::ListProjects(uuid)` stays: the surface calls `Projects::load()`,
and hands the state `show_projects(uuid, projects)`; the state builds the
list from `projects.move_destinations(tag)`. The surface needs the tag it
already has (`self.tag`). `open_projects(projects)` likewise takes the
loaded `Projects` rather than string rows.

`surface::rank` becomes the daemon's matcher closure: fzf, falling back to
substring with the one-time notification as now, passed to
`state.on_query(query, &matcher)`.

### Tests

In `panel/projects.rs`, with `substring_matches` as the matcher and a
`Projects` literal:

- `narrow` keeps every row with an empty query, ranks matches for a query,
  reports no change for the same text, and resets the highlight to the top.
- `step` stops at both ends and does nothing with nothing shown.
- `escape` clears typed text first and closes on the second.
- `pick` on Open: a shown row spawns `open_args(name)`; with nothing shown a
  typed new name spawns it too; a typed repo name spawns it (and `choice`
  says Clone); rejected text is `Nothing`. On Move: a shown row spawns
  `task move <uuid> <name>`; typed text is `Nothing`.
- `hint` says clone for a remote repo name, make for a new name, open for an
  existing folder, the rejection reason, and the type-a-name line with an
  empty list; `None` on Move and while anything is shown.
- `label` gives a repo the GitHub icon and a folder the folder icon.

The `state.rs` tests that drive the list (`show_projects`, `on_query`,
Enter, Escape) keep passing through the new types; their fixtures change
from `Vec<String>` rows to `Projects` literals.

## 3. A Mode for the panel state (`src/panel/state.rs`, `keys.rs`, `surface.rs`)

### Today

`ideas: bool` and `projects: Option<ProjectList>`; the guard
`!self.ideas && self.projects.is_none()` in `hidden`, `tabs`,
`shows_clear_all`, `visible`, `empty_text`, `set_cards`, `on_focus`,
`on_press`, `pick` and `rerender`; `on_key` an if-ladder on the two fields;
`surface::connect_keys` reading both to choose among `keys::key_action`,
`ideas_key_action` and `project_key_action`; `surface::render` one function
branching on both.

### Design

```rust
/// Which of the three screens the panel shows while it has the keyboard.
/// Without the keyboard it is always on Tasks.
pub enum Mode {
    Tasks,
    Ideas,
    Projects(ProjectList),
}
```

`PanelState` replaces `ideas` and `projects` with `mode: Mode` and adds
`projects: Option<Projects>` (the loaded lists, `Some` only while
`Mode::Projects`). `keyboard`, `filter`, `expanded`, `agents`, `focus` and
`armed` stay as they are: `filter` is "the tab picked, or on Ideas the one
picked before it", and `focus` is kept across the Move list so Escape goes
back to the same card.

- `on_tasks()` replaces the ten guards: `matches!(self.mode, Mode::Tasks)`.
- `tab()` is `Mode::Ideas → Tab::Ideas`, else `Tab::Filter(self.filter)`.
- `on_ideas()` and `projects()` stay as accessors for the surface
  (`projects()` returns `Option<&ProjectList>` from the mode).
- `release()`, `take_keyboard()` and `set_cards()` set `mode = Tasks` where
  they set the two fields today.
- `on_key` becomes:

```rust
/// A key while the panel has the keyboard, mapped by the mode it is in.
/// None when the key is not the panel's and GTK should have it.
pub fn on_key(&mut self, key: gdk::Key, ctrl: bool, shift: bool) -> Option<Vec<Effect>> {
    if !self.keyboard { return None; }
    let action = match &self.mode {
        Mode::Ideas => keys::ideas_key_action(key, ctrl),
        Mode::Projects(_) => keys::project_key_action(key),
        Mode::Tasks => keys::key_action(key, ctrl, shift),
    };
    self.on_action(action)
}

/// What a mapped key does, by mode. The test seam: the state tests drive
/// this with `KeyAction`s, as they always have.
pub(crate) fn on_action(&mut self, action: KeyAction) -> Option<Vec<Effect>>
```

`on_action` is today's `on_key` body, its if-ladder a `match &mut self.mode`
with the Clear-all-armed branch inside `Mode::Tasks`. `surface::connect_keys`
calls `state.on_key(key, ctrl, shift)` and no longer imports `keys`.

`on_query(query, matcher)` narrows the list and returns `Render` on change;
`on_folder(at)` calls `list.pick(at, projects)` and spawns or does nothing
(releasing first on Open, moving the focus to the neighbour on Move, as
today); `escape_projects` maps `Escaped::Cleared` to `[ClearQuery, Render]`
and `Closed` to release (Open) or `mode = Tasks` + rerender (Move).

`surface::render` splits into `draw_tasks(shown, empty)`, `draw_projects(list)`
and `draw_ideas()`, chosen by one `match` on a copy of the mode's tag (the
surface already clones what it needs out of the borrow), with the shared
tail (tabs and footer visibility, `fit`, `present`, region, blur, focus)
after. `Effect::ListProjects` calls `Projects::load()` and
`state.show_projects(uuid, projects)`.

### Tests

- Every existing `state.rs` test keeps passing, through `on_action`.
- New: a mode table test: `take_keyboard` → Tasks; `pick(Tab::Ideas)` →
  Ideas and back; `show_projects` → Projects(Move) and Escape twice → Tasks
  on the same card; `open_projects` → Projects(Open) and Escape → keyboard
  released; `set_cards` with the moving task gone → Tasks; the keyboard
  going → Tasks.
- New: `on_key` picks the mapper by mode: `s` on Tasks runs Start, `s` on
  Ideas is typing (None), `s` on Projects is typing (None); Escape on each.
- `keys.rs` tests unchanged.

## Order of work

1. `project::Projects`, `Row`, `Choice`, `choice`, `rows`,
   `move_destinations` on `Projects`, with tests; `github::choose`/`Choice`
   removed; `main::project_open` and the daemon's `Request::Projects` on the
   new value; `open` moved into the lib. The panel still receives string rows
   at this step (`Row::name()` strings), so it keeps working.
2. `panel/projects.rs` with its tests; `state.rs` holds the new `ProjectList`
   and `Projects`; constants move; `surface::rank` becomes the matcher
   closure; the hint uses `choice`.
3. `Mode` in `state.rs`; `on_key(gdk::Key, …)` and `on_action`; the surface
   stops choosing the mapper; `render` split.
4. Docs: CONTEXT.md gains **Mode** and **Choice**, README's project-list
   paragraph says the hint and the pick agree, llms.txt unchanged (no command
   changes), ADR 0003's "typed rows instead of a string echoed back" now
   true in code.

## Out of scope

Splitting `project.rs` into smaller files (D6), the herdr `Session` (D1), the
HOME and workspace lookups (D2), folding the action tables (D3), and the
card-notes toggle (task b96c1f78). The e2e harness's HERDR_* leak is task
a030807c.
