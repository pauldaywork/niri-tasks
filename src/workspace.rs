//! Which workspace a command is about, decided once, as a value that always
//! has a usable tag: the focused one for a keybind, this terminal's herdr
//! session (or its ~/Projects folder) for an agent, and whichever is asking
//! for the commands both run. Each command decides once, and every caller
//! gets the name, tag, herdr session and folder from that one value.

use crate::session::{self, Session};
use crate::{dirs::Dirs, niri, tag};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// A named niri workspace whose name folds to a usable task tag. Built by one
/// of the three policies, or [`Workspace::named`] for a name a flag carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    name: String,
    tag: String,
}

impl Workspace {
    /// `name`, refused unnamed or with nothing a tag keeps.
    ///
    /// Every entry point refuses rather than guesses: writing an untagged task
    /// would be worse than doing nothing, since every list is filtered by tag
    /// and an untagged task is invisible to all of them.
    pub fn named(name: &str) -> Result<Workspace> {
        anyhow::ensure!(
            !name.is_empty(),
            "This workspace has no name — Mod+Alt+W opens a project on a named one."
        );
        let tag = tag::workspace_tag(name);
        anyhow::ensure!(
            !tag.is_empty(),
            "Workspace name '{name}' has no usable tag characters."
        );
        Ok(Workspace { name: name.to_string(), tag })
    }

    /// The focused workspace: what a keybind and the panel act on.
    pub fn focused() -> Result<Workspace> {
        Workspace::named(&niri::focused_workspace_name()?.unwrap_or_default())
    }

    /// The workspace *this terminal* belongs to, rather than whichever
    /// workspace happens to be focused right now.
    ///
    /// [`Workspace::focused`] answers "what am I looking at", which is right
    /// for a keybind: you press Mod+Alt+T and the task lands where you are. It
    /// is wrong for anything long-running, like an agent working a list —
    /// switch workspace while it runs and the focused answer moves with you, so
    /// work started on one project finishes filing tasks onto another.
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
    /// reason to try the folder; `from_session` says why.
    ///
    /// The obvious alternative — walk this process's parents to the niri window
    /// running it and read *its* workspace — does not survive contact with this
    /// setup. herdr breaks the chain (the shell's parent is the herdr *server*,
    /// which belongs to no window), and ghostty is one process for every window it
    /// draws, so even unbroken the pid identifies the application rather than the
    /// terminal you are typing in.
    pub fn of_session() -> Result<Workspace> {
        let names: Vec<String> = niri::workspaces()?.into_iter().filter_map(|w| w.name).collect();
        // The named herdr session this process runs in, from what herdr hands
        // its panes; none outside herdr, or in its unnamed default session.
        let name = match session::current_pane().map(|p| p.session) {
            Some(s) => from_session(&s, &names)?,
            None => {
                let dirs = Dirs::from_env()?;
                let cwd = std::env::current_dir().context("cannot read the current directory")?;
                from_folder(&dirs, &cwd, &names)?
            }
        };
        Workspace::named(&name)
    }

    /// The workspace whoever is asking belongs to: the herdr session's from a
    /// pane in a named herdr session, where agents and terminals run, and the
    /// focused one from anywhere else, which is how keybinds and the task panel
    /// run.
    ///
    /// A session that matches no workspace is an error, as in
    /// [`Workspace::of_session`], rather than a reason to fall back to focus:
    /// the focus is exactly the answer that files an agent's task, or opens its
    /// session, on the wrong workspace.
    pub fn of_caller() -> Result<Workspace> {
        match session::current_pane() {
            Some(_) => Workspace::of_session(),
            None => Workspace::focused(),
        }
    }

    /// The workspace's name, raw, as niri has it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The task tag its name folds to, never empty.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The herdr session its project terminal attaches to.
    pub fn session_name(&self) -> String {
        session::herdr_session_name(&self.name)
    }

    /// Where its terminals start (see [`Dirs::start_dir`]).
    pub fn folder(&self, dirs: &Dirs) -> PathBuf {
        dirs.start_dir(&self.name)
    }

    /// The workspace's herdr session, in the folder its terminals start in.
    pub fn session(&self) -> Result<Session> {
        Ok(Session::at(self.session_name(), self.folder(&Dirs::from_env()?)))
    }
}

/// The workspace a named herdr session was opened for, among `names`. A
/// session that matches none is an error, not a reason to try the folder: the
/// workspace was renamed, and the folder would only give a plausible guess.
fn from_session(session: &str, names: &[String]) -> Result<String> {
    session::workspace_for_session(session, names).cloned().with_context(|| {
        format!("herdr session '{session}' does not match any named workspace — it may have been renamed since this terminal was opened")
    })
}

/// The workspace named by the `~/Projects` folder `cwd` is in, among `names`.
fn from_folder(dirs: &Dirs, cwd: &Path, names: &[String]) -> Result<String> {
    let project = dirs.project_from_cwd(cwd).context(
        "not in a named herdr session or a ~/Projects folder, so there is no terminal to take the workspace from",
    )?;
    names.iter().find(|n| **n == project).cloned().with_context(|| {
        format!("folder ~/Projects/{project} does not match any named workspace")
    })
}

/// Name workspace 1 if it has no name.
///
/// Run once at startup. Naming matters more than it looks: the name is the tag
/// the task shortcuts scope to, so an unnamed workspace silently has no tasks.
/// The daemon runs it once as it starts.
pub fn name_default() -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn names(of: &[&str]) -> Vec<String> {
        of.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn named_refuses_an_empty_name_and_a_tagless_one() {
        let e = Workspace::named("").unwrap_err();
        assert_eq!(
            e.to_string(),
            "This workspace has no name — Mod+Alt+W opens a project on a named one."
        );
        let e = Workspace::named("!!!").unwrap_err();
        assert_eq!(e.to_string(), "Workspace name '!!!' has no usable tag characters.");

        let ws = Workspace::named("my project").unwrap();
        assert_eq!(ws.name(), "my project");
        assert_eq!(ws.tag(), "my_project");
        assert_eq!(ws.session_name(), "my_project");
    }

    #[test]
    fn a_session_names_its_workspace_or_says_it_was_renamed() {
        let all = names(&["alpha", "my project"]);
        // The session name is the lossy one herdr was given; matching goes
        // through the same rule, so the raw name comes back.
        let session = session::herdr_session_name("my project");
        assert_eq!(from_session(&session, &all).unwrap(), "my project");
        assert_eq!(from_session("alpha", &all).unwrap(), "alpha");

        let e = from_session("gone", &all).unwrap_err();
        assert_eq!(
            e.to_string(),
            "herdr session 'gone' does not match any named workspace — it may have been renamed since this terminal was opened"
        );
    }

    #[test]
    fn a_projects_folder_names_its_workspace_or_says_which_is_missing() {
        let dirs = Dirs::at(Path::new("/home/x"));
        let all = names(&["alpha"]);

        let cwd = Path::new("/home/x/Projects/alpha/src");
        assert_eq!(from_folder(&dirs, cwd, &all).unwrap(), "alpha");

        let e = from_folder(&dirs, Path::new("/home/x/Projects/beta"), &all).unwrap_err();
        assert_eq!(e.to_string(), "folder ~/Projects/beta does not match any named workspace");

        let e = from_folder(&dirs, Path::new("/elsewhere"), &all).unwrap_err();
        assert_eq!(
            e.to_string(),
            "not in a named herdr session or a ~/Projects folder, so there is no terminal to take the workspace from"
        );
    }
}
