//! The programs niritasks runs beside itself: whether one is installed, and
//! the argv that opens a terminal, the project terminal with its herdr
//! session, and the editor.
//!
//! A leaf: nothing here reaches into the rest of the crate, so `session` and
//! `project` can both lean on it without leaning on each other.

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
/// and fails on one that does not exist, which `Dirs::start_dir` rules out.
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
///
/// `session` is the herdr session name, the workspace name already made safe
/// for herdr (`session::herdr_session_name`). The caller names it so this
/// module depends on nothing else in the crate.
pub fn startup_commands(dir: &std::path::Path, session: &str) -> Vec<Vec<String>> {
    let mut commands = vec![project_terminal_command(dir, session, on_path(SESSION_MANAGER))];
    if on_path(EDITOR) {
        commands.push(vec![EDITOR.to_string(), dir.display().to_string()]);
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// The terminal joins the session it is handed, already made safe for
    /// herdr, while the folder keeps its raw name.
    #[test]
    fn startup_commands_join_the_named_session_in_the_raw_folder() {
        let cmds = startup_commands(std::path::Path::new("/home/x/Projects/my project"), "my_project");
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
