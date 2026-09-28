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
`description` and `annotations` you read — step 5 compares against them.
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
   picker row show one line at that width. Imperative, specific.
2. **New notes** — each one line, each becoming one annotation, in this order
   where they apply: `Goal: …`, `Context: …`, `Decided: …` (one per decision),
   `Steps: …`, `Done when: …`, `Out of scope: …`. They **replace** the existing
   notes, so fold in everything the old notes said that still holds.
3. **Before** — the current description and notes, so the user can see nothing
   was dropped.

Then, as the last thing in your reply before the question, print **exactly
what will be sent** — the JSON payload step 5 passes to the write, in a `json`
code block under the heading **Will be written to the task**, one note per
line:

```json
{
  "description": "Show each Claude agent's topic in herdr's sidebar",
  "notes": [
    "Goal: …",
    "Done when: …"
  ]
}
```

and under it, in one line: the task's uuid, that these notes **replace** all
existing ones, and that `+planned` is added with the other tags left as they
are. This block is the payload itself, not a summary of it: step 5 sends it
character for character. The user is approving this block, so it must be
valid JSON and must match the proposal above it.

On **Change something** (or any other answer), take their feedback, revise,
print the block again, and ask again. Only **Write it to the task** leads to
step 5.

## 5. Write

Re-read the task (`task rc.json.array=on <uuid> export`). If its description or
notes differ from what you read in step 1, show the difference and ask before
writing — someone else changed it meanwhile.

Otherwise write the description, the notes and the tag in **one** import, so the
task is never half-updated. Put the **Will be written to the task** block the
user approved into the heredoc exactly as printed — no rewording, no
reordering, nothing added. If anything needs to change after approval, go back
to step 4 and print the block again instead. Then run:

```bash
task rc.json.array=on <uuid> export | python3 -c '
import datetime, json, sys
task = json.load(sys.stdin)[0]
new = json.loads(sys.argv[1])
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
task["description"] = new["description"]
task["annotations"] = [{"entry": now, "description": n} for n in new["notes"]]
task["tags"] = sorted(set(task.get("tags", [])) | {"planned"})
print(json.dumps(task))
' "$(cat <<'EOF'
{"description": "…", "notes": ["Goal: …", "Done when: …"]}
EOF
)" | task rc.verbose=nothing import
```

Taskwarrior bumps identical note timestamps a second apart itself. Do not touch
the status, start, other tags or any other field.

## 6. Verify and report

Export once more and check: the description matches, the notes are exactly the
new list, `planned` is in `tags`, and the other tags are unchanged. Tell the user
what was written, and stop. Leave the session open — if the user then asks you
to start on the task, that is a new request of theirs, not part of this skill.

Always address the task by uuid, never its numeric id: ids are renumbered as
tasks complete.
