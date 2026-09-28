# Refine a task with Claude — design

Date: 2026-09-28

## Goal

From a task's action menu, hand the task to a Claude Code session in the
workspace's herdr session, work it up into a proper plan — either a quick draft
you approve or refine, or a full `/grill-me` interview — and write the result
back to the task: a sharper description, a consolidated set of notes, and a
`+planned` tag whose icon marks it on the task panel and in the picker.

## Decisions

| Question | Decision |
|---|---|
| Menu shape | Two rows, **Refine** (quick) and **Grill me** (full interview) |
| Write-back | Description replaced with a card-length one-liner; **all existing notes replaced** by one consolidated set; `+planned` added |
| herdr session not running | Open the project terminal as the project picker does, wait for the session, then add the tab |
| After the write | Leave the tab open; Claude confirms what it wrote |
| Planned marker | Tag `+planned` (not `ready`, which sits beside Taskwarrior's virtual `+READY`) |
| Planned icon | `\u{f014e}` — Material Design `clipboard-check`, solid and single-colour like the blocked lock `\u{f023}` |
| Sorting | Unchanged; the icon is the only signal |

## 1. Menu

`task_menu` (`src/main.rs`) grows two rows after **Note**, and `.lines(4)`
becomes `.lines(6)`:

```
Edit
Note
Refine
Grill me
Update status
Move to workspace
```

Each calls `task_refine(uuid, Mode::Quick | Mode::Grill)`.

## 2. Launcher: `niritasks task refine <uuid> [--grill]`

A new `TaskCommand::Refine { uuid, grill }`, so it is scriptable and testable
like `task edit` and `task note`. It runs from a fuzzel pick, with no terminal,
so every failure is reported through `notify::tasks` rather than stderr.

Steps:

1. **Resolve.** Workspace name → session `herdr_session_name(ws)`, folder
   `start_dir(home, ws)`. The task must exist and be pending.
2. **Ensure the session.** `herdr --session S workspace list` succeeding means
   it is running. Otherwise spawn the project terminal
   (`project::project_terminal_command`) and poll that command until it
   answers, up to ~10 s; on timeout, notify and stop.
3. **Reuse or create.** The agent name is `task-<first 8 of uuid>` (fits
   herdr's `[a-z][a-z0-9_-]{0,31}`). If `herdr --session S agent get <name>`
   finds it, `agent focus` it and stop — picking Refine twice on one task goes
   back to the running session rather than opening a second.
4. **Tab.** `herdr --session S tab create --workspace <id from workspace list>
   --cwd <folder> --label "<Refine|Grill>: <description, elided>" --focus`;
   read the root pane from `.result.root_pane`.
5. **Agent.** `herdr --session S agent start <name> --kind claude --pane <pane>
   -- --permission-mode default --disallowedTools Edit Write NotebookEdit
   EnterPlanMode ExitPlanMode`. See *Revision: no plan mode* below.
6. **Prompt.** `herdr --session S agent prompt <name> "/refine-task <uuid>"`,
   with ` grill` appended in grill mode. Only the uuid is passed: the skill
   reads the task itself, so nothing needs quoting and it always sees the
   current version.

Every herdr argv is built by a pure function (as `project_terminal_command`
is) so it is unit-testable; the runner only executes them and parses JSON.

Best effort: focus the niri window showing session S. If no reliable way to
identify that window turns up during implementation, the herdr tab focus is
enough and this is dropped.

## 3. Skill: `refine-task`

`.claude/skills/refine-task/SKILL.md` in this repo, symlinked into
`~/.claude/skills/refine-task/` by `install.sh` exactly as `workspace-tasks`
is. `disable-model-invocation: true` — it is only ever invoked by name.

Arguments: `<uuid> [grill]`.

1. **Read.** `task <uuid> export`. Stop if it is missing or not pending. Show
   the description and notes.
2. **Ground.** Read enough of the project (CONTEXT.md, docs, the code the task
   touches) to understand it, read-only.
3. **Work it up.**
   - Quick: draft immediately. Ask only questions the code cannot answer —
     zero to three, one round.
   - Grill: invoke the `grilling` skill on the task and continue until its
     frontier is empty.
4. **Propose** in the reply, then ask with AskUserQuestion — **Write it to the
   task** or **Change something**. The proposal shows:
   - the new description: one line, ≤ ~50 characters, so it fits a card;
   - the new notes: one line each, each becoming one annotation, folding in
     everything the old notes said;
   - the old description and notes beside them, so nothing is dropped
     silently.

   **Change something** sends it back to revise — the approve-or-refine loop.
   The skill never carries out the task; its only write is step 5.
5. **Write.** Re-export the task. If it changed since step 1, show what changed
   and ask before continuing. Otherwise edit the exported JSON — description,
   `annotations` replaced, `planned` added to `tags` — and `task import` it:
   one write, so the task is never left half-updated. Status and other tags are
   untouched.
6. **Verify and report.** Export once more, confirm the three changes landed,
   and say what was written. The tab stays open.

## 4. Planned tasks on screen

- `Task` (`src/task.rs`) gains `#[serde(default)] pub tags: Vec<String>` and
  an `is_planned()` helper.
- `panel/model.rs`: new `Status::Planned`, icon `\u{f014e}`. Precedence is
  Active, then Blocked, then Planned, then Pending.
- `panel/surface.rs`: a `planned` CSS class, styled the same as pending for
  now.
- `rows.rs`: planned rows carry the glyph in the marker slot that holds `▶ `
  for the active task; active still wins.

## Testing

- Unit tests: the herdr argv builders; agent-name derivation from a uuid;
  Planned precedence in `cards`; the picker marker in `rows::build`; `tags`
  absent from export deserialising to empty.
- End to end, against a throwaway herdr session and a throwaway task: Refine
  and Grill me from the menu, the session-not-running path, and picking Refine
  twice on one task.
- Skill: run both modes on a real throwaway task and check the written result
  with `task <uuid> export`.

## To verify during implementation

- Whether Claude asks permission to run the step-5 write. In default mode it
  does, once, after the user has chosen **Write it to the task**.
- How to identify and focus the niri window showing a given herdr session.

## Revision: no plan mode (2026-09-28)

The first version started Claude with `--permission-mode plan` and used plan
approval as the go-ahead for the write. In use, Claude Code's approval screen
reads "ready to execute — would you like to proceed?" and every yes option
means *implement the plan*. A refined task's notes read exactly like a plan
(Steps, Done when), so approving would have started the task instead of saving
it, and there was no option that only saved it.

So the session now starts in `default` permission mode with the file-editing
tools (`Edit`, `Write`, `NotebookEdit`) and the plan-mode tools disallowed: it
cannot start the work, and the implement-this screen never appears. The skill
asks its own two-option question for approval, and says in plain terms that it
refines the task and never does it. `default` is explicit because the user's
own default may run commands unasked.

## Revision: a Bash sandbox instead of prompts (2026-09-28)

Manual mode asked about nearly every command while Claude researched. Auto
mode would stop the prompts, but its guard against starting the task is a
classifier's judgement, which the Claude Code docs say is not a rule. So the
session runs with Claude Code's Bash sandbox (`--settings`,
`refine::sandbox_settings`):

- `autoAllowBashIfSandboxed` — sandboxed commands run without asking;
- `denyWrite` the project folder, `allowWrite` only taskwarrior's data
  location (`task _get rc.data.location`);
- `allowUnsandboxedCommands: false`, `failIfUnavailable: true` — no escape
  hatch, and no session at all without a working sandbox;
- `allowedTools` `Bash(task *)` and `Bash(python3 *)` — Claude Code still asks
  for these two inside the sandbox, and the skill's write is exactly them.

On Ubuntu 24.04+, the sandbox's socket filter cannot load (Ubuntu's
`bwrap-userns-restrict` AppArmor profile denies the capability it needs), so
Unix sockets are allowed and hidden with `denyRead` instead — by whole folder,
not by name: `/run` (the session's and the system's, `/var/run` included),
`/tmp` (X11, VS Code, Chrome; the sandbox keeps its own temp folder under it),
`/var/snap` and `~/.config/herdr`. A first version hid named sockets and
missed Xwayland and VS Code's IPC socket, which is why. Abstract sockets need
nothing: sandboxed commands get their own network namespace. Before starting,
the launcher reads `/proc/net/unix` and refuses to open Claude if any socket
on the machine sits outside those folders (or inside the sandbox's temp
folder), naming it.

Alternatives weighed (2026-09-28): Landlock tools such as landrun only block
pathname sockets from Landlock ABI v9, and this kernel (7.0) has v8; wrapping
all of Claude in bubblewrap breaks its inner sandbox under the same AppArmor
profile and cuts herdr's status hooks off; restoring the socket filter means
overriding Ubuntu's bwrap hardening system-wide; containers or VMs cost the
most plumbing. The built-in sandbox with folder-level hiding was kept.

The session also searches and fetches from the web unasked (`WebSearch`,
`WebFetch` allowed). Because a fetched page could try to talk it into sending
a secret out in a URL, credentials (`refine::CREDENTIALS`: `~/.ssh`,
`~/.gnupg`, `~/.config/gh`, Claude's own credentials and the like) are hidden
from both of its readers — `Read(...)` deny rules for the Read tool, and
`denyRead` for sandboxed Bash. All the rules travel in the one `--settings`
object (`refine::session_settings`). Verified: with that fence, `niri msg`, `busctl --user`,
`docker ps` and herdr all fail to connect, a write to the project fails with
`Read-only file system`, and the full export | python3 | import write succeeds
with no prompt. The sandbox needs `bubblewrap` and `socat` installed.
