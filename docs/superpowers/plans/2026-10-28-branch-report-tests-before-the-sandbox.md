# Branch Report: Run the Tests Before the Sandbox — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Explain branch runs the project's test suite first, outside the sandbox and in a tab where the user can watch it. It saves the log and exit code to the reviews folder, then starts the fenced Claude, which reports from that log instead of running the tests itself.

**Architecture:** This follows Start working's shape (`work::set_up_here`). `niritasks task report <uuid> --branch` opens a tab in the task's worktree that runs `niritasks task report <uuid> --branch --here --workspace <ws>`. That process holds a per-task lock, runs the test command with the terminal attached, and tees the output to `branch-<uuid8>-<stamp>-tests.log`. It writes a `…-tests.json` beside the log, then starts the Claude session in a new tab exactly as `refine::launch_branch` does now, and closes its own tab. The mod gets the JSON path as a new option, `tests`. It reads the file outside the sandbox and refuses a Test suite slide that contradicts the exit code. The skill reads the log rather than running suites.

**Tech Stack:** Rust (`cargo test`), the TypeScript refine mod (`cd claude/refine-mod && claude plugin test .`), bash (`tests/all.sh`), skills in Markdown.

**Spec:** the task's notes (`task rc.json.array=on b5447e66-d3fa-410a-b70e-99dcddf259f1 export`), above all the Decided note of 2026-10-28. This builds on `docs/superpowers/plans/2026-10-27-report-what-a-task-s-branch-has-changed.md`, which is already implemented on this branch.

## Global Constraints

- **Which command:**
  1. If `git -C <worktree> config --get niri-tasks.test` is set, run that string with `sh -c` in the worktree.
  2. Otherwise, if `<worktree>/tests/all.sh` exists, run `bash tests/all.sh` with `NIRITASKS_HEADLESS=1` in its environment.
  3. Otherwise run nothing, write no tests file, and the session behaves exactly as it does today: Claude runs what it finds, in the sandbox.
- **Headless only:** no nested-niri e2e test runs from a report. The task's "Out of scope" note says so.
- **Not fenced:** the test run is not sandboxed. It runs as the user, in the worktree, as `bash tests/all.sh` would.
- **Files:**
  - The log is `<reviews>/branch-<uuid8>-<stamp>-tests.log` and the results file is `<reviews>/branch-<uuid8>-<stamp>-tests.json`. `<reviews>` is `~/.local/share/niri-tasks/reviews` (`refine::reports_dir`).
  - `<stamp>` is UTC `YYYYMMDD-HHMMSS`, the same format the mod's `reportPath` uses.
  - The JSON is `{"command": string, "exit_code": number, "log": string (absolute path), "started": ISO-8601 UTC, "finished": ISO-8601 UTC}`.
- **One run per task:** a second Explain branch press while the tests run opens no second tab. It notifies "Already running this task's tests — see its tab." Other launches in the same herdr session never wait on the test run: the session claim is taken only after the tests finish, as `work::set_up` does.
- **Failure:** a failing test run does not stop the report, because the report says what broke. Only a failure to *start* the command, or to write the files, keeps the tab open showing the error and "Press Enter to close this tab", as `set_up_here` does.
- **Plan reports unchanged:** a plan report, a refine, and a branch report with no `tests` option behave exactly as before. Existing tests stay green unchanged.
- **Commits:**
  - Conventional Commits, with a subject of at most 72 characters and a body wrapped at 72 that says what and why.
  - End each message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
  - Tests stay green at every commit.
  - `cargo test` checks that README's Commands block and llms.txt name every subcommand and flag. A new flag is documented in the same commit, unless it is hidden from `--help`, as `--here` is hidden for `task start`.

---

### Task 11: `tests/all.sh` runs headless on request, and runs the mod's tests

**Files:** Modify `tests/all.sh`, and README's testing section if it lists the suites.

