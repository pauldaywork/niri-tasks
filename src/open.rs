//! `task open`: what the Go to list opens outside the panel and herdr —
//! the design spec and the implementation plan a task's notes link
//! (`Spec: <path>`, `Plan: <path>`), each in a VS Code instance of its own,
//! a terminal in the task's worktree, and VS Code on it. A note's path is the repository's own,
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
    /// The design spec a `Spec: <path>` note links, as [`opener`] opens it.
    Spec,
    /// The implementation plan a `Plan: <path>` note links, likewise.
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
/// notification. Pure: the worktree, the project folder, the user's
/// folders, whether VS Code is installed and which files exist are handed
/// in. Each thing missing is an error saying what to do.
pub fn command(
    target: Target,
    task: &Task,
    worktree: Option<&Path>,
    folder: &Path,
    dirs: &Dirs,
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
            Ok((opener(&path, dirs, editor)?, format!("the {}", target.name())))
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

/// Whether `path` is a Markdown file, by its extension.
fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

/// What opens the document at `path`, one a task's notes link. Markdown
/// opens in a VS Code instance of its own, apart from the user's, never
/// with xdg-open, whatever the desktop opens Markdown with; anything else,
/// a refine report's HTML page, with xdg-open, in the browser. `editor` is
/// whether VS Code is installed; Markdown without it is an error.
pub fn opener(path: &Path, dirs: &Dirs, editor: bool) -> Result<Vec<String>> {
    if !is_markdown(path) {
        return Ok(refine::open_report_command(&path.to_string_lossy()));
    }
    anyhow::ensure!(editor, "VS Code ({}) is not installed: Markdown opens in it.", programs::EDITOR);
    Ok(programs::document_command(&documents_data(dirs), &extensions(dirs), path))
}

/// Open the document at `path` as [`opener`] says, through niri, so what
/// opens outlives the `niritasks` process a press spawned. The caller has
/// checked the file is there.
pub fn open_document(path: &Path) -> Result<()> {
    let dirs = Dirs::from_env()?;
    let argv = opener(path, &dirs, programs::on_path(programs::EDITOR))?;
    if is_markdown(path) {
        share_settings(&dirs);
    }
    niri::spawn(argv).with_context(|| format!("could not open {}", path.display()))
}

/// Where the VS Code that documents open in keeps its state, apart from
/// the user's own VS Code.
fn documents_data(dirs: &Dirs) -> PathBuf {
    dirs.data().join("niri-tasks/vscode")
}

/// The extensions the user's own VS Code installed, which that VS Code
/// shares.
fn extensions(dirs: &Dirs) -> PathBuf {
    dirs.home().join(".vscode/extensions")
}

/// Link the user's VS Code settings, keybindings and snippets into the
/// documents' VS Code, so it looks and behaves as theirs does and a change
/// made in either reaches both. Only what is missing is linked, so a file
/// put there by hand stays. Best effort: without them the document still
/// opens, in VS Code's defaults.
fn share_settings(dirs: &Dirs) {
    let theirs = dirs.home().join(".config/Code/User");
    let ours = documents_data(dirs).join("User");
    for name in ["settings.json", "keybindings.json", "snippets"] {
        let (from, to) = (theirs.join(name), ours.join(name));
        if from.exists() && to.symlink_metadata().is_err() {
            let _ = std::fs::create_dir_all(&ours).and_then(|()| std::os::unix::fs::symlink(&from, &to));
        }
    }
}

/// `task open`: open `target` for `task` through niri, so what opens
/// outlives the niritasks process a press spawned, and say so. What is
/// missing is an error, which `main` turns into a notification: the Go to
/// list is gone by then.
pub fn open(ws: &Workspace, task: &Task, target: Target) -> Result<()> {
    let worktree = work::worktree_path(ws, &task.uuid);
    let dirs = Dirs::from_env()?;
    let folder = ws.folder(&dirs);
    let editor = programs::on_path(programs::EDITOR);
    let (argv, what) = command(target, task, worktree.as_deref(), &folder, &dirs, editor, &|p| p.is_file())?;
    if matches!(target, Target::Spec | Target::Plan) {
        share_settings(&dirs);
    }
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

    fn dirs() -> Dirs {
        Dirs::at(Path::new("/home/x"))
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

    /// A Markdown plan or spec opens in a VS Code of its own, never with
    /// xdg-open, and only when VS Code is installed.
    #[test]
    fn a_linked_document_opens_in_a_vs_code_of_its_own() {
        let own = |file: &str| {
            ["code", "--user-data-dir", "/home/x/.local/share/niri-tasks/vscode", "--extensions-dir", "/home/x/.vscode/extensions", file]
                .map(String::from)
                .to_vec()
        };
        let (argv, what) = command(Target::Plan, &noted(), Some(Path::new(WORKTREE)), Path::new(FOLDER), &dirs(), true, &always).unwrap();
        assert_eq!(argv, own(&format!("{WORKTREE}/docs/p.md")));
        assert_eq!(what, "the plan");
        let (argv, _) = command(Target::Spec, &noted(), None, Path::new(FOLDER), &dirs(), true, &always).unwrap();
        assert_eq!(argv, own(&format!("{FOLDER}/docs/s.md")));
        let e = command(Target::Plan, &noted(), None, Path::new(FOLDER), &dirs(), false, &always).unwrap_err();
        assert_eq!(e.to_string(), "VS Code (code) is not installed: Markdown opens in it.");
    }

    /// Markdown, whatever the case of its extension, opens in the
    /// separate VS Code; anything else, a refine report's HTML, in the
    /// browser with xdg-open, with or without VS Code.
    #[test]
    fn markdown_opens_in_vs_code_and_anything_else_in_the_browser() {
        for md in ["/r/a.md", "/r/a.MD", "/r/a.markdown"] {
            assert_eq!(opener(Path::new(md), &dirs(), true).unwrap()[0], "code", "{md}");
        }
        let report = "/home/x/.local/share/niri-tasks/reviews/refine-1.html";
        assert_eq!(opener(Path::new(report), &dirs(), true).unwrap(), refine::open_report_command(report));
        assert_eq!(opener(Path::new(report), &dirs(), false).unwrap(), refine::open_report_command(report));
    }

    #[test]
    fn no_note_or_no_file_says_so() {
        let e = command(Target::Spec, &bare(), None, Path::new(FOLDER), &dirs(), false, &always).unwrap_err();
        assert_eq!(e.to_string(), "This task's notes link no spec: add a `Spec: <path>` note.");
        let e = command(Target::Plan, &noted(), None, Path::new(FOLDER), &dirs(), false, &never).unwrap_err();
        assert_eq!(e.to_string(), format!("The plan is not there: {FOLDER}/docs/p.md"));
    }

    /// A terminal in the worktree, and only with one.
    #[test]
    fn the_worktree_opens_in_a_terminal() {
        let (argv, _) = command(Target::Worktree, &bare(), Some(Path::new(WORKTREE)), Path::new(FOLDER), &dirs(), false, &never).unwrap();
        assert_eq!(argv, programs::terminal_command(Path::new(WORKTREE)));
        assert!(command(Target::Worktree, &bare(), None, Path::new(FOLDER), &dirs(), true, &never).is_err());
    }

    /// VS Code on the worktree, else the project folder, and only when it
    /// is installed.
    #[test]
    fn the_editor_opens_on_the_worktree_else_the_folder() {
        let (argv, _) = command(Target::Editor, &bare(), Some(Path::new(WORKTREE)), Path::new(FOLDER), &dirs(), true, &never).unwrap();
        assert_eq!(argv, vec![programs::EDITOR.to_string(), WORKTREE.to_string()]);
        let (argv, _) = command(Target::Editor, &bare(), None, Path::new(FOLDER), &dirs(), true, &never).unwrap();
        assert_eq!(argv, vec![programs::EDITOR.to_string(), FOLDER.to_string()]);
        assert!(command(Target::Editor, &bare(), None, Path::new(FOLDER), &dirs(), false, &never).is_err());
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
