# reject-worktree Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `reject-worktree` Claude skill, the opposite of `finish-worktree`: from a task's worktree it throws the attempt away (worktree, local branch, origin branch) and puts the task back on the list, pending and not started.

**Architecture:** A single Markdown skill file at `.claude/skills/reject-worktree/SKILL.md`, shaped like `.claude/skills/finish-worktree/SKILL.md` (same Identify step, same `-C M` cleanup commands). `install.sh` symlinks it into `~/.claude/skills` beside finish-worktree, and README lists it. No Rust changes: the skill drives existing commands (`wt`, `git`, `niritasks task status`, `niritasks task note`).

**Tech Stack:** Claude Code skills (Markdown + YAML frontmatter), bash, worktrunk (`wt`), git, Taskwarrior 3.5 through `niritasks`.

**Spec:** Taskwarrior task `5ea1f862-5b93-49e8-8858-379bfb4ec84b` — read it with `task rc.json.array=on 5ea1f862-5b93-49e8-8858-379bfb4ec84b export`. Its description and annotations are the spec.

## Global Constraints

- Name: `reject-worktree`, at `.claude/skills/reject-worktree/SKILL.md`.
- Triggers: "/reject-worktree", "reject this worktree", "scrap this attempt", "abandon this branch".
- Ask only when there is work to lose: no commits in `main..HEAD`, a clean worktree and no branch on origin → go straight ahead. Otherwise show the commits, the uncommitted changes and whether origin has the branch, then ask once.
- Refuse on the base branch. A branch that isn't `task/…`, or whose task is missing or not pending, still gets its worktree and branch removed, but the task step is skipped and the report says why.
- Remove with `wt remove --foreground -D -f -C M <branch>`, then `git -C M push origin --delete <branch>` if `ls-remote` finds the branch. Everything runs with `-C M`.
- Put the task back with `niritasks task status <uuid8> stopped`, never `task stop`.
- With a reason from the user: `niritasks task note <uuid8> "Rejected: <reason>"`. With no reason, no note.
- Leave the herdr workspace open and tell the user that closing it tidies it away.
- Out of scope: a Reject entry in the action menu or action row; any change to finish-worktree.
- Commits: Conventional Commits, subject ≤ 72 chars, imperative, lowercase first word, ending with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` trailer.
- Do **not** run `install.sh` from this worktree: it would symlink `~/.claude/skills` to files inside a worktree that will be deleted. It is run from the main checkout after landing.

## File Structure

- Create `.claude/skills/reject-worktree/SKILL.md` — the whole skill.
- Modify `install.sh:89-91` — add a `link` for the new skill after finish-worktree's.
- Modify `README.md:159` (list of skills) and `README.md:200-203` (the finish-worktree paragraph) — mention the new skill.

## Background the engineer needs

- `niritasks task start` (`src/work.rs`) has worktrunk make a worktree on a branch `task/<slug>-<uuid8>` and marks the task active. The last 8 characters of the branch name, after the final `-`, are the task's uuid prefix.
- `M` is the main checkout (`/home/paul/Projects/niri-tasks` here). The skill deletes the worktree the shell is standing in, so every command at and after removal must use `git -C M` / `wt -C M`.
- `niritasks task status <uuid8> stopped` on a **completed** task reopens it (`src/task.rs:825`). That is why the skill must skip the task step for any task that is not `pending`.
- `wt remove` flags: `-D` deletes an unmerged branch, `-f` removes a dirty worktree, `--foreground` blocks until done, `-C <path>` sets the working directory. It runs the repo's pre-remove hooks.
- `git ls-remote --exit-code --heads origin <branch>` exits 0 when the branch exists on origin and 2 when it doesn't.

---

### Task 1: Write the reject-worktree skill

**Files:**
- Create: `.claude/skills/reject-worktree/SKILL.md`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces: the file path `.claude/skills/reject-worktree/SKILL.md` and the skill name `reject-worktree` (frontmatter `name:`), which Task 2 links and lists and Task 3 runs.

- [ ] **Step 1: Re-read the model**

Read `.claude/skills/finish-worktree/SKILL.md` in full. The new skill copies its frontmatter shape, its step 1 (Identify) and its step 9 cleanup commands, and its "Keep going / the stops" framing.

- [ ] **Step 2: Write the skill file**

Create `.claude/skills/reject-worktree/SKILL.md` with exactly this content:

````markdown
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
  completed")

Don't close the herdr workspace. This session runs in it, and closing it would
end the session before the user reads the report. Say that closing it tidies it
away.

## The stops (and only these)

| Stop | Left behind | Resumes with |
|---|---|---|
| The branch is the base branch | nothing changed | nothing: there is no attempt to reject |
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
````

- [ ] **Step 3: Check the file**

Run:
```bash
head -12 .claude/skills/reject-worktree/SKILL.md
grep -c 'task stop`\|niritasks task status <uuid8> stopped' .claude/skills/reject-worktree/SKILL.md
grep -n 'wt remove --foreground -D -f -C M <branch>' .claude/skills/reject-worktree/SKILL.md
```
Expected: the frontmatter shows `name: reject-worktree` and the four triggers; the grep count is at least 2; the `wt remove` line is found.

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/reject-worktree/SKILL.md
git commit -m "$(cat <<'EOF'
feat(skills): add the reject-worktree skill

The opposite of finish-worktree: throw a task worktree's attempt away,
worktree and branch, locally and on origin, and put the task back on
the list, stopped, with a Rejected note when a reason is given. It
asks first only when there are commits, changes or a pushed branch to
lose.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Link the skill in install.sh and list it in README

**Files:**
- Modify: `install.sh:89-91`
- Modify: `README.md:159`, `README.md:200-203`

**Interfaces:**
- Consumes: `.claude/skills/reject-worktree/SKILL.md` from Task 1.
- Produces: the symlink target `$CLAUDE_SKILLS/reject-worktree/SKILL.md` that `install.sh` makes when run from the main checkout.

- [ ] **Step 1: Write the failing check**

Run:
```bash
grep -n 'reject-worktree' install.sh README.md
```
Expected: no output (exit 1).

- [ ] **Step 2: Add the link to install.sh**

In `install.sh`, directly after these lines:
```bash
# finish-worktree: the other end of Start working — lands a task's worktree
# branch on main, pushes, and marks the task completed through niritasks.
link "$REPO/.claude/skills/finish-worktree/SKILL.md" "$CLAUDE_SKILLS/finish-worktree/SKILL.md"
```
add a blank line and then:
```bash
# reject-worktree: finish-worktree's opposite — throws a task's worktree and
# branch away, locally and on origin, and puts the task back, stopped.
link "$REPO/.claude/skills/reject-worktree/SKILL.md" "$CLAUDE_SKILLS/reject-worktree/SKILL.md"
```

- [ ] **Step 3: Add the skill to README's list**

In `README.md`, change:
```
the Claude skills (`workspace-tasks`, `refine-task`, `finish-worktree`) into `~/.claude/skills`,
```
to:
```
the Claude skills (`workspace-tasks`, `refine-task`, `finish-worktree`, `reject-worktree`) into `~/.claude/skills`,
```

- [ ] **Step 4: Describe it after the finish-worktree paragraph**

In `README.md`, directly after the paragraph that ends
`It stops to ask only for a conflict with no obvious
resolution, red tests, or `main` holding commits origin doesn't have.`
add a blank line and then:
```
To throw an attempt away instead, `/reject-worktree` in that worktree's Claude
removes the worktree and its branch, locally and on origin, and puts the task
back on the list, stopped, to be started fresh. Give it a reason and the task
gets a `Rejected:` note. It asks first only when there are commits, uncommitted
changes or a pushed branch to lose.
```

