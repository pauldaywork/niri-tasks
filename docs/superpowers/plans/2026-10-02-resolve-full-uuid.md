# Resolve the Full Uuid Before Start and Refine — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `niritasks task start <typed>`, `task start <typed> --here` and `task refine <typed>` name every branch, agent and prompt after the task's own uuid, so a task typed by its number (or any form other than its full uuid) gets the same worktree and session the task panel finds.

**Architecture:** Resolve the typed form once, at the edge, and use only the task as found after that. `work::launch` and `refine::launch` take the resolved `&task::Task` rather than a uuid string and a description, so `main.rs` cannot hand them the typed form again. `work::set_up`, which `--here` reaches, already looks the task up; it uses `t.uuid` from then on. An integration test runs the built binary with fake `herdr`, `wt` and `notify-send` on `PATH`, a fake niri IPC socket and a scratch task database, and asserts on the arguments the fakes were given.

**Tech Stack:** Rust 2021, clap derive, `niri-ipc` 26.4 (already a dependency, used by the test to build the fake niri reply), `serde_json`, Taskwarrior 2.6 CLI. No new crates.

**Spec:** Taskwarrior task `2aa08325-4d80-465d-b4bd-0774e3ebcc56`. Read it with `task rc.json.array=on 2aa08325-4d80-465d-b4bd-0774e3ebcc56 export`. Its description and notes are the spec.

## Global Constraints

- The typed form is only ever used to look the task up (`task::get`). Every name built after that comes from `t.uuid`: `work::branch_name`, `work::work_agent_name`, `work::find_task_worktree`, `work::plan_prompt`, `refine::agent_name`, `refine::prompt`, and the `--here` command line `work::launch` types into the herdr pane.
- `TaskCommand::Session` already does this right (`src/main.rs:431-438`). Match its comment's wording where a comment is needed.
- The panel and action menu pass full uuids and need no change.
- Test red first, against the binary (`env!("CARGO_BIN_EXE_niritasks")`), in the style of `tests/link_pane.rs`: one `#[test]` function per file (parallel threads writing and spawning executables can fail with "Text file busy"), fakes that log `"$*"`, herdr's pane variables scrubbed from the child's environment.
- Done when: `cargo test`, `cargo clippy --all-targets -- -D warnings` and `bash tests/all.sh` pass.
- House style: every item gets a doc comment that says *why*, in the plain voice of the surrounding code. Commits use Conventional Commits (`fix(scope): …`, imperative, lowercase, ≤72 chars), ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Correction to the spec (found while planning)

The spec's example, `niritasks task start 6377` for `637796b7-…`, does not reach the bug: Taskwarrior 2.6.2 resolves a uuid prefix only from 8 characters on, so `task::get("6377")` reads `6377` as a task *number*, finds no such task, and the command stops at "task not found". (Checked: prefixes of 4, 6 and 7 characters export nothing. 8 characters, and longer prefixes such as `ca5e8109-6e`, export the task.) An 8-or-more-character prefix gives the same `uuid8` as the full uuid, so it was always safe.

The form that does trigger the bug is a **task number**: `niritasks task start 34` resolves task 34, then names the agent `work-34`, the branch `task/<slug>-34`, and sends `--here … '34'` and `/refine-task 34`. Numbers are renumbered as tasks complete, so that prompt can later point at a different task. llms.txt rule 3 already tells agents never to use the number. A person typing one is the remaining case. The tests below therefore type the task's number, `1`. The fix is the one the spec gives.

## Decisions made while planning (flag any you disagree with)

