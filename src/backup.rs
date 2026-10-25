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
