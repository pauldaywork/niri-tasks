//! niritasks — workspace-scoped Taskwarrior for niri.
//!
//! The logic lives here rather than in `main.rs` so it can be exercised by
//! integration tests — in particular `tests/differential.rs`, which checks the
//! pure functions against the shell pipelines they were ported from.

pub mod ipc;
pub mod niri;
pub mod notify;
pub mod overlay;
pub mod picker;
pub mod project;
pub mod rows;
pub mod session;
pub mod tag;
pub mod taskbox;
pub mod task;
pub mod text;
pub mod theme;

use anyhow::{Context, Result};

/// The focused workspace's task tag.
///
/// Every entry point starts here, and refuses to run rather than guess: writing
/// an untagged task would be worse than doing nothing, since every list is
/// filtered by tag and an untagged task is invisible to all of them.
pub fn require_workspace_tag() -> Result<String> {
    let name = niri::focused_workspace_name()?.unwrap_or_default();
    anyhow::ensure!(
        !name.is_empty(),
        "This workspace has no name — name it with Mod+Alt+Ctrl+W first."
    );

    let t = tag::workspace_tag(&name);
    anyhow::ensure!(
        !t.is_empty(),
        "Workspace name '{name}' has no usable tag characters."
    );
    Ok(t)
}

/// The task tag for the workspace *this terminal* belongs to, rather than
/// whichever workspace happens to be focused right now.
///
/// `require_workspace_tag` answers "what am I looking at", which is right for a
/// keybind: you press Mod+Alt+T and the task lands where you are. It is wrong
/// for anything long-running, like an agent working a list — switch workspace
/// while it runs and the focused answer moves with you, so work started on one
/// project finishes filing tasks onto another.
///
/// The anchor is the tmux session, because `niritasks tmux-session` names it after the
/// workspace the terminal was opened on and that name does not move.
///
/// The obvious alternative — walk this process's parents to the niri window
/// running it and read *its* workspace — does not survive contact with either
/// half of this setup. tmux breaks the chain (the shell's parent is the tmux
/// *server*, which belongs to no window), and ghostty is one process for every
/// window it draws, so even unbroken the pid identifies the application rather
/// than the terminal you are typing in.
pub fn session_workspace_tag() -> Result<String> {
    let session = tmux_session().context(
        "not inside a tmux session, so there is no terminal to take the workspace from",
    )?;

    let names: Vec<String> = niri::workspaces()?.into_iter().filter_map(|w| w.name).collect();
    let workspace = session::workspace_for_session(&session, &names).with_context(|| {
        format!("tmux session '{session}' does not match any named workspace — it may have been renamed since this terminal was opened")
    })?;

    let t = tag::workspace_tag(workspace);
    anyhow::ensure!(
        !t.is_empty(),
        "Workspace name '{workspace}' has no usable tag characters."
    );
    Ok(t)
}

/// This shell's tmux session name, or `None` outside tmux.
fn tmux_session() -> Option<String> {
    // $TMUX is set inside a session; asking tmux itself then resolves which
    // one, without this having to parse the socket path.
    std::env::var_os("TMUX")?;
    let out = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !name.is_empty()).then_some(name)
}

/// Name workspace 1 if it has no name.
///
/// Run once at startup. Naming matters more than it looks: the name is the tag
/// the task shortcuts scope to, so an unnamed workspace silently has no tasks.
/// Lives here rather than in the binary because the overlay daemon runs it too.
pub fn workspace_default() -> Result<()> {
    let all = niri::workspaces()?;
    if let Some(ws) = all.iter().find(|w| w.idx == 1) {
        if ws.name.is_none() {
            niri::set_workspace_name(
                "general",
                Some(niri_ipc::WorkspaceReferenceArg::Index(1)),
            )?;
        }
    }
    Ok(())
}
