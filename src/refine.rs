//! Handing a task to Claude in its workspace's herdr session, to be worked up
//! into a plan by the `refine-task` skill.
//!
//! Only the task's uuid crosses over: the skill reads the task itself, so
//! nothing needs quoting through two CLIs and it always sees the current
//! version rather than the one the menu was opened on.

use crate::{herdr, niri, project, session, text};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Draft straight away; ask only what the code cannot answer.
    Quick,
    /// A full `/grill-me` interview before the draft.
    Grill,
}

/// How long a just-opened project terminal gets to bring its herdr session up.
const SESSION_WAIT: Duration = Duration::from_secs(10);
const SESSION_POLL: Duration = Duration::from_millis(250);

/// Longest description, in characters, a tab label carries before eliding.
const LABEL_DESCRIPTION_MAX: usize = 30;

/// The herdr agent name for a task's refine session. One name per task is
/// what lets a second Refine find the first instead of opening another.
pub fn agent_name(uuid: &str) -> String {
    let short: String = uuid.chars().take(8).collect();
    format!("task-{}", short.to_ascii_lowercase())
}

/// Only the uuid crosses to the skill — the description and everything else
/// about the task is read by the skill itself, so nothing here needs quoting
/// through two CLIs (this one, then herdr's `agent prompt`).
pub fn prompt(uuid: &str, mode: Mode) -> String {
    match mode {
        Mode::Quick => format!("/refine-task {uuid}"),
        Mode::Grill => format!("/refine-task {uuid} grill"),
    }
}

/// The verb says which mode the tab is in; the description is cut short
/// because the label shares herdr's sidebar with every other tab's label.
pub fn tab_label(mode: Mode, description: &str) -> String {
    let verb = match mode {
        Mode::Quick => "Refine",
        Mode::Grill => "Grill",
    };
    let d = text::collapse_whitespace(description);
    let short = if d.chars().count() > LABEL_DESCRIPTION_MAX {
        let cut: String = d.chars().take(LABEL_DESCRIPTION_MAX - 1).collect();
        format!("{cut}…")
    } else {
        d
    };
    format!("{verb}: {short}")
}

/// Open Claude on a task in `workspace`'s herdr session.
///
/// Opens the project terminal first if the session is not running, and goes
/// back to the task's existing tab if it is already being refined.
pub fn launch(workspace: &str, uuid: &str, description: &str, mode: Mode) -> Result<()> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let dir = session::start_dir(Path::new(&home), workspace);
    let s = session::herdr_session_name(workspace);
    let name = agent_name(uuid);

    let list = match herdr::run(&herdr::workspace_list(&s)) {
        Ok(list) => list,
        Err(_) => {
            anyhow::ensure!(
                project::on_path(project::SESSION_MANAGER),
                "herdr is not installed, so there is no session to refine in."
            );
            // Through niri, so the window lands on the focused workspace —
            // the one the task belongs to — as the project picker's does.
            niri::spawn(project::project_terminal_command(&dir, &s, true))?;
            wait_for_session(&s)?
        }
    };

    if herdr::run(&herdr::agent_get(&s, &name)).is_ok() {
        herdr::run(&herdr::agent_focus(&s, &name))?;
        return Ok(());
    }

    let label = tab_label(mode, description);
    let created = match herdr::first_workspace_id(&list) {
        Some(id) => herdr::run(&herdr::tab_create(&s, &id, &dir, &label))?,
        None => herdr::run(&herdr::workspace_create(&s, &dir, &label))?,
    };
    let pane = herdr::root_pane_id(&created).context("herdr did not say which pane it made")?;

    herdr::run(&herdr::agent_start_claude_plan(&s, &name, &pane))?;
    herdr::run(&herdr::agent_prompt(&s, &name, &prompt(uuid, mode)))?;
    Ok(())
}

/// Poll until the session answers with a workspace in it. A server that
/// answers but never gets one is returned as it is at the deadline — `launch`
/// then creates the workspace itself.
fn wait_for_session(s: &str) -> Result<Value> {
    let deadline = Instant::now() + SESSION_WAIT;
    let mut answered = None;
    loop {
        if let Ok(list) = herdr::run(&herdr::workspace_list(s)) {
            if herdr::first_workspace_id(&list).is_some() {
                return Ok(list);
            }
            answered = Some(list);
        }
        if Instant::now() >= deadline {
            return match answered {
                Some(list) => Ok(list),
                None => bail!("herdr session {s} did not start within {}s.", SESSION_WAIT.as_secs()),
            };
        }
        std::thread::sleep(SESSION_POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// herdr names must match `[a-z][a-z0-9_-]{0,31}`; a uuid's first eight
    /// characters are hex, and enough to tell one task's session from another.
    #[test]
    fn the_agent_is_named_after_its_task() {
        assert_eq!(agent_name("00DEEEE1-3cbd-465d-8c85-c4c4d643b1d0"), "task-00deeee1");
    }

    #[test]
    fn the_prompt_invokes_the_skill_with_the_uuid() {
        assert_eq!(prompt("u-1", Mode::Quick), "/refine-task u-1");
        assert_eq!(prompt("u-1", Mode::Grill), "/refine-task u-1 grill");
    }

    #[test]
    fn the_tab_says_what_it_is_for() {
        assert_eq!(tab_label(Mode::Quick, "fix the peek"), "Refine: fix the peek");
        assert_eq!(tab_label(Mode::Grill, "fix the peek"), "Grill: fix the peek");
    }

    /// A tab label shares herdr's sidebar with every other tab; a long
    /// description is cut, by characters, with an ellipsis.
    #[test]
    fn long_descriptions_are_elided_in_the_label() {
        let label = tab_label(Mode::Quick, &"é".repeat(40));
        assert_eq!(label, format!("Refine: {}…", "é".repeat(29)));
        assert_eq!(tab_label(Mode::Quick, "two\n lines"), "Refine: two lines");
    }
}
