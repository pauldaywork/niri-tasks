# Conventional Commits Types on Tasks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every task description starts with a Conventional Commits type (`feat:`, `fix:` …) so the kind of change shows at a glance.

**Architecture:** No niritasks code changes. The rule lives in the four places that tell Claude how to write a task — the refine-task skill, the workspace-tasks skill, `claude/rules/niri-tasks.md` (loaded by every Claude session) and `llms.txt` — plus one regression test that pins taskwarrior keeping `feat:` and friends as description text when `niritasks task add` word-splits. Then every existing pending task is backfilled, with the user approving the new descriptions first.

**Tech Stack:** Markdown skill/rules files, Rust integration test (`cargo test --test write_path`), Taskwarrior 2.6.

**Spec:** Taskwarrior task `82830753-1188-4859-8f35-bc34f6c546ce` — read it with `task rc.json.array=on 82830753 export`. Its description and notes are the spec.

## Global Constraints

- Types are exactly the Conventional Commits set: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style`, `revert`. No `bug:` — a bug is `fix:`.
- Format is `<type>: <imperative description>`, type lowercase. The prefix counts toward the ~50-character card width.
- No enforcement in niritasks code: the task box stays terse. refine-task adds the prefix; Claude adds it when it files a task.
- Task descriptions only. This repo's git commit messages keep their plain-sentence, no-prefix style — so every commit in this plan is a plain sentence, no `feat:`.
- Out of scope: Conventional Commits for git commits; branch naming (`task/feat-…` slugs are fine); any UI display or parsing of the type.
- Always address a task by uuid (or its first 8 characters), never its number.
- Change a description with `niritasks task edit <uuid> "<text>"` (one quoted argument, kept literal), never `task <uuid> modify`.
- End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

Already verified while planning: in a sandboxed taskwarrior 2.6.2, `task add <type>: do the thing` keeps `<type>: do the thing` as the description for all 11 types. So the test in Task 1 passes on first run — it is a guard against a future taskwarrior or a UDA starting to eat the prefix, not a red-green cycle.

### Checked before planning: nothing that reads a description breaks

Every use of `.description` in `src/` was traced, and the risky ones run for real:

- **Identity is by uuid, never description:** herdr agent names (`task-<uuid8>`, `work-<uuid8>`), the daemon's socket protocol (`edit <uuid>`), finding a task's worktree (`find_task_worktree` matches `-<uuid8>`), `finish-worktree`'s task lookup.
- **Branch and worktree names:** `work::slug` folds `:` into `-`, so `feat: Give every task…` becomes `task/feat-give-every-task-a-conventional-82830753`. This very task was started that way, and its worktree and herdr workspace came up correctly.
- **Real binary against a scratch database:** `niritasks task add feat: … due:…` kept `feat:` and still set the due date; `task list --dry-run`, `get-text`, `edit` (`fix: … due:friday` stayed literal), `note`, `status active`, `active` and `status completed` all behaved.
- **Speak:** Haiku's rewrite (`src/speak_prompt.md`) of three typed descriptions dropped or paraphrased the type; none read "feat colon" aloud.
- **Cosmetic only:** herdr tab labels read `Refine: feat: …` / `Start: feat: …`, and the prefix takes ~6 of their 30 characters; the add notification reads `Added to +tag: feat: …`.

## File Structure

| File | Change |
|---|---|
| `tests/write_path.rs` | New section in `write_path_lifecycle`: add keeps each type prefix in the description |
| `.claude/skills/refine-task/SKILL.md` | Step 4 item 1 requires the type; example payload gets one |
| `.claude/skills/workspace-tasks/SKILL.md` | Phase 5 `finding=` example carries a type, plus one sentence on choosing it |
| `claude/rules/niri-tasks.md` | New `## Filing a task` section |
| `llms.txt` | New rule 8 in **Rules agents get wrong**; rule 1's `task add` example shows the type |

---

### Task 1: Pin that `niritasks task add` keeps a type prefix

**Files:**
- Modify: `tests/write_path.rs` — insert after the `// ---- add parses taskwarrior attributes ----` section (it ends with `assert_eq!(raw(&with_attrs.uuid, "priority"), "H");`, around line 98) and before `// ---- add with notes hands back the uuid ----`.

