//! A herdr, a niri, a clock and a notifier that answer from a table: the
//! adapter the session module's tests run against, so every step is checked
//! without a herdr server or a compositor. It models what the module reads:
//! a session running or not, its workspaces, tabs and panes, its agents, the
//! worktrees it has opened, niri's windows, and it logs every call it gets,
//! and every claim's release.

use super::{Agent, Claim, Claude, Created, HerdrError, HerdrResult, Opened, Port, WindowInfo, Workspace};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

/// How a scripted `agent start` goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StartOutcome {
    /// Claude starts and is idle.
    #[default]
    Ok,
    /// Fails with no agent to show for it.
    Fails,
    /// Fails, but the agent is there and `blocked`: Claude asking a start-up
    /// question. `agent wait` then finds it idle.
    FailsBlockedThenIdle,
}

/// One agent the fake session holds.
#[derive(Debug, Clone)]
pub struct FakeAgent {
    /// None for a Claude started by hand, which nobody has named.
    pub name: Option<String>,
    /// The pane it runs in, which `agent get` and `agent rename` also take.
    pub pane: String,
    /// herdr's word for its state: `idle`, `blocked` and so on.
    pub status: String,
}

#[derive(Debug, Default)]
struct State {
    installed: bool,
    running: bool,
    /// `workspace_list` calls still to be refused before a stopped session is
    /// up; 0 with `running` false means it never starts.
    boot_polls: u32,
    workspaces: Vec<Workspace>,
    next_id: u32,
    worktrees: HashMap<PathBuf, Opened>,
    agents: Vec<FakeAgent>,
    start: StartOutcome,
    /// One per confirmed `agent prompt` attempt: None delivered, Some(code)
    /// failed with it. Once they run out, every prompt is delivered.
    prompt_codes: Vec<Option<String>>,
    windows: Vec<WindowInfo>,
    log: Vec<String>,
}

/// The fake port. Build one with [`Fake::running`] or [`Fake::stopped`] and
/// the `with_*` methods, hand a clone to `Session::with_port`, act, then read
/// [`Fake::log`] on the one kept: clones share their state, which is how a
/// test still sees the fake after the Session has taken its box.
#[derive(Debug, Default, Clone)]
pub struct Fake {
    state: Rc<RefCell<State>>,
}

impl Fake {
    /// herdr installed, the session running with these `(id, label)`
    /// workspaces.
    pub fn running(workspaces: &[(&str, &str)]) -> Fake {
        let fake = Fake::default();
        {
            let mut s = fake.state.borrow_mut();
            s.installed = true;
            s.running = true;
            s.workspaces = workspaces
                .iter()
                .map(|(id, label)| Workspace { id: id.to_string(), label: label.to_string() })
                .collect();
        }
        fake
    }

    /// herdr installed, the session stopped: the first `boot_polls`
    /// `workspace_list` calls are refused and the next ones answer (0 means it
    /// never starts), with one workspace `w1` labelled `label`.
    pub fn stopped(boot_polls: u32, label: &str) -> Fake {
        let fake = Fake::default();
        {
            let mut s = fake.state.borrow_mut();
            s.installed = true;
            s.boot_polls = boot_polls;
            s.workspaces = vec![Workspace { id: "w1".into(), label: label.into() }];
        }
        fake
    }

    /// A session whose server comes up with no workspace in it: a fresh
    /// server answering before its client has made one.
    pub fn without_workspaces(self) -> Fake {
        self.state.borrow_mut().workspaces.clear();
        self
    }

    /// No herdr on the machine: every herdr call fails to run at all.
    pub fn not_installed(self) -> Fake {
        self.state.borrow_mut().installed = false;
        self
    }

    /// An agent already live in the session.
    pub fn with_agent(self, name: Option<&str>, pane: &str, status: &str) -> Fake {
        self.state.borrow_mut().agents.push(FakeAgent {
            name: name.map(String::from),
            pane: pane.into(),
            status: status.into(),
        });
        self
    }

    /// A window niri has open.
    pub fn with_window(self, id: u64, app_id: &str, title: &str) -> Fake {
        self.state.borrow_mut().windows.push(WindowInfo {
            id,
            app_id: Some(app_id.into()),
            title: Some(title.into()),
        });
        self
    }

