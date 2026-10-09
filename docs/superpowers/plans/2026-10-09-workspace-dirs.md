# Workspace and Dirs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One `Dirs` value reads `HOME`; one `Workspace` value answers "which workspace" and carries its tag, session name and folder; the session-opening callers take it; `project.rs` splits into project, programs and matching; nothing the user sees changes.

**Architecture:** see `docs/superpowers/specs/2026-10-09-workspace-dirs-design.md` (committed with this plan). Taskwarrior task `2d0c1baf`.

**Tech Stack:** Rust (anyhow, niri_ipc), bash PATH-shim tests.

## Global Constraints

- Conventional Commits with an attribution trailer (`Co-Authored-By: <your model> <noreply@anthropic.com>`); subject ≤72, body wrapped at 72 saying what and why.
- `cargo build 2>&1 | grep -E "^(warning|error)"` prints nothing and `cargo test` passes in full after every task, including `tests/typed_task_number.rs`, `tests/link_pane.rs` and `tests/write_path.rs`, which must not be edited.
- **Every user-visible string stays byte-identical.** Before deleting a function, copy its error texts verbatim. `grep -rn` the old text after each task to prove it still exists once.
- Work in the worktree `/home/paul/.worktrees/niri-tasks/task-workspace-dirs-2d0c1baf` (branch `task/workspace-dirs-2d0c1baf`). **Never run `install.sh`, `cargo install`, `systemctl --user restart niri-tasks`, `tests/e2e-*.sh`**, or start herdr sessions.
- Docs on every pub item; comments are full sentences that say why. Do not touch `docs/superpowers/plans/*` other than this file, nor `.ua/`. Line numbers are anchors from main @ 4281dac; match on quoted text.

---

### Task 1: `Dirs`, the one HOME read

**Files:**
- Create: `src/dirs.rs`
- Modify: `src/lib.rs` (`pub mod dirs;`), `src/session.rs` (delete `start_dir`, `project_from_cwd` and their tests; keep `Component` import only if still used), `src/ideas.rs`, `src/github.rs`, `src/refine.rs`, `src/project.rs` (`list`), `src/main.rs` (`terminal`), `src/work.rs` (`repo_for`), `src/session.rs` (`Session::for_workspace`)

- [ ] **Step 1: Write `src/dirs.rs`**

```rust
//! Where things are on this machine: the home folder, the XDG data and cache
//! folders, and `~/Projects`. The one module that reads `HOME`, so a path
//! rule lives here once and every test can point it at a scratch folder.

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

/// The folders this process works from. Build one with [`Dirs::from_env`]
/// where a command needs paths, or [`Dirs::at`] in a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    home: PathBuf,
    /// `XDG_DATA_HOME`, when set and absolute.
    data: Option<PathBuf>,
    /// `XDG_CACHE_HOME`, when set and absolute.
    cache: Option<PathBuf>,
}

impl Dirs {
    /// From this process's environment. `HOME` is required; the XDG folders
    /// are optional, and a relative one is ignored, as the XDG spec says.
    pub fn from_env() -> Result<Dirs> {
        let home = std::env::var_os("HOME").context("HOME is unset")?;
        Ok(Dirs {
            home: PathBuf::from(home),
            data: xdg("XDG_DATA_HOME"),
            cache: xdg("XDG_CACHE_HOME"),
        })
    }

    /// A home with no XDG overrides: what tests build on a scratch folder.
    pub fn at(home: &Path) -> Dirs {
        Dirs { home: home.to_path_buf(), data: None, cache: None }
    }

    /// With `XDG_DATA_HOME` set to `p` (relative is ignored, as from the environment).
    pub fn with_data(mut self, p: &Path) -> Dirs {
        self.data = p.is_absolute().then(|| p.to_path_buf());
        self
    }

    /// With `XDG_CACHE_HOME` set to `p`.
    pub fn with_cache(mut self, p: &Path) -> Dirs {
        self.cache = p.is_absolute().then(|| p.to_path_buf());
        self
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    /// `~/Projects`: one folder per project, each the name of its workspace.
    pub fn projects(&self) -> PathBuf {
        self.home.join("Projects")
    }

    /// Where this tool keeps data: `$XDG_DATA_HOME`, else `~/.local/share`.
    pub fn data(&self) -> PathBuf {
        self.data.clone().unwrap_or_else(|| self.home.join(".local/share"))
    }

    /// Where this tool keeps caches: `$XDG_CACHE_HOME`, else `~/.cache`.
    pub fn cache(&self) -> PathBuf {
        self.cache.clone().unwrap_or_else(|| self.home.join(".cache"))
    }

    /// Where a workspace's terminals start.
    ///
    /// `~/Projects/<workspace>` -> `~/Projects` -> `~`. Uses the *raw* workspace
    /// name, not the sanitised one. Always an existing directory, which
    /// `ghostty +new-window` needs: it resolves the path before asking the
    /// running ghostty for a window, and fails outright on one that is not there.
    pub fn start_dir(&self, workspace_raw: &str) -> PathBuf {
        let projects = self.projects();
        let in_project = projects.join(workspace_raw);
        if !workspace_raw.is_empty() && in_project.is_dir() {
            return in_project;
        }
        if projects.is_dir() {
            return projects;
        }
        self.home.clone()
    }

    /// The project folder `cwd` is inside, if it is inside one: the first path
    /// component under `~/Projects`. [`Dirs::start_dir`] run backwards.
    pub fn project_from_cwd(&self, cwd: &Path) -> Option<String> {
        let rest = cwd.strip_prefix(self.projects()).ok()?;
        match rest.components().next()? {
            Component::Normal(name) => name.to_str().map(str::to_string),
            _ => None,
        }
    }
}

/// An XDG folder variable, when set and absolute.
fn xdg(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute())
}
```