- [ ] **Step 5: Run the check again**

Run:
```bash
bash -n install.sh && echo syntax ok
grep -n 'reject-worktree' install.sh README.md
```
Expected: `syntax ok`; two lines in `install.sh` (the comment and the `link`), and two places in `README.md` (the list and the new paragraph).

Do not run `install.sh` itself here (see Global Constraints).

- [ ] **Step 6: Commit**

```bash
git add install.sh README.md
git commit -m "$(cat <<'EOF'
docs: link and list the reject-worktree skill

install.sh links it into ~/.claude/skills beside finish-worktree, and
README lists it and says what it does.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Acceptance run on throwaway worktrees

Prove the spec's "Done when" by running the skill as an agent would, on two throwaway task worktrees of this repo. This step pushes one throwaway branch to origin, which the skill then deletes. Nothing is committed unless the run finds a defect in the skill.

**Files:**
- Modify (only if a defect is found): `.claude/skills/reject-worktree/SKILL.md`

**Interfaces:**
- Consumes: `.claude/skills/reject-worktree/SKILL.md` from Task 1.
- Produces: nothing other tasks use.

Throughout, `M=/home/paul/Projects/niri-tasks` and `SKILL=$PWD/.claude/skills/reject-worktree/SKILL.md` (the absolute path in this worktree, since the skill isn't installed yet).

- [ ] **Step 1: Make fixture A — clean, no commits, never pushed**

Fixture setup uses plain `task` on purpose: `niritasks task status … active` would link this session's herdr pane to the throwaway task. The tag `reject_check` is no workspace's, so the task stays off every panel.
```bash
task add "chore: Throwaway for the reject-worktree check A" +reject_check
A=$(task rc.json.array=on +reject_check description~'check A' status:pending export | jq -r '.[0].uuid[0:8]')
task "$A" start
wt -C /home/paul/Projects/niri-tasks switch --create "task/chore-reject-check-a-$A" --no-cd
WT_A=$(wt -C /home/paul/Projects/niri-tasks list --format=json | jq -r --arg b "task/chore-reject-check-a-$A" '.items[] | select(.branch == $b) | .worktree.path')
echo "$A $WT_A"
```
Expected: an 8-hex uuid prefix and a worktree path. If `.branch` isn't the field name, look at `wt list --format=json | jq '.items[0]'` and use the right one.

- [ ] **Step 2: Run the skill on fixture A with a fresh subagent**

Dispatch a general-purpose subagent with this prompt (fill in the values):

> Read `<SKILL>` and follow it exactly, as if the user had typed `/reject-worktree` with no reason. You are standing in the worktree `<WT_A>`: prefix every command up to and including step 2 with `cd <WT_A> && `; the skill's own `-C M` commands need no prefix. If the skill tells you to ask the user something, don't answer it yourself: stop and return the question. At the end, return the skill's report verbatim.

Expected: the subagent asks no question and returns a report saying the worktree and local branch were removed, origin "never pushed", nothing thrown away, and the task stopped and pending with no note.

- [ ] **Step 3: Verify fixture A**

```bash
git -C /home/paul/Projects/niri-tasks worktree list | grep reject-check-a || echo no worktree
git -C /home/paul/Projects/niri-tasks branch --list "task/chore-reject-check-a-$A"
task rc.json.array=on "$A" export | jq -r '.[0] | .status, (.start // "not started"), ((.annotations // []) | length)'
```
Expected: `no worktree`; empty branch list; `pending`, `not started`, `0`.

- [ ] **Step 4: Make fixture B — a commit, an uncommitted change, pushed to origin**

```bash
task add "chore: Throwaway for the reject-worktree check B" +reject_check
B=$(task rc.json.array=on +reject_check description~'check B' status:pending export | jq -r '.[0].uuid[0:8]')
task "$B" start
wt -C /home/paul/Projects/niri-tasks switch --create "task/chore-reject-check-b-$B" --no-cd
WT_B=$(wt -C /home/paul/Projects/niri-tasks list --format=json | jq -r --arg b "task/chore-reject-check-b-$B" '.items[] | select(.branch == $b) | .worktree.path')
echo scratch > "$WT_B/reject-check.txt"
git -C "$WT_B" add reject-check.txt
git -C "$WT_B" commit -m "chore: throwaway commit for the reject-worktree check"
git -C "$WT_B" push -u origin "task/chore-reject-check-b-$B"
echo dirty > "$WT_B/reject-check-dirty.txt"
echo "$B $WT_B"
```

- [ ] **Step 5: Run the skill on fixture B with a reason**

Dispatch a fresh general-purpose subagent with the prompt from Step 2, but with `<WT_B>` and "as if the user had typed `/reject-worktree wrong approach`".

Expected: it stops at step 2 and returns a question that shows the one commit, the untracked `reject-check-dirty.txt`, and that origin has the branch. Nothing has been removed yet: check with `git -C /home/paul/Projects/niri-tasks worktree list | grep reject-check-b`.

Then answer it with SendMessage to that subagent: "Yes, throw it away." Expected: the final report lists the commit and its tip SHA, the dirty file, the origin branch deleted, and the task stopped with the note `Rejected: wrong approach`.

- [ ] **Step 6: Verify fixture B**

```bash
git -C /home/paul/Projects/niri-tasks worktree list | grep reject-check-b || echo no worktree
git -C /home/paul/Projects/niri-tasks branch --list "task/chore-reject-check-b-$B"
git -C /home/paul/Projects/niri-tasks ls-remote --heads origin "task/chore-reject-check-b-$B"
task rc.json.array=on "$B" export | jq -r '.[0] | .status, (.start // "not started"), [.annotations[]?.description][]'
```
Expected: `no worktree`; empty branch list; empty `ls-remote`; `pending`, `not started`, `Rejected: wrong approach`.

- [ ] **Step 7: Remove the fixture tasks**

```bash
niritasks task status "$A" deleted --yes
niritasks task status "$B" deleted --yes
task +reject_check status:pending count
```
Expected: `0`.

- [ ] **Step 8: Fix and commit only if a step above failed because of the skill**

If a subagent asked when it shouldn't have, left something behind, used `task stop`, or the report missed an item, fix the wording in `.claude/skills/reject-worktree/SKILL.md`, rerun the failing fixture from its step, and commit:
```bash
git add .claude/skills/reject-worktree/SKILL.md
git commit -m "$(cat <<'EOF'
fix(skills): <what the acceptance run showed reject-worktree got wrong>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
If every check passed, there is nothing to commit.

---

## After landing

Once finish-worktree has landed the branch, run `bash install.sh` **from the main checkout** so `~/.claude/skills/reject-worktree/SKILL.md` links to `M`'s copy. Then `/reject-worktree` is available in every session.
