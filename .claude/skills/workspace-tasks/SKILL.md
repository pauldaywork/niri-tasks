---
name: workspace-tasks
description: >
  Work through every pending Taskwarrior task tagged for the current niri workspace,
  one at a time: clarify anything ambiguous first, mark each task active while it is
  being worked on, mark it done when it is finished, and finally offer any bugs or
  issues found along the way as new tasks on the same tag. Triggers on: "/workspace-tasks",
  "do the tasks for this workspace", "work through my tasks", "what tasks are on this
  workspace", "do all my tasks", "clear the task list", "work the workspace tasks".
---

# workspace-tasks: work the current workspace's task list

The niri workspace **is** the task filter. Every task carries the workspace's
name as a tag, so "my tasks" always means "the tasks for the project I am
looking at". This skill reads that list, works it top to bottom, and keeps the
Taskwarrior state honest while it does — because the active task is shown on
screen by the `niri-tasks` overlay, a wrong `start`/`stop` is visible to the
user, not just wrong in a database.

Tasks are raised from the desktop in a one-line box, so they are often terse.
**Ambiguity is the normal case, not the exception.** Clarifying is the first
phase of this skill, not an interruption to it.

---

## Phase 1 — read the list

```bash
wt tag                                   # the focused workspace's tag
task "+$(wt tag)" status:pending export  # the tasks, as JSON
```

`wt tag` exits non-zero when the workspace has no name. Stop there and tell the
user to name it with `Mod+Shift+Alt+W` — an unnamed workspace has no tag, so it
has no tasks, and guessing a tag would work on somebody else's list.

**Write the tag down and reuse that value for the rest of the run.** Do not
re-run `wt tag` later. It reports whatever workspace is focused *now*, and the
user may well have switched away while you worked — re-resolving it at the end
would file the follow-up tasks onto the wrong project.

Read the JSON, not the table. Four fields matter:

| Field | Why |
|---|---|
| `uuid` | The only stable handle — see the warning below |
| `description` | The one-liner typed into the box |
| `annotations` | Longer detail added later via the **Note** action. Terse tasks often have their real specification here. Always read them. |
| `start` | Present means the task is already active |

`annotations` and `start` are **absent** from the JSON rather than empty when a
task has neither, so read them with `t.get(...)`, not `t[...]`.

> [!WARNING]
> **Address tasks by `uuid`, never by the numeric `id`.** Taskwarrior renumbers
> ids as tasks leave the pending list, so completing the first task renumbers
> every task after it. A loop that captured `id` up front will mark the wrong
> tasks done — silently, and with no way to tell afterwards which was which.
> Every command below takes a uuid, and every one of them accepts it.

If a task is already `start`ed, say so and ask whether to pick it up or leave
it — something else may be mid-flight on it.

## Phase 2 — clarify before touching anything

Go through the whole list first and decide, per task, whether you could hand the
finished work over and have the user agree it is what they asked for. Batch the
questions for every unclear task into one round rather than stopping the user
three separate times.

Ask when the task:

- names an outcome but not a target — *"make it lower"*, *"a bit too big"*,
  *"match the other one"*. Numbers and references are what make these
  actionable. Where you can, produce the options and let the user pick a
  concrete one rather than asking them to imagine it.
- could plausibly mean two different files, repos, or components
- implies a visual or aesthetic judgement you cannot verify yourself
- looks like it might already be done, or contradicts a task above it

Do **not** ask about things you can find out yourself. Which file holds the
code, whether a binary is installed, what the current value is — read the repo.
Reserve the questions for what only the user knows.

Restate the list back to the user before starting, so a
task you have misread gets caught before any work goes into it.

## Phase 3 — work them, one at a time

For each task, in the order the user confirmed:

**1. Mark it active.** Exactly one task per workspace may be active — that is
the rule the desktop UI enforces, and the overlay shows only one:

```bash
task "+<tag>" status:pending +ACTIVE export \
  | python3 -c "import json,sys; [print(t['uuid']) for t in json.load(sys.stdin)]" \
  | xargs -r -n1 -I{} task {} stop      # clear any other active task first
task <uuid> start
```

