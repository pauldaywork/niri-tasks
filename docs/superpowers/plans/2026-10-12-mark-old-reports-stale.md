# Mark Old Reports Stale Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After a re-refine that changes the plan, the card's Open report must not open a report of the old plan.

**Architecture:** `withReportNotes` in `claude/refine-mod/hooks/report.ts` gains the plan as read (`expected`). When the description or the plan's own notes differ from it (every `Report…` note ignored on both sides), each `Report: <path>` note kept from an earlier session is rewritten in place as `Report (earlier draft): <path>`. The Rust side already skips earlier-draft notes in `Task::report_path()` (`src/task.rs:165`), so the card stops offering Open report with no Rust change.

**Tech Stack:** TypeScript Claude Code mod (`claude/refine-mod`), tests through `claude-code/testing`, run with `claude plugin test claude/refine-mod` from the repository root.

**Spec:** Taskwarrior task `cc01aac6-6170-44af-a7d2-8c17e118f3dd` (`task rc.json.array=on cc01aac6-6170-44af-a7d2-8c17e118f3dd export`): its description and notes are the spec.

## Global Constraints

- Relabel, not fingerprint: no new stored state. The only input is `expected`, which `write_task_plan` already receives.
- The plan has changed when the description or the plan's own notes differ from `expected`, ignoring every `Report…` note on both sides. If the plan is unchanged, earlier `Report:` notes stay as they are.
- A relabelled note keeps its path and its place among the notes: `Report: <path>` becomes `Report (earlier draft): <path>`.
- Report mode (Generate report, `withReportNote`) is left alone.
- Out of scope: spotting a plan edited by hand outside a refine; fingerprints stored beside report notes; deleting old report files.
- Done when: a re-refine that changes the plan with no new report writes the old note as `Report (earlier draft): <path>`, the card no longer shows Open report for it, an unchanged re-refine keeps `Report: <path>`, and the mod's tests pass.
- Commits: Conventional Commits, scope `refine`, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Comments match the mod's voice: plain sentences saying what a thing is and why, no "we".

## Decisions this plan makes

- **This session's reports are still judged against the notes before relabelling.** A report made this session records the plan shown, which holds the earlier session's note as `Report: <path>`. If the comparison used the relabelled notes, a report of the *new* plan would wrongly read as an earlier draft. So `now` (what each made report is compared with) is built from the plan's own notes *before* the relabel. A test pins this.
- **`expected` is a required third parameter**, not read off a `PlanInput`: `withReportNotes(made, plan, expected)`. Its only caller is the `write_task_plan` handler.
- **A report note is one starting `Report: ` or `Report (earlier draft): `**, not any note starting `Report`, so a plan note that happens to begin with "Report" still counts as part of the plan when deciding whether it changed. `ownNotes` keeps its existing looser test: it also requires the path of a report made this session.

## Files

| File | Change | Responsibility |
|---|---|---|
| `claude/refine-mod/hooks/report.ts` | Modify | `withReportNotes` takes `expected` and relabels earlier-session notes on a changed plan |
| `claude/refine-mod/hooks/refine.ts` | Modify | pass `input.expected`; update the comments |
| `claude/refine-mod/tests/report.test.ts` | Modify | new `withReportNotes` cases; existing calls gain `expected` |
| `claude/refine-mod/tests/refine.test.ts` | Modify | an end-to-end write showing the relabelled note logged and imported |
| `.claude/skills/refine-task/SKILL.md` | Modify | say the tool relabels an earlier session's `Report:` note on a changed plan |

---