**Interfaces:**
- Consumes: `task::add(tag: &str, description_args: &[&str]) -> Result<Option<String>>` (returns the new task's full uuid), `text::add_args(&str) -> Vec<&str>` (whitespace split, as `niritasks task add` does), `task::get(uuid: &str) -> Result<Option<Task>>` with `Task.description: String`.
- Produces: nothing later tasks call.

- [ ] **Step 1: Write the test section**

Insert, using a tag of its own so the later counts on `TAG` are unchanged:

```rust
    // ---- add keeps a Conventional Commits type as text --------------------
    // Every task description starts with a type, "feat: …" or "fix: …". Split
    // into words, "feat:" is a word of its own in attribute shape, so pin that
    // taskwarrior leaves each type in the description instead of eating it.
    for kind in [
        "feat", "fix", "docs", "refactor", "perf", "test", "build", "ci", "chore", "style", "revert",
    ] {
        let typed = format!("{kind}: keep the type");
        let uuid = task::add("typesandbox", &text::add_args(&typed))
            .expect("add with a type")
            .expect("a description was given, so a task was made");
        assert_eq!(
            task::get(&uuid).expect("get").expect("exists").description,
            typed,
            "{kind}: must stay in the description, not be read as an attribute"
        );
    }
```

- [ ] **Step 2: Run it**

Run: `cargo test --test write_path`
Expected: PASS (`test write_path_lifecycle ... ok`). It passes first time — see Global Constraints.

- [ ] **Step 3: Check the test can fail**

Temporarily replace `&text::add_args(&typed)` with `&text::add_args("keep the type")`, so the task is added without its type but still compared against `typed`. Run `cargo test --test write_path`; expected: FAIL with `feat: must stay in the description`. Revert the change and run again; expected: PASS.

- [ ] **Step 4: Run the whole cargo suite and clippy**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 5: Commit**

```bash
git add tests/write_path.rs
git commit -m "Test that niritasks task add keeps a Conventional Commits type in the description

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Require the type wherever Claude writes a task

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md` (step 4, item 1 and the example JSON)
- Modify: `.claude/skills/workspace-tasks/SKILL.md:213-214` (Phase 5 example)
- Modify: `claude/rules/niri-tasks.md` (append a section)
- Modify: `llms.txt:9` and the end of the numbered **Rules agents get wrong** list (after rule 7, line 15)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: the wording Task 3 follows when it proposes new descriptions.

- [ ] **Step 1: refine-task step 4**

In `.claude/skills/refine-task/SKILL.md`, replace:

```markdown
1. **New description** — one line, about 50 characters or fewer: the card and
   picker row show one line at that width. Imperative, specific.
```

with:

```markdown
1. **New description** — one line, about 50 characters or fewer: the card and
   picker row show one line at that width. It starts with a Conventional
   Commits type, lowercase, a colon and a space — `feat`, `fix`, `docs`,
   `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style` or `revert` —
   and the type counts toward the 50. A bug is `fix:`; there is no `bug:`.
   Keep the type the task already has unless it is wrong. After it,
   imperative and specific: `fix: Keep the daemon to one task box`.
```

And in the example payload a few lines below, replace:

```json
  "description": "Show each Claude agent's topic in herdr's sidebar",
```

with:

```json
  "description": "feat: Show each Claude agent's topic in herdr's sidebar",
```

- [ ] **Step 2: workspace-tasks Phase 5**

In `.claude/skills/workspace-tasks/SKILL.md`, replace:

```bash
finding="the finding, as one actionable line"
```

with:

```bash
finding="fix: the finding, as one actionable line"
```

and directly after the closing fence of that code block, before `Using the tag **captured in Phase 1**`, add this paragraph:

```markdown
Start the line with its Conventional Commits type — `fix:` for a bug, `test:`
for a missing test, `docs:`, `refactor:`, `perf:`, `chore:` and so on, never
`bug:` — as every task description does (see `claude/rules/niri-tasks.md`).
```

- [ ] **Step 3: the rules file every session loads**

Append to `claude/rules/niri-tasks.md` (after the existing `## Finishing a worktree branch` section, one blank line between):

```markdown
## Filing a task

Start every Taskwarrior task description you write — `niritasks task add`,
`task add`, an edit, a refine — with a Conventional Commits type, lowercase,
then a colon and a space: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`,
`build`, `ci`, `chore`, `style` or `revert`. Then the description, imperative:
`fix: Keep the daemon to one task box`. A bug is `fix:`; there is no `bug:`.
The type counts toward the ~50 characters a task card shows. This is for task
descriptions only: git commit messages keep their repo's own style.
```

- [ ] **Step 4: llms.txt**

In `llms.txt` rule 1, replace the tail:

```markdown
To file a task from a terminal, tag it yourself: `task add "+$(niritasks tag --session)" -- "<description>"`.
```

with:

```markdown
To file a task from a terminal, tag it yourself: `task add "+$(niritasks tag --session)" -- "<type>: <description>"` (see rule 8).
```

After rule 7 (the line starting `7. **\`niritasks daemon\``), add:

