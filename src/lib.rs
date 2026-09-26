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
use std::path::Path;

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
/// Two anchors, both of which were fixed when the terminal opened and do not
/// move with the focus:
///
/// 1. The herdr session. `niritasks project open` names it after the
///    workspace, and herdr hands the name to every pane in it.
/// 2. Failing that, the project folder the shell is in. A terminal opened with
///    `niritasks terminal` starts in `~/Projects/<workspace>`, so the folder
///    names the workspace the same way the session does.
///
/// A named herdr session that matches no workspace is an error rather than a
/// reason to try the folder: it means the workspace was renamed, and the
/// folder would only give a plausible-looking guess.
///
/// The obvious alternative — walk this process's parents to the niri window
/// running it and read *its* workspace — does not survive contact with this
/// setup. herdr breaks the chain (the shell's parent is the herdr *server*,
/// which belongs to no window), and ghostty is one process for every window it
/// draws, so even unbroken the pid identifies the application rather than the
/// terminal you are typing in.
pub fn session_workspace_tag() -> Result<String> {
    let names: Vec<String> = niri::workspaces()?.into_iter().filter_map(|w| w.name).collect();

    let herdr = session::session_from_env(
        std::env::var("HERDR_SESSION").ok().as_deref(),
        std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
    );
    let workspace = match herdr {
        Some(s) => session::workspace_for_session(&s, &names).with_context(|| {
            format!("herdr session '{s}' does not match any named workspace — it may have been renamed since this terminal was opened")
        })?,
        None => {
            let home = std::env::var("HOME").context("HOME is unset")?;
            let cwd = std::env::current_dir().context("cannot read the current directory")?;
            let project = session::project_from_cwd(Path::new(&home), &cwd).context(
                "not in a named herdr session or a ~/Projects folder, so there is no terminal to take the workspace from",
            )?;
            names.iter().find(|n| **n == project).with_context(|| {
                format!("folder ~/Projects/{project} does not match any named workspace")
            })?
        }
    };

    let t = tag::workspace_tag(workspace);
    anyhow::ensure!(
        !t.is_empty(),
        "Workspace name '{workspace}' has no usable tag characters."
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
