# Review Moving Refine onto a Claude Code Mod — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decide, piece by piece, which parts of Refine a Claude Code mod should take over, record the verdicts and their reasons in `docs/adr/0002-*.md`, and file a pending niritasks task for each piece adopted.

**Architecture:** This is a decision, not a build. Task 1 checks each candidate against the mods API as this machine's Claude Code build defines it, then writes the ADR. Task 2 files one task per adopted piece and links the tasks from the ADR. No Rust, skill or install change is made.

**Tech Stack:** Claude Code 2.1.291 mods (plugin hooks modules), Taskwarrior 2.6.2 via `niritasks`, Markdown.

**Spec:** Taskwarrior task `ddd1b95d-21e4-4ce8-ab96-7a4c7b8dc738` — read it with `task rc.json.array=on ddd1b95d-21e4-4ce8-ab96-7a4c7b8dc738 export`. Its notes are the spec; the ones this plan quotes are copied verbatim under Global Constraints.

## Global Constraints

- Done when: "docs/adr/0002 records keep or move for each piece of Refine with reasons and sources, and each adopted piece is a pending task on niri_tasks".
- Decided: "The OS sandbox, credential and socket hiding and ensure_no_exposed_sockets stay; a mod runs inside the session it would guard, so it can add to the fence, not replace it". Record it; do not reopen it.
- Decided: "herdr and niri orchestration stays in refine.rs; it runs before Claude exists". Record it; do not reopen it.
- Out of scope: "Building the mod; changing refine.rs, herdr.rs or the skill; Start working's sessions". No file outside `docs/adr/` is changed (this plan file aside).
- The five candidates to weigh, verbatim: "a registered write_task_plan tool for the heredoc and the Bash(python3 *) allow; a Pane with Write/Change buttons for AskUserQuestion; a tool.call guard for --disallowedTools; a session.start hook for herdr agent prompt; a status line naming the task".
- Each candidate gets "what it replaces, gains and costs (JS in a Rust repo, install.sh linking a plugin, Claude Code 2.1.287+)".
- Sources to cite: https://claude.dev/blog/getting-started-with-claude-code-mods/ and the plugin-authoring skill's `reference.md`; "a mod is a plugin hooks module (Claude Code 2.1.287+) loaded per session via --plugin-dir".
- Every task description starts with a lowercase Conventional Commits type, a colon and a space, and fits about 50 characters including the type (`claude/rules/niri-tasks.md`).
- Tasks are filed with `niritasks task add` from a pane of the `niri_tasks` herdr session, so they land on `+niri_tasks`; tasks are addressed by uuid, never by numeric id.
- Commits follow Conventional Commits with the `docs(adr):` type and scope; end each message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- The branch is landed afterwards with the `finish-worktree` skill, not `superpowers:finishing-a-development-branch`.

## Where Refine lives today (read before Task 1)

Every row below is a piece the ADR must give a verdict on.

