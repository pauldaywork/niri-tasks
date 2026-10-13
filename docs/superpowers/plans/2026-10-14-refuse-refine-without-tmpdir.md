# Refuse a Refine Tab Without Its TMPDIR Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Refine tab's Claude never starts unless the pane's shell has `TMPDIR` set to the session's own temp folder; otherwise the tab closes with an error saying so.

**Architecture:** `Session::start_claude` (src/session.rs) already types `export TMPDIR=…` into the pane with `pane_run`. After it, it types a second line that prints a marker only when `$TMPDIR` equals the session's temp folder, then waits for that marker with herdr's `pane wait-output` (10s). A timeout or any herdr failure returns an error before `agent_start`; refine's `launch_in` already closes the tab on that error. Workers get no check.

**Tech Stack:** Rust, herdr 0.9.3 CLI (`herdr pane wait-output`), the session module's `Port` trait with its `Process` adapter and test `Fake`.

**Spec:** Taskwarrior task `14602b4e-035d-40b1-a908-e165b721e8bb` (its description and notes); background in `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, addendum 2026-10-12.

## Global Constraints

- Check in the pane's shell before Claude starts, not in the refiner (the refine-task skill or the mod): a check from inside the session runs after Claude has bound its proxy socket and temp folder.
- The marker is built at runtime so the echoed command line cannot match it. Verified on herdr 0.9.3: `wait-output --match X` matches its own command text as echoed on screen, so a literal marker in the typed line would always "pass".
- Wait timeout: about 10s (`10000` ms).
- Workers (`Claude::Worker`) get no export and no check.
- Unattended runs are out of scope (they set `TMPDIR` on the `claude -p` process directly).
- herdr's timeout answer, as probed: exit 1, stderr `{"error":{"code":"timeout","message":"timed out waiting for output match"},"id":"cli:pane:wait-output"}`.
- Commit messages: Conventional Commits, `fix(refine): …` / `refactor(session): …`, subject ≤72 chars, ending with the `Co-Authored-By` trailer from the session.

## File Structure

- `src/session/herdr.rs` — argv builder `pane_wait_output`, shell-line builder `check_tmpdir`, marker const `TMPDIR_SET`, a shared `shell_quote` helper, and `Process::pane_wait_output`. Pure builders are unit-tested here.
- `src/session/fake.rs` — `Fake::pane_wait_output`, logged; `Fake::with_tmpdir_unset()` makes it time out.
- `src/session.rs` — `Port::pane_wait_output`, the `TMPDIR_WAIT` const, and the check in `start_claude`; its tests.
- `src/refine.rs` — tests only: the refine order and the tab closing on a failed check.

---

### Task 1: herdr's `pane wait-output` behind the port, and the check line

**Files:**
- Modify: `src/session/herdr.rs` (builders near `export_tmpdir` at ~line 104 and `pane_run` at ~line 234; `impl Port for Process` at ~line 348; tests near `the_temp_folder_is_exported_quoted` at ~line 724)
- Modify: `src/session.rs:232` (the `Port` trait)
- Modify: `src/session/fake.rs` (State, a `with_*` builder, `impl Port for Fake`)

**Interfaces:**
- Produces (in `src/session/herdr.rs`):
  - `pub const TMPDIR_SET: &str = "niri-tasks-tmpdir-set";`
  - `pub fn check_tmpdir(tmp: &Path) -> String`
  - `pub fn pane_wait_output(session: &str, pane: &str, text: &str, timeout: Duration) -> Vec<String>`
  - `fn shell_quote(s: &str) -> String` (private, used by `export_tmpdir` and `check_tmpdir`)
- Produces (in `src/session.rs`, trait `Port`):
  - `fn pane_wait_output(&self, session: &str, pane: &str, text: &str, timeout: Duration) -> HerdrResult<()>;`
- Produces (in `src/session/fake.rs`):
  - `pub fn with_tmpdir_unset(self) -> Fake` — every `pane_wait_output` then answers herdr's `timeout`.
  - Log line: `pane_wait_output {session} {pane} {text} {ms}ms`, e.g. `pane_wait_output alpha p1 niri-tasks-tmpdir-set 10000ms`.

- [ ] **Step 1: Write the failing tests in `src/session/herdr.rs`**

Add to the `tests` module, after `the_temp_folder_is_exported_quoted`:

```rust
    /// The check prints the marker only when `$TMPDIR` is the folder, and
    /// the line typed never holds the marker itself: herdr's `wait-output`
    /// searches the pane's screen, the echoed command included.
    #[test]
    fn the_temp_folder_check_prints_its_marker_only_when_set() {
        let tmp = Path::new("/c/a'b/t");
        let line = check_tmpdir(tmp);
        assert!(!line.contains(TMPDIR_SET), "{line}");
        let run = |tmpdir: Option<&str>| {
            let mut sh = std::process::Command::new("sh");
            sh.args(["-c", &line]).env_remove("TMPDIR");
            if let Some(t) = tmpdir {
                sh.env("TMPDIR", t);
            }
            String::from_utf8(sh.output().unwrap().stdout).unwrap()
        };
        assert_eq!(run(Some("/c/a'b/t")), format!("{TMPDIR_SET}\n"));
        assert_eq!(run(Some("/tmp")), "");
        assert_eq!(run(None), "");
    }

    #[test]
    fn waiting_for_pane_output_matches_text_with_a_timeout() {
        assert_eq!(
            pane_wait_output("alpha", "w1:p4", TMPDIR_SET, Duration::from_secs(10)),
            vec![
                "herdr", "--session", "alpha", "pane", "wait-output", "--match", "niri-tasks-tmpdir-set",
                "--timeout", "10000", "w1:p4",
            ]
        );
    }
