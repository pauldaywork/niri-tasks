# Run finish-worktree's tests in the container Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In niri-tasks, finish-worktree's step 6 runs `bash tests/container.sh` instead of the host `tests/all.sh`, so every suite runs against the rebased tree and none is skipped for touching the desktop.

**Architecture:** Only text changes: the `finish-worktree` skill (`.claude/skills/finish-worktree/SKILL.md`) gains a container block in step 6 that runs `tests/container.sh`, decides from *this run's* log whether the suites were reached, and falls back to the old host run when they were not. Step 3's lock wait grows to cover a container run, step 10's report names where the tests ran, and README's `/finish-worktree` paragraph says it. `tests/container.sh` itself is not touched.

**Tech Stack:** Markdown skill read by Claude Code, bash, Docker (through `tests/container.sh`).

**Spec:** Taskwarrior task `69c9be9b-8058-4856-bf7a-98411e30e052` (description and notes: `task rc.json.array=on 69c9be9b export`).

## Global Constraints

- Change only step 6's niri-tasks line in the skill (plus step 3's wait and step 10's report); Explain branch's test run (`git config niri-tasks.test`, else headless all.sh) stays as it is.
- When the container run has no all.sh summary line in its log (no Docker, daemon down, image build failed, run refused), fall back to `cargo build` + `NIRITASKS=$PWD/target/debug/niritasks bash tests/all.sh` and say so in the report; a run that reached the summary is the test result, red or green.
- Raise step 3's lock wait from 600s to 1800s.
- A parity warning at the top of the run is named in the report, not acted on: no automatic `NIRITASKS_CONTAINER_REBUILD=1`.
- On red, the check run in `M` to see whether main was already red is also `bash tests/container.sh`.
- Out of scope: Explain branch's Tests: tab and `git config niri-tasks.test`; other repos' test commands; changes to `tests/container.sh`.
- Skills load from `~/Projects/niri-tasks`, never a worktree: the end-to-end check of the skill happens after the merge, not in this branch.

## Facts the engineer needs

- `tests/container.sh` exits **before** it creates or relinks `target/container-tests/latest.log` when Docker is missing, the daemon is unreachable, or a run of the same checkout is already going (`tests/container.sh:43-55`). In those cases `latest.log` still points at the *previous* run's log, which may well end in a summary line. So "read latest.log" must mean "read latest.log if it moved during this run". The block below compares `readlink` before and after.
- `tests/all.sh:148-152` ends every run with `  PASS  <name> (<n>s)` / `  SKIP  <name> — <why>` / `  FAIL  <name>` lines and then the summary `N passed   N failed   N skipped` (three spaces between). Regex: `^[0-9]+ passed +[0-9]+ failed +[0-9]+ skipped$`.
- `tests/container/inside.sh:60-67` prints `parity: the image matches this machine` or `parity: the image differs from this machine (< machine, > image):` followed by indented `<`/`>` lines.
- A container run takes ~460s, more on a worktree's first run (its build volume compiles niri-tasks). Claude Code's foreground Bash call is capped at 600s, so the skill must tell the agent to run the block in the background and wait for it.
- `latest.log` is a relative symlink: `readlink` prints just the file name, e.g. `20261103T104301Z-12345.log`.

## File Structure

- Modify: `.claude/skills/finish-worktree/SKILL.md` — step 3 (lock wait), step 6 (container block + host fallback), step 10 (report).
- Modify: `README.md:264-270` — the `/finish-worktree` paragraph.

---

### Task 1: The skill tests niri-tasks in the container

**Files:**
- Modify: `.claude/skills/finish-worktree/SKILL.md` (step 3 at the `timeout 600` block; step 6 whole; step 10 second bullet)

**Interfaces:**
- Consumes: `tests/container.sh` (unchanged), its `target/container-tests/latest.log` symlink, all.sh's summary line format.
- Produces: the step 6 container block (below), which Task 2's README wording describes.

- [ ] **Step 1: Try the container block's fallback detection before writing it into the skill**

Save this as `$SCRATCH/block.sh` (any scratch directory outside the repo):

```bash
LOGS=target/container-tests
before=$(readlink "$LOGS/latest.log" 2>/dev/null)
bash tests/container.sh 2>&1 | tail -n 3; echo "exit ${PIPESTATUS[0]}"
after=$(readlink "$LOGS/latest.log" 2>/dev/null)
if [ -n "$after" ] && [ "$after" != "$before" ] &&
    grep -Eq '^[0-9]+ passed +[0-9]+ failed +[0-9]+ skipped$' "$LOGS/$after"; then
    grep -E '^parity:|^  [<>] |^  (PASS|SKIP|FAIL)  |^[0-9]+ passed ' "$LOGS/$after"
else
    echo "no test run in the container"
fi
```

