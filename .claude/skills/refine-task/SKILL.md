---
name: refine-task
description: >
  Work one Taskwarrior task up into a plan — a sharper one-line description and a
  consolidated set of notes — then write it back and tag it +planned. Invoked by
  niri-tasks' task card's Refine and Grill me buttons as `/refine-task <uuid> [grill]`,
  and by `niritasks task refine <uuid> --unattended` as `/refine-task <uuid> auto`.
disable-model-invocation: true
argument-hint: <uuid> [grill|auto]
---

# refine-task: turn a terse task into a plan

Arguments: `$ARGUMENTS` — a task uuid, then optionally `grill` or `auto`.

Tasks are typed into a one-line box, so they are terse. Your job is to turn one
into something an agent or the user could pick up cold: a description that
still fits a task card, and notes that carry the goal, the decisions and what
"done" means.

**You refine the task; you never do it.** However concrete the steps you write
into its notes, do not carry out any of them — no file edits, no config changes,
no commands beyond reading. Your one write is the Taskwarrior update in step 5,
and the write tool asks the user itself before it writes (in `auto` mode it
writes at once: the command that started you was the approval). A report the
user asks for there is written by the report tool, not by you. The session was
started without file-editing tools and without plan mode for exactly this
reason: approving a plan in plan mode means "implement it", which is not what
the user is approving here.

Your Bash commands run in a sandbox that can read anything but write only to
the task database. A command that fails with `Read-only file system` or
`Operation not permitted` hit that fence on purpose: do not look for a way
around it, and do not ask the user to lift it.

## 1. Read the task

```bash
task rc.json.array=on <uuid> export
```

Stop and say so if it returns `[]` or its `status` is not `pending`. Keep the
`description` and `annotations` you read — the write in step 5 checks against
them. Show the user the description and every note before going further.

Then select the mod's tools with ToolSearch
(`select:mcp__niri-tasks-refine__write_task_plan,mcp__niri-tasks-refine__show_task_report`).
If the write tool is not there, stop
before step 2 and tell the user the refine mod did not load, so the task
cannot be written: they should run `install.sh` and check that `claude
--version` is 2.1.287 or later. Do not interview them first.

The report tool may be missing on its own; then the user is not offered a
report, and nothing else changes.

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
  as the plan to grill, and follow it until its frontier is empty — except for
  how a round is put to the user. The "Format a round like so" block in
  `grilling` (❓ **Q1** … ➡️, the numbered markdown list) does not apply in a
  refine session; this bullet replaces it, even though `grilling` loads after
  it. Ask each round with **one AskUserQuestion call**, one question per tab,
  so the user takes them one at a time and sends all the answers together:
  - At most 4 questions a round. If the frontier holds more, ask the 4 that
    most other decisions hang on, and leave the rest for the next round.
  - Each question's text carries what the markdown body would have: the
    decision and enough context to make it. Its `header` names the decision
    in 12 characters or fewer.
  - 2–4 options per question. Your recommended answer is the **first** option,
    its label ending in "(Recommended)", and its description says why. An open
    question still gets 2–4 likely answers; the user can always pick "Other"
    and type their own. Use an option's `preview` when comparing code,
    layouts or config. If a question's options are not exclusive, set
    `multiSelect`.
  - Don't repeat the round as text in your reply; the dialog is the round. If
    the dialog is dismissed or errors, re-ask that round as plain text, still
    with the recommended answer first.

  The rest of `grilling` holds, including rounds, recomputing the frontier
  after each one, looking up facts rather than asking, and stopping when the
  frontier is empty. A question that hangs on another still open waits for a
  later round, so never put both in one dialog. Grilling's final confirmation
  is the write tool's question in step 5; don't ask for it in a separate
  dialog.
- **`auto`:** an unattended run — `niritasks task refine <uuid> --unattended`,
  or the task box's Add & all — in a `claude -p` process with no one at the
  keyboard. Ask nothing: AskUserQuestion is not available, and a question
  would end the run with the task unplanned. Where quick mode would have
  asked, take the answer you would have recommended and record it as its own
  `Decided: …` note, saying in it that it was decided for the user, so they
  can see what was chosen and change it. Decide at most three such questions;
  past that, the task is too open for an unattended plan: write the plan with
  what you have and say so in a `Decided:` note. Never offer or build a
  report: the run builds one itself afterwards, with `report-task`. Grilling
  is never unattended.

## 4. Propose

Show the proposal in your reply. Do not ask the user to approve it yourself —
no AskUserQuestion for approval: the write tool in step 5 shows them exactly
what it will write and asks them. The proposal contains:

1. **New description** — one line, about 50 characters or fewer: the task card
   shows one line at that width. It starts with a Conventional
   Commits type, lowercase, a colon and a space — `feat`, `fix`, `docs`,
   `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style` or `revert` —
   and the type counts toward the 50. A bug is `fix:`; there is no `bug:`.
   Keep the type the task already has unless it is wrong. After it,
   imperative and specific: `fix: Keep the daemon to one task box`.
2. **New notes** — each one line, each becoming one annotation, in this order
   where they apply: `Goal: …`, `Context: …`, `Decided: …` (one per decision),
   `Steps: …`, `Done when: …`, `Out of scope: …`. They **replace** the existing
   notes, so fold in everything the old notes said that still holds. Keep any
   `Report: …` or `Report (earlier draft): …` notes as they are: they link
   earlier reports.