```markdown
8. **Start every task description with a Conventional Commits type**: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style` or `revert`, lowercase, then `: ` and an imperative description, as in `fix: Keep the daemon to one task box`. A bug is `fix:`; there is no `bug:`. The type counts toward the ~50 characters a task card shows. niritasks does not add or check it, so it is on you when you add, edit or refine a task. Git commit messages are not task descriptions and keep this repo's plain-sentence style.
```

- [ ] **Step 5: Run the llms.txt tests**

Run: `cargo test llms_txt`
Expected: PASS (`llms_txt_runs_every_subcommand_and_flag` and `llms_txt_has_the_llmstxt_shape`). They check that llms.txt names every command and keeps its llmstxt shape; the new rule adds no heading.

- [ ] **Step 6: Check a fresh session files with a type**

The live `~/.claude/rules/niri-tasks.md` links to the main checkout, not this worktree, so hand the worktree's rules to a fresh session directly. Run from the worktree:

```bash
claude -p --append-system-prompt "$(cat claude/rules/niri-tasks.md)" \
  "Add a task to this workspace: the task panel flickers when a task completes. Do not run anything — print only the exact shell command you would run."
```

Expected: the printed command's description starts with `fix:` (e.g. `niritasks task add fix: Stop the panel flickering when a task completes` or a `task add … -- "fix: …"`). If it has no type, tighten the rules wording in Step 3 and run again. Repeat once with a feature ("add a keyboard shortcut to open the panel") and expect `feat:`.

- [ ] **Step 7: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md .claude/skills/workspace-tasks/SKILL.md claude/rules/niri-tasks.md llms.txt
git commit -m "Start every task description with a Conventional Commits type in refine-task, workspace-tasks, the rules and llms.txt

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Backfill every pending task's type

Runs in the main session (needs the user's approval mid-task), not a subagent. Changes the user's live task database, not the repo, so there is nothing to commit.

**Files:** none. Writes to `~/.task` through `niritasks task edit`.

**Interfaces:**
- Consumes: the wording from Task 2's rules (types, format, `fix:` for bugs).
- Produces: nothing.

- [ ] **Step 1: List the pending tasks with no type**

```bash
task rc.json.array=on status:pending export | python3 -c '
import json, re, sys
pat = re.compile(r"^(feat|fix|docs|refactor|perf|test|build|ci|chore|style|revert)(\([^)]*\))?!?: ")
for t in json.load(sys.stdin):
    if not pat.match(t["description"]):
        print(t["uuid"], ",".join(t.get("tags", [])), t["description"], sep="\t")
'
```

This is every workspace, not just `niri_tasks`: the spec says every pending task. At planning time there were 23. Read each one's notes (`task rc.json.array=on <uuid> export`) where the description alone does not say whether it is a bug, a feature or a chore.

- [ ] **Step 2: Propose the new descriptions**

Show a table: uuid8, tag, old description, new description. For each new one:
- prepend the right type; keep the existing wording otherwise, only capitalising its first word after the type (the card style: `fix: Keep the daemon…`);
- a lowercase, long ad-hoc description (e.g. `use remaining fable credits to do an analysis …`) keeps its text — rewording is refine-task's job, not this backfill's;
- do not change any task that already has a type.

Then ask with AskUserQuestion, one question, "Write these descriptions?": **Write them all** / **Change some**. On **Change some**, take the edits, show the table again and ask again.

- [ ] **Step 3: Write them**

For each approved row, re-read the task first and skip (and report) any whose description changed since Step 1. Otherwise:

```bash
niritasks task edit <uuid> "<new description>"
```

One quoted argument, so `due:`-shaped text stays literal.

- [ ] **Step 4: Verify**

Run the Step 1 command again. Expected: no output. Report how many tasks were changed and any skipped.

---

## Done when

- `cargo test` passes, including the type-prefix section of `write_path_lifecycle`.
- refine-task, workspace-tasks, `claude/rules/niri-tasks.md` and `llms.txt` all require the prefix.
- A fresh Claude session asked to add a task files it with a type (Task 2 Step 6).
- Task 3 Step 4's check prints nothing.

Then land the branch with the `finish-worktree` skill and mark the task done with `niritasks task status 82830753 completed`.