Run it from the worktree root with Docker made unreachable:

Run: `DOCKER_HOST=unix:///nonexistent bash "$SCRATCH/block.sh"`
Expected: `docker is not running, or this user cannot reach it`, `exit 1`, `no test run in the container` — and *not* a PASS/summary listing, even if `target/container-tests/latest.log` already exists from an earlier run (check with `ls -l target/container-tests/latest.log`).

- [ ] **Step 2: Try it against a real run**

Run it with Docker up, in the background (it takes ~8 minutes, longer on this worktree's first run): `bash "$SCRATCH/block.sh"`
Expected: `exit 0` (or non-zero if a suite really fails), then a `parity:` line, one `  PASS  ` line per suite (six, as `tests/all.sh:132-144` names them: `cargo test`, `tests/e2e-tag.sh`, `herdr/task-docs tests`, `refine mod tests`, `tests/e2e-panel.sh`, `tests/e2e-box.sh`), no `  SKIP  ` lines, and `6 passed   0 failed   0 skipped`. If a suite is skipped or fails here, stop and report it: the skill change relies on the container running all six.

- [ ] **Step 3: Raise the lock wait in step 3**

In `.claude/skills/finish-worktree/SKILL.md`, replace:

```markdown
**3. Take the lock.** Another agent may be finishing another worktree of the
same repository at the same moment, on the same `M`.
```bash
L="$(git rev-parse --git-common-dir)/finish-worktree.lock"
timeout 600 bash -c "until mkdir '$L' 2>/dev/null; do sleep 5; done" && echo "$(git branch --show-current) $(date -Is)" > "$L/owner"
```
```

with:

```markdown
**3. Take the lock.** Another agent may be finishing another worktree of the
same repository at the same moment, on the same `M`. The holder may be
running the tests, which in niri-tasks take eight minutes or more, so the
wait is half an hour.
```bash
L="$(git rev-parse --git-common-dir)/finish-worktree.lock"
timeout 1800 bash -c "until mkdir '$L' 2>/dev/null; do sleep 5; done" && echo "$(git branch --show-current) $(date -Is)" > "$L/owner"
```
```

The wait itself is past a foreground command's 10-minute limit, so add, as the first bullet under that block:

```markdown
- Run the wait in the background and wait for it to finish: it can outlast a
  foreground command.
```

Leave the stale-lock bullet ("more than an hour old") and the release bullet as they are.

- [ ] **Step 4: Rewrite step 6**

Replace the whole of step 6, from `**6. Test the rebased branch.**` up to (not including) `**7. Merge, then push.**`, with:

````markdown
**6. Test the rebased branch.**
- If the repo's `.config/wt.toml` has `[pre-merge]` hooks, `wt merge` runs them
  in step 7, and that is the test.
- Otherwise run the repo's own check here. Use what README or CLAUDE.md say runs
  everything, such as a `tests/all.sh`, and failing that, the language's
  default (`cargo test`, `npm test`, `pytest`, `go test ./...`).
- Point end-to-end suites at the build of this branch, not an installed copy:
  one that runs the installed binary by default proves nothing about the
  branch.
- **In niri-tasks, test in the container.** `tests/container.sh` runs
  `tests/all.sh` on a desktop of its own, against a copy of this worktree that
  it builds itself and points `NIRITASKS` at, so every suite runs and none
  touches the user's desktop. It takes about eight minutes, longer on a
  worktree's first run: run this in the background and wait for it.
  ```bash
  LOGS=target/container-tests
  before=$(readlink "$LOGS/latest.log" 2>/dev/null)
  bash tests/container.sh 2>&1 | tail -n 3; echo "exit ${PIPESTATUS[0]}"
  after=$(readlink "$LOGS/latest.log" 2>/dev/null)
  if [ -n "$after" ] && [ "$after" != "$before" ] &&
      grep -Eq '^[0-9]+ passed +[0-9]+ failed +[0-9]+ skipped$' "$LOGS/$after"; then
      grep -E '^parity:|^  [<>] |^  (PASS|SKIP|FAIL)  |^[0-9]+ passed ' "$LOGS/$after"
  else
      echo "no test run in the container"
  fi
  ```
  - A run that printed the summary line (`N passed N failed N skipped`) is the
    test result, red or green: `0 failed` is green, anything else red. The exit
    status alone can't tell a failed suite from a run that never started.
  - `latest.log` moves only once a run gets as far as its log. A run turned
    away at the door (no Docker, a run of this checkout already going) leaves
    the last run's log there, so the block reads it only if it moved.
  - `no test run in the container` (no Docker, the daemon down, the image
    build failing, the run refused) means the host run below, and the report
    says so, with the reason from the lines above it.
  - A `parity: the image differs` line, with the differences under it, goes in
    the report. Don't rebuild the image for it: that takes minutes and needs
    the network.
- **On the host** (any other repo, or niri-tasks when the container did not
  run): in niri-tasks that is `cargo build`, then `NIRITASKS=$PWD/target/debug/niritasks bash
  tests/all.sh`. Run a suite that sandboxes itself: one that types only into a
  nested compositor of its own, or borrows a spare workspace and gives it back.
  Leave out only a suite that types or clicks on the user's own desktop, or
  uses their clipboard, and name it in the report as skipped.
- Red means stop. Nothing has been merged yet. Before asking, run the same
  check in `M`, so you can say whether `main` was already red without this
  branch or the branch broke it. In niri-tasks that is the container block
  again, run from `M` (`cd` there in the same command), falling back to the
  host run as above.
````

- [ ] **Step 5: Name where the tests ran in step 10's report**

Replace the bullet:

```markdown
- the tests that ran and their result, and the suites skipped
```

with:

```markdown
- the tests that ran and their result, and the suites skipped; in niri-tasks,
  whether they ran in the container or on the host, and why it fell back, and
  any parity difference the container named
```

- [ ] **Step 6: Check the skill reads straight and the block is the one tried**

Run: `grep -n 'timeout 1800\|tests/container.sh\|no test run in the container\|on the host' .claude/skills/finish-worktree/SKILL.md`
Expected: `timeout 1800` in step 3; the container block, the fallback and the M check in step 6; the report bullet in step 10. `grep -c 'timeout 600' .claude/skills/finish-worktree/SKILL.md` prints `0`.

Extract the block from the skill and diff it with the one tried in Steps 1–2:

Run: `awk '/^  LOGS=target\/container-tests/{f=1} f{print} /^  fi$/{if(f)exit}' .claude/skills/finish-worktree/SKILL.md | sed 's/^  //' | diff - "$SCRATCH/block.sh" && echo same`
Expected: `same`.

Read steps 3, 6 and 10 top to bottom once more: the Explain-branch test run, other repos' commands and `tests/container.sh` are untouched (`git diff --stat` shows only `SKILL.md`).

- [ ] **Step 7: Commit**

```bash
git add .claude/skills/finish-worktree/SKILL.md
git commit -m "test(finish-worktree): run niri-tasks' suites in the container

Step 6 now runs tests/container.sh, so every suite runs against the
rebased tree and none is left out for touching the desktop. A run that
never reached all.sh's summary falls back to the host all.sh, and the
report says which ran. The lock wait grows to 30 minutes to cover a
container run held by another finisher.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: README says /finish-worktree tests in the container

**Files:**
- Modify: `README.md:264-270` (the paragraph beginning "When the work is done, `/finish-worktree`")

**Interfaces:**
- Consumes: Task 1's behaviour (container first, host all.sh when the container can't run).
- Produces: nothing later tasks use.

- [ ] **Step 1: Rewrite the paragraph's test clause**

Replace:

```markdown
When the work is done, `/finish-worktree` in that worktree's Claude lands it:
it syncs `main` with origin, rebases the branch and tests the result, then
fast-forwards `main`, pushes,
```

with:

```markdown
When the work is done, `/finish-worktree` in that worktree's Claude lands it:
it syncs `main` with origin, rebases the branch and tests the result in a
container (`tests/container.sh`, every suite, nothing on your desktop touched;
`tests/all.sh` on the host when Docker can't run it, and it says so), then
fast-forwards `main`, pushes,
```

Leave the rest of the paragraph as it is.

- [ ] **Step 2: Check it**

Run: `sed -n 264,272p README.md`
Expected: the paragraph reads as one sentence through "removes the worktree.", with the container clause in it; `git diff --stat` shows only `README.md`.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs(readme): say /finish-worktree tests in the container

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## After the merge (not in this branch)

Skills load from `~/Projects/niri-tasks`, so the task's "Done when" is checked once this branch has landed:

1. Finish a niri-tasks worktree with Docker up: the report says the tests ran in the container, all six suites passed, none skipped.
2. Finish one with Docker stopped (`sudo systemctl stop docker docker.socket`, start it again after): the report says the container did not run, why, and that `tests/all.sh` ran on the host.
3. Start a second finisher while a first is in its container run: it waits on the lock and goes on once the first releases it, without timing out.
