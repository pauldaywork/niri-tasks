//! The herdr CLI, driven from outside any herdr pane.
//!
//! Every call names its session with `--session`, which wins over the
//! `HERDR_*` variables a pane inherits — so this behaves the same from a
//! fuzzel pick as from a terminal inside some other herdr session. The argv
//! builders are pure so they can be tested without a herdr server; [`run`] is
//! the only part that talks to one.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

fn cmd(session: &str, rest: &[&str]) -> Vec<String> {
    ["herdr", "--session", session]
        .iter()
        .chain(rest)
        .map(|s| s.to_string())
        .collect()
}

/// The session's herdr workspaces. Also the liveness check: a stopped session
/// answers `server_not_running`.
pub fn workspace_list(session: &str) -> Vec<String> {
    cmd(session, &["workspace", "list"])
}

/// A first herdr workspace, for a session that has none yet — a fresh server
/// can answer before its client has made one, and `tab create` needs one.
pub fn workspace_create(session: &str, dir: &Path, label: &str) -> Vec<String> {
    let dir = dir.display().to_string();
    cmd(session, &["workspace", "create", "--cwd", &dir, "--label", label, "--focus"])
}

/// Open a tab in the project folder. Takes `--focus` because the user just
/// asked for this tab from a menu and should land in it.
pub fn tab_create(session: &str, workspace_id: &str, dir: &Path, label: &str) -> Vec<String> {
    let dir = dir.display().to_string();
    cmd(
        session,
        &["tab", "create", "--workspace", workspace_id, "--cwd", &dir, "--label", label, "--focus"],
    )
}

/// Check if an agent is already running. Used as the "already being refined?"
/// check — an error means no agent by that name.
pub fn agent_get(session: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "get", name])
}

/// Return the user to an existing refine tab instead of opening a duplicate.
pub fn agent_focus(session: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "focus", name])
}

/// Claude as a named agent in `pane`, fenced in to refining a task rather
/// than doing it.
///
/// Not plan mode: approving a plan there means "go and implement it", and a
/// refined task's notes read exactly like a plan — so approval started the
/// task instead of saving it. The skill asks its own question instead, and the
/// file-editing tools are taken away so it cannot start the work either way.
/// `default` is explicit because the user's own default may be a mode that
/// runs commands unasked; here anything that changes something asks first.
///
/// The timeout is doubled from herdr's 30s default: a cold Claude Code start
/// with hooks and plugins has been seen near 4s, and a slow disk should not
/// turn that into an error.
pub fn agent_start_claude_refiner(session: &str, name: &str, pane: &str) -> Vec<String> {
    cmd(
        session,
        &[
            "agent", "start", name, "--kind", "claude", "--pane", pane, "--timeout", "60000",
            "--", "--permission-mode", "default",
            "--disallowedTools", "Edit", "Write", "NotebookEdit", "EnterPlanMode", "ExitPlanMode",
        ],
    )
}

/// Send a prompt to the agent. Goes by agent name, which herdr validates, so
/// the prompt can't land in the wrong pane.
pub fn agent_prompt(session: &str, name: &str, text: &str) -> Vec<String> {
    cmd(session, &["agent", "prompt", name, text])
}

/// Run one herdr command and return its JSON.
///
/// herdr writes server errors as JSON on stderr with exit status 1, so a
/// failure carries herdr's own message rather than a bare status. A success
/// with no JSON on stdout is `Null`, not an error: only the callers that read
/// ids need a body, and they say so when it is missing.
pub fn run(argv: &[String]) -> Result<Value> {
    let out = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .with_context(|| format!("could not run `{}` — is herdr installed?", argv[0]))?;
    if !out.status.success() {
        bail!("{}", error_message(&out.stderr));
    }
    Ok(serde_json::from_slice(&out.stdout).unwrap_or(Value::Null))
}

/// herdr's error message out of its stderr, or the stderr itself when it is
/// not herdr's JSON (a usage error, say).
pub fn error_message(stderr: &[u8]) -> String {
    serde_json::from_slice::<Value>(stderr)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| String::from_utf8_lossy(stderr).trim().to_string())
}

/// The first herdr workspace in a `workspace list` response. A project's
/// session has one, named after the project.
pub fn first_workspace_id(list: &Value) -> Option<String> {
    list["result"]["workspaces"].get(0)?["workspace_id"]
        .as_str()
        .map(str::to_string)
}

/// The new pane in a `tab create` or `workspace create` response.
pub fn root_pane_id(created: &Value) -> Option<String> {
    created["result"]["root_pane"]["pane_id"]
        .as_str()
        .map(str::to_string)
}

/// The tab a `tab create` or `workspace create` response made — read so a tab
/// can be closed again if starting Claude in it fails.
pub fn created_tab_id(created: &Value) -> Option<String> {
    created["result"]["tab"]["tab_id"].as_str().map(str::to_string)
}

