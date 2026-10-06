# Refine keeps its fence; a mod takes the write

Refine starts Claude in a herdr tab fenced in from outside: `refine.rs` and
`herdr.rs` hand it a Bash sandbox, hidden sockets and credentials, a list of
removed tools and a standing instruction, all before Claude exists, and the
`refine-task` skill then works the task up, asks for approval with
AskUserQuestion and writes it with a `python3 | task import` pipe. Claude
Code 2.1.287 added mods: a plugin hooks module, loaded for one session with
`--plugin-dir`, that can register tools, guard tool calls, draw panes and
act at session start. This records, piece by piece, which of Refine's parts
such a mod should take over.

A mod here means the new kind: a TypeScript function-hooks module that
exports `register(on, options)` and reaches everything outside itself
through the `$` engine interface (`reference.md`, "What a plugin of function
hooks is"), not a classic plugin of command hooks in `settings.json` style.
Its contract for this build is `types/claude-code.d.ts`, cited below as
`d.ts:<line>`.

Decided before the review, and recorded rather than reopened:

- The OS sandbox, credential and socket hiding and `ensure_no_exposed_sockets`
  stay. A mod runs inside the session it would guard, so it can add to the
  fence, not replace it.
- herdr and niri orchestration stays in `refine.rs`. It runs before Claude
  exists.
- A mod in TypeScript is acceptable. It touches no Rust, only the Claude
  session, so the language is not weighed against any candidate below.

The two that move are the write and the approval in front of it. Today both
are instructions: the skill asks the model to wait for **Write it to the
task**, then to paste the approved JSON into a heredoc character for
character. A tool the mod registers can do the write itself, through a fixed
argv, and can ask the person in the engine's own dialog before it does, with
the answer going to the mod rather than the model. That enforces the
approved write path: a write through the tool happens only after the
person's answer, and only with the payload the mod showed them, drawn from
the tool's own arguments rather than from the block the model printed. It
does not enforce that every write goes through the tool. The sandbox still
lets any command write the task data, so a write around the tool is still
stopped only by the instruction (see Consequences). Everything else stays
where it is, because a flag or a setting set from outside the session does
it as well or better.

## Verdicts

| Piece | Today | Verdict | Why, in a line |
|---|---|---|---|
| herdr and niri orchestration | `refine.rs` `launch` | Keep | Runs before Claude exists, so no session-scoped mod can do it. |
| OS sandbox, socket and credential hiding, and the `WebSearch`/`WebFetch` allows | `refine.rs` `session_settings`, `ensure_no_exposed_sockets` | Keep | A mod runs inside the session it would guard: it can add to the fence, not replace it. The web allows write nothing and are what grounding needs. |
| `task`/`python3` allows | `refine.rs:109` | Move (A), in part | `Bash(python3 *)` goes with the heredoc; `Bash(task *)` stays for the skill's reads. |
| Removed tools | `herdr.rs` `--disallowedTools` | Keep | The flag removes the tools from the model's list from outside the session; a guard would leave them listed and fail open. |
| Standing instruction | `herdr.rs` `REFINER_SYSTEM_PROMPT` | Keep | Not a candidate; `prompt.section` could add a section, but the flag already does it from outside. |
| Kick-off prompt | `refine.rs` `agent prompt` | Keep | No Refine prompt has been lost, and the herdr agent is still needed to find the tab. |
| Approval | skill §4 AskUserQuestion | Move (B) | The write tool shows its own payload, then asks with `$.ui.ask`, so the tool writes nothing without the answer. |
| The write | skill §5 heredoc | Move (A) | A registered tool writes the approved payload through a fixed argv; the model no longer re-types it. |
| Reading, grounding, drafting | skill §1-3, §6 | Keep | Instructions to the model; a skill is the right home. |
| Task on screen | herdr tab label | Keep (E not added) | The label already names the mode and task; the transcript reports the write. |

## Considered options

### A — A registered `write_task_plan` tool

**Replaces:** the `python3` one-liner and heredoc in `refine-task` §5, and
the `Bash(python3 *)` allow at `refine.rs:109`.

**Gains:** `$.tool.register` declares `mcp__<plugin>__write_task_plan`,
served by the mod's own `tool.call` hook (`d.ts:2969`, `reference.md`
"Tools and agent types the model can call"). The model passes the
description and notes as data; the mod runs
`["task", "rc.hooks=off", "rc.json.array=on", <uuid>, "export"]`, merges the
fields in the module, and pipes the result to
`["task", "rc.hooks=off", "rc.verbose=nothing", "import"]` on stdin.
`$.process.run` takes an argv with no shell and a `stdin` string
(`d.ts:3425`, `d.ts:7768`); a probe mod on 2.1.291 confirmed both. The uuid
comes from the session, not the model: `refine.rs` already builds the
`--settings` JSON per launch, and a probe confirmed that its
`pluginConfigs.<plugin>.options` reach `register(on, options)`
(`d.ts:7405`, `reference.md` "Developing one"). So the model can write only
the description, the notes and `+planned`, on its own task. The tool also
takes the description and notes the skill read in step 1, and refuses if the
export at write time differs from them, so §5's stale-write check moves into
the tool instead of being left to the model; a concurrent edit is reported,
not overwritten. The tool answers
at `tool.call` without calling `next`, so core's permission prompt never runs
(`d.ts:3877`): no `allow` rule is needed. That should also settle the
auto-mode task's worry (`a67de29b`) that auto mode drops `Bash(python3 *)`
and hands the write to its classifier, since the call never reaches the
permission decision; the probe checked this in `default` mode only.
If the mod is not loaded, the tool does not exist and the approved write
path fails closed; a call no hook answers fails too (`d.ts:2969`). That
covers the tool alone: a write the model makes around it is the residual
risk in Consequences.