    /// How the next `agent start` goes.
    pub fn with_start(self, outcome: StartOutcome) -> Fake {
        self.state.borrow_mut().start = outcome;
        self
    }

    /// How each confirmed prompt attempt goes, in order.
    pub fn with_prompt_codes(self, codes: &[Option<&str>]) -> Fake {
        self.state.borrow_mut().prompt_codes = codes.iter().map(|c| c.map(String::from)).collect();
        self
    }

    /// A worktree herdr already has open, which `worktree open` finds again.
    pub fn with_worktree(self, path: &str, workspace: &str, pane: &str) -> Fake {
        self.state.borrow_mut().worktrees.insert(
            PathBuf::from(path),
            Opened { workspace: workspace.into(), pane: Some(pane.into()) },
        );
        self
    }

    /// Every call so far, one readable line each.
    pub fn log(&self) -> Vec<String> {
        self.state.borrow().log.clone()
    }

    /// The agents the session holds now.
    pub fn agents(&self) -> Vec<FakeAgent> {
        self.state.borrow().agents.clone()
    }

    fn note(&self, line: String) {
        self.state.borrow_mut().log.push(line);
    }

    fn refuse(code: &str, message: &str) -> HerdrError {
        HerdrError { code: Some(code.into()), message: message.into() }
    }

    /// Not installed is the outer error; a stopped server is herdr's own
    /// refusal, as the process adapter reports them.
    fn check(&self) -> HerdrResult<()> {
        let s = self.state.borrow();
        anyhow::ensure!(s.installed, "could not run `herdr` — is herdr installed?");
        if !s.running {
            return Ok(Err(Self::refuse("server_not_running", "server not running")));
        }
        Ok(Ok(()))
    }

    fn mint(&self, prefix: &str) -> String {
        let mut s = self.state.borrow_mut();
        s.next_id += 1;
        format!("{prefix}{}", s.next_id)
    }
}

/// What the fake's claim holds: the fake itself, to log the release when the
/// claim is dropped, which is the moment the process adapter's lock goes.
struct Release {
    fake: Fake,
    session: String,
}

impl Drop for Release {
    fn drop(&mut self) {
        self.fake.note(format!("release {}", self.session));
    }
}

