---
name: refine-task
description: >
  Work one Taskwarrior task up into a plan — a sharper one-line description and a
  consolidated set of notes — then write it back and tag it +planned. Invoked by
  niri-tasks' Refine and Grill me menu rows as `/refine-task <uuid> [grill]`.
disable-model-invocation: true
argument-hint: <uuid> [grill]
---

# refine-task: turn a terse task into a plan

Arguments: `$ARGUMENTS` — a task uuid, then optionally `grill`.

Tasks are typed into a one-line box, so they are terse. Your job is to turn one
into something an agent or the user could pick up cold: a description that
still fits a task card, and notes that carry the goal, the decisions and what
"done" means.

**You refine the task; you never do it.** However concrete the steps you write
into its notes, do not carry out any of them — no file edits, no config
changes, no commands beyond reading. Your one write is the Taskwarrior update in
step 5, and only after the user says so in step 4. The session was started
without file-editing tools and without plan mode for exactly this reason:
approving a plan in plan mode means "implement it", which is not what the user
is approving here.

Your Bash commands run in a sandbox that can read anything but write only to
the task database. A command that fails with `Read-only file system` or
`Operation not permitted` hit that fence on purpose: do not look for a way
around it, and do not ask the user to lift it.

## 1. Read the task

```bash
task rc.json.array=on <uuid> export
```

Stop and say so if it returns `[]` or its `status` is not `pending`. Keep the
`description` and `annotations` you read — the write in step 5 checks against them.
Show the user the description and every note before going further.

## 2. Ground it

Read enough of the project to understand what the task touches: `CONTEXT.md`,
`AGENTS.md`/`CLAUDE.md`, the docs and the code the task names. Facts you can look
up are yours to find; never ask the user for one. When the task turns on
something outside the project — a tool's options, a library's API, how others
solve it — search the web and read the primary sources. Read only — nothing
here changes a file.

## 3. Work it up

- **No `grill` argument (quick):** draft straight away. Ask only questions the
  code cannot answer and a wrong guess would make the plan wrong — at most
  three, in one round, each with your recommended answer. None is fine.
- **`grill`:** invoke the `grilling` skill with the task (description and notes)
  as the plan to grill, and follow it until its frontier is empty.

## 4. Propose

Show the proposal in your reply, then ask with AskUserQuestion — one question,
"Write this to the task?", with two options: **Write it to the task** and
**Change something**. The proposal contains:

1. **New description** — one line, about 50 characters or fewer: the card and
   picker row show one line at that width. It starts with a Conventional
   Commits type, lowercase, a colon and a space — `feat`, `fix`, `docs`,
   `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style` or `revert` —
   and the type counts toward the 50. A bug is `fix:`; there is no `bug:`.
   Keep the type the task already has unless it is wrong. After it,
   imperative and specific: `fix: Keep the daemon to one task box`.
2. **New notes** — each one line, each becoming one annotation, in this order
   where they apply: `Goal: …`, `Context: …`, `Decided: …` (one per decision),
   `Steps: …`, `Done when: …`, `Out of scope: …`. They **replace** the existing
   notes, so fold in everything the old notes said that still holds.
3. **Before** — the current description and notes, so the user can see nothing
   was dropped.

Then, as the last thing in your reply before the question, print **exactly
what will be sent** — the `description` and `notes` step 5 passes to the tool, in a `json`
code block under the heading **Will be written to the task**, one note per
line:

```json
{
  "description": "feat: Show each Claude agent's topic in herdr's sidebar",
  "notes": [
    "Goal: …",
    "Done when: …"
  ]
}
```

and under it, in one line: the task's uuid, that these notes **replace** all
existing ones, and that `+planned` is added with the other tags left as they
are. This block is what the tool gets, not a summary of it: step 5 sends it
character for character. The user is approving this block, so it must be
valid JSON and must match the proposal above it.

On **Change something** (or any other answer), take their feedback, revise,
print the block again, and ask again. Only **Write it to the task** leads to
step 5.

## 5. Write

Write with the `mcp__niri-tasks-refine__write_task_plan` tool — the session's
one way to change the task. It is deferred: select it with ToolSearch
(`select:mcp__niri-tasks-refine__write_task_plan`) if it is not loaded. Call
it once, with:

- `expected`: the `description` and the annotations' `description`s, in
  order, exactly as you read them in step 1;
- `description` and `notes`: the **Will be written to the task** block the
  user approved, exactly as printed — no rewording, no reordering, nothing
  added. If anything needs to change after approval, go back to step 4 and
  print the block again instead.

The tool writes the description, replaces the notes and adds `+planned` in
one import, and touches nothing else. It refuses, writing nothing, if the
task changed since step 1: show the user what it says changed and ask before
going on. If the tool is missing, say so and stop — do not write the task
any other way.

## 6. Verify and report

Export once more and check: the description matches, the notes are exactly the
new list, `planned` is in `tags`, and the other tags are unchanged. Tell the user
what was written, and stop. Leave the session open — if the user then asks you
to start on the task, that is a new request of theirs, not part of this skill.

Always address the task by uuid, never its numeric id: ids are renumbered as
tasks complete.