Tests in `dirs.rs`: move `session.rs`'s `start_dir_*` and `project_from_cwd_*` tests here unchanged in substance, calling `Dirs::at(&home).start_dir(..)` / `.project_from_cwd(..)`; add `data_and_cache_prefer_absolute_xdg_and_fall_back_to_home` (with_data absolute → used; relative → ignored → `~/.local/share`; same for cache → `~/.cache`).

- [ ] **Step 2: Switch the readers of HOME**

- `ideas.rs`: `pub fn file(dirs: &Dirs, tag: &str) -> Option<PathBuf>` uses `dirs.data().join("niri-tasks/ideas").join(..)`; `file_for(tag)` = `Dirs::from_env().ok().and_then(|d| file(&d, tag))`. Its tests build `Dirs::at(..)`/`.with_data(..)`; keep the relative-XDG test (now `with_data` drops it).
- `github.rs`: `cache_path()` = `Dirs::from_env().ok().map(|d| d.cache().join("niritasks/github-repos"))`. Keep the doc ("`$XDG_CACHE_HOME/niritasks/github-repos`, or `~/.cache/…`"). Adapt any test.
- `refine.rs`: `refine_mod_dir(dirs: &Dirs) -> PathBuf` = `dirs.data().join("niri-tasks/refine-mod")`; `hidden_paths(home: &Path)` keeps its signature; `launch` does `let dirs = Dirs::from_env()?; let home = dirs.home();` in place of the HOME read, and `refine_mod_dir(&dirs)`. Adapt the `refine_mod_dir` tests to `Dirs::at(..).with_data(..)`.
- `project.rs::list`: `let dir = Dirs::from_env()?.projects();`.
- `main.rs::terminal`: `let dirs = Dirs::from_env()?; … let dir = dirs.start_dir(&workspace);`.
- `work.rs::repo_for`: `let repo = Dirs::from_env()?.start_dir(workspace);`.
- `session.rs::for_workspace`: `dir: Dirs::from_env()?.start_dir(workspace)`; delete `start_dir`/`project_from_cwd` and their tests from `session.rs`; `lib.rs::session_workspace` uses `Dirs::from_env()?` and `dirs.project_from_cwd(&cwd)` (its two error texts unchanged).

Check: `grep -rn 'var("HOME")\|var_os("HOME")' src` prints only `src/dirs.rs` (and a string inside a `speak.rs` test, which is not a read). `grep -rn 'join("Projects")' src` prints only `src/dirs.rs`.

Run: `cargo build 2>&1 | grep -E "^(warning|error)"; cargo test 2>&1 | grep -E "^test result|FAILED"`

- [ ] **Step 3: Commit**

```
feat(dirs): read HOME and the XDG folders in one place

Eight files read HOME and three joined ~/Projects; the folder rules sat
in session.rs beside the herdr code. Dirs holds the home and the XDG
data and cache folders once, with the start-folder and project-from-cwd
rules, so each path rule lives in one module and tests point it at a
scratch folder.
```

---

### Task 2: `Workspace`, and main.rs/daemon.rs on it

**Files:**
- Create: `src/workspace.rs`
- Modify: `src/lib.rs` (delete every fn; keep `pub mod` list + doc; add `pub mod workspace;`), `src/main.rs` (imports; `Tag`, `Move`, `Add`, `Start`, `Refine`, `Session` arms), `src/daemon.rs` (`workspace_default` → `workspace::name_default`, `require_workspace_tag` → `Workspace::focused().map(|w| w.tag().to_string())`)

