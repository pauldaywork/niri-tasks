# Report Word Grace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `show_task_report` writes a report whose lines run a word or two over their word limits, naming the overage, instead of refusing the whole report and making the model resend thousands of tokens.

**Architecture:** `hooks/report.ts` gains `WORD_GRACE = 2` beside `LIMITS`. Its `text()` check files a line one or two words over its word limit as a warning rather than an error, and `parseReport()` answers `{ report, warnings }` (or the refusal string, as now). Both `show_task_report` paths in `hooks/refine.ts` (report mode and before approval) log each warning to the transcript and add them to the "Wrote the report" result.

**Tech Stack:** TypeScript Claude Code mod (`claude/refine-mod`), tests through `claude-code/testing`, run with `claude plugin test claude/refine-mod`.

**Spec:** Taskwarrior task `81432e40-ed42-48e4-9704-3671af0f27a6` (`task rc.json.array=on 81432e40-ed42-48e4-9704-3671af0f27a6 export`): its description and notes are the spec.

## Global Constraints

- A grace of 2 words on every word limit, as one constant beside `LIMITS`; character, line and item-count limits stay strict.
- The published limits stay as they are (tool schema descriptions in `refine.ts`, `LIMITS`, `report-catalogue.md`) so the model still aims for them; the grace is not advertised. Do not edit `.claude/skills/refine-task/report-catalogue.md`, `SKILL.md` or the tool descriptions in `refine.ts`.
- Out of scope: raising the limits themselves, and loosening character, line, Mermaid or item-count limits.
- Done when: a report with a line one or two words over its word limit is written, the overage named in the tool result and the transcript log, and one 3 or more words over is still refused; `report.test.ts` covers both and the refine-mod tests and type-check pass.
- Commits: Conventional Commits, type `perf` (the task's type), scope `refine`, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Comments match the mod's voice: plain sentences saying what a thing is and why, no "we".

## Decisions this plan makes

- **Warnings ride on the errors list.** Every check in `report.ts` takes an `errors: Errors` argument; only `text()` raises warnings. Rather than thread a second argument through some forty call sites, `Errors` becomes `string[] & { warnings: string[] }`, made by `findings()`. The diff stays in `text()` and `parseReport()`.
- **A refusal lists errors only.** When a report is refused, lines within the grace are not mentioned: they would be taken, so naming them would only make the model trim them too.
- **Wording.** The warning is the same sentence a refusal would give (`needs_your_eye[0] has 31 words; at most 30`). The tool result ends with ` Taken, though over its word limits: <warnings joined by "; ">.`; the transcript gets one line per warning, `Over its word limit: <warning>`, after the `Report: <path>` line. One line each keeps every line well under `MAX_LINE`. Neither names the grace.

## Files

- Modify `claude/refine-mod/hooks/report.ts`: `WORD_GRACE`, `Errors` with warnings, `text()`, `parseReport()` answering `Parsed`, and the new `overLimits()`.
- Modify `claude/refine-mod/hooks/refine.ts`: both `show_task_report` paths read `Parsed`, log the warnings and add `overLimits()` to the result.
- Modify `claude/refine-mod/tests/report.test.ts`, `tests/markdown.test.ts`, `tests/page.test.ts`: the new answer shape, and the grace's tests.
- Modify `claude/refine-mod/tests/refine.test.ts`: the overage named in the result and log on both paths, and 3 over still refused.

---

### Task 1: The grace in `parseReport`

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts` (the `LIMITS` comment and block near line 65, `Errors`/`text()` near lines 114-135, `parseReport()` near lines 320-336)
- Modify: `claude/refine-mod/hooks/refine.ts:376-387` and `:424-427` (read the new answer shape only; no warning output yet)
- Test: `claude/refine-mod/tests/report.test.ts`, `tests/markdown.test.ts`, `tests/page.test.ts`

**Interfaces:**
- Consumes: nothing new.
- Produces, from `hooks/report.ts`:
  - `export const WORD_GRACE = 2`
  - `export type Parsed = { report: Report; warnings: string[] }`
  - `export const parseReport = (e: Record<string, unknown>): Parsed | string` — the report and the lines let through over their word limits (each `"<at> has <n> words; at most <limit>"`, in the order the fields are checked), or every refusal joined by `"; "` as today.

- [ ] **Step 1: Update `report.test.ts` for the new shape and add the grace's tests**

In `claude/refine-mod/tests/report.test.ts`:

Change the import on line 2 to also take `WORD_GRACE`:

```ts
import { LIMITS, WORD_GRACE, madeAlready, openArgv, parseReport, reportPath, reportsDir, unreportable, withReportNote, withReportNotes } from '../hooks/report'
```

Replace the first two tests of `describe('parseReport', …)` with:

```ts
  test('takes the smallest report, with the optional lists empty', () => {
    expect(parseReport(MINIMAL)).toEqual({
      report: { ...MINIMAL, unchanged: [], needs_your_eye: [], terms: [] },
      warnings: [],
    })
  })

  test('takes a plan that touches no file', () => {
    expect(parseReport({ ...MINIMAL, files: [] })).toEqual({
      report: { ...MINIMAL, files: [], unchanged: [], needs_your_eye: [], terms: [] },
      warnings: [],
    })
  })
```

Replace the test `'refuses text over its word limit, saying the count and the limit'` with (one word over is now taken; three over is refused):

```ts
  test('refuses text three words over its word limit, saying the count and the limit', () => {
    expect(parseReport({ ...MINIMAL, summary: words(38) })).toBe('summary has 38 words; at most 35')
    expect(parseReport({ ...MINIMAL, changes: [words(23)] })).toBe('changes[0] has 23 words; at most 20')
  })
```

In `describe('lenses', …)`, the first test's second line becomes:

```ts
    expect(typeof parsed !== 'string' && parsed.report.sections[0]?.kind).toBe('data-flow')
```

After the `describe('parseReport keeps the page to one layout', …)` block, add:

```ts
describe('the word grace', () => {
  test('takes a line one or two words over its word limit, and names each', () => {
    const input = {
      ...MINIMAL,
      summary: words(36),
      needs_your_eye: [words(31)],
      sections: [{ heading: words(10 + WORD_GRACE), points: ['p'] }],
    }
    expect(parseReport(input)).toEqual({
      report: { ...input, unchanged: [], terms: [] },
      warnings: [
        'summary has 36 words; at most 35',
        'needs_your_eye[0] has 31 words; at most 30',
        'sections[0].heading has 12 words; at most 10',
      ],
    })
  })

  test('refuses a line three words over, and names no line within the grace beside it', () => {
    expect(parseReport({ ...MINIMAL, summary: words(36), needs_your_eye: [words(30 + WORD_GRACE + 1)] })).toBe(
      'needs_your_eye[0] has 33 words; at most 30',
    )
  })

  test('gives no grace on characters, lines or items', () => {
    expect(parseReport({ ...MINIMAL, title: 'x'.repeat(LIMITS.titleChars + 1) })).toBe(
      `title has ${LIMITS.titleChars + 1} characters; at most ${LIMITS.titleChars}`,
    )
    expect(parseReport({ ...MINIMAL, files: [{ path: 'p'.repeat(LIMITS.files.pathChars + 1), change: 'new', why: 'w' }] })).toBe(
      `files[0].path has ${LIMITS.files.pathChars + 1} characters; at most ${LIMITS.files.pathChars}`,
    )
    expect(parseReport({ ...MINIMAL, changes: ['a', 'b', 'c', 'd'] })).toBe('changes must have 1 to 3 items, not 4')
    const long = Array.from({ length: LIMITS.mermaidLines + 1 }, () => 'a --> b').join('\n')
    expect(
      parseReport({ ...MINIMAL, sections: [{ heading: 'h', points: ['p'], look_at: 'x', diagram: { mermaid: long } }] }),
    ).toBe(`sections[0].diagram.mermaid has ${LIMITS.mermaidLines + 1} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`)
  })
})
```

The last expectation copies the existing Mermaid refusal: check its exact wording against the `diagram()` function in `hooks/report.ts` (`${at}.mermaid has ${lines} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`, with `at` = `sections[0].diagram`).

- [ ] **Step 2: Update `markdown.test.ts` and `page.test.ts` for the new shape**

In `claude/refine-mod/tests/markdown.test.ts`, after the imports, add:

```ts
// The report parseReport takes, failing the test where it is refused.
const parsed = (input: Record<string, unknown>): Report => {
  const answer = parseReport(input)
  if (typeof answer === 'string') throw new Error(answer)
  return answer.report
}
```

Then:
- `const MINIMAL = parseReport({ … }) as Report` becomes `const MINIMAL = parsed({ … })` (the object literal unchanged).
- `const EDGES = parseReport(EDGES_INPUT) as Report` becomes `const EDGES = parsed(EDGES_INPUT)`.
- Each `const report = parseReport(EXAMPLE) as Report` (three of them) becomes `const report = parsed(EXAMPLE)`.
- In `'refuses a term the twin reads back as another'`, replace the three lines from `const report = parseReport(input)` to the `twinOf(report as Report, PLAN)` expectation with:

```ts
    expect(twinOf(parsed(input), PLAN)).toBe('its Markdown twin reads back as a different report')
```

Leave `expect(typeof parseReport(EDGES_INPUT)).toBe('object')` as it is. If `EXAMPLE` is not typed as `Record<string, unknown>`, keep calling `parsed(EXAMPLE)` and widen the helper's parameter to `Parameters<typeof parseReport>[0]`; do not cast.

In `claude/refine-mod/tests/page.test.ts`, the test `'the worked example renders every part'` becomes:

```ts
test('the worked example renders every part', () => {
  const parsed = parseReport(EXAMPLE)
  if (typeof parsed === 'string') throw new Error(parsed)
  const page = reportPage(parsed.report, EXAMPLE_PLAN)
  for (const section of parsed.report.sections) expect(page).toContain(section.heading)
  // A before/after pair and six single diagrams, all left to the browser here.
  expect(page.split('<pre class="mermaid">').length - 1).toBe(8)
})
```

If `page.test.ts` then no longer uses its `import type { Report }`, leave it if other tests use it; check with `grep -n 'Report\b' claude/refine-mod/tests/page.test.ts`.

- [ ] **Step 3: Run the tests to see them fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL — `report.test.ts` cannot import `WORD_GRACE`, or (once it can) `parseReport(MINIMAL)` answers the bare report rather than `{ report, warnings }`; the grace test gets `'summary has 36 words; at most 35; …'` as a refusal.

- [ ] **Step 4: Implement the grace in `hooks/report.ts`**

Replace the comment above `LIMITS` and add the constant right after the `LIMITS` block:

```ts
// What keeps a report easy to take in: about four things held in mind at
// once, one idea per line, short plain sentences. A report over them is
// refused with every reason at once, so it is fixed in one go; a line only
// a word or two over its word limit is taken, and named, so one word does
// not cost a whole report resent.
export const LIMITS = {
  …unchanged…
} as const

// How many words a line may run over its word limit and still be taken.
// Not in the tool's descriptions or the catalogue: the model aims for the
// limits themselves.
export const WORD_GRACE = 2
```

Replace `type Errors = string[]` with:

```ts
// The problems found, refused all at once, and beside them the lines taken
// though over their word limits, which the tool names.
type Errors = string[] & { warnings: string[] }
const findings = (): Errors => Object.assign([] as string[], { warnings: [] as string[] })
```

In `text()`, replace the word-limit line

```ts
  if (limit.words !== undefined && n > limit.words) errors.push(`${at} has ${n} words; at most ${limit.words}`)
```

with

```ts
  if (limit.words !== undefined && n > limit.words) {
    const over = `${at} has ${n} words; at most ${limit.words}`
    if (n - limit.words <= WORD_GRACE) errors.warnings.push(over)
    else errors.push(over)
  }
```

Above `parseReport`, add the answer's type, and change `parseReport`'s head, its `errors` and its success line:

```ts
// A report the tool takes, and the lines in it over their word limits.
export type Parsed = { report: Report; warnings: string[] }

// Answers the report, or every problem with it, joined.
export const parseReport = (e: Record<string, unknown>): Parsed | string => {
  const errors = findings()
  …unchanged…
  if (errors.length === 0) return { report, warnings: errors.warnings }
  …unchanged…
}
```

- [ ] **Step 5: Read the new shape in `hooks/refine.ts`**

In the report-mode path (around line 376), replace

```ts
      const report = parseReport(e)
      if (typeof report === 'string') return { deny: `show_task_report: ${report}` }

      const shown = { description: task.description, notes: notesOf(task) }
      // The page is built from the report's Markdown twin as it reads back,
      // never from the fields beside it, so the two cannot disagree.
      const twin = twinOf(report, shown)
```

with

```ts
      const parsed = parseReport(e)
      if (typeof parsed === 'string') return { deny: `show_task_report: ${parsed}` }

      const shown = { description: task.description, notes: notesOf(task) }
      // The page is built from the report's Markdown twin as it reads back,
      // never from the fields beside it, so the two cannot disagree.
      const twin = twinOf(parsed.report, shown)
```

In the before-approval path (around line 424), replace

```ts
    const report = parseReport(e)
    if (typeof report === 'string') return { deny: `show_task_report: ${report}` }

    const twin = twinOf(report, askedFor)
```

with

```ts
    const parsed = parseReport(e)
    if (typeof parsed === 'string') return { deny: `show_task_report: ${parsed}` }

    const twin = twinOf(parsed.report, askedFor)
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `claude plugin test claude/refine-mod`
Expected: PASS, `0 fail`.

- [ ] **Step 7: Type-check**

Type-check against this build's API, from a scratch folder. `claude-code.d.ts` is written when the `plugin-authoring` skill loads; take the newest (load that skill first if the `ls` finds none):

```bash
D="$(dirname "$(ls -t /tmp/claude-1000/bundled-skills/*/*/plugin-authoring/types/claude-code.d.ts | head -1)")"
S="$(mktemp -d)"; M="$PWD/claude/refine-mod"
cat > "$S/tsconfig.json" <<EOF
{ "compilerOptions": { "target": "es2023", "lib": ["es2023"], "types": [], "module": "esnext",
  "moduleResolution": "bundler", "strict": true, "noUncheckedIndexedAccess": true, "noEmit": true, "skipLibCheck": true },
  "include": ["$D/claude-code.d.ts", "$M/hooks", "$M/tests"] }
EOF
(cd "$S" && npx -y -p typescript@5.6.3 tsc -p tsconfig.json); echo "tsc exit $?"; rm -r "$S"
```

Expected: no output, then `tsc exit 0`.

- [ ] **Step 8: Commit**

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/hooks/refine.ts claude/refine-mod/tests/report.test.ts claude/refine-mod/tests/markdown.test.ts claude/refine-mod/tests/page.test.ts
git commit -m "$(cat <<'EOF'
perf(refine): take report lines a word or two over their limits

A report line one or two words over its word limit no longer refuses
the whole report: parseReport takes it and answers the overage as a
warning beside the report. Three words over is still refused, and
character, line and item limits stay strict.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Name the overage in the result and the transcript

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts` (add `overLimits()` after `parseReport`)
- Modify: `claude/refine-mod/hooks/refine.ts` (both `show_task_report` paths, their `Report:` log and `result`)
- Test: `claude/refine-mod/tests/report.test.ts`, `claude/refine-mod/tests/refine.test.ts`

**Interfaces:**
- Consumes: `parseReport(e): Parsed | string` and `Parsed = { report: Report; warnings: string[] }` from Task 1; in `refine.ts`, the local `parsed` each path now holds.
- Produces: `export const overLimits = (warnings: readonly string[]): string` in `hooks/report.ts` — `''` for none, else `` ` Taken, though over its word limits: ${warnings.join('; ')}.` `` (leading space, so it is appended to a result sentence).

- [ ] **Step 1: Write the failing tests**

In `claude/refine-mod/tests/report.test.ts`, add `overLimits` to the import on line 2, and inside `describe('the word grace', …)` add:

```ts
  test('the tool result names each line taken over its limit, and says nothing when none is', () => {
    expect(overLimits([])).toBe('')
    expect(overLimits(['summary has 36 words; at most 35', 'changes[0] has 22 words; at most 20'])).toBe(
      ' Taken, though over its word limits: summary has 36 words; at most 35; changes[0] has 22 words; at most 20.',
    )
  })
```

In `claude/refine-mod/tests/refine.test.ts`, after the `MINIMAL` constant, add:

```ts
// A needs_your_eye line one word over its limit of 30, which the tool takes
// and names; and one three over, which it refuses.
const OVER = Array.from({ length: 31 }, () => 'word').join(' ')
const TOO_FAR_OVER = Array.from({ length: 33 }, () => 'word').join(' ')
const OVERAGE = 'needs_your_eye[0] has 31 words; at most 30'
```

After the test `'a report over its limits is refused with every reason'`, add:

```ts
test('a report a word over its limit is written, and the overage named', { options: REPORTS }, async ($, on) => {
  const { seen, writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, needs_your_eye: [OVER] })
  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH} and opened it in the browser. ` +
      'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
      `and the task will link the report in its notes. Taken, though over its word limits: ${OVERAGE}.`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(seen).toContain(`log: Over its word limit: ${OVERAGE}`)
})

test('a report three words over its limit is still refused', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, needs_your_eye: [TOO_FAR_OVER] })
  expect(answer.deny).toBe('show_task_report: needs_your_eye[0] has 33 words; at most 30')
  expect(writes).toEqual([])
})
```

After the test `'report mode: writes the page, opens it, and links it from the task, asking nothing'`, add:

```ts
test('report mode: a report a word over its limit is written, and the overage named', { options: REPORT_MODE }, async ($, on) => {
  const { seen, writes } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, needs_your_eye: [OVER] })
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH}, opened it in the browser, and linked it from the task ` +
      `as the note "Report: ${REPORT_PATH}". Taken, though over its word limits: ${OVERAGE}.`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(seen).toEqual([`log: Report: ${REPORT_PATH}`, `log: Over its word limit: ${OVERAGE}`])
})
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL — `report.test.ts` cannot import `overLimits`; the two "a word over" tests in `refine.test.ts` get the result without `Taken, though over its word limits: …` and no `Over its word limit:` log. `'a report three words over its limit is still refused'` passes already (Task 1 refuses it).

