//! GitHub rows for the project list.
//!
//! The task panel's project list (Mod+Alt+W) lists the folders in
//! `~/Projects`; underneath them it also offers the account's GitHub repos
//! that are not cloned yet, and cloning one is what "opening" it means. The
//! rows come from `gh` — the authenticated CLI already knows the account and
//! the clone protocol, so there is no username or URL scheme configured here.
//!
//! The list is cached rather than fetched while the list waits: `gh repo
//! list` costs about a second, which is the whole latency budget of a
//! keybind. Each `project open` fires a background refresh and the list reads
//! the cache, so the rows are at most one invocation stale, and the very
//! first open after install shows no GitHub rows at all.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// The suffix that tells a GitHub row apart from a local folder on the
/// project list. Picking a row runs `project open` with the whole row, so
/// the suffix is also how the selection is recognised as a repo to clone.
const MARKER: &str = "  (github)";

/// A repo name rendered as a project list row.
pub fn mark(name: &str) -> String {
    format!("{name}{MARKER}")
}

/// The repo name back out of a project list row, or `None` for a row (or typed
/// text) that is not a GitHub row.
pub fn unmark(row: &str) -> Option<&str> {
    row.strip_suffix(MARKER).filter(|n| !n.is_empty())
}

/// The remote repos worth offering: not already a folder in `~/Projects`,
/// and not a name the project list could never show again after cloning (it
/// filters dotfiles). Order is preserved — `gh` lists most recently pushed
/// first, which is the useful order for "clone what I was just working on".
pub fn remote_only(remote: &[String], local: &[String]) -> Vec<String> {
    remote
        .iter()
        .filter(|n| !n.starts_with('.') && !local.contains(n))
        .cloned()
        .collect()
}

/// What the project list's accepted text means, GitHub rows included.
#[derive(Debug, PartialEq, Eq)]
pub enum Choice {
    /// A local project folder — open it.
    Local(String),
    /// A GitHub repo — clone it, then open it.
    Clone(String),
    /// A genuinely new name — create the folder, then open it.
    Create(String),
    /// Nothing usable was typed.
    Nothing,
    /// Normalised into something that must not become a directory.
    Rejected(String),
}

/// Resolve accepted project list text against both lists.
///
/// A marked row clones. Everything else goes through [`project::resolve`]
/// as before, with one addition: text that resolves to a *creatable* name
/// which happens to be one of the remote repos clones instead — creating an
/// empty folder that shadows your own repo is never what you want.
///
/// [`project::resolve`]: crate::project::resolve
pub fn choose(selected: &str, local: &[String], remote: &[String]) -> Choice {
    if let Some(repo) = unmark(selected) {
        return Choice::Clone(repo.to_string());
    }

    match crate::project::resolve(selected, local) {
        crate::project::Resolved::Existing(n) => Choice::Local(n),
        crate::project::Resolved::Create(n) if remote.contains(&n) => Choice::Clone(n),
        crate::project::Resolved::Create(n) => Choice::Create(n),
        crate::project::Resolved::Nothing => Choice::Nothing,
        crate::project::Resolved::Rejected(msg) => Choice::Rejected(msg),
    }
}

/// Where the repo list is cached: `$XDG_CACHE_HOME/niritasks/github-repos`,
/// or `~/.cache/niritasks/github-repos`.
pub fn cache_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .map(|c| c.join("niritasks/github-repos"))
}

