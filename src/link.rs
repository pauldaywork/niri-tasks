//! The link between a task and the herdr agent working on it: the agent's
//! name. Refine names its agent `task-<uuid8>`, Generate report its
//! `report-<uuid8>` and Start working its `work-<uuid8>`, so nothing is stored
//! on the task — the panel finds a task's agent by asking herdr for those
//! names. A Claude started by hand gets the same `work-` name, and its pane the
//! task's label, when it marks the task active. The names are spelled in
//! [`names`].

use crate::session::{current_pane, CurrentPane, Session};
use crate::workspace::Workspace;
use crate::names;
use anyhow::{Context, Result};

/// What to rename the agent in this pane to when it marks `uuid` active, or
/// `None` to leave it as it is.
///
/// A `task-`, `work-` or `report-` agent is left alone: it was named by Refine,
/// Generate report or Start working, or linked already, and taking that name
/// away would lose the task it was named for — a refine marking its task
/// active is still that task's refine, and a report still that task's report.
/// Any other agent, named or not, takes the task's `work-` name.
pub fn pane_link_name(current: Option<&str>, uuid: &str) -> Option<String> {
    if current.is_some_and(names::is_task_agent) {
        return None;
    }
    Some(names::work_agent(uuid))
}

/// Name the agent in the herdr pane this process runs in after `t`, as
/// [`pane_link_name`] decides, and label the pane with the task — so a Claude
/// started by hand, marking its task active, can be found from the task's
/// card like one Start working made.
///
/// Outside a herdr pane, in a pane with no agent (a script in a plain shell),
/// or without herdr at all, there is nothing to link and this does nothing.
/// Only a rename herdr refuses — the name held by another live agent, say —
/// is an error.
pub fn link_current_pane(t: &crate::task::Task) -> Result<()> {
    let Some(CurrentPane { session, pane: Some(pane), .. }) = current_pane() else {
        return Ok(());
    };
    link_pane(&Session::named(&session), &pane, t)
}

/// [`link_current_pane`]'s steps in `session`'s `pane`: name its agent after
/// `t` as [`pane_link_name`] decides and, when it did, label the pane with
/// the task, as Start working labels its own.
fn link_pane(session: &Session, pane: &str, t: &crate::task::Task) -> Result<()> {
    // Any failure to ask, herdr not installed included, is nothing to link.
    let Ok(Some(agent)) = session.agent(pane) else {
        return Ok(());
    };
    let Some(name) = pane_link_name(agent.name.as_deref(), &t.uuid) else {
        return Ok(());
    };
    session
        .rename_agent(pane, &name)
        .with_context(|| format!("The task is active, but this pane's agent could not be named {name}"))?;
    session.label_pane(pane, &names::PaneName::for_task(names::Role::Work, t.id, &t.uuid, &t.description));
    Ok(())
}

/// Which of a task's Claudes `task session` goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    /// Its working Claude if it has one, else its refine: bare `task
    /// session`.
    Either,
    /// Its working Claude, `work-<uuid8>`, alone: `--work`.
    Work,
    /// Its refine Claude, `task-<uuid8>`, alone: `--refine`.
    Refine,
}

impl Which {
    /// The agent names it goes to, the one it prefers first.
    fn names(self, uuid: &str) -> Vec<String> {
        match self {
            Which::Either => names::both_agents(uuid).to_vec(),
            Which::Work => vec![names::work_agent(uuid)],
            Which::Refine => vec![names::refine_agent(uuid)],
        }
    }

    /// What `task session` says with none of them live.
    fn none_live(self) -> &'static str {
        match self {
            Which::Either => "No Claude is working on this task in this workspace's herdr session.",
            Which::Work => "No working Claude is on this task in this workspace's herdr session.",
            Which::Refine => "No refine Claude is on this task in this workspace's herdr session.",
        }
    }
}

/// Which of `names` is `uuid`'s agent, as `which` picks: with
/// [`Which::Either`] its working Claude (`work-`) if it has one, else its
/// refine (`task-`).
pub fn session_agent(names: &[String], uuid: &str, which: Which) -> Option<String> {
    which.names(uuid).into_iter().find(|want| names.iter().any(|n| n == want))
}

/// Every named live agent in `ws`'s herdr session, from one `agent list` —
/// what the panel asks once for all its cards. A session that is not
/// running, or no herdr at all, has none: the panel asks this on every open
/// and must not fail for it.
pub fn live_agent_names(ws: &Workspace) -> Vec<String> {
    // By name alone: listing agents needs no folder, so no HOME is read.
    Session::named(&ws.session_name()).agent_names()
}

/// The live agent `which` picks for `uuid` in `ws`'s herdr session, if
/// there is one.
pub fn live_agent(ws: &Workspace, uuid: &str, which: Which) -> Option<String> {
    session_agent(&live_agent_names(ws), uuid, which)
}