- [ ] **Step 3: Add `overLimits` to `hooks/report.ts`**

Right after `parseReport`:

```ts
// What the tool's answer adds for the lines taken over their word limits:
// each named, so the next report keeps to the limits. Nothing when none is.
export const overLimits = (warnings: readonly string[]): string =>
  warnings.length === 0 ? '' : ` Taken, though over its word limits: ${warnings.join('; ')}.`
```

- [ ] **Step 4: Log and answer the overage in `hooks/refine.ts`**

Add `overLimits` to the `./report` import on line 28 (keep the names sorted):

```ts
import { madeAlready, openArgv, overLimits, parseReport, reportPath, reportsDir, unreportable, withReportNote, withReportNotes } from './report'
```

In the report-mode path, right after `$.ui.log(\`Report: ${path}\`)`, add:

```ts
      for (const warning of parsed.warnings) $.ui.log(`Over its word limit: ${warning}`)
```

and change its success `result` to end with the overage:

```ts
      return {
        result:
          (unattended
            ? `Wrote the report to ${path} and linked it from the task as the note "Report: ${path}".`
            : `Wrote the report to ${path}, opened it in the browser, and linked it from the task ` +
              `as the note "Report: ${path}".`) + overLimits(parsed.warnings),
      }
```

In the before-approval path, right after its `$.ui.log(\`Report: ${path}\`)`, add:

