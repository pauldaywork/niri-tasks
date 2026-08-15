//! wt — workspace-scoped Taskwarrior for niri.
//!
//! The logic lives here rather than in `main.rs` so it can be exercised by
//! integration tests — in particular `tests/differential.rs`, which checks the
//! pure functions against the shell pipelines they were ported from.

pub mod niri;
pub mod notify;
pub mod picker;
pub mod project;
pub mod rows;
pub mod session;
pub mod tag;
pub mod task;
pub mod text;

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
