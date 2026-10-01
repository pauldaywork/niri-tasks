# Go to a Task's Session — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** From a task's menu, get back to the Claude working on it in herdr — including a Claude started by hand, which gets linked to the task when it marks the task active.

**Architecture:** No data is stored on the task; the link is the herdr agent's *name*. `niritasks task status <uuid> active`, run inside a herdr pane, renames that pane's agent to `work-<uuid8>` (unless it is already a `task-`/`work-` agent). The task menu asks the workspace's herdr session for a live `work-<uuid8>` or `task-<uuid8>` agent, and when there is one it leads with **Go to session**, which raises the project terminal and `herdr agent focus`es that agent. The new logic lives in a new `src/link.rs`; herdr argv builders and parsers go in `src/herdr.rs` alongside the existing ones.

**Tech Stack:** Rust (anyhow, clap, serde_json), herdr 0.9.1, Taskwarrior 2.6, niri, fuzzel.

**Spec:** Taskwarrior task `c70a474a-a6f9-440a-9cbc-5d36cab08daa` — read it with `task rc.json.array=on c70a474a-a6f9-440a-9cbc-5d36cab08daa export`. Its annotations (Goal, Context, Decided ×3, Steps, Done when, Out of scope) are the spec.

## Global Constraints

- Link by agent name, never by stored data: no UDAs, no herdr ids on the task.
- The rename happens only when `HERDR_PANE_ID` and a herdr session (`HERDR_SESSION`, or `HERDR_SOCKET_PATH` as backup — `session::session_from_env`) are both present.
- The name given is exactly `work::work_agent_name(uuid)` → `work-<uuid8>`.
- A pane whose agent is already named `task-…` or `work-…` is left alone.
- The rename is best effort: the status change succeeds even if the rename fails; the failure goes to stderr.
- **Go to session** is the menu's *first* entry, shown only when the workspace's herdr session has a live `work-<uuid8>` or `task-<uuid8>` agent; `work-` wins when both exist.
- **Go to session** raises the project terminal (`refine::open_session`) and then `herdr agent focus`es the agent. It never starts anything.
- A task with no live agent shows exactly today's menu.
- The panel's action row (Task 3, added at the user's request) gets a **Go to session** button on cards with a live agent, decided by one `herdr agent list` when the panel takes the keyboard — never per focus move, never on the tucked-away refresh.
- Out of scope: clicking a card to skip the menu; reviving a session whose agent has exited.
- Every public item gets a doc comment that says *why*; errors surface by returning `Err` (main already notifies). Match the surrounding code's comment density and naming.
- Commit messages: a plain sentence in the imperative, no `feat:` prefix, ending with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Facts about herdr 0.9.1 this plan relies on (verified)

- Agent commands take a live agent name **or a pane id** (`w1:p2`) as their target. Names must match `[a-z][a-z0-9_-]{0,31}` and be unique among live agents; a name is cleared when its agent exits.
- `herdr --session S agent rename <TARGET> <NAME>` renames.
- `herdr --session S agent get w1:p2` on a pane with an agent:
  `{"id":"cli:agent:get","result":{"agent":{"agent":"claude","name":"work-c70a474a","pane_id":"w7:p1",…},"type":"agent_info"}}`
  — an agent nobody has named has no `name` key. On a pane with no agent it exits 1 with
  `{"error":{"code":"agent_not_found","message":"agent target w1:p1 not found"},…}` on stderr.
- `herdr --session S agent list`:
  `{"id":"cli:agent:list","result":{"agents":[{"name":"task-6b57114f","pane_id":"w1:pD",…},{"name":"work-c53b6e3d","pane_id":"w6:p1",…}],"type":"agent_list"}}`
- With no server running, `agent list` exits 1 with `server_not_running` in about a millisecond, so asking on every menu open costs nothing noticeable.

## File Structure

- **Create `src/link.rs`** — the task ↔ herdr-agent link: what name a pane's agent should get, linking the current pane, finding a task's live agent, and going to it.
- **Modify `src/herdr.rs`** — argv builders `agent_rename`, `agent_list`; parsers `agent_name_of`, `agent_names`; their tests.
- **Modify `src/lib.rs`** — `pub mod link;`.
- **Modify `src/main.rs`** — `apply_status` links the pane on Active; new `task session <uuid>` subcommand; `task_menu` builds its entries with `menu_entries` and dispatches **Go to session**.
- **Create `tests/link_pane.rs`** — runs the real binary against a fake `herdr` on `PATH` and a sandboxed task database.
- **Modify `README.md`** — the commands list, the panel paragraph that lists the menu's entries, the herdr paragraph, and the panel's button table.
- **Modify `src/panel/actions.rs`, `keys.rs`, `style.rs`, `surface.rs`, `src/daemon.rs`, `CONTEXT.md`** (Task 3) — the Go to session button.

---

### Task 1: Name a hand-started Claude after the task it marks active

**Files:**
- Modify: `src/herdr.rs` (add after `agent_focus`, ~line 55, and after `agent_status`, ~line 145; tests at the end of `mod tests`)
- Create: `src/link.rs`
- Modify: `src/lib.rs:7-23` (module list)
- Modify: `src/main.rs:12-15` (imports), `src/main.rs:452-459` (`apply_status`)
- Create: `tests/link_pane.rs`
- Modify: `README.md` (herdr paragraph, ~line 99–110)

