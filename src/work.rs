//! Starting work on a task: its own git worktree, made by worktrunk and opened
//! as a workspace in the project's herdr session, with Claude planning it.
//!
//! The worktree is found again by the task's uuid, never its description, so
//! a description reworded since (by Refine, say) cannot fork a second one.

use crate::dirs::Dirs;
use crate::session::{current_pane, Claude, Session};
use crate::workspace::Workspace;
use crate::{notify, project, task, text};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Longest description, in characters, the setup tab's label carries.
const LABEL_DESCRIPTION_MAX: usize = 30;

/// A task's branch and the worktree it is checked out in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskWorktree {
    pub branch: String,
    pub path: PathBuf,
}

/// Longest slug, in characters: long enough to read, short enough that the
/// branch fits herdr's sidebar and a worktree path stays sane.
const SLUG_MAX: usize = 40;

/// The readable half of a task's branch name: its description in lowercase
/// ASCII, every other run of characters one `-`. Cut at a word when longer
/// than [`SLUG_MAX`]; `task` when nothing is left. The uuid, not this, is what
/// makes the branch unique.
pub fn slug(description: &str) -> String {
    let mut out = String::new();
    for c in description.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let mut s = out.trim_end_matches('-').to_string();
    if s.len() > SLUG_MAX {
        // All ASCII by now, so slicing by byte cannot split a character.
        let cut = &s[..SLUG_MAX];
        s = match cut.rfind('-') {
            Some(i) if i > 0 => cut[..i].to_string(),
            _ => cut.to_string(),
        };
    }
    if s.is_empty() {
        "task".to_string()
    } else {
        s
    }
}

/// The first eight characters of a uuid, lowercased — enough to tell one
/// task's branch and agent from another's.
fn uuid8(uuid: &str) -> String {
    uuid.chars().take(8).collect::<String>().to_ascii_lowercase()
}

/// The branch a task is worked on: `task/<slug>-<uuid8>`, readable in git and
/// herdr, unique by the uuid.
pub fn branch_name(description: &str, uuid: &str) -> String {
    format!("task/{}-{}", slug(description), uuid8(uuid))
}

/// The herdr agent name for a task's working Claude. `work-`, not Refine's
/// `task-`, so a refine still open on the task is never mistaken for it.
pub fn work_agent_name(uuid: &str) -> String {
    format!("work-{}", uuid8(uuid))
}

/// What the working Claude is asked first: plan the task, reading the task
/// itself for the spec — only the uuid crosses, as with Refine, so nothing
/// needs quoting through herdr and it always sees the current version.
pub fn plan_prompt(uuid: &str) -> String {
    format!(
        "/superpowers:writing-plans Plan Taskwarrior task {uuid}. Read it with \
         `task rc.json.array=on {uuid} export`; its description and notes are the spec."
    )
}

/// The task's worktree in a `wt list --format=json`, if it has one: a
/// `task/…-<uuid8>` branch checked out somewhere. Matched by uuid alone, so a
/// branch named after an older description still counts.
pub fn find_task_worktree(list: &Value, uuid: &str) -> Option<TaskWorktree> {
    let suffix = format!("-{}", uuid8(uuid));
    list["items"].as_array()?.iter().find_map(|item| {
        let branch = item["branch"].as_str()?;
        if !(branch.starts_with("task/") && branch.ends_with(&suffix)) {
            return None;
        }
        let path = item["worktree"]["path"].as_str()?;
        Some(TaskWorktree {
            branch: branch.to_string(),
            path: PathBuf::from(path),
        })
    })
}

/// The branch and worktree a `wt switch --format=json` landed on.
pub fn switch_result(result: &Value) -> Option<TaskWorktree> {
    Some(TaskWorktree {
        branch: result["branch"].as_str()?.to_string(),
        path: PathBuf::from(result["path"].as_str()?),
    })
}

/// One argument quoted for the POSIX shell a herdr pane runs — the setup step
/// is typed into one, and a workspace name may hold spaces or quotes.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// The repository a workspace's tasks are worked in: its `~/Projects` folder,
/// which has to be a git repository for there to be worktrees at all.
fn repo_for(ws: &Workspace) -> Result<PathBuf> {
    let repo = ws.folder(&Dirs::from_env()?);
    anyhow::ensure!(
        repo.join(".git").exists(),
        "{} is not a git repository, so there is no worktree to make.",
        repo.display()
    );
    Ok(repo)
}

