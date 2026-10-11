# Link the Plan Doc from a Started Task Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A task worked through Start working links the plan document that writing-plans saves, as a `Plan: <path>` note, the way a refine report is linked by `Report: <path>`, so the doc can be found from the task later.

**Architecture:** Two places add the note. `plan_prompt` in `src/work.rs`, the first thing the worktree Claude is told, gains a sentence: once the plan is saved, link it with `niritasks task note <uuid> "Plan: <repo-relative path>"`, and any spec as `Spec: …`. The `finish-worktree` skill gains a backstop in step 8: after marking the task completed, it lists the docs the landed range added under `docs/superpowers/plans` and `docs/superpowers/specs` with `git diff --name-only --diff-filter=A`, and adds a `Plan:` or `Spec:` note for each one the task's notes do not already carry. The docs (llms.txt, README, CONTEXT.md) describe the note. No Rust beyond the prompt string and its test changes.

**Tech Stack:** Rust (`cargo test`), a Markdown skill file, Taskwarrior via `niritasks task note`.

**Spec:** Taskwarrior task `a652c83c-7c71-47e4-9091-207594fb9aad`, "feat: Link the plan doc from a started task". Read it with `task rc.json.array=on a652c83c-7c71-47e4-9091-207594fb9aad export`; its notes are the spec and are quoted in **Global Constraints**.

## Global Constraints

- **The note's shape:** `Plan: docs/superpowers/plans/<file>.md` and `Spec: docs/superpowers/specs/<file>.md`. The path is repo-relative, the same in the worktree and on `main` after finish-worktree removes the worktree. Never an absolute worktree path (goes stale) and never a main-checkout path (does not exist until the branch lands).
- **How the note is added:** `niritasks task note <uuid> "Plan: <path>"`, never raw `task annotate` (llms.txt rule 2: niritasks passes `--`, so a path is never read as an attribute).
- **Two writers, one note:** the prompt tells the worktree Claude to add the note as soon as the plan is saved; finish-worktree step 8 adds a note for each plan or spec doc the branch added that the task does not already link. It never duplicates a note already there.
- **Which docs count:** files the landed range *added* (`--diff-filter=A`) under `docs/superpowers/plans` and `docs/superpowers/specs`. There is one doc to link, two if a spec was written. Nothing else.
- **Done when:** a task started with Start working has a `Plan: docs/superpowers/plans/<file>.md` note once its Claude saves the plan; finish-worktree adds the note when it is missing and never duplicates one already there; `cargo test` passes with the prompt test updated; llms.txt, README and CONTEXT.md describe the note.
- **Out of scope:** a card button or CLI command that opens the plan; a mod or hook that enforces the note in the Start session; linking an implementation document.
- **Skill changes only run after landing:** the `finish-worktree` skill a session loads comes from `~/Projects/niri-tasks`, so the new step 8 is checked by hand on this branch (Task 2 step 4) and for real when the next task worktree lands.
- **Commits:** Conventional Commits, subject at most 72 characters, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. The task's type is `feat`; the skill commit is `feat(finish-worktree)`, the docs commit `docs`.
- **Tests stay green at every commit.**

## File Structure

- `src/work.rs` (modify): `plan_prompt()` at lines 62–70 gains the linking sentence; the assertion in `the_branch_is_named_after_the_task` at lines 355–358 is updated.
- `.claude/skills/finish-worktree/SKILL.md` (modify): step 7 records `<old>` before merging; step 8 gains the note backstop; step 10 reports the notes added.
- `llms.txt` (modify): the `niritasks task start <uuid>` row (line 47) and the **Finishing work** paragraph (line 55).
- `README.md` (modify): the **Start working** button row (line 53) and the `/finish-worktree` paragraph (lines 206–210).
- `CONTEXT.md` (modify): a **Plan note** entry after **Refine report** (after line 63).

---

### Task 1: The prompt asks for the note

