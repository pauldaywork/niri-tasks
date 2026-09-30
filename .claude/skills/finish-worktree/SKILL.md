---
name: finish-worktree
description: >
  Use when the work in a git worktree branch is finished and the user wants it
  landed on the default branch and wrapped up, including a task worktree made by
  `niritasks task start`. Triggers on: "/finish-worktree", "finish this
  worktree", "finish up", "land this task", "merge and push this branch",
  "wrap up the worktree", "we're done here, merge it".
---

# finish-worktree: land a worktree branch

The user runs one agent per task, each in its own worktree made by worktrunk
(`wt`), often several at once. This lands one of them without the user doing
anything, and stops only where a decision is really theirs.

**Keep going.** Don't ask between steps, and don't offer a menu. The stops are
listed at the end, and there are no others. When one of them happens, stop,
say what state everything was left in, and ask.

## The shape of it

Two rules decide almost everything below:

- **Conflicts get resolved in the branch, never in `main`.** The branch is
  rebased onto an up-to-date `main`, and then `main` only ever fast-forwards.
  No merge commits, and no conflict resolution in the main checkout.
- **What gets pushed is what was tested.** The tests run on the rebased branch,
  the exact tree `main` is about to point at. A test run from before the rebase
  proves nothing about it.

## Steps

Run these from the branch's worktree. `M` is the main checkout.

**1. Identify.**
```bash
wt list --format=json | jq -r '.repo.default_branch, (.items[] | select(.worktree.main) | .worktree.path)'
git branch --show-current
```
- The first line is the base branch (`main` below).
- The second line is `M`.
- The branch must not be the base branch.
- A branch named `task/<slug>-<uuid8>` belongs to a Taskwarrior task. Look it up
  with `task rc.json.array=on <uuid8> export`. If it's missing or already
  completed, say so in the report and skip step 8.
- Any other branch name has no task, so don't look one up. Step 8 is skipped,
  and the report says so.

**2. Preflight.** Nothing has changed yet, so any failure here is a clean stop.
- `git status --porcelain` in the worktree is empty. Uncommitted work at finish
  time is a surprise. Committing it blind can sweep scratch files onto `main`,
  so stop and show it.
- `M` is on the base branch, and `git -C M status --porcelain --untracked-files=no`
  is empty. Changes there are the user's own work in progress.
- The branch was cut from `main`. No commit of its own may sit on any other
  local branch:
  ```bash
  for c in $(git rev-list main..HEAD); do git branch --contains "$c" --format='%(refname:short)'; done | sort -u
  ```
  This must print only this branch. If another branch holds its commits, this
  branch was cut from that one, and landing it would publish that branch's work
  too. Sibling worktrees' branches cut from `main` don't show up here, and
  aren't a problem.

**3. Take the lock.** Another agent may be finishing another worktree of the
same repository at the same moment, on the same `M`.
```bash
L="$(git rev-parse --git-common-dir)/finish-worktree.lock"
timeout 600 bash -c "until mkdir '$L' 2>/dev/null; do sleep 5; done" && echo "$(git branch --show-current) $(date -Is)" > "$L/owner"
```
- If it times out, read `$L/owner`. An owner more than an hour old is a stale
  lock from a run that died: remove it with `rm -r "$L"` and take it again.
  Otherwise stop.
- **Release it (`rm -r "$L"`) at every exit after this point**: after the push,
  and at any stop.

**4. Bring `main` level with origin.**
```bash
git -C M fetch origin
git -C M rev-list --left-right --count main...origin/main   # "<ahead> <behind>"
```
- `0 0`: already level.
- `0 N`: behind only. Fast-forward it with `git -C M merge --ff-only origin/main`.
- First number above 0: `main` has commits that aren't on origin and aren't
  this branch's, so stop. List them with `git -C M log --oneline origin/main..main`.

**5. Rebase the branch onto `main`:** `wt step rebase`. On a conflict, see
[Conflicts](#conflicts). Once it's resolved, `git add` the files, then run
`GIT_EDITOR=true git rebase --continue` (there is no terminal for an editor).

**6. Test the rebased branch.**
- If the repo's `.config/wt.toml` has `[pre-merge]` hooks, `wt merge` runs them
  in step 7, and that is the test.
- Otherwise run the repo's own check here. Use what README or CLAUDE.md say runs
  the tests, and failing that, the language's default (`cargo test`,
  `npm test`, `pytest`, `go test ./...`).
