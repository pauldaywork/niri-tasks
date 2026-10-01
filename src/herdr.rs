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

/// Every live agent in the session — one call that answers whether a task
/// has a Claude on it, for the menu that opens on every card click.
pub fn agent_list(session: &str) -> Vec<String> {
    cmd(session, &["agent", "list"])
}

/// Return the user to an existing refine tab instead of opening a duplicate.
pub fn agent_focus(session: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "focus", name])
}

/// Give the agent in a pane a name — how a Claude started by hand gets the
/// `work-<uuid8>` name Start working would have given it, so the task's menu
/// can find it. Targets the pane, since an unnamed agent has nothing else.
pub fn agent_rename(session: &str, target: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "rename", target, name])
}

/// The standing instruction a refine session starts with, in Claude's system
/// prompt rather than only in the skill: it carries more weight there, and it
/// survives the conversation being summarised, which a skill's text may not.
/// It steers; the sandbox in `settings` is what enforces.
pub const REFINER_SYSTEM_PROMPT: &str = "You are refining a Taskwarrior task, not doing it. \
Never carry out its steps, however concrete they are — no file edits, no config changes, \
no commands that change anything. Your only write is the task update, and only after the \
user approves it.";

/// Claude as a named agent in `pane`, fenced in to refining a task rather
/// than doing it.
///
/// Not plan mode: approving a plan there means "go and implement it", and a
/// refined task's notes read exactly like a plan — so approval started the
/// task instead of saving it. The skill asks its own question instead, and the
/// file-editing tools are taken away so it cannot start the work either way.
/// `default` is explicit because the user's own default may be auto mode, whose
/// guard against starting the task is a classifier's judgement, not a rule.
///
/// What keeps it from prompting instead is `settings` (see
/// `refine::session_settings`): a Bash sandbox whose commands run unasked while
/// the OS stops them writing anywhere but the task database, plus the few
/// allow and deny rules that go with it.
///
/// The timeout is doubled from herdr's 30s default: a cold Claude Code start
/// with hooks and plugins has been seen near 4s, and a slow disk should not
/// turn that into an error.
pub fn agent_start_claude_refiner(session: &str, name: &str, pane: &str, settings: &str) -> Vec<String> {
    cmd(
        session,
        &[
            "agent", "start", name, "--kind", "claude", "--pane", pane, "--timeout", "60000",
            "--", "--permission-mode", "default",
            "--disallowedTools", "Edit", "Write", "NotebookEdit", "EnterPlanMode", "ExitPlanMode",
            "--append-system-prompt", REFINER_SYSTEM_PROMPT,
            "--settings", settings,
        ],
    )
}

/// Claude as a named agent in `pane`, with no flags of ours: this session is
/// meant to do the work, so it runs in whatever mode the user runs Claude in.
pub fn agent_start_claude(session: &str, name: &str, pane: &str) -> Vec<String> {
    cmd(
        session,
        &["agent", "start", name, "--kind", "claude", "--pane", pane, "--timeout", "60000"],
    )
}

/// Wait for an agent to be ready for a prompt — for the user to answer what
/// it asked while starting, such as Claude's folder-trust question on a new
/// worktree. Ten minutes, since a person is the one being waited on.
pub fn agent_wait_ready(session: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "wait", name, "--until", "idle", "--timeout", "600000"])
}

/// A prompt herdr confirms Claude started on: `--wait` fails with
/// `agent_prompt_stalled` when no activity follows within five seconds — a
/// prompt swallowed by a Claude still finishing its start-up. The timeout only
/// bounds how long we watch it work; running past it means it arrived.
pub fn agent_prompt_confirmed(session: &str, name: &str, text: &str) -> Vec<String> {
    cmd(session, &["agent", "prompt", name, text, "--wait", "--timeout", "15000"])
}

/// herdr's error code out of its stderr JSON — `agent_prompt_stalled`,
/// `timeout` and the like — for callers that act on which error it was.
pub fn error_code(stderr: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(stderr)
        .ok()
        .and_then(|v| v["error"]["code"].as_str().map(str::to_string))
}

/// Like [`run`], but a herdr failure comes back with its code rather than as
/// an error: `Ok(Err((code, message)))`. Only failing to run herdr at all is
/// an `Err`.
pub fn run_coded(argv: &[String]) -> Result<std::result::Result<Value, (Option<String>, String)>> {
    let out = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .with_context(|| format!("could not run `{}` — is herdr installed?", argv[0]))?;
    if !out.status.success() {
        return Ok(Err((error_code(&out.stderr), error_message(&out.stderr))));
    }
    Ok(Ok(serde_json::from_slice(&out.stdout).unwrap_or(Value::Null)))
}

/// An agent's lifecycle state from an `agent get` response: `idle`,
/// `working`, `blocked`, `done` or `unknown`.
pub fn agent_status(got: &Value) -> Option<String> {
    got["result"]["agent"]["agent_status"].as_str().map(str::to_string)
}

