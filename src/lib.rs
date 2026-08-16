//! wt — workspace-scoped Taskwarrior for niri.
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

use anyhow::Result;

/// The focused workspace's task tag.
///
/// Every entry point starts here, and refuses to run rather than guess: writing
/// an untagged task would be worse than doing nothing, since every list is
/// filtered by tag and an untagged task is invisible to all of them.
pub fn require_workspace_tag() -> Result<String> {
    let name = niri::focused_workspace_name()?.unwrap_or_default();
    anyhow::ensure!(
        !name.is_empty(),
        "This workspace has no name — name it with Mod+Shift+Alt+W first."
    );

    let t = tag::workspace_tag(&name);
    anyhow::ensure!(
        !t.is_empty(),
        "Workspace name '{name}' has no usable tag characters."
    );
    Ok(t)
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
