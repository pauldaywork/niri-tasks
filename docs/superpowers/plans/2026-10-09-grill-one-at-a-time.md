# Ask Grilling Questions One at a Time Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A card's Grill me asks each grilling round as one tabbed AskUserQuestion dialog, one question per tab, and the answers arrive together, instead of the `grilling` skill's numbered markdown list.

**Architecture:** One edit to the `grill` bullet in step 3 of `.claude/skills/refine-task/SKILL.md`. It still invokes the upstream `grilling` skill for the method (design tree, rounds, frontier, recommended answers), and overrides only how a round is put to the user: one AskUserQuestion call, at most 4 questions, recommended answer first and marked "(Recommended)". Step 4's "no AskUserQuestion" is narrowed so it plainly covers approving the proposal only, not grilling.

**Tech Stack:** A Claude Code skill (Markdown), Claude Code's AskUserQuestion tool, the `mattpocock-skills` plugin's `grilling` skill.

**Spec:** Taskwarrior task `2bfa68c8-f63b-4cfd-9f27-71f59275942c`. Read it with `task rc.json.array=on 2bfa68c8-f63b-4cfd-9f27-71f59275942c export`; its description and annotations are the spec.

## Global Constraints

- Change `.claude/skills/refine-task/SKILL.md` only. Do not fork, copy or edit the upstream `grilling` skill (`~/.claude/plugins/.../skills/productivity/grilling/SKILL.md`): plugin files are overwritten when the marketplace updates, and upstream fixes must keep coming through.
- Each round is exactly one AskUserQuestion call. The recommended answer is the first option, its label ending in "(Recommended)".
- A round never has more than 4 questions; extra frontier questions wait for the next round.
- Leave quick mode's "at most three questions, in one round" as it is.
- Out of scope: `/grill-me` or `/grilling` typed outside a refine session, the quick Refine flow, and the upstream plugin.
- Commits: Conventional Commits, type `feat`, scope `refine`, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Background for the implementer

- **How Grill me reaches the skill.** The card's Grill me button runs `niritasks task refine <uuid> --grill`, which opens a "Refine: …" tab in the workspace's herdr session and prompts Claude with `/refine-task <uuid> grill` (`src/refine.rs:31-32`). `install.sh:87` symlinks the repo's `SKILL.md` into `~/.claude/skills/refine-task/`, so an edit in the repo is live in the next refine session with no install step — as long as `install.sh` has been run once on this machine.
- **What `grilling` says now.** It asks "the whole frontier in one round: number each question and give your recommended answer", in a fixed `❓ **Q1** …` / `➡️ <recommended>` markdown format, then waits for answers. Everything else in it — the design tree, recomputing the frontier after each round, a question that hangs on another open one waits for a later round, looking facts up instead of asking, stopping when the frontier is empty — still applies unchanged.
- **AskUserQuestion's shape.** 1–4 questions per call, each shown as its own tab; each question has a `header` chip of at most 12 characters, 2–4 options (each a `label` and a `description`), and an automatic "Other" for free text; one submit returns every answer at once. An option may carry a `preview` (monospace box) for comparing code or layouts.
- **Nothing blocks the tool in a refine session.** `session_settings` (`src/refine.rs:100-124`) only allows WebSearch, WebFetch and `Bash(task *)` unasked and denies reads of credentials; AskUserQuestion needs no permission. The refine mod's own tests (`claude/refine-mod/tests/refine.test.ts:54-61`) stub AskUserQuestion for the write tool's approval dialog; that is a separate path the skill never calls.
- **Why step 4 needs a touch.** Step 4 says "Do not ask the user to approve it yourself — no AskUserQuestion". Read on its own, "no AskUserQuestion" could make a model shy of using the tool in step 3. Narrow it to approving the proposal.
- **No automated test exists for skill wording.** The `cargo test` suite and the mod's tests check the launch and the write tool, not what Claude asks. The check is a real Grill me run (Task 1, Step 4).

## File Structure

- `.claude/skills/refine-task/SKILL.md`: step 3's `grill` bullet gains the round format; step 4's "no AskUserQuestion" is narrowed to approval.

---

