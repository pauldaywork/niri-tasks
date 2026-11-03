# Count Each Suite's Tests in all.sh's Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tests/all.sh`'s closing block gives, for each suite, its tests passed, failed and skipped (inner skips included) and its failing tests with their first line of reason, so a green run proves how much passed and a red one says what broke.

**Architecture:** all.sh's `suite()` moves to a new sourced file, `tests/lib/suites.sh`, with one awk parser per summary format (cargo, unittest, bun, e2e) and a `closing_block` function. `suite()` tees each suite's stdout and stderr to files of their own in a `mktemp -d` dir, keeps the suite's own exit status, and reads the files with that suite's parser. A Rust integration test, `tests/all_summary.rs`, sources the file and checks each parser and the runner's verdicts, so `cargo test` (and so every all.sh and container run) covers it. Then the two skills that read all.sh's text are updated.

**Tech Stack:** bash 5, mawk (`/usr/bin/awk` is mawk 1.3.4: no `gensub`, no `{n}` intervals), Rust integration tests (`std::process::Command`, no new crates).

**Spec:** Taskwarrior task `b39ff86c-8459-491b-ab01-dac0a67a774f` (its description and notes: `task rc.json.array=on b39ff86c export`). Background: `~/.local/share/niri-tasks/reviews/test-result-reporting-review-20261104.md`. The run to check against: `~/.local/share/niri-tasks/reviews/e2e-panel-peek-focused-flake-20261103/20261103T110452Z-485336.log`.

## Global Constraints

- Parse each suite's own output in all.sh (tee to a file per suite). No JUnit, TAP, nextest or unittest-xml-reporting.
- Closing block: one line per suite with its counts and duration (PASS and FAIL alike), failing tests indented under it, then one final line worded so suites and tests can't be confused: `suites: 5 of 6 passed, 1 failed, 0 skipped · tests: 1247 passed, 2 failed, 0 skipped`.
- A suite that exits without its summary is `FAIL <suite> — stopped before its summary: <last stderr line>`. e2e-tag's summary gains `skipped: 0`.
- The exit code stays the one gate: all.sh exits non-zero only when a suite that ran failed (non-zero exit, a failure in its summary, or no summary and a non-zero exit). Skips, counts it can't read and its own parse errors never change the exit code or stop the closing block printing. Unreadable counts show as such.
- A suite's exit code comes through its tee, never tee's.
- Keep in the log: the `── name ──` headers, the `$ <command>` line, `SKIP: <reason>` after a header, the `────` rule before the closing block, a `FAIL <suite>` line per failed suite in it, and the exit code. Indented failing-test lines must not start with `FAIL`, `PASS` or `──` (they start `      - `).
- Every reader of the text is updated in this branch: finish-worktree step 6's grep and its prose naming the summary line; report-branch step 3; branch-catalogue.md's `suite.runs` (and `suite.not_run`) rows. The rest of step 6 stays with task 92236866.
- The branch report gains no skipped-count field: `report.ts`, the page and the markdown are unchanged. Inner skips go in the existing `not_run` list (at most 6 entries, so one per suite).
- Out of scope: the rest of finish-worktree's step 6 (task 92236866), the e2e-panel `peek --focused` flake (task c6c9e88d).
- Commit messages: Conventional Commits, as in recent history (`test(e2e): …`, `fix(finish-worktree): …`).

## One reading of the spec to know about

The spec lists "no summary" among the ways a suite fails, and also requires "a green run with a suite whose counts can't be read still exits 0". Both hold only one way: **no summary and a non-zero exit** is `FAIL … stopped before its summary`, and **no summary and exit 0** is `PASS <suite> — counts unreadable`. A suite that dies (`die`, an early `exit 1`) always exits non-zero, so nothing is lost; a suite that exits 0 with output a parser can't read (a cargo or bun format change) stays green, as the spec asks.

## What the formats look like (checked 2026-11-04 on this machine)

