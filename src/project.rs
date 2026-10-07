//! Project folders: the `~/Projects` list, its name normalisation, the list a
//! task can move to, and moving it.
//!
//! A name typed on the project list that matches no folder becomes a
//! directory name, so it is normalised and then guarded. The re-check after
//! normalisation matters: typing "my project" may well have just become an
//! existing "my-project", in which case the list should open that rather than
//! fail to create it.
//!
//! Also here: which programs a freshly-opened project workspace starts with.

use anyhow::{Context, Result};

/// What to do with a name typed on the project list.
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

/// Resolve a typed name against the list of existing project folders.
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
    // list can never show again (it leaves dotfiles out).
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

/// The projects a name can mean, read once per use: the `~/Projects`
/// folders, and the account's GitHub repos not cloned yet, from the cache
/// `project open` refreshes. One value, so the panel's list, its hint and
/// `project open` all read the same two lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projects {
    /// `~/Projects`.
    pub dir: std::path::PathBuf,
    /// Its folders, sorted, dotfiles left out.
    pub local: Vec<String>,
    /// The repos worth offering: not a folder already, not a dotfile, in
    /// `gh`'s most-recently-pushed order.
    pub remote: Vec<String>,
}

/// One row of the project list: a folder in `~/Projects`, or a GitHub repo
/// not cloned yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Folder(String),
    Repo(String),
}

impl Row {
    /// The name a pick spawns, bare: `project open` tells a repo from a
    /// folder by [`Projects::choice`], not by how the row read.
    pub fn name(&self) -> &str {
        match self {
            Row::Folder(name) | Row::Repo(name) => name,
        }
    }
}

/// What `project open` does with a name, and so what Enter on the project
/// list will do with what is typed: one decision for the hint and the act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// A folder that exists: open its workspace.
    Open(String),
    /// A new name, normalised and allowed: make the folder, then open it.
    Create(String),
    /// A GitHub repo not cloned yet: clone it, then open it.
    Clone(String),
    /// Normalised into something that must not become a directory.
    Rejected(String),
    /// Nothing usable was typed.
    Nothing,
}

impl Projects {
    /// Read `~/Projects` and the GitHub cache. The cache missing is an empty
    /// repo list, never an error: the list works without GitHub.
    pub fn load() -> Result<Projects> {
        let (dir, local) = list()?;
        let cached = crate::github::cache_path()
            .map(|cache| crate::github::read_cache(&cache))
            .unwrap_or_default();
        let remote = crate::github::remote_only(&cached, &local);
        Ok(Projects { dir, local, remote })
    }

    /// The project list Mod+Alt+W shows: the folders, then the repos.
    pub fn rows(&self) -> Vec<Row> {
        self.local
            .iter()
            .cloned()
            .map(Row::Folder)
            .chain(self.remote.iter().cloned().map(Row::Repo))
            .collect()
    }

    /// The folders a task on `current_tag` can move to: every folder but the
    /// one it is on. Never a repo: a repo is not a workspace until it is
    /// cloned.
    pub fn move_destinations(&self, current_tag: &str) -> Vec<Row> {
        move_destinations(&self.local, current_tag).into_iter().map(Row::Folder).collect()
    }

    /// What `typed` means, a picked row's name or text: [`resolve`] against
    /// the folders, then a creatable name that is one of the repos clones
    /// instead, because an empty folder shadowing your own repo is never
    /// what you want.
    pub fn choice(&self, typed: &str) -> Choice {
        match resolve(typed, &self.local) {
            Resolved::Existing(name) => Choice::Open(name),
            Resolved::Create(name) if self.remote.contains(&name) => Choice::Clone(name),
            Resolved::Create(name) => Choice::Create(name),
            Resolved::Nothing => Choice::Nothing,
            Resolved::Rejected(why) => Choice::Rejected(why),
        }
    }
}

/// The program the project list ranks what is typed against the folders
/// with.
pub const MATCHER: &str = "fzf";

