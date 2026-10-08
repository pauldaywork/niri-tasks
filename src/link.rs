//! The link between a task and the herdr agent working on it: the agent's
//! name. Refine names its agent `task-<uuid8>` and Start working names its
//! `work-<uuid8>`, so nothing is stored on the task — the panel finds a task's
//! agent by asking herdr for those names. A Claude started by hand gets the
//! same `work-` name when it marks the task active.

use crate::session::herdr;
use crate::{refine, session, work};
use anyhow::{Context, Result};
use std::path::Path;

/// What to rename the agent in this pane to when it marks `uuid` active, or
/// `None` to leave it as it is.
///
/// A `task-` or `work-` agent is left alone: it was named by Refine or Start
/// working, or linked already, and taking that name away would lose the task
/// it was named for — a refine marking its task active is still that task's
/// refine. Any other agent, named or not, takes the task's `work-` name.
pub fn pane_link_name(current: Option<&str>, uuid: &str) -> Option<String> {
    if current.is_some_and(|n| n.starts_with("task-") || n.starts_with("work-")) {
        return None;
    }
    Some(work::work_agent_name(uuid))
}

/// Name the agent in the herdr pane this process runs in after `uuid`, as
/// [`pane_link_name`] decides — so a Claude started by hand, marking its task
/// active, can be found from the task's card like one Start working made.
///
/// Outside a herdr pane, in a pane with no agent (a script in a plain shell),
/// or without herdr at all, there is nothing to link and this does nothing.
/// Only a rename herdr refuses — the name held by another live agent, say —
/// is an error.
pub fn link_current_pane(uuid: &str) -> Result<()> {
    let Some(pane) = std::env::var("HERDR_PANE_ID").ok().filter(|p| !p.is_empty()) else {
        return Ok(());
    };
    let Some(s) = session::session_from_env(
        std::env::var("HERDR_SESSION").ok().as_deref(),
        std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
    ) else {
        return Ok(());
    };
    let Ok(got) = herdr::run(&herdr::agent_get(&s, &pane)) else {
        return Ok(());
    };
    let Some(name) = pane_link_name(herdr::agent_name_of(&got).as_deref(), uuid) else {
        return Ok(());
    };
    herdr::run(&herdr::agent_rename(&s, &pane, &name))
        .with_context(|| format!("The task is active, but this pane's agent could not be named {name}"))?;
    Ok(())
}

/// Which of `names` is `uuid`'s agent: its working Claude (`work-`) if it
/// has one, else its refine (`task-`).
pub fn session_agent(names: &[String], uuid: &str) -> Option<String> {
    [work::work_agent_name(uuid), refine::agent_name(uuid)]
        .into_iter()
        .find(|want| names.iter().any(|n| n == want))
}

/// Every named live agent in `workspace`'s herdr session, from one
/// `agent list` — what the panel asks once for all its cards. A session that
/// is not running, or no herdr at all, has none: the panel asks
/// this on every open and must not fail for it.
pub fn live_agent_names(workspace: &str) -> Vec<String> {
    let s = session::herdr_session_name(workspace);
    herdr::run(&herdr::agent_list(&s))
        .map(|list| herdr::agent_names(&list))
        .unwrap_or_default()
}

/// The live agent working on `uuid` in `workspace`'s herdr session, if there
/// is one.
pub fn live_agent(workspace: &str, uuid: &str) -> Option<String> {
    session_agent(&live_agent_names(workspace), uuid)
}

/// Bring the terminal showing `workspace`'s herdr session forward and focus
/// the agent working on `uuid` in it. Never starts anything: with no live
/// agent — it exited since the panel slid out, say — this is an error, not a
/// new session.
pub fn go_to(workspace: &str, uuid: &str) -> Result<()> {
    let name = live_agent(workspace, uuid)
        .context("No Claude is working on this task in this workspace's herdr session.")?;
    let home = std::env::var("HOME").context("HOME is unset")?;
    let dir = session::start_dir(Path::new(&home), workspace);
    let s = session::herdr_session_name(workspace);
    refine::open_session(&dir, &s)?;
    herdr::run(&herdr::agent_focus(&s, &name))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unnamed_or_otherwise_named_agent_takes_the_tasks_work_name() {
        assert_eq!(pane_link_name(None, "7CD9FD3A-d27b-4387").as_deref(), Some("work-7cd9fd3a"));
        assert_eq!(pane_link_name(Some("reviewer"), "7cd9fd3a-d27b").as_deref(), Some("work-7cd9fd3a"));
    }

    #[test]
    fn a_pane_named_by_refine_or_start_working_is_left_alone() {
        assert_eq!(pane_link_name(Some("task-11111111"), "7cd9fd3a-d27b"), None);
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
        assert_eq!(session_agent(&live, "7CD9FD3A-d27b").as_deref(), Some("work-7cd9fd3a"));
    }

    #[test]
    fn a_refine_alone_is_still_a_session_to_go_to() {
        let live = names(&["reviewer", "task-7cd9fd3a"]);
        assert_eq!(session_agent(&live, "7cd9fd3a-d27b").as_deref(), Some("task-7cd9fd3a"));
    }

    #[test]
    fn another_tasks_agents_are_not_this_ones() {
        let live = names(&["work-11111111", "task-22222222", "reviewer"]);
        assert_eq!(session_agent(&live, "7cd9fd3a-d27b"), None);
        assert_eq!(session_agent(&[], "7cd9fd3a-d27b"), None);
    }
}
