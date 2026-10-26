//! The status line a task's Claude shows under its prompt:
//! `#61 · <description> · Active · task/<slug>-<uuid8>`. Refine and Start
//! working pass it in their launch `--settings`, so the user's own Claude
//! settings are never touched. Claude Code runs `niritasks task status-line
//! <uuid>` on each refresh, so the number, description and state are read
//! again every time and never go stale.

use crate::{names, task, text, work};
use serde_json::{json, Value};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

/// The task's state in the Update status menu's words (`task::Status`), and
/// `Pending` for a task not started, Taskwarrior's own word: the menu's
/// `Stopped` would say it had been started.
pub fn state_word(t: &task::Task, now: i64) -> &'static str {
    let parked = t.wait.as_deref().and_then(task::stamp_secs).is_some_and(|w| w > now);
    match t.status.as_str() {
        "completed" => "Completed",
        "deleted" => "Deleted",
        _ if parked => "Waiting",
        _ if t.is_active() => "Active",
        _ => "Pending",
    }
}

/// `#id · description · state · branch`, leaving out a number a finished
/// task has lost to gc and a branch git did not name.
pub fn line(t: &task::Task, now: i64, branch: Option<&str>) -> String {
    let mut parts = Vec::new();
    if t.id != 0 {
        parts.push(format!("#{}", t.id));
    }
    parts.push(text::collapse_whitespace(&t.description));
    parts.push(state_word(t, now).to_string());
    if let Some(b) = branch.filter(|b| !b.is_empty()) {
        parts.push(b.to_string());
    }
    parts.join(" · ")
}

/// The folder Claude is working in, from the JSON Claude Code hands a
/// status-line command on stdin: `workspace.current_dir`, else `cwd`.
pub fn cwd_from(input: &str) -> Option<PathBuf> {
    let v: Value = serde_json::from_str(input).ok()?;
    v["workspace"]["current_dir"].as_str().or(v["cwd"].as_str()).map(PathBuf::from)
}

/// Print `uuid`'s status line. Never fails and never notifies: Claude Code
/// runs it on every refresh, so a task it cannot read prints a line saying
/// so instead. Reads Claude Code's JSON from stdin unless that is a
/// terminal, so it can be run by hand too.
pub fn run(uuid: &str) {
    let mut input = String::new();
    if !std::io::stdin().is_terminal() {
        let _ = std::io::stdin().read_to_string(&mut input);
    }
    let dir = cwd_from(&input).or_else(|| std::env::current_dir().ok());
    let branch = dir.as_deref().and_then(work::branch_in);
    let mut out = std::io::stdout().lock();
    match task::get(uuid) {
        Ok(Some(t)) => {
            let _ = writeln!(out, "{}", line(&t, task::now_secs(), branch.as_deref()));
        }
        Ok(None) => {
            let _ = writeln!(out, "task {}: not found", names::uuid8(uuid));
        }
        Err(_) => {
            let _ = writeln!(out, "task {}: could not read it", names::uuid8(uuid));
        }
    }
}

/// The shell command Claude Code runs for `uuid`'s status line, with `exe`
/// the `niritasks` binary: absolute, since a Claude's `$PATH` need not have it.
pub fn command(exe: &Path, uuid: &str) -> String {
    format!("{} task status-line {}", work::sh_quote(&exe.display().to_string()), work::sh_quote(uuid))
}

/// Claude Code's `statusLine` setting running `command`.
pub fn setting(command: &str) -> Value {
    json!({ "type": "command", "command": command, "padding": 0 })
}

/// The `--settings` JSON a working Claude starts with: its status line and
/// nothing else, so it otherwise runs as the user runs Claude.
pub fn worker_settings(command: &str) -> String {
    json!({ "statusLine": setting(command) }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(v: serde_json::Value) -> task::Task {
        serde_json::from_value(v).unwrap()
    }

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn the_line_is_number_description_state_and_branch() {
        let t = task(serde_json::json!({"id": 61, "uuid": "u", "description": "feat: Name  the task", "status": "pending", "start": "20261012T000000Z"}));
        assert_eq!(line(&t, NOW, Some("task/feat-x-7b4f4c5c")), "#61 · feat: Name the task · Active · task/feat-x-7b4f4c5c");
    }

    #[test]
    fn no_branch_and_no_number_leave_their_place_out() {
        let t = task(serde_json::json!({"id": 0, "uuid": "u", "description": "x", "status": "completed"}));
        assert_eq!(line(&t, NOW, None), "x · Completed");
        assert_eq!(line(&t, NOW, Some("")), "x · Completed");
    }

    /// The Update status menu's words; a task never started is Pending,
    /// Taskwarrior's own, not the menu's Stopped.
    #[test]
    fn the_state_reads_as_the_status_menu_does() {
        let state = |v: serde_json::Value| state_word(&task(v), NOW);
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending"})), "Pending");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending", "start": "20261012T000000Z"})), "Active");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending", "wait": "99991229T130000Z"})), "Waiting");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "pending", "wait": "20001229T130000Z"})), "Pending", "a wait gone by");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "completed"})), "Completed");
        assert_eq!(state(serde_json::json!({"uuid": "u", "description": "x", "status": "deleted"})), "Deleted");
    }

    /// Claude Code's status-line input: the workspace's folder first, then
    /// the session's cwd.
    #[test]
    fn the_folder_comes_from_claude_codes_input() {
        assert_eq!(cwd_from(r#"{"cwd":"/a","workspace":{"current_dir":"/b"}}"#), Some(PathBuf::from("/b")));
        assert_eq!(cwd_from(r#"{"cwd":"/a"}"#), Some(PathBuf::from("/a")));
        assert_eq!(cwd_from("not json"), None);
    }

    #[test]
    fn the_command_quotes_the_binary_and_the_uuid() {
        assert_eq!(
            command(Path::new("/home/x/.cargo/bin/niritasks"), "7b4f4c5c-185a"),
            "'/home/x/.cargo/bin/niritasks' task status-line '7b4f4c5c-185a'"
        );
    }

    #[test]
    fn the_worker_settings_hold_the_status_line_alone() {
        let v: serde_json::Value = serde_json::from_str(&worker_settings("niritasks task status-line u")).unwrap();
        assert_eq!(v, serde_json::json!({"statusLine": {"type": "command", "command": "niritasks task status-line u", "padding": 0}}));
    }
}
