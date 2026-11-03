# Stop e2e-panel's peek --focused Check Flaking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tests/e2e-panel.sh`'s two `task peek --focused` checks pass every time in `tests/container.sh`, and when one fails it says why.

**Architecture:** Make every failure say why first. Then find the cause from that output and fix it in the shared code, not with longer sleeps. Two silent paths turn a failed Taskwarrior read into "no task". `task::export_values` ignores `task`'s exit status, and `link::focused_task` drops a `task::get` error with `.ok()`. Both become real errors: `task peek --focused` reports them on stderr and exits non-zero, and the daemon's switch path logs them and still treats them as no task. The e2e keeps each CLI call's stderr and turns on the daemon's debug trace, so the next flake prints its cause.

**Tech Stack:** Rust (anyhow), Taskwarrior 3.5.0, bash + Pillow e2e in a nested niri, Docker (`tests/container.sh`).

**Spec:** Taskwarrior task `c6c9e88d-9f50-462c-9a9e-e55af90214f4`, its description and notes (`task rc.json.array=on c6c9e88d export`). There is no separate spec file. The evidence is in `~/.local/share/niri-tasks/reviews/e2e-panel-peek-focused-flake-20261103/`.

## What was found (2026-11-04, while planning)

- **`export_values` cannot report a failed `task`.** `src/task.rs` `export_values` never checks `out.status`. A `task` that fails (a locked or unreadable store, say) prints nothing on stdout, and that is returned as `Ok(vec![])`. Its doc comment ("`export` exits non-zero when the filter matches nothing") describes 2.6.2. On 3.5.0, the version on the machine and in the container, every empty filter exits 0. Checked: `zz +WAITING`, `+nosuchtag status:completed`, `1234abcd`, `+nosuchtag +BLOCKED status:pending` all exit 0 with `[\n]`. `export_all`, a few lines down, already does `ensure!(out.status.success(), …)`. So the `.ok()` in `focused_task` alone would change nothing: `task::get` returns `Ok(None)`, not `Err`.
- **The stub log rules out the steps before the lookup.** `herdr-stub.log` lines 8–11 show both CLI calls ran `pane list` and `agent get`. `src/main.rs` `TaskCommand::Peek` runs `session::current_pane()` and `Workspace::of_session()` *before* `focused_task`. So the second call ("on no task failed") got past the lookup. With an empty focus, `focused_uuid8` returns None and `task::get` is never called. Only `ipc::send` (`NO_DAEMON`) is left after it, or a crash. The daemon was alive: the next check, `focused_none`, passed against it.
- **Daemon errors are safe to surface.** The tick, `cards_for_workspace` (`src/daemon.rs`, around line 118), already turns a `listing` `Err` into no cards, which is what an empty `Ok` gives today. `src/daemon.rs:379` matches `Ok(Some(t))` only. No caller of `export`/`export_values` needs a failed `task` to mean "empty".
- `debug()` in `src/daemon.rs` prints `[daemon] …` to stderr only under `NIRITASKS_DEBUG`. The herdr-stub daemon's stderr already goes to `$SB/daemon-herdr.err`.

## Global Constraints

- Diagnose before fixing. Fix the root cause the output names, in the shared code (`focused_task`, `task::export_values` or the CLI path), never with longer sleeps in the test.
- A task lookup that errors is never a silent clear. `task peek --focused` reports it on stderr and exits non-zero. A pane on no task still peeks at nothing and exits 0.
- The daemon's switch path (`refresh_peeks`), which shares `focused_task`, still treats an error as no task.
- Out of scope: reworking `settle` into polling for the whole suite, and other e2e-panel checks unless the same cause is shown to hit them.
- Done when: e2e-panel passes in `tests/container.sh` repeatedly (5 runs of `bash tests/container.sh bash tests/e2e-panel.sh`, plus one full `bash tests/container.sh`), and a failing `task peek --focused` prints why.
- Commits: Conventional Commits, scopes as the history uses them (`test(e2e)`, `fix(link)`, `fix(task)`).

---

### Task 1: Make the e2e's peek --focused failures say why

**Files:**
- Modify: `tests/e2e-panel.sh` (the herdr-stub daemon start near line 458, `check()` near line 446, the two `--focused` calls near lines 483–492)
- Modify: `src/daemon.rs` (`set_peek`, near line 432)