| # | Piece | Where | What it does |
|---|---|---|---|
| 1 | herdr/niri orchestration | `src/refine.rs:259-344` (`show_session_window`, `open_session`, `launch`), `wait_for_session` at `:349` | Opens or focuses the session's window, reuses the task's tab by agent name `task-<uuid8>`, makes a tab, starts Claude, sends the prompt. **Decided: keep.** |
| 2 | OS sandbox, credential and socket hiding | `src/refine.rs:66-220` (`CREDENTIALS`, `session_settings`, `hidden_paths`, `ensure_no_exposed_sockets`) | `--settings` JSON: Bash sandbox, `denyWrite` project, `allowWrite` task data, `denyRead` sockets and credentials, `Read(...)` deny rules; refuses to launch if a socket is reachable. **Decided: keep.** |
| 3 | Permission allows | `src/refine.rs:109` | `"allow": ["WebSearch", "WebFetch", "Bash(task *)", "Bash(python3 *)"]` — `task` and `python3` are there only for the skill's write. |
| 4 | Tool removal | `src/herdr.rs:95-106` (`agent_start_claude_refiner`) | `--permission-mode default --disallowedTools Edit Write NotebookEdit EnterPlanMode ExitPlanMode`. |
| 5 | Standing instruction | `src/herdr.rs:67-74` (`REFINER_SYSTEM_PROMPT`) | `--append-system-prompt`: "You are refining a Taskwarrior task, not doing it…" |
| 6 | Kick-off prompt | `src/refine.rs:39-46` (`prompt`), `:341` | `herdr agent prompt task-<uuid8> "/refine-task <uuid> [grill]"`. |
| 7 | Approval | `.claude/skills/refine-task/SKILL.md` §4 | AskUserQuestion "Write this to the task?" with **Write it to the task** / **Change something**; the model is trusted to honour the answer. |
| 8 | The write | `.claude/skills/refine-task/SKILL.md` §5 | `task export | python3 -c '…' "$(cat <<'EOF' … EOF)" | task import`, the approved JSON pasted into the heredoc by the model. |
| 9 | Reading, grounding, drafting, verifying | `.claude/skills/refine-task/SKILL.md` §1-3, §6 | Instructions to the model. |
| 10 | Task identity on screen | `src/refine.rs:51-64` (`tab_label`) | herdr tab label `Refine: <description…>`. A status line would be a second place. |

The mods reference is at
`/tmp/claude-1000/bundled-skills/2.1.291/7728c92798b7d4a80f08d6667eb48a48/plugin-authoring/reference.md`,
with the exact contract in `types/claude-code.d.ts` and worked examples in
`examples/` (`tool-call.ts`, `pane.tsx`, `band.tsx`) beside it. If that
folder is gone (it is a per-version temp extract), load the
`plugin-authoring` skill and use the paths it names. The reference says it
itself: "The API is early access and moves between releases: the declaration
file is the authority, this note the map." Cite the types file for every
fact the ADR rests on, by the identifier and the line number in it.

Facts already found in `reference.md` that bear on the verdicts (verify
each in the types file before relying on it):

- A hooks module "runs in an environment of its own, with no DOM and no
  Node: everything outside it is reached through `$`", and "`$.process.run`
  runs a host command by argv". So a tool a mod registers does its work
  through `$.process.run` or `$.fs` — **not** through the Bash tool, and so
  not inside the Bash sandbox row 2 sets up. Find out whether `$.process.run`
  is sandboxed in any way; this decides candidate A.
- "`$.tool.register` declares a tool … listed as `mcp__<plugin>__<name>`. The
  plugin serves it by hooking `tool.call` with the matcher
  `{ tool: 'mcp__<plugin>__<name>' }`."
- A guard is written `on(...).catch(($, e, next) => next.called ? next(e) : { deny: 'why' })`;
  "Without the handler, what the guard judges goes on when it throws,
  answers a wrong shape or outlasts its budget". So a guard without `.catch`
  fails open.
- A `session.start` hook "fires once when the session is ready and is awaited
  before the first prompt", and "`$.prompt.submit` queues a prompt that
  starts a turn of its own once the session is idle".
- `claude --plugin-dir <folder>` "loads the plugin from disk for that session
  only"; in an interactive session the folder is watched and a save reloads
  the module.
- `claude plugin validate <dir>` lists a module's `gatingHooks`;
  `claude plugin test <folder>` runs its `*.test.ts` files.

The installed build is `claude --version` → `2.1.291 (Claude Code)`, above
the 2.1.287 floor.

---

### Task 1: Weigh each piece and write ADR 0002

**Files:**
- Create: `docs/adr/0002-<slug>.md` — the slug is the decision in a few
  hyphenated words, chosen once the verdicts are in (Step 8); 0001's is
  `0001-task-panel-in-gtk-not-quickshell.md`, so e.g.
  `0002-refine-keeps-its-fence-mod-adds-approval.md` if the verdicts land
  that way.