- **The launchers take `&Task`.** The spec asks to pass `t.uuid` and to consider "resolve once at the edge" everywhere. Changing `work::launch(workspace, uuid, description)` to `work::launch(workspace, t: &Task)` and `refine::launch(workspace, uuid, description, mode)` to `refine::launch(workspace, t: &Task, mode)` does both: the type says "a task that was found", and `main.rs` has no typed string to pass by mistake. Each has one caller (`src/main.rs`). Moving the other task-command rules into the library is candidate 3's refactor and stays out of scope.
- **`set_up` shadows `uuid`** with the found task's uuid on the line after `task::get`. Then every later use in the function is the resolved one, including any added later, and the diff stays small. `set_up_here` keeps taking the typed `&str`: it is the `--here` entry point, and `set_up` is where it is resolved.
- **Refine's test keeps the real `HOME`.** `refine::launch` refuses to start while any socket on the machine sits outside its sandbox's hidden folders. One of those is `$HOME/.config/herdr`, where the real herdr sessions keep their sockets. With a scratch `HOME`, those sockets count as exposed and Refine refuses. With the real `HOME`, the check sees the same machine a real Refine does, and the fakes on `PATH` still keep it away from the real herdr. Start's test uses a scratch `HOME` because it needs `~/Projects/alpha/.git`.
- **The fake niri socket lives directly under `/tmp`**, not in the test's temp folder. Refine's socket check hides `/tmp` but leaves `/tmp/claude-<uid>` visible, and a Claude session's `TMPDIR` may point there.

## File Structure

- Create: `tests/typed_task_number.rs`. Sandbox, fakes and the one test function. Task 1 writes it with the Start checks. Task 2 adds the Refine checks to the end of the same function.
- Modify: `src/work.rs`. `launch` takes `&task::Task`; `set_up` uses the found uuid.
- Modify: `src/refine.rs`. `launch` takes `&task::Task`.
- Modify: `src/main.rs:405-429`. `Start` and `Refine` pass `&t`.

---

### Task 1: Start names its worktree and agent after the task's uuid

**Files:**
- Create: `tests/typed_task_number.rs`
- Modify: `src/work.rs:208-252` (`launch`), `src/work.rs:278-292` (`set_up`)
- Modify: `src/main.rs:405-416` (`TaskCommand::Start`)

**Interfaces:**
- Consumes: `task::Task { uuid: String, description: String, status: String, … }` and `task::get(&str) -> Result<Option<Task>>` (`src/task.rs`), both unchanged.
- Produces: `pub fn work::launch(workspace: &str, t: &task::Task) -> Result<()>`. The test harness in `tests/typed_task_number.rs` (`Sandbox::new`, `Sandbox::niritasks`, `Sandbox::take`, `Sandbox::herdr_log`, `Sandbox::wt_log`, `FAKE_HERDR`, `FAKE_WT`, `fake_niri`), which Task 2 extends.

- [ ] **Step 1: Write the failing test**

Create `tests/typed_task_number.rs`:

```rust
//! `niritasks task start` and `task start --here` name what they make after
//! the task's own uuid, not the form it was typed in. A task number finds the
//! same task, but a branch or agent named from it ends in `-1`, not the uuid's
//! first eight characters, so the task panel, which passes full uuids, never
//! finds it again and makes a second worktree and a second Claude.
//!
//! Checked by running the real binary against a scratch task database, fake
//! `herdr`, `wt` and `notify-send` on `PATH` that log what they are asked, and
//! a fake niri socket that reports one focused workspace, `alpha`. No herdr
//! server, worktree or real task is touched.
//!
//! herdr's pane variables are scrubbed from the child's environment first:
//! this test is as likely as not to run inside a herdr pane itself, and must
//! never act on the pane running it.
//!
//! One test function, run in order, because writing an executable and then
//! spawning it from parallel test threads can fail with "Text file busy".

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Logs every call. `agent get` finds only the agent named `$FAKE_AGENT`
/// and fails as herdr does on any other. Every other call answers with the
/// fields the launchers read: a workspace, its root pane, and an empty
/// workspace list, so no niri window is looked for.
const FAKE_HERDR: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  *" agent get $FAKE_AGENT")
    printf '{"result":{"agent":{"agent":"claude","name":"%s","agent_status":"idle"}}}' "$FAKE_AGENT"
    exit 0 ;;
  *" agent get "*)
    printf '%s' '{"error":{"code":"agent_not_found","message":"agent target not found"}}' >&2
    exit 1 ;;
esac
printf '%s' '{"result":{"workspace":{"workspace_id":"w1"},"root_pane":{"pane_id":"w1:p1"},"workspaces":[]}}'
"#;

/// Logs every call. `wt list` answers `$FAKE_WT_LIST`, `wt switch` answers
/// `$FAKE_WT_SWITCH`.
const FAKE_WT: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_WT_LOG"
case "$*" in
  *" list "*) printf '%s' "$FAKE_WT_LIST" ;;
  *" switch "*) printf '%s' "$FAKE_WT_SWITCH" ;;
esac
"#;

/// A repository with no task worktrees.
const NO_WORKTREES: &str = r#"{"items":[]}"#;

fn write_exe(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A niri IPC socket that answers every request with one focused workspace,
/// `alpha`. Directly under `/tmp` rather than the test's temp folder: Refine
/// refuses to start while a socket sits where its sandbox could reach it,
/// and its sandbox keeps `/tmp/claude-<uid>`, where a Claude session's
/// `TMPDIR` may point, in reach.
fn fake_niri() -> PathBuf {
    let path = PathBuf::from(format!("/tmp/niritasks-number-test-niri-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind the fake niri socket");
    let alpha = niri_ipc::Workspace {
        id: 1,
        idx: 1,
        name: Some("alpha".to_string()),
        output: None,
        is_urgent: false,
        is_active: true,
        is_focused: true,
        active_window_id: None,
    };
    let reply: niri_ipc::Reply = Ok(niri_ipc::Response::Workspaces(vec![alpha]));
    let line = format!("{}\n", serde_json::to_string(&reply).unwrap());
    // One request per connection, as niri_ipc's Socket sends them.
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            if reader.read_line(&mut request).is_ok() {
                let _ = reader.get_mut().write_all(line.as_bytes());
            }
        }
    });
    path
}

struct Sandbox {
    dir: PathBuf,
    niri_socket: PathBuf,
    uuid: String,
}

impl Sandbox {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("niritasks-number-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["data", "bin", "home/Projects/alpha/.git", "worktree"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        std::fs::write(dir.join("taskrc"), format!("data.location={}/data\n", dir.display())).unwrap();
        write_exe(&dir.join("bin/herdr"), FAKE_HERDR);
        write_exe(&dir.join("bin/wt"), FAKE_WT);
        // So the notifications do not pop up on the desktop.
        write_exe(&dir.join("bin/notify-send"), "#!/bin/sh\nexit 0\n");

        let niri_socket = fake_niri();
        let mut s = Self { dir, niri_socket, uuid: String::new() };
        assert!(s.task(&["add", "start me"]).status.success(), "task add");
        s.uuid = String::from_utf8(s.task(&["1", "_uuids"]).stdout).unwrap().trim().to_string();
        assert_eq!(s.uuid.len(), 36, "task 1, by uuid");
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

    fn herdr_log(&self) -> PathBuf {
        self.dir.join("herdr.log")
    }

    fn wt_log(&self) -> PathBuf {
        self.dir.join("wt.log")
    }

    /// What a fake was asked since the last call, one call per line.
    fn take(&self, log: &Path) -> String {
        let calls = std::fs::read_to_string(log).unwrap_or_default();
        let _ = std::fs::remove_file(log);
        calls
    }

    /// `niritasks <args>` with `home` as `HOME`, the fakes first on `PATH`,
    /// `agent` the one herdr agent that exists, and `wt_list` what `wt list`
    /// says. `wt switch` always reports the task's branch, made in
    /// `worktree/`.
    fn niritasks(&self, args: &[&str], home: &Path, agent: &str, wt_list: &str) -> Output {
        let path = format!("{}:{}", self.dir.join("bin").display(), std::env::var("PATH").unwrap());
        let switched = format!(
            r#"{{"branch":"task/start-me-{}","path":"{}"}}"#,
            &self.uuid[..8],
            self.dir.join("worktree").display()
        );
        Command::new(env!("CARGO_BIN_EXE_niritasks"))
            .args(args)
            .env("PATH", path)
            .env("HOME", home)
            .env("TASKRC", self.dir.join("taskrc"))
            .env("TASKDATA", self.dir.join("data"))
            .env("NIRI_SOCKET", &self.niri_socket)
            .env("FAKE_HERDR_LOG", self.herdr_log())
            .env("FAKE_WT_LOG", self.wt_log())
            .env("FAKE_AGENT", agent)
            .env("FAKE_WT_LIST", wt_list)
            .env("FAKE_WT_SWITCH", switched)
            .env_remove("HERDR_SESSION")
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_SOCKET_PATH")
            .env_remove("HERDR_TAB_ID")
            .env_remove("HERDR_WORKSPACE_ID")
            .output()
            .expect("run niritasks")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
        let _ = std::fs::remove_file(&self.niri_socket);
    }
}

#[test]
fn a_task_number_names_everything_after_the_tasks_uuid() {
    let s = Sandbox::new();
    let uuid8 = &s.uuid[..8];
    let branch = format!("task/start-me-{uuid8}");
    let work = format!("work-{uuid8}");
    let home = s.dir.join("home");

    // ---- Start goes back to the worktree the task already has -----------
    // Made as the panel's Start working makes it: on a branch that ends in
    // the uuid's first eight characters, with its Claude still running.
    let has_worktree = format!(
        r#"{{"items":[{{"branch":"{branch}","worktree":{{"path":"{}"}}}}]}}"#,
        s.dir.join("worktree").display()
    );
    let out = s.niritasks(&["task", "start", "1"], &home, &work, &has_worktree);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent focus {work}")), "{calls}");
    assert!(!calls.contains("pane run"), "no second worktree: {calls}");

    // ---- with no worktree yet, Start hands --here the full uuid ---------
    let out = s.niritasks(&["task", "start", "1"], &home, "", NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains("--session alpha pane run w1:p1 "), "{calls}");
    assert!(calls.contains(&format!("--here --workspace 'alpha' '{}'", s.uuid)), "{calls}");

    // ---- --here names the branch and the agent after the uuid -----------
    let out = s.niritasks(&["task", "start", "1", "--here", "--workspace", "alpha"], &home, "", NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let made = s.take(&s.wt_log());
    assert!(made.contains(&format!("switch --create {branch} --no-cd")), "{made}");
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent start {work} --kind claude")), "{calls}");
    assert!(
        calls.contains(&format!("agent prompt {work} /superpowers:writing-plans Plan Taskwarrior task {}.", s.uuid)),
        "{calls}"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test typed_task_number`