In this task callers (`refine::launch`, `work::*`, `link::*`, `Session::for_workspace`) keep `&str`; main.rs passes `ws.name()`. Task 3 switches them.

- [ ] **Step 1: Write `src/workspace.rs`**

```rust
//! Which workspace a command is about, decided once, as a value that always
//! has a usable tag: the focused one for a keybind, this terminal's herdr
//! session (or its ~/Projects folder) for an agent, and whichever is asking
//! for the commands both run. The policies and their refusals were spread
//! over the crate root; they live here.

use crate::{dirs::Dirs, niri, session, tag};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// A named niri workspace whose name folds to a usable task tag. Built by one
/// of the three policies, or [`Workspace::named`] for a name a flag carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    name: String,
    tag: String,
}

impl Workspace {
    /// `name`, refused unnamed or with nothing a tag keeps. Every entry point
    /// refuses rather than guesses: an untagged task is invisible to every
    /// list, which is worse than doing nothing.
    pub fn named(name: &str) -> Result<Workspace> {
        anyhow::ensure!(
            !name.is_empty(),
            "This workspace has no name — Mod+Alt+W opens a project on a named one."
        );
        let tag = tag::workspace_tag(name);
        anyhow::ensure!(!tag.is_empty(), "Workspace name '{name}' has no usable tag characters.");
        Ok(Workspace { name: name.to_string(), tag })
    }

    /// The focused workspace: what a keybind and the panel act on.
    pub fn focused() -> Result<Workspace> {
        Workspace::named(&niri::focused_workspace_name()?.unwrap_or_default())
    }

    /// The workspace *this terminal* belongs to, rather than whichever is
    /// focused: [move the long doc of `lib::session_workspace_tag` here, verbatim]
    pub fn of_session() -> Result<Workspace> {
        let names: Vec<String> = niri::workspaces()?.into_iter().filter_map(|w| w.name).collect();
        let name = match session::current_pane().map(|p| p.session) {
            Some(s) => from_session(&s, &names)?,
            None => {
                let dirs = Dirs::from_env()?;
                let cwd = std::env::current_dir().context("cannot read the current directory")?;
                from_folder(&dirs, &cwd, &names)?
            }
        };
        Workspace::named(&name)
    }

    /// The workspace whoever is asking belongs to: [move `lib::caller_workspace`'s doc]
    pub fn of_caller() -> Result<Workspace> {
        match session::current_pane() {
            Some(_) => Workspace::of_session(),
            None => Workspace::focused(),
        }
    }

    pub fn name(&self) -> &str { &self.name }
    /// The task tag its name folds to.
    pub fn tag(&self) -> &str { &self.tag }
    /// Its herdr session's name.
    pub fn session_name(&self) -> String { session::herdr_session_name(&self.name) }
    /// Where its terminals start.
    pub fn folder(&self, dirs: &Dirs) -> PathBuf { dirs.start_dir(&self.name) }
}

/// The workspace a named herdr session was opened for, among `names`. A
/// session that matches none is an error, not a reason to try the folder: the
/// workspace was renamed, and the folder would only give a plausible guess.
fn from_session(session: &str, names: &[String]) -> Result<String> {
    session::workspace_for_session(session, names)
        .cloned()
        .with_context(|| format!("herdr session '{session}' does not match any named workspace — it may have been renamed since this terminal was opened"))
}

/// The workspace named by the `~/Projects` folder `cwd` is in, among `names`.
fn from_folder(dirs: &Dirs, cwd: &Path, names: &[String]) -> Result<String> {
    let project = dirs.project_from_cwd(cwd).context(
        "not in a named herdr session or a ~/Projects folder, so there is no terminal to take the workspace from",
    )?;
    names
        .iter()
        .find(|n| **n == project)
        .cloned()
        .with_context(|| format!("folder ~/Projects/{project} does not match any named workspace"))
}

/// Name workspace 1 if it has no name. [move `lib::workspace_default`'s doc and body]
pub fn name_default() -> Result<()> { … }
```

