//! Which folder a workspace's terminals start in, and which herdr session a
//! workspace's project terminal attaches to.
//!
//! Two rules, and the inverse of each, because `niritasks tag --session` has
//! to get from a terminal back to the workspace it was opened for:
//!
//! * **Folder**: a workspace named "hansard-votes" works in
//!   `~/Projects/hansard-votes` ([`start_dir`]). Inverse: [`project_from_cwd`].
//! * **Session**: its project terminal runs `herdr --session <name>`, with the
//!   name made safe for herdr ([`herdr_session_name`]). Inverse:
//!   [`workspace_for_session`], fed by [`session_from_env`].
//!
//! The folder uses the *raw* workspace name and the session the sanitised one.
//! A workspace called "my project" looks in `~/Projects/my project` while
//! running in session `my_project`.

use std::path::{Component, Path, PathBuf};

/// herdr refuses session names longer than this (herdr `src/session.rs`,
/// `MAX_SESSION_NAME_LEN`).
pub const SESSION_NAME_MAX: usize = 64;

/// The herdr session a workspace's project terminal attaches to.
///
/// herdr only accepts `[A-Za-z0-9._-]`, at most 64 bytes, and not `.` or `..`;
/// anything else makes `herdr --session` exit 2 before it draws a thing. And
/// `default` is not a name at all to herdr — it means the unnamed default
/// session. So each disallowed character becomes `_` (per character, not
/// run-collapsing, case kept), the result is cut to 64, and the three names
/// herdr treats specially get a `ws-` prefix.
pub fn herdr_session_name(workspace: &str) -> String {
    let mut name: String = workspace
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Every char is ASCII by now, so truncating at a byte count cannot split one.
    name.truncate(SESSION_NAME_MAX);
    if matches!(name.as_str(), "" | "." | ".." | "default") {
        name = format!("ws-{name}");
    }
    name
}

/// Which of `workspaces` this herdr session was opened for.
///
/// The session name is a *lossy* rendering of the workspace name — spaces and
/// slashes both became underscores — so it cannot simply be unfolded. Matching
/// each live workspace name through the same rule instead recovers the
/// original exactly, and answers `None` when the workspace it named is gone or
/// has since been renamed.
pub fn workspace_for_session<'a>(session: &str, workspaces: &'a [String]) -> Option<&'a String> {
    workspaces
        .iter()
        .find(|name| herdr_session_name(name) == session)
}

/// The herdr session this process is running in, from the environment herdr
/// gives its panes.
///
/// `HERDR_SESSION` carries the name. herdr's client sets it and its server,
/// and so every pane, inherits it — but only for a named session: the default
/// session removes it, and there is no name to recover then. `HERDR_SOCKET_PATH`
/// is the backup, `<config>/sessions/<name>/herdr.sock` for a named session and
/// `<config>/herdr.sock` for the default one. Passed in rather than read here
/// so the tests need not touch the real environment.
pub fn session_from_env(herdr_session: Option<&str>, socket_path: Option<&str>) -> Option<String> {
    if let Some(name) = herdr_session.filter(|s| !s.is_empty()) {
        return Some(name.to_string());
    }
    let dir = Path::new(socket_path?).parent()?;
    let is_named = dir.parent()?.file_name()? == "sessions";
    is_named.then(|| dir.file_name()?.to_str().map(str::to_string)).flatten()
}

/// Where a workspace's terminals start.
///
/// `~/Projects/<workspace>` -> `~/Projects` -> `~`. Uses the *raw* workspace
/// name, not the sanitised one. Always an existing directory, which
/// `ghostty +new-window` needs: it resolves the path before asking the running
/// ghostty for a window, and fails outright on one that is not there.
pub fn start_dir(home: &Path, workspace_raw: &str) -> PathBuf {
    let projects = home.join("Projects");

    let in_project = projects.join(workspace_raw);
    if !workspace_raw.is_empty() && in_project.is_dir() {
        return in_project;
    }
    if projects.is_dir() {
        return projects;
    }
    home.to_path_buf()
}

