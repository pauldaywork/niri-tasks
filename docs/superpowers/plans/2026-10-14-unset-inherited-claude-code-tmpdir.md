# Unset an Inherited CLAUDE_CODE_TMPDIR in Refine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A refine always gets its private temp folder (`session_tmp` under `refine-tmp`), even when `CLAUDE_CODE_TMPDIR` is set in the environment it starts from.

**Architecture:** Refine moves Claude Code's temp folder by setting `TMPDIR` only. Claude Code 2.1.296 prefers `CLAUDE_CODE_TMPDIR` when `<path>/claude-<uid>` is at most 44 bytes, so an inherited one (from the daemon's or herdr's environment, or the pane shell's rc files) would override the private folder. Clear it on both ways a refine starts: `env_remove` on the unattended child process, and `unset` in the pane's shell line, which runs after the rc files.

**Tech Stack:** Rust 2021, `std::process::Command`, `cargo test`.

**Spec:** Taskwarrior task `9d4dead3-d3ae-4fcf-ad58-9ea617c9b43f` (description and notes; read with `task rc.json.array=on 9d4dead3 export`). Background: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` (lines ~105–120).

## Global Constraints

- Clear `CLAUDE_CODE_TMPDIR` on both start paths: `run_claude` in `src/refine.rs` and `export_tmpdir` in `src/session/herdr.rs`.
- The pane line is exactly `unset CLAUDE_CODE_TMPDIR; export TMPDIR='…'` (path single-quoted, `'` escaped as `'\''`).
- The limit Claude Code applies is 44 bytes for `<path>/claude-<uid>` (ADR 0002), not 32.
- Out of scope: setting `CLAUDE_CODE_TMPDIR` ourselves instead of `TMPDIR` (rejected in ADR 0002: too short for the cache paths).
- Commits: Conventional Commits, type `fix`, scope `refine`; end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

---

### Task 1: Clear CLAUDE_CODE_TMPDIR for the unattended run

**Files:**
- Modify: `src/refine.rs:310-320` (`session_tmp` doc)
- Modify: `src/refine.rs:789-798` (`run_claude` doc and command)
- Test: `src/refine.rs:1335-1340` (in `a_run_logs_its_result_and_denials_under_its_step_and_fails_on_a_bad_exit`)

**Interfaces:**
- Consumes: nothing new.
- Produces: no signature changes. `run_claude(project, log, step, argv, tmp, denials)` now starts its child with `TMPDIR=tmp` and without `CLAUDE_CODE_TMPDIR`.

- [ ] **Step 1: Write the failing test**

In `src/refine.rs`, replace the `TMPDIR` echo block of `a_run_logs_its_result_and_denials_under_its_step_and_fails_on_a_bad_exit`:

```rust
        // The session's own temp folder reaches Claude through its
        // environment, since the settings' `env` does not move it.
        let echo = ["sh", "-c", "echo \"$TMPDIR\""].map(String::from);
        run_claude(&dir, &log, Step::Refine, &echo, tmp, &mut denials).unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("\n/home/x/.cache/niri-tasks/refine-tmp/d9f76b94\n"), "{text}");
```

with:

```rust
        // The session's own temp folder reaches Claude through its
        // environment, since the settings' `env` does not move it, and an
        // inherited `CLAUDE_CODE_TMPDIR`, which Claude would prefer, is
        // cleared.
        std::env::set_var("CLAUDE_CODE_TMPDIR", "/tmp/x");
        let echo = ["sh", "-c", "echo \"$TMPDIR|$CLAUDE_CODE_TMPDIR\""].map(String::from);
        let ran = run_claude(&dir, &log, Step::Refine, &echo, tmp, &mut denials);
        std::env::remove_var("CLAUDE_CODE_TMPDIR");
        ran.unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.ends_with("\n/home/x/.cache/niri-tasks/refine-tmp/d9f76b94|\n"), "{text}");
```

(`set_var` is safe in edition 2021 and other tests already use it, e.g. `src/ipc.rs:201`. Nothing else in the crate reads `CLAUDE_CODE_TMPDIR`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test a_run_logs_its_result_and_denials`
Expected: FAIL — the log ends with `…/d9f76b94|/tmp/x`.

- [ ] **Step 3: Write minimal implementation**

In `run_claude`, add the removal beside `TMPDIR`:

```rust
        .current_dir(project)
        .env("TMPDIR", tmp)
        .env_remove("CLAUDE_CODE_TMPDIR")
        .stdin(Stdio::null())
```