Compare each text against `lib.rs` before deleting it; they must match byte for byte (note today's `focused_workspace` checks emptiness then `usable_tag`, as `named` does; `session_workspace` today calls `usable_tag` on the result, as `named` does).

Tests: `named_refuses_an_empty_name_and_a_tagless_one` (both texts), `a_session_names_its_workspace_or_says_it_was_renamed` (match through `herdr_session_name`; mismatch text), `a_projects_folder_names_its_workspace_or_says_which_is_missing` (with `Dirs::at(tmp)`: `~/Projects/alpha/src` + names [alpha] → alpha; `~/Projects/beta` + [alpha] → "folder ~/Projects/beta does not match…"; `/elsewhere` → "not in a named herdr session…").

- [ ] **Step 2: Switch `lib.rs`, `main.rs`, `daemon.rs`**

`lib.rs`: only the module doc and `pub mod` lines remain (add `dirs`, `workspace`). `main.rs`: `use niri_tasks::workspace::Workspace;` and
- `Command::Tag { session }`: `let ws = if session { Workspace::of_session()? } else { Workspace::focused()? }; print!("{}", ws.tag());`
- `Move`, `Add` (text path): `let tag = Workspace::of_caller()?.tag().to_string();` (keep the comments).
- `Start` (non-`--here`), `Refine`, `Session`: `let workspace = Workspace::of_caller()?;` and pass `workspace.name()` to the existing `&str` callers for now.
- `terminal()` unchanged (Task 1 did it).
`daemon.rs`: `crate::workspace_default()` → `crate::workspace::name_default()`; the two `crate::require_workspace_tag()` → `Workspace::focused().map(|w| w.tag().to_string())`; update the `no_cards_text` doc.

Check: `grep -rn "require_workspace_tag\|session_workspace_tag\|caller_workspace\|workspace_default" src tests` prints nothing except inside `workspace.rs` docs if you kept the names in prose. `cargo build` warning grep empty; `cargo test` green.

- [ ] **Step 3: Commit**

```
feat(workspace): decide which workspace once, as a value with its tag

Nine functions at the crate root answered "which workspace" three ways
and every caller re-derived the tag. Workspace is a named workspace
whose name folds to a usable tag, built by the focused, session or
caller policy with the same refusals as before, and the session-or-
folder choice is two pure functions with tests. main.rs and the daemon
decide the policy once per command.
```

---

### Task 3: The session-opening callers take `&Workspace`

**Files:**
- Modify: `src/refine.rs` (`launch(ws: &Workspace, t, mode)`), `src/work.rs` (`launch(ws, t)`, `set_up(ws, ..)`, `set_up_here(ws, uuid)`, `repo_for(ws)`, `start_working` unchanged), `src/link.rs` (`live_agent_names(ws)`, `live_agent(ws, uuid)`, `go_to(ws, uuid)`), `src/session.rs` (`Session::for_workspace(ws: &Workspace)`), `src/main.rs` (pass `&workspace`; `--here --workspace <name>` → `Workspace::named(&name)?`), `src/daemon.rs` (agent names: `Workspace::named(name).ok().map(|w| live_agent_names(&w))`, read the call at ~:262 first)

- [ ] **Step 1: Switch the signatures**

- `Session::for_workspace(ws: &Workspace) -> Result<Session>`: `name: ws.session_name(), dir: ws.folder(&Dirs::from_env()?)`. Doc: "The session for `ws`, through the process adapter."
- `refine::launch`: replace `workspace: &str` with `ws: &Workspace`; `Session::for_workspace(ws)`; `session.new_tab(&workspaces, session.dir(), &tab_label(..), ws.name())`.
- `work::repo_for(ws: &Workspace)`: `let repo = ws.folder(&Dirs::from_env()?);` then the git check (text unchanged). `work::launch(ws, t)`: `repo_for(ws)`, `Session::for_workspace(ws)`, `new_tab(.., ws.name())`, and the `--here --workspace` command string uses `sh_quote(ws.name())`. `set_up_here(ws, uuid)` / `set_up(ws, ..)`: same substitutions.
- `link::live_agent_names(ws: &Workspace)`: `Session::named(&ws.session_name()).agent_names()`. `live_agent(ws, uuid)`, `go_to(ws, uuid)`: `Session::for_workspace(ws)`.
- `main.rs`: pass `&workspace`; the `--here` arm: `let workspace = Workspace::named(&workspace.context("--here needs --workspace")?)?; return work::set_up_here(&workspace, &uuid);`.
- `daemon.rs`: adapt the `live_agent_names` call site to build `Workspace::named(..)` from the focused niri workspace's name; an unnamed workspace yields no agents, as today an empty name did.

Check: `grep -rn "herdr_session_name\|start_dir" src | grep -v "^src/session.rs\|^src/workspace.rs\|^src/dirs.rs\|^src/project.rs\|^src/main.rs"` — only `project.rs::start` (Task 4 keeps it) and `main::terminal` may still call them. The three integration suites pass unchanged (the `--here --workspace 'alpha'` argv and all herdr argv are the same).

- [ ] **Step 2: Commit**

```
refactor: open sessions for a Workspace, not a bare name

Refine, Start working, Go to session and the pane link took the
workspace as a string and each rebuilt its herdr session name and its
folder. They take the Workspace value now, which carries both, so the
naming rules are read in one place and a caller cannot pass a name that
was never checked.
```

---

### Task 4: Split `project.rs` into project, programs and matching

**Files:**
- Create: `src/programs.rs`, `src/panel/matching.rs`
- Modify: `src/project.rs` (remove the moved items and their tests; `start` uses `programs::startup_commands(dir, &session::herdr_session_name(workspace))`), `src/lib.rs` (`pub mod programs;`), `src/panel/mod.rs` (or wherever panel's modules are declared: `pub mod matching;`), callers: `src/work.rs`, `src/speak.rs`, `src/session/herdr.rs` (`project::on_path`/`SESSION_MANAGER` → `programs::`), `src/session.rs` (`project::project_terminal_command` → `programs::`), `src/main.rs` (`project::terminal_command` → `programs::`), `src/panel/surface.rs`, `src/panel/state.rs`, `src/panel/projects.rs` (`project::fzf_matches`/`substring_matches` → `crate::panel::matching::`)

- [ ] **Step 1: `programs.rs`**

Module doc: "The programs niritasks runs beside itself: whether one is installed, and the argv that opens a terminal, the project terminal with its herdr session, and the editor." Move verbatim with docs: `EDITOR`, `on_path`, `found_in`, `SESSION_MANAGER`, `terminal_command`, `project_terminal_command`, and `startup_commands` with the new signature `pub fn startup_commands(dir: &Path, session: &str) -> Vec<Vec<String>>` (the body no longer calls `herdr_session_name`; doc says `session` is the herdr session name, `session::herdr_session_name`). Move their tests (`found_in`, any argv tests). `programs.rs` must import nothing from the crate.

- [ ] **Step 2: `panel/matching.rs`**

Module doc: "Ranking the project list's folders against what is typed: fzf when installed, a case-blind substring match otherwise. The panel's `Matcher` is one of these two." Move `MATCHER`, `fzf_matches`, `substring_matches` and their tests verbatim. Update the doc references in `panel/surface.rs:58`, `panel/projects.rs:25`.

- [ ] **Step 3: Slim `project.rs`**

Module doc trimmed to what stays (drop "Also here: which programs a freshly-opened project workspace starts with."). `start(dir, workspace)` becomes `programs::startup_commands(dir, &crate::session::herdr_session_name(workspace))` and keeps its doc. Fix every caller listed above.

Check: `wc -l src/project.rs` ≈ 520 or less; `grep -rn "project::\(on_path\|SESSION_MANAGER\|EDITOR\|terminal_command\|project_terminal_command\|startup_commands\|fzf_matches\|substring_matches\|MATCHER\)" src` prints nothing. `cargo build` warning grep empty; `cargo test` green.

- [ ] **Step 4: Commit**

```
refactor(project): split programs and matching out of project.rs

project.rs held the ~/Projects list with open and move, typed-name
resolution, fzf and substring matching, the PATH lookup and the ghostty,
herdr and VS Code argv. programs.rs takes the external programs, with
startup_commands taking the session name so it depends on nothing in
the crate; panel/matching.rs takes the two matchers beside the panel's
Matcher type. Session no longer depends on project, so the cycle the
session review noted is gone.
```

---

### Task 5: Docs

**Files:** `CONTEXT.md` (**Workspace tag** entry)

- [ ] Append to the **Workspace tag** entry one sentence: "In the code, `workspace::Workspace` is a named workspace whose name folds to a usable tag, built once per command by the focused, session or caller policy; `dirs::Dirs` is where its folder, and every other path, comes from." Read the entry first and match its voice. Run `cargo test --bin niritasks 2>&1 | tail -3`.
- [ ] Commit: `docs: name the Workspace and Dirs values in CONTEXT.md`.

---

## Self-review notes

- Spec coverage: decision 1 → Task 1; 2 → Task 2; 3 → Task 3; 4 → Task 4; docs → Task 5.
- Intermediate states: after Task 1, `lib.rs::session_workspace` uses `Dirs`; after Task 2, callers still take `&str` and main.rs passes `ws.name()`; after Task 3, `Workspace` flows through; after Task 4, `project.rs` imports `programs` and `session`, `session` imports `programs` only.
- Error texts: five texts move from `lib.rs` to `workspace.rs` (empty name, tagless, session mismatch, not-in-folder, folder-no-workspace); `work::repo_for`'s git text and `Dirs::from_env`'s "HOME is unset" are unchanged. The integration suites pin the argv.