**Interfaces:**
- Consumes: `work::work_agent_name(uuid: &str) -> String`; `session::session_from_env(Option<&str>, Option<&str>) -> Option<String>`; `herdr::{run, agent_get}`.
- Produces:
  - `herdr::agent_rename(session: &str, target: &str, name: &str) -> Vec<String>`
  - `herdr::agent_name_of(got: &serde_json::Value) -> Option<String>` (reads an `agent get` response)
  - `link::pane_link_name(current: Option<&str>, uuid: &str) -> Option<String>`
  - `link::link_current_pane(uuid: &str) -> anyhow::Result<()>`

- [ ] **Step 1: Write the failing herdr tests**

Append inside `mod tests` in `src/herdr.rs`:

```rust
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
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib herdr::tests`
Expected: compile error — `cannot find function agent_rename` / `agent_name_of`.

- [ ] **Step 3: Add the two herdr functions**

In `src/herdr.rs`, directly after `agent_focus`:

```rust
/// Give the agent in a pane a name — how a Claude started by hand gets the
/// `work-<uuid8>` name Start working would have given it, so the task's menu
/// can find it. Targets the pane, since an unnamed agent has nothing else.
pub fn agent_rename(session: &str, target: &str, name: &str) -> Vec<String> {
    cmd(session, &["agent", "rename", target, name])
}
```

Directly after `agent_status`:

```rust
/// An agent's name from an `agent get` response, or `None` for one nobody
/// has named — a Claude started by hand rather than by `agent start`.
pub fn agent_name_of(got: &Value) -> Option<String> {
    got["result"]["agent"]["name"].as_str().map(str::to_string)
}
```

- [ ] **Step 4: Run the herdr tests**

Run: `cargo test --lib herdr::tests`
Expected: all pass.

- [ ] **Step 5: Write the failing `link` unit tests and the module skeleton**

Create `src/link.rs`:

```rust
//! The link between a task and the herdr agent working on it: the agent's
//! name. Refine names its agent `task-<uuid8>` and Start working names its
//! `work-<uuid8>`, so nothing is stored on the task — the menu finds a task's
//! agent by asking herdr for those names. A Claude started by hand gets the
//! same `work-` name when it marks the task active.

use crate::{herdr, session, work};
use anyhow::{Context, Result};

/// What to rename the agent in this pane to when it marks `uuid` active, or
/// `None` to leave it as it is.
///
/// A `task-` or `work-` agent is left alone: it was named by Refine or Start
/// working, or linked already, and taking that name away would lose the task
/// it was named for — a refine marking its task active is still that task's
/// refine. Any other agent, named or not, takes the task's `work-` name.
pub fn pane_link_name(current: Option<&str>, uuid: &str) -> Option<String> {
    if current.is_some_and(|n| n.starts_with("task-") || n.starts_with("work-")) {
        return None;
    }
    Some(work::work_agent_name(uuid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unnamed_or_otherwise_named_agent_takes_the_tasks_work_name() {
        assert_eq!(pane_link_name(None, "7CD9FD3A-d27b-4387").as_deref(), Some("work-7cd9fd3a"));
        assert_eq!(pane_link_name(Some("reviewer"), "7cd9fd3a-d27b").as_deref(), Some("work-7cd9fd3a"));
    }

    #[test]
    fn a_pane_named_by_refine_or_start_working_is_left_alone() {
        assert_eq!(pane_link_name(Some("task-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("work-11111111"), "7cd9fd3a-d27b"), None);
        assert_eq!(pane_link_name(Some("work-7cd9fd3a"), "7cd9fd3a-d27b"), None, "already linked");
    }
}
```

Note: `use … session` and `Context, Result` are unused until Step 7; that is a warning, not an error. Add `pub mod link;` to `src/lib.rs` between `pub mod ipc;` and `pub mod niri;`.

- [ ] **Step 6: Run them**

Run: `cargo test --lib link::tests`
Expected: PASS (the function is written with its tests; the failing half of this cycle is the integration test in Step 8).

- [ ] **Step 7: Add `link_current_pane` and call it from `apply_status`**

In `src/link.rs`, after `pane_link_name`:

```rust
/// Name the agent in the herdr pane this process runs in after `uuid`, as
/// [`pane_link_name`] decides — so a Claude started by hand, marking its task
/// active, can be found from the task's menu like one Start working made.
///
/// Outside a herdr pane, in a pane with no agent (a script in a plain shell),
/// or without herdr at all, there is nothing to link and this does nothing.
/// Only a rename herdr refuses — the name held by another live agent, say —
/// is an error.
pub fn link_current_pane(uuid: &str) -> Result<()> {
    let Some(pane) = std::env::var("HERDR_PANE_ID").ok().filter(|p| !p.is_empty()) else {
        return Ok(());
    };
    let Some(s) = session::session_from_env(
        std::env::var("HERDR_SESSION").ok().as_deref(),
        std::env::var("HERDR_SOCKET_PATH").ok().as_deref(),
    ) else {
        return Ok(());
    };
    let Ok(got) = herdr::run(&herdr::agent_get(&s, &pane)) else {
        return Ok(());
    };
    let Some(name) = pane_link_name(herdr::agent_name_of(&got).as_deref(), uuid) else {
        return Ok(());
    };
    herdr::run(&herdr::agent_rename(&s, &pane, &name))
        .with_context(|| format!("The task is active, but this pane's agent could not be named {name}"))?;
    Ok(())
}
```