Update `run_claude`'s doc, replacing its last three lines:

```rust
/// `tmp` is the session's own temp folder ([`session_tmp`]), set as
/// `TMPDIR`, which is the only way it reaches Claude Code, with any
/// inherited `CLAUDE_CODE_TMPDIR` cleared (see [`session_tmp`]).
```

Fix `session_tmp`'s second paragraph (it said 32 bytes; ADR 0002 measured 44):

```rust
/// It is set as `TMPDIR`, not `CLAUDE_CODE_TMPDIR`: Claude Code ignores a
/// `CLAUDE_CODE_TMPDIR` whose `<path>/claude-<uid>` is over 44 bytes and
/// falls back to `/tmp`, which both shares the user's sessions' folder
/// and hides the sandbox's own proxy socket. With `TMPDIR` there is no
/// length limit (Claude uses `<TMPDIR>/claude-<uid>`). But Claude prefers
/// a `CLAUDE_CODE_TMPDIR` short enough to use, so one inherited from the
/// daemon, herdr or the pane's shell is cleared wherever `TMPDIR` is set.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test a_run_logs_its_result_and_denials`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/refine.rs
git commit -m "fix(refine): clear an inherited CLAUDE_CODE_TMPDIR when unattended

Claude Code prefers CLAUDE_CODE_TMPDIR over TMPDIR when it is short
enough, so one inherited from the daemon's environment put an unattended
refine back in a shared temp folder. Also correct session_tmp's doc: the
limit ADR 0002 measured is 44 bytes, not 32.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Unset CLAUDE_CODE_TMPDIR in the refine pane's shell line