```

If the test module does not already import `Duration`, it gets it through `use super::*;` (the file imports `std::time::{Duration, Instant}` at the top); check and add `use std::time::Duration;` to the test module only if the compiler asks.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib session::herdr::tests 2>&1 | tail -20`
Expected: compile errors — `cannot find function check_tmpdir`, `TMPDIR_SET`, `pane_wait_output`.

- [ ] **Step 3: Add the builders in `src/session/herdr.rs`**

Replace `export_tmpdir` (~line 104-111) with:

```rust
/// The shell line that sets a refine's temp folder in its pane before
/// Claude starts there: herdr's `agent start` launches Claude in the
/// pane's own shell, which then passes the variable on.
pub fn export_tmpdir(tmp: &Path) -> String {
    format!("export TMPDIR={}", shell_quote(&tmp.display().to_string()))
}

/// What [`check_tmpdir`] prints once the pane's shell has the temp folder.
pub const TMPDIR_SET: &str = "niri-tasks-tmpdir-set";

/// The shell line, typed after [`export_tmpdir`], that prints
/// [`TMPDIR_SET`] only when `$TMPDIR` is `tmp`: `pane run` succeeding only
/// means herdr typed the export, not that the shell ran it. The marker is
/// put together by `printf` as it runs, because `wait-output` searches the
/// whole screen, and the echoed line would match a marker written out in it.
pub fn check_tmpdir(tmp: &Path) -> String {
    let (head, tail) = TMPDIR_SET.split_at("niri-tasks".len());
    format!(
        "[ \"$TMPDIR\" = {} ] && printf '%s%s\\n' {head} {tail}",
        shell_quote(&tmp.display().to_string())
    )
}

/// `s` single-quoted for a POSIX shell, with any `'` in it closed, escaped
/// and reopened.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
```

`head` is `niri-tasks`, `tail` is `-tmpdir-set`; the typed line holds `niri-tasks -tmpdir-set`, never the joined marker. The existing `the_temp_folder_is_exported_quoted` test pins `export_tmpdir`'s output unchanged.

After `pane_run` (~line 234), add:

```rust
/// Wait up to `timeout` for `text` to show in a pane's recent output,
/// including what is there already. A miss fails with herdr's `timeout`.
pub fn pane_wait_output(session: &str, pane: &str, text: &str, timeout: Duration) -> Vec<String> {
    let ms = timeout.as_millis().to_string();
    cmd(session, &["pane", "wait-output", "--match", text, "--timeout", &ms, pane])
}
```

- [ ] **Step 4: Put it on the port**

In `src/session.rs`, in `trait Port`, after `fn pane_run(…)` (line 232):

```rust
    /// Wait up to `timeout` for `text` in a pane's output.
    fn pane_wait_output(&self, session: &str, pane: &str, text: &str, timeout: Duration) -> HerdrResult<()>;
