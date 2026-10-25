# Back Up and Restore Tasks Across a Reinstall Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `niritasks export [PATH]` saves every Taskwarrior task and the refine reports folder to one `.tar.gz`; `niritasks import <ARCHIVE>` puts them back into a freshly installed Taskwarrior 3.5.

**Architecture:** A new `src/backup.rs` owns the archive: it stages `tasks.json` (the whole store from `task rc.hooks=0 export`) and a WAL-safe `sqlite3 .backup` copy of `taskchampion.sqlite3` in a scratch folder, then has `tar` pack them with the reviews folder. Import unpacks with `tar`, refuses a non-empty store without `--force`, runs `task rc.hooks=0 import tasks.json`, and copies `reviews/` back without overwriting. The two Taskwarrior calls it needs (`export_all`, `import_file`) go in `src/task.rs`, the only module that talks to `task`. Two top-level clap commands in `src/main.rs` print the results.

**Tech Stack:** Rust 2021 (rustc 1.95), clap 4 derive, anyhow, serde_json; external `task` 3.5.0, `tar` (GNU), `sqlite3` 3.46 — all three already on the machine, no new crates.

**Spec:** Taskwarrior task `3caa9b56-f748-48f8-917c-ed15e49dd66c` — its description and notes (`task rc.json.array=on 3caa9b56 export`). No separate spec file.

## Global Constraints

- The live store is Taskwarrior 3.5.0's `~/.task/taskchampion.sqlite3` (`data.location=$HOME/.task`). Find it with `task::data_location()` (`task _get rc.data.location`), never a hard-coded path.
- Never read, back up or import `~/.task-2.6/` or `~/.task-2.6-backup-20261009/`.
- The live `~/.task` is only ever **read**. Every test and check that imports points `TASKRC` and `TASKDATA` (and `HOME`/`XDG_DATA_HOME`) at a scratch folder.
- One `.tar.gz` holding `tasks.json` (`task rc.hooks=0 export`, every status), `taskchampion.sqlite3` (a copy taken with `sqlite3 .backup`, never a raw `cp`) and `reviews/`. JSON is what restores; the sqlite copy is a fallback restored by hand as the README says.
- Commands are top-level: `niritasks export [PATH]` and `niritasks import <ARCHIVE>`, not under `task`.
- `export` defaults to `~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz` (local time) and prints the path and the task count.
- `export` lists every `Report:` or `Report (earlier draft):` note whose file is missing — task uuid8, status and path, `#anchor` stripped before checking — as a warning that does not stop the backup.
- `import` refuses when the store already holds tasks unless `--force`; runs `task rc.hooks=0 import tasks.json`; copies `reviews/` back without overwriting files already there; prints the count imported.
- The reviews folder comes from `refine::reports_dir(&dirs)` (`$XDG_DATA_HOME/niri-tasks/reviews`, else `~/.local/share/niri-tasks/reviews`), with `Dirs` from `dirs::Dirs::from_env()`.
- Out of scope: git sync (task b1bc5fe7), 2.6 data, scheduled backups, pushing or checking worktree branches from `export`, backing up `~/.taskrc` or the rest of `~/.local/share/niri-tasks`.
- Commits: Conventional Commits with the repo's scopes, e.g. `feat(backup): …`, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` trailer.

## Facts checked on 2026-10-25 (so you need not)

- `task rc.hooks=0 rc.json.array=on rc.gc=off export` with no filter exports every status (269 tasks: 82 pending, 144 completed, 43 deleted). On an empty store it prints `[` `]` and exits 0.
- `task rc.hooks=0 import <file>` of that JSON into an empty scratch store, exported again, gives identical objects for every task — every field, `id` and `urgency` included. A missing file exits 2.
- `sqlite3 <db> ".backup <file>"` works on the live store while its `-wal` is present.
- GNU `tar -f` is resolved against the starting directory, but `-C` moves later member names; the code passes an absolute archive path anyway.

## File Structure

- Create `src/backup.rs` — the archive: what goes in, how it is made and unpacked, the missing-report check, the no-overwrite copy. Unit tests for its pure parts at the bottom.
- Modify `src/lib.rs` — `pub mod backup;`.
- Modify `src/task.rs` — `export_all()` and `import_file()`, beside `export_values()` and `data_location()`.
- Modify `src/main.rs` — `Command::Export` / `Command::Import`, their dispatch and two small printing functions.
- Create `tests/backup.rs` — the binary run against sandboxed stores: export contents, missing report listed, round trip, guard, `--force`.
- Modify `README.md` — requirements line, two command lines, a "Backing up before a reinstall" section.
- Modify `llms.txt` — two command rows.

---

### Task 1: The backup module's pure helpers

**Files:**
- Create: `src/backup.rs`
- Modify: `src/lib.rs` (add `pub mod backup;` in alphabetical order, after `pub mod actions;`)