**Files:**
- Modify: `src/session/herdr.rs:105-111` (`export_tmpdir` and its doc)
- Modify: `src/session.rs:187-188` (`Claude::Refiner::tmp` doc)
- Modify: `src/refine.rs:183-188` (`session_settings`'s doc, the paragraph on `$TMPDIR`)
- Test: `src/session/herdr.rs:724-733` (`the_temp_folder_is_exported_quoted`)
- Test: `src/session.rs:990` (`a_clean_start_starts_once_and_waits_for_nothing`)
- Test: `src/refine.rs:1531` (`refine_holds_the_session_from_opening_it_until_claude_has_started`)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `pub fn export_tmpdir(tmp: &Path) -> String` keeps its name and signature; it now returns `unset CLAUDE_CODE_TMPDIR; export TMPDIR='<tmp>'`. Its one caller is `src/session.rs:553`, unchanged.

- [ ] **Step 1: Write the failing tests**

In `src/session/herdr.rs`, replace `the_temp_folder_is_exported_quoted` with:

```rust
    /// A refine's temp folder is exported in its pane's shell, quoted so a
    /// `'` in the path cannot end the string early, after unsetting any
    /// `CLAUDE_CODE_TMPDIR` the shell has, which Claude would prefer.
    #[test]
    fn the_temp_folder_is_exported_quoted() {
        assert_eq!(
            export_tmpdir(Path::new("/home/x/.cache/niri-tasks/refine-tmp/d9f76b94")),
            "unset CLAUDE_CODE_TMPDIR; export TMPDIR='/home/x/.cache/niri-tasks/refine-tmp/d9f76b94'"
        );
        assert_eq!(
            export_tmpdir(Path::new("/c/a'b/t")),
            "unset CLAUDE_CODE_TMPDIR; export TMPDIR='/c/a'\\''b/t'"
        );
    }
```

In `src/session.rs` (`a_clean_start_starts_once_and_waits_for_nothing`), change the expected pane line:

```rust
        assert_eq!(log[start - 1], "pane_run alpha p1 unset CLAUDE_CODE_TMPDIR; export TMPDIR='/c/tmp'", "{log:?}");
```

In `src/refine.rs` (`refine_holds_the_session_from_opening_it_until_claude_has_started`), change the line looked up:

```rust
            at(&log, "pane_run alpha p1 unset CLAUDE_CODE_TMPDIR; export TMPDIR='/c/tmp'") + 1,
```

- [ ] **Step 2: Run tests to verify they fail**

`cargo test` takes one name filter, so run them one at a time:

```bash
cargo test the_temp_folder_is_exported_quoted
cargo test a_clean_start_starts_once_and_waits_for_nothing
cargo test refine_holds_the_session_from_opening_it
```

Expected: all three FAIL (the first two on a mismatched string; the third panics in `at` because the line is not in the log).

- [ ] **Step 3: Write minimal implementation**

In `src/session/herdr.rs`:

```rust
/// The shell line that sets a refine's temp folder in its pane before
/// Claude starts there: herdr's `agent start` launches Claude in the
/// pane's own shell, which then passes the variable on. It first unsets
/// `CLAUDE_CODE_TMPDIR`, which Claude would prefer to `TMPDIR`; the line
/// runs after the shell's rc files, so this clears one exported there
/// too. Single-quoted, with any `'` in the path closed, escaped and
/// reopened.
pub fn export_tmpdir(tmp: &Path) -> String {
    format!("unset CLAUDE_CODE_TMPDIR; export TMPDIR='{}'", tmp.display().to_string().replace('\'', "'\\''"))
}
```

In `src/session.rs`, the `Refiner` field doc:

```rust
        /// The session's own temp folder, exported as `TMPDIR`, with
        /// `CLAUDE_CODE_TMPDIR` unset, in the pane's shell before Claude
        /// starts.
        tmp: PathBuf,
```

In `src/refine.rs`, the `session_settings` doc lines 183–188, replace:

```rust
/// folder ([`session_tmp`]), not the temp folder the user's unsandboxed
/// Claude sessions share. It is not set here: the settings' `env` does not
/// reach Claude Code's temp folder (measured 2026-10-12), so
/// `TMPDIR` is exported in the tab's shell before Claude starts
/// ([`Claude::Refiner`]'s `tmp`) and set on an unattended run's process.
```

with:

```rust
/// folder ([`session_tmp`]), not the temp folder the user's unsandboxed
/// Claude sessions share. It is not set here: the settings' `env` does not
/// reach Claude Code's temp folder (measured 2026-10-12), so
/// `TMPDIR` is exported in the tab's shell before Claude starts
/// ([`Claude::Refiner`]'s `tmp`) and set on an unattended run's process,
/// with `CLAUDE_CODE_TMPDIR` cleared in both.
```

Do not run `cargo fmt`: the repo is not rustfmt-clean, and formatting would touch unrelated code. Match the surrounding hand formatting.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test the_temp_folder_is_exported_quoted
cargo test a_clean_start_starts_once_and_waits_for_nothing
cargo test refine_holds_the_session_from_opening_it
```

Expected: PASS

- [ ] **Step 5: Run the whole suite**

Run: `cargo test`
Expected: all pass. Also `grep -rn "export TMPDIR=" src tests` must show no expectation still lacking the `unset` prefix.

- [ ] **Step 6: Commit**

```bash
git add src/session/herdr.rs src/session.rs src/refine.rs
git commit -m "fix(refine): unset CLAUDE_CODE_TMPDIR in the refine pane

The pane's shell can hand Claude a CLAUDE_CODE_TMPDIR from herdr's
environment or its own rc files, and Claude prefers it over the TMPDIR
refine exports. Unset it in the same line, which runs after the rc
files.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Verify against a real refine

No code. The sandboxed e2e suites cannot run a Claude-driven refine (claude is off their PATH and their TASKRC is under the fenced `/tmp`), so the real check is on the desktop.

- [ ] **Step 1: Run the full local suites from the worktree**

Run: `NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh`
Expected: every suite that runs passes (skips are fine).

- [ ] **Step 2: Real pane refine, with CLAUDE_CODE_TMPDIR in the daemon's environment**

This needs the build running as the user's daemon, so do it after landing (finish-worktree), or ask the user to do it then:

1. Stop the running daemon and start the built one with the variable set:
   `CLAUDE_CODE_TMPDIR=/tmp/x niritasks daemon` (from a terminal, in place of the usual one).
2. Add a scratch task (`niritasks task add "chore: Scratch refine tmp check"`) and press Refine on it in the panel.
3. In the refine session, ask: `Run: echo "$TMPDIR"`.
4. Expected: a path under `~/.cache/niri-tasks/refine-tmp/<first 8 of the scratch uuid>/`, not `/tmp/x/…` or `/tmp/claude-<uid>`.
5. Close the session, delete the scratch task, restart the daemon normally.

- [ ] **Step 3: Unattended refine, same check**

`CLAUDE_CODE_TMPDIR=/tmp/x niritasks task refine <scratch-uuid> --unattended`, then confirm in the run's log (or `ls ~/.cache/niri-tasks/refine-tmp/<uuid8>/`) that Claude's `claude-<uid>` folder was made there and nothing new appeared under `/tmp/x`.
