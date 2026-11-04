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

/// A fresh scratch directory of this call's own: tests run in parallel and
/// share formats, so the name needs a counter, not just the format.
fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("niritasks-all-summary-{name}-{}-{n}", std::process::id()));
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
    let block = &stdout[stdout.find("──────").expect("a closing block")..];
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

/// A suite killed with nothing on stderr still says how it stopped.
#[test]
fn a_silent_kill_shows_its_exit_status() {
    let (stdout, _, code) = run_all(
        r#"suite "killed" "" e2e bash -c 'echo "  PASS  one"; exit 137'"#,
    );
    assert_eq!(
        closing(&stdout),
        "──────────────────────────────────────────────────────────────
  FAIL  killed — stopped before its summary: exited 137

suites: 0 of 1 passed, 1 failed, 0 skipped · tests: 0 passed, 0 failed, 0 skipped (not counted: killed)"
    );
    assert_eq!(code, 1);
}
