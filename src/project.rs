//! Project folders: the `~/Projects` picker's name normalisation, the list a task can move to, and moving it.
//!
//! fuzzel echoes typed text verbatim when it matches no entry, which is what
//! turns the picker into a "new project" box. That text becomes a directory
//! name, so it is normalised and then guarded.
//!
//! Ported from `open_project_workspace.sh:70-91`. The re-check after
//! normalisation matters: typing "my project" may well have just become an
//! existing "my-project", in which case the picker should open that rather than
//! fail to create it.
//!
//! Also here: which programs a freshly-opened project workspace starts with.

use anyhow::{Context, Result};

/// What the picker decided to do with the text the user accepted.
#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    /// Matches a folder that already exists — open it.
    Existing(String),
    /// A new name, already normalised and validated — create it.
    Create(String),
    /// Nothing usable was typed.
    Nothing,
    /// Normalised into something that must not become a directory.
    Rejected(String),
}

/// Runs of whitespace become a single dash, so new folders never carry spaces
/// and stay painless to type at a shell prompt.
pub fn normalize(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Resolve accepted picker text against the list of existing project folders.
pub fn resolve(typed: &str, existing: &[String]) -> Resolved {
    let trimmed = typed.trim();
    if trimmed.is_empty() {
        return Resolved::Nothing;
    }

    // An exact match on what was typed is opened as-is, before any rewriting —
    // a folder that genuinely contains a space is still selectable.
    if existing.iter().any(|p| p == trimmed) {
        return Resolved::Existing(trimmed.to_string());
    }

    let normalized = normalize(trimmed);
    if normalized.is_empty() {
        return Resolved::Nothing;
    }

    // Re-check after normalising: "my project" may already exist as "my-project".
    if existing.contains(&normalized) {
        return Resolved::Existing(normalized);
    }

    // Guard the cases that would write outside ~/Projects, or make a folder the
    // picker can never show again (it filters dotfiles from its list).
    if normalized.contains('/') {
        return Resolved::Rejected(format!("Project name can't contain '/': {normalized}"));
    }
    if normalized.starts_with('.') {
        return Resolved::Rejected(format!("Project name can't start with '.': {normalized}"));
    }

    Resolved::Create(normalized)
}

/// The project folders offered as destinations when moving a task off this
/// workspace.
///
/// Every folder except the one the task is already on — a move to where it
/// already lives is not a move, and listing it only invites picking it. The
/// comparison is on the folded tag, not the folder name, because the tag is
/// what the task actually carries: the folder `niri-tasks` and the tag
/// `niri_tasks` are the same place.
pub fn move_destinations(names: &[String], current_tag: &str) -> Vec<String> {
    names
        .iter()
        .filter(|n| crate::tag::workspace_tag(n) != current_tag)
        .cloned()
        .collect()
}

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

/// The editor opened beside the terminal when a project workspace starts.
pub const EDITOR: &str = "code";

/// Whether a bare command name resolves to something executable on `$PATH`.
///
/// The editor is looked up rather than simply spawned because it is not a
/// requirement of this tool: niri answers a `Spawn` for a missing binary with a
/// desktop notification, so a machine without VS Code would get an error every
/// time it opened a project instead of just getting its terminal.
pub fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| found_in(&path, bin))
}

/// The lookup itself, against a given `PATH` value.
///
/// Split out from [`on_path`] so it can be tested without writing to the real
/// `PATH`, which the rest of the suite is shelling out against in parallel.
fn found_in(path: &std::ffi::OsStr, bin: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::env::split_paths(path).any(|dir| {
        std::fs::metadata(dir.join(bin))
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    })
}

/// The session manager the project terminal runs, one session per workspace.
pub const SESSION_MANAGER: &str = "herdr";