### Task 1: Ask each grilling round through AskUserQuestion

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md:63-64` (the `grill` bullet) and `:68-70` (step 4's opening)

**Interfaces:**
- Consumes: the `grilling` skill (upstream, unchanged); Claude Code's AskUserQuestion tool.
- Produces: nothing other tasks rely on.

- [ ] **Step 1: Confirm the current text**

Run: `sed -n 56,72p .claude/skills/refine-task/SKILL.md`
Expected: the quick bullet (lines 60–62), then

```markdown
- **`grill`:** invoke the `grilling` skill with the task (description and notes)
  as the plan to grill, and follow it until its frontier is empty.
```

and step 4 opening with "Show the proposal in your reply. Do not ask the user to approve it yourself — no AskUserQuestion: the write tool in step 5 shows them exactly what it will write and asks them." If the text differs, stop and re-read the file before editing.

- [ ] **Step 2: Replace the `grill` bullet**

Replace those two lines with:

```markdown
- **`grill`:** invoke the `grilling` skill with the task (description and notes)
  as the plan to grill, and follow it until its frontier is empty — except for
  how a round is put to the user. Instead of its numbered markdown list, ask
  each round with **one AskUserQuestion call**, one question per tab, so the
  user takes them one at a time and sends all the answers together:
  - At most 4 questions a round. If the frontier holds more, ask the 4 that
    most other decisions hang on, and leave the rest for the next round.
  - Each question's text carries what the markdown body would have: the
    decision and enough context to make it. Its `header` names the decision
    in 12 characters or fewer.
  - 2–4 options per question. Your recommended answer is the **first** option,
    its label ending in "(Recommended)", and its description says why. An open
    question still gets 2–4 likely answers; the user can always pick "Other"
    and type their own. Use an option's `preview` when comparing code,
    layouts or config.
  - Don't repeat the round as text in your reply; the dialog is the round.

  Everything else in `grilling` holds: rounds, recomputing the frontier after
  each one, looking up facts rather than asking, and stopping when the
  frontier is empty.
```

- [ ] **Step 3: Narrow step 4's ban to approval**

In step 4, replace:

```markdown
Show the proposal in your reply. Do not ask the user to approve it yourself —
no AskUserQuestion: the write tool in step 5 shows them exactly what it will
write and asks them.
```

with:

```markdown
Show the proposal in your reply. Do not ask the user to approve it yourself —
no AskUserQuestion for approval: the write tool in step 5 shows them exactly
what it will write and asks them.
```

Run: `git diff --stat` — Expected: `.claude/skills/refine-task/SKILL.md` only.

- [ ] **Step 4: Try it on a real card**

The skill is symlinked into `~/.claude/skills`, so the edit is live. Check the link points at this worktree's file, not the main checkout's:

Run: `readlink -f ~/.claude/skills/refine-task/SKILL.md`

If it points at the main checkout, check this worktree's copy instead by running a Claude Code session in the worktree, where the project's `.claude/skills/refine-task/SKILL.md` takes over. Add a throwaway task with something to decide, for example:

```bash
niritasks task add "feat: Colour the clock by battery level"
```

and Grill me on its card (or `niritasks task refine <uuid> --grill`; in a worktree session, `/refine-task <uuid> grill`). Check, and note each in the report:

1. The first round arrives as one AskUserQuestion dialog with 1–4 tabs, not a numbered markdown list.
2. Every question's first option ends in "(Recommended)".
3. One submit sends all the answers, and the next round builds on them (a frontier that needed more than 4 questions carries the rest over).
4. When the frontier is empty, step 4's proposal is printed and step 5's write tool asks "Write this to the task?" as before. Pick **Change something** or dismiss it so nothing is written, then delete the throwaway task with `niritasks task status <uuid> deleted --yes` or the card's Remove.

Expected: all four hold. If a round comes as a markdown list, tighten the bullet's wording and try again.

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md
git commit -m "$(cat <<'EOF'
feat(refine): ask each grilling round as a tabbed dialog

Grill me's rounds came as one long numbered list, which is a lot to
take in at once. refine-task now has each round asked through
AskUserQuestion, one question per tab with the recommendation first,
and the answers sent together. The upstream grilling skill is still
what drives the rounds, so its fixes keep coming through.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