/// The `folders` that fuzzy-match `query`, best first, as fzf ranks them.
///
/// `fzf --filter` runs once per keystroke, which costs about 2ms for a
/// `~/Projects` of 20 folders, so the panel never waits on it. The user's
/// `FZF_DEFAULT_OPTS` are left out: a `--tac` or `--exact` there would change
/// what the list shows. An error means fzf could not run, and the caller
/// falls back on [`substring_matches`].
pub fn fzf_matches(folders: &[String], query: &str) -> Result<Vec<String>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(MATCHER)
        .arg(format!("--filter={query}"))
        .env_remove("FZF_DEFAULT_OPTS")
        .env_remove("FZF_DEFAULT_OPTS_FILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("could not run `fzf`")?;
    {
        // Dropped at the end of the block, which closes fzf's input.
        let mut stdin = child.stdin.take().context("fzf stdin")?;
        for folder in folders {
            writeln!(stdin, "{folder}")?;
        }
    }
    let out = child.wait_with_output().context("fzf failed")?;
    // 1 is fzf's "nothing matched", not a failure.
    anyhow::ensure!(
        out.status.success() || out.status.code() == Some(1),
        "`fzf --filter` failed ({})",
        out.status
    );
    Ok(String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect())
}

/// The `folders` holding `query`, whatever the case, in the list's order: the
/// project list's matching when fzf is not installed. Plainer than fzf, but
/// typing part of a name still finds it.
pub fn substring_matches(folders: &[String], query: &str) -> Vec<String> {
    let query = query.to_lowercase();
    folders.iter().filter(|f| f.to_lowercase().contains(&query)).cloned().collect()
}

/// `~/Projects` and the folders in it, sorted, dotfiles left out.
///
/// Shared by the project list and a card's Move to workspace, so the two
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

/// The `niritasks` words that open `row`, a row of the project list or a
/// new folder's name: what picking one on the panel spawns. The row goes as
/// the list shows it, so a GitHub row's marker is what makes it a clone.
/// The row follows `--` because a name may start with a dash, which clap
/// would otherwise take for a flag.
pub fn open_args(row: &str) -> Vec<String> {
    vec!["project".to_string(), "open".to_string(), "--".to_string(), row.to_string()]
}

/// The project list Mod+Alt+W shows: every `~/Projects` folder, then the
/// account's GitHub repos not cloned yet, from the cache `project open`
/// refreshes. Read on every open, so a folder made since shows.
pub fn open_rows() -> Result<Vec<String>> {
    let (_, names) = list()?;
    let cached = crate::github::cache_path()
        .map(|cache| crate::github::read_cache(&cache))
        .unwrap_or_default();
    Ok(rows_to_open(&names, &cached))
}

