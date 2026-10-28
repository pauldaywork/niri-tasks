//! Explain branch's test run: the project's tests, run before the report
//! session starts, outside its sandbox, in a herdr tab of their own.
//!
//! The fenced report session cannot run every suite: some need sockets,
//! a display or files its sandbox hides. So the run happens first, as the
//! user, in the worktree, with its output shown live in the tab and kept
//! in a log beside the reports; a results file names the command, its
//! exit code, the log and the times, and the session is handed that file
//! (the refine mod's `tests` option, and the prompt) to report on. The
//! exit code never decides whether the session starts: a failing run is
//! what the report says broke.

use crate::dirs::Dirs;
use crate::workspace::Workspace;
use crate::{refine, task, work};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// A worktree's test command: a shell line, and what its environment
/// gains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCommand {
    pub shell: String,
    pub env: Vec<(String, String)>,
}

impl TestCommand {
    /// The command as the results file names it: the environment it gains
    /// first, as a shell line would set it.
    pub fn display(&self) -> String {
        let mut out = String::new();
        for (k, v) in &self.env {
            out.push_str(&format!("{k}={v} "));
        }
        out.push_str(&self.shell);
        out
    }
}

/// The git setting a repository names its own test command in, run with
/// `sh -c` in the worktree.
pub const CONFIG_KEY: &str = "niri-tasks.test";