`<tag>` is the value captured in Phase 1, here and everywhere below.

**2. Do the work.** Normal engineering: read the surrounding code, match its
idiom, run the tests. Note that a task raised on a project workspace frequently
lands in a *different* repo from the one the workspace is named after — the
workspace names the project, not the checkout. Find where the code actually
lives before assuming.

**3. Verify it.** A task is not finished because the edit was applied. Run the
suite. For anything on screen, look at it — `niri msg action screenshot-screen`
writes to `~/Pictures/Screenshots`, and cropping the region of interest and
reading the image back is usually faster than describing it. Delete screenshots
you created when you are done with them; they are your scratch, not the user's.

**4. Mark it done.**

```bash
task <uuid> done
```

Mark it done as soon as the work is complete and verified — that is the point of
the skill, and a finished task left pending is as wrong as an unfinished one
marked done. The one exception is a task whose success is a matter of the user's
taste rather than a test passing: show them the result and let them confirm
before closing it.

**If a task turns out to be blocked**, stop it rather than leaving it active:

```bash
task <uuid> stop
task <uuid> annotate "blocked: <one line on why>"
```

A stale active task sits on the user's screen claiming work is in progress that
is not. Never end a session with one.

**Keep a findings list as you go.** Working through real code turns up things
that are not the task: a bug next to the one you fixed, a broken edge case, a
stale comment, a test that passes for the wrong reason. Write each one down when
you see it — what it is, where, and why it matters. Do **not** chase them, and
do not stop to ask about them mid-task; that is scope creep and it derails the
task you are actually on. They get raised in Phase 5, once the list is worked.

The exception is a finding that blocks the current task or makes the work you
just did wrong. That is not a follow-up, it is part of the task in front of you.

## Phase 4 — report

Say plainly, per task: done, blocked, or skipped, and what changed. Name the
files and the repo. If you left anything uncommitted, say so — this skill does
not commit unless the user asks.

## Phase 5 — offer the findings as new tasks

If the findings list is not empty, end by asking the user whether to file them.
Show the list first — one line each, specific enough to act on months later —
then ask. Never file them silently: the task list is the user's, and a run that
quietly grows it by five items is worse than one that mentions them.

Ask once, for the whole set, and let them pick which ones they want rather than
forcing all-or-nothing. `AskUserQuestion` with `multiSelect: true` fits this.

For each one they accept:

```bash
task add "+<tag>" "the finding, as one actionable line"
```

Using the tag **captured in Phase 1**, not a fresh `wt tag` — see the warning
there.

> [!IMPORTANT]
> Pass the description as **one quoted argument**, exactly as above.
> Taskwarrior classifies each argument whole: an argument that is *entirely*
> `+tag` or `due:friday` becomes metadata, but a multi-word argument is
> description text and is not scanned inside. So the quoting does two jobs at
> once — `"+<tag>"` still tags the task, while a finding that happens to mention
> `due:` or `priority:` keeps those words as text instead of having them eaten.
> Word-split the description and that stops being true. (This is the same
> distinction the `wt` tool draws between `wt task add`, which splits, and
> `wt task edit`, which does not.)

Confirm what was filed, then stop. Do not start working the tasks you just
created — they are the next run's list, not this one's.

---

## Invariants

- **uuid, never id.** Ids renumber the moment a task completes.
- **One active task at a time.** Stop the previous before starting the next.
- **Never end with a stale active task.** Done, or stopped, or explicitly
  handed back mid-flight — never silently left running.
- **Never invent tasks.** Work the list as it stands. Something worth doing that
  is not on the list goes on the findings list and is offered in Phase 5 — it is
  never work you just do, and never a task you file without being asked.
- **Findings are raised, not chased.** Note them the moment you see them; do not
  fix them. The only exception is a finding that blocks the task in front of you.
- **Never complete a task you did not finish.** A partial fix stays pending with
  an annotation explaining where it got to.
