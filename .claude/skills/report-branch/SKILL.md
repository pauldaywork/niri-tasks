---
name: report-branch
description: >
  Explain what a Taskwarrior task's worktree branch has changed so far, how
  it works and how it is tested, as an HTML deck opened in the browser and
  linked from the task as a Branch report: note, after running the project's
  test suite. Invoked by the Explain branch row of niri-tasks' task card's Go
  to list as `/report-branch <uuid> <base>`.
disable-model-invocation: true
argument-hint: <uuid> <base>
---

# report-branch: explain what a task's branch has changed

Arguments: `$ARGUMENTS`: a task uuid, then the base branch the task's
branch is measured against (the repository's default branch).

This session runs in the task's worktree. Your job is to explain its branch
to someone who has not read the diff: what it changes, how the new code
works, how it is tested, where a reviewer should look hardest, and how far it
has got with the task's plan. You do not review it for merging, fix it, or
carry on the task.

**You report on the branch; you never change it.** No file edits, no
commits, no config changes. The task's one change is the `Branch report:
<path>` note the report tool adds itself. You must not change the task any
other way: no `task modify`, no `task annotate`, no `task import`.

Your Bash commands run in a sandbox that can read anything but secrets and
write only to the task database and `$TMPDIR`. The worktree is read-only to
you: every build and test output goes under `$TMPDIR`. Commands reach a
fixed list of source and docs hosts and no others. A command that fails with
`Read-only file system`, `Operation not permitted`, or a refused connection
hit that fence on purpose: do not look for a way around it, and do not ask
the user to lift it.

## 1. Read the task and the branch

```bash
task rc.json.array=on <uuid> export
```

Stop and say so if it returns `[]` or its `status` is `deleted`. Show the
user the description and every note. Then select the report tool with
ToolSearch (`select:mcp__niri-tasks-refine__show_task_report`). If it is not
there, stop and tell the user the refine mod did not load: they should run
`install.sh` and check `claude --version` is 2.1.287 or later.

Measure the branch against the merge base with the base branch, committed
work only:

```bash
mb=$(git merge-base HEAD <base>)
git log --reverse --format='%h %s' "$mb"..HEAD
git diff --stat "$mb" HEAD
git diff "$mb" HEAD -- <file>       # each file worth reading
git status --short                   # uncommitted: the report leaves it out
```

Stop and say so if the log is empty: the branch has nothing to report yet.

## 2. Ground it

Read what the diff touches until you could explain it: `CONTEXT.md`,
`AGENTS.md`/`CLAUDE.md`, the code around each change, and the plan or spec
the task's notes link (`Plan:`, `Spec:`). Facts you can look up are yours to
find; never ask the user for one.

## 3. Run the tests

Find the project's suites: the README's testing section, `AGENTS.md` or
`CLAUDE.md`, a `tests/all.sh` or `Makefile` target, CI config, and the
manifests (`Cargo.toml`, `package.json`, `pyproject.toml`). Run every suite
that runs headless, with its build outputs under `$TMPDIR`:

- Rust: `CARGO_TARGET_DIR="$TMPDIR/target" cargo test --locked --no-fail-fast`
  (without `--no-fail-fast`, one test binary the fence stops hides every
  binary after it)
- Python: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s <dir>`
- A Claude Code mod (a folder with `.claude-plugin/plugin.json`, or tests that
  import `claude-code/testing`): `claude plugin test <dir>`, never plain
  `bun test`, which cannot load `claude-code/testing` and fails every file;
  point any cache under `$TMPDIR`
- Other Bun code: `bun test`

Run them all in one background command, each suite's output to its own log
under `$TMPDIR`, and read the diff while they run. A first cargo build takes
minutes: wait for the command's completion notice, never `sleep`, which the
harness refuses.

Never run a suite that needs a display, a nested compositor, a real
workspace or a network host outside the fence: an e2e script is named, not
run. A suite the sandbox stops (a write outside `$TMPDIR`, a refused host)
is not run either, with the reason; never count it as failed. The same holds
for a single test in a suite that does run: one failing with `Read-only file
system`, `Operation not permitted`, `EACCES`, a socket or bind error under
`/tmp` or `/run`, or its own message that it could not make a path outside
`$TMPDIR`, was stopped by the fence, not broken by the branch. Leave it out
of `failed` and `failing`, and list it under `not_run` with "stopped by the
sandbox" and the test's name as the why. For each suite run, take the passed
and failed counts from its own summary, and name each failing test.

Then find the tests the branch adds or changes (`git diff "$mb" HEAD` on the
test files and the `#[test]`, `def test_` or `test(` lines it adds), and for
each: its layer (unit, integration, e2e), the behaviour it pins, and its
result from the runs. Note any changed code that no test covers.

## 4. Fill the report

Read `branch-catalogue.md` beside this skill
(`~/.claude/skills/report-branch/branch-catalogue.md` when installed), and
the lenses, diagram and writing sections of the refine report catalogue it
points to. Then fill every field from what you read and ran:

- `title` is the task's description, as read in step 1.
- Name commits by sha alone: the tool reads each subject, the branch, its
  base and its numbers from git, and refuses a story that leaves a commit
  out.
- `plan_vs_done` takes each `Steps:` and `Done when:` note the task has,
  split into its items; none when it has neither.
- The tool shows the task's description and notes on the last slide; do not
  repeat them.

## 5. Show it

Call `mcp__niri-tasks-refine__show_task_report` with the fields. The tool
reads git, builds the deck, saves it under the reviews folder, opens it in
the browser, and adds a `Branch report: <path>` note to the task. Its answer
says what happened:

- **Wrote the branch report to … and linked it from the task**: go to
  step 6.
- **show_task_report: nothing written: the task is deleted** or **… no
  longer exists**: stop and tell the user.
- **show_task_report: nothing written: git … failed**: stop and tell the
  user the tool's answer.
- Any other **nothing written:** answer that is not about a field (the
  task's own description or notes need making one line, the report has no
  branch fields): relay it to the user and stop; editing fields cannot fix
  it.
- **show_task_report: this session's branch report is made and linked**:
  the report is done already; go to step 6.
- Any other **show_task_report: …** answer lists every field over its limit,
  not allowed, or not matching git: fix them all and call again.
- **Wrote the branch report to … but task import failed**: tell the user the
  path and the error, and stop.
- **not armed** or **show_task_report failed**: do not retry and do not
  write anything any other way: tell the user the tool's answer and stop.

## 6. Verify and report

Export the task once more and check: the description and the earlier notes
are as you read them in step 1 (a `Report:` note among them untouched), the
last note is `Branch report: <path>` with the path the tool answered, and
the tags are unchanged. Tell the user the path in one line, and stop. Leave
the session open: choosing Branch report in the Go to list opens the deck
again, and to make another they close this tab first.

Always address the task by uuid, never its numeric id.