### Task 1: Relabel an earlier session's report note when the plan changes

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts` (the block from `export type MadeReport` to the end of `withReportNotes`, around lines 360–395)
- Modify: `claude/refine-mod/hooks/refine.ts:321-325`
- Test: `claude/refine-mod/tests/report.test.ts:191-243`, `claude/refine-mod/tests/refine.test.ts` (after the test `'a report made before the plan changed is linked as an earlier draft'`, around line 387)

**Interfaces:**
- Consumes: `Plan = { description: string; notes: string[] }` from `hooks/plan.ts`; `MadeReport = { path: string; plan: Plan }` from `hooks/report.ts`.
- Produces: `withReportNotes(made: readonly MadeReport[], plan: Plan, expected: Plan): string[]`.

- [ ] **Step 1: Update the existing `withReportNotes` tests and add the new ones**

In `claude/refine-mod/tests/report.test.ts`, replace the whole `describe('withReportNotes', …)` block (lines 191–243) with:

```ts
describe('withReportNotes', () => {
  const plan = { description: 'feat: A', notes: ['Goal: x'] }

  test("adds a note linking a report made for this very plan, after the plan's own", () => {
    expect(withReportNotes([{ path: '/r/a.html', plan }], plan, plan)).toEqual(['Goal: x', 'Report: /r/a.html'])
  })

  test('marks a report made before the plan changed', () => {
    expect(withReportNotes([{ path: '/r/a.html', plan }], { ...plan, notes: ['Goal: y'] }, plan)).toEqual([
      'Goal: y',
      'Report (earlier draft): /r/a.html',
    ])
  })

  test('relabels a kept note for a report of this session once the plan is revised', () => {
    const revised = { ...plan, notes: ['Goal: y', 'Report: /r/a.html'] }
    expect(withReportNotes([{ path: '/r/a.html', plan }], revised, plan)).toEqual([
      'Goal: y',
      'Report (earlier draft): /r/a.html',
    ])
  })

  test('keeps a kept note for an unrevised plan once, as it was', () => {
    const kept = { ...plan, notes: ['Goal: x', 'Report: /r/a.html'] }
    expect(withReportNotes([{ path: '/r/a.html', plan }], kept, plan)).toEqual(['Goal: x', 'Report: /r/a.html'])
    // A second report on the same plan: the first's note does not make it differ.
    const made = [
      { path: '/r/a.html', plan },
      { path: '/r/b.html', plan: kept },
    ]
    expect(withReportNotes(made, kept, kept)).toEqual(['Goal: x', 'Report: /r/a.html', 'Report: /r/b.html'])
  })

  test("keeps an earlier session's report note in its place on an unchanged plan", () => {
    const notes = ['Goal: x', 'Report: /old/z.html', 'Done when: y']
    const read = { ...plan, notes }
    expect(withReportNotes([{ path: '/r/a.html', plan }], read, read)).toEqual([
      ...notes,
      'Report (earlier draft): /r/a.html',
    ])
  })

  test("relabels an earlier session's report note in its place when the plan changed", () => {
    const read = { ...plan, notes: ['Goal: x', 'Report: /old/z.html', 'Done when: y'] }
    const revised = { ...plan, notes: ['Goal: x', 'Report: /old/z.html', 'Done when: z'] }
    expect(withReportNotes([], revised, read)).toEqual([
      'Goal: x',
      'Report (earlier draft): /old/z.html',
      'Done when: z',
    ])
    // A new description alone is a changed plan too.
    expect(withReportNotes([], { ...read, description: 'feat: B' }, read)).toEqual([
      'Goal: x',
      'Report (earlier draft): /old/z.html',
      'Done when: y',
    ])
  })

  test('report notes alone do not make the plan differ', () => {
    // The model dropped one report note and kept another: the plan is the same.
    const read = { ...plan, notes: ['Goal: x', 'Report (earlier draft): /old/y.html', 'Report: /old/z.html'] }
    const kept = { ...plan, notes: ['Goal: x', 'Report: /old/z.html'] }
    expect(withReportNotes([], kept, read)).toEqual(['Goal: x', 'Report: /old/z.html'])
  })

  test("leaves an earlier session's earlier-draft note as it is", () => {
    const read = { ...plan, notes: ['Goal: x', 'Report (earlier draft): /old/y.html'] }
    const revised = { ...plan, notes: ['Goal: y', 'Report (earlier draft): /old/y.html'] }
    expect(withReportNotes([], revised, read)).toEqual(['Goal: y', 'Report (earlier draft): /old/y.html'])
  })

  test('a report of the changed plan stays the report while the old one is relabelled', () => {
    // The plan shown for /r/a.html held the old note unrelabelled, as the model kept it.
    const read = { ...plan, notes: ['Goal: x', 'Report: /old/z.html'] }
    const revised = { ...plan, notes: ['Goal: y', 'Report: /old/z.html'] }
    expect(withReportNotes([{ path: '/r/a.html', plan: revised }], revised, read)).toEqual([
      'Goal: y',
      'Report (earlier draft): /old/z.html',
      'Report: /r/a.html',
    ])
  })

  test('links each report once, in the order made', () => {
    const made = [
      { path: '/r/a.html', plan: { ...plan, description: 'feat: Old' } },
      { path: '/r/b.html', plan },
    ]
    expect(withReportNotes(made, plan, plan)).toEqual([
      'Goal: x',
      'Report (earlier draft): /r/a.html',
      'Report: /r/b.html',
    ])
  })

  test("none made: the plan's own notes", () => {
    expect(withReportNotes([], plan, plan)).toEqual(['Goal: x'])
  })
})
```

- [ ] **Step 2: Add the end-to-end test**

In `claude/refine-mod/tests/refine.test.ts`, directly after the test `'a report made before the plan changed is linked as an earlier draft'` (it ends around line 387), add:

```ts
// TASK refined once before, in an earlier session that made a report.
const OLD_REPORT = '/r/refine-0b8f6a52-20260101-000000.html'
const REPORTED = {
  ...PLANNED,
  annotations: [
    { entry: '20260101T000001Z', description: 'first note' },
    { entry: '20260101T000002Z', description: `Report: ${OLD_REPORT}` },
  ],
}
const REREFINE = {
  ...CALL,
  expected: { description: 'feat: Old words', notes: ['first note', `Report: ${OLD_REPORT}`] },
}