- [ ] **Step 1:** With `NIRITASKS_HEADLESS=1` set, report `tests/e2e-tag.sh`, `tests/e2e-panel.sh` and `tests/e2e-box.sh` as SKIP with the reason `headless run (NIRITASKS_HEADLESS)`. Do this in each one's `why_*` function, checked first, and leave the existing prerequisite checks as they are. Print the nested-niri warning only when one of those suites will actually run.
- [ ] **Step 2:** Add a suite, `refine mod tests`, that runs `claude plugin test claude/refine-mod`. Put it after the herdr plugin tests. Its `why_mod` gives `no claude on $PATH` when claude is missing.
- [ ] **Step 3:** Run cargo as `cargo test --no-fail-fast`, so one failing test binary doesn't hide the rest.
- [ ] **Step 4:** Update the header comment: six suites, what headless means, and why the mod suite is there.
- [ ] **Step 5:** Run `NIRITASKS_HEADLESS=1 NIRITASKS= bash tests/all.sh`. Expect cargo, the plugin tests and the mod tests to PASS, and the three e2e suites to SKIP with the headless reason. Commit, for example `test: let all.sh run headless and run the mod's tests`.

### Task 12: the mod reads the test run and holds the Test suite slide to it

**Files:** Modify:
- `claude/refine-mod/hooks/report.ts`: the type, the parser for the tests file, the check.
- `claude/refine-mod/hooks/refine.ts`: the option, the read outside the sandbox, the call in `reportSession`'s branch path.
- `claude/refine-mod/.claude-plugin/plugin.json`: the `tests` option, a string defaulting to `""`.
- `claude/refine-mod/hooks/markdown.ts`: the twin line.
- `claude/refine-mod/hooks/branch-page.ts`: the suite slide shows the run.

Tests go in `tests/report.test.ts`, `tests/refine.test.ts`, `tests/markdown.test.ts` and `tests/page.test.ts`.

**Interfaces produced:**
- `type TestRun = { command: string; exit_code: number; log: string; started: string; finished: string }`.
- `parseTestRun(text: string): TestRun | string`. A string return is the reason it's unreadable.
- `checkSuite(suite: Suite, run: TestRun): string[]` gives the refusals:
  - When `exit_code === 0`, any `runs[i].failed > 0`, or a non-empty `failing`, is refused with: `the test run passed (exit 0), so no suite can have failed`.
  - When `exit_code !== 0`, a suite where every `failed === 0` and `failing` is empty is refused with: `the test run failed (exit <n>): name the suite and the tests that failed, from its log <log>`.
- `Branch` gains `run?: TestRun`. `withGit`, or the step after it, carries the run onto the branch. In the twin, the run is a line under the suite heading: `_Run before this session: \`<command>\`, exit <n>, log \`<log>\`._`. It is read back by the reader, so `twinOf` still round-trips. With no run, the twin is byte-identical to today's.
- `branch-page.ts`: when a run is present, the suite slide shows one line with the command, the exit code and the log path, marked up with the existing `path()` helper.

**Behaviour:**
- In a branch session, the `tests` option is the JSON's path, or `""`. When it's non-empty, `reportSession` reads the file with `$` outside the sandbox, the way `readGit` reads git.
- It refuses with `nothing written: could not read the test run <path>: <why>` when the file is missing or malformed. When the file is good, it runs `checkSuite` and refuses with every reason at once, as `withGit` does.
- With `tests` empty, nothing changes.
- The tool description in `branchSpec` gains one sentence: when the session's prompt names a test run, the suite comes from its log, and the tool checks it against the run's exit code.

- [ ] Write failing tests for: `parseTestRun` (good, missing field, bad JSON); `checkSuite` (both directions, and the consistent cases that pass); the twin round-trip with and without a run; the page showing the run line; and a branch session in `refine.test.ts` with `tests` set (read, refused when inconsistent, written when consistent) and with `tests: ""` (unchanged).
- [ ] Implement, run `claude plugin test .` and `claude plugin validate .`, and commit, for example `feat(report): check a branch report's suite against the test run`.

### Task 13: Explain branch runs the tests first, in a tab of their own

**Files:** Modify:
- `src/main.rs`: the hidden `--here` and `--workspace` on `task report` (only with `--branch`), and the `explain_branch` flow.
- `src/refine.rs`: the `Branch` struct gains `tests: Option<PathBuf>`, `session_settings` sends `tests`, and `branch_prompt` gets a third argument.
- A new `src/branch_tests.rs`, or functions in `src/work.rs` if they fit better: finding the command, running it, writing the files.
- `src/session.rs`: a per-task lock like `hold_setup`, if a separate name is needed so it doesn't collide with Start working's.
- `src/names.rs`: the tab label.