```

In `src/session/herdr.rs`, `impl Port for Process`, after `fn pane_run`:

```rust
    fn pane_wait_output(&self, s: &str, pane: &str, text: &str, timeout: Duration) -> HerdrResult<()> {
        self.call(&pane_wait_output(s, pane, text, timeout), |_| Ok(()))
    }
```

In `src/session/fake.rs`, add to `struct State` after `start: StartOutcome,`:

```rust
    /// Whether the pane's shell never gets the temp folder, so waiting for
    /// the check's marker times out.
    tmpdir_unset: bool,
```

Add after `with_start`:

```rust
    /// A pane whose shell never gets the temp folder: every wait for output
    /// times out, as herdr's does.
    pub fn with_tmpdir_unset(self) -> Fake {
        self.state.borrow_mut().tmpdir_unset = true;
        self
    }
```

And in `impl Port for Fake`, after `fn pane_run`:

```rust
    fn pane_wait_output(&self, session: &str, pane: &str, text: &str, timeout: Duration) -> HerdrResult<()> {
        self.note(format!("pane_wait_output {session} {pane} {text} {}ms", timeout.as_millis()));
        if let Err(e) = self.check()? {
            return Ok(Err(e));
        }
        if self.state.borrow().tmpdir_unset {
            return Ok(Err(Self::refuse("timeout", "timed out waiting for output match")));
        }
        Ok(Ok(()))
    }
```

- [ ] **Step 5: Run the tests**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked|error"`
Expected: every `test result: ok`, the two new tests among them, no warnings about the fake's `with_tmpdir_unset` being unused that fail the build (it is `pub` on a test module type; if `cargo build` warns `dead_code`, leave it — Task 2 uses it).

- [ ] **Step 6: Commit**

```bash
git add src/session/herdr.rs src/session/fake.rs src/session.rs
git commit -m "refactor(session): add herdr's pane wait-output and a TMPDIR check" -m "The check line prints a marker only when the pane's shell has the
refine's temp folder, built at runtime so the echoed line cannot match
it. Nothing calls it yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Start a Refine's Claude only once its shell has TMPDIR

**Files:**
- Modify: `src/session.rs` (const near line 268; `start_claude` at ~line 547-556; tests at ~line 984-992 and new ones after them)
- Modify: `src/refine.rs` (tests at ~line 1530-1543 and a new one after `a_failed_refine_lets_go_and_closes_its_tab`)

**Interfaces:**
- Consumes: `herdr::check_tmpdir(&Path) -> String`, `herdr::TMPDIR_SET`, `Port::pane_wait_output(&self, &str, &str, &str, Duration) -> HerdrResult<()>`, `Fake::with_tmpdir_unset()`, fake log line `pane_wait_output alpha p1 niri-tasks-tmpdir-set 10000ms` (all from Task 1).
- Produces: `const TMPDIR_WAIT: Duration` in `src/session.rs`; `start_claude`'s error, whose `to_string()` reads `the Refine tab's shell did not set TMPDIR to <tmp> within 10s, so Claude was not started`.

- [ ] **Step 1: Write the failing tests in `src/session.rs`**

Replace `a_clean_start_starts_once_and_waits_for_nothing` (~line 983-992) with:

```rust
    /// A refiner's temp folder is exported, checked and waited for in its
    /// pane's shell, in that order, just before Claude starts there.
    #[test]
    fn a_clean_start_starts_once_and_waits_for_nothing() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]));
        let refiner = Claude::Refiner { settings: "{}".into(), mod_dir: PathBuf::from("/m"), tmp: PathBuf::from("/c/tmp") };
        s.start_claude(s.claim().unwrap(), "task-abc", "p1", &refiner).unwrap();
        let log = f.log();
        let start = log.iter().position(|l| l == "agent_start alpha task-abc p1 refiner").expect("started");
        assert_eq!(
            log[start - 3..start],
            [
                "pane_run alpha p1 export TMPDIR='/c/tmp'".to_string(),
                format!("pane_run alpha p1 {}", herdr::check_tmpdir(Path::new("/c/tmp"))),
                "pane_wait_output alpha p1 niri-tasks-tmpdir-set 10000ms".to_string(),
            ],
            "{log:?}"
        );
        assert!(!logged_start(&f, "agent_wait_ready"));
    }

    /// A shell that never shows the temp folder set is an error saying so,
    /// and Claude is never started in it.
    #[test]
    fn a_refiner_whose_shell_lacks_its_temp_folder_is_not_started() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_tmpdir_unset());
        let refiner = Claude::Refiner { settings: "{}".into(), mod_dir: PathBuf::from("/m"), tmp: PathBuf::from("/c/tmp") };
        let err = s.start_claude(s.claim().unwrap(), "task-abc", "p1", &refiner).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the Refine tab's shell did not set TMPDIR to /c/tmp within 10s, so Claude was not started"
        );
        assert!(logged(&f, "pane_wait_output alpha p1 niri-tasks-tmpdir-set 10000ms"), "{:?}", f.log());
        assert!(!logged_start(&f, "agent_start"), "{:?}", f.log());
        assert_eq!(f.log().last().map(String::as_str), Some("release alpha"));
    }

    /// A worker runs as the user runs Claude: nothing is typed or waited
    /// for in its shell first.
    #[test]
    fn a_worker_is_started_without_a_temp_folder_check() {
        let (s, f) = session(Fake::running(&[("w1", "alpha")]).with_tmpdir_unset());
        s.start_claude(s.claim().unwrap(), "work-abc", "p1", &Claude::Worker { settings: "{}".into() }).unwrap();
        assert!(!logged_start(&f, "pane_run"), "{:?}", f.log());
        assert!(!logged_start(&f, "pane_wait_output"), "{:?}", f.log());
        assert!(logged(&f, "agent_start alpha work-abc p1 worker"));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib session::tests 2>&1 | grep -E "^test |panicked|test result"`
Expected: `a_clean_start_starts_once_and_waits_for_nothing` and `a_refiner_whose_shell_lacks_its_temp_folder_is_not_started` FAIL (no check typed, no wait logged; Claude started anyway); `a_worker_is_started_without_a_temp_folder_check` passes already.

- [ ] **Step 3: Implement the check in `start_claude`**

In `src/session.rs`, after `const SETUP_POLL` (~line 284), add:

```rust
/// How long a Refine tab's shell gets to show its temp folder set before
/// Claude would start without it.
const TMPDIR_WAIT: Duration = Duration::from_secs(10);
```

Replace the last paragraph of `start_claude`'s doc comment and its first block (~line 547-554):

```rust
    /// A refiner's temp folder is exported in the pane's shell first, here
    /// rather than in the herdr port so the fake logs it too: herdr starts
    /// Claude in that same shell, which passes the variable on. `pane run`
    /// only types the line, so a second one prints a marker once the shell
    /// has the folder, and Claude starts only when that shows up: a Claude
    /// started without it falls back to `/tmp/claude-<uid>`, the temp folder
    /// the user's unsandboxed sessions share, and binds its proxy socket
    /// there before anything inside the session could check.
    pub fn start_claude(&self, claim: Claim, name: &str, pane: &str, claude: &Claude) -> Result<()> {
        if let Claude::Refiner { tmp, .. } = claude {
            self.port.pane_run(&self.name, pane, &herdr::export_tmpdir(tmp))??;
            self.port.pane_run(&self.name, pane, &herdr::check_tmpdir(tmp))??;
            self.port
                .pane_wait_output(&self.name, pane, herdr::TMPDIR_SET, TMPDIR_WAIT)
                .and_then(|waited| Ok(waited?))
                .with_context(|| {
                    format!(
                        "the Refine tab's shell did not set TMPDIR to {} within {}s, so Claude was not started",
                        tmp.display(),
                        TMPDIR_WAIT.as_secs()
                    )
                })?;
        }
```