Expected: FAIL at the first scenario's `agent focus work-<uuid8>` assertion. The printed herdr calls show `workspace create` and `pane run … '1'`: Start looked for a branch ending in `-1`, missed the task's worktree, and set about making a second one.

If it fails earlier instead, at `bind the fake niri socket` or with "niri is not running", the sandbox the command runs in is blocking writes to `/tmp` itself. Rerun the command outside that sandbox. Don't move the socket (see Decisions).

- [ ] **Step 3: Make `work::launch` take the found task**

In `src/work.rs`, replace the signature and the three uses of the old parameters in `launch` (currently lines 208-252). The doc comment gains one sentence. The body is otherwise unchanged:

```rust
/// Start working on a task from its menu: back to its worktree if it has one,
/// otherwise a short-lived tab in the project's session that makes one.
///
/// Making it happens in that tab rather than here because worktrunk asks the
/// user to approve a repo's hooks before running them the first time, and
/// refuses outright without a terminal to ask on; the tab is also where the
/// hooks' output — a database clone, say — can be read.
///
/// The task as found, not a uuid as typed: the worktree and the agent are
/// found again by the uuid's first eight characters, and a typed task number
/// has none of them.
pub fn launch(workspace: &str, t: &task::Task) -> Result<()> {
    let repo = repo_for(workspace)?;
    anyhow::ensure!(project::on_path("wt"), "worktrunk (wt) is not installed.");
    let s = session::herdr_session_name(workspace);
    let name = work_agent_name(&t.uuid);

    let list = refine::open_session(&repo, &s)?;

    if let Some(wt) = find_task_worktree(&wt_list(&repo)?, &t.uuid) {
```

Further down in the same function, change `return start_claude(&s, &name, &pane, uuid);` to:

```rust
        return start_claude(&s, &name, &pane, &t.uuid);
```

change `let label = format!("Start: {}", short(description));` to:

```rust
    let label = format!("Start: {}", short(&t.description));
```

