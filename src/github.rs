//! The account's GitHub repos, for the project list and `project open`.
//!
//! The task panel's project list (Mod+Alt+W) lists the folders in
//! `~/Projects`; underneath them it also offers the account's GitHub repos
//! that are not cloned yet, and cloning one is what "opening" it means. The
//! repos come from `gh`: the authenticated CLI already knows the account and
//! the clone protocol, so there is no username or URL scheme configured here.
//!
//! The list is cached rather than fetched while the list waits: `gh repo
//! list` costs about a second, which is the whole latency budget of a
//! keybind. Each `project open` fires a background refresh and the list reads
//! the cache, so the repos are at most one invocation stale, and the very
//! first open after install shows none at all. Cloning goes through `gh` too.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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