/// The cached repo names — one per line, written by [`spawn_refresh`]. A
/// missing or unreadable cache is an empty list, never an error: the project list
/// works without GitHub rows.
pub fn read_cache(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Fire-and-forget refresh of the cache for the *next* project list.
///
/// `gh` writes to a sibling temp file that is moved over the cache only on
/// success, so a flaky network truncates nothing. Failure is silent by
/// design — no `gh`, no auth, no network all just mean the project list keeps
/// showing whatever it last knew. The explicit `--limit` matters: the
/// default is 30, which quietly drops repos on any account past its first
/// thirty.
pub fn spawn_refresh(path: &Path) {
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
    }

    let _ = std::process::Command::new("sh")
        .args([
            "-c",
            r#"gh repo list --no-archived --limit 500 --json name --jq '.[].name' > "$1.tmp" && mv "$1.tmp" "$1""#,
            "sh",
        ])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Clone `name` from the authenticated account into `dest`, blocking.
///
/// `gh repo clone` rather than `git clone`: the bare name resolves against
/// the authenticated account, over whatever protocol `gh` is configured for.
pub fn clone(name: &str, dest: &Path) -> Result<()> {
    let out = std::process::Command::new("gh")
        .arg("repo")
        .arg("clone")
        .arg(name)
        .arg(dest)
        .output()
        .context("could not run `gh` — is it installed?")?;

    anyhow::ensure!(
        out.status.success(),
        "gh repo clone {name} failed: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_marked_row_round_trips_to_its_repo_name() {
        assert_eq!(mark("convo"), "convo  (github)");
        assert_eq!(unmark(&mark("convo")), Some("convo"));
    }

    /// Local rows and typed text are not GitHub rows.
    #[test]
    fn plain_text_does_not_unmark() {
        assert_eq!(unmark("convo"), None);
        assert_eq!(unmark(""), None);
    }

    /// A repo actually named like a marked row still unmarks — the row for
    /// it would be `x  (github)  (github)`, and only the project list's own
    /// suffix comes off.
    #[test]
    fn only_the_outer_marker_comes_off() {
        assert_eq!(unmark("x  (github)  (github)"), Some("x  (github)"));
    }

    #[test]
    fn repos_already_cloned_are_not_offered() {
        assert_eq!(
            remote_only(&strings(&["convo", "hansard", "ragraph"]), &strings(&["hansard"])),
            strings(&["convo", "ragraph"])
        );
    }

    /// `gh` orders by most recently pushed; that order survives.
    #[test]
    fn remote_order_is_preserved() {
        assert_eq!(
            remote_only(&strings(&["zeta", "alpha"]), &[]),
            strings(&["zeta", "alpha"])
        );
    }

    /// A cloned `.github` folder would vanish from the project list, which filters
    /// dotfiles — so it is never offered.
    #[test]
    fn dotfile_repos_are_not_offered() {
        assert_eq!(remote_only(&strings(&[".github", "convo"]), &[]), strings(&["convo"]));
    }

    #[test]
    fn picking_a_marked_row_clones() {
        assert_eq!(
            choose("convo  (github)", &strings(&["alpha"]), &strings(&["convo"])),
            Choice::Clone("convo".into())
        );
    }

    /// Typing a repo's bare name clones it rather than creating an empty
    /// folder that shadows the repo.
    #[test]
    fn typing_a_repo_name_clones_it() {
        assert_eq!(
            choose("convo", &strings(&["alpha"]), &strings(&["convo"])),
            Choice::Clone("convo".into())
        );
    }

    /// The normalised form is checked too: "my repo" is "my-repo" on GitHub.
    #[test]
    fn a_spaced_variant_of_a_repo_name_clones_it() {
        assert_eq!(
            choose("my repo", &strings(&["alpha"]), &strings(&["my-repo"])),
            Choice::Clone("my-repo".into())
        );
    }

    /// A local folder wins over a same-named repo — it is already here.
    #[test]
    fn a_local_folder_wins_over_a_same_named_repo() {
        assert_eq!(
            choose("alpha", &strings(&["alpha"]), &strings(&["alpha"])),
            Choice::Local("alpha".into())
        );
    }

    #[test]
    fn new_names_still_create_folders() {
        assert_eq!(
            choose("brand new thing", &strings(&["alpha"]), &strings(&["convo"])),
            Choice::Create("brand-new-thing".into())
        );
    }

    #[test]
    fn nothing_and_rejections_pass_through() {
        assert_eq!(choose("  ", &[], &[]), Choice::Nothing);
        assert!(matches!(choose("../etc", &[], &[]), Choice::Rejected(_)));
    }

    #[test]
    fn a_missing_cache_is_an_empty_list() {
        assert_eq!(read_cache(Path::new("/nonexistent/github-repos")), Vec::<String>::new());
    }

    #[test]
    fn the_cache_is_one_name_per_line_blanks_skipped() {
        let f = std::env::temp_dir().join(format!("niritasks-gh-cache-{}", std::process::id()));
        std::fs::write(&f, "convo\n\nragraph\n").unwrap();
        assert_eq!(read_cache(&f), strings(&["convo", "ragraph"]));
        std::fs::remove_file(&f).ok();
    }
}