In `src/main.rs`, add `link` to the `use niri_tasks::{…}` list (alphabetical: after `ipc`), and replace `apply_status` with:

```rust
/// Move a task and say so. The one place both the menu and `task status` do
/// it, so a task marked done from a script looks exactly like one marked done
/// from its card.
///
/// Marking a task active from inside a herdr pane also names that pane's
/// agent after it (see `link::link_current_pane`): that is how a Claude
/// started by hand becomes findable from the task's menu. Best effort — the
/// task is active either way, so a refused rename is only reported.
fn apply_status(uuid: &str, description: &str, status: task::Status) -> Result<()> {
    task::set_status(uuid, status)?;
    if status == task::Status::Active {
        if let Err(e) = link::link_current_pane(uuid) {
            eprintln!("{e:#}");
        }
    }
    notify::tasks(&format!("{}: {description}", status.label()));
    Ok(())
}
```

(`task::Status` derives `PartialEq` — `main.rs` already compares it with `!=` in the `Status` handler.)

- [ ] **Step 8: Write the end-to-end test against a fake herdr**

Create `tests/link_pane.rs`:

```rust
//! `niritasks task status <uuid> active`, run inside a herdr pane, names that
//! pane's agent after the task. Checked by running the real binary against a
//! fake `herdr` on `PATH` that logs what it is asked, and a scratch task
//! database, so no herdr server and no real task is touched.
//!
//! herdr's pane variables are scrubbed from the child's environment first:
//! this test is as likely as not to run inside a herdr pane itself, and must
//! never rename the agent running it.
//!
//! One test function, run in order, because writing an executable and then
//! spawning it from parallel test threads can fail with "Text file busy".

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Logs every call; `agent get` answers `$FAKE_AGENT_GET`, or fails as herdr
/// does on a pane with no agent when that is empty; `agent rename` fails as
/// herdr does on a taken name when `$FAKE_RENAME_FAILS` is set.
const FAKE_HERDR: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  *" agent get "*)
    if [ -n "$FAKE_AGENT_GET" ]; then printf '%s' "$FAKE_AGENT_GET"; exit 0; fi
    printf '%s' '{"error":{"code":"agent_not_found","message":"agent target not found"}}' >&2
    exit 1 ;;
  *" agent rename "*)
    if [ -n "$FAKE_RENAME_FAILS" ]; then
      printf '%s' '{"error":{"code":"agent_name_taken","message":"agent name is already in use"}}' >&2
      exit 1
    fi ;;
esac
printf '{}'
"#;

const UNNAMED_CLAUDE: &str = r#"{"result":{"agent":{"agent":"claude","pane_id":"w1:p2"}}}"#;

fn write_exe(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

struct Sandbox {
    dir: PathBuf,
    uuid: String,
}

impl Sandbox {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("niritasks-link-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("taskrc"), format!("data.location={}/data\n", dir.display())).unwrap();
        write_exe(&dir.join("bin/herdr"), FAKE_HERDR);
        // So the status notification does not pop up on the desktop.
        write_exe(&dir.join("bin/notify-send"), "#!/bin/sh\nexit 0\n");

        let mut s = Self { dir, uuid: String::new() };
        assert!(s.task(&["add", "link me"]).status.success(), "task add");
        s.uuid = String::from_utf8(s.task(&["_uuids"]).stdout).unwrap().trim().to_string();
        assert_eq!(s.uuid.len(), 36, "one task, by uuid");
        s
    }

    fn task(&self, args: &[&str]) -> Output {
        Command::new("task")
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .args(["rc.verbose=nothing", "rc.confirmation=off"])
            .args(args)
            .output()
            .expect("task")
    }

    fn log(&self) -> PathBuf {
        self.dir.join("herdr.log")
    }

    /// What the fake herdr was asked since the last call, one call per line.
    fn take_calls(&self) -> String {
        let calls = std::fs::read_to_string(self.log()).unwrap_or_default();
        let _ = std::fs::remove_file(self.log());
        calls
    }

    /// `niritasks task status <uuid> active`, in pane `w1:p2` of session
    /// `alpha` when `in_pane`, with every other herdr variable removed.
    fn mark_active(&self, in_pane: bool, agent_get: &str, rename_fails: bool) -> Output {
        let path = format!("{}:{}", self.dir.join("bin").display(), std::env::var("PATH").unwrap());
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_niritasks"));
        cmd.args(["task", "status", &self.uuid, "active"])
            .env("PATH", path)
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .env("FAKE_HERDR_LOG", self.log())
            .env("FAKE_AGENT_GET", agent_get)
            .env_remove("HERDR_SESSION")
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_SOCKET_PATH")
            .env_remove("HERDR_TAB_ID")
            .env_remove("HERDR_WORKSPACE_ID")
            .env_remove("FAKE_RENAME_FAILS");
        if in_pane {
            cmd.env("HERDR_SESSION", "alpha").env("HERDR_PANE_ID", "w1:p2");
        }
        if rename_fails {
            cmd.env("FAKE_RENAME_FAILS", "1");
        }
        cmd.output().expect("run niritasks")
    }

    fn is_active(&self) -> bool {
        let out = self.task(&["rc.json.array=on", &self.uuid, "export"]);
        String::from_utf8_lossy(&out.stdout).contains("\"start\"")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn marking_a_task_active_links_the_panes_agent() {
    let s = Sandbox::new();
    let work_name = format!("work-{}", &s.uuid[..8]);

    // ---- outside herdr: herdr is never asked ----------------------------
    let out = s.mark_active(false, UNNAMED_CLAUDE, false);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(s.is_active());
    assert_eq!(s.take_calls(), "", "no pane, no herdr");

    // ---- a Claude started by hand takes the task's work- name -----------
    let out = s.mark_active(true, UNNAMED_CLAUDE, false);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take_calls();
    assert!(calls.contains("--session alpha agent get w1:p2"), "{calls}");
    assert!(calls.contains(&format!("--session alpha agent rename w1:p2 {work_name}")), "{calls}");

    // ---- a pane Refine or Start working named is left alone -------------
    for name in ["task-0000aaaa", "work-0000bbbb"] {
        let named = format!(r#"{{"result":{{"agent":{{"agent":"claude","name":"{name}","pane_id":"w1:p2"}}}}}}"#);
        let out = s.mark_active(true, &named, false);
        assert!(out.status.success());
        let calls = s.take_calls();
        assert!(!calls.contains("agent rename"), "{name} kept: {calls}");
    }

    // ---- a plain shell pane has no agent to name ------------------------
    let out = s.mark_active(true, "", false);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!s.take_calls().contains("agent rename"));

    // ---- a refused rename is reported, and the task is still active -----
    let out = s.mark_active(true, UNNAMED_CLAUDE, true);
    assert!(out.status.success(), "the status change stands");
    assert!(s.is_active());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("already in use"), "{stderr}");
}
```