and change the last argument of the `--here` command from `sh_quote(uuid)` to:

```rust
        sh_quote(&t.uuid)
```

- [ ] **Step 4: Make `work::set_up` use the found uuid**

In `src/work.rs`, `set_up` (currently lines 278-292), add the shadowing line and its comment directly after the `task::get` line. Nothing else in the function changes: `find_task_worktree`, `branch_name`, `work_agent_name` and `start_claude` already read `uuid`, which is now the found one.

```rust
fn set_up(workspace: &str, uuid: &str) -> Result<()> {
    let t = task::get(uuid)?.context("task not found")?;
    // The uuid as found, not as typed, from here on: the branch and the agent
    // are named after its first eight characters, and a typed task number
    // has none of them.
    let uuid = t.uuid.as_str();
    let repo = repo_for(workspace)?;
```

- [ ] **Step 5: Pass the found task from `main.rs`**

In `src/main.rs`, `TaskCommand::Start` (currently lines 405-416), replace the last line, `work::launch(&workspace, &uuid, &t.description)?;`, with:

```rust
            work::launch(&workspace, &t)?;
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test --test typed_task_number`
Expected: PASS, 1 test.

- [ ] **Step 7: Run the whole suite and clippy**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: every test passes and clippy reports nothing.

- [ ] **Step 8: Commit**

