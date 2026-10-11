//! `task open`: what the Go to list opens outside the panel and herdr —
//! the design spec and the implementation plan a task's notes link
//! (`Spec: <path>`, `Plan: <path>`), a terminal in the task's worktree, and
//! VS Code on it. A note's path is the repository's own,
//! `docs/superpowers/plans/<file>.md`, so where it is depends on where the
//! task is being worked: its worktree while it has one, else the
//! workspace's `~/Projects` folder.

use crate::dirs::Dirs;
use crate::task::{self, Task};
use crate::workspace::Workspace;
use crate::{niri, notify, programs, refine, work};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// What `task open` opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Target {
    /// The design spec a `Spec: <path>` note links, with xdg-open.
    Spec,
    /// The implementation plan a `Plan: <path>` note links, with xdg-open.
    Plan,
    /// A terminal in the task's worktree.
    Worktree,
    /// VS Code on the task's worktree, else the project folder.
    Editor,
}

impl Target {
    /// Its name in a sentence, and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Target::Spec => "spec",
            Target::Plan => "plan",
            Target::Worktree => "worktree",
            Target::Editor => "editor",
        }
    }
}

/// Where a linked document is: an absolute path as it is; a relative one,
/// as the notes write them, in the task's worktree when it has one, else
/// the workspace's folder.
pub fn resolve(linked: &str, worktree: Option<&Path>, folder: &Path) -> PathBuf {
    let path = Path::new(linked);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    worktree.unwrap_or(folder).join(path)
}

/// What opening `target` for `task` runs, and what it opens, for the
/// notification. Pure: the worktree, the project folder, whether VS Code is
/// installed and which files exist are handed in. Each thing missing is an
/// error saying what to do.
pub fn command(
    target: Target,
    task: &Task,
    worktree: Option<&Path>,
    folder: &Path,
    editor: bool,
    is_file: &dyn Fn(&Path) -> bool,
) -> Result<(Vec<String>, String)> {
    match target {
        Target::Spec | Target::Plan => {
            let (linked, note) = match target {
                Target::Spec => (task.spec_path(), task::SPEC_NOTE),
                _ => (task.plan_path(), task::PLAN_NOTE),
            };
            let linked = linked
                .with_context(|| format!("This task's notes link no {}: add a `{note}<path>` note.", target.name()))?;
            let path = resolve(linked, worktree, folder);
            anyhow::ensure!(is_file(&path), "The {} is not there: {}", target.name(), path.display());
            // xdg-open, as a report is opened.
            Ok((refine::open_report_command(&path.to_string_lossy()), format!("the {}", target.name())))
        }
        Target::Worktree => {
            let dir = worktree.context("This task has no worktree: Get started makes one.")?;
            Ok((programs::terminal_command(dir), "a terminal in its worktree".to_string()))
        }
        Target::Editor => {
            anyhow::ensure!(editor, "VS Code ({}) is not installed.", programs::EDITOR);
            let dir = worktree.unwrap_or(folder);
            Ok((vec![programs::EDITOR.to_string(), dir.display().to_string()], "VS Code".to_string()))
        }
    }
}

