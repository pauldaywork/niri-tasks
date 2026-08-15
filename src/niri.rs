//! Talking to niri over `$NIRI_SOCKET`.
//!
//! The shell scripts shelled out to `niri msg -j …` and piped the result
//! through `jq`. This replaces both: one socket connection, typed responses.

use anyhow::{bail, Context, Result};
use niri_ipc::{socket::Socket, Action, Request, Response, Workspace, WorkspaceReferenceArg};

/// Ask niri for the current workspace list.
pub fn workspaces() -> Result<Vec<Workspace>> {
    let mut socket = Socket::connect().context("niri is not running, or $NIRI_SOCKET is unset")?;
    match socket.send(Request::Workspaces).context("niri request failed")? {
        Ok(Response::Workspaces(ws)) => Ok(ws),
        Ok(other) => bail!("unexpected reply to Workspaces: {other:?}"),
        Err(e) => bail!("niri refused the Workspaces request: {e}"),
    }
}

/// The focused workspace's name, or `None`.
///
/// niri workspaces are unnamed until something names them, and an unnamed
/// workspace has no tag to scope tasks to.
pub fn focused_workspace_name() -> Result<Option<String>> {
    Ok(workspaces()?
        .into_iter()
        .find(|w| w.is_focused)
        .and_then(|w| w.name))
}

/// The focused workspace, whole.
pub fn focused_workspace() -> Result<Option<Workspace>> {
    Ok(workspaces()?.into_iter().find(|w| w.is_focused))
}

/// How many windows are on a given workspace.
///
/// Used to decide whether re-picking an already-open project should spawn a
/// terminal or just switch to it.
pub fn window_count(workspace_id: u64) -> Result<usize> {
    let mut socket = Socket::connect().context("niri is not running")?;
    match socket.send(Request::Windows).context("niri request failed")? {
        Ok(Response::Windows(windows)) => Ok(windows
            .iter()
            .filter(|w| w.workspace_id == Some(workspace_id))
            .count()),
        Ok(other) => bail!("unexpected reply to Windows: {other:?}"),
        Err(e) => bail!("niri refused the Windows request: {e}"),
    }
}

fn action(action: Action) -> Result<()> {
    let mut socket = Socket::connect().context("niri is not running")?;
    match socket.send(Request::Action(action)).context("niri request failed")? {
        Ok(_) => Ok(()),
        Err(e) => bail!("niri refused the action: {e}"),
    }
}

pub fn focus_workspace(reference: WorkspaceReferenceArg) -> Result<()> {
    action(Action::FocusWorkspace { reference })
}

/// Name the focused workspace (or a specific one).
pub fn set_workspace_name(name: &str, workspace: Option<WorkspaceReferenceArg>) -> Result<()> {
    action(Action::SetWorkspaceName {
        name: name.to_string(),
        workspace,
    })
}

pub fn spawn(command: Vec<String>) -> Result<()> {
    action(Action::Spawn { command })
}

/// The highest workspace index on a given output.
///
/// `focus-workspace-down` only creates a new workspace when you are already on
/// the last one; otherwise it moves to (and would rename) whatever workspace
/// exists below. So callers jump straight to the last workspace on the current
/// output — niri always keeps an empty one there — before naming it.
pub fn last_workspace_idx(workspaces: &[Workspace], output: &str) -> Option<u8> {
    workspaces
        .iter()
        .filter(|w| w.output.as_deref() == Some(output))
        .map(|w| w.idx)
        .max()
}

/// Find a workspace whose name matches `name`, case-insensitively.
///
/// Case-insensitive because the tag rule already folds case: opening "Alpha"
/// when a workspace named "alpha" exists must not create a second one.
pub fn find_workspace_by_name<'a>(
    workspaces: &'a [Workspace],
    name: &str,
) -> Option<&'a Workspace> {
    workspaces
        .iter()
        .find(|w| w.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(id: u64, idx: u8, name: Option<&str>, output: Option<&str>) -> Workspace {
        Workspace {
            id,
            idx,
            name: name.map(String::from),
            output: output.map(String::from),
            is_urgent: false,
            is_active: false,
            is_focused: false,
            active_window_id: None,
        }
    }

    #[test]
    fn finds_workspace_ignoring_case() {
        let list = vec![ws(1, 1, Some("alpha"), None), ws(2, 2, Some("Beta"), None)];
        assert_eq!(find_workspace_by_name(&list, "ALPHA").unwrap().id, 1);
        assert_eq!(find_workspace_by_name(&list, "beta").unwrap().id, 2);
        assert!(find_workspace_by_name(&list, "gamma").is_none());
    }

    #[test]
    fn unnamed_workspaces_never_match() {
        let list = vec![ws(1, 1, None, None)];
        assert!(find_workspace_by_name(&list, "").is_none());
    }

    #[test]
    fn last_idx_is_scoped_to_the_output() {
        let list = vec![
            ws(1, 1, None, Some("DP-1")),
            ws(2, 5, None, Some("DP-1")),
            ws(3, 9, None, Some("HDMI-1")),
        ];
        assert_eq!(last_workspace_idx(&list, "DP-1"), Some(5));
        assert_eq!(last_workspace_idx(&list, "HDMI-1"), Some(9));
        assert_eq!(last_workspace_idx(&list, "nope"), None);
    }
}
