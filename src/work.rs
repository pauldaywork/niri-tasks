//! Starting work on a task: its own git worktree, made by worktrunk and opened
//! as a workspace in the project's herdr session, with Claude planning it.
//!
//! The worktree is found again by the task's uuid, never its description, so
//! a description reworded since (by Refine, say) cannot fork a second one.

use crate::dirs::Dirs;
use crate::session::{current_pane, Claim, Claude, Session};
use crate::workspace::Workspace;
use crate::{names, notify, programs, task};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// The branch a task is worked on: `task/<slug>-<uuid8>`, readable in git and
/// herdr, unique by the uuid.
pub fn branch_name(description: &str, uuid: &str) -> String {
    format!("task/{}-{}", slug(description), names::uuid8(uuid))
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
    let suffix = format!("-{}", names::uuid8(uuid));
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
///
/// Everything from opening the session to Claude's start, or to the setup
/// tab's process holding the task's setup lock, runs under the session's
/// claim, so a second press waits and then finds this one's Claude,
/// worktree or setup tab rather than starting its own. A press that finds a
/// setup tab under way, even one showing an error, points at it instead.
pub fn launch(ws: &Workspace, t: &task::Task) -> Result<()> {
    let repo = repo_for(ws)?;
    anyhow::ensure!(programs::on_path("wt"), "worktrunk (wt) is not installed.");
    let session = ws.session()?;
    let name = names::work_agent(&t.uuid);

    let claim = session.claim()?;
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
        return start_working(&session, claim, &name, &tab.pane, &t.uuid);
    }

    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    let command = format!(
        "{} task start --here --workspace {} {}",
        sh_quote(&exe.display().to_string()),
        sh_quote(ws.name()),
        sh_quote(&t.uuid)
    );
    open_setup_tab(&session, claim, &workspaces, &repo, ws.name(), t, &command)
}

/// What a press of Start working says when a setup tab for its task, from an
/// earlier press, is still open.
const SETUP_UNDER_WAY: &str = "Already being set up — see its tab.";

/// Open `t`'s setup tab and run `command` in it, under the launch's `claim`
/// — unless a setup tab for it is under way already, from an earlier press,
/// in which case the user is pointed at that one and nothing opens.
///
/// The claim is kept past `run_in_pane` until the tab's process is seen
/// holding the setup lock: in between, a second press waiting on the claim
/// would find neither a worktree nor a held lock, and open a second tab. A
/// tab whose command never starts is waited for a few seconds at most.
fn open_setup_tab(
    session: &Session,
    claim: Claim,
    workspaces: &[crate::session::Workspace],
    repo: &Path,
    workspace_label: &str,
    t: &task::Task,
    command: &str,
) -> Result<()> {
    if session.setup_under_way(&t.uuid)? {
        session.notify(SETUP_UNDER_WAY);
        return Ok(());
    }
    let label = format!("Start: {}", names::elide(&t.description));
    let tab = session.new_tab(workspaces, repo, &label, workspace_label)?;
    session.run_in_pane(&tab.pane, command)?;
    session.wait_for_setup(&t.uuid)?;
    drop(claim);
    Ok(())
}

/// Start the working Claude in `pane`, hand it the task to plan, and mark the
/// task active, as Update status → Active would. Other active tasks on the
/// workspace stay active: they are other worktrees' agents at work.
fn start_working(session: &Session, claim: Claim, name: &str, pane: &str, uuid: &str) -> Result<()> {
    start_and_prompt(session, claim, name, pane, &plan_prompt(uuid))?;
    task::set_active(uuid)
}

/// Start the working Claude in `pane` under `claim`, then send `text`. The
/// start lets the claim go once herdr has the agent, before a blocked
/// start's question or the prompt's retries: a second launch waiting on it
/// then finds the agent and focuses it.
fn start_and_prompt(session: &Session, claim: Claim, name: &str, pane: &str, text: &str) -> Result<()> {
    session.start_claude(claim, name, pane, &Claude::Worker)?;
    session.prompt(name, text)
}