- [ ] **Step 9: Run it and the whole suite**

Run: `cargo test --test link_pane`
Expected: PASS. (To see it fail meaningfully, temporarily comment out the `link::link_current_pane` call in `apply_status`: the "takes the task's work- name" assertion fails. Restore it.)

Run: `cargo test`
Expected: every suite passes, no warnings about unused imports in `src/link.rs`.

- [ ] **Step 10: Document it in the README**

In `README.md`, the herdr paragraph ends:

```
repo's `.config/wt.toml` hooks, asking in a herdr tab the first time a repo's
hooks need approving.
```

Append, as a new paragraph directly after it:

```
A Claude you start by hand in a herdr pane is linked to a task when it marks
that task active (`niritasks task status <uuid> active`): its agent is named
`work-<uuid8>`, the name Start working gives its own, so the task's menu can
find it. An agent already named by Refine or Start working keeps its name.
```

- [ ] **Step 11: Commit**

```bash
git add src/herdr.rs src/link.rs src/lib.rs src/main.rs tests/link_pane.rs README.md
git commit -m "Name a hand-started Claude after the task it marks active

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Go to session from the task menu

**Files:**
- Modify: `src/herdr.rs` (add `agent_list` beside `agent_get`; `agent_names` beside `agent_name_of`; tests)
- Modify: `src/link.rs` (add `session_agent`, `live_agent`, `go_to`; tests)
- Modify: `src/main.rs` — `TaskCommand` (~line 66–123), `task_command` (~line 184–322), `task_menu` (~line 380–416), `mod tests` (end of file)
- Modify: `README.md` (commands list ~line 137; panel paragraph ~line 56–58)

**Interfaces:**
- Consumes: `refine::agent_name(uuid) -> String` (`task-<uuid8>`); `work::work_agent_name(uuid) -> String`; `refine::open_session(dir: &Path, s: &str) -> Result<Value>` (`pub(crate)`, reachable from `link` inside the crate); `session::{start_dir, herdr_session_name}`; `herdr::{run, agent_focus}`.
- Produces:
  - `herdr::agent_list(session: &str) -> Vec<String>`
  - `herdr::agent_names(list: &serde_json::Value) -> Vec<String>`
  - `link::session_agent(names: &[String], uuid: &str) -> Option<String>`
  - `link::live_agent_names(workspace: &str) -> Vec<String>`
  - `link::live_agent(workspace: &str, uuid: &str) -> Option<String>`
  - `link::go_to(workspace: &str, uuid: &str) -> anyhow::Result<()>`
  - CLI: `niritasks task session <uuid>`
  - `main.rs`: `const GO_TO_SESSION: &str = "Go to session";`, `fn menu_entries(has_session: bool) -> Vec<String>`

- [ ] **Step 1: Write the failing herdr tests**

Append inside `mod tests` in `src/herdr.rs`:

```rust
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
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib herdr::tests`
Expected: compile error — `agent_list` / `agent_names` not found.

- [ ] **Step 3: Add the two herdr functions**

In `src/herdr.rs`, directly after `agent_get`:

```rust
/// Every live agent in the session — one call that answers whether a task
/// has a Claude on it, for the menu that opens on every card click.
pub fn agent_list(session: &str) -> Vec<String> {
    cmd(session, &["agent", "list"])
}
```

Directly after `agent_name_of`:

```rust
/// The names of the named agents in an `agent list` response. Unnamed ones
/// are left out: a task's agent is found by name.
pub fn agent_names(list: &Value) -> Vec<String> {
    list["result"]["agents"]
        .as_array()
        .map(|agents| agents.iter().filter_map(|a| a["name"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}
```

Run: `cargo test --lib herdr::tests` — Expected: PASS.

- [ ] **Step 4: Write the failing `link` tests**

Append inside `mod tests` in `src/link.rs`:

```rust
    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    /// The working Claude is the one you most likely want back; a refine
    /// still open beside it is second.
    #[test]
    fn the_working_claude_is_preferred_over_a_refine() {
        let live = names(&["task-7cd9fd3a", "work-7cd9fd3a"]);
        assert_eq!(session_agent(&live, "7CD9FD3A-d27b").as_deref(), Some("work-7cd9fd3a"));
    }

    #[test]
    fn a_refine_alone_is_still_a_session_to_go_to() {
        let live = names(&["reviewer", "task-7cd9fd3a"]);
        assert_eq!(session_agent(&live, "7cd9fd3a-d27b").as_deref(), Some("task-7cd9fd3a"));
    }

    #[test]
    fn another_tasks_agents_are_not_this_ones() {
        let live = names(&["work-11111111", "task-22222222", "reviewer"]);
        assert_eq!(session_agent(&live, "7cd9fd3a-d27b"), None);
        assert_eq!(session_agent(&[], "7cd9fd3a-d27b"), None);
    }
```

Run: `cargo test --lib link::tests`
Expected: compile error — `session_agent` not found.

- [ ] **Step 5: Add `session_agent`, `live_agent` and `go_to`**

In `src/link.rs`, change the imports to:

```rust
use crate::{herdr, refine, session, work};
use anyhow::{Context, Result};
use std::path::Path;
```

and add after `link_current_pane`:

```rust
/// Which of `names` is `uuid`'s agent: its working Claude (`work-`) if it
/// has one, else its refine (`task-`).
pub fn session_agent(names: &[String], uuid: &str) -> Option<String> {
    [work::work_agent_name(uuid), refine::agent_name(uuid)]
        .into_iter()
        .find(|want| names.iter().any(|n| n == want))
}

/// Every named live agent in `workspace`'s herdr session, from one
/// `agent list` — what the panel asks once for all its cards. A session that
/// is not running, or no herdr at all, has none: the menu and the panel ask
/// this on every open and must not fail for it.
pub fn live_agent_names(workspace: &str) -> Vec<String> {
    let s = session::herdr_session_name(workspace);
    herdr::run(&herdr::agent_list(&s))
        .map(|list| herdr::agent_names(&list))
        .unwrap_or_default()
}

/// The live agent working on `uuid` in `workspace`'s herdr session, if there
/// is one.
pub fn live_agent(workspace: &str, uuid: &str) -> Option<String> {
    session_agent(&live_agent_names(workspace), uuid)
}

/// Bring the terminal showing `workspace`'s herdr session forward and focus
/// the agent working on `uuid` in it. Never starts anything: with no live
/// agent — it exited since the menu opened, say — this is an error, not a
/// new session.
pub fn go_to(workspace: &str, uuid: &str) -> Result<()> {
    let name = live_agent(workspace, uuid)
        .context("No Claude is working on this task in this workspace's herdr session.")?;
    let home = std::env::var("HOME").context("HOME is unset")?;
    let dir = session::start_dir(Path::new(&home), workspace);
    let s = session::herdr_session_name(workspace);
    refine::open_session(&dir, &s)?;
    herdr::run(&herdr::agent_focus(&s, &name))?;
    Ok(())
}
```

Run: `cargo test --lib link::tests` — Expected: PASS.

- [ ] **Step 6: Write the failing CLI and menu tests**

Append inside `mod tests` at the end of `src/main.rs`:

```rust
    /// The menu's Go to session runs this; a script can too.
    #[test]
    fn going_to_a_tasks_session_is_a_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "session", "c53b6e3d"]).is_ok());
    }

    /// Go to session leads the menu when there is a session to go to, and a
    /// task with none gets exactly the menu it always had.
    #[test]
    fn go_to_session_leads_the_menu_only_when_there_is_one() {
        let without = menu_entries(false);
        assert_eq!(
            without,
            vec!["Edit", "Note", "Refine", "Grill me", "Start working", "Update status", "Move to workspace"]
        );
        let with = menu_entries(true);
        assert_eq!(with[0], GO_TO_SESSION);
        assert_eq!(with[1..], without[..]);
    }
