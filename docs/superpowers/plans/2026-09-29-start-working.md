# Start Working in a Worktree — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A **Start working** menu row that gives the task its own worktrunk worktree, opens it as a herdr workspace in the project's session, starts Claude there with `/superpowers:writing-plans` and the task, and marks the task active.

**Architecture:** `niritasks task start <uuid>` (menu) finds or creates the task's worktree. Creating happens in a short-lived herdr tab running `niritasks task start --here --workspace <ws> <uuid>`, so worktrunk's hook approval and output reach a terminal. Pure pieces live in a new `src/work.rs`; the session-opening half of `refine::launch` is extracted and shared.

**Tech Stack:** Rust (anyhow, clap, serde_json), worktrunk 0.79 (`wt`), herdr 0.9.1, Taskwarrior.

**Spec:** `docs/superpowers/specs/2026-09-29-start-working-design.md`

## Global Constraints

- Branch is exactly `task/<slug>-<uuid8>`; slug rule as in the spec (ASCII `[a-z0-9]` runs joined by `-`, ≤40 chars cut at a `-`, empty → `task`).
- Agent name is `work-<uuid8>` (Refine's is `task-<uuid8>`; they must not collide).
- Prompt is exactly: ``/superpowers:writing-plans Plan Taskwarrior task <uuid>. Read it with `task rc.json.array=on <uuid> export`; its description and notes are the spec.``
- Claude starts with no extra flags (the user's own mode; no sandbox).
- Never pass `--yes` to `wt`: hook approval must stay the user's.
- An existing worktree is found by uuid (`task/…-<uuid8>` with a `worktree.path`), never by the current description.
- Every public item gets a why-doc-comment; errors surface by returning `Err` (main already notifies).

---

### Task 1: Share session opening

**Files:** Modify `src/refine.rs` (`launch`, ~l.274–308).

**Produces:** `pub(crate) fn open_session(dir: &Path, s: &str) -> Result<serde_json::Value>` — the session's `workspace list`, after making sure it runs and is in view.

- [ ] Extract, verbatim in behaviour:

```rust
/// Make `s`'s herdr session running and in front of the user, and return its
/// workspace list: start the project terminal if the session is stopped (a
/// window then comes with it), otherwise focus the window showing it or attach
/// another. Shared by every launcher that opens something in a project's
/// session.
pub(crate) fn open_session(dir: &Path, s: &str) -> Result<Value> {
    match herdr::run(&herdr::workspace_list(s)) {
        Ok(list) => {
            show_session_window(dir, s, &list)?;
            Ok(list)
        }
        Err(_) => {
            anyhow::ensure!(project::on_path(project::SESSION_MANAGER), "herdr is not installed.");
            // Through niri, so the window lands on the focused workspace —
            // the one the task belongs to — as the project picker's does.
            niri::spawn(project::project_terminal_command(dir, s, true))?;
            wait_for_session(s)
        }
    }
}
```

and in `launch` replace the `already_running` block with `let list = open_session(&dir, &s)?;`.

- [ ] `cargo test` — all pass unchanged. Commit: "Share opening a project's herdr session between launchers".

### Task 2: herdr argv for worktrees and plain Claude

**Files:** Modify `src/herdr.rs`.

**Produces:** `worktree_open(session, repo: &Path, path: &Path, label) -> Vec<String>`, `agent_start_claude(session, name, pane) -> Vec<String>`, `pane_run(session, pane, command) -> Vec<String>`, `opened_workspace_id(&Value) -> Option<String>`.

- [ ] Tests first:

```rust
    #[test]
    fn a_worktree_opens_as_its_own_focused_workspace() {
        assert_eq!(
            worktree_open("alpha", Path::new("/p/alpha"), Path::new("/w/alpha/task-x-1234abcd"), "task/x-1234abcd"),
            vec!["herdr", "--session", "alpha", "worktree", "open", "--cwd", "/p/alpha",
                 "--path", "/w/alpha/task-x-1234abcd", "--label", "task/x-1234abcd", "--focus", "--json"]
        );
    }

    #[test]
    fn a_working_claude_starts_with_no_extra_flags() {
        assert_eq!(
            agent_start_claude("alpha", "work-1234abcd", "w2:p1"),
            vec!["herdr", "--session", "alpha", "agent", "start", "work-1234abcd",
                 "--kind", "claude", "--pane", "w2:p1", "--timeout", "60000"]
        );
    }

    #[test]
    fn a_command_is_run_in_a_pane_by_id() {
        assert_eq!(pane_run("alpha", "w1:p4", "echo hi"),
                   vec!["herdr", "--session", "alpha", "pane", "run", "w1:p4", "echo hi"]);
    }

    /// Shape from herdr 0.9.1's `worktree open --json`.
    #[test]
    fn an_opened_worktrees_workspace_is_read_from_the_response() {
        let v: Value = serde_json::from_str(
            r#"{"result":{"already_open":false,"root_pane":{"pane_id":"w2:p1"},"workspace":{"workspace_id":"w2"}}}"#,
        ).unwrap();
        assert_eq!(opened_workspace_id(&v).as_deref(), Some("w2"));
        assert_eq!(root_pane_id(&v).as_deref(), Some("w2:p1"));
    }
```

- [ ] Run, see them fail; implement each with `cmd(...)` like the existing builders, with why-doc-comments (worktree_open: the herdr-worktrunk plugin's own registration step, so the worktree groups under the project; agent_start_claude: this session does the work, so the user's own mode; pane_run: how the setup step reaches a terminal). `opened_workspace_id` reads `result.workspace.workspace_id`.
- [ ] `cargo test`, clippy clean. Commit: "Add the herdr calls for opening a worktree and a working Claude".

### Task 3: Pure pieces of `src/work.rs`

**Files:** Create `src/work.rs`; add `pub mod work;` to `src/lib.rs` (after `pub mod theme;`).

**Produces:** `slug(&str) -> String`, `branch_name(description, uuid) -> String`, `work_agent_name(uuid) -> String`, `plan_prompt(uuid) -> String`, `pub struct TaskWorktree { pub branch: String, pub path: PathBuf }`, `find_task_worktree(list: &Value, uuid) -> Option<TaskWorktree>`, `switch_result(&Value) -> Option<TaskWorktree>`, `sh_quote(&str) -> String`.

- [ ] Tests first:

```rust
    #[test]
    fn the_slug_is_the_description_in_lowercase_ascii_runs() {
        assert_eq!(slug("Show each Claude agent's topic in herdr's sidebar!"),
                   "show-each-claude-agent-s-topic-in-herdr");
        assert_eq!(slug("  Fix   café bug  "), "fix-caf-bug");
        assert_eq!(slug("!!!"), "task");
    }

    #[test]
    fn a_long_slug_is_cut_at_a_word() {
        let s = slug("alpha beta gamma delta epsilon zeta eta theta iota");
        assert!(s.len() <= 40, "{s}");
        assert!(!s.ends_with('-'));
        assert_eq!(s, "alpha-beta-gamma-delta-epsilon-zeta-eta");
    }

    #[test]
    fn the_branch_and_agent_are_named_after_the_task() {
        let u = "7CD9FD3A-d27b-4387-8249-aaf0d6785f90";
        assert_eq!(branch_name("Fix the peek", u), "task/fix-the-peek-7cd9fd3a");
        assert_eq!(work_agent_name(u), "work-7cd9fd3a");
        assert_eq!(plan_prompt("u-1"),
            "/superpowers:writing-plans Plan Taskwarrior task u-1. Read it with `task rc.json.array=on u-1 export`; its description and notes are the spec.");
    }

    /// Shape from worktrunk 0.79's `wt list --format=json`.
    #[test]
    fn a_tasks_worktree_is_found_by_uuid_alone() {
        let list: Value = serde_json::from_str(r#"{"schema":2,"items":[
            {"branch":"main","worktree":{"path":"/p/repo"}},
            {"branch":"task/old-name-7cd9fd3a","worktree":{"path":"/w/repo/task-old-name-7cd9fd3a"}},
            {"branch":"task/other-11111111","worktree":{"path":"/w/repo/task-other-11111111"}},
            {"branch":"task/branch-only-22222222"}
        ]}"#).unwrap();
        let found = find_task_worktree(&list, "7cd9fd3a-0000").unwrap();
        assert_eq!(found.branch, "task/old-name-7cd9fd3a");
        assert_eq!(found.path, PathBuf::from("/w/repo/task-old-name-7cd9fd3a"));
        assert!(find_task_worktree(&list, "22222222-0000").is_none(), "a branch with no worktree is not one");
        assert!(find_task_worktree(&list, "99999999-0000").is_none());
    }

    /// Shape from worktrunk 0.79's `wt switch --format=json`.
    #[test]
    fn a_switch_result_gives_branch_and_path() {
        let v: Value = serde_json::from_str(
            r#"{"action":"created","branch":"task/x-1234abcd","path":"/w/repo/task-x-1234abcd"}"#).unwrap();
        let w = switch_result(&v).unwrap();
        assert_eq!((w.branch.as_str(), w.path.to_str().unwrap()), ("task/x-1234abcd", "/w/repo/task-x-1234abcd"));
    }

    #[test]
    fn arguments_are_quoted_for_the_panes_shell() {
        assert_eq!(sh_quote("my project"), "'my project'");
        assert_eq!(sh_quote("it's"), r#"'it'\''s'"#);
    }
```

- [ ] See them fail; implement per the spec's slug rule (ASCII alphanumerics kept, any other run → one `-`, trim, >40 → cut at last `-` within 40, empty → `task`; `uuid8` = first 8 chars lowercased). Why-doc-comment on each.
- [ ] `cargo test`, clippy. Commit: "Name a task's branch, agent and prompt, and find its worktree".

### Task 4: Launch, set up here, and the menu row

**Files:** Modify `src/work.rs` (I/O), `src/main.rs` (command + menu), `README.md` (commands block, keybind row, menu mention).

- [ ] `work::launch(workspace, uuid, description)`:
  1. `repo = session::start_dir(home, workspace)`; `ensure!(repo.join(".git").exists(), "{repo} is not a git repository, so there is no worktree to make.")`; `ensure!(project::on_path("wt"), "worktrunk (wt) is not installed.")`.
  2. `s = herdr_session_name(workspace)`; `list = refine::open_session(&repo, &s)?`.
  3. If `find_task_worktree(&wt_list(&repo)?, uuid)` → `opened = run(worktree_open(...))`; if `agent_get(work-<uuid8>)` ok → `agent_focus`, `notify::tasks("Back to its worktree.")`, return; else `tab_create(&s, opened_workspace_id, &path, "Claude")` → `start_claude(...)`.
  4. Else: project workspace id from `first_workspace_id(&list)` (or `workspace_create(&s, &repo, workspace)`), `tab_create(.., &repo, "Start: <description ≤30 chars…>")`, then `pane_run(&s, &pane, "<sh_quote(current_exe)> task start --here --workspace <sh_quote(workspace)> <uuid>")`.
- [ ] `work::set_up_here(workspace, uuid)`: inside the tab. Re-read the task; `find_task_worktree` again (a retry after a half-finished run); else run `wt -C <repo> switch --create <branch> --no-cd --format=json` with stdin and stderr inherited (so approval can be answered and hooks are seen), stdout captured → `switch_result`. Then `worktree_open` → `root_pane_id` → `start_claude`. On success close own tab (`HERDR_TAB_ID`, best effort). On any error print it with "Press Enter to close this tab.", wait for a line, return the error.
- [ ] `start_claude(s, name, pane, uuid, workspace)`: `agent_start_claude`, `agent_prompt(plan_prompt)` (context "Claude started, but the prompt was not delivered."), `task::set_active(&tag::workspace_tag(workspace), uuid)`.
- [ ] `wt_list(repo)`: `wt -C <repo> list --format=json`, parsed.
- [ ] main.rs: `TaskCommand::Start { uuid, #[arg(long)] here: bool, #[arg(long)] workspace: Option<String> }`. `--here` → `work::set_up_here(ws, uuid)`. Otherwise: `require_workspace_tag()?`, focused workspace name, task must exist and be pending, `work::launch`. Menu: add "Start working" after "Grill me", `.lines(7)`, arm → `TaskCommand::Start { uuid: selected, here: false, workspace: None }`.
- [ ] README: `niritasks task start <uuid>` line in Commands; add "start working in its own worktree" to the `Mod+Alt+Ctrl+T` row; requirements note that Start working needs worktrunk.
- [ ] `cargo build` no warnings, `cargo test`, clippy. Commit: "Start working on a task in its own worktree from its menu".

### Task 5: Live check

- [ ] Don't create a throwaway project under `~/Projects` (the project picker would list it). Instead:
  - (a) Run the exact sequence the code builds — `wt switch --create`, `herdr worktree open`, `agent start`, `agent prompt` — by hand against a scratch repo and a throwaway herdr session, then delete both.
  - (b) `./install.sh`.
  - (c) Hand the menu path to the user: Start working on a throwaway task in a real project; check the worktree under `~/.worktrees/<repo>/`, the grouped herdr workspace, Claude running writing-plans, and the task's ▶; pick again → back to it; then remove the worktree with the plugin's remove key and delete the task.