**Files:**
- Modify: `src/work.rs:62-70` (`plan_prompt`)
- Test: `src/work.rs:352-359` (`the_branch_is_named_after_the_task`)

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub fn plan_prompt(uuid: &str) -> String`, unchanged signature, longer text. `start_task` at `src/work.rs:222` keeps calling it as before.

- [ ] **Step 1: Update the test to expect the new prompt**

In `src/work.rs`, replace the `plan_prompt` assertion inside `the_branch_is_named_after_the_task` (lines 355–358) with:

```rust
        assert_eq!(
            plan_prompt("u-1"),
            "/superpowers:writing-plans Plan Taskwarrior task u-1. Read it with `task rc.json.array=on u-1 export`; its description and notes are the spec. Once the plan is saved, link it from the task with `niritasks task note u-1 \"Plan: <path relative to the repo root>\"`, and any spec you write with `niritasks task note u-1 \"Spec: <path relative to the repo root>\"`; never raw `task annotate`."
        );
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --bin niritasks the_branch_is_named_after_the_task`
Expected: FAIL with an `assertion `left == right` failed` whose right side holds the new sentence.

- [ ] **Step 3: Change the prompt**

Replace `plan_prompt` and its doc comment (lines 62–70) with:

```rust
/// What the working Claude is asked first: plan the task, reading the task
/// itself for the spec — only the uuid crosses, as with Refine, so nothing
/// needs quoting through herdr and it always sees the current version — and
/// link the saved plan from the task as `Plan: <path>`, the way a refine
/// report is linked by `Report: <path>`. The path is repo-relative, so it
/// holds in the worktree and on `main` once the branch lands and the
/// worktree is gone. `finish-worktree` adds the note if this is forgotten.
pub fn plan_prompt(uuid: &str) -> String {
    format!(
        "/superpowers:writing-plans Plan Taskwarrior task {uuid}. Read it with \
         `task rc.json.array=on {uuid} export`; its description and notes are the spec. \
         Once the plan is saved, link it from the task with \
         `niritasks task note {uuid} \"Plan: <path relative to the repo root>\"`, \
         and any spec you write with \
         `niritasks task note {uuid} \"Spec: <path relative to the repo root>\"`; \
         never raw `task annotate`."
    )
}
```

- [ ] **Step 4: Run the test to verify it passes, then the whole suite**

Run: `cargo test --bin niritasks the_branch_is_named_after_the_task`
Expected: PASS.

Run: `cargo test`
Expected: everything passes.

- [ ] **Step 5: Commit**

```bash
git add src/work.rs
git commit -m "feat(work): ask the started task's Claude to link its plan