/// The project folder `cwd` is inside, if it is inside one: the first path
/// component under `~/Projects`. [`start_dir`] run backwards.
pub fn project_from_cwd(home: &Path, cwd: &Path) -> Option<String> {
    let rest = cwd.strip_prefix(home.join("Projects")).ok()?;
    match rest.components().next()? {
        Component::Normal(name) => name.to_str().map(str::to_string),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_workspace_name_is_its_own_session_name() {
        assert_eq!(herdr_session_name("niri-tasks"), "niri-tasks");
        assert_eq!(herdr_session_name("my_project"), "my_project");
        assert_eq!(herdr_session_name("v1.2"), "v1.2", "herdr allows dots");
        assert_eq!(herdr_session_name("MyProject"), "MyProject", "case is kept");
    }

    #[test]
    fn characters_herdr_rejects_become_underscores_one_for_one() {
        assert_eq!(herdr_session_name("my project"), "my_project");
        assert_eq!(herdr_session_name("a  b"), "a__b", "runs are not collapsed");
        assert_eq!(herdr_session_name("a/b:c"), "a_b_c");
        assert_eq!(herdr_session_name("café"), "caf_", "non-ASCII is one char, one underscore");
    }

    #[test]
    fn long_names_are_cut_to_what_herdr_accepts() {
        let long = "x".repeat(100);
        assert_eq!(herdr_session_name(&long).len(), SESSION_NAME_MAX);
        let unicode = "é".repeat(100);
        assert_eq!(herdr_session_name(&unicode).len(), SESSION_NAME_MAX);
    }

    #[test]
    fn names_herdr_treats_specially_are_prefixed() {
        assert_eq!(herdr_session_name("default"), "ws-default");
        assert_eq!(herdr_session_name("."), "ws-.");
        assert_eq!(herdr_session_name(".."), "ws-..");
        assert_eq!(herdr_session_name(""), "ws-");
        // Only the exact names: these are ordinary.
        assert_eq!(herdr_session_name("Default"), "Default");
        assert_eq!(herdr_session_name("defaults"), "defaults");
    }

    /// The session name cannot be unfolded — "a  b" and "a__b" both become
    /// "a__b" — so the live workspace names are run through the same rule and
    /// compared, which recovers the original.
    #[test]
    fn a_session_finds_the_workspace_that_named_it() {
        let workspaces: Vec<String> = ["niri-tasks", "a  b", "My Project", "default"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        assert_eq!(workspace_for_session("niri-tasks", &workspaces), Some(&workspaces[0]));
        assert_eq!(workspace_for_session("a__b", &workspaces), Some(&workspaces[1]));
        assert_eq!(workspace_for_session("My_Project", &workspaces), Some(&workspaces[2]));
        assert_eq!(workspace_for_session("ws-default", &workspaces), Some(&workspaces[3]));
    }

    /// A session whose workspace was renamed or closed matches nothing, rather
    /// than resolving to a plausible-looking wrong one.
    #[test]
    fn a_session_with_no_live_workspace_matches_nothing() {
        let workspaces = vec!["niri-tasks".to_string()];
        assert_eq!(workspace_for_session("old-name", &workspaces), None);
        assert_eq!(workspace_for_session("niri", &workspaces), None, "no prefix matching");
    }

    #[test]
    fn herdr_session_comes_from_herdr_session_first() {
        assert_eq!(
            session_from_env(Some("alpha"), Some("/h/.config/herdr/sessions/beta/herdr.sock")),
            Some("alpha".to_string())
        );
    }

    #[test]
    fn socket_path_is_the_backup_for_a_named_session() {
        assert_eq!(
            session_from_env(None, Some("/h/.config/herdr/sessions/beta/herdr.sock")),
            Some("beta".to_string())
        );
        assert_eq!(
            session_from_env(Some(""), Some("/h/.config/herdr/sessions/beta/herdr.sock")),
            Some("beta".to_string()),
            "an empty HERDR_SESSION is no name"
        );
    }

    #[test]
    fn the_default_session_and_no_herdr_have_no_session() {
        assert_eq!(session_from_env(None, Some("/h/.config/herdr/herdr.sock")), None);
        assert_eq!(session_from_env(None, None), None);
    }

    #[test]
    fn start_dir_falls_back_through_the_chain() {
        let tmp = std::env::temp_dir().join(format!("niritasks-session-test-{}", std::process::id()));
        let home = tmp.join("home");
        let projects = home.join("Projects");
        std::fs::create_dir_all(projects.join("alpha")).unwrap();
        std::fs::create_dir_all(projects.join("my project")).unwrap();

        // Exact project folder wins, by its raw name.
        assert_eq!(start_dir(&home, "alpha"), projects.join("alpha"));
        assert_eq!(start_dir(&home, "my project"), projects.join("my project"));
        // Unknown project, or an unnamed workspace, falls back to ~/Projects.
        assert_eq!(start_dir(&home, "nope"), projects);
        assert_eq!(start_dir(&home, ""), projects);

        // Without ~/Projects at all, fall back to home.
        let bare = tmp.join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        assert_eq!(start_dir(&bare, "anything"), bare);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn project_from_cwd_is_the_folder_under_projects() {
        let home = Path::new("/home/x");
        assert_eq!(
            project_from_cwd(home, Path::new("/home/x/Projects/alpha")),
            Some("alpha".to_string())
        );
        assert_eq!(
            project_from_cwd(home, Path::new("/home/x/Projects/alpha/src/deep")),
            Some("alpha".to_string())
        );
        assert_eq!(
            project_from_cwd(home, Path::new("/home/x/Projects/my project")),
            Some("my project".to_string())
        );
        assert_eq!(project_from_cwd(home, Path::new("/home/x/Projects")), None);
        assert_eq!(project_from_cwd(home, Path::new("/home/x")), None);
        assert_eq!(project_from_cwd(home, Path::new("/tmp/alpha")), None);
    }
}