/// worktrunk's list of the repository's branches and worktrees.
fn wt_list(repo: &Path) -> Result<Value> {
    let out = Command::new("wt")
        .arg("-C")
        .arg(repo)
        .args(["list", "--format=json"])
        .output()
        .context("could not run `wt` — is worktrunk installed?")?;
    anyhow::ensure!(out.status.success(), "`wt list` failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    serde_json::from_slice(&out.stdout).context("could not parse `wt list` output as JSON")
}

/// Start working on a task from its card: back to its worktree if it has one,
/// otherwise a short-lived tab in the project's session that makes one.
///
/// Making it happens in that tab rather than here because worktrunk asks the
/// user to approve a repo's hooks before running them the first time, and
/// refuses outright without a terminal to ask on; the tab is also where the
/// hooks' output — a database clone, say — can be read.
///
/// The task as found, not a uuid as typed: the worktree and the agent are
/// found again by the uuid's first eight characters, and a typed task number
/// has none of them.
pub fn launch(ws: &Workspace, t: &task::Task) -> Result<()> {
    let repo = repo_for(ws)?;
    anyhow::ensure!(project::on_path("wt"), "worktrunk (wt) is not installed.");
    let session = ws.session()?;
    let name = work_agent_name(&t.uuid);

    let workspaces = session.open()?;

    if let Some(wt) = find_task_worktree(&wt_list(&repo)?, &t.uuid) {
        let opened = session.open_worktree(&repo, &wt.path, &wt.branch)?;
        if session.focus_agent(&name)? {
            notify::tasks("Back to its worktree.");
            return Ok(());
        }
        // The worktree outlived its Claude: a fresh one, in a tab of its own
        // so whatever the workspace's first pane is doing is left alone.
        let tab = session.tab_in(&opened.workspace, &wt.path, "Claude")?;
        return start_working(&session, &name, &tab.pane, &t.uuid);
    }

    let label = format!("Start: {}", short(&t.description));
    let tab = session.new_tab(&workspaces, &repo, &label, ws.name())?;
    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    let command = format!(
        "{} task start --here --workspace {} {}",
        sh_quote(&exe.display().to_string()),
        sh_quote(ws.name()),
        sh_quote(&t.uuid)
    );
    session.run_in_pane(&tab.pane, &command)
}

/// Start the working Claude in `pane`, hand it the task to plan, and mark the
/// task active, as Update status → Active would. Other active tasks on the
/// workspace stay active: they are other worktrees' agents at work.
fn start_working(session: &Session, name: &str, pane: &str, uuid: &str) -> Result<()> {
    session.start_claude(name, pane, &Claude::Worker)?;
    session.prompt(name, &plan_prompt(uuid))?;
    task::set_active(uuid)
}

/// The setup step, run inside the tab [`launch`] opened: make the worktree
/// (approval and hook output land here), open it as its own workspace, start
/// Claude there, then close this tab. On failure the tab stays, with the
/// error, until the user has read it.
///
/// `workspace` is the raw `--workspace` name, checked here rather than by the
/// caller so that a name [`Workspace::named`] refuses also holds the tab open
/// with its error, like any other failure.
pub fn set_up_here(workspace: &str, uuid: &str) -> Result<()> {
    match Workspace::named(workspace).and_then(|ws| set_up(&ws, uuid).map(|()| ws)) {
        Ok(ws) => {
            // Best effort: closing our own tab ends this process, and a tab
            // left open is only untidy. It is closed through the workspace's
            // session, the one launch opened it in.
            if let Some(tab) = current_pane().and_then(|p| p.tab) {
                if let Ok(session) = ws.session() {
                    session.close_tab(&tab);
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("\n{e:#}\n\nPress Enter to close this tab.");
            let _ = std::io::stdin().read_line(&mut String::new());
            Err(e)
        }
    }
}

fn set_up(ws: &Workspace, uuid: &str) -> Result<()> {
    let t = task::get(uuid)?.context("task not found")?;
    // The uuid as found, not as typed, from here on: the branch and the agent
    // are named after its first eight characters, and a typed task number
    // has none of them.
    let uuid = t.uuid.as_str();
    let repo = repo_for(ws)?;
    let session = ws.session()?;

    // Found again first: a retry after a run that made the worktree but
    // failed later must not try to make it twice.
    let wt = match find_task_worktree(&wt_list(&repo)?, uuid) {
        Some(wt) => wt,
        None => create_worktree(&repo, &branch_name(&t.description, uuid))?,
    };

    let opened = session.open_worktree(&repo, &wt.path, &wt.branch)?;
    let pane = opened.pane.context("herdr did not say which pane it opened")?;
    start_working(&session, &work_agent_name(uuid), &pane, uuid)
}

/// `wt switch --create`, with this tab's terminal on stdin and stderr so
/// worktrunk can ask to approve the repo's hooks and show what they print; only
/// stdout, the JSON result, is captured. Never `--yes`: approving hook text is
/// the user's call (ADR 0002 in ubuntu-setup).
fn create_worktree(repo: &Path, branch: &str) -> Result<TaskWorktree> {
    let out = Command::new("wt")
        .arg("-C")
        .arg(repo)
        .args(["switch", "--create", branch, "--no-cd", "--format=json"])
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context("could not run `wt` — is worktrunk installed?")?;
    anyhow::ensure!(out.status.success(), "`wt switch --create {branch}` failed (see above).");
    let result: Value = serde_json::from_slice(&out.stdout).context("could not parse `wt switch` output as JSON")?;
    switch_result(&result).context("worktrunk did not say which worktree it made")
}

/// A description cut to fit a tab label, by characters, with an ellipsis.
fn short(description: &str) -> String {
    let d = text::collapse_whitespace(description);
    if d.chars().count() > LABEL_DESCRIPTION_MAX {
        let cut: String = d.chars().take(LABEL_DESCRIPTION_MAX - 1).collect();
        format!("{cut}…")
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slug_is_the_description_in_lowercase_ascii_runs() {
        assert_eq!(
            slug("Show each Claude agent's topic in herdr's sidebar!"),
            "show-each-claude-agent-s-topic-in-herdr"
        );
        assert_eq!(slug("  Fix   café bug  "), "fix-caf-bug");
        assert_eq!(slug("!!!"), "task");
    }

    #[test]
    fn a_long_slug_is_cut_at_a_word() {
        let s = slug("alpha beta gamma delta epsilon zeta eta theta iota");
        assert!(s.len() <= 40, "{s}");
        assert!(!s.ends_with('-'));
        assert_eq!(s, "alpha-beta-gamma-delta-epsilon-zeta-eta");
    }

    #[test]
    fn the_branch_and_agent_are_named_after_the_task() {
        let u = "7CD9FD3A-d27b-4387-8249-aaf0d6785f90";
        assert_eq!(branch_name("Fix the peek", u), "task/fix-the-peek-7cd9fd3a");
        assert_eq!(work_agent_name(u), "work-7cd9fd3a");
        assert_eq!(
            plan_prompt("u-1"),
            "/superpowers:writing-plans Plan Taskwarrior task u-1. Read it with `task rc.json.array=on u-1 export`; its description and notes are the spec."
        );
    }

    /// Shape from worktrunk 0.79's `wt list --format=json`.
    #[test]
    fn a_tasks_worktree_is_found_by_uuid_alone() {
        let list: Value = serde_json::from_str(
            r#"{"schema":2,"items":[
            {"branch":"main","worktree":{"path":"/p/repo"}},
            {"branch":"task/old-name-7cd9fd3a","worktree":{"path":"/w/repo/task-old-name-7cd9fd3a"}},
            {"branch":"task/other-11111111","worktree":{"path":"/w/repo/task-other-11111111"}},
            {"branch":"task/branch-only-22222222"}
        ]}"#,
        )
        .unwrap();
        let found = find_task_worktree(&list, "7cd9fd3a-0000").unwrap();
        assert_eq!(found.branch, "task/old-name-7cd9fd3a");
        assert_eq!(found.path, PathBuf::from("/w/repo/task-old-name-7cd9fd3a"));
        assert!(find_task_worktree(&list, "22222222-0000").is_none(), "a branch with no worktree is not one");
        assert!(find_task_worktree(&list, "99999999-0000").is_none());
    }

    /// Shape from worktrunk 0.79's `wt switch --format=json`.
    #[test]
    fn a_switch_result_gives_branch_and_path() {
        let v: Value = serde_json::from_str(
            r#"{"action":"created","branch":"task/x-1234abcd","path":"/w/repo/task-x-1234abcd"}"#,
        )
        .unwrap();
        let w = switch_result(&v).unwrap();
        assert_eq!(
            (w.branch.as_str(), w.path.to_str().unwrap()),
            ("task/x-1234abcd", "/w/repo/task-x-1234abcd")
        );
    }

    #[test]
    fn arguments_are_quoted_for_the_panes_shell() {
        assert_eq!(sh_quote("my project"), "'my project'");
        assert_eq!(sh_quote("it's"), r#"'it'\''s'"#);
    }
}