```

Run: `cargo test --bin niritasks`
Expected: compile error — `menu_entries`, `GO_TO_SESSION` not found; no `Session` variant.

- [ ] **Step 7: Add the `task session` command**

In `src/main.rs`, add to `enum TaskCommand`, directly after the `Start { … }` variant:

```rust
    /// Go to the Claude working on a task, in the workspace's herdr session
    Session { uuid: String },
```

In `task_command`, add an arm after `TaskCommand::Refine { … }`:

```rust
        TaskCommand::Session { uuid } => {
            require_workspace_tag()?;
            let workspace = niri::focused_workspace_name()?.unwrap_or_default();
            // The uuid as found, not as typed: the agent's name is made from
            // its first eight characters, and a typed prefix may be shorter.
            let t = task::get(&uuid)?.context("task not found")?;
            link::go_to(&workspace, &t.uuid)?;
        }
```

- [ ] **Step 8: Build the menu from `menu_entries` and dispatch Go to session**

In `src/main.rs`, directly above `fn task_menu`, add:

```rust
/// The menu entry that goes back to the Claude working on a task.
const GO_TO_SESSION: &str = "Go to session";

/// The task menu's entries. Go to session leads, and only when a Claude is
/// working on the task — a task with none shows the menu as it always has.
fn menu_entries(has_session: bool) -> Vec<String> {
    let mut entries = Vec::new();
    if has_session {
        entries.push(GO_TO_SESSION.to_string());
    }
    entries.extend(
        ["Edit", "Note", "Refine", "Grill me", "Start working", "Update status", "Move to workspace"]
            .map(String::from),
    );
    entries
}
```

Replace the top of `task_menu`, from `let action = Picker::new()` through the closing `])?;` of `.run(&[…])`, with:

```rust
    // Asked of herdr on every open; a session that is not running answers
    // at once, and no answer just means no Go to session.
    let workspace = niri::focused_workspace_name()?.unwrap_or_default();
    let entries = menu_entries(link::live_agent(&workspace, &selected).is_some());
    let action = Picker::new()
        .lines(entries.len())
        .width(20)
        .prompt("")
        .run(&entries)?;