**Interfaces:**
- Produces: `$SB/peek-focused.err` and `$SB/peek-focused-none.err` (each CLI call's stderr). `[daemon] peek +<tag>: <uuid or None>` lines in `$SB/daemon-herdr.err` for every peek the daemon sets, whether from a request or from a switch.

- [ ] **Step 1: Trace every peek the daemon sets**

In `src/daemon.rs`, make `set_peek` log first:

```rust
fn set_peek(tag: &str, uuid: Option<&str>) {
    debug(&format!("peek +{tag}: {uuid:?}"));
    PEEKS.with(|p| remember_peek(&mut p.borrow_mut(), tag, uuid));
```

- [ ] **Step 2: Start the herdr-stub daemon with the trace on**

In `tests/e2e-panel.sh`, change the daemon start after `nested_daemon_stop`:

```bash
"${NENV[@]}" PATH="$STUB_PATH" NIRITASKS_DEBUG=1 "$NIRITASKS" daemon >"$SB/daemon-herdr.err" 2>&1 &
```

- [ ] **Step 3: Let `check` carry a reason**

Give `check` an optional fourth argument that is appended to its `bad` message:

```bash
check() {  # label frame-to-match message [why]
    shot "$1" || { summary; exit 1; }
    if same "$2" "$1"; then
        ok "$3"
    else
        read -r x0 x1 y0 y1 < <(measure "$1" "$2")
        bad "$3 — not so: columns ${x0}-${x1}, rows ${y0}-${y1} differ from $2${4:+
      $4}"
    fi
}
```

- [ ] **Step 4: Keep each CLI call's stderr and show it**

Replace the two `--focused` blocks with:

```bash
# --focused is what herdr's focus hooks run: the same lookup, from the pane.
# Each call's stderr is kept, and a failure shows it with the daemon's last
# peeks, so a flake says which side dropped the task.
peeks() { grep -F '[daemon] peek ' "$SB/daemon-herdr.err" | tail -n 4 | tr '\n' ';'; }
focus "work-${ship:0:8}"
"${NENV[@]}" PATH="$STUB_PATH" HERDR_SESSION=e2e "$NIRITASKS" task peek --focused >/dev/null 2>"$SB/peek-focused.err" ||
    bad "task peek --focused failed: $(tail -n 5 "$SB/peek-focused.err")"
settle
check focused_cli one "task peek --focused peeks at the focused pane's task" \
    "stderr: $(tail -n 5 "$SB/peek-focused.err") daemon: $(peeks)"
focus ""
"${NENV[@]}" PATH="$STUB_PATH" HERDR_SESSION=e2e "$NIRITASKS" task peek --focused >/dev/null 2>"$SB/peek-focused-none.err" ||
    bad "task peek --focused on no task failed: $(tail -n 5 "$SB/peek-focused-none.err")"
settle
check focused_none baseline "task peek --focused on a pane with no task leaves the edge empty" \
    "stderr: $(tail -n 5 "$SB/peek-focused-none.err") daemon: $(peeks)"
```

- [ ] **Step 5: Prove a failure now prints its reason**

Make the second call fail on purpose by pointing it at a socket nobody listens on. Change only its `"${NENV[@]}"` line, temporarily, to add `XDG_RUNTIME_DIR=/nonexistent`. Then run:

Run: `bash tests/container.sh bash tests/e2e-panel.sh`
Expected: `FAIL  task peek --focused on no task failed: Error: The niri-tasks daemon is not running …`. Revert the temporary change, run again, and expect every check to pass (the flake permitting).

- [ ] **Step 6: Commit**

```bash
git add tests/e2e-panel.sh src/daemon.rs
git commit -m "test(e2e): say why a task peek --focused check fails"
```

### Task 2: Stop a failed Taskwarrior read becoming a silent clear

**Files:**
- Modify: `src/task.rs` (`export` doc comment and `export_values`, near lines 414–454)
- Modify: `src/link.rs` (`focused_task`, near line 135, and its tests)
- Modify: `src/main.rs` (`TaskCommand::Peek`, near line 500)
- Modify: `src/daemon.rs` (`refresh_peeks`, near line 459)

**Interfaces:**
- Consumes: `crate::task::get(&str) -> Result<Option<Task>>`, `link::focused_uuid8`, `session::fake::Fake` (as in the existing link.rs tests).
- Produces: `pub fn focused_task(session: &Session) -> Result<Option<String>>`. `Ok(None)` means the pane is on no task, or on a uuid8 Taskwarrior does not know. `Err` means the lookup failed. Plus the private `fn focused_task_in(session, branch_in, get) -> Result<Option<String>>` for tests.

- [ ] **Step 1: Write the failing tests in `src/link.rs`'s `mod tests`**

```rust
    /// A focused pane on a task, with Taskwarrior asked by `get`.
    fn on_task() -> Session {
        let fake = Fake::running(&[("w1", "alpha")])
            .with_pane("w1:p1", true, "/work")
            .with_agent(Some("work-1234abcd"), "w1:p1", "idle");
        Session::with_port("alpha", PathBuf::new(), Box::new(fake))
    }

    /// A lookup that fails is an error naming the task, never a pane on no
    /// task: `task peek --focused` would clear the peek and exit 0.
    #[test]
    fn a_failed_task_lookup_is_an_error_not_no_task() {
        let err = focused_task_in(&on_task(), |_| None, |_| anyhow::bail!("database is locked")).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("1234abcd") && msg.contains("database is locked"), "{msg}");
    }

    #[test]
    fn the_focused_panes_task_is_looked_up_by_its_uuid8() {
        let found = focused_task_in(&on_task(), |_| None, |u| {
            assert_eq!(u, "1234abcd");
            Ok(Some(task(5, "1234abcd-0000-4000-8000-000000000000", "x")))
        });
        assert_eq!(found.unwrap().as_deref(), Some("1234abcd-0000-4000-8000-000000000000"));
        assert_eq!(focused_task_in(&on_task(), |_| None, |_| Ok(None)).unwrap(), None, "a branch that outlived its task");
    }

    /// A pane on no task asks Taskwarrior nothing, so it cannot fail.
    #[test]
    fn a_pane_on_no_task_is_no_task_without_asking() {
        let s = Session::with_port("alpha", PathBuf::new(), Box::new(Fake::running(&[("w1", "alpha")]).with_pane("w1:p1", true, "/")));
        assert_eq!(focused_task_in(&s, |_| None, |_| panic!("asked Taskwarrior")).unwrap(), None);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test link::tests`
Expected: compile error, `cannot find function focused_task_in`.

- [ ] **Step 3: Make `focused_task` return the error**

In `src/link.rs`, replace `focused_task` with:

```rust
/// The full uuid of the task `session`'s focused pane is on, as
/// [`focused_uuid8`] finds it, asking git for the folder's branch. None too
/// for a uuid8 Taskwarrior does not know: a branch that outlived its task.
/// A Taskwarrior that fails is an error, never no task: the caller decides
/// what a failed lookup means.
pub fn focused_task(session: &Session) -> Result<Option<String>> {
    focused_task_in(session, crate::work::branch_in, crate::task::get)
}

/// [`focused_task`] with the branch reader and the task lookup handed in.
fn focused_task_in(
    session: &Session,
    branch_in: impl Fn(&Path) -> Option<String>,
    get: impl Fn(&str) -> Result<Option<crate::task::Task>>,
) -> Result<Option<String>> {
    let Some(uuid8) = focused_uuid8(session, branch_in) else {
        return Ok(None);
    };
    let task = get(&uuid8).with_context(|| format!("Could not look up task {uuid8}, the focused pane's"))?;
    Ok(task.map(|t| t.uuid))
}
```

- [ ] **Step 4: Run the link tests**

Run: `cargo test link::tests`
Expected: the three new tests pass. The build fails in `main.rs` and `daemon.rs` until Step 5, so if `cargo test` will not build, do Step 5 first and then run this.

- [ ] **Step 5: Update both callers**

`src/main.rs`, in `TaskCommand::Peek`:

```rust
            let uuid = if focused {
                link::focused_task(&session::Session::named(&pane.session))?
            } else {
```

`src/daemon.rs`, in `refresh_peeks`'s thread. A failed lookup is still no task there, and the trace says why:

```rust
        std::thread::spawn(move || {
            let tag = ws.tag().to_string();
            let uuid = crate::link::focused_task(&crate::session::Session::named(&ws.session_name()))
                .unwrap_or_else(|e| {
                    debug(&format!("focused pane of +{tag}: {e:#}"));
                    None
                });
            debug(&format!("focused pane of +{tag}: {uuid:?}"));
            gtk4::glib::idle_add_once(move || set_peek(&tag, uuid.as_deref()));
        });
```

- [ ] **Step 6: Make a failed `task export` an error**

In `src/task.rs`, fix `export`'s doc comment, which describes 2.6.2:

```rust
/// Run a `task export` filter and parse the result. A filter that matches
/// nothing exits 0 with an empty list on Taskwarrior 3; a `task` that
/// fails is an error, never no tasks.
```

In `export_values`, right after `.output()…?;`, check the status the same way `export_all` does:

```rust
    anyhow::ensure!(
        out.status.success(),
        "`task export` failed: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
```

Keep the empty-stdout `Ok(Vec::new())` branch below it.

- [ ] **Step 7: Run the whole Rust suite**

Run: `cargo test`
Expected: all pass. `cargo build` gives no warnings.

- [ ] **Step 8: Commit**

```bash
git add src/task.rs src/link.rs src/main.rs src/daemon.rs
git commit -m "fix(link): report a failed task lookup from task peek --focused"
```

Body: `task export` failing used to read as no tasks, and `focused_task` dropped the error, so `task peek --focused` cleared the peek and exited 0. The CLI now exits non-zero with the reason. The daemon's switch path logs it and still peeks at nothing.

### Task 3: Run until it fails, fix the cause it names, prove it holds

**Files:** whatever the cause is in. This depends on the output. See the table below.

- [ ] **Step 1: Run e2e-panel alone, five times**

Run: `for i in 1 2 3 4 5; do bash tests/container.sh bash tests/e2e-panel.sh 2>&1 | tail -n 3; done`
Each run keeps its log. Note each `FAIL` line in full.

- [ ] **Step 2: Run the full suite once**

Run: `bash tests/container.sh`
Expected: every suite passes. The first flake came only under a full, loaded run.

- [ ] **Step 3: If a peek --focused check failed, read its reason and fix that cause**

| The failure prints | Cause | Fix |
|---|---|---|
| `Could not look up task …: \`task export\` failed: <Taskwarrior error>` | Taskwarrior failed under load. Read its message: lock, busy or I/O. | Stop and bring the message to the user with a proposed fix in `src/task.rs`'s read path. The fix depends on the error Taskwarrior gives, and a retry there would change every read. |
| `Error: The niri-tasks daemon is not running …` | `ipc::send` could not connect: the daemon's socket was briefly missing or refused. | Check `$SB/daemon-herdr.err` and the timestamps against `nested_daemon_stop`/`start` around it. Whatever is removing or rebinding the socket is the fix. |
| exit 0, the edge empty, `daemon:` shows `peek +e2e: Some(…)` then `peek +e2e: None` | A later `set_peek(None)` overwrote the CLI's peek: a late `refresh_peeks` thread, say. | Fix the ordering in `src/daemon.rs`, for example by not letting a refresh started before a request overwrite it. Add a unit test next to `a_peek_is_kept_per_workspace_tag`. |
| exit 0, the edge empty, the `daemon:` trace has no `Some` peek | The request never reached `set_peek`. | Trace `ipc::listen` → `Request::decode` with a temporary `debug` and repeat. |
| Something else | — | Stop and report it to the user with the log. |

Any fix that changes `src/link.rs` adds a unit test there, alongside the tests from Task 2. After a fix, commit it as `fix(<scope>): <what>`, then repeat Steps 1–2.

- [ ] **Step 4: If nothing failed, keep going until the done condition holds**

Done means five clean runs of `bash tests/container.sh bash tests/e2e-panel.sh` plus one clean full `bash tests/container.sh`, all with this branch's diagnostics in. Report the runs: how many and which passed. If none failed, say plainly that the flake did not recur. The surfaced error path from Task 2 is then the fix in place, and the next flake will print its cause.