- Read only: `src/refine.rs`, `src/herdr.rs`, `.claude/skills/refine-task/SKILL.md`, `install.sh`, `docs/adr/0001-task-panel-in-gtk-not-quickshell.md`, `CONTEXT.md`.

**Interfaces:**
- Consumes: nothing from another task.
- Produces: the ADR file, whose **Adopted** list (Step 7) names each adopted
  piece by a short label (`A`–`E` below) and the one-line task description
  Task 2 files for it.

Label the five candidates and use the labels throughout the ADR:

- **A** — registered `write_task_plan` tool, replacing rows 3 (`Bash(python3 *)`, maybe `Bash(task *)`) and 8.
- **B** — Pane with Write/Change buttons, replacing row 7.
- **C** — `tool.call` guard, replacing row 4.
- **D** — `session.start` hook, replacing row 6.
- **E** — status line naming the task, beside row 10.

Row 5 (`--append-system-prompt`) is not in the candidate list; the ADR
records it as kept unless Step 2 finds a mod hook that clearly does it
better, in which case it says so as a note, not a sixth candidate.

- [ ] **Step 1: Read the sources**

Fetch https://claude.dev/blog/getting-started-with-claude-code-mods/ with
WebFetch and read it whole. If it does not load, say so in the ADR's
Sources and rely on `reference.md` and the types file. Then read
`reference.md` whole, and in `types/claude-code.d.ts` read the declarations
found by:

```bash
T=/tmp/claude-1000/bundled-skills/2.1.291/7728c92798b7d4a80f08d6667eb48a48/plugin-authoring/types/claude-code.d.ts
grep -n "process\b\|process:\|run(\|interface Process\|tool\.call\|tool\.check\|register(\|session\.start\|prompt\.submit\|Pane\|statusLine\|status(\|ui\.status\|system\.prompt\|systemPrompt\|userConfig\|env\b" "$T" | head -200
```

Expected: line numbers for `$.process.run`, `$.tool.register`, the
`tool.call` and `tool.check` events, `session.start`, `$.prompt.submit`,
the `Pane` component and `$.ui.open`/pane opening, `$.ui.status`, the
system-prompt event, and how a plugin gets per-session values
(`userConfig` options or the environment). Note each line number; the ADR
cites them.

- [ ] **Step 2: Answer the questions that decide each candidate**

Write the answers into `refine-mod-answers.md` in the session's scratchpad
directory (never the repo). Each answer is one or two sentences with its
source (types file line, reference.md section, or blog paragraph).

A — `write_task_plan` tool:
1. Does `$.process.run` run inside Claude Code's Bash sandbox, or as the
   Claude Code process itself, unsandboxed? (Look at its doc comment in the
   types file and in `reference.md`'s "Work that outlives a dispatch".)
2. Can the tool's handler confine itself to a fixed argv —
   `["task", "rc.json.array=on", uuid, "export"]` then
   `["task", "rc.verbose=nothing", "import"]` with stdin — and does
   `$.process.run` take stdin? If not, could `$.fs` write a temp file that
   `task import <file>` reads?
3. Does a registered tool still go through Claude Code's permission prompt,
   and would `--settings` `allow` need `mcp__<plugin>__write_task_plan`?
4. What it removes: the python3 one-liner, the heredoc the model copies the
   approved JSON into by hand, and the `Bash(python3 *)` allow. Whether
   `Bash(task *)` can go too depends on the skill's reads (§1, §5, §6 all run
   `task … export`), so it likely stays.
5. Net fence: today the write runs sandboxed with `allowWrite` only on the
   task data. With A it runs outside the sandbox but through a fixed argv
   the model cannot change. Say which is narrower and why.