```

(`Picker::lines` takes a `usize`.)

In the `match action.as_deref()` below, add as the first arm:

```rust
        // Back to the Claude working on it — never starts one.
        Some(GO_TO_SESSION) => return task_command(TaskCommand::Session { uuid: selected }),
```

(`selected` is the full uuid on both routes into `task_menu`: the picker returns `t.uuid`, and `task menu <uuid>` is given one by the task card.)

- [ ] **Step 9: Run the tests**

Run: `cargo test`
Expected: every suite passes, including `every_card_button_is_a_command_the_cli_accepts`.

- [ ] **Step 10: Document it in the README**

In `README.md`'s commands block, after the two `task start` lines:

```
niritasks task start <uuid>        # its own worktree (task/<slug>-<uuid8>), opened in the herdr session,
                                   #   with Claude planning it; picked again, back to both
```

add:

```
niritasks task session <uuid>      # back to the Claude working on it (work-/task-<uuid8>) in the herdr session
```

In the panel paragraph, replace:

```
the card itself opens its full menu, which also has Note, Grill me, Update
status and Move to workspace. Every button gives the keyboard back as it runs,
```

with:

```
the card itself opens its full menu, which also has Note, Grill me, Update
status and Move to workspace — led by Go to session while a Claude is working
on the task. Every button gives the keyboard back as it runs,
```

- [ ] **Step 11: Commit**

```bash
git add src/herdr.rs src/link.rs src/main.rs README.md
git commit -m "Go to the Claude working on a task from its menu

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: A Go to session button on the keyboard panel's cards

While the panel has the keyboard (Mod+Alt+Ctrl+T), every card shows its action row. A card whose task has a live `work-`/`task-` agent gets a **Go to session** button first in the row — on an active task it sits where Start working sits on other cards. Which cards get it is decided **once**, when the panel takes the keyboard, from one `herdr agent list`: not on each focus move (the row would jump as you move, and each move would spawn herdr on the GTK thread), and not on the daemon's background refresh (tucked cards show no buttons).

**Files:**
- Modify: `src/panel/actions.rs` (whole `impl Action` and its tests)
- Modify: `src/panel/keys.rs` (`key_action` and tests)
- Modify: `src/panel/style.rs:44-67` (colours) and its `every_action_button_has_its_colour` test stays as is
- Modify: `src/panel/surface.rs` — `struct Panel` (~line 100–120), `Panel::new`'s struct literal (~line 193–199), `take_keyboard` (~line 333–348), `release_keyboard` (~line 350–360), `card_widget` (~line 515), `key`'s doc comment (~line 556)
- Modify: `src/daemon.rs:185-197` (`Request::Panel`)
- Modify: `README.md` (button table, ~line 46–52), `CONTEXT.md` (**Action row**)

**Interfaces:**
- Consumes: `link::live_agent_names(workspace: &str) -> Vec<String>` and `link::session_agent(names: &[String], uuid: &str) -> Option<String>` (Task 2); CLI `niritasks task session <uuid>` (Task 2).
- Produces: `Action::Session`; `Action::for_status(status: Status, has_session: bool) -> Vec<Action>`; `Panel::take_keyboard(self: &Rc<Self>, agents: Vec<String>) -> bool`.

- [ ] **Step 1: Update and add the `actions.rs` tests (failing)**

In `src/panel/actions.rs`'s `mod tests`, replace the first three tests and `labels_read_as_the_menu_does` and `each_button_runs_its_menu_entrys_command` with:

```rust
    #[test]
    fn an_active_task_gets_stop_in_place_of_start() {
        let names: Vec<&str> = Action::for_status(Status::Active, false).iter().map(|a| a.name()).collect();
        assert_eq!(names, vec!["refine", "edit", "stop", "remove"]);
    }

    #[test]
    fn a_task_not_yet_active_gets_start_and_no_stop() {
        for status in [Status::Pending, Status::Blocked, Status::Planned] {
            let got = Action::for_status(status, false);
            assert_eq!(got, vec![Action::Start, Action::Refine, Action::Edit, Action::Remove], "{status:?}");
        }
    }

    /// Go to session leads the row only while a Claude is on the task — on an
    /// active one, in the place Start working has on the others.
    #[test]
    fn a_task_with_a_live_claude_gets_go_to_session_first() {
        assert_eq!(
            Action::for_status(Status::Active, true),
            vec![Action::Session, Action::Refine, Action::Edit, Action::Stop, Action::Remove]
        );
        // A refine open on a task not yet started.
        assert_eq!(
            Action::for_status(Status::Planned, true),
            vec![Action::Session, Action::Start, Action::Refine, Action::Edit, Action::Remove]
        );
    }

    #[test]
    fn more_is_no_one_task_and_gets_no_buttons() {
        assert!(Action::for_status(Status::More, false).is_empty());
        assert!(Action::for_status(Status::More, true).is_empty());
    }

    #[test]
    fn labels_read_as_the_menu_does() {
        let labels: Vec<&str> = Action::ALL.iter().map(|a| a.label()).collect();
        assert_eq!(labels, vec!["Go to session", "Start working", "Refine", "Edit", "Stop", "Remove"]);
        assert_eq!(Action::CONFIRM_REMOVE, "Confirm remove");
    }

    #[test]
    fn each_button_runs_its_menu_entrys_command() {
        let u = "c53b6e3d-ca05-4aae-8588-4ee1abc25f5b";
        assert_eq!(Action::Session.args(u), vec!["task", "session", u]);
        assert_eq!(Action::Start.args(u), vec!["task", "start", u]);
        assert_eq!(Action::Refine.args(u), vec!["task", "refine", u]);
        assert_eq!(Action::Edit.args(u), vec!["task", "edit", u]);
        assert_eq!(Action::Stop.args(u), vec!["task", "status", u, "stopped"]);
        assert_eq!(Action::Remove.args(u), vec!["task", "status", u, "deleted", "--yes"]);
    }
```

Leave `every_button_has_its_own_icon` and `start_never_only_marks_the_task_active` as they are.

Run: `cargo test --lib panel::actions`
Expected: compile errors — no `Action::Session`, `for_status` takes one argument.

- [ ] **Step 2: Add `Action::Session`**

In `src/panel/actions.rs`:

Add `Session` as the enum's first variant, with a doc comment:

```rust
pub enum Action {
    /// Back to the Claude working on the task; only on a card that has one.
    Session,
    Start,
    Refine,
    Edit,
    Stop,
    Remove,
}
```

Replace `ALL` and `for_status`:

```rust
    /// In the order the buttons sit, left to right.
    pub const ALL: [Action; 6] = [Session, Start, Refine, Edit, Stop, Remove];

    /// The buttons a card gets, left to right. Go to session leads while a
    /// Claude is working on the task (`has_session`), as it leads the menu. An
    /// active task is already being worked, so it gets Stop and no Start
    /// working; the rest have nothing to stop. None on "+N more", which stands
    /// for no one task.
    pub fn for_status(status: Status, has_session: bool) -> Vec<Action> {
        let skip = match status {
            Status::More => return Vec::new(),
            Status::Active => Start,
            Status::Pending | Status::Blocked | Status::Planned => Stop,
        };
        Self::ALL
            .into_iter()
            .filter(|a| *a != skip && (*a != Session || has_session))
            .collect()
    }
```

Add a `Session` arm to each of `label`, `icon`, `name` and `args`:

```rust
            Session => "Go to session",      // label()
            Session => "\u{f120}",           // icon(): Font Awesome's terminal
            Session => "session",            // name()
            Session => &["task", "session", uuid],  // args()
```

and in `icon`'s doc comment change "play, magic wand, pencil, stop and trash can" to "terminal, play, magic wand, pencil, stop and trash can".

- [ ] **Step 3: Give it a key and a colour**

In `src/panel/keys.rs`, `key_action`, add after the `gdk::Key::Right | gdk::Key::Tab` arm:

```rust
        gdk::Key::g => KeyAction::Run(Action::Session),
```

and in its tests add to `letters_and_delete_run_their_buttons`:

```rust
        assert_eq!(key_action(gdk::Key::g), KeyAction::Run(Action::Session));
```

and to `capitals_run_their_buttons_too`:

```rust
        assert_eq!(key_action(gdk::Key::G), KeyAction::Run(Action::Session));
```

In `src/panel/style.rs`, after `pub const REMOVE`, add:

```rust
/// Go to session blue: mocha's `blue`, apart from every other button's colour.
pub const SESSION: &str = "#89b4fa";
```

change the action-colours doc comment's first sentence list to include it ("Go to session blue, Refine mauve, …"), and make the table:

```rust
pub const ACTION_COLOURS: [(&str, &str); 6] = [
    ("session", SESSION),
    ("start", ACTIVE),
    ("refine", REFINE),
    ("edit", EDIT),
    ("stop", STOP),
    ("remove", REMOVE),
];
```

- [ ] **Step 4: Run the panel's pure tests**