/// An agent's name from an `agent get` response, or `None` for one nobody
/// has named — a Claude started by hand rather than by `agent start`.
pub fn agent_name_of(got: &Value) -> Option<String> {
    got["result"]["agent"]["name"].as_str().map(str::to_string)
}

/// The names of the named agents in an `agent list` response. Unnamed ones
/// are left out: a task's agent is found by name.
pub fn agent_names(list: &Value) -> Vec<String> {
    list["result"]["agents"]
        .as_array()
        .map(|agents| agents.iter().filter_map(|a| a["name"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Open an existing git worktree as its own herdr workspace, grouped under
/// the repository's — the same registration the herdr-worktrunk plugin does
/// after worktrunk has made the checkout, so a worktree opened from a task
/// sits in the sidebar exactly like one opened from the plugin.
pub fn worktree_open(session: &str, repo: &Path, path: &Path, label: &str) -> Vec<String> {
    let repo = repo.display().to_string();
    let path = path.display().to_string();
    cmd(
        session,
        &["worktree", "open", "--cwd", &repo, "--path", &path, "--label", label, "--focus", "--json"],
    )
}

/// Type a command into a pane's shell and run it. How a step that needs a
/// terminal — worktrunk asking to approve a repo's hooks — gets one.
pub fn pane_run(session: &str, pane: &str, command: &str) -> Vec<String> {
    cmd(session, &["pane", "run", pane, command])
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

/// The workspace a `worktree open` response opened (or found already open) —
/// read so another tab can be opened in it.
pub fn opened_workspace_id(opened: &Value) -> Option<String> {
    opened["result"]["workspace"]["workspace_id"]
        .as_str()
        .map(str::to_string)
}

/// The tab a `tab create` or `workspace create` response made — read so a tab
/// can be closed again if starting Claude in it fails.
pub fn created_tab_id(created: &Value) -> Option<String> {
    created["result"]["tab"]["tab_id"].as_str().map(str::to_string)
}

/// Every herdr workspace's label in the session, in herdr's order.
///
/// herdr's default `window_title` is `"{hostname}: {workspace}"`, where
/// `{workspace}` is the label of the herdr workspace that client has focused:
/// the project's own workspace, or a task's worktree after Start working.
/// `refine` looks for the outer terminal by any of these labels, because
/// which one the title shows depends on what the user last focused.
pub fn workspace_labels(list: &Value) -> Vec<String> {
    list["result"]["workspaces"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|w| w["label"].as_str().map(str::to_string))
        .collect()
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
            agent_start_claude_refiner("alpha", "task-0123abcd", "w1:p3", "{\"sandbox\":{}}"),
            vec![
                "herdr", "--session", "alpha", "agent", "start", "task-0123abcd",
                "--kind", "claude", "--pane", "w1:p3", "--timeout", "60000",
                "--", "--permission-mode", "default",
                "--disallowedTools", "Edit", "Write", "NotebookEdit", "EnterPlanMode", "ExitPlanMode",
                "--append-system-prompt", REFINER_SYSTEM_PROMPT,
                "--settings", "{\"sandbox\":{}}",
            ]
        );
    }

    #[test]
    fn a_worktree_opens_as_its_own_focused_workspace() {
        assert_eq!(
            worktree_open("alpha", Path::new("/p/alpha"), Path::new("/w/alpha/task-x-1234abcd"), "task/x-1234abcd"),
            vec![
                "herdr", "--session", "alpha", "worktree", "open", "--cwd", "/p/alpha",
                "--path", "/w/alpha/task-x-1234abcd", "--label", "task/x-1234abcd", "--focus", "--json",
            ]
        );
    }

    #[test]
    fn a_working_claude_starts_with_no_extra_flags() {
        assert_eq!(
            agent_start_claude("alpha", "work-1234abcd", "w2:p1"),
            vec![
                "herdr", "--session", "alpha", "agent", "start", "work-1234abcd",
                "--kind", "claude", "--pane", "w2:p1", "--timeout", "60000",
            ]
        );
    }

    /// Waiting on the user to answer whatever Claude asked at startup: ten
    /// minutes, and only `idle` counts as ready.
    #[test]
    fn waiting_for_an_agent_is_until_idle_with_a_long_timeout() {
        assert_eq!(
            agent_wait_ready("alpha", "work-1234abcd"),
            vec!["herdr", "--session", "alpha", "agent", "wait", "work-1234abcd", "--until", "idle", "--timeout", "600000"]
        );
    }

    /// `--wait` makes herdr confirm Claude actually started on the prompt;
    /// the short timeout only bounds how long we watch it work.
    #[test]
    fn a_confirmed_prompt_waits_briefly() {
        assert_eq!(
            agent_prompt_confirmed("alpha", "work-1", "go"),
            vec!["herdr", "--session", "alpha", "agent", "prompt", "work-1", "go", "--wait", "--timeout", "15000"]
        );
    }

    /// herdr puts a machine-readable code beside the message on stderr.
    #[test]
    fn a_herdr_error_code_is_read_from_stderr() {
        let stderr = br#"{"id":"cli:agent:prompt","error":{"code":"agent_prompt_stalled","message":"no activity"}}"#;
        assert_eq!(error_code(stderr).as_deref(), Some("agent_prompt_stalled"));
        assert_eq!(error_code(b"usage: herdr"), None);
    }

    /// Shape from herdr 0.9.1's `agent get`.
    #[test]
    fn an_agents_status_is_read_from_agent_get() {
        let v: Value = serde_json::from_str(r#"{"result":{"agent":{"agent_status":"blocked","name":"work-1"}}}"#).unwrap();
        assert_eq!(agent_status(&v).as_deref(), Some("blocked"));
        assert_eq!(agent_status(&Value::Null), None);
    }

    #[test]
    fn a_command_is_run_in_a_pane_by_id() {
        assert_eq!(
            pane_run("alpha", "w1:p4", "echo hi"),
            vec!["herdr", "--session", "alpha", "pane", "run", "w1:p4", "echo hi"]
        );
    }

    /// Shape from herdr 0.9.1's `worktree open --json`.
    #[test]
    fn an_opened_worktrees_workspace_is_read_from_the_response() {
        let v: Value = serde_json::from_str(
            r#"{"result":{"already_open":false,"root_pane":{"pane_id":"w2:p1"},"workspace":{"workspace_id":"w2"}}}"#,
        )
        .unwrap();
        assert_eq!(opened_workspace_id(&v).as_deref(), Some("w2"));
        assert_eq!(root_pane_id(&v).as_deref(), Some("w2:p1"));
    }

    /// The standing instruction says the three things that matter: refine,
    /// never do; the one write; and only once the user approves.
    #[test]
    fn the_standing_instruction_forbids_doing_the_task() {
        assert!(REFINER_SYSTEM_PROMPT.contains("not doing it"));
        assert!(REFINER_SYSTEM_PROMPT.contains("Never carry out"));
        assert!(REFINER_SYSTEM_PROMPT.contains("only write is the task update"));
        assert!(REFINER_SYSTEM_PROMPT.contains("approves"));
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
    fn workspace_labels_and_tab_id_are_read_from_herdr_responses() {
        let list: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"workspace_id":"w1","label":"niri-tasks","focused":false},{"workspace_id":"wF","label":"task/fix-it-6a970973","focused":true}]}}"#,
        ).unwrap();
        assert_eq!(workspace_labels(&list), vec!["niri-tasks", "task/fix-it-6a970973"]);

        let empty: Value = serde_json::from_str(
            r#"{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[]}}"#,
        ).unwrap();
        assert!(workspace_labels(&empty).is_empty());

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

    /// By pane id, which is what a process inside the pane knows itself by.
    #[test]
    fn an_agent_is_renamed_by_its_pane() {
        assert_eq!(
            agent_rename("alpha", "w1:p2", "work-1234abcd"),
            vec!["herdr", "--session", "alpha", "agent", "rename", "w1:p2", "work-1234abcd"]
        );
    }

    /// Shape from herdr 0.9.1's `agent get`. A Claude started by hand, which
    /// nobody has named, has no `name` at all.
    #[test]
    fn an_agents_name_is_read_from_agent_get() {
        let named: Value =
            serde_json::from_str(r#"{"result":{"agent":{"agent":"claude","name":"work-1","pane_id":"w7:p1"}}}"#).unwrap();
        assert_eq!(agent_name_of(&named).as_deref(), Some("work-1"));
        let unnamed: Value =
            serde_json::from_str(r#"{"result":{"agent":{"agent":"claude","pane_id":"w7:p1"}}}"#).unwrap();
        assert_eq!(agent_name_of(&unnamed), None);
    }

    #[test]
    fn agents_are_listed_per_session() {
        assert_eq!(agent_list("alpha"), vec!["herdr", "--session", "alpha", "agent", "list"]);
    }

    /// Shape from herdr 0.9.1's `agent list`. An unnamed agent is skipped:
    /// nothing can be looked up by a name it does not have.
    #[test]
    fn agent_names_are_read_from_agent_list() {
        let list: Value = serde_json::from_str(
            r#"{"id":"cli:agent:list","result":{"agents":[
                {"agent":"claude","name":"task-6b57114f","pane_id":"w1:pD"},
                {"agent":"claude","pane_id":"w1:p3"},
                {"agent":"claude","name":"work-c53b6e3d","pane_id":"w6:p1"}
            ],"type":"agent_list"}}"#,
        )
        .unwrap();
        assert_eq!(agent_names(&list), vec!["task-6b57114f", "work-c53b6e3d"]);
        assert!(agent_names(&Value::Null).is_empty());
    }
}