/// The command that tests `worktree`, if it has one: the repository's
/// [`CONFIG_KEY`] when set; else `bash tests/all.sh` with
/// `NIRITASKS_HEADLESS=1`, so no nested-niri e2e suite runs from a report,
/// when there is a `tests/all.sh`; else none, and the session runs what it
/// finds itself, in its sandbox, as it always has.
pub fn test_command(worktree: &Path) -> Result<Option<TestCommand>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(["config", "--get", CONFIG_KEY])
        .output()
        .context("could not run git")?;
    // `git config --get` exits 1 when the key is unset; anything else that
    // is not success is git failing to read the config at all.
    if out.status.success() {
        let shell = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !shell.is_empty() {
            return Ok(Some(TestCommand { shell, env: Vec::new() }));
        }
    } else if out.status.code() != Some(1) {
        anyhow::bail!(
            "could not read {CONFIG_KEY} from git in {}: {}",
            worktree.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    if worktree.join("tests/all.sh").is_file() {
        return Ok(Some(TestCommand {
            shell: "bash tests/all.sh".into(),
            env: vec![("NIRITASKS_HEADLESS".into(), "1".into())],
        }));
    }
    Ok(None)
}

/// A UTC time as the reports' file names stamp it, `YYYYMMDD-HHMMSS`, the
/// form the refine mod's `reportPath` uses.
fn file_stamp(secs: i64) -> String {
    task::stamp(secs).trim_end_matches('Z').replace('T', "-")
}

/// A UTC time in ISO-8601, `YYYY-MM-DDTHH:MM:SSZ`, as the results file
/// gives its times.
fn iso(secs: i64) -> String {
    let s = task::stamp(secs);
    format!("{}-{}-{}T{}:{}:{}Z", &s[0..4], &s[4..6], &s[6..8], &s[9..11], &s[11..13], &s[13..15])
}

fn unix_secs(t: SystemTime) -> i64 {
    t.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Run `cmd` in `worktree`, started at `now`, and write its log and results
/// file to `reviews`: `branch-<uuid8>-<stamp>-tests.log` and `.json`. The
/// path of the results file.
///
/// Its output reaches this tab's terminal as it comes, and the log too:
/// stdout and stderr together, through `tee`, under bash for the exit code
/// of the command rather than of `tee`. stdin is the terminal's, for a
/// suite that asks something. Only a command that cannot be started, or
/// files that cannot be written, are errors; the command's own exit code,
/// whatever it is, goes in the results file.
pub fn run_tests(worktree: &Path, cmd: &TestCommand, reviews: &Path, uuid: &str, now: SystemTime) -> Result<PathBuf> {
    std::fs::create_dir_all(reviews).with_context(|| format!("could not make {}", reviews.display()))?;
    let started = unix_secs(now);
    let base = format!("branch-{}-{}-tests", crate::names::uuid8(uuid), file_stamp(started));
    let log = reviews.join(format!("{base}.log"));
    let json = reviews.join(format!("{base}.json"));
    // The command and the log go in as arguments, never into the script's
    // text, so nothing in them needs quoting.
    let status = Command::new("bash")
        .args(["-c", r#"sh -c "$1" 2>&1 | tee "$2"; exit "${PIPESTATUS[0]}""#, "bash"])
        .arg(&cmd.shell)
        .arg(&log)
        .current_dir(worktree)
        .envs(cmd.env.iter().map(|(k, v)| (k, v)))
        .status()
        .with_context(|| format!("could not run {} in {}", cmd.display(), worktree.display()))?;
    let finished = unix_secs(SystemTime::now()).max(started);
    // A command killed by a signal is 128 + the signal, as bash reports
    // it; only bash itself killed leaves no code at all.
    let exit_code = status.code().unwrap_or(-1);
    let results = serde_json::json!({
        "command": cmd.display(),
        "exit_code": exit_code,
        "log": log,
        "started": iso(started),
        "finished": iso(finished),
    });
    std::fs::write(&json, format!("{results:#}\n")).with_context(|| format!("could not write {}", json.display()))?;
    Ok(json)
}

/// What the tests tab runs: this binary's `task report <uuid> --branch
/// --here --workspace <ws>`, each argument quoted for the pane's shell.
pub fn here_command(exe: &str, workspace: &str, uuid: &str) -> String {
    format!(
        "{} task report {} --branch --here --workspace {}",
        work::sh_quote(exe),
        work::sh_quote(uuid),
        work::sh_quote(workspace)
    )
}

/// The tests tab's step, run inside the tab [`refine::explain_branch`]
/// opened: run the branch's tests where the user can watch, then start
/// the report session on them in a tab of its own, then close this tab.
/// On failure the tab stays, with the error, until the user has read it.
///
/// It holds the task's tests lock from the start until this process ends,
/// the error's wait included, so that a press of Explain branch meanwhile
/// finds it held and points at this tab rather than opening another. The
/// session's claim is taken only once the tests are done, by the launch,
/// so other launches in the session never wait on a test run.
pub fn run_here(workspace: &str, uuid: &str) -> Result<()> {
    // Kept only to be dropped when this function returns, after the wait for
    // Enter.
    let mut _tests = None;
    let done = Workspace::named(workspace).and_then(|ws| {
        _tests = Some(ws.session()?.hold_tests(uuid)?);
        test_then_explain(&ws, uuid).map(|()| ws)
    });
    work::end_tab(done)
}

fn test_then_explain(ws: &Workspace, uuid: &str) -> Result<()> {
    let t = task::get(uuid)?.context("task not found")?;
    let dirs = Dirs::from_env()?;
    let worktree = work::worktree_path(ws, &t.uuid).context("This task has no worktree, so no branch to explain.")?;
    let base = work::default_branch(&ws.folder(&dirs))?;
    // Looked for again: the press found one, but this is what runs.
    let tests = match test_command(&worktree)? {
        Some(cmd) => {
            eprintln!("Running {} in {}\n", cmd.display(), worktree.display());
            Some(run_tests(&worktree, &cmd, &refine::reports_dir(&dirs), &t.uuid, SystemTime::now())?)
        }
        None => None,
    };
    refine::launch_branch(ws, &t, &refine::Branch { worktree, base, tests })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("niritasks-branch-tests-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").arg("-C").arg(dir).args(args).output().unwrap().status.success();
        assert!(ok, "git {args:?}");
    }

    /// A repository of its own, so the user's own git config's
    /// `niri-tasks.test`, if any, is not what is read.
    fn repo(label: &str) -> PathBuf {
        let dir = scratch(label);
        git(&dir, &["init", "-q"]);
        dir
    }

    /// The repository's own setting wins over `tests/all.sh`, which wins
    /// over nothing.
    #[test]
    fn the_test_command_is_the_config_else_all_sh_else_none() {
        let dir = repo("which");
        assert_eq!(test_command(&dir).unwrap(), None);
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        std::fs::write(dir.join("tests/all.sh"), "exit 0\n").unwrap();
        assert_eq!(
            test_command(&dir).unwrap(),
            Some(TestCommand { shell: "bash tests/all.sh".into(), env: vec![("NIRITASKS_HEADLESS".into(), "1".into())] })
        );
        git(&dir, &["config", CONFIG_KEY, "make check"]);
        assert_eq!(test_command(&dir).unwrap(), Some(TestCommand { shell: "make check".into(), env: vec![] }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The set environment comes first in the command the results file
    /// names.
    #[test]
    fn a_command_shows_its_environment_first() {
        let all = TestCommand { shell: "bash tests/all.sh".into(), env: vec![("NIRITASKS_HEADLESS".into(), "1".into())] };
        assert_eq!(all.display(), "NIRITASKS_HEADLESS=1 bash tests/all.sh");
        assert_eq!(TestCommand { shell: "make check".into(), env: vec![] }.display(), "make check");
    }

    /// A run writes the command's output to its log and the results file
    /// beside it, named by the task and the start, with its exit code: a
    /// failing command is a result, not an error.
    #[test]
    fn a_run_writes_its_log_and_results() {
        let work = scratch("run-work");
        let reviews = scratch("run-reviews").join("reviews");
        let cmd = TestCommand { shell: "echo hi; echo \"$WHO\" >&2; pwd; exit 3".into(), env: vec![("WHO".into(), "there".into())] };
        // 2026-10-28 01:02:03 UTC.
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_793_149_323);
        let json = run_tests(&work, &cmd, &reviews, "7CD9FD3A-d27b", now).unwrap();
        assert_eq!(json, reviews.join("branch-7cd9fd3a-20261028-010203-tests.json"));
        let log = reviews.join("branch-7cd9fd3a-20261028-010203-tests.log");
        let text = std::fs::read_to_string(&log).unwrap();
        assert_eq!(text, format!("hi\nthere\n{}\n", work.display()));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(v["command"], "WHO=there echo hi; echo \"$WHO\" >&2; pwd; exit 3");
        assert_eq!(v["exit_code"], 3);
        assert_eq!(v["log"], log.display().to_string());
        assert!(Path::new(v["log"].as_str().unwrap()).is_absolute());
        assert_eq!(v["started"], "2026-10-28T01:02:03Z");
        let finished = v["finished"].as_str().unwrap();
        assert!(finished.len() == 20 && finished.ends_with('Z') && finished.as_bytes()[10] == b'T', "{finished}");
        assert!(finished >= "2026-10-28T01:02:03Z", "{finished}");
        let _ = std::fs::remove_dir_all(&work);
        let _ = std::fs::remove_dir_all(reviews.parent().unwrap());
    }

    #[test]
    fn a_passing_run_says_exit_code_0() {
        let work = scratch("pass");
        let json = run_tests(&work, &TestCommand { shell: "true".into(), env: vec![] }, &work, "u", SystemTime::now()).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(v["exit_code"], 0);
        let _ = std::fs::remove_dir_all(&work);
    }

    /// The tests tab runs this binary's report on the task, in tab mode,
    /// on its workspace, every argument quoted for the pane's shell.
    #[test]
    fn the_tests_tab_runs_the_report_here() {
        assert_eq!(
            here_command("/bin/nt", "my project", "7cd9fd3a-d27b"),
            "'/bin/nt' task report '7cd9fd3a-d27b' --branch --here --workspace 'my project'"
        );
    }

    #[test]
    fn times_are_stamped_as_the_mod_stamps_them() {
        assert_eq!(file_stamp(1_793_149_323), "20261028-010203");
        assert_eq!(iso(1_793_149_323), "2026-10-28T01:02:03Z");
    }
}
