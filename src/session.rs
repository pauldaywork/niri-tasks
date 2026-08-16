//! tmux session naming and project-folder resolution.
//!
//! Ported from `tmux-niri-session.sh`, which ghostty runs as its `command =`.
//! Two details there are easy to lose and are pinned by tests below:
//!
//! * The *sanitised* name is used for the tmux session, but the **raw** name is
//!   used for the directory lookup. A workspace called "my project" looks for
//!   `~/Projects/my project` while running in session `my_project_1`.
//! * `tr -c 'A-Za-z0-9_-' '_'` replaces each disallowed character individually.
//!   It does **not** collapse runs, unlike `workspace_tag`. Two spaces become
//!   two underscores.

use std::path::{Path, PathBuf};

/// Sanitise a workspace name for use as a tmux session name.
///
/// Per-character replacement, deliberately not run-collapsing — see module docs.
pub fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The separator between the workspace name and the number.
///
/// It exists because the shell version produced one by accident: it ran
/// `echo "$ws" | tr -c 'A-Za-z0-9_-' '_'`, and `echo` appends a newline, which
/// is not in the allowed set, so it became a trailing underscore. Every session
/// on this machine is named that way — `ubuntu-setup_1`, `keystone_2`.
///
/// Reproducing an accident sounds like the wrong call, but `tmux new-session -A`
/// treats the name as the identity: dropping the underscore would not rename
/// those sessions, it would create a second parallel set beside them and stop
/// the shortcut ever reattaching to the ones you have open. So it is a real
/// separator now, written down rather than emerging from a newline.
const SEPARATOR: &str = "_";

/// Pick the smallest unused numeric suffix for a session name, given a
/// predicate that reports whether a session already exists.
pub fn next_session_name(base: &str, exists: impl Fn(&str) -> bool) -> String {
    let mut n = 1u32;
    loop {
        let candidate = format!("{base}{SEPARATOR}{n}");
        if !exists(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Where a new tmux session should start.
///
/// `~/Projects/<workspace>` -> `~/Projects` -> `~`. Uses the *raw* workspace
/// name, not the sanitised one.
pub fn start_dir(home: &Path, workspace_raw: &str) -> PathBuf {
    let projects = home.join("Projects");

    let in_project = projects.join(workspace_raw);
    if in_project.is_dir() {
        return in_project;
    }
    if projects.is_dir() {
        return projects;
    }
    home.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sanitizes_characters_tmux_dislikes() {
        assert_eq!(sanitize_session_name("my:project"), "my_project");
        assert_eq!(sanitize_session_name("my.project"), "my_project");
        assert_eq!(sanitize_session_name("my project"), "my_project");
    }

    #[test]
    fn keeps_dashes_and_underscores_unlike_the_tag_rule() {
        // workspace_tag() would turn the dash into an underscore; this must not.
        assert_eq!(sanitize_session_name("ubuntu-setup"), "ubuntu-setup");
        assert_eq!(sanitize_session_name("my_project"), "my_project");
    }

    #[test]
    fn does_not_collapse_runs() {
        // The contrast with workspace_tag(), which would give "a_b".
        assert_eq!(sanitize_session_name("a  b"), "a__b");
        assert_eq!(sanitize_session_name("a...b"), "a___b");
    }

    #[test]
    fn preserves_case_unlike_the_tag_rule() {
        assert_eq!(sanitize_session_name("MyProject"), "MyProject");
    }

    #[test]
    fn first_session_gets_suffix_one() {
        assert_eq!(next_session_name("work", |_| false), "work_1");
    }

    #[test]
    fn finds_the_smallest_unused_suffix() {
        let taken: HashSet<&str> = ["work_1", "work_2", "work_4"].into_iter().collect();
        assert_eq!(
            next_session_name("work", |s| taken.contains(s)),
            "work_3",
            "should fill the gap at 3, not jump past 4"
        );
    }

    #[test]
    fn start_dir_falls_back_through_the_chain() {
        let tmp = std::env::temp_dir().join(format!("wt-session-test-{}", std::process::id()));
        let home = tmp.join("home");
        let projects = home.join("Projects");
        std::fs::create_dir_all(projects.join("alpha")).unwrap();

        // Exact project folder wins.
        assert_eq!(start_dir(&home, "alpha"), projects.join("alpha"));
        // Unknown project falls back to ~/Projects.
        assert_eq!(start_dir(&home, "nope"), projects);

        // Without ~/Projects at all, fall back to home.
        let bare = tmp.join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        assert_eq!(start_dir(&bare, "anything"), bare);

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// The raw name is used for the directory even when it needs sanitising for
    /// the session name — the pairing that makes `open_project_workspace.sh`
    /// land you in the right folder.
    #[test]
    fn directory_uses_raw_name_while_session_uses_sanitized() {
        let tmp = std::env::temp_dir().join(format!("wt-raw-test-{}", std::process::id()));
        let projects = tmp.join("Projects");
        std::fs::create_dir_all(projects.join("my project")).unwrap();

        assert_eq!(start_dir(&tmp, "my project"), projects.join("my project"));
        assert_eq!(sanitize_session_name("my project"), "my_project");
        // ...and the full name the shell version would have produced.
        assert_eq!(
            next_session_name(&sanitize_session_name("my project"), |_| false),
            "my_project_1"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }
}