/// The session's first herdr workspace's label.
///
/// A project session names its one workspace after the project, and this is
/// also the string herdr's default `window_title` puts after `": "` in the
/// outer terminal's title — which is how `refine` tells whether that terminal
/// is still open.
pub fn first_workspace_label(list: &Value) -> Option<String> {
    list["result"]["workspaces"].get(0)?["label"].as_str().map(str::to_string)
}

/// Close a tab. Used, best-effort, to clean up a bare-shell tab left behind
/// when `agent start` fails in it — so a retry does not pile another one up
/// beside it.
pub fn tab_close(session: &str, tab_id: &str) -> Vec<String> {
    cmd(session, &["tab", "close", tab_id])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn every_call_names_its_session() {
        assert_eq!(workspace_list("alpha"), vec!["herdr", "--session", "alpha", "workspace", "list"]);
    }

    #[test]
    fn a_tab_opens_in_the_project_folder_and_takes_focus() {
        assert_eq!(
            tab_create("alpha", "w1", Path::new("/home/x/Projects/alpha"), "Refine: fix it"),
            vec![
                "herdr", "--session", "alpha", "tab", "create", "--workspace", "w1",
                "--cwd", "/home/x/Projects/alpha", "--label", "Refine: fix it", "--focus",
            ]
        );
    }

    #[test]
    fn a_workspace_is_created_when_the_session_has_none() {
        assert_eq!(
            workspace_create("alpha", Path::new("/p"), "Refine: x"),
            vec!["herdr", "--session", "alpha", "workspace", "create", "--cwd", "/p", "--label", "Refine: x", "--focus"]
        );
    }

    /// Claude's own arguments go after `--`, which is how herdr tells them
    /// from its own. No plan mode, and no tools that edit files: see
    /// [`agent_start_claude_refiner`] for why.
    #[test]
    fn claude_starts_unable_to_edit_files_or_plan() {
        assert_eq!(
            agent_start_claude_refiner("alpha", "task-0123abcd", "w1:p3"),
            vec![
                "herdr", "--session", "alpha", "agent", "start", "task-0123abcd",
                "--kind", "claude", "--pane", "w1:p3", "--timeout", "60000",
                "--", "--permission-mode", "default",
                "--disallowedTools", "Edit", "Write", "NotebookEdit", "EnterPlanMode", "ExitPlanMode",
            ]
        );
    }

    #[test]
    fn agent_lookups_and_prompts_go_by_name() {
        assert_eq!(agent_get("a", "task-1"), vec!["herdr", "--session", "a", "agent", "get", "task-1"]);
        assert_eq!(agent_focus("a", "task-1"), vec!["herdr", "--session", "a", "agent", "focus", "task-1"]);
        assert_eq!(
            agent_prompt("a", "task-1", "/refine-task u"),
            vec!["herdr", "--session", "a", "agent", "prompt", "task-1", "/refine-task u"]
        );
    }

    /// Shapes copied from herdr 0.9.1's real responses.
    #[test]
    fn ids_are_read_from_herdr_responses() {
        let list: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"workspace_id":"w1","label":"hansard"}]}}"#,
        ).unwrap();
        assert_eq!(first_workspace_id(&list).as_deref(), Some("w1"));

        let empty: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[]}}"#,
        ).unwrap();
        assert_eq!(first_workspace_id(&empty), None);

        let created: Value = serde_json::from_str(
            r#"{"id":"cli:tab:create","result":{"root_pane":{"pane_id":"w1:p2","tab_id":"w1:t2"},"tab":{"tab_id":"w1:t2"},"type":"tab_created"}}"#,
        ).unwrap();
        assert_eq!(root_pane_id(&created).as_deref(), Some("w1:p2"));
    }

    #[test]
    fn a_tab_is_closed_by_id() {
        assert_eq!(
            tab_close("alpha", "w1:t2"),
            vec!["herdr", "--session", "alpha", "tab", "close", "w1:t2"]
        );
    }

    /// Shapes copied from herdr 0.9.1's real responses, like `ids_are_read_…` above.
    #[test]
    fn workspace_label_and_tab_id_are_read_from_herdr_responses() {
        let list: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"workspace_id":"w1","label":"hansard"}]}}"#,
        ).unwrap();
        assert_eq!(first_workspace_label(&list).as_deref(), Some("hansard"));

        let empty: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[]}}"#,
        ).unwrap();
        assert_eq!(first_workspace_label(&empty), None);

        let created: Value = serde_json::from_str(
            r#"{"id":"cli:tab:create","result":{"root_pane":{"pane_id":"w1:p2","tab_id":"w1:t2"},"tab":{"tab_id":"w1:t2"},"type":"tab_created"}}"#,
        ).unwrap();
        assert_eq!(created_tab_id(&created).as_deref(), Some("w1:t2"));
    }

    #[test]
    fn server_errors_are_read_from_the_json_herdr_writes_to_stderr() {
        let stderr = br#"{"id":"cli:agent:get","error":{"code":"agent_not_found","message":"agent target task-nope not found"}}"#;
        assert_eq!(error_message(stderr), "agent target task-nope not found");
        assert_eq!(error_message(b"usage: herdr ...\n"), "usage: herdr ...", "non-JSON passes through");
    }
}
