//! Project-name normalisation for the `~/Projects` picker.
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
    if existing.iter().any(|p| *p == normalized) {
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

/// What to launch on a project workspace that is starting empty.
///
/// The terminal takes no argument: ghostty runs `wt tmux-session`, which finds
/// `~/Projects/<workspace>` from the workspace name itself. The editor has no
/// such indirection, so it is handed the path.
///
/// `code <dir>` deliberately, not `code -n <dir>`: VS Code reuses an existing
/// window already holding that folder. That is the right trade — `-n` would
/// stack a duplicate window every time — but it does mean that if the project's
/// editor window is parked on some *other* workspace, opening the project
/// focuses that window instead of putting a new one here.
pub fn startup_commands(dir: &std::path::Path) -> Vec<Vec<String>> {
    let mut commands = vec![vec!["ghostty".to_string()]];
    if on_path(EDITOR) {
        commands.push(vec![EDITOR.to_string(), dir.display().to_string()]);
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A directory holding `runnable` (executable) and `readable` (not).
    fn path_fixture(label: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("wt-path-{label}-{}", std::process::id()));
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

    /// The terminal is unconditional; the editor is only added when installed,
    /// so a machine without VS Code opens a project without a failed spawn.
    #[test]
    fn startup_commands_always_include_the_terminal() {
        let cmds = startup_commands(std::path::Path::new("/home/x/Projects/alpha"));
        assert_eq!(cmds[0], vec!["ghostty"]);
        assert!(cmds.len() <= 2);
    }

    /// When the editor is there, it is handed the project path — the terminal
    /// finds its own via the workspace name, the editor cannot.
    #[test]
    fn the_editor_is_given_the_project_directory() {
        if !on_path(EDITOR) {
            return; // nothing to assert on a machine without it
        }
        let cmds = startup_commands(std::path::Path::new("/home/x/Projects/alpha"));
        assert_eq!(
            cmds[1],
            vec![EDITOR.to_string(), "/home/x/Projects/alpha".to_string()]
        );
    }
}