/// A new terminal window whose shell starts in `dir`.
///
/// `+new-window`, not a bare `ghostty --working-directory=…`. ghostty runs as
/// one process for every window (single-instance, over D-Bus), and a second
/// `ghostty` launched with arguments either hands off to it and loses them, or
/// — with `-e` — starts a whole separate ghostty process. `+new-window` asks
/// the running one for a window and hands it the directory and command; D-Bus
/// starts ghostty first if nothing is running. It resolves `dir` before asking
/// and fails on one that does not exist, which `session::start_dir` rules out.
pub fn terminal_command(dir: &std::path::Path) -> Vec<String> {
    vec![
        "ghostty".to_string(),
        "+new-window".to_string(),
        format!("--working-directory={}", dir.display()),
    ]
}

/// The project terminal: `herdr --session <session>` in `dir`, or a plain
/// terminal there when herdr is not installed.
///
/// herdr attaches to the session if it is running — whatever was running in it
/// still is — restores its layout and folders if it was stopped, and otherwise
/// creates it with its first pane in `dir`. `session` is the workspace name
/// already made safe for herdr (`session::herdr_session_name`).
pub fn project_terminal_command(dir: &std::path::Path, session: &str, herdr: bool) -> Vec<String> {
    let mut cmd = terminal_command(dir);
    if herdr {
        cmd.extend(["-e", SESSION_MANAGER, "--session", session].map(String::from));
    }
    cmd
}