test("a re-refine that changes the plan relabels an earlier session's report", { options: { uuid: UUID } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, WRITE, [REPORTED])
  await $.session.start(START)
  const written = await $.tool.call({ ...REREFINE, notes: ['step one', `Report: ${OLD_REPORT}`] })
  expect(written.deny).toBeUndefined()
  expect(seen).toContain(`log: Note 2: Report (earlier draft): ${OLD_REPORT}`)
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.map((a: { description: string }) => a.description)).toEqual([
    'step one',
    `Report (earlier draft): ${OLD_REPORT}`,
  ])
})

test("a re-refine that keeps the plan keeps an earlier session's report", { options: { uuid: UUID } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, WRITE, [REPORTED])
  await $.session.start(START)
  const written = await $.tool.call({ ...REREFINE, ...REREFINE.expected })
  expect(written.deny).toBeUndefined()
  expect(seen).toContain(`log: Note 2: Report: ${OLD_REPORT}`)
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.at(-1).description).toBe(`Report: ${OLD_REPORT}`)
})
```

`world`'s export answers `REPORTED` both times the handler reads the task, and its notes equal `REREFINE.expected`, so `current` lets the write through.

- [ ] **Step 3: Run the tests to see them fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL. In `report.test.ts`, `"relabels an earlier session's report note in its place when the plan changed"` and `'a report of the changed plan stays the report while the old one is relabelled'` get `Report: /old/z.html` where they expect `Report (earlier draft): /old/z.html`. In `refine.test.ts`, `"a re-refine that changes the plan relabels an earlier session's report"` fails the same way. The other cases may pass already, since the current function ignores the third argument.

- [ ] **Step 4: Implement the relabel in `report.ts`**

In `claude/refine-mod/hooks/report.ts`, replace everything from `const samePlan = …` to the end of `withReportNotes` with:

```ts
const samePlan = (a: Plan, b: Plan): boolean =>
  a.description === b.description && a.notes.length === b.notes.length && a.notes.every((n, i) => n === b.notes[i])

// What a note linking a report starts with, as the tool writes it: the
// report of the plan, or one of a plan since changed. The card opens only
// the first.
const REPORT_NOTE = 'Report: '
const EARLIER_DRAFT_NOTE = 'Report (earlier draft): '

const isReportNote = (note: string): boolean => note.startsWith(REPORT_NOTE) || note.startsWith(EARLIER_DRAFT_NOTE)

// The plan with its report notes dropped: what a refine changes.
const planOnly = (plan: Plan): Plan => ({ description: plan.description, notes: plan.notes.filter(n => !isReportNote(n)) })

// The plan's own notes: those that link one of the session's reports are
// the mod's to write, so they are dropped here and written afresh. A note
// linking a report from an earlier session is the plan's own.
const ownNotes = (made: readonly MadeReport[], notes: readonly string[]): string[] =>
  notes.filter(note => !(note.startsWith('Report') && made.some(m => note.endsWith(m.path))))