Run: `cargo test --lib panel`
Expected: `actions`, `keys` and `style` tests pass (`every_action_button_has_its_colour` now covers `session`). `surface.rs` does not compile yet if `cargo` reports the old one-argument `for_status` call — fix that in Step 5 before running again.

- [ ] **Step 5: Ask herdr once when the panel takes the keyboard**

In `src/panel/surface.rs`:

Add a field to `struct Panel`, after `keyboard`:

```rust
    /// The live herdr agents in the workspace's session, asked once as the
    /// panel takes the keyboard: which cards get Go to session. Asked then
    /// and no other time — not on each focus move, which would move the row
    /// under the user and spawn herdr on every key, and not on the tucked
    /// refresh, whose cards show no buttons.
    agents: RefCell<Vec<String>>,
```

and initialise it in `Panel::new`'s struct literal, after `keyboard: Cell::new(false),`:

```rust
            agents: RefCell::new(Vec::new()),
```

Change `take_keyboard`:

```rust
    /// Slide out and take the keyboard, every card wrapped with its buttons,
    /// focusing the first. `agents` are the session's live agent names, for
    /// which cards get Go to session. False when there are no cards to take
    /// it for.
    pub fn take_keyboard(self: &Rc<Self>, agents: Vec<String>) -> bool {
        if self.all.borrow().is_empty() {
            return false;
        }
        *self.agents.borrow_mut() = agents;
        self.keyboard.set(true);
```

(the rest of the body unchanged). In `release_keyboard`, after `self.window.set_keyboard_mode(KeyboardMode::None);`, add:

```rust
        self.agents.borrow_mut().clear();
```

In `card_widget`, replace `for action in Action::for_status(card.status) {` with:

```rust
        let has_session = crate::link::session_agent(&self.agents.borrow(), uuid).is_some();
        for action in Action::for_status(card.status, has_session) {
```

In `key`'s doc comment, change "(Stop on a task that is not active, anything on "+N more")" to "(Stop on a task that is not active, Go to session on one with no Claude, anything on "+N more")".

In `src/daemon.rs`, `Request::Panel`, replace the first two statements and the `if` condition:

```rust
        Request::Panel => {
            let focused = niri::focused_workspace().ok().flatten();
            let output = focused.as_ref().and_then(|w| w.output.clone());
            let panel = output
                .as_ref()
                .and_then(|o| PANELS.with(|p| p.borrow().get(o).cloned()));
            // One `herdr agent list` per slide-out, for every card at once;
            // a session that is not running answers at once with none.
            let agents = focused
                .as_ref()
                .and_then(|w| w.name.as_deref())
                .map(crate::link::live_agent_names)
                .unwrap_or_default();
            if !panel.is_some_and(|p| p.take_keyboard(agents)) {
```

(the body of the `if` is unchanged).

- [ ] **Step 6: Run everything**

Run: `cargo test`
Expected: all suites pass, including `every_card_button_is_a_command_the_cli_accepts` in `main.rs`, which now checks `task session <uuid>` too.

Run: `cargo build --release 2>&1 | grep -E "^(warning|error)"` — Expected: no output.

- [ ] **Step 7: Document the button**

In `README.md`, add as the first row of the panel's button table (above **Start working**):

```
| **Go to session** (blue) | `g` | The menu's Go to session: back to the Claude working on the task — only while one is, checked as the panel slides out |
```

In `CONTEXT.md`, replace the **Action row** definition's body with:

```
The buttons along a task card's bottom edge while the task panel has the
keyboard — Go to session, Start working, Refine, Edit, Stop and Remove, an
active task getting Stop in place of Start working, and only a task with a
live Claude getting Go to session — each running what the same entry in the
task's action menu runs.
```

- [ ] **Step 8: Commit**

```bash
git add src/panel/actions.rs src/panel/keys.rs src/panel/style.rs src/panel/surface.rs src/daemon.rs README.md CONTEXT.md
git commit -m "Add a Go to session button to the keyboard panel's cards

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Acceptance (manual, after all three tasks)

These need a live herdr session, niri and fuzzel, so they are checked by hand. Build and install first: `./install.sh` (it builds the binary).

1. In the project's herdr session, open a new tab, run `claude` by hand, and have it run `niritasks task status <uuid> active` for some pending task. `herdr agent list` shows that pane's agent as `work-<uuid8>`.
2. Click that task's card (or pick it in `niritasks task list`): the menu's first entry is **Go to session**. Pick it with the terminal on another workspace or behind another window: the project terminal comes forward, focused on that Claude's pane.
3. A task with no live agent shows the menu exactly as before (seven entries, Edit first).
4. Run `niritasks task status <uuid> active` from a Refine tab's Claude (`task-<uuid8>`): `herdr agent list` still shows `task-<uuid8>` for it.
5. After `systemctl --user restart niri-tasks.service`, Mod+Alt+Ctrl+T: the task from step 1 has a blue terminal button first in its row; `g` (or clicking it) brings the terminal forward on that Claude. A task with no Claude has today's row.

Known limit, unchanged by this work: `refine::open_session` finds the terminal by the session's *first* herdr workspace's label in the window title. When the only ghostty windows on that session are showing a worktree workspace, it attaches another client instead of raising one — the same as Refine and Start working do today.