The rest of `start_claude` (from `let started = self.port.agent_start(…)`) is unchanged. On the early return `claim` is dropped, which is the `release alpha` the new test expects last.

- [ ] **Step 4: Run the session tests**

Run: `cargo test --lib session::tests 2>&1 | grep -E "panicked|test result"`
Expected: `test result: ok`.

- [ ] **Step 5: Update refine's order test and add the closing-tab test in `src/refine.rs`**

In `refine_holds_the_session_from_opening_it_until_claude_has_started` (~line 1530), replace the `assert_eq!(at(&log, "pane_run alpha p1 export TMPDIR='/c/tmp'") + 1, …)` block with:

```rust
        let export = at(&log, "pane_run alpha p1 export TMPDIR='/c/tmp'");
        assert_eq!(
            log[export + 1..=export + 3].iter().map(|l| l.split(' ').next().unwrap()).collect::<Vec<_>>(),
            ["pane_run", "pane_wait_output", "agent_start"],
            "the temp folder is set and checked in the pane's shell just before Claude starts in it: {log:?}"
        );
```

After `a_failed_refine_lets_go_and_closes_its_tab`, add:

```rust
    /// A tab whose shell never shows its temp folder set is closed, with
    /// the error saying why, and no Claude is started or prompted in it.
    #[test]
    fn a_refine_whose_shell_lacks_its_temp_folder_closes_its_tab() {
        let fake = Fake::running(&[("w1", "alpha")]).with_tmpdir_unset();
        let err = refine(&fake).unwrap_err().to_string();
        assert!(err.contains("did not set TMPDIR"), "{err}");
        let log = fake.log();
        assert!(at(&log, "pane_wait_output alpha p1") < at(&log, "tab_close alpha"), "{log:?}");
        assert!(!log.iter().any(|l| l.starts_with("agent_start") || l.starts_with("agent_prompt")), "{log:?}");
        assert!(fake.agents().is_empty());
    }
```

- [ ] **Step 6: Run the whole suite and clippy**

Run: `cargo test 2>&1 | grep -E "test result|FAILED|panicked"` then `cargo clippy --all-targets 2>&1 | tail -5`
Expected: every `test result: ok`; clippy with no new warnings.

- [ ] **Step 7: Check it against a real herdr**

From a terminal inside herdr, in this worktree, build and run the check against a scratch pane so `wait-output` and the line are proved together:

```bash
cargo build --release
pane=$(herdr pane split "$HERDR_PANE_ID" --direction right --no-focus | jq -r '.result.pane.pane_id // .result.pane_id')
herdr pane run "$pane" "export TMPDIR='/tmp/x y'"
herdr pane run "$pane" "[ \"\$TMPDIR\" = '/tmp/x y' ] && printf '%s%s\\n' niri-tasks -tmpdir-set"
herdr pane wait-output --match niri-tasks-tmpdir-set --timeout 10000 "$pane" >/dev/null; echo "set: $?"
herdr pane run "$pane" "export TMPDIR=/tmp"
herdr pane run "$pane" "clear"
herdr pane run "$pane" "[ \"\$TMPDIR\" = '/tmp/x y' ] && printf '%s%s\\n' niri-tasks -tmpdir-set"
herdr pane wait-output --match niri-tasks-tmpdir-set --source visible --timeout 3000 "$pane" >/dev/null; echo "unset: $?"
herdr pane close "$pane"
```

Expected: `set: 0` and `unset: 1`. (If `pane split`'s JSON shape differs, read the id from `herdr pane list`.) Then press Refine on any task from the panel: the tab opens and Claude starts as before.

- [ ] **Step 8: Commit**

```bash
git add src/session.rs src/refine.rs
git commit -m "fix(refine): refuse to start Claude until its shell has TMPDIR" -m "herdr's pane run only types the export, so a Refine tab's Claude could
start before its shell ran it and fall back to /tmp/claude-<uid>, the
temp folder the user's unsandboxed sessions share. The tab now waits up
to 10s for the shell to show the session's temp folder set, and closes
with an error saying so when it does not. Workers are unchanged.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