```ts
    for (const warning of parsed.warnings) $.ui.log(`Over its word limit: ${warning}`)
```

and change its `result` to:

```ts
    return {
      result:
        `Wrote the report to ${path} and opened it in the browser. ` +
        'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
        'and the task will link the report in its notes.' +
        overLimits(parsed.warnings),
    }
```

Leave the "but task import failed" denial as it is: the log already names the overage there.

- [ ] **Step 5: Run the tests to see them pass**

Run: `claude plugin test claude/refine-mod`
Expected: PASS, `0 fail`. The existing report tests' exact `result` and `seen` expectations still hold, since `MINIMAL` has no overage.

- [ ] **Step 6: Type-check and validate**

Run the type-check from Task 1 Step 7 again (same script). Expected: no output, then `tsc exit 0`.

Run: `claude plugin validate claude/refine-mod`
Expected: `✔ Validation passed`.

Confirm the grace is not advertised:

Run: `git diff main -- .claude/skills claude/refine-mod/hooks/refine.ts | grep -n -i 'grace\|description:'`
Expected: no output (no tool description or skill file changed).

- [ ] **Step 7: Commit**

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/hooks/refine.ts claude/refine-mod/tests/report.test.ts claude/refine-mod/tests/refine.test.ts
git commit -m "$(cat <<'EOF'
perf(refine): name a report's lines taken over their word limits

show_task_report now ends its answer with each line it took over its
word limit and logs each to the transcript, in report mode and before
approval, so the model keeps to the limits next time without resending
the report.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