/// `task open`: open `target` for `task` through niri, so what opens
/// outlives the niritasks process a press spawned, and say so. What is
/// missing is an error, which `main` turns into a notification: the Go to
/// list is gone by then.
pub fn open(ws: &Workspace, task: &Task, target: Target) -> Result<()> {
    let worktree = work::worktree_path(ws, &task.uuid);
    let folder = ws.folder(&Dirs::from_env()?);
    let editor = programs::on_path(programs::EDITOR);
    let (argv, what) = command(target, task, worktree.as_deref(), &folder, editor, &|p| p.is_file())?;
    niri::spawn(argv).with_context(|| format!("could not open {what}"))?;
    notify::tasks(&format!("Opened {what}: {}", task.description));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noted() -> Task {
        serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Spec: docs/s.md"},
                {"entry":"20261006T020000Z","description":"Plan: docs/p.md"}
            ]}"#,
        )
        .unwrap()
    }

    fn bare() -> Task {
        serde_json::from_str(r#"{"uuid":"u","description":"d"}"#).unwrap()
    }

    const FOLDER: &str = "/home/x/Projects/repo";
    const WORKTREE: &str = "/home/x/.worktrees/repo/task-a-7cd9fd3a";
    fn always(_: &Path) -> bool {
        true
    }
    fn never(_: &Path) -> bool {
        false
    }

    /// A relative path, as the working Claude writes one, is the worktree's
    /// while the task has one, else the project folder's; an absolute one
    /// is kept.
    #[test]
    fn a_relative_path_is_read_in_the_worktree_while_there_is_one() {
        let folder = Path::new(FOLDER);
        assert_eq!(resolve("docs/p.md", Some(Path::new(WORKTREE)), folder), Path::new(WORKTREE).join("docs/p.md"));
        assert_eq!(resolve("docs/p.md", None, folder), folder.join("docs/p.md"));
        assert_eq!(resolve("/tmp/spec.md", Some(Path::new(WORKTREE)), folder), PathBuf::from("/tmp/spec.md"));
    }

    /// The plan and the spec open with xdg-open, the path the shell's `$1`.
    #[test]
    fn a_linked_document_opens_with_xdg_open() {
        let (argv, what) = command(Target::Plan, &noted(), Some(Path::new(WORKTREE)), Path::new(FOLDER), false, &always).unwrap();
        assert_eq!(argv, refine::open_report_command(&format!("{WORKTREE}/docs/p.md")));
        assert_eq!(what, "the plan");
        let (argv, _) = command(Target::Spec, &noted(), None, Path::new(FOLDER), false, &always).unwrap();
        assert_eq!(argv, refine::open_report_command(&format!("{FOLDER}/docs/s.md")));
    }

    #[test]
    fn no_note_or_no_file_says_so() {
        let e = command(Target::Spec, &bare(), None, Path::new(FOLDER), false, &always).unwrap_err();
        assert_eq!(e.to_string(), "This task's notes link no spec: add a `Spec: <path>` note.");
        let e = command(Target::Plan, &noted(), None, Path::new(FOLDER), false, &never).unwrap_err();
        assert_eq!(e.to_string(), format!("The plan is not there: {FOLDER}/docs/p.md"));
    }

    /// A terminal in the worktree, and only with one.
    #[test]
    fn the_worktree_opens_in_a_terminal() {
        let (argv, _) = command(Target::Worktree, &bare(), Some(Path::new(WORKTREE)), Path::new(FOLDER), false, &never).unwrap();
        assert_eq!(argv, programs::terminal_command(Path::new(WORKTREE)));
        assert!(command(Target::Worktree, &bare(), None, Path::new(FOLDER), true, &never).is_err());
    }

    /// VS Code on the worktree, else the project folder, and only when it
    /// is installed.
    #[test]
    fn the_editor_opens_on_the_worktree_else_the_folder() {
        let (argv, _) = command(Target::Editor, &bare(), Some(Path::new(WORKTREE)), Path::new(FOLDER), true, &never).unwrap();
        assert_eq!(argv, vec![programs::EDITOR.to_string(), WORKTREE.to_string()]);
        let (argv, _) = command(Target::Editor, &bare(), None, Path::new(FOLDER), true, &never).unwrap();
        assert_eq!(argv, vec![programs::EDITOR.to_string(), FOLDER.to_string()]);
        assert!(command(Target::Editor, &bare(), None, Path::new(FOLDER), false, &never).is_err());
    }

    /// Each target is named on the command line as it is in a sentence.
    #[test]
    fn targets_are_named_as_the_cli_takes_them() {
        use clap::ValueEnum;
        for target in Target::value_variants() {
            assert_eq!(target.to_possible_value().unwrap().get_name(), target.name());
        }
    }
}