Start working's prompt now tells the worktree Claude to add a
\`Plan: <repo-relative path>\` note once writing-plans saves the plan,
and a \`Spec:\` note for any spec, through \`niritasks task note\`, so
the document can be found from the task later the way a refine report
is found by its \`Report:\` note. The path is repo-relative because the
worktree is removed when the branch lands.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: finish-worktree links what the branch added

**Files:**
- Modify: `.claude/skills/finish-worktree/SKILL.md` (steps 7, 8 and 10)

**Interfaces:**
- Consumes: the `Plan:` / `Spec:` note shape from Global Constraints. The task uuid8 from step 1 of the skill.
- Produces: the step-8 shell that later finishes run. Nothing in code depends on it.

- [ ] **Step 1: Record `<old>` in step 7**

In `.claude/skills/finish-worktree/SKILL.md`, replace the step 7 code block:

````markdown
**7. Merge, then push.**
```bash
wt merge --no-squash --no-remove   # fast-forwards main to the branch, every commit kept
git -C M push origin main
```
````

with:

````markdown
**7. Merge, then push.**
```bash
OLD=$(git -C M rev-parse main)     # what main was: steps 8 and 10 use <old>..<new>
wt merge --no-squash --no-remove   # fast-forwards main to the branch, every commit kept
git -C M push origin main
NEW=$(git -C M rev-parse main)
```
````

- [ ] **Step 2: Add the backstop to step 8**

Replace the whole of step 8:

````markdown
**8. Mark the task completed.** The work is on origin now, so it's done, even if
cleanup fails.
```bash
niritasks task status <uuid8> completed
```
Confirm with `task rc.json.array=on <uuid8> export` that its status is now
`completed`. Always go through `niritasks`, never `task done`: niritasks is the
one path a task card's Complete also uses, and it sends the same notification.
````

with:

````markdown
**8. Mark the task completed, and link its plan.** The work is on origin now, so
it's done, even if cleanup fails.
```bash
niritasks task status <uuid8> completed
```
Confirm with `task rc.json.array=on <uuid8> export` that its status is now
`completed`. Always go through `niritasks`, never `task done`: niritasks is the
one path a task card's Complete also uses, and it sends the same notification.

Then link the plan and spec documents the branch added, so they can be found
from the task later. The Claude that planned the task was asked to add a
`Plan: <path>` note when it saved the plan; this is the backstop from git for
when it didn't.
```bash
NOTES=$(task rc.json.array=on <uuid8> export | jq -r '.[0].annotations[]?.description')
git -C M diff --name-only --diff-filter=A "$OLD..$NEW" -- docs/superpowers/plans docs/superpowers/specs |
while read -r p; do
  case "$p" in docs/superpowers/specs/*) note="Spec: $p" ;; *) note="Plan: $p" ;; esac
  if grep -qxF "$note" <<<"$NOTES"; then echo "already linked: $note"
  else niritasks task note <uuid8> "$note" && echo "linked: $note"; fi
done
```
- The path is repo-relative, exactly as `git diff` prints it: the worktree is
  about to be removed, so a worktree path would go stale.
- Only files the range *added* count. A plan the branch edited was linked when
  it was written.
- Never `task <uuid8> annotate`: a path can look like an attribute, and
  `niritasks task note` passes `--` so it never does.
- Nothing printed means the branch added no plan or spec, which is normal for a
  branch that was not planned with writing-plans. Say so in the report.
````

- [ ] **Step 3: Name the notes in step 10**

Replace, in step 10:

```markdown
- the task completed (or "no task: branch isn't `task/…`")
```

with:

```markdown
- the task completed (or "no task: branch isn't `task/…`")
- the `Plan:` and `Spec:` notes added, the ones already there, or "no plan or
  spec doc added"
```

- [ ] **Step 4: Dry-run the step-8 shell against this branch**

The skill a session loads comes from `~/Projects/niri-tasks`, so the new step
only runs for real once this lands. Prove the shell now, with the branch's own
range in place of `$OLD..$NEW` and the task's real uuid8, and `niritasks task note`
replaced by `echo` so nothing is written:

```bash
NOTES=$(task rc.json.array=on a652c83c export | jq -r '.[0].annotations[]?.description')
git diff --name-only --diff-filter=A "$(git merge-base main HEAD)..HEAD" -- docs/superpowers/plans docs/superpowers/specs |
while read -r p; do
  case "$p" in docs/superpowers/specs/*) note="Spec: $p" ;; *) note="Plan: $p" ;; esac
  if grep -qxF "$note" <<<"$NOTES"; then echo "already linked: $note"
  else echo "WOULD RUN: niritasks task note a652c83c \"$note\""; fi
done
```

Expected: one line for `docs/superpowers/plans/2026-10-11-link-the-plan-doc.md`.
It reads `already linked:` if the task already carries that `Plan:` note (the
planning session adds it), otherwise `WOULD RUN:`. No other line. Both outcomes
prove the shell; what matters is that the path is repo-relative and the
duplicate check matches the note byte for byte.

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/finish-worktree/SKILL.md
git commit -m "feat(finish-worktree): link the plan and spec docs a branch added

After marking the task completed, step 8 lists the files the landed
range added under docs/superpowers/plans and docs/superpowers/specs and
adds a \`Plan:\` or \`Spec:\` note for each one the task's notes do not
already carry, through \`niritasks task note\`. The planning Claude is
asked to add the note itself; this is the backstop from git that cannot
be forgotten. Step 7 records the pre-merge main so the range is known,
and step 10 names what was linked.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: Describe the note in the docs

**Files:**
- Modify: `llms.txt:47` (the `task start` row) and `llms.txt:55` (**Finishing work**)
- Modify: `README.md:53` (the **Start working** row) and `README.md:206-210` (the `/finish-worktree` paragraph)
- Modify: `CONTEXT.md` after line 63 (a **Plan note** entry under **Refine report**)

**Interfaces:**
- Consumes: the note shape and the two writers from Tasks 1 and 2.
- Produces: nothing in code.

- [ ] **Step 1: llms.txt**

Replace the `task start` row (line 47):

```markdown
| `niritasks task start <uuid>` | Make the task's own git worktree, branch `task/<slug>-<uuid8>`, and open it in a herdr tab with Claude planning it; run again, go back to both; it also marks the task active, so do not run `task status <uuid> active` after it | herdr tab, follows the herdr session, else focus |
```

with:

```markdown
| `niritasks task start <uuid>` | Make the task's own git worktree, branch `task/<slug>-<uuid8>`, and open it in a herdr tab with Claude planning it; that Claude links the plan it saves as a `Plan: docs/superpowers/plans/<file>.md` note (and a spec as `Spec: docs/superpowers/specs/<file>.md`), repo-relative, through `niritasks task note`; run again, go back to both; it also marks the task active, so do not run `task status <uuid> active` after it | herdr tab, follows the herdr session, else focus |
```

Replace the **Finishing work** paragraph (line 55):

```markdown
**Finishing work.** A task started with `niritasks task start` gets its own worktree. When the work is done, the `finish-worktree` skill lands it: it rebases, tests, fast-forwards `main`, pushes, runs `niritasks task status <uuid8> completed` and removes the worktree.
```

with:

```markdown
**Finishing work.** A task started with `niritasks task start` gets its own worktree. When the work is done, the `finish-worktree` skill lands it: it rebases, tests, fast-forwards `main`, pushes, runs `niritasks task status <uuid8> completed`, adds a `Plan:` or `Spec:` note for each doc the branch added under `docs/superpowers/plans` or `docs/superpowers/specs` that the task does not already link, and removes the worktree.
```

- [ ] **Step 2: README.md**

Replace the **Start working** row (line 53):

```markdown
| **Start working** (green) | `s` | The task's own worktree, herdr tab and Claude — not on an active task, which is already being worked |
```

with:

```markdown
| **Start working** (green) | `s` | The task's own worktree, herdr tab and Claude, which plans the task first and links the plan it saves as a `Plan: docs/superpowers/plans/<file>.md` note — not on an active task, which is already being worked |
```

Replace the `/finish-worktree` paragraph (lines 206–210):

```markdown
When the work is done, `/finish-worktree` in that worktree's Claude lands it:
it syncs `main` with origin, rebases the branch and tests the result, then
fast-forwards `main`, pushes, marks the task completed with `niritasks task status`,
and removes the worktree. It stops to ask only for a conflict with no obvious
resolution, red tests, or `main` holding commits origin doesn't have.
```

with:

```markdown
When the work is done, `/finish-worktree` in that worktree's Claude lands it:
it syncs `main` with origin, rebases the branch and tests the result, then
fast-forwards `main`, pushes, marks the task completed with `niritasks task status`,
links any plan or spec doc the branch added that the task doesn't yet carry as a
`Plan:` or `Spec:` note, and removes the worktree. It stops to ask only for a
conflict with no obvious resolution, red tests, or `main` holding commits origin
doesn't have.
```

- [ ] **Step 3: CONTEXT.md**

After the **Refine report** entry's `_Avoid_` line (line 63) and its blank line, insert:

```markdown
**Plan note**:
A `Plan: docs/superpowers/plans/<file>.md` note on a task worked through Start
working, linking the implementation plan the `writing-plans` skill saved, the
way a refine report is linked by `Report: <path>`; a spec, when one is written,
is linked the same way as `Spec: docs/superpowers/specs/<file>.md`. The path is
repo-relative, so it holds once the worktree is gone. The worktree's Claude
adds it when it saves the plan, and `finish-worktree` adds any the branch added
that the task doesn't yet carry. Added with `niritasks task note`, never raw
`task annotate`.
_Avoid_: plan link, plan annotation

```

- [ ] **Step 4: Check the docs still read as a whole**

Run:
```bash
grep -n "Plan: " llms.txt README.md CONTEXT.md
cargo test
```
Expected: each of the three files matches at least once; `cargo test` passes (nothing in it reads these files, this just confirms the tree is still green before the commit).

- [ ] **Step 5: Commit**

```bash
git add llms.txt README.md CONTEXT.md
git commit -m "docs: describe the Plan note a started task carries

llms.txt's task start row and Finishing work paragraph, README's Start
working row and finish-worktree paragraph, and a Plan note entry in
CONTEXT.md say that the worktree's Claude links its plan as a
repo-relative \`Plan:\` note and that finish-worktree adds any the
branch added that the task does not yet carry.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

## Self-review

- **Spec coverage.** Step (1) of the spec is Task 1: the prompt and its test. Step (2) is Task 2: step 8 lists the added docs with `git -C M diff --name-only --diff-filter=A <old>..<new> -- …`, adds the missing notes, and step 10 names them. Step (3) is Task 3: llms.txt's `task start` row and Finishing work paragraph, README's Start working line (and its finish paragraph, so the two writers are both described), and a Plan note entry under Refine report in CONTEXT.md. The Decided notes (both writers, repo-relative path, `niritasks task note`) are Global Constraints and appear in each task's text. Done when: covered by Task 1 step 4, Task 2 step 4 and Task 3 step 4.
- **Placeholders.** None. Every step carries its text or code.
- **Type consistency.** `plan_prompt(uuid: &str) -> String` is unchanged. The note text `Plan: <path>` and `Spec: <path>` is spelled the same in the prompt, the skill, and all three docs.