**Interfaces:**
- Consumes: `crate::dirs::Dirs` (`Dirs::at(&Path)`, `.home()`), `crate::refine::{reports_dir, report_file}`, `crate::task::Task` (`report_notes()`, `uuid`, `status`), `crate::names::uuid8(&str) -> String`.
- Produces:
  - `pub fn default_archive(dirs: &Dirs, stamp: &str) -> PathBuf`
  - `pub fn local_stamp() -> Result<String>`
  - `pub struct MissingReport { pub uuid8: String, pub status: String, pub path: String }` with `pub fn line(&self) -> String`
  - `pub fn missing_reports(tasks: &[Task], dirs: &Dirs) -> Vec<MissingReport>`
  - `fn copy_missing(from: &Path, to: &Path) -> Result<(usize, usize)>` (private; `(copied, kept)`)
  - consts `TASKS_JSON = "tasks.json"`, `STORE = "taskchampion.sqlite3"`, `REVIEWS = "reviews"`

- [ ] **Step 1: Write the failing tests**

Create `src/backup.rs` with only the module doc, the imports and the tests, so the test names fail to resolve:

```rust
//! `niritasks export` and `niritasks import`: every task, and the refine
//! reports their notes link, in one archive to carry across a reinstall.
//!
//! The archive is a `.tar.gz` of `tasks.json`, the whole store as `task
//! export` writes it and what import reads back; `taskchampion.sqlite3`, a
//! copy of the store taken through SQLite's own backup, to restore by hand if
//! the JSON will not import; and `reviews/`, the reports folder. `tar` and
//! `sqlite3` do the packing and the copy.

use crate::{dirs::Dirs, names, refine, task::Task};
use anyhow::{ensure, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("niritasks-backup-unit-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn task(json: &str) -> Task {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_default_archive_is_in_the_home_folder_named_by_the_minute() {
        assert_eq!(
            default_archive(&Dirs::at(Path::new("/home/x")), "20261025-1342"),
            PathBuf::from("/home/x/niri-tasks-backup-20261025-1342.tar.gz")
        );
    }

    #[test]
    fn the_local_stamp_is_a_date_and_a_minute() {
        let stamp = local_stamp().unwrap();
        assert_eq!(stamp.len(), 13, "{stamp}");
        assert_eq!(&stamp[8..9], "-", "{stamp}");
        assert!(stamp.chars().filter(|c| *c != '-').all(|c| c.is_ascii_digit()), "{stamp}");
    }

    /// Both kinds of report note count, an `#anchor` is dropped before the
    /// file is looked for, and a `Plan:` note is not a report.
    #[test]
    fn a_report_note_whose_file_is_gone_is_listed_without_its_anchor() {
        let home = scratch("missing");
        let dirs = Dirs::at(&home);
        let reviews = refine::reports_dir(&dirs);
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(reviews.join("here.html"), "<html>").unwrap();
        let t = task(
            r#"{"uuid":"3caa9b56-f748-48f8-917c-ed15e49dd66c","description":"d","status":"completed","annotations":[
                {"entry":"20261001T000000Z","description":"Report: ~/.local/share/niri-tasks/reviews/here.html#slide-3"},
                {"entry":"20261002T000000Z","description":"Report (earlier draft): gone.html#s1"},
                {"entry":"20261003T000000Z","description":"Plan: docs/superpowers/plans/nowhere.md"}]}"#,
        );
        let missing = missing_reports(&[t], &dirs);
        assert_eq!(
            missing,
            vec![MissingReport { uuid8: "3caa9b56".into(), status: "completed".into(), path: "gone.html".into() }]
        );
        assert_eq!(missing[0].line(), "3caa9b56  completed  gone.html");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn copy_missing_copies_what_is_not_there_and_keeps_what_is() {
        let dir = scratch("copy");
        let (from, to) = (dir.join("from"), dir.join("to"));
        std::fs::create_dir_all(from.join("sub")).unwrap();
        std::fs::write(from.join("a.html"), "backed up").unwrap();
        std::fs::write(from.join("sub/b.md"), "# b").unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(to.join("a.html"), "mine").unwrap();

        assert_eq!(copy_missing(&from, &to).unwrap(), (1, 1));
        assert_eq!(std::fs::read_to_string(to.join("a.html")).unwrap(), "mine");
        assert_eq!(std::fs::read_to_string(to.join("sub/b.md")).unwrap(), "# b");

        // An archive with no reviews folder copies nothing.
        assert_eq!(copy_missing(&dir.join("absent"), &to).unwrap(), (0, 0));
        std::fs::remove_dir_all(&dir).ok();
    }
}
```

Add `pub mod backup;` to `src/lib.rs` after `pub mod actions;`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --lib backup::`
Expected: compile errors — `cannot find function default_archive`, `local_stamp`, `missing_reports`, `copy_missing`, `cannot find struct MissingReport`.

- [ ] **Step 3: Write the helpers**

Insert between the `use` lines and `#[cfg(test)]` in `src/backup.rs`:

```rust
/// The archive's task list, what import reads.
const TASKS_JSON: &str = "tasks.json";

/// Taskwarrior 3's store, by its own file name: in the data location, and in
/// the archive.
const STORE: &str = "taskchampion.sqlite3";

/// The reports folder's name, under `niri-tasks` in the data folder and at
/// the archive's top level.
const REVIEWS: &str = "reviews";

/// A `Report:` or `Report (earlier draft):` note whose file is not there:
/// backed up without it, and dangling after a restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingReport {
    pub uuid8: String,
    /// The task's status, as `task export` writes it.
    pub status: String,
    /// As the note writes it, any `#anchor` dropped.
    pub path: String,
}