// The notes the task is written with: the plan's own, then one linking each
// report made this session, in the order made, marked as an earlier draft
// when the plan changed since. A report note the model kept is relabelled,
// never doubled. When the plan differs from `expected`, the task as read, a
// `Report:` note from an earlier session reports the old plan, so it is
// marked as an earlier draft in its place and the card no longer opens it.
export const withReportNotes = (made: readonly MadeReport[], plan: Plan, expected: Plan): string[] => {
  const own = ownNotes(made, plan.notes)
  const changed = !samePlan(planOnly(expected), planOnly(plan))
  // Each made report is held to the notes before any relabel: the plan it
  // was shown kept the earlier session's note as the model did.
  const now = { description: plan.description, notes: own }
  return [
    ...own.map(note =>
      changed && note.startsWith(REPORT_NOTE) ? EARLIER_DRAFT_NOTE + note.slice(REPORT_NOTE.length) : note,
    ),
    ...made.map(m =>
      samePlan({ description: m.plan.description, notes: ownNotes(made, m.plan.notes) }, now)
        ? `${REPORT_NOTE}${m.path}`
        : `${EARLIER_DRAFT_NOTE}${m.path}`,
    ),
  ]
}
```

- [ ] **Step 5: Pass `input.expected` from the handler**

In `claude/refine-mod/hooks/refine.ts`, replace lines 321–325:

```ts
    // The plan as the tool will write it, from its own arguments, not from
    // what the model printed, with a note linking each report made this
    // session; refused whole before a line could be cut.
    const plan = { description: input.description, notes: withReportNotes(made, input) }
```

with:

```ts
    // The plan as the tool will write it, from its own arguments, not from
    // what the model printed, with a note linking each report made this
    // session, and an earlier session's report marked as an earlier draft
    // when the plan differs from the task as read; refused whole before a
    // line could be cut.
    const plan = { description: input.description, notes: withReportNotes(made, input, input.expected) }
```

Then check the comment above `const made: MadeReport[] = []` (around line 291) still holds; it does not mention relabelling, so it stays.

- [ ] **Step 6: Run the tests to see them pass**

Run: `claude plugin test claude/refine-mod`
Expected: PASS, every file.

Also run: `grep -n "withReportNotes(" claude/refine-mod/hooks/*.ts claude/refine-mod/tests/*.ts`
Expected: every call passes three arguments.

- [ ] **Step 7: Commit**

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/hooks/refine.ts claude/refine-mod/tests/report.test.ts claude/refine-mod/tests/refine.test.ts
git commit -m "$(cat <<'EOF'
fix(refine): mark an old report stale when the plan changes

A Report: note from an earlier session was written back unchanged by a
re-refine, so the card's Open report still opened a report of the old
plan. write_task_plan already holds the task as read, so when the
description or the plan's own notes differ from it, that note is now
relabelled Report (earlier draft): in its place. An unchanged plan
keeps it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Say so in the refine-task skill

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md:253-257` and `:264-266`

**Interfaces:**
- Consumes: the behaviour from Task 1.
- Produces: nothing code reads.

- [ ] **Step 1: Rewrite the paragraph on the tool's report notes**

In `.claude/skills/refine-task/SKILL.md`, replace:

```markdown
When the task is written, the tool adds a note linking each report made in
this session: `Report: <path>`, or `Report (earlier draft): <path>` for one
made before the plan changed. Do not add these notes yourself. The tool
relabels any `Report …` note for this session's reports, so keeping earlier
ones is always safe.
```

with:

```markdown
When the task is written, the tool adds a note linking each report made in
this session: `Report: <path>`, or `Report (earlier draft): <path>` for one
made before the plan changed. Do not add these notes yourself. If the
description or notes differ from what you read, the tool also rewrites a
`Report: <path>` note from an earlier session as
`Report (earlier draft): <path>` in its place, since that report shows the
old plan; an unchanged plan keeps it. So keeping earlier `Report …` notes
as they are is always safe.
```

- [ ] **Step 2: Rewrite the verify step's note check**

In the same file, under `## 6. Verify and report`, replace:

```markdown
Export once more and check: the description matches, the notes are exactly the
new list, followed by any `Report …` notes the tool added, `planned` is in
```

with:

```markdown
Export once more and check: the description matches, the notes are exactly the
new list (with a kept `Report:` note as `Report (earlier draft):` if the plan
changed), followed by any `Report …` notes the tool added, `planned` is in
```

- [ ] **Step 3: Check nothing else says otherwise**

Run: `grep -n "this session's reports\|relabel" .claude/skills/refine-task/*.md claude/refine-mod/hooks/*.ts`
Expected: no line still says the tool relabels only this session's report notes.

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md
git commit -m "$(cat <<'EOF'
docs(refine): say a changed plan marks an old report stale

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### After landing

The refine mod and skill run from `~/Projects/niri-tasks` once installed, never from this worktree. After `finish-worktree` lands the branch and `install.sh` puts the mod in place, check by hand on a throwaway task that already has a `Report: <path>` note: re-refine it with a changed note and choose **Write it to the task** with no new report. The note should now read `Report (earlier draft): <path>` and the card should no longer offer Open report. A second re-refine that changes nothing keeps `Report: <path>`.
