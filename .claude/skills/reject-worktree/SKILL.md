---
name: reject-worktree
description: >
  Use when the user wants to throw away the attempt in a git worktree branch,
  including a task worktree made by `niritasks task start`: remove the worktree
  and its branch, locally and on origin, and put the task back on the list,
  stopped, to be started fresh. The opposite of finish-worktree. Triggers on:
  "/reject-worktree", "reject this worktree", "scrap this attempt",
  "abandon this branch".
argument-hint: "[reason]"
---

# reject-worktree: throw a worktree's attempt away

The opposite of finish-worktree. The user runs one agent per task, each in its
own worktree made by worktrunk (`wt`). When an attempt has gone wrong, this
deletes it — worktree, local branch and origin branch — and puts the task back
on the list, pending and not started, so it can be started fresh.

Arguments: `$ARGUMENTS` is the reason, if the user gave one. A reason given in
the conversation counts too. Don't ask for one.

**Keep going.** Ask once, and only when there is work to lose. The stops are
listed at the end, and there are no others.

## Steps

Run these from the branch's worktree. `M` is the main checkout.

**1. Identify.**
```bash
wt list --format=json | jq -r '.repo.default_branch, (.items[] | select(.worktree.main) | .worktree.path)'
git branch --show-current
```
- The first line is the base branch (`main` below).
- The second line is `M`.
- **The branch must not be the base branch.** If it is, stop: rejecting would
  delete `main`. Nothing has changed.
- If `git branch --show-current` prints nothing, HEAD is detached (a rebase
  left open, say). Stop and say so: there is no branch to remove by, and
  `wt remove` with an empty branch would aim at `M`. Nothing has changed.
- A branch named `task/<slug>-<uuid8>` belongs to a Taskwarrior task. Look it up
  with `task rc.json.array=on <uuid8> export`. If it's missing, or its status
  isn't `pending`, the task step (4) is skipped and the report says why. Never
  run step 4 on a completed task: `stopped` would reopen it.
- Any other branch name has no task, so don't look one up. Step 4 is skipped,
  and the report says so.

**2. Check what would be lost.** Nothing has changed yet.
```bash
git log --oneline main..HEAD                                # commits only this branch has
git status --porcelain                                      # uncommitted changes
git -C M ls-remote --exit-code --heads origin <branch>      # exit 0: on origin; exit 2: never pushed
git rev-parse HEAD                                          # the tip, for the report
```
- **Nothing to lose** — no commits, empty `status`, and `ls-remote` exited 2:
  go straight on to step 3. Don't ask.
- **Otherwise** show the user all three — the commits, the uncommitted changes,
  and whether origin has the branch — and ask once whether to throw them away.
  On a no, stop. Nothing has changed.

**3. Remove the worktree and the branch.** This deletes the worktree the shell is
standing in, so from here on run everything with `-C M`.
```bash
wt remove --foreground -D -f -C M <branch>   # runs the repo's pre-remove hooks
git -C M ls-remote --exit-code --heads origin <branch> && git -C M push origin --delete <branch>
```
- `-D` deletes the branch although it isn't merged, and `-f` removes the
  worktree although it's dirty. Step 2 is what made those safe.
- Never pass `--yes`. If `wt remove` stops for a hook approval, stop and ask the
  user to run that command themselves, with `!` in front.
- If `wt remove` fails, stop and report. Leave the task as it is: the attempt
  still exists.
- When `ls-remote` exits 2, there's no origin branch to delete. That's normal.
  A failed `push --delete` is reported, not retried; carry on to step 4.

Check it's gone:
```bash
git -C M worktree list                                # no line for the worktree
git -C M branch --list <branch>                       # empty
git -C M ls-remote --heads origin <branch>            # empty
```

**4. Put the task back.** Skip this when step 1 said to.
```bash
niritasks task status <uuid8> stopped
niritasks task note <uuid8> "Rejected: <reason>"      # only with a reason
task rc.json.array=on <uuid8> export | jq -r '.[0] | .status, (.start // "not started")'
```
The export must print `pending` and `not started`. Always go through
`niritasks`, never `task stop`: niritasks is the one path a task card's Stop
also uses, and it sends the same notification. With no reason, add no note.

**5. Report**, in this order:
- what was removed: the worktree path, the local branch, and the origin branch
  (or "never pushed")
- what was thrown away: the commits and uncommitted changes from step 2, or
  "nothing". With commits, give the old tip from `git rev-parse HEAD`: until
  git collects garbage, `git -C M branch <branch> <sha>` brings them back.
- the task: stopped and pending, with the note if one was added — or why the
  step was skipped ("no task: branch isn't `task/…`", "task missing", "task is
  <status>")

Don't close the herdr workspace. This session runs in it, and closing it would
end the session before the user reads the report. Say that closing it tidies it
away.

## The stops (and only these)

| Stop | Left behind | Resumes with |
|---|---|---|
| The branch is the base branch | nothing changed | nothing: there is no attempt to reject |
| HEAD is detached | nothing changed | finish or abort the rebase, then rerun |
| The user says no at step 2 | nothing changed | nothing, or rerun once the work is saved elsewhere |
| `wt` wants a hook approval | nothing removed | the user runs the command with `!` |
| `wt remove` fails | whatever `wt` left; the task untouched | fix what it reports, then rerun |

## Mistakes this prevents

- **Asking when there's nothing to lose.** A clean worktree with no commits and
  no origin branch goes without a question.
- **Running a command without `-C M` after step 3.** The worktree, and with it
  the shell's directory, is gone.
- **`task <uuid> stop` instead of `niritasks task status <uuid> stopped`.**
- **Stopping a completed task.** `stopped` reopens it. Only a `pending` task is
  put back.
- **Leaving the branch on origin.** The next Start working makes the same
  branch name, and the old one would be in its way.