**Costs:** `$.process.run` is not sandboxed. It runs "as the user the
session runs as" (`d.ts:3425`), and a probe's `touch` landed in a folder the
session's sandbox `denyWrite`s. The write is still no wider than today's,
because the argv is fixed and the payload is data, but two things follow.
`Bash(task *)` and `allowWrite` on the task data stay: the skill reads with
`task … export` (§1, §5, §6), and Taskwarrior 2.6.2 will not export from
read-only data files (tested on a `chmod 444` copy). And `~/.task/hooks` is
inside that `allowWrite` with hooks on, so the argv must carry
`rc.hooks=off`, or a hook the session planted would run outside the sandbox.
That also skips any hook the user adds to Taskwarrior later, for this one
write. The fixed argv relies on `~/.taskrc` and anything it includes lying
outside `allowWrite`; here `~/.taskrc` is outside `~/.task` and its
`include` lines are all commented out. The mod can also read the merged
settings at `session.start` (`$.settings.read`, `d.ts:3497`) and register no
tool unless the sandbox is on and `failIfUnavailable` is set. (And the
shared costs in Consequences.)

**Verdict:** Move. The hypothesis held: there is a fixed-argv route, and with
`rc.hooks=off` it is no wider than today's sandboxed write. Candidate A
stays as the spec named it, with no narrowing of `Bash(task *)`: that would
not stop a write around the tool, which Consequences records as the residual
risk.

### B — A pane with Write and Change buttons

**Replaces:** the AskUserQuestion in `refine-task` §4.

**Gains:** both conditions hold. A Button's `onPress` runs "in the plugin's
own environment" (`d.ts:1103`, `ui.press` at `d.ts:3924`), so a press can
run the write with no model turn, and a `Code` element can show the exact
payload (`d.ts:1570`). Approval of the tool's write becomes enforced: the
tool writes nothing without the press. A write around the tool is not
covered; see the residual risk in Consequences.