- Leave out any suite that drives the real screen, keyboard or clipboard, such
  as a `tests/e2e-*.sh` that types with `wtype`. Name it in the report as
  skipped.
- Red means stop. Nothing has been merged yet. Before asking, run the same
  check in `M`, so you can say whether `main` was already red without this
  branch or the branch broke it.

**7. Merge, then push.**
```bash
wt merge --no-squash --no-remove   # fast-forwards main to the branch, every commit kept
git -C M push origin main
```
- Never pass `--yes`, and never force-push.
- If `wt merge` stops for a hook approval, stop and ask the user to run that
  command themselves, with `!` in front.
- If `main` or origin moved in the meantime (the merge refuses a
  non-fast-forward, or the push is rejected), go back to step 4. Do this at most
  twice, then stop.
- Release the lock once the push has succeeded.

**8. Mark the task completed.** The work is on origin now, so it's done, even if
cleanup fails.
```bash
niritasks task status <uuid8> completed
```
Confirm with `task rc.json.array=on <uuid8> export` that its status is now
`completed`. Always go through `niritasks`, never `task done`: niritasks is the
one path the task menu also uses, and it sends the same notification.

**9. Clean up.** This deletes the worktree the shell is standing in, so from
here on run everything with `-C M`.
```bash
wt remove --foreground -C M <branch>        # runs the repo's pre-remove hooks
git -C M ls-remote --exit-code --heads origin <branch> && git -C M push origin --delete <branch>
```
When `ls-remote` exits 2, the branch was never pushed, and there's nothing to
delete. That's normal. Any other failure here is reported, not retried. The
work has already landed.

**10. Report**, in this order:
- what landed: `<old>..<new>` on `origin/main`, and how many commits
- the tests that ran and their result, and the suites skipped
- each conflict resolved, and how
- the task completed (or "no task: branch isn't `task/…`")
- anything left behind

Don't close the herdr workspace. This session runs in it, and closing it would
end the session before the user reads the report. Say that closing it tidies it
away.

## Conflicts

| Resolve it yourself | Stop and ask |
|---|---|
| Both sides added separate things: lines, list items, imports, functions, tests, doc sections. Keep both, in a sensible order. | Both sides changed the same line or logic in different ways. |
| Whitespace or formatting only | One side deleted what the other side edited |
| A lockfile: take `main`'s, then regenerate it | Any resolution that changes behaviour, or picks one side's intent over the other's |
| Code one side moved and the other side left alone | More than a handful of conflicting hunks |

The commit messages say what each side meant, so read them: `git log -1 <sha>`
for each side. Two messages giving opposite reasons for the same line mean it's
a stop.

When you stop:
- leave the rebase open, so the answer can be applied straight away
- release the lock
- show the file, both sides, each side's commit message, and the resolution you
  would pick

When the user answers, apply it, run `GIT_EDITOR=true git rebase --continue`,
then carry on **from step 3**. The stop released the lock, and `main` may have
moved since, so take the lock again, re-sync and re-rebase. Only then test.

## The stops (and only these)

| Stop | Left behind | Resumes with |
|---|---|---|
| The worktree or `M` is dirty | nothing changed | commit or clean it up, then rerun |
| `main` is ahead of origin | nothing changed | push or drop those commits, then rerun |
| The branch was cut from another branch | nothing changed | land that branch first |
| A conflict that isn't obvious | rebase open in the worktree | the user's answer, then step 3 |
| Tests red after the rebase | the branch is rebased, and `main` is at most fast-forwarded to origin | fix it (on the branch, or on `main` if it was red already), then rerun |
| `wt` wants a hook approval | as step 7 found it | the user runs the command with `!` |
| `main` or origin moved three times | the branch is rebased, `main` level with origin | rerun |

At every stop, the lock has been released.

## Mistakes this prevents

- **Resolving the conflict in `main` with a merge commit**, or merging `main`
  into the branch. Rebase the branch, then fast-forward.
- **Pushing without testing the merged tree.** The branch's own test run
  happened before the rebase, so it doesn't count.
- **Marking the task completed before the push.** It isn't done until it's on
  origin.
- **Removing the worktree before the push.** If the push fails, the work
  survives only in the branch.
- **`task <uuid> done` instead of `niritasks task status <uuid> completed`.**