/// The setup step, run inside the tab [`launch`] opened: make the worktree
/// (approval and hook output land here), open it as its own workspace, start
/// Claude there, then close this tab. On failure the tab stays, with the
/// error, until the user has read it.
///
/// It holds the task's setup lock from the start until this process ends,
/// the error's wait included, so that a press of Start working meanwhile
/// finds it held and points at this tab rather than opening another. The
/// kernel drops it with the process, so a closed tab never leaves it held.
/// `uuid` is the full one [`launch`] passes, whose first eight characters
/// name the lock.
///
/// `workspace` is the raw `--workspace` name, checked here rather than by the
/// caller so that a name [`Workspace::named`] refuses also holds the tab open
/// with its error, like any other failure.
pub fn set_up_here(workspace: &str, uuid: &str) -> Result<()> {
    // Kept only to be dropped when this function returns, after the wait for
    // Enter below.
    let mut _setup = None;
    let done = Workspace::named(workspace).and_then(|ws| {
        _setup = Some(ws.session()?.hold_setup(uuid)?);
        set_up(&ws, uuid).map(|()| ws)
    });
    match done {
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

    // Claimed only now the worktree exists: making it can wait minutes on the
    // user approving the repo's hooks, and every other launch in the session
    // would wait with it. A second press meanwhile finds this tab's setup
    // lock held and opens no tab of its own; under the claim, the agent is
    // still looked for before one is started, in case one was started by
    // hand or by a tab from before the lock.
    let name = names::work_agent(uuid);
    let claim = session.claim()?;
    let opened = session.open_worktree(&repo, &wt.path, &wt.branch)?;
    if session.focus_agent(&name)? {
        notify::tasks("Back to its worktree.");
        return Ok(());
    }
    let pane = opened.pane.context("herdr did not say which pane it opened")?;
    start_working(&session, claim, &name, &pane, uuid)
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
    fn the_branch_is_named_after_the_task() {
        let u = "7CD9FD3A-d27b-4387-8249-aaf0d6785f90";
        assert_eq!(branch_name("Fix the peek", u), "task/fix-the-peek-7cd9fd3a");
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

    use crate::session::fake::{Fake, StartOutcome};

    fn session(fake: &Fake) -> Session {
        Session::with_port("alpha", PathBuf::from("/p/alpha"), Box::new(fake.clone()))
    }

    fn at(log: &[String], line: &str) -> usize {
        log.iter().position(|l| l.starts_with(line)).unwrap_or_else(|| panic!("no {line:?} in {log:?}"))
    }

    /// The claim a launch took goes once Claude has started and before the
    /// prompt, whose retries can take seconds; a second press waiting on it
    /// then finds the agent already there.
    #[test]
    fn the_claim_goes_after_the_start_and_before_the_prompt() {
        let fake = Fake::running(&[("w1", "alpha")]);
        let s = session(&fake);
        start_and_prompt(&s, s.claim().unwrap(), "work-abc", "p1", "/plan").unwrap();
        let log = fake.log();
        assert_eq!(log[0], "claim alpha", "{log:?}");
        assert!(at(&log, "agent_start alpha work-abc") < at(&log, "release alpha"), "{log:?}");
        assert!(at(&log, "release alpha") < at(&log, "agent_prompt alpha work-abc"), "{log:?}");
    }

    /// A start blocked on a question lets the claim go before the answer
    /// is waited for, and still before the prompt.
    #[test]
    fn a_blocked_start_lets_the_claim_go_before_the_wait() {
        let fake = Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::FailsBlockedThenIdle);
        let s = session(&fake);
        start_and_prompt(&s, s.claim().unwrap(), "work-abc", "p1", "/plan").unwrap();
        let log = fake.log();
        assert!(at(&log, "agent_start alpha work-abc") < at(&log, "release alpha"), "{log:?}");
        assert!(at(&log, "release alpha") < at(&log, "agent_wait_ready alpha work-abc"), "{log:?}");
        assert!(at(&log, "release alpha") < at(&log, "agent_prompt alpha work-abc"), "{log:?}");
    }

    /// A start that fails lets the claim go too, and sends no prompt.
    #[test]
    fn a_failed_start_lets_the_claim_go() {
        let fake = Fake::running(&[("w1", "alpha")]).with_start(StartOutcome::Fails);
        let s = session(&fake);
        assert!(start_and_prompt(&s, s.claim().unwrap(), "work-abc", "p1", "/plan").is_err());
        let log = fake.log();
        assert_eq!(log[at(&log, "agent_start alpha work-abc") + 1], "release alpha", "{log:?}");
        assert!(!log.iter().any(|l| l.starts_with("agent_prompt")), "{log:?}");
    }

    fn task(uuid: &str, description: &str) -> task::Task {
        serde_json::from_value(serde_json::json!({"uuid": uuid, "description": description})).unwrap()
    }

    fn workspaces() -> Vec<crate::session::Workspace> {
        vec![crate::session::Workspace { id: "w1".into(), label: "alpha".into() }]
    }

    const U: &str = "7cd9fd3a-d27b-4387-8249-aaf0d6785f90";

    /// A press that finds a setup tab already under way for its task opens
    /// no tab, runs nothing and points the user at the tab there is.
    #[test]
    fn a_launch_finding_its_setup_under_way_opens_no_tab() {
        let fake = Fake::running(&[("w1", "alpha")]).with_setup_held_after(0);
        let s = session(&fake);
        let claim = s.claim().unwrap();
        open_setup_tab(&s, claim, &workspaces(), Path::new("/p/alpha"), "alpha", &task(U, "Fix it"), "setup").unwrap();
        let log = fake.log();
        assert!(
            !log.iter().any(|l| l.starts_with("tab_create") || l.starts_with("workspace_create") || l.starts_with("pane_run")),
            "{log:?}"
        );
        assert!(log.contains(&format!("notify {SETUP_UNDER_WAY}")), "{log:?}");
    }

    /// A launch keeps its claim past running the setup command until it sees
    /// the tab holding the setup lock, so a second press waiting on the
    /// claim finds the lock held rather than opening a tab of its own.
    #[test]
    fn a_launch_keeps_its_claim_until_its_setup_tab_holds_the_lock() {
        // One free try for the look before the tab, two while the tab starts.
        let fake = Fake::running(&[("w1", "alpha")]).with_setup_held_after(3);
        let s = session(&fake);
        let claim = s.claim().unwrap();
        open_setup_tab(&s, claim, &workspaces(), Path::new("/p/alpha"), "alpha", &task(U, "Fix it"), "setup").unwrap();
        let log = fake.log();
        assert!(at(&log, "setup_release") < at(&log, "tab_create alpha w1 /p/alpha Start: Fix it"), "{log:?}");
        assert!(at(&log, "pane_run alpha") < at(&log, "setup_held"), "{log:?}");
        assert!(at(&log, "setup_held") < at(&log, "release alpha"), "{log:?}");
        assert!(!log.iter().any(|l| l.starts_with("notify")), "{log:?}");
    }

    /// A setup tab that never takes the lock is waited for a few seconds,
    /// then the claim goes anyway.
    #[test]
    fn a_launch_lets_its_claim_go_when_its_setup_tab_never_shows() {
        let fake = Fake::running(&[("w1", "alpha")]);
        let s = session(&fake);
        let claim = s.claim().unwrap();
        open_setup_tab(&s, claim, &workspaces(), Path::new("/p/alpha"), "alpha", &task(U, "Fix it"), "setup").unwrap();
        let log = fake.log();
        assert_eq!(log.last().map(String::as_str), Some("release alpha"), "{log:?}");
        assert!(at(&log, "pane_run alpha") < at(&log, "release alpha"), "{log:?}");
    }
}