Read `work::launch`, `work::set_up_here` and `work::set_up` first. This task copies their shape.

**Interfaces:**
- `fn test_command(worktree: &Path) -> Result<Option<TestCommand>>`, where `struct TestCommand { shell: String, env: Vec<(String, String)> }`. The order is the Global Constraints' "Which command". Test it with a temp git repo: config set, `tests/all.sh` only, and neither.
- `fn run_tests(worktree: &Path, cmd: &TestCommand, reviews: &Path, uuid: &str, now: SystemTime) -> Result<PathBuf>` runs the command with stdin and stderr inherited and stdout and stderr also teed to the log. The simplest faithful way is `sh -c '<cmd> 2>&1 | tee <log>; exit ${PIPESTATUS[0]}'` under bash, or an in-process tee. It writes the JSON and returns its path. Test it with a tiny command (`echo hi; exit 3`) in a temp folder: check the log's text, `exit_code` 3, and the JSON's fields.
- `branch_prompt(uuid, base, tests: Option<&Path>)` gives `/report-branch <uuid> <base>`, or `/report-branch <uuid> <base> <tests.json>`.

**Flow:**
1. `task report <uuid> --branch` has its checks as now: deleted task refused, linked report opened unless `--fresh`, no worktree refused.
2. If the branch agent is live, focus it, as now.
3. If the task's test lock is held, notify "Already running this task's tests — see its tab."
4. If `test_command` gives none, launch Claude directly as now, with `tests: None`.
5. Otherwise, open a tab in the worktree labelled `Tests: <short description>` (as `tab_label` shortens it). It runs `<exe> task report <uuid> --branch --here --workspace <ws>`. That process:
   1. holds the lock;
   2. runs the tests, so the user sees them live;
   3. starts the Claude session in a new tab, with `tests: Some(json)`, through the same `start`/`open_in` path `launch_branch` uses (claim taken only now, agent looked for first);
   4. closes its own tab.
   On an error it waits for Enter.

The tests' exit code never decides whether Claude starts.

- [ ] Write failing tests with the fake session, alongside the existing branch tests in `src/refine.rs` and `src/work.rs`. Cover:
  - the `--here` command line the tab is given;
  - no second tab while the lock is held;
  - direct launch when there's no command;
  - the prompt and the mod's `tests` setting when there is a command;
  - `test_command` and `run_tests` as above.
- [ ] Implement. Run the whole `cargo test` and `cargo clippy --all-targets`. Update README's Commands and llms.txt only if a flag shows in `--help` (keep `--here` and `--workspace` hidden, as `task start`'s are). Commit, for example `feat(report): run a branch's tests before its report session`.

### Task 14: the skill reads the run, and the docs say so

**Files:** Modify `.claude/skills/report-branch/SKILL.md`, `.claude/skills/report-branch/branch-catalogue.md` (prose only; the JSON fence stays byte-identical to the fixture unless the fixture changes), `CONTEXT.md`, `README.md` and `llms.txt`.

- [ ] The skill's arguments become `<uuid> <base> [<tests.json>]`. Step 3 gets two branches:
  - **With a tests file:** read it and its log. Don't run any suite. Fill `suite.runs` from each suite's own summary in the log. Fill `not_run` from the log's SKIP lines, with their reasons. Fill `failing` from the log's failures.
    - The mod checks the counts against the exit code: exit 0 means nothing failed.
    - Put the command in each run's `command`, as the log shows it.
  - **Without one:** the current text, unchanged.
- [ ] Add a "Tests before the report" paragraph to CONTEXT.md, and update the README Explain branch row. Say that a project chooses its command with `git config niri-tasks.test '<command>'`, and that this repo uses `tests/all.sh` headless.
- [ ] Commit, for example `docs(report): read the test run in report-branch`.

### Task 15: the end-to-end check (controller)

After Tasks 11–14, the branch is still installed from this worktree, so the links point here. Rebuild and reinstall with `bash install.sh` from the worktree; this restarts the daemon. Then ask the user to run Explain branch on task b5447e66 and to watch the tests tab. Check the transcript and the deck:
- the suite counts come from the log;
- no "stopped by the sandbox" entries for `typed_task_number`;
- the e2e suites are listed as skipped headless.
