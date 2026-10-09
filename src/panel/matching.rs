//! Ranking the project list's folders against what is typed: fzf when
//! installed, a case-blind substring match otherwise. The panel's `Matcher`
//! is one of these two.

use anyhow::{Context, Result};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::on_path;

    fn projects() -> Vec<String> {
        ["niri-tasks", "alp-theme", "keystone"].iter().map(|s| s.to_string()).collect()
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
}