**Costs:** a pane is the wrong shape for it. `$.ui.open` places a pane the
person did not ask for only from 144 columns, 110 once asked, and below that
it waits undrawn (`d.ts:2411`); a herdr tab in a niri column is often
narrower. Holding the write tool's call open until a press means polling
with `$.process.run(["sleep", "0.25"])`, as the mods blog's Blast Radius
example does, to stay inside the 10-second hook budget (`d.ts:5019`), plus a
band fallback for when the pane is not placed. `$.ui.ask` (`d.ts:2366`) does
the same job with none of that. It is a `$` call, and a hook's budget
"bounds the hook's OWN time: the clock stops while a `next(e)` call or any
`$` call of the hook's is in flight" (`HookBudget`, `d.ts:5003`). So the
write tool's hook can await the person's answer for as long as they take. A
pane's press, by contrast, arrives in a separate `ui.press` dispatch, which
is why a hook waiting for one has to keep making `$` calls. Awaited inside
the write tool's hook, `$.ui.ask` asks in the engine's own AskUserQuestion
dialog, at any width, and resolves to the label chosen or the text typed
under Other, to the mod, not to the model. On **Write it to the task** the
mod writes; on anything else it answers the call with the person's words and
the model revises.

`$.ui.ask` takes only labels, a header and `multiSelect` (`AskOptions`,
`d.ts:626`), with no preview, so the dialog cannot show the payload. The mod
shows it first with `$.ui.log`, which "appends one line to the transcript,
drawn like a system notice (dim; not sent to the model)" (`d.ts:2349`): one
line for the description and one per note, each under the 2,000 characters
the terminal draws of a line, the call refused if one is longer. What the
person approves is then what the tool will write, whatever the model printed
above it. `$.ui.ask` rejects in a `-p` run, so the probe could not exercise
it, and the build task proves the Write branch in a live session. The other
branch can be tested under `claude plugin test`: `$.ui.ask` is "a
`tool.call` of `AskUserQuestion` through every hook but the calling one"
(`d.ts:2369`), and a test's own hooks sit beneath the plugin
(`reference.md:77`), so a test hook can answer the question. (And the shared
costs in Consequences.)

**Verdict:** Move, but not as a pane. The hypothesis held on enforcement and
is overturned on form: the approval moves into the write tool as a
`$.ui.ask`, and the skill's §4 shows the proposal and calls the tool instead
of asking. The tool shows the payload from its own arguments with
`$.ui.log` before it asks.

### C — A `tool.call` guard in place of `--disallowedTools`

**Replaces:** `--disallowedTools Edit Write NotebookEdit EnterPlanMode
ExitPlanMode` in `herdr::agent_start_claude_refiner`.