- **cargo test** (not `-q`, which all.sh doesn't pass): `test tests::b ... FAILED`, then in `failures:` a `---- tests::b stdout ----` block with `thread 'tests::b' (1102921) panicked at src/lib.rs:4:22:` and the message on the next line; one `test result: FAILED. 1 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s` per test binary. All on stdout; `error: test failed, to rerun pass --lib` on stderr.
- **unittest**: all on stderr. `ERROR: test_err (test_x.T.test_err)` / `FAIL: test_fail (test_x.T.test_fail)`, a `-----` rule, `Traceback (most recent call last):`, indented frames (including `^^^^`/`~~~~` lines), then the unindented exception line `ValueError: bad value`. Then `Ran 4 tests in 0.001s` and `FAILED (failures=1, errors=1, skipped=1)` or `OK` / `OK (skipped=2)`.
- **bun** (under `claude plugin test`): all on stderr. A failing test's `error: <message>` line comes *before* its `(fail) group > fails` line, which may end ` [0.52ms]`. Summary ` 1 pass`, ` 1 skip`, ` 2 fail`, `Ran 4 tests across 1 file.`
- **e2e**: `  FAIL  <check> — <reason>` lines on stdout; summary `passed: 62   failed: 2   skipped: 0` (e2e-tag.sh's, until this branch, `passed: 19   failed: 0`); `die` and early exits write one line to stderr and print no summary; the cleanup trap may print `the run's files are kept in …` to stdout after that.

---

### Task 1: The four summary parsers

**Files:**
- Create: `tests/lib/suites.sh`
- Create: `tests/all_summary.rs`

**Interfaces:**
- Produces: `summary_cargo`, `summary_unittest`, `summary_bun`, `summary_e2e`, each called as `summary_<format> <stdout-file> <stderr-file>`. On a summary found: prints `P F S` (passed, failed, skipped, space-separated) on line 1, then one line per failing test, `name — reason` (just the check's text for e2e), and exits 0. With no summary: prints nothing and exits 1.
- Produces (test helpers in `tests/all_summary.rs`): `fn lib() -> PathBuf`, `fn scratch(name: &str) -> PathBuf`, `fn summary(format: &str, stdout: &str, stderr: &str) -> (String, bool)`.

- [ ] **Step 1: Write the failing tests**

Create `tests/all_summary.rs`:

```rust
//! tests/all.sh's closing block, from tests/lib/suites.sh: each suite
//! format's summary parser, and the runner's verdicts (counts, failing
//! tests, a suite that stops before its summary, one whose counts can't be
//! read) with the exit status all.sh ends on. The samples are the suites'
//! real output, cut down.

use std::path::{Path, PathBuf};
use std::process::Command;

fn lib() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/lib/suites.sh")
}

/// A fresh scratch directory of this test's own.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("niritasks-all-summary-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// `summary_<format>` on a suite's stdout and stderr, as all.sh calls it:
/// what it prints, and whether it found a summary.
fn summary(format: &str, stdout: &str, stderr: &str) -> (String, bool) {
    let dir = scratch(format);
    std::fs::write(dir.join("out"), stdout).unwrap();
    std::fs::write(dir.join("err"), stderr).unwrap();
    let run = Command::new("bash")
        .arg("-c")
        .arg(format!(r#"source "$1" && summary_{format} "$2/out" "$2/err""#))
        .arg("bash")
        .arg(lib())
        .arg(&dir)
        .output()
        .expect("run bash");
    let _ = std::fs::remove_dir_all(&dir);
    (String::from_utf8(run.stdout).unwrap(), run.status.success())
}

const CARGO_OUT: &str = r#"
running 4 tests
test tests::c ... ignored
test tests::a ... ok
test tests::b ... FAILED
test tests::d ... FAILED

failures:

---- tests::b stdout ----

thread 'tests::b' (1102921) panicked at src/lib.rs:4:22:
assertion `left == right` failed
  left: 1
 right: 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- tests::d stdout ----

thread 'tests::d' (1102922) panicked at src/lib.rs:6:22:
boom here


failures:
    tests::b
    tests::d

test result: FAILED. 1 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
test it_works ... ok
test it_also ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

"#;

const UNITTEST_ERR: &str = r#"EF.s
======================================================================
ERROR: test_err (test_x.T.test_err)
----------------------------------------------------------------------
Traceback (most recent call last):
  File "/repo/tests/test_x.py", line 5, in test_err
    def test_err(self): raise ValueError("bad value")
                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
ValueError: bad value

======================================================================
FAIL: test_fail (test_x.T.test_fail)
----------------------------------------------------------------------
Traceback (most recent call last):
  File "/repo/tests/test_x.py", line 4, in test_fail
    def test_fail(self): self.assertEqual(1, 2)
                         ~~~~~~~~~~~~~~~~^^^^^^
AssertionError: 1 != 2

----------------------------------------------------------------------
Ran 4 tests in 0.001s

FAILED (failures=1, errors=1, skipped=1)
"#;

const BUN_ERR: &str = r#"
tests/x.test.ts:
4 |   test('fails', () => { expect(1).toBe(2) })
                                      ^
error: expect(received).toBe(expected)

Expected: 2
Received: 1

      at <anonymous> (/repo/tests/x.test.ts:4:35)
(fail) group > fails
(pass) group > passes [0.31ms]
7 | test('throws', () => { throw new Error('kaboom') })
                                                   ^
error: kaboom
      at <anonymous> (/repo/tests/x.test.ts:7:48)
(fail) throws [0.52ms]

 1 pass
 1 skip
 2 fail
 2 expect() calls
Ran 4 tests across 1 file. [7.00ms]
"#;

const E2E_OUT: &str = r#"  PASS  a task with nothing peeked leaves the edge empty
  FAIL  task peek --focused peeks at the focused pane's task — not so: columns 1567-1600, rows 445-555 differ from one
  FAIL  task peek --focused on no task failed
  SKIP  no wtype: the typed checks
  PASS  niri-tasks.service was left alone (main pid 0)

passed: 2   failed: 2   skipped: 1
the run's files are kept in /tmp/tmp.fYoz04C2oQ
"#;

/// Every test binary's line added up; each failing test named with its
/// panic's first line.
#[test]
fn cargo_adds_up_every_binary_and_names_each_failing_test() {
    assert_eq!(
        summary("cargo", CARGO_OUT, "error: test failed, to rerun pass `--lib`\n"),
        (
            "3 2 1\ntests::b — assertion `left == right` failed\ntests::d — boom here\n".into(),
            true
        )
    );
}

/// Skips and errors come out of the verdict line; each FAIL and ERROR is
/// named with the exception its traceback ends on.
#[test]
fn unittest_counts_failures_errors_and_skips_and_names_each() {
    assert_eq!(
        summary("unittest", "", UNITTEST_ERR),
        (
            "1 2 1\ntest_err (test_x.T.test_err) — ValueError: bad value\n\
             test_fail (test_x.T.test_fail) — AssertionError: 1 != 2\n"
                .into(),
            true
        )
    );
    assert_eq!(summary("unittest", "", "Ran 34 tests in 2.558s\n\nOK\n"), ("34 0 0\n".into(), true));
    assert_eq!(summary("unittest", "", "Ran 34 tests in 2.558s\n\nOK (skipped=2)\n"), ("32 0 2\n".into(), true));
}

/// A failing test's `error:` line comes before its `(fail)` line, which may
/// carry a time.
#[test]
fn bun_counts_pass_fail_skip_and_names_each_failing_test() {
    assert_eq!(
        summary("bun", "bun test v1.3.13 (bf2e2cec)\n", BUN_ERR),
        ("1 2 1\ngroup > fails — expect(received).toBe(expected)\nthrows — kaboom\n".into(), true)
    );
}

/// The FAIL lines already carry their reason. e2e-tag.sh's summary had no
/// `skipped:` before this branch; the 2026-11-03 log still reads.
#[test]
fn e2e_reads_its_summary_and_fail_lines() {
    assert_eq!(
        summary("e2e", E2E_OUT, ""),
        (
            "2 2 1\ntask peek --focused peeks at the focused pane's task — not so: columns 1567-1600, rows 445-555 differ from one\n\
             task peek --focused on no task failed\n"
                .into(),
            true
        )
    );
    assert_eq!(summary("e2e", "passed: 19   failed: 0\n", ""), ("19 0 0\n".into(), true));
}

/// A suite that stopped partway has no summary, whatever its format.
#[test]
fn no_summary_prints_nothing_and_says_so() {
    for format in ["cargo", "unittest", "bun", "e2e"] {
        assert_eq!(
            summary(format, "  PASS  one\n", "the outer niri did not start\n"),
            (String::new(), false),
            "{format}"
        );
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --test all_summary`
Expected: all five FAIL (bash: `tests/lib/suites.sh: No such file or directory`, so empty output and `false`).

- [ ] **Step 3: Write the parsers**

Create `tests/lib/suites.sh`:

```bash
# shellcheck shell=bash
# tests/all.sh's summary of its suites. Each suite has its own summary
# format; a parser below reads one suite's stdout and stderr files and
# prints its counts, `passed failed skipped`, then one line per failing test
# (`name — first line of its reason`). With no summary to find it prints
# nothing and returns 1. Sourced by tests/all.sh; tests/all_summary.rs
# checks it. /usr/bin/awk is mawk here: no gensub, no {n} intervals.

# cargo test: a `test result:` line per test binary, added up (`ignored`
# counts as skipped). A failing test is `test <name> ... FAILED`; its reason
# is the first line of its panic, under `---- <name> stdout ----`.
summary_cargo() {
    cat -- "$@" | awk '
        /^test result: / {
            found = 1
            for (i = 1; i < NF; i++) {
                if ($(i+1) ~ /^passed;/)  p += $i
                if ($(i+1) ~ /^failed;/)  f += $i
                if ($(i+1) ~ /^ignored;/) s += $i
            }
        }
        /^test .* \.\.\. FAILED$/ { if (!($2 in why)) { order[++n] = $2; why[$2] = "" } }
        /^---- .* stdout ----$/ { cur = $2 }
        cur != "" && / panicked at / {
            line = $0
            if (line ~ /:$/) getline line
            else sub(/.* panicked at /, "", line)
            why[cur] = line; cur = ""
        }
        END {
            if (!found) exit 1
            print p + 0, f + 0, s + 0
            for (i = 1; i <= n; i++) print order[i] (why[order[i]] != "" ? " — " why[order[i]] : "")
        }'
}

# Python unittest: `Ran N tests`, then `OK` or `FAILED`, either with
# `(failures=F, errors=E, skipped=K, ...)`. A failing test is a `FAIL:` or
# `ERROR:` header; its reason is the unindented exception line its
# traceback ends on.
summary_unittest() {
    cat -- "$@" | awk '
        /^Ran [0-9]+ tests? in / { ran = $2; seen_ran = 1 }
        seen_ran && /^(OK|FAILED)( \(.*\))?$/ {
            verdict = 1
            line = $0; sub(/^[A-Z]+ ?\(?/, "", line); sub(/\)$/, "", line)
            k = split(line, parts, ", ")
            for (i = 1; i <= k; i++) { split(parts[i], kv, "="); c[kv[1]] = kv[2] + 0 }
        }
        /^(FAIL|ERROR): / { cur = $0; sub(/^[A-Z]+: /, "", cur); order[++n] = cur; tb = 0 }
        cur != "" && /^Traceback / { tb = 1; next }
        tb && /^[^ \t]/ { why[cur] = $0; tb = 0; cur = "" }
        END {
            if (!verdict) exit 1
            f = c["failures"] + c["errors"] + c["unexpected successes"]
            s = c["skipped"] + 0
            print ran - f - s, f, s
            for (i = 1; i <= n; i++) print order[i] (why[order[i]] != "" ? " — " why[order[i]] : "")
        }'
}

# bun, as claude plugin test runs it: ` N pass`, ` N fail` and ` N skip`.
# A failing test is `(fail) <name>`, maybe with ` [time]`; its reason is the
# `error:` line printed above it. bun lists failures again at the foot, so
# each is named once.
summary_bun() {
    cat -- "$@" | awk '
        /^ *[0-9]+ pass$/ { p = $1; seen_p = 1 }
        /^ *[0-9]+ fail$/ { f = $1; seen_f = 1 }
        /^ *[0-9]+ skip$/ { s = $1 }
        /^error: / { err = $0; sub(/^error: /, "", err) }
        /^\(fail\) / {
            name = $0; sub(/^\(fail\) /, "", name); sub(/ \[[0-9.]+m?s\]$/, "", name)
            if (!(name in why)) { order[++n] = name; why[name] = err }
            err = ""
        }
        END {
            if (!(seen_p && seen_f)) exit 1
            print p + 0, f + 0, s + 0
            for (i = 1; i <= n; i++) print order[i] (why[order[i]] != "" ? " — " why[order[i]] : "")
        }'
}

# The e2e scripts: `passed: N   failed: N   skipped: N` (e2e-tag.sh's had no
# skipped field before 2026-11, so it reads as 0). A failing check is its
# `  FAIL  ` line, which carries its own reason.
summary_e2e() {
    cat -- "$@" | awk '
        /^passed: [0-9]+ +failed: [0-9]+/ { found = 1; p = $2; f = $4; s = ($5 == "skipped:") ? $6 : 0 }
        /^  FAIL  / { order[++n] = substr($0, 9) }
        END {
            if (!found) exit 1
            print p + 0, f + 0, s + 0
            for (i = 1; i <= n; i++) print order[i]
        }'
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --test all_summary`
Expected: `test result: ok. 5 passed; 0 failed`.

- [ ] **Step 5: Check the parsers against the 2026-11-03 log**

Cut the log into its suite sections (stopping at the closing block's rule, so its `  FAIL  tests/e2e-panel.sh` line isn't read as a check) and feed each to its parser:

```bash
L=~/.local/share/niri-tasks/reviews/e2e-panel-peek-focused-flake-20261103/20261103T110452Z-485336.log
D=$(mktemp -d)
awk -v d="$D" '/^──────/{exit} /── .* ──/{n++} n{print > (d "/" n ".log")}' "$L"
bash -c 'source tests/lib/suites.sh
for pair in 1:cargo 2:e2e 3:unittest 4:bun 5:e2e 6:e2e; do
    echo "-- ${pair#*:}"; summary_${pair#*:} "$1/${pair%%:*}.log" /dev/null || echo "(no summary)"
done' bash "$D"
rm -rf "$D"
```

Expected (1247 passed, 2 failed, 0 skipped in all):
```
-- cargo
811 0 0
-- e2e
19 0 0
-- unittest
34 0 0
-- bun
274 0 0
-- e2e
62 2 0
task peek --focused peeks at the focused pane's task — not so: columns 1567-1600, rows 445-555 differ from one
task peek --focused on no task failed
-- e2e
47 0 0
```

- [ ] **Step 6: Commit**

```bash
git add tests/lib/suites.sh tests/all_summary.rs
git commit -m "test(all): read each suite's own summary format" -m "One parser per format all.sh's suites print (cargo, unittest, bun, the
e2e scripts): counts passed, failed and skipped, and names each failing
test with the first line of its reason. all.sh uses them next."
```

---

### Task 2: The runner and the closing block

**Files:**
- Modify: `tests/lib/suites.sh` (append the runner)
- Modify: `tests/all.sh:48-155` (source the file, pass a format to each `suite`, end on `closing_block`; header comment)
- Modify: `tests/e2e-tag.sh:347`
- Test: `tests/all_summary.rs` (append)

**Interfaces:**
- Consumes: `summary_cargo|unittest|bun|e2e <out> <err>` from Task 1.
- Produces: `suite <name> <why-it-cannot-run-or-empty> <format> <command...>`; `closing_block` (prints the block, returns 0 when no suite failed, else 1). Closing-block lines, which Task 3's skills read:
  - `  PASS  <name> — P passed, F failed, S skipped (Ns)` / `  FAIL  <name> — P passed, F failed, S skipped (Ns)`
  - `      - <failing test> — <reason>` under a FAIL, one per failing test; or `      - exited N, naming no failing test: <last stderr line>`
  - `  FAIL  <name> — stopped before its summary: <last stderr line> (Ns)`
  - `  PASS  <name> — counts unreadable (Ns)`
  - `  SKIP  <name> — <why>`
  - last line: `suites: X of T passed, Y failed, Z skipped · tests: P passed, F failed, S skipped`, plus ` (not counted: a, b)` when a suite's counts weren't read.

- [ ] **Step 1: Write the failing tests**

Append to `tests/all_summary.rs`:

```rust
/// all.sh's runner on suites given as shell lines, then its closing block:
/// stdout, stderr, and the exit status all.sh would end on.
fn run_all(suites: &str) -> (String, String, i32) {
    let run = Command::new("bash")
        .arg("-c")
        .arg(format!("set -uo pipefail\nsource \"$1\"\n{suites}\nclosing_block"))
        .arg("bash")
        .arg(lib())
        .output()
        .expect("run bash");
    (
        String::from_utf8(run.stdout).unwrap(),
        String::from_utf8(run.stderr).unwrap(),
        run.status.code().expect("an exit status"),
    )
}

/// The closing block, from its rule on, each suite's time taken out: a
/// suite can straddle a second.
fn closing(stdout: &str) -> String {
    let block = &stdout[stdout.rfind("──────").expect("a closing block")..];
    block
        .lines()
        .map(|line| match line.rfind(" (") {
            Some(at)
                if line.ends_with("s)")
                    && line[at + 2..line.len() - 2].chars().all(|c| c.is_ascii_digit()) =>
            {
                &line[..at]
            }
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Counts per suite, failing checks under their suite, a suite that died
/// with its reason, a skipped one; red, and the suite's own stderr still
/// reaches the terminal.
#[test]
fn the_closing_block_counts_each_suite_and_names_what_broke() {
    let (stdout, stderr, code) = run_all(
        r#"suite "green" "" e2e bash -c 'echo "  SKIP  no wtype"; echo; echo "passed: 3   failed: 0   skipped: 1"'
suite "skipped" "no wtype (sudo apt install wtype)" e2e true
suite "red" "" e2e bash -c 'echo "  FAIL  the card shows — not so"; echo "passed: 2   failed: 1   skipped: 0"; exit 1'
suite "dies" "" e2e bash -c 'echo "  PASS  one"; echo "the outer niri did not start" >&2; echo "files kept in /tmp/x"; exit 1'"#,
    );
    assert_eq!(
        closing(&stdout),
        "──────────────────────────────────────────────────────────────
  PASS  green — 3 passed, 0 failed, 1 skipped
  SKIP  skipped — no wtype (sudo apt install wtype)
  FAIL  red — 2 passed, 1 failed, 0 skipped
      - the card shows — not so
  FAIL  dies — stopped before its summary: the outer niri did not start

suites: 1 of 4 passed, 2 failed, 1 skipped · tests: 5 passed, 1 failed, 1 skipped (not counted: dies)"
    );
    assert_eq!(code, 1);
    assert!(stdout.contains("── dies ──"), "{stdout}");
    assert!(stdout.contains("$ bash -c "), "{stdout}");
    assert!(stdout.contains("── skipped ──\u{1b}[0m  SKIP: no wtype"), "{stdout}");
    assert!(stderr.contains("the outer niri did not start"), "{stderr}");
}

/// A suite that exits 0 with no summary it can read stays green: counts it
/// can't read never change the exit status.
#[test]
fn counts_it_cannot_read_leave_a_green_run_green() {
    let (stdout, _, code) = run_all(
        r#"suite "cargo test" "" cargo bash -c 'echo "test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s"'
suite "mute" "" bun true"#,
    );
    assert_eq!(
        closing(&stdout),
        "──────────────────────────────────────────────────────────────
  PASS  cargo test — 2 passed, 0 failed, 1 skipped
  PASS  mute — counts unreadable

suites: 2 of 2 passed, 0 failed, 0 skipped · tests: 2 passed, 0 failed, 1 skipped (not counted: mute)"
    );
    assert_eq!(code, 0);
}

/// The suite's own exit status decides, not tee's: a non-zero exit whose
/// summary names no failure is still a FAIL, with its last stderr line.
#[test]
fn a_failing_exit_with_a_clean_summary_is_red() {
    let (stdout, _, code) = run_all(
        r#"suite "crashed" "" cargo bash -c 'echo "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s"; echo "error: test failed, to rerun pass --test x" >&2; exit 101'"#,
    );
    assert_eq!(
        closing(&stdout),
        "──────────────────────────────────────────────────────────────
  FAIL  crashed — 5 passed, 0 failed, 0 skipped
      - exited 101, naming no failing test: error: test failed, to rerun pass --test x

suites: 0 of 1 passed, 1 failed, 0 skipped · tests: 5 passed, 0 failed, 0 skipped"
    );
    assert_eq!(code, 1);
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --test all_summary`
Expected: the three new tests FAIL (`suite: command not found`, so `closing` panics with "a closing block" or the exit code is 127); Task 1's five still pass.

- [ ] **Step 3: Append the runner to `tests/lib/suites.sh`**

Also change the file's opening comment's first line to `# tests/all.sh's suite runner and its summary of its suites.` Then append:

```bash

# The runner. Each suite's stdout and stderr go to the terminal as before
# and to files of their own here, for its parser and its last stderr line.
SUITE_LOGS=$(mktemp -d) || exit 1
trap 'rm -rf "$SUITE_LOGS"' EXIT
results=()      # the closing block's lines, in the order the suites ran
not_counted=()  # suites whose counts could not be read
suites_total=0 suites_passed=0 suites_failed=0 suites_skipped=0
tests_passed=0 tests_failed=0 tests_skipped=0

suite() {  # name, reason it cannot run (empty if it can), summary format, command...
    local name="$1" why="$2" format="$3"; shift 3
    suites_total=$((suites_total + 1))
    if [ -n "$why" ]; then
        printf '\n\033[1m── %s ──\033[0m  SKIP: %s\n' "$name" "$why"
        results+=("  SKIP  $name — $why")
        suites_skipped=$((suites_skipped + 1))
        return
    fi
    printf '\n\033[1m── %s ──\033[0m\n' "$name"
    # The command as run, so that a reader of the log (a branch report)
    # can name it rather than guess it.
    printf '$ %s\n' "$*"
    local start=$SECONDS out="$SUITE_LOGS/$suites_total.out" err="$SUITE_LOGS/$suites_total.err"
    # stderr through one tee, stdout through the other; the subshell exits
    # with the suite's own status, so PIPESTATUS[0] is the suite's, never tee's.
    ( "$@" 2>&1 1>&3 3>&- | tee "$err" >&2; exit "${PIPESTATUS[0]}" ) 3>&1 | tee "$out"
    local rc=${PIPESTATUS[0]}
    local secs=$((SECONDS - start)) parsed p f s last word failing line
    parsed=$(summary_"$format" "$out" "$err" 2>/dev/null)
    read -r p f s <<<"${parsed%%$'\n'*}"
    last=$(grep -v '^[[:space:]]*$' "$err" | tail -n 1)
    if [[ "$p $f $s" =~ ^[0-9]+\ [0-9]+\ [0-9]+$ ]]; then
        tests_passed=$((tests_passed + p))
        tests_failed=$((tests_failed + f))
        tests_skipped=$((tests_skipped + s))
        word=PASS
        if [ "$rc" -ne 0 ] || [ "$f" -gt 0 ]; then word=FAIL; fi
        results+=("  $word  $name — $p passed, $f failed, $s skipped (${secs}s)")
        failing=$(tail -n +2 <<<"$parsed")
        if [ -n "$failing" ]; then
            # "      - ": a failing test's line never starts FAIL, PASS or ──,
            # so a reader can't take it for a suite's line or the rule.
            while IFS= read -r line; do results+=("      - $line"); done <<<"$failing"
        elif [ "$word" = FAIL ]; then
            results+=("      - exited $rc, naming no failing test: ${last:-nothing on stderr}")
        fi
    elif [ "$rc" -ne 0 ]; then
        word=FAIL
        results+=("  FAIL  $name — stopped before its summary: ${last:-nothing on stderr} (${secs}s)")
        not_counted+=("$name")
    else
        # Exit 0 and no summary read: a parser out of date, not a red suite.
        word=PASS
        results+=("  PASS  $name — counts unreadable (${secs}s)")
        not_counted+=("$name")
    fi
    if [ "$word" = FAIL ]; then
        suites_failed=$((suites_failed + 1))
    else
        suites_passed=$((suites_passed + 1))
    fi
}

# One line per suite, in run order, failing tests under their suite, then
# suites and tests counted on one line worded so neither reads as the other.
# A skip is not a failure: only a suite that ran and said no fails this.
closing_block() {
    local uncounted=""
    if [ "${#not_counted[@]}" -gt 0 ]; then
        printf -v uncounted '%s, ' "${not_counted[@]}"
        uncounted=" (not counted: ${uncounted%, })"
    fi
    echo
    echo "──────────────────────────────────────────────────────────────"
    printf '%s\n' "${results[@]}"
    echo
    echo "suites: $suites_passed of $suites_total passed, $suites_failed failed, $suites_skipped skipped · tests: $tests_passed passed, $tests_failed failed, $tests_skipped skipped$uncounted"
    [ "$suites_failed" -eq 0 ]
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --test all_summary`
Expected: `test result: ok. 8 passed; 0 failed`.

- [ ] **Step 5: Wire it into `tests/all.sh`**

1. Delete line 59 (`passed=(); failed=(); skipped=()`) and the whole `suite() { … }` function (lines 110-127).
2. After `export NIRITASKS` (line 57), add:

```bash

# suite(), the four summary parsers and closing_block.
source "$ROOT/tests/lib/suites.sh"
```

3. Give each `suite` call its format as the third argument:

```bash
suite "cargo test"           "$(why_cargo)"  cargo    cargo test --no-fail-fast
suite "tests/e2e-tag.sh"     "$(why_tag)"    e2e      bash tests/e2e-tag.sh
suite "herdr/task-docs tests" "$(why_plugin)" unittest \
    python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs
suite "refine mod tests"     "$(why_mod)"    bun      claude plugin test claude/refine-mod
```
and
```bash
suite "tests/e2e-panel.sh"   "$(why_panel)"  e2e      bash tests/e2e-panel.sh
suite "tests/e2e-box.sh"     "$(why_box)"    e2e      bash tests/e2e-box.sh
```

4. Replace everything from the `echo` after the last `suite` line to the end of the file (old lines 146-155) with:

```bash

closing_block
```

5. In the header comment, after the paragraph ending "safe to put in a hook or a loop on a machine where only half of it can run." (line 28), add:

```bash
#
# The closing block, after a ──── rule, gives each suite's own counts —
# tests passed, failed and skipped inside it, read from its summary — and
# under a failed suite its failing tests with the first line of each reason,
# then one line counting suites and tests apart. A suite that stops before
# its summary says so, with its last line on stderr. Its output goes
# through tee to be read, so no suite sees a terminal: cargo prints no
# colour here.
```

- [ ] **Step 6: Give e2e-tag.sh's summary a skipped field**

In `tests/e2e-tag.sh:347` change

```bash
echo "passed: $pass   failed: $fail"
```
to
```bash
echo "passed: $pass   failed: $fail   skipped: 0"
```

- [ ] **Step 7: Run all.sh headless and read its closing block**

Run: `cargo build && NIRITASKS=$PWD/target/debug/niritasks NIRITASKS_HEADLESS=1 bash tests/all.sh 2>&1 | tail -n 12; echo "exit ${PIPESTATUS[0]}"`

Expected, with numbers near these (cargo grows by the 8 new tests; times differ):
```
──────────────────────────────────────────────────────────────
  PASS  cargo test — 8xx passed, 0 failed, 0 skipped (…s)
  SKIP  tests/e2e-tag.sh — headless run (NIRITASKS_HEADLESS)
  PASS  herdr/task-docs tests — 3x passed, 0 failed, 0 skipped (…s)
  PASS  refine mod tests — 27x passed, 0 failed, 0 skipped (…s)
  SKIP  tests/e2e-panel.sh — headless run (NIRITASKS_HEADLESS)
  SKIP  tests/e2e-box.sh — headless run (NIRITASKS_HEADLESS)

suites: 3 of 6 passed, 0 failed, 3 skipped · tests: 11xx passed, 0 failed, 0 skipped
exit 0
```
No line may read `counts unreadable`: each live format must parse. If one does, compare that suite's output in the run with its parser's fixture in `tests/all_summary.rs` and fix the parser, adding the new shape to the fixture.

- [ ] **Step 8: Run all.sh with one failing test, and with a suite that dies**

Add a failing test for the length of this step only, at the end of `tests/all_summary.rs`:

```rust
#[test]
fn deliberately_red() { assert_eq!(1, 2, "the step 8 check"); }
```

Run: `NIRITASKS=$PWD/target/debug/niritasks NIRITASKS_HEADLESS=1 bash tests/all.sh 2>&1 | tail -n 12; echo "exit ${PIPESTATUS[0]}"`
Expected: `  FAIL  cargo test — … passed, 1 failed, 0 skipped (…s)`, under it `      - deliberately_red — assertion `left == right` failed: the step 8 check`, the final line's `1 failed` in both counts, and `exit 1`. Remove the test again (`git diff tests/all_summary.rs` shows nothing beyond Step 1's additions).

Then make e2e-tag stop before its summary, at its own early `exit 1` (`tests/e2e-tag.sh:37-50`): `NIRITASKS=/nonexistent/niritasks bash -c 'source tests/lib/suites.sh; suite "tests/e2e-tag.sh" "" e2e bash tests/e2e-tag.sh; closing_block'; echo "exit $?"`
Expected: `  FAIL  tests/e2e-tag.sh — stopped before its summary: no /nonexistent/niritasks on $PATH — build it, or set NIRITASKS= (0s)` (or, on a machine with no niri running, `… niri is not running (no $NIRI_SOCKET)`), the last line ending `(not counted: tests/e2e-tag.sh)`, and `exit 1`. Nothing is renamed: it exits before it touches a workspace.

- [ ] **Step 9: Run the whole cargo suite**

Run: `cargo test --no-fail-fast 2>&1 | grep -E '^test result:|FAILED'`
Expected: every `test result: ok.`, no `FAILED`.

- [ ] **Step 10: Commit**

```bash
git add tests/lib/suites.sh tests/all_summary.rs tests/all.sh tests/e2e-tag.sh
git commit -m "test(all): count each suite's tests in all.sh's summary" -m "The closing block counted suites with the words the suites use for
single tests, so a green run couldn't show how many tests ran or were
skipped, and a red one named only the suite. Each suite's output is now
teed to a file and read in its own format: one line per suite with its
tests passed, failed and skipped, its failing tests under it, a suite
that stopped before its summary with its last stderr line, and a final
line counting suites and tests apart. The exit status is still the one
gate. e2e-tag.sh's summary gains skipped: 0 to match the other e2e
scripts."
```

---

### Task 3: The skills that read all.sh's text

**Files:**
- Modify: `.claude/skills/finish-worktree/SKILL.md:117-128` (step 6's block and the bullet under it)
- Modify: `.claude/skills/report-branch/SKILL.md:82-107` (step 3, with a tests file)
- Modify: `.claude/skills/report-branch/branch-catalogue.md:49-50`

**Interfaces:**
- Consumes: Task 2's closing-block lines (see Task 2's Interfaces).

- [ ] **Step 1: finish-worktree step 6: the grep and the block it shows**

In `.claude/skills/finish-worktree/SKILL.md`, replace these lines of the step 6 block:

```bash
      grep -Eq '^[0-9]+ passed +[0-9]+ failed +[0-9]+ skipped$' "$LOGS/$after"; then
      grep -E '^parity:|^  [<>] ' "$LOGS/$after"
      tac "$LOGS/$after" | sed '/^────/q' | tac | grep -E '^  (PASS|SKIP|FAIL)  |^[0-9]+ passed '
```
with
```bash
      grep -Eq '^suites: [0-9]+ of [0-9]+ passed, [0-9]+ failed, [0-9]+ skipped · tests: ' "$LOGS/$after"; then
      grep -E '^parity:|^  [<>] ' "$LOGS/$after"
      tac "$LOGS/$after" | sed '/^────/q' | tac | sed '/^suites: /q'
```

(The closing block now runs from the `────` rule to the `suites:` line, failing tests indented under their suite; `sed '/^suites: /q'` stops before container.sh's `kept`/`log` lines.)

Then replace the bullet

```markdown
  - A run that printed the summary line (`N passed N failed N skipped`) is the
    test result, red or green: `0 failed` is green, anything else red. The exit
    status alone can't tell a failed suite from a run that never started.
```
with
```markdown
  - A run that printed the summary line (`suites: N of N passed, N failed, N
    skipped · tests: …`) is the test result, red or green: `0 failed` in its
    suites count is green, anything else red. The lines above it give each
    suite's tests passed, failed and skipped, and under a failed suite its
    failing tests: name those in the report. The exit status alone can't tell
    a failed suite from a run that never started.
```

Leave the rest of step 6 as it is (task 92236866).

- [ ] **Step 2: Check the new block on a red log and an old one**

```bash
D=$(mktemp -d); LOGS=$D; after=red.log; before=""
cat > "$D/red.log" <<'EOF'
── cargo test ──
$ cargo test --no-fail-fast
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

──────────────────────────────────────────────────────────────
  PASS  cargo test — 811 passed, 0 failed, 0 skipped (33s)
  FAIL  tests/e2e-panel.sh — 62 passed, 2 failed, 0 skipped (95s)
      - task peek --focused on no task failed

suites: 0 of 2 passed, 1 failed, 0 skipped · tests: 873 passed, 2 failed, 0 skipped
kept  /x/artifacts (logs and the e2e suites' frames)
EOF
if grep -Eq '^suites: [0-9]+ of [0-9]+ passed, [0-9]+ failed, [0-9]+ skipped · tests: ' "$LOGS/$after"; then
    tac "$LOGS/$after" | sed '/^────/q' | tac | sed '/^suites: /q'
else echo "no test run in the container"; fi
rm -rf "$D"
```
Expected: the rule, the two suite lines, the indented failing test, a blank line and the `suites:` line; no `kept` line. Run the same `if` against the 2026-11-03 log (`after` set to that file's path, `LOGS=/`): expected `no test run in the container`, since that log predates the new line.

- [ ] **Step 3: report-branch step 3: read all.sh's closing block**

In `.claude/skills/report-branch/SKILL.md`, replace the four bullets from `- \`suite.runs\`: one entry for each suite whose header is not followed by` through the end of the bullet `- The tests file's \`exit_code\` is checked against your counts: … names a suite that failed.` with:

```markdown
- **A log from `tests/all.sh`** ends with its closing block: a `────`
  rule, one line per suite, and a last line starting `suites:`. Take each
  suite's counts and failing tests from that block, not from the suite's
  own lines above:
  - `  PASS  <suite> — P passed, F failed, S skipped (Ns)` or the same with
    `FAIL`: a `suite.runs` entry, with `passed` P and `failed` F. `command`
    is the line under that suite's `── <suite> ──` header that starts `$ `,
    without the `$ `.
  - Each `      - <test> — <reason>` line under a `FAIL`: a `suite.failing`
    entry, the reason shortened to fit. Its `file` is the suite's script
    for an e2e suite; otherwise the file that suite's section of the log
    names for that test (cargo's `panicked at src/…`, unittest's
    `File "…"`, bun's `at … (…test.ts:…)`).
  - `  FAIL  <suite> — stopped before its summary: <reason>`: a
    `suite.runs` entry with the counts you can see in its section's lines,
    and a `suite.failing` entry whose test is "stopped before its summary"
    and whose why is that reason. `counts unreadable` (a `PASS`): a
    `suite.runs` entry with the counts you can see, and say in the report
    that all.sh could not read them.
  - `  SKIP  <suite> — <reason>`: a `suite.not_run` entry with that reason
    as `why`. The sandbox stopped none of them, so give no sandbox reason.
  - A suite whose `S skipped` is above 0 also gets one `suite.not_run`
    entry (one per suite, never one per test): the suite's name and `(S
    skipped)`, and as `why` the reasons on its `  SKIP  ` lines in its
    section of the log, or "S tests skipped inside the suite" when it has
    none.
  - The tests file's `exit_code` is checked against the `suites:` line:
    `0` means its suites count says `0 failed`, every `failed` is 0 and
    `failing` is empty; anything else means at least one `FAIL` line, so
    at least one `failed` is above 0 or a suite stopped, and `failing`
    names it. A count that disagrees is wrong: read the log again, do not
    adjust it to fit.
- **Any other log** (another project's runner): work through it suite by
  suite; a suite starts at a header line and runs to the next.
  - `suite.runs`: one entry for each suite whose header is not followed by
    `SKIP:`. `command` is the line under the header that starts `$ `,
    without the `$ `; a log with no such line gives the `command` in the
    tests file instead; never make one up. `passed` and `failed` are the
    counts in that suite's own summary lines, never the sum over the log
    and never a count you made by tallying lines: `test result: ok. 12
    passed; 0 failed` lines for cargo (a cargo suite prints one per test
    binary, so add up that suite's lines, and only that suite's), `Ran N
    tests` and `OK` or `FAILED (failures=N)` for Python unittest, `N pass`
    and `N fail` for Bun and `claude plugin test`. If a suite prints no
    summary you can read, say so in the report and use the counts you can
    see in its lines; do not guess.
  - `suite.not_run`: one entry for each header followed by `SKIP:
    <reason>`, with that reason as `why`, shortened to fit. The sandbox
    stopped none of them, so give no sandbox reason.
  - `suite.failing`: each failing test the log names, with its file and
    what it says went wrong. Look in a failed suite for `FAILED`, `FAIL`,
    `panicked at` and `Traceback` lines.
  - The tests file's `exit_code` is checked against your counts: `0` means
    nothing failed anywhere, so every `failed` is 0 and `failing` is empty;
    anything else means at least one failed suite, so at least one `failed`
    is above 0 and `failing` names it. A count that disagrees is wrong:
    read the log again, do not adjust it to fit.
```

Keep the bullets after it ("A test a run lists as failing is a failure of the branch…", "Say in the report that the suites ran before the session…") and the paragraph before it ("Work through it suite by suite; a suite starts at a `── name ──` header line…") as they are.

- [ ] **Step 4: branch-catalogue.md's suite rows**

In `.claude/skills/report-branch/branch-catalogue.md`, replace the `suite.runs` and `suite.not_run` rows (lines 49-50) with:

```markdown
| `suite.runs` | Each suite that ran: a short name, the command, and the passed and failed counts from its own summary: from `tests/all.sh`'s closing block when the test log has one, else from that suite's own summary lines in the log. | 1–8; suite 6 words, command 200 characters |
| `suite.not_run` | Each suite that did not run, and why: an e2e that needs a display, one the log marks `SKIP:`, or one the sandbox stopped. A suite that ran with tests skipped inside it (all.sh's `S skipped` above 0) gets one entry too, naming how many and why. | 0–6; why 25 words |
```

- [ ] **Step 5: Check the refine mod still takes the catalogue**

Run: `claude plugin test claude/refine-mod 2>&1 | tail -n 4`
Expected: ` N pass`, ` 0 fail` (the catalogue's worked example is unchanged; this confirms it).

- [ ] **Step 6: Commit**

```bash
git add .claude/skills/finish-worktree/SKILL.md .claude/skills/report-branch/SKILL.md .claude/skills/report-branch/branch-catalogue.md
git commit -m "fix: read all.sh's per-suite counts in the skills" -m "finish-worktree's step 6 now finds the run by all.sh's new suites:
line and shows the whole closing block, failing tests included.
report-branch takes an all.sh log's counts and failing tests from the
closing block and lists inner skips under not_run; other runners' logs
are read as before."
```

---

## Landing this branch, and the checks after the merge

- **The landing itself runs the old step 6.** Skills load from `~/Projects/niri-tasks`, so `/finish-worktree` on this branch uses the old grep, which won't match the new `suites:` line: it will print `no test run in the container` for a container run that did finish. The landing agent must instead read the new closing block itself (`tac target/container-tests/latest.log | sed '/^────/q' | tac`): green is `0 failed` in its `suites:` count, and it is the test result; do not fall back to the host for it. Tell the landing session this when starting `/finish-worktree`.
- **After the merge** (Done when, checked from `~/Projects/niri-tasks`):
  1. The container run of the landing reads each suite's counts, `tests: N passed, 0 failed, …` in its last line.
  2. With one deliberately failing test on a throwaway branch, `bash tests/container.sh` exits non-zero and finish-worktree's step 6 block shows it red with the test named under its suite, and stops.
  3. Explain branch on that throwaway branch writes its report with the counts from the closing block and the failing test in `suite.failing`.
  4. Throw the branch away with `/reject-worktree`.
