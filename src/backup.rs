//! `niritasks export` and `niritasks import`: every task, and the refine
//! reports their notes link, in one archive to carry across a reinstall.
//!
//! The archive is a `.tar.gz` of `tasks.json`, the whole store as `task
//! export` writes it and what import reads back; `taskchampion.sqlite3`, a
//! copy of the store taken through SQLite's own backup, to restore by hand if
//! the JSON will not import; and `reviews/`, the reports folder. `tar` and
//! `sqlite3` do the packing and the copy.

use crate::{dirs::Dirs, names, refine, task, task::Task};
use anyhow::{bail, ensure, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

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