**Gains:** none the session needs. A guard can refuse a `Bash` call by its
command, but `--disallowedTools` takes scoped rules such as `Bash(rm *)`
too ([CLI reference](https://code.claude.com/docs/en/cli-reference)), and
the sandbox, not a rule, is what stops writes.

**Costs:** a bare name in `--disallowedTools` "removes the matching tools
from Claude's context" (CLI reference); a guard leaves them listed and costs
a refused call each time the model tries one. A guard that throws is skipped
unless it carries a `.catch` (`d.ts:3872`), and it lives inside the session
it guards, which is the argument that kept the sandbox outside. (And the
shared costs in Consequences.)

**Verdict:** Keep the flag. The hypothesis held.

### D — A `session.start` hook in place of `herdr agent prompt`

**Replaces:** `herdr agent prompt task-<uuid8> "/refine-task <uuid> [grill]"`
at `refine.rs:341`.

**Gains:** the route is clean: the uuid and mode can reach the mod through
`--settings` (`pluginConfigs` or `env`, both confirmed by the probe), and
`$.prompt.submit` queues a prompt for when the session is idle
(`d.ts:2871`).

**Costs:** it removes no failure. The only lost prompt on record is Start
working's, sent just after Claude's folder-trust question on a new worktree
(`8732a46`, `agent_prompt_confirmed`); Refine opens in the workspace's
project folder, not a fresh worktree, and no commit or `+niri_tasks` task
reports a lost Refine prompt. `session.start` fires again on every fresh
load of the plugin (`d.ts:4239`), so a prompt sent there needs a once-only
guard, and `agent_name`/`agent get` stay for "Already being refined"
(`refine.rs:319-323`). (And the shared costs in Consequences.)

**Verdict:** Keep `herdr agent prompt`. The hypothesis held.

### E — A status line naming the task

**Replaces:** nothing; it would sit beside `tab_label`.

**Gains:** `$.ui.status` pins one line per plugin under the prompt, beside
the engine's notices, without replacing the user's own `statusLine`
(`d.ts:2399`). It could show the full uuid, the mode and whether the write
has happened, which the tab label (`refine.rs:51-64`) does not.

**Costs:** the label already shows the mode and the task, and the skill's §6
reports the write in the transcript. Once A exists, a status after the
write is one line in the same module, not a piece of its own.

**Verdict:** Keep (do not add). The hypothesis held; if the build task wants
the line, it comes with A.

## Other mod features considered

**`prompt.section` and `prompt.compose`** (`d.ts:4071`, `d.ts:4095`) touch
the standing instruction. A hook can append a session section to the
system prompt, but `--append-system-prompt` already does that from outside
the session, and the system prompt survives a summary either way. Keep the
flag.

**`skill.prompt`** (`d.ts:4226`) touches the skill's text, and so both
"Reading, grounding, drafting" and, in the case that matters, "The write".
It fires when the engine expands a skill's prompt for the model, and a hook
can return other text in its place, so the mod could rewrite `refine-task`
as it loads: for one, dropping §5's heredoc in favour of "call the write
tool" only when the mod is loaded. Keep the skill as the one text: installed
and linked by `install.sh`, it can name the tool directly once A lands. A
second copy of its words inside a module is the drift ADR 0001 warns
against.

**`tool.check`** (`d.ts:3889`) touches the `task`/`python3` allows. A guard,
or a narrower settings rule such as `Bash(task rc.json.array=on * export)`
in place of `Bash(task *)`, would make `task … modify` ask the person. That
closes only the obvious route. Taskwarrior needs `allowWrite` on its data
even to export, and `autoAllowBashIfSandboxed` runs any sandboxed command
unasked, so `sed -i` on `pending.data` would still go through. Not adopted;
the residual risk is recorded in Consequences.

**`$.settings.read`** (`d.ts:3497`) strengthens A: the mod refuses to
register its tool unless the merged settings show the sandbox on. Taken into
A, not a piece of its own.

**`$.ui.ask`** (`d.ts:2366`) is the form B takes; see B.

Not relevant to any piece of Refine:

- `$.fs.stat(…, { resolve: true })` allow-lists (`d.ts:3205`): the file tools
  that write are removed, and reads are fenced by deny rules and the sandbox.
- `$.agent.register` (`d.ts:3121`) and `agent.offer` (`d.ts:4001`): Refine is
  the main session, not a subagent, and its fence comes from outside.
- `$.command.register` (`d.ts:3010`): a `/refine` command would move the
  skill's instructions into module code; a skill is their home.
- `$.model.complete` and `$.model.fork` (`d.ts:2525`, `d.ts:2552`): Refine
  needs no side completions.
- `session.append` (`d.ts:4273`): nothing to add to the transcript that the
  write tool's result does not already say.
- The `AbovePrompt` band (`d.ts:9924`): only a pane's fallback, and B needs
  no pane.
- `$.store` (`d.ts:3277`): nothing outlives a refine session but the task.
- `turn.complete` (`d.ts:4371`): nothing to check per turn once the write is
  a tool.
- `tool.describe` (`d.ts:4130`): the write tool's description is set when it
  is registered.
- `prompt.submit` guards (`d.ts:4023`): the person's prompts are not a risk.
- `$.ui.notice` (`d.ts:2285`): it annotates permission dialogs, and the write
  tool raises none.
- `session.compact` (`d.ts:4297`): it rewrites what a summary keeps, but the
  standing instruction is in the system prompt, which compaction does not
  summarise, and the skill re-reads the task before writing.
- `config.set` (`d.ts:4180`): it guards `/config` rows, a plugin's
  `userConfig` included. A's per-session uuid comes in through `--settings`
  `pluginConfigs` and is never set from the menu.
- `session.end` (`d.ts:4341`): nothing to tidy when a refine session ends;
  herdr closes the tab.
- `$.process.spawn` (`d.ts:3485`): A's two `task` runs are one-shot, which is
  what `$.process.run` is for.
- `$.ui.toast` (`d.ts:2397`): the write tool's result already tells the model
  and the person what was written.
- `prompt.fill` and `prompt.suggest` (`d.ts:4045`, `d.ts:4057`): the
  kick-off prompt stays with herdr (D), and nothing else needs drafting into
  the box.

## Consequences

The shared costs, once:

- **A second test runner.** The mod is checked with `claude plugin validate`
  and `claude plugin test` beside `cargo test` (`reference.md`, "The types
  are the reference" and "Developing one").
- **Loading.** `herdr::agent_start_claude_refiner` adds `--plugin-dir
  <folder>` to Claude's argv, a change the build task makes. `niritasks` is
  `cargo install`ed and does not know the repo's path when it runs, so
  `install.sh` links the mod's folder to a fixed path, as it already links
  the skills, and `herdr.rs` passes that path. The link must not go under
  `~/.claude/skills/<name>`: a plugin there is loaded in every session
  (`reference.md`, "Developing one").
- **The version floor.** Claude Code 2.1.287 or later, and the API "is early
  access and moves between releases: the declaration file is the authority"
  (`reference.md`). A Claude Code update can break the mod.
- **The session cannot edit its own mod.** The mod's folder lies outside
  every path the sandbox may write (the project is `denyWrite`, `allowWrite`
  is the task data alone, `session_settings`) and Edit and Write are
  removed, so the session can neither change its guard nor trigger a hot
  reload of it.

Keeping the fence outside is what makes the version floor tolerable. If the
mods API changes and the module fails to load, the write tool is missing and
the approved write path is gone; Refine does not become unfenced. The
sandbox, the hidden sockets and credentials, the removed tools and the
standing instruction are all flags and settings that a mod failure does not
touch.

The residual risk A and B leave: the approved write is enforced, but a write
around the tool is not. Taskwarrior 2.6.2 needs `allowWrite` on its data even
to export (tested on a `chmod 444` copy), so the sandbox keeps write access to
`~/.task`. `Bash(task *)` stays for the skill's reads, and
`autoAllowBashIfSandboxed` runs any sandboxed command unasked. A model that
ignored the skill and the standing instruction could still run
`task <uuid> modify …`, or rewrite `pending.data` with any command, without
the tool or `$.ui.ask`. Narrowing the `task` rule would not close this, since
the second route needs no `task` at all, so it is not part of A. What bounds
it is the same as today: the instruction, the sandbox keeping the write to
the task data, and Taskwarrior's undo. Closing it means a write path the
sandbox cannot reach at all, such as reads served by the mod too and no
`allowWrite`. That is a larger change than this review weighs.

Found along the way, outside the mod question: `allowWrite` on the task data
covers `~/.task/hooks`, and Taskwarrior hooks are on, so a refine session can
already write a hook that the user's own unsandboxed `task` commands would
then run. A `denyWrite` on that folder in `session_settings` would close it
whether or not a mod is built (task `b268d068`).

## Adopted

Both are built, from these `+niri_tasks` tasks:

- A — `feat: Write a refined task through a mod tool` (task `5ff28e6f`) — built
- B — `feat: Approve a refine's write inside the tool` (task `dc32d340`) — built

## Sources

- [Getting started with Claude Code mods](https://claude.dev/blog/getting-started-with-claude-code-mods/),
  read 2026-10-06.
- The `plugin-authoring` skill's `reference.md` and `types/claude-code.d.ts`,
  from Claude Code 2.1.291.
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference),
  for `--disallowedTools`.
- A probe mod run with `claude -p --plugin-dir` on 2.1.291: `$.process.run`
  outside the Bash sandbox, its `stdin`, no permission prompt for a served
  tool, and `pluginConfigs` and `env` from `--settings`.
- `src/refine.rs`, `src/herdr.rs`, `.claude/skills/refine-task/SKILL.md`,
  `install.sh`, `docs/superpowers/specs/2026-09-29-start-working-design.md`,
  commit `8732a46`.