/// Bring the terminal showing `ws`'s herdr session forward and focus the
/// agent `which` picks for `uuid` in it. Never starts anything: with no
/// live agent — it exited since the panel slid out, say — this is an error,
/// not a new session.
pub fn go_to(ws: &Workspace, uuid: &str, which: Which) -> Result<()> {
    let name = live_agent(ws, uuid, which).context(which.none_live())?;
    let session = ws.session()?;
    session.open()?;
    // It can still exit between the panel's list and this focus.
    anyhow::ensure!(session.focus_agent(&name)?, "The Claude working on this task has gone.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::fake::Fake;
    use std::path::PathBuf;

    fn task(id: u64, uuid: &str, description: &str) -> crate::task::Task {
        serde_json::from_value(serde_json::json!({"id": id, "uuid": uuid, "description": description})).unwrap()
    }

    /// A Claude started by hand takes the task's work- name and its pane the
    /// task's label.
    #[test]
    fn linking_a_hand_started_claude_names_and_labels_its_pane() {
        let fake = Fake::running(&[("w1", "alpha")]).with_agent(None, "p1", "idle");
        let s = Session::with_port("alpha", PathBuf::new(), Box::new(fake.clone()));
        link_pane(&s, "p1", &task(61, "7cd9fd3a-d27b", "fix the peek")).unwrap();
        let log = fake.log();
        assert!(log.contains(&"agent_rename alpha p1 work-7cd9fd3a".to_string()), "{log:?}");
        assert!(log.contains(&"pane_rename alpha p1 Work #61 · fix the peek".to_string()), "{log:?}");
    }

    /// A refine marking its task active keeps its name, and so its label.
    #[test]
    fn linking_a_refine_pane_leaves_its_label_alone() {
        let fake = Fake::running(&[("w1", "alpha")]).with_agent(Some("task-7cd9fd3a"), "p1", "idle");
        let s = Session::with_port("alpha", PathBuf::new(), Box::new(fake.clone()));
        link_pane(&s, "p1", &task(61, "7cd9fd3a-d27b", "fix the peek")).unwrap();
        assert!(!fake.log().iter().any(|l| l.starts_with("agent_rename") || l.starts_with("pane_rename")), "{:?}", fake.log());
    }

    #[test]
    fn an_unnamed_or_otherwise_named_agent_takes_the_tasks_work_name() {
        assert_eq!(pane_link_name(None, "7CD9FD3A-d27b-4387").as_deref(), Some("work-7cd9fd3a"));
        assert_eq!(pane_link_name(Some("reviewer"), "7cd9fd3a-d27b").as_deref(), Some("work-7cd9fd3a"));
    }

    #[test]
    fn a_pane_named_by_refine_report_or_start_working_is_left_alone() {
        assert_eq!(pane_link_name(Some("task-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("report-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("work-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("work-7cd9fd3a"), "7cd9fd3a-d27b"), None, "already linked");
    }

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    /// The working Claude is the one you most likely want back; a refine
    /// still open beside it is second.
    #[test]
    fn the_working_claude_is_preferred_over_a_refine() {
        let live = names(&["task-7cd9fd3a", "work-7cd9fd3a"]);
        assert_eq!(session_agent(&live, "7CD9FD3A-d27b", Which::Either).as_deref(), Some("work-7cd9fd3a"));
    }

    #[test]
    fn a_refine_alone_is_still_a_session_to_go_to() {
        let live = names(&["reviewer", "task-7cd9fd3a"]);
        assert_eq!(session_agent(&live, "7cd9fd3a-d27b", Which::Either).as_deref(), Some("task-7cd9fd3a"));
    }

    #[test]
    fn another_tasks_agents_are_not_this_ones() {
        let live = names(&["work-11111111", "task-22222222", "reviewer"]);
        assert_eq!(session_agent(&live, "7cd9fd3a-d27b", Which::Either), None);
        assert_eq!(session_agent(&[], "7cd9fd3a-d27b", Which::Either), None);
    }

    /// --work and --refine go to that Claude alone, never the other one in
    /// its place.
    #[test]
    fn work_and_refine_pick_their_own_claude_and_no_other() {
        let both = names(&["task-7cd9fd3a", "work-7cd9fd3a"]);
        assert_eq!(session_agent(&both, "7cd9fd3a-d27b", Which::Work).as_deref(), Some("work-7cd9fd3a"));
        assert_eq!(session_agent(&both, "7cd9fd3a-d27b", Which::Refine).as_deref(), Some("task-7cd9fd3a"));
        assert_eq!(session_agent(&names(&["task-7cd9fd3a"]), "7cd9fd3a-d27b", Which::Work), None);
        assert_eq!(session_agent(&names(&["work-7cd9fd3a"]), "7cd9fd3a-d27b", Which::Refine), None);
    }
}