3. **Before** — the current description and notes, so the user can see nothing
   was dropped.

Then, as the last thing in your reply, print **exactly
what will be written** — the `description` and `notes` step 5 passes to the
tool, in a `json` code block under the heading **Will be written to the
task**, one note per line:

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
are. This block is the description and notes themselves, not a summary of
them: step 5 passes them to the tool unchanged. The user is approving this
block, so it must match the proposal above it.

If the user asks for a report of the plan in chat, go on to step 5 all the
same: the tool's question offers **Show me a report first**.

In `auto` mode, show the proposal and print the block all the same: the
transcript is the run's log, and the block is what the user reads later to
see what was written and why. Nothing waits on them.

Then go straight to step 5.

## 5. Write

Write with the `mcp__niri-tasks-refine__write_task_plan` tool — the session's
one way to change the task. It is deferred: select it with ToolSearch
(`select:mcp__niri-tasks-refine__write_task_plan`) if it is not loaded. Call
it with:

- `expected`: the `description` and the annotations' `description`s, in
  order, exactly as you read them in step 1;
- `description` and `notes`: the **Will be written to the task** block you
  just printed, exactly as printed — no rewording, no reordering, nothing
  added. Each note is one line of plain text.

The tool shows the user the description and notes it was given, asks
"Write this to the task?", and writes only if they choose **Write it to the
task**. It writes the description, replaces the notes (adding a note that
links each report made this session) and adds `+planned` in one import, and
touches nothing else.

In `auto` mode the tool asks no one: it logs the plan, reads the task again,
writes and answers **Wrote the plan …**. Call it once; if it names a defect in
the plan itself (a line too long, a hidden character, a note on two lines), fix
that and call once more. That is the only retry: on any other **Nothing
written** or failure, stop and say what it answered — there is no one to ask,
and the run reports the task as unplanned.

Its answer says what happened:

- **Wrote the plan …** — go to step 6.
- **Nothing written: the person chose "Change something"** — ask them what to
  change, revise, print the block again (step 4) and call the tool again.
- **Nothing written: the person chose "Show me a report first"** — build the
  report as **Report, when asked** below says, then call the tool again with
  the same plan.
- **Nothing written: the person answered "…"** — those are their words about
  the plan: revise with them, print the block again and call again.
- **Nothing written: the task changed since it was read** — someone else
  edited it meanwhile: show the user what changed and ask before going on.
- **Nothing written: the person could not be asked** — stop, and tell the
  user the plan was not written because the tool could not ask them. If they
  dismissed the question, they can tell you what to change, and you call
  again after revising.
- **Nothing written: the task no longer exists** or **… not pending** — stop
  and tell the user.
- Any other **Nothing written: …** or **write_task_plan: …** answer, or a
  **show_task_report: …** answer other than "has not asked for a report"
  (see **Report, when asked**) — fix what it names (a line too long, a note
  on two lines, a control or invisible character) and call again.
- Anything else (not armed, `task export failed`, `task import failed`,
  `failed before writing`) — do not retry and do not write the task any other
  way. Export the task as in step 6, then tell the user the tool's answer and
  what the task now holds.
- The report tool answered not armed or failed — tell the user the report
  could not be made, and call `write_task_plan` again so they can approve or
  change the plan without it.

If the write tool is missing, say so and stop — do not write the task any other
way.

### Report, when asked

The user wants to see the plan explained before they approve it. Write
nothing to the task. Build the report:

1. Read `report-catalogue.md` in this skill's base directory: who the report
   is for, its fields and their limits, the lenses to look through, how to
   draw diagrams, and a worked example.
2. Fill the fields from the plan you just proposed and the code you read in
   step 2. Go through every lens the catalogue lists and give a part for
   each one that applies: before/after, structure, data flow, outside tools
   and how hard each is to swap, styling, code before/after, and whether a
   newcomer could follow it. Read the code each lens needs: count the files
   that name a tool before you rate its swap. Real paths go in `files`;
   everywhere else, plain words the user would use.
3. Call `mcp__niri-tasks-refine__show_task_report` with the fields. The tool
   builds the page in its fixed layout, saves it under the reviews folder and
   opens it in the browser.
4. On **show_task_report: the person has not asked for a report**, call
   `write_task_plan` again instead; they can choose the report there. On any
   other **show_task_report: …**, the answer lists every field over its
   limit or not allowed: fix them all and call it again. On **Wrote the
   report to …**, tell the user the path in one line, then call
   `write_task_plan` again with the same `expected`, `description` and
   `notes`. The plan has not changed, so do not print the block again.

When the task is written, the tool adds a note linking each report made in
this session: `Report: <path>`, or `Report (earlier draft): <path>` for one
made before the plan changed. Do not add these notes yourself. The tool
relabels any `Report …` note for this session's reports, so keeping earlier
ones is always safe.

If, after reading the report, the user answers with changes, revise, print
the block again (step 4) and call `write_task_plan` again.

## 6. Verify and report

Export once more and check: the description matches, the notes are exactly the
new list, followed by any `Report …` notes the tool added, `planned` is in
`tags`, and the other tags are unchanged. Tell the user what was written, and
stop. Leave the session open — if the user then asks you to start on the task,
that is a new request of theirs, not part of this skill.

Always address the task by uuid, never its numeric id: ids are renumbered as
tasks complete.