B — Pane with Write/Change buttons:
1. Can a Button's press handler run code of the mod's (call
   `$.process.run`, or call the A tool's write directly) without the model
   taking a turn? Find the press event in the types file.
2. Can the mod show the exact payload in the pane (a `Code` element with the
   JSON) so what the person approves is what is written, character for
   character, by the mod rather than by the model re-typing it?
3. Does the pane work in herdr's terminal pane — the terminal's fullscreen
   layout docks it; the main screen opens it inline (`e.viewport.isFullscreen`)?
4. Gain to weigh: with B, approval becomes enforced — the model can no
   longer write without the press — where today SKILL.md §4-5 only asks it
   to wait. Combined with A, a `tool.call` hook can refuse the write unless
   the pane's Write was pressed for the same payload.

C — `tool.call` guard:
1. `--disallowedTools` is set on the command line, outside the session.
   A guard runs inside it and is skipped if it throws, unless it has a
   `.catch`. Is there anything a guard can do that the flag cannot (for
   example, refusing a `Bash` command by its argv rather than a whole tool)?
2. Does `--disallowedTools` also hide the tools from the model's tool list,
   which a guard would not (a refused call costs a turn)?

D — `session.start` hook:
1. How does the mod learn the task uuid and mode? (Plugin `userConfig`
   options are settings-wide, not per session; check whether `$.env` or a
   `--plugin-dir` per-task folder is the only route.)
2. Today `refine::launch` sends the prompt with `herdr agent prompt`
   (`src/refine.rs:341`) after `agent start` returns. `herdr.rs` already has
   `agent_prompt_confirmed` for Start working's "prompt swallowed by a
   Claude still finishing its start-up" case — has Refine's prompt ever been
   lost? (`git log --oneline -S agent_prompt_confirmed` and the task history
   `task +niri_tasks all | grep -i prompt` say.) D only gains if it removes
   a real failure.
3. D would not remove `agent_name`/`agent get`: "Already being refined"
   (`src/refine.rs:319-323`) still needs the herdr agent.

E — status line:
1. Is a status line a mod event or `$.ui.status`, and does it replace the
   user's own `statusLine` setting or sit beside it?
2. The herdr tab label (`tab_label`) already names the task in herdr's
   sidebar. What does E show that the label does not (the uuid, the mode,
   whether the write has happened)?

Cross-cutting costs, to state once in the ADR and refer to per candidate:
- JS/TS in a Rust repo: a new language, `claude plugin validate` and
  `claude plugin test` as a second test runner beside `cargo test`.
- Loading: `herdr::agent_start_claude_refiner` would add
  `--plugin-dir <repo>/claude/refine-mod` (or wherever the mod lives) to
  Claude's argv — a change to herdr.rs that a later build task makes, not
  this one. Whether `install.sh` must link anything depends on that: a
  `--plugin-dir` path into the repo needs no link; check and say which.
- Version floor: Claude Code 2.1.287+, and an early-access API that "moves
  between releases", so a Claude Code update can break Refine.
- The mod's folder sits in the project the sandbox `denyWrite`s, so the
  session cannot edit its own guard — note this as a property, verify it
  from `session_settings`.

- [ ] **Step 3: Decide each candidate**

For each of A–E, decide **move** (adopt the mod piece) or **keep** (stay as
today). Starting hypotheses — overturn any the answers contradict, and say
so in the ADR:

- A: adopt only if Step 2 A.2 finds a fixed-argv route and A.5 judges it no
  wider than today's sandboxed write. Otherwise keep.
- B: adopt if B.1 and B.2 hold — it is the one candidate that turns an
  instruction into enforcement. If B.1 fails, keep AskUserQuestion.
- C: keep the flag. The flag is outside the session, which is the same
  argument as the sandbox decision; a guard adds nothing unless C.1 finds a
  per-argv refusal the flag cannot make.
- D: keep `herdr agent prompt` unless D.1 has a clean per-session route
  and D.2 shows a real lost-prompt failure.
- E: keep (do not add) unless E.2 finds something the tab label cannot show.

- [ ] **Step 4: Write the ADR header and context**

Follow 0001's form: an H1 that states the decision as a sentence, then
prose, then `## Considered options` and `## Consequences`. Start the file
with:

```markdown
# Refine keeps its fence outside Claude; a mod may take over <the adopted pieces>

Refine starts Claude in a herdr tab fenced in from outside: `refine.rs` and
`herdr.rs` hand it a Bash sandbox, hidden sockets and credentials, a list of
removed tools and a standing instruction, all before Claude exists, and the
`refine-task` skill then works the task up, asks for approval with
AskUserQuestion and writes it with a `python3 | task import` pipe. Claude
Code 2.1.287 added mods: a plugin hooks module, loaded for one session with
`--plugin-dir`, that can register tools, guard tool calls, draw panes and
act at session start. This records, piece by piece, which of Refine's parts
such a mod should take over.
```

Adjust the H1 to the verdicts: if nothing is adopted, it reads
`# Refine stays as it is; a Claude Code mod takes over none of it`.

- [ ] **Step 5: Write the verdict table**

Under the context, a `## Verdicts` section with one row per piece of the
"Where Refine lives today" table (all ten), so the Done-when's "each piece
of Refine" is visibly covered:

```markdown
## Verdicts

| Piece | Today | Verdict | Why, in a line |
|---|---|---|---|
| herdr and niri orchestration | `refine.rs` `launch` | Keep | Runs before Claude exists, so no session-scoped mod can do it. |
| OS sandbox, socket and credential hiding | `refine.rs` `session_settings`, `ensure_no_exposed_sockets` | Keep | A mod runs inside the session it would guard: it can add to the fence, not replace it. |
| `task`/`python3` allows | `refine.rs:109` | <Keep or Move (A)> | <from Step 3> |
| Removed tools | `herdr.rs` `--disallowedTools` | <Keep or Move (C)> | <from Step 3> |
| Standing instruction | `herdr.rs` `REFINER_SYSTEM_PROMPT` | Keep | Not a candidate; <one line from Step 2 if a hook was found>. |
| Kick-off prompt | `refine.rs` `agent prompt` | <Keep or Move (D)> | <from Step 3> |
| Approval | skill §4 AskUserQuestion | <Keep or Move (B)> | <from Step 3> |
| The write | skill §5 heredoc | <Keep or Move (A)> | <from Step 3> |
| Reading, grounding, drafting | skill §1-3, §6 | Keep | Instructions to the model; a skill is the right home. |
| Task on screen | herdr tab label | <Keep, or Add (E)> | <from Step 3> |
```

Every `<…>` here is filled from Step 3 before the commit; none survives
into the file.

- [ ] **Step 6: Write the considered options**

`## Considered options`, one `### A — …` to `### E — …` subsection per
candidate, each with exactly these bold lead-ins, a short paragraph each,
and the sources inline as links or `types/claude-code.d.ts:<line>`:

```markdown
### B — A pane with Write and Change buttons

**Replaces:** the AskUserQuestion in `refine-task` §4.
**Gains:** …
**Costs:** … (and the shared costs in Consequences)
**Verdict:** Move / Keep, because …
```

- [ ] **Step 7: Write the consequences and adopted list**

`## Consequences` states the shared costs once (JS/TS in a Rust repo, the
second test runner, `--plugin-dir` and whether `install.sh` changes, the
2.1.287 floor and the early-access API), then what keeping the fence
outside means if the mods API changes. Then:

```markdown
## Adopted

Each is a pending task on `+niri_tasks`:

- B — `feat: Approve a refine from a pane, not a question` (task `<uuid8>`)
```

one line per adopted candidate, description written to the Global
Constraints rule (type first, ~50 characters). Leave `<uuid8>` as it is —
Task 2 fills it in. If nothing is adopted, write `None: every piece stays
as it is.` and Task 2 files nothing.

Then `## Sources`: the blog URL, `reference.md` and `types/claude-code.d.ts`
with the Claude Code version they came from (2.1.291), and the repo files
of the table above.

- [ ] **Step 8: Name the file and check it**

Rename to the final slug, then check every candidate and every row is there
and no template marker survived:

```bash
ls docs/adr/
grep -c "^### [A-E] —" docs/adr/0002-*.md
grep -n "<Keep\|<from Step\|<one line\|<the adopted" docs/adr/0002-*.md
```

Expected: one `0002-*.md`; `5`; no output from the last grep (only
`<uuid8>` under Adopted may remain).

- [ ] **Step 9: Commit**

```bash
git add docs/adr/0002-*.md
git commit -m "$(cat <<'EOF'
docs(adr): decide which parts of Refine a Claude Code mod takes over

Weighs a write tool, an approval pane, a tool.call guard, a
session.start prompt and a status line against Refine as it is, and
records keep or move for each piece with sources.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: File a task per adopted piece and link it from the ADR

**Files:**
- Modify: `docs/adr/0002-*.md` (the `## Adopted` list only)

**Interfaces:**
- Consumes: Task 1's `## Adopted` list — a label, a description, and a
  `<uuid8>` marker per line.
- Produces: one pending `+niri_tasks` task per adopted piece; the ADR with
  each `<uuid8>` replaced.

If Task 1's Adopted list says `None`, skip to Step 5 and record that no
task was filed.

- [ ] **Step 1: Check the tag tasks will land on**

```bash
niritasks tag --session
```

Expected: `niri_tasks`. If it prints anything else, or fails because this
shell is not in a herdr pane, file each task in Step 2 as
`task add +niri_tasks -- "<description>"` instead.

- [ ] **Step 2: File each task**

For each Adopted line, with the exact description written there:

```bash
niritasks task add 'feat: Approve a refine from a pane, not a question'
```

Then find its uuid:

```bash
task +niri_tasks status:pending rc.json.array=on export \
  | python3 -c 'import json,sys; [print(t["uuid"], t["description"]) for t in json.load(sys.stdin)]' \
  | grep -F 'Approve a refine from a pane'
```

- [ ] **Step 3: Give each task its notes**

One `niritasks task note` per note, in the order the `refine-task` skill
uses, so the task reads like a refined one (do not add `+planned`; it has
not been through Refine):

```bash
niritasks task note <uuid> 'Goal: <the gain from the ADR, one line>'
niritasks task note <uuid> 'Context: Decided in docs/adr/0002-<slug>.md, candidate <A-E>'
niritasks task note <uuid> 'Done when: <what the ADR says the piece replaces is gone and the replacement is tested>'
```

Each `<…>` comes from that candidate's subsection in the ADR.

- [ ] **Step 4: Verify the tasks**

```bash
task +niri_tasks status:pending rc.json.array=on export \
  | python3 -c 'import json,sys; [print(t["uuid"][:8], t["description"], len(t.get("annotations",[]))) for t in json.load(sys.stdin)]'
```

Expected: one line per adopted piece, each description starting with a
lowercase type and a colon, each with 3 notes.

- [ ] **Step 5: Link them from the ADR and commit**

Replace each `<uuid8>` in the ADR's `## Adopted` list with the task's first
eight uuid characters, then:

```bash
grep -n "<uuid8>" docs/adr/0002-*.md   # expect no output
git add docs/adr/0002-*.md
git commit -m "$(cat <<'EOF'
docs(adr): link the tasks filed for Refine's mod pieces

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

If nothing was adopted, there is no commit here; say so in the hand-off.

- [ ] **Step 6: Hand off**

Tell the user the verdict per candidate in one line each, the tasks filed
(uuid8 and description), and that the branch is ready for the
`finish-worktree` skill.