/// [`open_rows`] from the folders and the cached repo names: the folders,
/// then each repo that is not one already, marked as GitHub's. Split out so
/// it is tested without the real `~/Projects` or cache.
pub fn rows_to_open(names: &[String], cached: &[String]) -> Vec<String> {
    let mut rows = names.to_vec();
    rows.extend(crate::github::remote_only(cached, names).iter().map(|n| crate::github::mark(n)));
    rows
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

    /// The fallback without fzf: the folders holding the text, in the
    /// list's order, whatever the case.
    #[test]
    fn substring_matches_keep_the_order_and_ignore_case() {
        assert_eq!(substring_matches(&projects(), "T"), projects());
        assert_eq!(substring_matches(&projects(), "the"), vec!["alp-theme".to_string()]);
        assert!(substring_matches(&projects(), "zz").is_empty());
    }

    /// fzf's ranking, best first, and fuzzy: "kst" is no substring of
    /// keystone. Skipped on a machine without fzf.
    #[test]
    fn fzf_ranks_the_fuzzy_matches() {
        if !on_path(MATCHER) {
            return;
        }
        assert_eq!(fzf_matches(&projects(), "nt").unwrap(), vec!["niri-tasks".to_string()]);
        assert_eq!(fzf_matches(&projects(), "kst").unwrap(), vec!["keystone".to_string()]);
    }

    /// fzf exits 1 when nothing matches: no match, not a failure.
    #[test]
    fn fzf_with_no_match_is_an_empty_list() {
        if !on_path(MATCHER) {
            return;
        }
        assert!(fzf_matches(&projects(), "zzz").unwrap().is_empty());
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
        assert_eq!(normalize("tabs\tand spaces"), "tabs-and-spaces");
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

    /// Picking a row runs `project open` with the row as the list shows it.
    #[test]
    fn opening_a_row_is_project_open_with_the_row() {
        assert_eq!(open_args("alpha  (github)"), vec!["project", "open", "--", "alpha  (github)"]);
    }

    /// The folders first, then each cached repo that is not one already,
    /// marked as GitHub's.
    #[test]
    fn rows_to_open_are_folders_then_uncloned_repos() {
        let names: Vec<String> = ["alpha", "hansard"].iter().map(|s| s.to_string()).collect();
        let cached: Vec<String> = ["convo", "hansard"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            rows_to_open(&names, &cached),
            vec!["alpha".to_string(), "hansard".to_string(), crate::github::mark("convo")]
        );
    }

    fn projects_of(local: &[&str], remote: &[&str]) -> Projects {
        Projects {
            dir: std::path::PathBuf::from("/home/x/Projects"),
            local: local.iter().map(|s| s.to_string()).collect(),
            remote: remote.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// The list Mod+Alt+W shows: the folders in their order, then the repos.
    #[test]
    fn rows_are_folders_then_uncloned_repos() {
        let p = projects_of(&["alpha", "hansard"], &["convo"]);
        assert_eq!(
            p.rows(),
            vec![Row::Folder("alpha".into()), Row::Folder("hansard".into()), Row::Repo("convo".into())]
        );
        assert_eq!(p.rows()[2].name(), "convo");
    }

    /// A task can move to a folder, never to a repo that is not one yet, and
    /// never to the folder it is already on.
    #[test]
    fn move_destinations_are_other_folders_only() {
        let p = projects_of(&["alpha", "niri-tasks"], &["convo"]);
        assert_eq!(p.move_destinations("niri_tasks"), vec![Row::Folder("alpha".into())]);
    }

    #[test]
    fn a_folder_name_opens_it() {
        assert_eq!(projects_of(&["alpha"], &[]).choice("alpha"), Choice::Open("alpha".into()));
    }

    #[test]
    fn a_new_name_makes_a_folder() {
        assert_eq!(projects_of(&["alpha"], &["convo"]).choice("brand new thing"), Choice::Create("brand-new-thing".into()));
    }

    #[test]
    fn a_repo_name_clones_it() {
        assert_eq!(projects_of(&["alpha"], &["convo"]).choice("convo"), Choice::Clone("convo".into()));
    }

    /// The case the panel's hint got wrong: "my repo" normalises to a repo
    /// name, so Enter clones, and the hint has to say so.
    #[test]
    fn a_spaced_variant_of_a_repo_name_clones_it() {
        assert_eq!(projects_of(&["alpha"], &["my-repo"]).choice("my repo"), Choice::Clone("my-repo".into()));
    }

    #[test]
    fn a_local_folder_wins_over_a_same_named_repo() {
        assert_eq!(projects_of(&["alpha"], &["alpha"]).choice("alpha"), Choice::Open("alpha".into()));
    }

    #[test]
    fn nothing_and_rejections_pass_through() {
        assert_eq!(projects_of(&[], &[]).choice("  "), Choice::Nothing);
        assert!(matches!(projects_of(&[], &[]).choice("../etc"), Choice::Rejected(_)));
        assert!(matches!(projects_of(&[], &[]).choice(".hidden"), Choice::Rejected(_)));
    }
}