impl MissingReport {
    /// One line of export's warning: uuid8, status, path.
    pub fn line(&self) -> String {
        format!("{}  {:<9}  {}", self.uuid8, self.status, self.path)
    }
}

/// `~/niri-tasks-backup-<stamp>.tar.gz`, `stamp` as [`local_stamp`] writes it.
pub fn default_archive(dirs: &Dirs, stamp: &str) -> PathBuf {
    dirs.home().join(format!("niri-tasks-backup-{stamp}.tar.gz"))
}

/// Now, in local time, as `YYYYMMDD-HHMM`: what the default archive is named
/// by. From `date`, since the rest of this tool only ever needs UTC.
pub fn local_stamp() -> Result<String> {
    let out = Command::new("date").arg("+%Y%m%d-%H%M").output().context("could not run `date`")?;
    ensure!(out.status.success(), "`date` failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Every report note, on any task, whose file is not there. A note's path is
/// found as [`refine::report_file`] finds it, once its `#anchor` is dropped.
pub fn missing_reports(tasks: &[Task], dirs: &Dirs) -> Vec<MissingReport> {
    tasks
        .iter()
        .flat_map(|t| {
            t.report_notes().into_iter().filter_map(move |n| {
                let path = n.path.split('#').next().unwrap_or_default().trim().to_string();
                (!refine::report_file(&path, dirs).is_file()).then(|| MissingReport {
                    uuid8: names::uuid8(&t.uuid),
                    status: t.status.clone(),
                    path,
                })
            })
        })
        .collect()
}

/// Copy every file under `from` to the same place under `to`, making folders,
/// bar a file `to` already has, which is left as it is. Answers how many
/// were copied and how many kept. A `from` that is not there copies nothing.
fn copy_missing(from: &Path, to: &Path) -> Result<(usize, usize)> {
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok((0, 0));
    };
    let (mut copied, mut kept) = (0, 0);
    for entry in entries {
        let entry = entry.with_context(|| format!("could not read {}", from.display()))?;
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type()?.is_dir() {
            let (c, k) = copy_missing(&src, &dst)?;
            copied += c;
            kept += k;
        } else if dst.exists() {
            kept += 1;
        } else {
            std::fs::create_dir_all(to).with_context(|| format!("could not make {}", to.display()))?;
            std::fs::copy(&src, &dst).with_context(|| format!("could not copy {}", dst.display()))?;
            copied += 1;
        }
    }
    Ok((copied, kept))
}
```

`TASKS_JSON`, `STORE` and `REVIEWS` are unused until Task 2; the dead-code warning is expected for this one commit.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --lib backup::`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add src/backup.rs src/lib.rs
git commit -m "feat(backup): add the archive's name, missing-report check and copy

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `niritasks export`

**Files:**
- Modify: `src/task.rs` (add `export_all` after `export_values`, near line 432)
- Modify: `src/backup.rs` (add `Stage`, `Exported`, `copy_store`, `export`)
- Modify: `src/main.rs` (Command enum ~line 26-52, `dispatch` ~line 330, imports line 11-15)
- Create: `tests/backup.rs`

**Interfaces:**
- Consumes: Task 1's `TASKS_JSON`, `STORE`, `REVIEWS`, `missing_reports`, `MissingReport::line`, `default_archive`, `local_stamp`; `task::data_location() -> Result<PathBuf>`; `task::Task`.
- Produces:
  - `task::export_all() -> Result<Vec<serde_json::Value>>` — every task, any status, hooks and gc off; a failed `task` is an error.
  - `backup::Stage` (private) — `Stage::new(what: &str) -> Result<Stage>`, `.0: PathBuf`, removed on drop.
  - `pub struct Exported { pub archive: PathBuf, pub tasks: usize, pub missing: Vec<MissingReport> }`
  - `pub fn export(dirs: &Dirs, archive: &Path) -> Result<Exported>`
  - CLI: `niritasks export [PATH]`; stdout `Backed up <n> tasks to <absolute path>`; stderr warning block when report files are missing.
  - `tests/backup.rs`'s `Sandbox` and `filled()` fixture, which Task 3 adds tests to.

- [ ] **Step 1: Write the failing integration test**

Create `tests/backup.rs`:

```rust
//! `niritasks export` and `niritasks import`, run as the binary against
//! sandboxed task stores.
//!
//! Each sandbox has its own `TASKRC`, `TASKDATA` and `HOME`, so nothing here
//! reads or writes the real `~/.task` or reviews folder. Every test makes its
//! own sandboxes under its own names, so they can run in parallel.

use serde_json::Value;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("niritasks-backup-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["data", "home", "bin"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        std::fs::write(dir.join("taskrc"), format!("data.location={}/data\n", dir.display())).unwrap();
        // So a refusal's notification does not pop up on the desktop.
        let fake = dir.join("bin/notify-send");
        std::fs::write(&fake, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir }
    }

    fn reviews(&self) -> PathBuf {
        self.dir.join("home/.local/share/niri-tasks/reviews")
    }

    fn task(&self, args: &[&str]) -> Output {
        Command::new("task")
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .args(["rc.verbose=nothing", "rc.confirmation=off", "rc.hooks=0"])
            .args(args)
            .output()
            .expect("task")
    }

    /// Add a task from word-split `args`, answering its uuid.
    fn add(&self, args: &[&str]) -> String {
        assert!(self.task(&[&["add"][..], args].concat()).status.success(), "task add");
        let uuid = String::from_utf8(self.task(&["+LATEST", "_uuids"]).stdout).unwrap().trim().to_string();
        assert_eq!(uuid.len(), 36, "one new task, by uuid: {uuid:?}");
        uuid
    }

    fn niritasks(&self, args: &[&str]) -> Output {
        let path = format!("{}:{}", self.dir.join("bin").display(), std::env::var("PATH").unwrap());
        Command::new(env!("CARGO_BIN_EXE_niritasks"))
            .args(args)
            .env("PATH", path)
            .env("HOME", self.dir.join("home"))
            .env_remove("XDG_DATA_HOME")
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .output()
            .expect("run niritasks")
    }

    /// Every task as taskwarrior exports it, by uuid, less `id` and
    /// `urgency`: the working-set number and a figure that drifts with age.
    fn tasks(&self) -> BTreeMap<String, Value> {
        let out = self.task(&["rc.json.array=on", "rc.gc=off", "export"]);
        let list: Vec<Value> = serde_json::from_slice(&out.stdout).expect("task export JSON");
        list.into_iter()
            .map(|mut t| {
                let o = t.as_object_mut().unwrap();
                o.remove("id");
                o.remove("urgency");
                let uuid = o["uuid"].as_str().unwrap().to_string();
                (uuid, t)
            })
            .collect()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A store of three tasks, one per status, the pending one tagged, with a
/// priority and two report notes: one whose file is there behind an anchor,
/// one whose file is gone. Two report files in its reviews folder, one in a
/// subfolder. Answers the sandbox and the pending task's uuid.
fn filled(name: &str) -> (Sandbox, String) {
    let s = Sandbox::new(name);
    std::fs::create_dir_all(s.reviews().join("sub")).unwrap();
    std::fs::write(s.reviews().join("kept.html"), "backed up").unwrap();
    std::fs::write(s.reviews().join("sub/deep.md"), "# deep").unwrap();

    let pending = s.add(&["feat:", "pending", "one", "+alpha", "priority:H"]);
    let notes = [
        "Report: ~/.local/share/niri-tasks/reviews/kept.html#slide-2",
        "Report (earlier draft): gone.html#s3",
    ];
    for note in notes {
        assert!(s.task(&[&pending, "annotate", "--", note]).status.success(), "annotate");
    }
    let done = s.add(&["fix:", "finished", "one"]);
    assert!(s.task(&[&done, "done"]).status.success(), "done");
    let gone = s.add(&["chore:", "removed", "one"]);
    assert!(s.task(&[&gone, "delete"]).status.success(), "delete");
    (s, pending)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn export_writes_one_archive_and_lists_the_missing_report() {
    let (s, pending) = filled("export");
    let archive = s.dir.join("backup.tar.gz");
    let out = s.niritasks(&["export", archive.to_str().unwrap()]);
    let (stdout, stderr) = (text(&out.stdout), text(&out.stderr));
    assert!(out.status.success(), "export failed: {stderr}");
    assert!(stdout.contains(&format!("Backed up 3 tasks to {}", archive.display())), "{stdout}");

    // The gone report is listed by uuid8, status and path, anchor dropped;
    // the one that is there is not.
    assert!(stderr.contains(&format!("{}  pending    gone.html", &pending[..8])), "{stderr}");
    assert!(!stderr.contains("kept.html"), "{stderr}");

    let listing = text(&Command::new("tar").arg("-tzf").arg(&archive).output().unwrap().stdout);
    for member in ["tasks.json", "taskchampion.sqlite3", "reviews/kept.html", "reviews/sub/deep.md"] {
        assert!(listing.lines().any(|l| l.trim_end_matches('/') == member), "{member} not in:\n{listing}");
    }

    // The sqlite copy is a whole store, not a main file missing its WAL.
    let unpacked = s.dir.join("unpacked");
    std::fs::create_dir_all(&unpacked).unwrap();
    assert!(Command::new("tar").arg("-xzf").arg(&archive).arg("-C").arg(&unpacked).status().unwrap().success());
    let count = Command::new("sqlite3")
        .arg(unpacked.join("taskchampion.sqlite3"))
        .arg("select count(*) from tasks")
        .output()
        .expect("sqlite3");
    assert_eq!(text(&count.stdout).trim(), "3");
    let json: Vec<Value> = serde_json::from_slice(&std::fs::read(unpacked.join("tasks.json")).unwrap()).unwrap();
    assert_eq!(json.len(), 3);

    // A second export to the same file is refused, the first left alone.
    let again = s.niritasks(&["export", archive.to_str().unwrap()]);
    assert!(!again.status.success(), "a second export overwrote the archive");
    assert!(text(&again.stderr).contains("already there"), "{}", text(&again.stderr));
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --test backup`
Expected: FAIL — `export failed: error: unrecognized subcommand 'export'`.

- [ ] **Step 3: Add `task::export_all`**

In `src/task.rs`, directly after `fn export_values` ends (the `serde_json::from_str(trimmed)…` line and its closing brace, ~line 432):

```rust
/// Every task in the store, whatever its status, exactly as taskwarrior
/// wrote it: what `niritasks export` backs up, and how `niritasks import`
/// tells an empty store. Hooks off, so no hook runs or prints into the JSON.
/// Unlike [`export_values`], a `task` that fails is an error, never an empty
/// store: a backup of nothing would look like a backup.
pub fn export_all() -> Result<Vec<Value>> {
    let out = base()
        .args(["rc.hooks=0", "rc.gc=off", "export"])
        .output()
        .context("could not run `task` — is taskwarrior installed?")?;
    anyhow::ensure!(
        out.status.success(),
        "`task export` failed: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    serde_json::from_slice(&out.stdout).context("could not parse `task export` output as JSON")
}
```

- [ ] **Step 4: Add `Stage`, `copy_store` and `export` to `src/backup.rs`**

Change the `use crate::…` line to `use crate::{dirs::Dirs, names, refine, task, task::Task};` and the anyhow line to `use anyhow::{bail, ensure, Context, Result};`. Then add, after `copy_missing`:

```rust
/// What an export wrote.
#[derive(Debug)]
pub struct Exported {
    /// The archive, as an absolute path.
    pub archive: PathBuf,
    /// How many tasks it holds, every status.
    pub tasks: usize,
    /// Report notes whose file is gone, backed up without it.
    pub missing: Vec<MissingReport>,
}

/// Write every task, the store's sqlite copy and the reviews folder to
/// `archive`, which must not be there yet. The store is only read.
pub fn export(dirs: &Dirs, archive: &Path) -> Result<Exported> {
    let archive = std::path::absolute(archive).with_context(|| format!("could not resolve {}", archive.display()))?;
    if archive.exists() {
        bail!("{} is already there; name another file", archive.display());
    }
    let values = task::export_all()?;
    let tasks = values
        .iter()
        .map(|v| serde_json::from_value::<Task>(v.clone()))
        .collect::<Result<Vec<_>, _>>()
        .context("could not parse a task from `task export`")?;
    let store = task::data_location()?.join(STORE);
    ensure!(store.is_file(), "no {STORE} in {}: export backs up a Taskwarrior 3 store", store.parent().unwrap_or(&store).display());

    let stage = Stage::new("export")?;
    std::fs::write(stage.0.join(TASKS_JSON), serde_json::to_vec_pretty(&values)?)
        .context("could not write tasks.json")?;
    copy_store(&store, &stage.0)?;

    let mut tar = Command::new("tar");
    tar.arg("-czf").arg(&archive).arg("-C").arg(&stage.0).args([TASKS_JSON, STORE]);
    let reviews = refine::reports_dir(dirs);
    if reviews.is_dir() {
        tar.arg("-C").arg(reviews.parent().context("the reviews folder has no parent")?).arg(REVIEWS);
    }
    let out = tar.output().context("could not run `tar`")?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&archive);
        bail!("`tar` could not write {}: {}", archive.display(), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(Exported { archive, tasks: tasks.len(), missing: missing_reports(&tasks, dirs) })
}

/// Copy the store into `into` with SQLite's online backup, never a plain
/// copy: Taskwarrior keeps its latest writes in the `-wal` file beside it,
/// which a copy of the main file alone would leave out. Run in `into`, so
/// the target needs no quoting in the dot-command.
fn copy_store(store: &Path, into: &Path) -> Result<()> {
    let out = Command::new("sqlite3")
        .arg(store)
        .arg(format!(".backup {STORE}"))
        .current_dir(into)
        .output()
        .context("could not run `sqlite3` — is it installed?")?;
    ensure!(
        out.status.success() && into.join(STORE).is_file(),
        "sqlite3 could not back up {}: {}",
        store.display(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(())
}

/// A scratch folder under the temp folder, removed when dropped.
struct Stage(PathBuf);

impl Stage {
    fn new(what: &str) -> Result<Stage> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("niritasks-{what}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).with_context(|| format!("could not make {}", dir.display()))?;
        Ok(Stage(dir))
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
```

- [ ] **Step 5: Add the `Export` command to `src/main.rs`**

Add `backup,` to the `use niri_tasks::{ … }` list (alphabetical: `actions::{Action, TaskState}, backup, dirs, …`).

In `enum Command`, between `Project(ProjectCommand),` and `/// Open a terminal …`:

```rust
    /// Save every task, whatever its status, and the refine reports to one archive, before a reinstall
    ///
    /// The archive holds tasks.json (what import reads), a copy of
    /// taskchampion.sqlite3 to restore by hand if that fails, and the reviews
    /// folder. Report notes whose file is gone are listed, and backed up
    /// without it. Push every task worktree branch first: their Plan: and
    /// Spec: files live only there.
    Export {
        /// Where to write it; by default ~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz
        path: Option<PathBuf>,
    },
```

In `dispatch`, after the `Command::Project(…)` arm:

```rust
        Command::Export { path } => return backup_export(path),
```

After `fn dispatch` (before `fn task_command`):

```rust
/// `niritasks export`: the archive and its task count on stdout, and any
/// report notes whose file is gone on stderr, as a warning.
fn backup_export(path: Option<PathBuf>) -> Result<()> {
    let dirs = dirs::Dirs::from_env()?;
    let path = match path {
        Some(p) => p,
        None => backup::default_archive(&dirs, &backup::local_stamp()?),
    };
    let done = backup::export(&dirs, &path)?;
    println!("Backed up {} tasks to {}", done.tasks, done.archive.display());
    if !done.missing.is_empty() {
        eprintln!(
            "{} report notes name a file that is gone, backed up without it:",
            done.missing.len()
        );
        for m in &done.missing {
            eprintln!("  {}", m.line());
        }
    }
    Ok(())
}
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --test backup && cargo test --lib backup::`
Expected: `export_writes_one_archive_and_lists_the_missing_report … ok`; the 4 unit tests still pass; no dead-code warnings for the consts.

- [ ] **Step 7: Commit**

```bash
git add src/task.rs src/backup.rs src/main.rs tests/backup.rs
git commit -m "feat(backup): add niritasks export

Write every task, a sqlite .backup copy of the store and the reviews
folder to one .tar.gz, and list report notes whose file is gone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: `niritasks import`

**Files:**
- Modify: `src/task.rs` (add `import_file` after `export_all`)
- Modify: `src/backup.rs` (add `Imported`, `import`)
- Modify: `src/main.rs` (Command enum, `dispatch`, `backup_import`)
- Modify: `tests/backup.rs` (two tests)

**Interfaces:**
- Consumes: Task 2's `task::export_all`, `Stage`, `TASKS_JSON`, `REVIEWS`; Task 1's `copy_missing`; `tests/backup.rs`'s `Sandbox`, `filled`, `text`.
- Produces:
  - `task::import_file(path: &Path) -> Result<()>`
  - `pub struct Imported { pub tasks: usize, pub reports: usize, pub kept: usize }`
  - `pub fn import(dirs: &Dirs, archive: &Path, force: bool) -> Result<Imported>`
  - CLI: `niritasks import <ARCHIVE> [--force]`; stdout `Imported <n> tasks; restored <r> report files, kept <k> already there`; on a non-empty store without `--force`, exit 1 with a message naming `--force`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/backup.rs`:

```rust
/// Export the `filled` store, answering the archive's path.
fn exported(s: &Sandbox) -> PathBuf {
    let archive = s.dir.join("backup.tar.gz");
    let out = s.niritasks(&["export", archive.to_str().unwrap()]);
    assert!(out.status.success(), "export failed: {}", text(&out.stderr));
    archive
}

#[test]
fn import_into_an_empty_store_gives_back_every_task_and_report() {
    let (src, _) = filled("roundtrip-src");
    let archive = exported(&src);
    let dst = Sandbox::new("roundtrip-dst");
    // A report the new machine already has is not overwritten.
    std::fs::create_dir_all(dst.reviews()).unwrap();
    std::fs::write(dst.reviews().join("kept.html"), "mine").unwrap();

    let out = dst.niritasks(&["import", archive.to_str().unwrap()]);
    let stdout = text(&out.stdout);
    assert!(out.status.success(), "import failed: {}", text(&out.stderr));
    assert!(
        stdout.contains("Imported 3 tasks; restored 1 report files, kept 1 already there"),
        "{stdout}"
    );

    // Same uuids, statuses, tags, notes and dates: every field but id and urgency.
    assert_eq!(dst.tasks(), src.tasks());
    assert_eq!(std::fs::read_to_string(dst.reviews().join("kept.html")).unwrap(), "mine");
    assert_eq!(std::fs::read_to_string(dst.reviews().join("sub/deep.md")).unwrap(), "# deep");
}

#[test]
fn import_refuses_a_store_with_tasks_unless_forced() {
    let (s, pending) = filled("guard");
    let archive = exported(&s);
    // An edit made after the backup, which the guard is there to keep.
    assert!(s.task(&[&pending, "modify", "feat: newer edit"]).status.success(), "modify");
    let description = |s: &Sandbox| s.tasks()[&pending]["description"].as_str().unwrap().to_string();

    let out = s.niritasks(&["import", archive.to_str().unwrap()]);
    assert!(!out.status.success(), "import into a store with tasks was not refused");
    assert!(text(&out.stderr).contains("--force"), "{}", text(&out.stderr));
    assert_eq!(description(&s), "feat: newer edit");

    let forced = s.niritasks(&["import", "--force", archive.to_str().unwrap()]);
    assert!(forced.status.success(), "import --force failed: {}", text(&forced.stderr));
    assert_eq!(description(&s), "feat: pending one");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --test backup import_`
Expected: both FAIL — `import failed: error: unrecognized subcommand 'import'` and `import --force failed: …unrecognized subcommand`.

- [ ] **Step 3: Add `task::import_file`**

In `src/task.rs`, after `export_all`:

```rust
/// Import a file of tasks as `task export` writes them, hooks off so no
/// on-add or on-modify hook fires for a task coming back. A task whose uuid
/// the store already holds is replaced by the file's.
pub fn import_file(path: &std::path::Path) -> Result<()> {
    let out = base()
        .arg("rc.hooks=0")
        .arg("import")
        .arg(path)
        .output()
        .context("could not run `task` — is taskwarrior installed?")?;
    anyhow::ensure!(
        out.status.success(),
        "`task import` failed: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(())
}
```

- [ ] **Step 4: Add `Imported` and `import` to `src/backup.rs`**

After `export`:

```rust
/// What an import did.
#[derive(Debug)]
pub struct Imported {
    /// How many tasks the archive held, every one imported.
    pub tasks: usize,
    /// Report files copied into the reviews folder.
    pub reports: usize,
    /// Report files left alone, the reviews folder already having one there.
    pub kept: usize,
}

/// Put an [`export`]'s tasks and reports back. Refused when the store
/// already holds a task, unless `force`: an import replaces any task the
/// backup also has, so an old backup would undo newer edits.
pub fn import(dirs: &Dirs, archive: &Path, force: bool) -> Result<Imported> {
    ensure!(archive.is_file(), "no archive at {}", archive.display());
    let held = task::export_all()?.len();
    if held > 0 && !force {
        bail!(
            "The task store already holds {held} tasks, and an import would replace any the backup also has with its older copy. Import into an empty store, or pass --force to import anyway."
        );
    }

    let stage = Stage::new("import")?;
    let out = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(&stage.0)
        .output()
        .context("could not run `tar`")?;
    ensure!(
        out.status.success(),
        "`tar` could not unpack {}: {}",
        archive.display(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    let json = stage.0.join(TASKS_JSON);
    ensure!(json.is_file(), "{} has no {TASKS_JSON}: not a niritasks export", archive.display());
    let tasks = serde_json::from_slice::<Vec<serde_json::Value>>(
        &std::fs::read(&json).context("could not read tasks.json")?,
    )
    .context("could not parse tasks.json")?
    .len();
    if tasks > 0 {
        task::import_file(&json)?;
    }

    let (reports, kept) = copy_missing(&stage.0.join(REVIEWS), &refine::reports_dir(dirs))?;
    Ok(Imported { tasks, reports, kept })
}
```

- [ ] **Step 5: Add the `Import` command to `src/main.rs`**

Change `use std::path::PathBuf;` to `use std::path::{Path, PathBuf};` (if `Path` is already imported elsewhere in the file, leave the line alone).

In `enum Command`, after the `Export { … },` variant:

```rust
    /// Put an export's tasks and refine reports back, after a reinstall
    ///
    /// Refused when the task store already holds tasks, unless --force: an
    /// older backup would replace newer edits. Report files already in the
    /// reviews folder are kept.
    Import {
        /// The .tar.gz niritasks export wrote
        archive: PathBuf,
        /// Import into a store that already holds tasks; a task in both takes the backup's copy
        #[arg(long)]
        force: bool,
    },
```

In `dispatch`, after the `Command::Export` arm:

```rust
        Command::Import { archive, force } => return backup_import(&archive, force),
```

After `fn backup_export`:

```rust
/// `niritasks import`: how many tasks came back, and how many report files.
fn backup_import(archive: &Path, force: bool) -> Result<()> {
    let done = backup::import(&dirs::Dirs::from_env()?, archive, force)?;
    println!(
        "Imported {} tasks; restored {} report files, kept {} already there",
        done.tasks, done.reports, done.kept
    );
    Ok(())
}
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --test backup`
Expected: 3 passed.

Then the whole suite: `cargo test`
Expected: every test passes (unit, `write_path`, `link_pane`, `typed_task_number`, `backup`).

- [ ] **Step 7: Commit**

```bash
git add src/task.rs src/backup.rs src/main.rs tests/backup.rs
git commit -m "feat(backup): add niritasks import

Import an export's tasks.json and copy its reviews back without
overwriting, refusing a store that already holds tasks unless --force.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Document both commands

**Files:**
- Modify: `README.md` (Requirements ~line 198, Commands block ~line 412, new section before `## Notes` ~line 489)
- Modify: `llms.txt` (command table, before the `niritasks terminal` row, line 61)

**Interfaces:**
- Consumes: the CLI and output strings from Tasks 2-3.
- Produces: docs only.

- [ ] **Step 1: Requirements**

In `README.md`'s `### Requirements`, after the first paragraph (ending `and says so once.`), add a paragraph:

```markdown
`niritasks export` and `import` need `tar` and `sqlite3`
(`sudo apt install sqlite3`).
```

- [ ] **Step 2: Commands block**

In the `## Commands` code block, insert before the `niritasks terminal` line:

```
niritasks export [<path>]          # every task, any status, and the reviews folder to one .tar.gz, by default
                                   #   ~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz; lists report notes whose file is gone
niritasks import <archive>         # put an export back, into an empty store (see Backing up before a reinstall)
niritasks import <archive> --force #   into a store that already holds tasks; a task in both takes the backup's copy
```

- [ ] **Step 3: The section**

Insert before `## Notes`:

````markdown
## Backing up before a reinstall

Before a clean install:

1. **Push every task worktree branch.** A task's `Plan:` and `Spec:` notes
   name files in its repo's `docs/superpowers/`, and until the branch lands
   they live only on that branch. In each repo,
   `git branch --list 'task/*'` lists them; `git push origin <branch>` each
   one. `export` does not do this for you.
2. **Run `niritasks export`.** It writes every task, whatever its status,
   to `~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz` (or the path you give),
   and prints the path and how many tasks it holds. The archive holds
   `tasks.json`, the store as `task export` writes it; `taskchampion.sqlite3`,
   a copy of the store taken through SQLite's own backup; and `reviews/`,
   the refine reports folder. It then lists each `Report:` or
   `Report (earlier draft):` note whose file is already gone — the task's
   uuid8, status and path. Those notes will dangle after the restore
   whatever you do; the backup goes ahead without them.
3. **Copy the archive off the machine.**

After reinstalling Taskwarrior 3.5 and niri-tasks, run
`niritasks import <archive>`. It imports `tasks.json` with hooks off,
copies `reviews/` back into `~/.local/share/niri-tasks/reviews` without
overwriting any file already there, and prints how many tasks it imported.
It refuses a store that already holds tasks, so an old backup cannot undo
newer edits; `--force` imports anyway, and a task in both takes the
backup's copy.

If `tasks.json` will not import, restore the SQLite copy by hand, into a
store with nothing in it and no `task` running:

```bash
tar -xzf niri-tasks-backup-<stamp>.tar.gz taskchampion.sqlite3
mkdir -p "$(task _get rc.data.location)"
cp taskchampion.sqlite3 "$(task _get rc.data.location)/"
task count                      # every task, back
```

Not backed up: `~/.taskrc`, and the rest of `~/.local/share/niri-tasks`
(unattended logs, ideas, the VS Code profile).
````

- [ ] **Step 4: llms.txt rows**

In `llms.txt`'s command table, insert before the `| \`niritasks terminal\` |` row:

```markdown
| `niritasks export [<path>]` | Write every task, whatever its status, and the reviews folder to one `.tar.gz`, by default `~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz`: `tasks.json` (what import reads), a sqlite `.backup` copy of `taskchampion.sqlite3`, and `reviews/`. Prints the path and the task count, and lists on stderr each `Report:` or `Report (earlier draft):` note whose file is gone (uuid8, status, path), backing up without it. Refuses a path already there; only reads the store. Push every task worktree branch first: their `Plan:` and `Spec:` files live only there | scriptable |
| `niritasks import <archive>` | Put an export back: `task import` of its `tasks.json`, hooks off, and its `reviews/` copied into the reviews folder without overwriting a file already there; prints how many tasks it imported. Refused when the store already holds any task; `--force` imports anyway, a task in both taking the backup's copy | scriptable |
```

- [ ] **Step 5: Check the help text matches**

Run: `cargo run -q -- export --help && cargo run -q -- import --help`
Expected: the doc comments from Tasks 2-3, with `[PATH]`, `<ARCHIVE>` and `--force`.

- [ ] **Step 6: Commit**

```bash
git add README.md llms.txt
git commit -m "docs: document backing up and restoring tasks across a reinstall

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The live round trip

No code and no commit: the check the task's "Done when" asks for, against the real store. The live `~/.task` is only read; the import goes into a scratch store and a scratch reviews folder.

**Files:** none.

- [ ] **Step 1: Export the live store**

```bash
cargo build -q
./target/debug/niritasks export
```

Expected: `Backed up <n> tasks to /home/paul/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz` (269 on 2026-10-25, more since), then the missing-report warning. On 2026-10-25 there were five: `architecture-review-2026-10-02.html` on three tasks and the refine reports of two deleted tasks. Note the archive path for the next step. Leave the archive in place; it is the user's backup.

- [ ] **Step 2: Import it into a scratch store and compare**

```bash
A=/home/paul/niri-tasks-backup-<stamp>.tar.gz      # the path step 1 printed
T=$(mktemp -d)
mkdir -p "$T/data" "$T/xdg"
printf 'data.location=%s/data\n' "$T" > "$T/taskrc"
SCRATCH=(env TASKRC="$T/taskrc" TASKDATA="$T/data" XDG_DATA_HOME="$T/xdg")

"${SCRATCH[@]}" ./target/debug/niritasks import "$A"
tar -xzf "$A" -C "$T" tasks.json
"${SCRATCH[@]}" task rc.hooks=0 rc.json.array=on rc.gc=off export > "$T/restored.json"
python3 -I - "$T/tasks.json" "$T/restored.json" <<'EOF'
import json, sys
strip = lambda t: {k: v for k, v in t.items() if k not in ("id", "urgency")}
live, back = ({t["uuid"]: strip(t) for t in json.load(open(p))} for p in sys.argv[1:])
print(len(live), "backed up,", len(back), "restored")
assert live == back, [u for u in live if live.get(u) != back.get(u)][:5]
print("same uuids, statuses, tags, annotations and dates")
EOF
diff -r ~/.local/share/niri-tasks/reviews "$T/xdg/niri-tasks/reviews" && echo "reviews restored"
```

Expected: `Imported <n> tasks; restored <files> report files, kept 0 already there`; `<n> backed up, <n> restored`; `same uuids, statuses, tags, annotations and dates`; `reviews restored`. `tasks.json` is the live export the archive took, so a task edited on the live store since step 1 cannot make this fail.

- [ ] **Step 3: The guard, on the now non-empty scratch store**

```bash
"${SCRATCH[@]}" ./target/debug/niritasks import "$A" && echo "NOT REFUSED" || echo "refused, as it should be"
rm -rf "$T"
```

Expected: the `--force` message on stderr, then `refused, as it should be`.

- [ ] **Step 4: Confirm the live store was not written**

Run: `task count` and `git status` (in the worktree).
Expected: the same task count as step 1 printed (unless the user added tasks meanwhile); a clean tree.

- [ ] **Step 5: The full suite**

Run: `cargo test`
Expected: all pass. Report the live round trip's output to the user, with the archive's path and the missing-report list.