```bash
git add tests/typed_task_number.rs src/work.rs src/main.rs
git commit -m "$(cat <<'EOF'
fix(work): name a started task's worktree after its own uuid

niritasks task start and task start --here built the branch, the agent
name and the --here command from the uuid as typed. A task number finds
the same task, but names ending in -1 are never found again by the task
panel, which looks for the uuid's first eight characters, so a later
Start working made a second worktree and a second Claude. work::launch
now takes the task as found, and set_up uses its uuid once it has it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Refine names its session after the task's uuid

**Files:**
- Modify: `tests/typed_task_number.rs` (end of the test function and its module doc)
- Modify: `src/refine.rs:298-341` (`launch`), `src/refine.rs:8` (imports)
- Modify: `src/main.rs:418-429` (`TaskCommand::Refine`)

**Interfaces:**
- Consumes: the Task 1 harness: `Sandbox::niritasks(&self, args: &[&str], home: &Path, agent: &str, wt_list: &str) -> Output`, `Sandbox::take(&self, log: &Path) -> String`, `Sandbox::herdr_log()`, `NO_WORKTREES`, and `s.uuid` (the full uuid of task 1).
- Produces: `pub fn refine::launch(workspace: &str, t: &task::Task, mode: Mode) -> Result<()>`.

- [ ] **Step 1: Write the failing test**

In `tests/typed_task_number.rs`, change the first paragraph of the module doc to cover Refine:

```rust
//! `niritasks task start`, `task start --here` and `task refine` name what
//! they make after the task's own uuid, not the form it was typed in. A task
//! number finds the same task, but a branch or agent named from it ends in
//! `-1`, not the uuid's first eight characters, so the task panel, which
//! passes full uuids, never finds it again and makes a second worktree, a
//! second Claude or a second refine session.
```

Then append to the end of `a_task_number_names_everything_after_the_tasks_uuid`, after the `--here` scenario:

```rust
    // Refine with the real HOME: it refuses to start while a socket sits
    // outside its sandbox's hidden folders, and the real herdr sessions'
    // sockets are hidden only as $HOME/.config/herdr. The fakes on PATH
    // still keep it away from the real herdr.
    let real_home = PathBuf::from(std::env::var("HOME").unwrap());
    let refiner = format!("task-{uuid8}");

    // ---- Refine goes back to the session the task already has -----------
    let out = s.niritasks(&["task", "refine", "1"], &real_home, &refiner, NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent focus {refiner}")), "{calls}");
    assert!(!calls.contains("agent start"), "no second session: {calls}");

    // ---- a new refine session is named and prompted with the uuid -------
    let out = s.niritasks(&["task", "refine", "1"], &real_home, "", NO_WORKTREES);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let calls = s.take(&s.herdr_log());
    assert!(calls.contains(&format!("--session alpha agent start {refiner} ")), "{calls}");
    assert!(calls.contains(&format!("agent prompt {refiner} /refine-task {}", s.uuid)), "{calls}");
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test typed_task_number`
Expected: FAIL at the Refine scenario's `agent focus task-<uuid8>` assertion. The printed herdr calls show `agent get task-1`, then `agent start task-1 …` and `agent prompt task-1 /refine-task 1`: a second session, handed the task number.

If it fails with "Refine would leave these sockets in reach of its sandbox", the machine has a socket outside `/run`, `/tmp`, `/var/snap` and `~/.config/herdr`. A real Refine refuses on that machine too, so this is not the bug. Report the socket path and stop. Don't change the test to get past the check.

- [ ] **Step 3: Make `refine::launch` take the found task**

In `src/refine.rs`, add `task` to the crate import on line 8:

```rust
use crate::{herdr, niri, notify, project, session, task, text};
```

Then change `launch` (currently lines 298-341). Its doc comment gains one sentence, its signature takes the task, and the three uses of the old parameters read the task's fields:

```rust
/// Open Claude on a task in `workspace`'s herdr session.
///
/// Opens the project terminal first if the session is not running, and goes
/// back to the task's existing tab if it is already being refined.
///
/// The task as found, not a uuid as typed: the session is found again by the
/// uuid's first eight characters, and a typed task number has none of them.
pub fn launch(workspace: &str, t: &task::Task, mode: Mode) -> Result<()> {
    let home = std::env::var("HOME").context("HOME is unset")?;
    let home = Path::new(&home);
    let dir = session::start_dir(home, workspace);
    let s = session::herdr_session_name(workspace);
    let name = agent_name(&t.uuid);
```

Further down, change `let label = tab_label(mode, description);` to:

```rust
    let label = tab_label(mode, &t.description);
```

and change `herdr::run(&herdr::agent_prompt(&s, &name, &prompt(uuid, mode)))` to:

```rust
    herdr::run(&herdr::agent_prompt(&s, &name, &prompt(&t.uuid, mode)))
```

Leave `crate::task::data_location()` in `launch` as it is, or shorten it to `task::data_location()` now that `task` is imported. Either is fine; clippy accepts both.

- [ ] **Step 4: Pass the found task from `main.rs`**

In `src/main.rs`, `TaskCommand::Refine` (currently lines 418-429), replace the last line, `refine::launch(&workspace, &uuid, &t.description, mode)?;`, with:

```rust
            refine::launch(&workspace, &t, mode)?;
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --test typed_task_number`
Expected: PASS, 1 test.

- [ ] **Step 6: Check nothing else passes a typed uuid on**

Run: `grep -n "&uuid" src/main.rs`
Expected: every remaining hit is one of these:
- a `task::get(&uuid)` lookup;
- a call that hands the uuid straight to Taskwarrior, which resolves it itself (`task::replace_text`, `task::modify_description`, `task::annotate`, `edit_in_box`);
- `refine::spawn_quick(&uuid)` in `Add`, where `uuid` is the full uuid `task::add` just returned;
- `speak::run(&uuid)` or `work::set_up_here(&workspace, &uuid)`, the internal `--here` forms. The first is only run by `speak::toggle` with a found uuid. The second resolves the uuid in `set_up`.

Nothing on that list builds a herdr name, branch or prompt from a typed uuid. If some other hit does, report it rather than fixing it here: it is outside this task.

- [ ] **Step 7: Run every suite and clippy**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && bash tests/all.sh`
Expected: every test passes, clippy reports nothing, and `tests/all.sh` ends with no FAIL lines. A SKIP for a suite whose prerequisites this machine lacks is fine.

- [ ] **Step 8: Commit**

```bash
git add tests/typed_task_number.rs src/refine.rs src/main.rs
git commit -m "$(cat <<'EOF'
fix(refine): name a refine session after the task's own uuid

niritasks task refine named its herdr agent and built the refine-task
prompt from the uuid as typed. Refined by its number, a task got a
task-1 session that a later Refine with the full uuid never found, and
the skill was handed a number that changes as other tasks complete.
refine::launch now takes the task as found, as Start's launcher does.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
