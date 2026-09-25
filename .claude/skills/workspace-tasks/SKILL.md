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
niritasks tag --session                                   # this terminal's workspace tag
task "+$(niritasks tag --session)" status:pending export  # the tasks, as JSON
```

**`--session`, not bare `niritasks tag`.** Bare `niritasks tag` answers "which workspace is
focused *right now*", which is the right answer for a keybind and the wrong one
for you: a run takes minutes, the user switches workspace while it goes, and the
tag moves with them — so the findings get filed onto whatever project they
happened to be reading. `--session` takes the workspace from the tmux session
this terminal was opened on, which does not move.

It fails, rather than guessing, in three cases, and each wants a different thing
from you:

| It says | What to do |
|---|---|
| the workspace has no name | Stop. Tell the user to name it with `Mod+Shift+Alt+W` — an unnamed workspace has no tag, so it has no tasks. |
| not inside a tmux session | Fall back to `niritasks tag`, and **say so**: the tag is focus-derived, so ask the user not to switch workspace mid-run. |
| the session matches no named workspace | The workspace was renamed since this terminal opened. Ask which project the list belongs to rather than picking one. |

**Write the tag down and reuse that value for the rest of the run** either way.
That is belt and braces with `--session`, and the only thing keeping the
fallback honest.

Read the JSON, not the table. Four fields matter:

| Field | Why |
|---|---|
| `uuid` | The only stable handle — see the warning below |
| `description` | The one-liner typed into the box |
| `annotations` | Longer detail added later — by the user via the **Note** action, or by a previous run of this skill (Phases 2 and 5). Terse tasks often have their real specification here. Always read them. |
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

**Write the answers back onto the task, before starting any work:**

```bash
task <uuid> annotate -- "the answer, as the specification it now is"
```

The answer to a clarifying question *is* the task — "make it lower" only became
actionable when the user said how low. Left in the chat it dies with the
session, and the task is back to its terse one line for whoever reads it next,
including the next run of this skill. Annotating is also how the detail reaches
the desktop: the picker marks an annotated task with `¶` and the **Note** box
lists the notes above the input, so it is visible from the keybind, not only
from here.

Annotate only the tasks you actually asked about, one annotation each, and
record the decision rather than the exchange — *"80 chars, ellipsised"*, not
*"asked how wide, user said 80"*. Note the `--`; Phase 5 sets out what eats your
text without it.

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
task <uuid> annotate -- "blocked: <one line on why>"
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

For each one they accept, file the line and then put the context on it as an
annotation:

```bash
finding="the finding, as one actionable line"
task add "+<tag>" -- "$finding"
uuid=$(task "+<tag>" status:pending export | python3 -c "
import json,sys
print(next(t['uuid'] for t in json.load(sys.stdin) if t['description'] == sys.argv[1]))
" "$finding")
task "$uuid" annotate -- "src/overlay.rs:410 — what you saw, and why it matters"
```

Using the tag **captured in Phase 1**, not a freshly resolved one — see the
table there.

One line is all the picker shows, and it is not enough to act on months later:
you are holding the file, the line number and the reason right now, and nobody
will have them again without rediscovering the finding from scratch. The
annotation is where they go. Skip it only when the one line genuinely says
everything — an annotation that restates the description is noise.

Read the uuid back out of the export rather than off `task add`'s *"Created task
8."*: that is an id, and ids renumber — the same trap Phase 1 warns about.
Matching on the description keeps it exact when several findings are filed in
the same second.

> [!IMPORTANT]
> **Quote the text as one argument, and put `--` in front of it.** Both halves
> matter, and the `--` is not optional politeness — it is what stops taskwarrior
> reading your text as instructions.
>
> Taskwarrior decides what an argument is from its **first word**. A word that
> looks like `attribute:value` anywhere else in the argument stays literal —
> `"fix the lint due:friday now"` keeps its `due:friday` as text — but the same
> word at the *front* takes the whole argument with it. `"project.rs:52 says
> so"` is read as the `project` attribute with the value `52 says so`, and a
> file:line reference is exactly the shape a good finding starts with.
>
> The two failures do not look alike, and the second is the dangerous one:
>
> * `task add "+tag" "project.rs:52 fix the lint"` **fails outright** — "A task
>   must have a description", because the description became an attribute.
> * `task <uuid> annotate "project.rs:52 says so"` **silently sets `project`,
>   files no annotation, and exits 0.** It even prints "Annotated 1 task."
>   Nothing tells you the note is gone.
>
> `--` ends attribute parsing, so everything after it is text. `"+<tag>"` still
> has to sit *before* the `--` to register as a tag. This is what the `niritasks` tool
> itself does — see `task::annotate` and `task::modify_description` in
> `src/task.rs`, both of which pass `--`.

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
- **What only exists in the chat is lost.** A clarification that made a task
  actionable, and the context behind a finding, belong on the task as
  annotations — the session ends, the task list does not.
- **Never complete a task you did not finish.** A partial fix stays pending with
  an annotation explaining where it got to.
