//! The link between a task and the herdr agent working on it: the agent's
//! name. Refine names its agent `task-<uuid8>` and Start working names its
//! `work-<uuid8>`, so nothing is stored on the task — the menu finds a task's
//! agent by asking herdr for those names. A Claude started by hand gets the
//! same `work-` name when it marks the task active.

use crate::{herdr, session, work};
use anyhow::{Context, Result};

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
/// active, can be found from the task's menu like one Start working made.
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
}