impl Port for Fake {
    fn workspace_list(&self, session: &str) -> HerdrResult<Vec<Workspace>> {
        self.note(format!("workspace_list {session}"));
        let answer = self.check()?;
        // Counted after the check, so this call is still refused and the
        // session answers from the next one on.
        {
            let mut s = self.state.borrow_mut();
            if !s.running && s.boot_polls > 0 {
                s.boot_polls -= 1;
                if s.boot_polls == 0 {
                    s.running = true;
                }
            }
        }
        if let Err(e) = answer {
            return Ok(Err(e));
        }
        Ok(Ok(self.state.borrow().workspaces.clone()))
    }
    fn workspace_create(&self, session: &str, dir: &Path, label: &str) -> HerdrResult<Created> {
        self.note(format!("workspace_create {session} {} {label}", dir.display()));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        let id = self.mint("w");
        let pane = self.mint("p");
        let tab = self.mint("t");
        self.state.borrow_mut().workspaces.push(Workspace { id, label: label.into() });
        Ok(Ok(Created { pane, tab: Some(tab) }))
    }
    fn tab_create(&self, session: &str, workspace: &str, dir: &Path, label: &str) -> HerdrResult<Created> {
        self.note(format!("tab_create {session} {workspace} {} {label}", dir.display()));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        Ok(Ok(Created { pane: self.mint("p"), tab: Some(self.mint("t")) }))
    }
    fn tab_close(&self, session: &str, tab: &str) -> HerdrResult<()> {
        self.note(format!("tab_close {session} {tab}"));
        self.check()
    }
    fn worktree_open(&self, session: &str, repo: &Path, path: &Path, label: &str) -> HerdrResult<Opened> {
        self.note(format!("worktree_open {session} {} {} {label}", repo.display(), path.display()));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        let opened = self.state.borrow().worktrees.get(path).cloned();
        Ok(Ok(match opened {
            Some(o) => o,
            None => {
                let o = Opened { workspace: self.mint("w"), pane: Some(self.mint("p")) };
                self.state.borrow_mut().worktrees.insert(path.to_path_buf(), o.clone());
                o
            }
        }))
    }
    fn pane_run(&self, session: &str, pane: &str, command: &str) -> HerdrResult<()> {
        self.note(format!("pane_run {session} {pane} {command}"));
        self.check()
    }
    fn agent_get(&self, session: &str, target: &str) -> HerdrResult<Agent> {
        self.note(format!("agent_get {session} {target}"));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        let found = self
            .state
            .borrow()
            .agents
            .iter()
            .find(|a| a.name.as_deref() == Some(target) || a.pane == target)
            .cloned();
        Ok(match found {
            Some(a) => Ok(Agent { name: a.name, status: Some(a.status) }),
            None => Err(Self::refuse("agent_not_found", "agent target not found")),
        })
    }
    fn agent_list(&self, session: &str) -> HerdrResult<Vec<String>> {
        self.note(format!("agent_list {session}"));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        Ok(Ok(self.state.borrow().agents.iter().filter_map(|a| a.name.clone()).collect()))
    }
    fn agent_focus(&self, session: &str, name: &str) -> HerdrResult<()> {
        self.note(format!("agent_focus {session} {name}"));
        self.check()
    }
    fn agent_rename(&self, session: &str, target: &str, name: &str) -> HerdrResult<()> {
        self.note(format!("agent_rename {session} {target} {name}"));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        let mut s = self.state.borrow_mut();
        if s.agents.iter().any(|a| a.name.as_deref() == Some(name)) {
            return Ok(Err(Self::refuse("agent_name_taken", "an agent already has that name")));
        }
        if let Some(a) = s.agents.iter_mut().find(|a| a.pane == target || a.name.as_deref() == Some(target)) {
            a.name = Some(name.into());
        }
        Ok(Ok(()))
    }
    fn agent_start(&self, session: &str, name: &str, pane: &str, claude: &Claude) -> HerdrResult<()> {
        let kind = match claude {
            Claude::Refiner { .. } => "refiner",
            Claude::Worker => "worker",
        };
        self.note(format!("agent_start {session} {name} {pane} {kind}"));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        let outcome = self.state.borrow().start;
        let agent = |status: &str| FakeAgent { name: Some(name.into()), pane: pane.into(), status: status.into() };
        match outcome {
            StartOutcome::Ok => {
                self.state.borrow_mut().agents.push(agent("idle"));
                Ok(Ok(()))
            }
            StartOutcome::Fails => Ok(Err(Self::refuse("agent_start_failed", "claude exited"))),
            StartOutcome::FailsBlockedThenIdle => {
                self.state.borrow_mut().agents.push(agent("blocked"));
                Ok(Err(Self::refuse("timeout", "agent did not become ready")))
            }
        }
    }
    fn agent_wait_ready(&self, session: &str, name: &str) -> HerdrResult<()> {
        self.note(format!("agent_wait_ready {session} {name}"));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        if let Some(a) = self.state.borrow_mut().agents.iter_mut().find(|a| a.name.as_deref() == Some(name)) {
            a.status = "idle".into();
        }
        Ok(Ok(()))
    }
    fn agent_prompt(&self, session: &str, name: &str, text: &str) -> HerdrResult<()> {
        self.note(format!("agent_prompt {session} {name} {text}"));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        let code = {
            let mut s = self.state.borrow_mut();
            if s.prompt_codes.is_empty() { None } else { s.prompt_codes.remove(0) }
        };
        Ok(match code {
            None => Ok(()),
            Some(code) => Err(Self::refuse(&code, "prompt not taken")),
        })
    }
    fn windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        self.note("windows".into());
        Ok(self.state.borrow().windows.clone())
    }
    fn focus_window(&self, id: u64) -> anyhow::Result<()> {
        self.note(format!("focus_window {id}"));
        Ok(())
    }
    fn spawn(&self, command: Vec<String>) -> anyhow::Result<()> {
        self.note(format!("spawn {}", command.join(" ")));
        Ok(())
    }
    fn sleep(&self, d: Duration) {
        self.note(format!("sleep {}ms", d.as_millis()));
    }
    fn notify(&self, text: &str) {
        self.note(format!("notify {text}"));
    }
    fn herdr_installed(&self) -> bool {
        self.state.borrow().installed
    }
    fn claim(&self, session: &str) -> anyhow::Result<Claim> {
        self.note(format!("claim {session}"));
        Ok(Claim::new(Release { fake: self.clone(), session: session.into() }))
    }
}
