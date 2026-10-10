---
name: report-task
description: >
  Build the HTML report of a planned Taskwarrior task's plan — the description
  and notes a refine wrote — open it in the browser and link it from the task,
  without refining again. Invoked by niri-tasks' task card's Report button as
  `/report-task <uuid>`.
disable-model-invocation: true
argument-hint: <uuid>
---

# report-task: explain a planned task's plan as a report

Arguments: `$ARGUMENTS` — a task uuid.

The task already has a plan: a refine wrote its description and notes and
tagged it `+planned`. Your job is to explain that plan as the refine report
would have — what changes, what needs the reader's judgement, diagrams, the
files and the checks — so the person can read it before deciding what to do
with the task. You do not refine, and you do not do the task.

**You report on the plan; you never change it and never carry it out.** No
file edits, no config changes, no commands beyond reading. The task's one
change is the `Report: <path>` note the report tool adds itself when it
writes the report. There is no `write_task_plan` in this session, and you
must not change the task any other way: no `task modify`, no `task
annotate`, no `task import`.

Your Bash commands run in a sandbox that can read anything but write only to
the task database. A command that fails with `Read-only file system` or
`Operation not permitted` hit that fence on purpose: do not look for a way
around it, and do not ask the user to lift it.

## 1. Read the task

```bash
task rc.json.array=on <uuid> export
```

Stop and say so if it returns `[]`, if its `status` is not `pending`, or if
`planned` is not among its `tags`: an unplanned task has no plan to report
on, and its card's Refine or Grill me is the way to get one. Show the user
the description and every note before going further.

Then select the report tool with ToolSearch
(`select:mcp__niri-tasks-refine__show_task_report`). If it is not there,
stop and tell the user the refine mod did not load, so the report cannot be
made: they should run `install.sh` and check that `claude --version` is
2.1.287 or later.

## 2. Ground it

Read enough of the project to understand what the plan touches: `CONTEXT.md`,
`AGENTS.md`/`CLAUDE.md`, the docs and the code the notes name. Facts you can
look up are yours to find; never ask the user for one. When the plan turns on
something outside the project — a tool's options, a library's API — search
the web and read the primary sources. Read only — nothing here changes a
file. Do not interview the user: the plan is decided, and the report
explains it.

## 3. Fill the report

Read `report-catalogue.md` in the `refine-task` skill's directory, beside
this skill's (`~/.claude/skills/refine-task/report-catalogue.md` when
installed): who the report is for, its fields and their limits, the lenses to
look through, how to draw diagrams, and a worked example. Then fill the
fields from the task's plan and the code you read in step 2:

- `title` is the task's description, as read in step 1.
- Go through every lens the catalogue lists and give a part for each one
  that applies: before/after, structure, data flow, outside tools and how
  hard each is to swap, styling, code before/after, and whether a newcomer
  could follow it. Read the code each lens needs: count the files that name
  a tool before you rate its swap.
- Real paths go in `files`; everywhere else, plain words the user would use.
- The tool shows the plan itself — the task's description and notes, read
  when you call — on the report's last slide. Do not repeat them in the
  fields.

## 4. Show it

Call `mcp__niri-tasks-refine__show_task_report` with the fields. The tool
builds the page in its fixed layout, saves it under the reviews folder,
opens it in the browser, and adds a `Report: <path>` note to the task. It
asks the user nothing: pressing Report was the ask. Its answer says what
happened:

- **Wrote the report to … and linked it from the task** — go to step 5.
- **show_task_report: nothing written: the task has no plan**, **… is
  completed, not pending** or **… no longer exists** — stop and tell the
  user; nothing was written.
- **show_task_report: this session's report is made and linked** — the
  report is done already; do not make another. Go to step 5.
- Any other **show_task_report: …** answer lists every field over its limit
  or not allowed: fix them all and call again.
- **Wrote the report to … but task import failed** — the page is open but
  the task does not link it: tell the user the path and the error, and stop.
- **not armed**, **task export failed**, or **show_task_report failed** —
  do not retry and do not write anything any other way: tell the user the
  tool's answer and stop.

## 5. Verify and report

Export the task once more and check: the description and the earlier notes
are as you read them in step 1, the last note is `Report: <path>` with the
path the tool answered, and the tags are unchanged, `planned` still among
them. Tell the user the path in one line, and stop. Leave the session open.
A second Report press switches back to this tab; to make another report they close this tab first.

Always address the task by uuid, never its numeric id: ids are renumbered as
tasks complete.