/// What to launch on a project workspace that is starting empty.
///
/// `code <dir>` deliberately, not `code -n <dir>`: VS Code reuses an existing
/// window already holding that folder. That is the right trade — `-n` would
/// stack a duplicate window every time — but it does mean that if the project's
/// editor window is parked on some *other* workspace, opening the project
/// focuses that window instead of putting a new one here.
pub fn startup_commands(dir: &std::path::Path, workspace: &str) -> Vec<Vec<String>> {
    let session = crate::session::herdr_session_name(workspace);
    let mut commands = vec![project_terminal_command(dir, &session, on_path(SESSION_MANAGER))];
    if on_path(EDITOR) {
        commands.push(vec![EDITOR.to_string(), dir.display().to_string()]);
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn existing() -> Vec<String> {
        ["alpha", "my-project", "with space"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn whitespace_runs_become_a_single_dash() {
        assert_eq!(normalize("my project"), "my-project");
        assert_eq!(normalize("a   b   c"), "a-b-c");
        assert_eq!(normalize("  padded  "), "padded");
    }

    #[test]
    fn exact_match_opens_existing() {
        assert_eq!(
            resolve("alpha", &existing()),
            Resolved::Existing("alpha".into())
        );
    }

    /// A folder that really does contain a space stays selectable — it is
    /// matched before normalisation would rewrite it.
    #[test]
    fn existing_folder_with_a_space_is_matched_before_rewriting() {
        assert_eq!(
            resolve("with space", &existing()),
            Resolved::Existing("with space".into())
        );
    }

    /// The re-check: typing the spaced form of an existing dashed folder opens
    /// it rather than trying to create a duplicate.
    #[test]
    fn normalizing_can_land_on_an_existing_folder() {
        assert_eq!(
            resolve("my project", &existing()),
            Resolved::Existing("my-project".into())
        );
    }

    #[test]
    fn genuinely_new_names_are_created_normalized() {
        assert_eq!(
            resolve("brand new thing", &existing()),
            Resolved::Create("brand-new-thing".into())
        );
    }

    #[test]
    fn empty_input_does_nothing() {
        assert_eq!(resolve("", &existing()), Resolved::Nothing);
        assert_eq!(resolve("   ", &existing()), Resolved::Nothing);
    }

    #[test]
    fn rejects_names_that_would_escape_the_projects_dir() {
        assert!(matches!(
            resolve("../etc", &existing()),
            Resolved::Rejected(_)
        ));
        assert!(matches!(
            resolve("a/b", &existing()),
            Resolved::Rejected(_)
        ));
    }

    #[test]
    fn rejects_dotfiles_the_picker_could_never_show_again() {
        assert!(matches!(
            resolve(".hidden", &existing()),
            Resolved::Rejected(_)
        ));
    }

    /// The folder you are already on is not offered — and it is recognised by
    /// its tag, so the dashed folder name and the underscored tag it folds to
    /// count as the same place.
    #[test]
    fn destinations_leave_out_the_workspace_you_are_on() {
        let names: Vec<String> = ["niri-tasks", "alp-theme", "keystone"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        assert_eq!(
            move_destinations(&names, "niri_tasks"),
            vec!["alp-theme".to_string(), "keystone".to_string()]
        );
    }

    /// A tag that matches no folder — a workspace named for something that is
    /// not a project — filters nothing out, rather than silently dropping one.
    #[test]
    fn an_unrelated_tag_keeps_every_destination() {
        let names: Vec<String> = ["alpha", "beta"].iter().map(|s| s.to_string()).collect();
        assert_eq!(move_destinations(&names, "scratch").len(), 2);
    }

    /// A directory holding `runnable` (executable) and `readable` (not).
    fn path_fixture(label: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("niritasks-path-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, mode) in [("runnable", 0o755), ("readable", 0o644)] {
            let f = dir.join(name);
            std::fs::write(&f, "").unwrap();
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(mode)).unwrap();
        }
        dir
    }

    #[test]
    fn path_lookup_wants_an_executable_not_just_a_file() {
        let dir = path_fixture("exec");
        let path = std::ffi::OsString::from(format!("/nonexistent:{}", dir.display()));

        assert!(found_in(&path, "runnable"));
        // A non-executable file of the right name is not the program.
        assert!(!found_in(&path, "readable"));
        assert!(!found_in(&path, "absent"));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A plain terminal asks the running ghostty for a window in the folder.
    #[test]
    fn a_terminal_is_a_new_ghostty_window_in_the_folder() {
        assert_eq!(
            terminal_command(std::path::Path::new("/home/x/Projects/my project")),
            vec!["ghostty", "+new-window", "--working-directory=/home/x/Projects/my project"]
        );
    }

    /// With herdr, the project terminal opens the workspace's own session, in
    /// the same window-in-the-folder a plain terminal gets.
    #[test]
    fn the_project_terminal_opens_the_workspace_session() {
        assert_eq!(
            project_terminal_command(std::path::Path::new("/home/x/Projects/alpha"), "alpha", true),
            vec![
                "ghostty",
                "+new-window",
                "--working-directory=/home/x/Projects/alpha",
                "-e",
                "herdr",
                "--session",
                "alpha",
            ]
        );
    }

    /// Without herdr it is just the plain terminal, still in the folder.
    #[test]
    fn without_herdr_the_project_terminal_is_a_plain_one() {
        let dir = std::path::Path::new("/home/x/Projects/alpha");
        assert_eq!(project_terminal_command(dir, "alpha", false), terminal_command(dir));
    }

    /// The session is named from the workspace, made safe for herdr, while the
    /// folder keeps its raw name.
    #[test]
    fn startup_commands_name_the_session_from_the_workspace() {
        let cmds = startup_commands(std::path::Path::new("/home/x/Projects/my project"), "my project");
        assert_eq!(cmds[0][2], "--working-directory=/home/x/Projects/my project");
        if on_path(SESSION_MANAGER) {
            assert_eq!(cmds[0].last().unwrap(), "my_project");
        }
        assert!(cmds.len() <= 2, "terminal, and the editor only when installed");
    }

    /// When the editor is there, it is handed the project path — the terminal
    /// finds its own via the workspace name, the editor cannot.
    #[test]
    fn the_editor_is_given_the_project_directory() {
        if !on_path(EDITOR) {
            return; // nothing to assert on a machine without it
        }
        let cmds = startup_commands(std::path::Path::new("/home/x/Projects/alpha"), "alpha");
        assert_eq!(
            cmds[1],
            vec![EDITOR.to_string(), "/home/x/Projects/alpha".to_string()]
        );
    }
}
