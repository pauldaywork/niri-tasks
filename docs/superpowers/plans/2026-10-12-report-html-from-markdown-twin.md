# Report HTML From Its Markdown Twin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The refine report's HTML slides are built only from its Markdown twin, diagrams included, so the two can never disagree.

**Architecture:** Diagrams become Mermaid only (the model-written SVG option goes). `markdown.ts` writes every field into the twin (each part's lens as a hidden comment, More detail as a `<details>` block) and gains the pure inverse `reportFromMarkdown(md) -> { report, plan }` plus `twinOf(report, plan)`, which writes the twin, reads it back and checks it re-writes to the same text. Both `show_task_report` paths in `refine.ts` build the page and draw the diagrams from what `twinOf` read back; `page.ts`'s slides, limits and beautiful-mermaid drawing are unchanged.

**Tech Stack:** TypeScript Claude Code mod (`claude/refine-mod`), tests through `claude-code/testing`, run with `claude plugin test claude/refine-mod`.

**Spec:** Taskwarrior task `a462ed7b-6c06-4a4d-8d3a-51a397d19b7b` (`task rc.json.array=on a462ed7b-6c06-4a4d-8d3a-51a397d19b7b export`): its description and notes are the spec.

## Global Constraints

- The source is the `.md` twin the mod writes; rebuilding the HTML from a hand-edited `.md` is out of scope.
- Diagrams are Mermaid only, kept as ```` ```mermaid ```` blocks. Drop the model-written SVG diagram option.
- Read the twin back with a new pure `reportFromMarkdown(md) -> { report, plan }`, the inverse of `reportMarkdown`; keep `page.ts`'s slides, limits and beautiful-mermaid drawing as they are. No Marp, markdown-it, reveal.js/Slidev or Pandoc.
- The twin carries each part's lens as `<!-- lens: <kind> -->` under its heading, and More detail as a `<details>` block.
- Out of scope: a command to rebuild the HTML from an edited `.md`; restyling diagrams; Marp, PDF or PPTX export; reports already written.
- Done when: `reportFromMarkdown(reportMarkdown(r, p))` equals `{ r, p }` for the example report and edge cases (`|` in table cells, backticks in code, new and removed files, empty lists, every lens); the example report's page built from its twin is byte-identical to the page built from its fields; a real refine report opens with its diagrams drawn; the refine-mod tests pass.
- Commits: Conventional Commits, scope `refine`, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Comments match the mod's voice: plain sentences saying what a thing is and why, no "we".

## Decisions this plan makes

- **A plain part's mark.** The model may leave `kind` out for a plain part, and `{ kind: 'part' }` and no kind must both read back exactly. So a part with a kind gets `<!-- lens: <kind> -->` and a part without one gets `<!-- lens -->`. Every part carries a mark, which is also how the reader tells a part from a fixed slide that shares its heading (a part headed "Files", say).
- **The reader is positional.** It reads blocks in the exact order `reportMarkdown` writes them and throws, naming the line, at anything else. It never parses Markdown in general, so a summary that starts with `- ` or detail that holds `## ` lines reads back as written.
- **`twinOf` guards the round trip at run time.** It reads the twin back and checks that the read-back re-writes to the same Markdown; if not, `show_task_report` writes nothing and says why. The one input known to fail is a task note holding a line break (report mode reads the task's own notes), which `reportMarkdown` cannot keep on one numbered line.
- **The page is built from the string just written, not re-read from disk.** The engine tests' `$.fs` refuses reads, and the string is byte-for-byte the file. The HTML is still written before the `.md`, so every existing `writes[0]` (page) and `writes[1]` (twin) assertion keeps holding.

## Files

| File | Change | Responsibility |
|---|---|---|
| `claude/refine-mod/hooks/report.ts` | Modify | `Diagram` is `{ mermaid }`; drop SVG validation; export `CHANGES`, `SWAPS` for the reader |
| `claude/refine-mod/hooks/page.ts` | Modify | drop the SVG branch and the `'mermaid' in` checks |
| `claude/refine-mod/hooks/draw.ts` | Modify | drop the `'mermaid' in` check; reword a comment |
| `claude/refine-mod/hooks/markdown.ts` | Modify | lossless writer, `reportFromMarkdown`, `twinOf` |
| `claude/refine-mod/hooks/refine.ts` | Modify | Mermaid-only tool schema; both report paths build from `twinOf` |
| `claude/refine-mod/tests/report.test.ts` | Modify | SVG tests go |
| `claude/refine-mod/tests/page.test.ts` | Modify | SVG tests go |
| `claude/refine-mod/tests/draw.test.ts` | Modify | SVG part goes |
| `claude/refine-mod/tests/markdown.test.ts` | Modify | new format, round trips, byte-identical page |
| `claude/refine-mod/tests/refine.test.ts` | Modify | a twin that does not read back writes nothing |
| `.claude/skills/refine-task/report-catalogue.md` | Modify | Mermaid only |
| `README.md`, `CONTEXT.md` | Modify | the HTML is built from the twin |

`tests/example-report.ts` holds no SVG diagram, so it needs no change.

---

### Task 1: Diagrams are Mermaid only

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts`
- Modify: `claude/refine-mod/hooks/page.ts:120-151` (`legendFor`, `drawing`) and `:409-411` (`browser`)
- Modify: `claude/refine-mod/hooks/draw.ts:47-54` (`drawSources`), `:73-74` (comment)
- Modify: `claude/refine-mod/hooks/markdown.ts:37-38` (`drawing`)
- Modify: `claude/refine-mod/hooks/refine.ts:62` (`DIAGRAM`)
- Modify: `.claude/skills/refine-task/report-catalogue.md`
- Test: `claude/refine-mod/tests/report.test.ts`, `tests/page.test.ts`, `tests/draw.test.ts`, `tests/markdown.test.ts`

**Interfaces:**
- Produces: `export type Diagram = { mermaid: string }` in `report.ts`; `export const CHANGES: readonly string[]` and `export const SWAPS: readonly string[]` in `report.ts` (Task 2's reader uses them).

- [ ] **Step 1: Change the tests to Mermaid only**

In `claude/refine-mod/tests/report.test.ts`, replace the test `'a diagram is Mermaid or SVG, not both, and not too long'` with:

```ts
  test('a diagram is Mermaid, and not too long', () => {
    expect(parseReport(drawn({ svg: '<svg></svg>' }))).toBe('sections[0].diagram must be { "mermaid": "…" }')
    expect(parseReport(drawn('flowchart LR'))).toBe('sections[0].diagram must be { "mermaid": "…" }')
    const long = Array.from({ length: LIMITS.mermaidLines + 1 }, () => 'a --> b').join('\n')
    expect(parseReport(drawn({ mermaid: long }))).toBe(
      `sections[0].diagram.mermaid has ${LIMITS.mermaidLines + 1} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`,
    )
  })
```

In the same file:
- Rename `'refuses every element the page must not hold, in detail, SVG and Mermaid'` to `'refuses every element the page must not hold, in detail and Mermaid'` and delete its three lines `expect(parseReport(drawn({ svg: '<svg><script>x</script></svg>' }))).toContain(` … `)`.
- Rename `'refuses an HTML comment in detail or SVG'` to `'refuses an HTML comment in detail'` and delete its `drawn({ svg: '<svg><!-- b --></svg>' })` expectation (three lines).
- In `"refuses closing the page's own structure"`, delete the `drawn({ svg: '<svg></figure></svg>' })` expectation (three lines).
- Delete the whole test `'refuses <style> in SVG, and keeps its <title>'`.

In `claude/refine-mod/tests/page.test.ts`, inside `describe('diagrams', …)`:
- Change the helper's parameter type: `const part = (diagram: { mermaid: string }) =>`.
- Delete the test `'pass SVG through as given'`.
- In `'carry the legend under the drawing, only where the change colours apply'`, delete the line `expect(part({ svg: '<svg viewBox="0 0 1 1"><rect/></svg>' })).not.toContain('<p class="legend">')`.

In `claude/refine-mod/tests/draw.test.ts`, in `'gives each Mermaid part its source, flowcharts with the change colours'`, replace the part `{ heading: 'c', points: ['p'], look_at: 'x', diagram: { svg: '<svg></svg>' } },` with `{ heading: 'c', points: ['p'] },` (its three slots stay `undefined`, so the expectations hold).

In `claude/refine-mod/tests/markdown.test.ts`, replace the test `'a Mermaid diagram is a mermaid block; a drawn SVG points at the HTML'` with:

```ts
  test('a diagram is a mermaid block, and a pair is two', () => {
    const md = reportMarkdown(
      {
        ...MINIMAL,
        sections: [
          { heading: 'Flow', points: ['p'], look_at: 'The arrow.', diagram: { mermaid: 'flowchart LR\n  a --> b' } },
          { heading: 'Pair', points: ['p'], look_at: 'Both.', before: { mermaid: 'flowchart LR\n  a' }, after: { mermaid: 'flowchart LR\n  b' } },
        ],
      },
      PLAN,
    )
    expect(md).toContain('## Flow\n\n_The arrow._\n\n```mermaid\nflowchart LR\n  a --> b\n```\n\n- p')
    expect(md).not.toContain('see the HTML report')
    expect(md).toContain('Before:\n\n```mermaid\nflowchart LR\n  a\n```\n\nAfter:\n\n```mermaid\nflowchart LR\n  b\n```')
  })
```

- [ ] **Step 2: Run the tests to see the SVG ones fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL in `report.test.ts` — `'a diagram is Mermaid, and not too long'` gets `sections[0].diagram must have exactly one of mermaid or svg` (or the SVG's own refusal) instead of `must be { "mermaid": "…" }`. The other edited tests may pass already.

- [ ] **Step 3: Make `Diagram` Mermaid only in `report.ts`**

In `claude/refine-mod/hooks/report.ts`:

Replace `export type Diagram = { mermaid: string } | { svg: string }` with:

```ts
// A diagram is Mermaid source: text the Markdown twin keeps as it is, and the
// page draws before it opens.
export type Diagram = { mermaid: string }
```

Replace `const CHANGES: readonly string[] = ['new', 'change', 'remove']` with:

```ts
export const CHANGES: readonly string[] = ['new', 'change', 'remove']
```

Replace the comment and constants from `// What would break the page's one layout, refused in detail and SVG, which` down to and including `const SWALLOWS_SVG = /<\s*(style)(?![\w-])/i` with:

```ts
// What would break the page's one layout, refused in detail, which the page
// shows as given (Mermaid source is escaped, so it cannot): a comment left
// open hides the rest of the page, and closing one of the page's own
// elements moves what follows out of its place. Drawn diagrams are held to
// them too.
export const COMMENT = /<!--/
export const CLOSES = /<\s*\/\s*(details|section|main|footer|nav|figure|header|body|html)(?![\w-])/i

// A drawn diagram is one whole <svg> element, nothing before or after.
export const WHOLE_SVG = /^\s*<svg[\s>][\s\S]*<\/svg>\s*$/i

// Elements that swallow the rest of the page as text, or restyle it.
const SWALLOWS_DETAIL = /<\s*(style|plaintext|xmp|textarea|title|noscript|template|html|head|body)(?![\w-])/i
```

Replace the whole `const diagram = …` function with:

```ts
const diagram = (errors: Errors, at: string, value: unknown): Diagram | undefined => {
  if (!isObject(value) || !('mermaid' in value)) {
    errors.push(`${at} must be { "mermaid": "…" }`)
    return undefined
  }
  const source = markup(errors, `${at}.mermaid`, value.mermaid, LIMITS.diagramChars)
  const lines = source.split('\n').length
  if (lines > LIMITS.mermaidLines) {
    errors.push(`${at}.mermaid has ${lines} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`)
  }
  return { mermaid: source }
}
```

Replace `const SWAPS: readonly string[] = ['easy', 'medium', 'hard']` with:

```ts
export const SWAPS: readonly string[] = ['easy', 'medium', 'hard']
```

In the `FORBIDDEN` comment, nothing names SVG; leave it. In `markup`'s comment, leave it (it does not name SVG).

- [ ] **Step 4: Drop the SVG branch in `page.ts`**

In `claude/refine-mod/hooks/page.ts`, in `legendFor`, delete the line `if (!('mermaid' in diagram)) return ''`.

Replace `drawing` and its comment with:

```ts
// One diagram as the page shows it: drawn before the page opened, else
// Mermaid in the browser.
const drawing = (diagram: Diagram, svg?: string): string =>
  svg !== undefined
    ? `<div class="drawn"${drawnWidth(svg)}>${svg}</div>`
    : `<pre class="mermaid">${escapeHtml(withMarks(diagram.mermaid))}</pre>`
```

In `reportPage`, replace the `browser` expression's inner test

```ts
    [s.diagram, s.before, s.after].some((d, slot) => d !== undefined && 'mermaid' in d && drawn[i * SLOTS + slot] === undefined),
```

with

```ts
    [s.diagram, s.before, s.after].some((d, slot) => d !== undefined && drawn[i * SLOTS + slot] === undefined),
```

- [ ] **Step 5: Drop the check in `draw.ts`**

In `claude/refine-mod/hooks/draw.ts`, in `drawSources`, replace

```ts
      d !== undefined && 'mermaid' in d && drawable(d.mermaid) ? withMarks(d.mermaid) : undefined,
```

with

```ts
      d !== undefined && drawable(d.mermaid) ? withMarks(d.mermaid) : undefined,
```

Replace the comment above `keptSvg`:

```ts
// An SVG from the renderer as the page will show it, held to what a model's
// SVG is; undefined when it is not one the page will take.
```

with

```ts
// An SVG from the renderer as the page will show it, held to what the page
// shows as given; undefined when it is not one the page will take.
```

- [ ] **Step 6: Drop the placeholder in `markdown.ts`**

In `claude/refine-mod/hooks/markdown.ts`, replace

```ts
const drawing = (diagram: Diagram): string =>
  'mermaid' in diagram ? fence(diagram.mermaid, 'mermaid') : '_A drawn diagram: see the HTML report._'
```

with

```ts
const drawing = (diagram: Diagram): string => fence(diagram.mermaid, 'mermaid')
```

- [ ] **Step 7: Mermaid only in the tool schema**

In `claude/refine-mod/hooks/refine.ts`, replace

```ts
const DIAGRAM = { type: 'object', properties: { mermaid: TEXT, svg: TEXT } }
```

with

```ts
const DIAGRAM = {
  type: 'object',
  description: 'A Mermaid diagram: { "mermaid": "<source>" }.',
  properties: { mermaid: TEXT },
  required: ['mermaid'],
}
```

- [ ] **Step 8: Mermaid only in the report catalogue**

In `.claude/skills/refine-task/report-catalogue.md`:

Replace the row

```
| `sections[].diagram` | One diagram: `{ "mermaid": "…" }` or `{ "svg": "<svg …>…</svg>" }`. | Mermaid 40 lines; 50,000 characters |
```

with

```
| `sections[].diagram` | One Mermaid diagram: `{ "mermaid": "…" }`. | 40 lines; 50,000 characters |
```

Replace the two paragraphs starting `` `detail`, an SVG and a Mermaid diagram may not contain `` and ending `` fine. `` with:

```
`detail` and a Mermaid diagram may not contain `<script>`, `<meta>`,
`<link>`, `<base>`, `<iframe>`, `<frame>`, `<frameset>`, `<object>`,
`<embed>`, `<form>` or `<portal>`: the page runs no script when every
diagram is drawn, loads only pinned Mermaid for one that is not, and sends
nothing.

`detail` is shown as given, so it may not break the page's one layout
either: no HTML comment (`<!--`), and no closing tag of the page's own
`<details>`, `<section>`, `<main>`, `<footer>`, `<nav>`, `<figure>`,
`<header>`, `<body>` or `<html>`. It may not contain `<style>`,
`<plaintext>`, `<xmp>`, `<textarea>`, `<title>`, `<noscript>`, `<template>`,
`<html>`, `<head>` or `<body>`, which swallow or restyle the page.
```

In the lens table's `styling` row, replace `A \`before\`/\`after\` pair of screen mock-ups (SVG) or of the layout.` with `A \`before\`/\`after\` pair of the layout, as a \`flowchart\` of the screen's regions.`

Delete the last bullet of the diagram list:

```
- Use SVG only for what Mermaid draws badly, such as a screen mock-up. Give
  it a `viewBox`, and draw lines and text in `currentColor`.
```

Then check nothing else names SVG as a diagram choice:

Run: `grep -n -i svg .claude/skills/refine-task/*.md claude/refine-mod/hooks/report.ts claude/refine-mod/hooks/refine.ts claude/refine-mod/hooks/markdown.ts`
Expected: only `WHOLE_SVG` in `report.ts` (the drawn diagram check).

- [ ] **Step 9: Run the tests**

Run: `claude plugin test claude/refine-mod`
Expected: PASS, `0 fail`.

- [ ] **Step 10: Commit**

```bash
git add claude/refine-mod .claude/skills/refine-task/report-catalogue.md
git commit -m "$(cat <<'EOF'
refactor(refine): take Mermaid diagrams only in a report

A model-written SVG diagram cannot be kept as text in the report's
Markdown twin, and no report has used one. Diagrams are now Mermaid
source only, so the twin can carry every diagram the slides draw.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: A lossless twin that reads back

**Files:**
- Modify: `claude/refine-mod/hooks/markdown.ts` (whole file below)
- Test: `claude/refine-mod/tests/markdown.test.ts`

**Interfaces:**
- Consumes: `Diagram = { mermaid: string }`, `CHANGES`, `SWAPS`, `LENSES` from `report.ts` (Task 1).
- Produces, in `markdown.ts`:
  - `export type ReadBack = { report: Report; plan: Plan }`
  - `export const reportFromMarkdown = (markdown: string): ReadBack` — throws `Error('line <n>: expected <what>')`
  - `export type Twin = ReadBack & { markdown: string }`
  - `export const twinOf = (report: Report, plan: Plan): Twin | string` — the string is a reason, e.g. `its Markdown twin does not read back (line 30: expected "")`
  - `reportMarkdown` and `markdownPath` keep their signatures.

- [ ] **Step 1: Write the failing tests**

In `claude/refine-mod/tests/markdown.test.ts`, change the imports at the top to:

```ts
import { describe, expect, test } from 'claude-code/testing'
import { drawSources } from '../hooks/draw'
import { markdownPath, reportFromMarkdown, reportMarkdown, twinOf } from '../hooks/markdown'
import { reportPage } from '../hooks/page'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'
import { EXAMPLE, EXAMPLE_PLAN } from './example-report'
```

In `'the smallest report reads top to bottom as its slides do'`, the part now carries its mark: replace

```ts
        '## What changes',
        '',
        '- The words.',
```

with

```ts
        '## What changes',
        '',
        '<!-- lens -->',
        '',
        '- The words.',
```

In `'the outside tools are a table, and detail follows the points'`, replace

```ts
    expect(md).toContain('More detail:\n\n<p>More.</p>')
```

with

```ts
    expect(md).toContain('| herdr | Its CLI. | hard | Panes. |\n\n<details>\n<summary>More detail</summary>\n<p>More.</p>\n</details>')
```

Add after the `MINIMAL` and `PLAN` constants:

```ts
// What a reader of the twin could trip on, as parseReport takes it: pipes and
// backslashes in table cells, fences and blank lines in code, a new and a
// removed file, a part named like a fixed slide, detail that looks like
// Markdown, a plain part with no lens, and every lens.
const EDGES_INPUT = {
  title: 'fix: Keep `a | b` apart',
  summary: '- A summary that starts like a list, with | and ``` in it.',
  changes: ['Pipes: a | b stays one cell.'],
  unchanged: ['The panel.'],
  needs_your_eye: ['Backslash: a \\| b keeps its backslash.'],
  terms: [{ term: 'twin', meaning: 'The .md copy: read back.' }],
  sections: [
    {
      kind: 'before-after',
      heading: 'Before and after',
      look_at: 'The new box.',
      before: { mermaid: 'flowchart LR\n  a --> b' },
      after: { mermaid: 'flowchart LR\n  a --> c:::new' },
      points: ['p'],
    },
    {
      heading: 'A plain part',
      look_at: '_underscored_ words',
      diagram: { mermaid: 'sequenceDiagram\n  A->>B: `hi`' },
      points: ['- a dash', '_x_'],
    },
    { kind: 'part', heading: 'What stays the same', points: ['A part named like a fixed slide.'] },
    {
      kind: 'structure',
      heading: 'Files',
      points: ['Another.'],
      detail: '<p>Deep</p>\n\n## not a heading\n\n```\nnot a fence\n```\n',
    },
    { kind: 'data-flow', heading: 'Data', look_at: 'x', diagram: { mermaid: 'flowchart LR\n  a -->|"`md`"| b' }, points: ['p'] },
    {
      kind: 'outside-tools',
      heading: 'Tools',
      points: ['p'],
      tools: [{ tool: 'a|b', how: 'Pipe | escaped.', swap: 'easy', why: 'Ends with \\' }],
    },
    { kind: 'styling', heading: 'Look', points: ['p'] },
    {
      kind: 'code',
      heading: 'Code',
      points: ['p'],
      code: [
        { file: 'new.ts', before: '', after: 'const a = "```"\n' },
        { file: 'gone.ts', before: 'x ```` y', after: '' },
        { file: 'both.ts', before: '\tindented', after: 'line\n\nwith a gap' },
      ],
    },
    { kind: 'newcomer', heading: 'For a newcomer', points: ['p'] },
  ],
  files: [
    { path: 'src/a|b.rs', change: 'new', why: 'Holds | pipes.' },
    { path: 'src/old.rs', change: 'remove', why: 'Gone.' },
  ],
  checks: [{ check: 'a | b', how: ' leading space' }],
}
const EDGES = parseReport(EDGES_INPUT) as Report
```

Add at the end of the file:

```ts
describe('reportFromMarkdown', () => {
  test('the edge cases are a report the tool takes', () => {
    expect(typeof parseReport(EDGES_INPUT)).toBe('object')
  })

  test('reads the worked example back into the fields it was written from', () => {
    const report = parseReport(EXAMPLE) as Report
    expect(reportFromMarkdown(reportMarkdown(report, EXAMPLE_PLAN))).toEqual({ report, plan: EXAMPLE_PLAN })
  })

  test('reads back pipes, backslashes, fences, new and removed files, and every lens', () => {
    expect(reportFromMarkdown(reportMarkdown(EDGES, PLAN))).toEqual({ report: EDGES, plan: PLAN })
  })

  test('reads back empty lists, and a plan with no notes', () => {
    const plan = { description: 'd', notes: [] }
    expect(reportFromMarkdown(reportMarkdown(MINIMAL, plan))).toEqual({ report: MINIMAL, plan })
  })

  test('marks each part with its lens, and a plain part with none', () => {
    const md = reportMarkdown(EDGES, PLAN)
    expect(md).toContain('## Before and after\n\n<!-- lens: before-after -->\n\n_The new box._')
    expect(md).toContain('## A plain part\n\n<!-- lens -->\n\n')
    expect(md).toContain('## What stays the same\n\n<!-- lens: part -->\n\n')
  })

  test('throws, naming the line, at what it would not have written', () => {
    expect(() => reportFromMarkdown('# t\n\ns\n\n## What changes\n\nnot a list\n')).toThrow(
      'line 7: expected a "- " line',
    )
  })

  test("the worked example's page from its twin is the page from its fields, byte for byte", () => {
    const report = parseReport(EXAMPLE) as Report
    const twin = reportFromMarkdown(reportMarkdown(report, EXAMPLE_PLAN))
    expect(reportPage(twin.report, twin.plan)).toBe(reportPage(report, EXAMPLE_PLAN))
    expect(drawSources(twin.report)).toEqual(drawSources(report))
    const drawn = drawSources(report).map(s => (s === undefined ? undefined : '<svg viewBox="0 0 4 1"></svg>'))
    expect(reportPage(twin.report, twin.plan, undefined, drawn)).toBe(reportPage(report, EXAMPLE_PLAN, undefined, drawn))
  })

  test("the edge cases' page from their twin is the page from their fields", () => {
    const twin = reportFromMarkdown(reportMarkdown(EDGES, PLAN))
    expect(reportPage(twin.report, twin.plan)).toBe(reportPage(EDGES, PLAN))
  })
})

describe('twinOf', () => {
  test('is the twin and what reads back from it', () => {
    expect(twinOf(MINIMAL, PLAN)).toEqual({ markdown: reportMarkdown(MINIMAL, PLAN), report: MINIMAL, plan: PLAN })
  })

  test('says why when the twin does not read back: a note on two lines', () => {
    expect(twinOf(MINIMAL, { description: 'd', notes: ['one\ntwo'] })).toStartWith(
      'its Markdown twin does not read back (line ',
    )
  })
})
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL — `markdown.test.ts` cannot import `reportFromMarkdown` / `twinOf` (export not found), and the smallest-report test misses `<!-- lens -->`.

- [ ] **Step 3: Write the lossless writer and the reader**

Replace the whole of `claude/refine-mod/hooks/markdown.ts` with:

```ts
// The Markdown twin of a report: the same fields as the HTML slides, top to
// bottom, as a document plannotator-tui can open in herdr, where the comments
// made on it go back to the task's Claude. It holds every field the slides
// use, so it reads back into the report it was written from, and the slides
// are built from what reads back: the two cannot disagree. Pure, like
// report.ts, so the unit tests reach it directly.

import type { Plan } from './plan'
import { CHANGES, LENSES, SWAPS } from './report'
import type { Change, CodePair, Diagram, Lens, Report, Section, Swap, Tool } from './report'

// The twin's path: the report's, with .md for .html.
export const markdownPath = (htmlPath: string): string => `${htmlPath.replace(/\.html$/, '')}.md`

// A fence longer than any run of backticks in the text, so code that holds
// ``` stays one block.
const fence = (text: string, lang = ''): string => {
  const longest = Math.max(2, ...(text.match(/`+/g) ?? []).map(run => run.length))
  const bar = '`'.repeat(longest + 1)
  return `${bar}${lang}\n${text}\n${bar}`
}

const cell = (text: string): string => text.replace(/\|/g, '\\|')

// The tables' heads, as written and as read back.
const TOOL_HEAD = ['Tool', 'How it connects', 'Swap', 'Why']
const FILE_HEAD = ['Change', 'File', 'Why']
const CHECK_HEAD = ['Check', 'How it is checked']

const headRow = (head: readonly string[]): string => `| ${head.join(' | ')} |`

const table = (head: readonly string[], rows: readonly (readonly string[])[]): string =>
  [
    headRow(head),
    `|${head.map(() => ' --- ').join('|')}|`,
    ...rows.map(row => `| ${row.map(cell).join(' | ')} |`),
  ].join('\n')

const bullets = (lines: readonly string[]): string => lines.map(line => `- ${line}`).join('\n')

// A part's lens, as a comment under its heading, which the rendered Markdown
// hides: `<!-- lens: <kind> -->`, or `<!-- lens -->` for a plain part given
// none. It also tells a part from a fixed slide that shares its heading.
const lensMark = (s: Section): string => (s.kind === undefined ? '<!-- lens -->' : `<!-- lens: ${s.kind} -->`)
const LENS_MARK = /^<!-- lens(?:: (\S+))? -->$/

// More detail, collapsed, as the slide shows it.
const MORE = '<summary>More detail</summary>'

const drawing = (diagram: Diagram): string => fence(diagram.mermaid, 'mermaid')

const code = (pairs: readonly CodePair[]): string =>
  pairs
    .map(pair =>
      [
        `### ${pair.file}`,
        pair.before === '' ? '_New file._' : `Before:\n\n${fence(pair.before)}`,
        pair.after === '' ? '_Removed._' : `After:\n\n${fence(pair.after)}`,
      ].join('\n\n'),
    )
    .join('\n\n')

const tools = (list: readonly Tool[]): string => table(TOOL_HEAD, list.map(t => [t.tool, t.how, t.swap, t.why]))

const part = (s: Section): string => {
  const out = [`## ${s.heading}`, lensMark(s)]
  if (s.look_at !== undefined) out.push(`_${s.look_at}_`)
  if (s.diagram !== undefined) out.push(drawing(s.diagram))
  if (s.before !== undefined && s.after !== undefined) {
    out.push(`Before:\n\n${drawing(s.before)}`, `After:\n\n${drawing(s.after)}`)
  }
  out.push(bullets(s.points))
  if (s.code !== undefined) out.push(code(s.code))
  if (s.tools !== undefined) out.push(tools(s.tools))
  if (s.detail !== undefined) out.push(`<details>\n${MORE}\n${s.detail}\n</details>`)
  return out.join('\n\n')
}

// The report as Markdown, in the slides' order, ending with the plan the
// person was shown, as the HTML's last slide does.
export const reportMarkdown = (report: Report, plan: Plan): string => {
  const out = [`# ${report.title}`, report.summary, `## What changes\n\n${bullets(report.changes)}`]
  if (report.unchanged.length > 0) out.push(`## What stays the same\n\n${bullets(report.unchanged)}`)
  if (report.needs_your_eye.length > 0) {
    out.push(`## Where your judgement is needed\n\n${bullets(report.needs_your_eye)}`)
  }
  if (report.terms.length > 0) {
    out.push(`## Words used here\n\n${bullets(report.terms.map(t => `**${t.term}**: ${t.meaning}`))}`)
  }
  out.push(...report.sections.map(part))
  if (report.files.length > 0) {
    out.push(`## Files\n\n${table(FILE_HEAD, report.files.map(f => [f.change, `\`${f.path}\``, f.why]))}`)
  }
  out.push(`## Done when\n\n${table(CHECK_HEAD, report.checks.map(c => [c.check, c.how]))}`)
  const notes = plan.notes.length === 0 ? 'No notes.' : plan.notes.map((n, i) => `${i + 1}. ${n}`).join('\n')
  out.push(`## The plan\n\n**Description:** ${plan.description}\n\n${notes}`)
  return `${out.join('\n\n')}\n`
}

// What reads back from a twin: the report and the plan it was written from.
export type ReadBack = { report: Report; plan: Plan }

// The twin read back into the fields it was written from, the inverse of
// reportMarkdown: block by block, in the order it writes them, never as
// Markdown in general, so a line that only looks like a heading or a list
// reads back as written. Throws, naming the line, at anything reportMarkdown
// would not have written.
export const reportFromMarkdown = (markdown: string): ReadBack => {
  const lines = markdown.split('\n')
  let at = 0
  const fail = (wanted: string): never => {
    throw new Error(`line ${at + 1}: expected ${wanted}`)
  }
  const take = (): string => {
    const line = lines[at]
    if (line === undefined) return fail('more lines')
    at += 1
    return line
  }
  const line = (exact: string): void => {
    if (lines[at] !== exact) fail(JSON.stringify(exact))
    at += 1
  }
  const match = (pattern: RegExp, wanted: string): RegExpExecArray => {
    const found = pattern.exec(lines[at] ?? '')
    if (found === null) return fail(wanted)
    at += 1
    return found
  }
  const gap = (): void => line('')
  // The first line of the next block, past the blank line that ends this one.
  const next = (): string | undefined => (lines[at] === '' ? lines[at + 1] : undefined)
  // A part starts at line i: its heading, then its lens mark.
  const partAt = (i: number): boolean =>
    lines[i]?.startsWith('## ') === true && lines[i + 1] === '' && LENS_MARK.test(lines[i + 2] ?? '')
  // The fixed slide headed `heading` comes next, not a part of that name.
  const slide = (heading: string): boolean => next() === `## ${heading}` && !partAt(at + 1)
  const oneOf = <T extends string>(value: string, allowed: readonly string[], wanted: string): T =>
    allowed.includes(value) ? (value as T) : fail(wanted)

  const list = (): string[] => {
    const items: string[] = []
    while (lines[at]?.startsWith('- ')) items.push(take().slice(2))
    return items.length > 0 ? items : fail('a "- " line')
  }
  const block = (lang: '' | 'mermaid'): string => {
    const open = lang === 'mermaid' ? /^(`{3,})mermaid$/ : /^(`{3,})$/
    const bar = match(open, `a ${lang === '' ? 'code' : lang} fence`)[1] ?? ''
    const body: string[] = []
    while (lines[at] !== bar) body.push(take())
    at += 1
    return body.join('\n')
  }
  // A table's rows, each cell as written before `cell` escaped its pipes.
  const rows = (head: readonly string[]): string[][] => {
    line(headRow(head))
    line(`|${head.map(() => ' --- ').join('|')}|`)
    const out: string[][] = []
    while (lines[at]?.startsWith('| ')) {
      const row = lines[at] ?? ''
      const cells = row.endsWith(' |') ? row.slice(2, -2).split(' | ').map(c => c.replace(/\\\|/g, '|')) : []
      if (cells.length !== head.length) fail(`a row of ${head.length} cells`)
      out.push(cells)
      at += 1
    }
    return out.length > 0 ? out : fail('a table row')
  }
  const diagram = (): Diagram => ({ mermaid: block('mermaid') })
  const codePair = (): CodePair => {
    const file = match(/^### (.+)$/, 'a "### <file>" heading')[1] ?? ''
    gap()
    let before = ''
    if (lines[at] === '_New file._') at += 1
    else {
      line('Before:')
      gap()
      before = block('')
    }
    gap()
    let after = ''
    if (lines[at] === '_Removed._') at += 1
    else {
      line('After:')
      gap()
      after = block('')
    }
    return { file, before, after }
  }
  const part = (): Section => {
    const heading = match(/^## (.+)$/, 'a part heading')[1] ?? ''
    gap()
    const kind = match(LENS_MARK, 'a <!-- lens --> mark')[1]
    const s: Section = {
      ...(kind === undefined ? {} : { kind: oneOf<Lens>(kind, LENSES, 'a lens') }),
      heading,
      points: [],
    }
    if (/^_.+_$/.test(next() ?? '')) {
      gap()
      s.look_at = take().slice(1, -1)
    }
    if (/^`{3,}mermaid$/.test(next() ?? '')) {
      gap()
      s.diagram = diagram()
    }
    if (next() === 'Before:') {
      gap()
      line('Before:')
      gap()
      s.before = diagram()
      gap()
      line('After:')
      gap()
      s.after = diagram()
    }
    gap()
    s.points = list()
    while (next()?.startsWith('### ')) {
      gap()
      s.code = [...(s.code ?? []), codePair()]
    }
    if (next() === headRow(TOOL_HEAD)) {
      gap()
      s.tools = rows(TOOL_HEAD).map(([tool = '', how = '', swap = '', why = '']) => ({
        tool,
        how,
        swap: oneOf<Swap>(swap, SWAPS, 'easy, medium or hard'),
        why,
      }))
    }
    if (next() === '<details>') {
      gap()
      line('<details>')
      line(MORE)
      const body: string[] = []
      while (lines[at] !== '</details>') body.push(take())
      at += 1
      s.detail = body.join('\n')
    }
    return s
  }
  // A fixed slide's list, or none when the slide was left out.
  const optional = (heading: string): string[] => {
    if (!slide(heading)) return []
    gap()
    line(`## ${heading}`)
    gap()
    return list()
  }

  const title = match(/^# (.+)$/, 'the "# <title>" line')[1] ?? ''
  gap()
  const summary = take()
  gap()
  line('## What changes')
  gap()
  const changes = list()
  const unchanged = optional('What stays the same')
  const needs_your_eye = optional('Where your judgement is needed')
  const terms = optional('Words used here').map(entry => {
    const t = /^\*\*(.+?)\*\*: (.+)$/.exec(entry) ?? fail('a "**<term>**: <meaning>" line')
    return { term: t[1] ?? '', meaning: t[2] ?? '' }
  })
  const sections: Section[] = []
  while (lines[at] === '' && partAt(at + 1)) {
    gap()
    sections.push(part())
  }
  const files = !slide('Files')
    ? []
    : (gap(),
      line('## Files'),
      gap(),
      rows(FILE_HEAD).map(([change = '', path = '', why = '']) => ({
        change: oneOf<Change>(change, CHANGES, 'new, change or remove'),
        path: /^`.*`$/.test(path) ? path.slice(1, -1) : fail('a `path`'),
        why,
      })))
  gap()
  line('## Done when')
  gap()
  const checks = rows(CHECK_HEAD).map(([check = '', how = '']) => ({ check, how }))
  gap()
  line('## The plan')
  gap()
  const description = match(/^\*\*Description:\*\* (.*)$/, 'the "**Description:** " line')[1] ?? ''
  gap()
  const notes: string[] = []
  if (lines[at] === 'No notes.') at += 1
  else {
    for (let n = 1; lines[at]?.startsWith(`${n}. `); n += 1) notes.push(take().slice(`${n}. `.length))
    if (notes.length === 0) fail('the notes')
  }
  gap()
  if (at !== lines.length) fail('the end')
  return {
    report: { title, summary, changes, unchanged, needs_your_eye, terms, sections, files, checks },
    plan: { description, notes },
  }
}

// A report's twin, and what reads back from it, which the slides are built
// from.
export type Twin = ReadBack & { markdown: string }

// The twin and what reads back from it; or why it does not read back as the
// report it was written from, as for a plan note on two lines, and then the
// slides would not match it and nothing is written.
export const twinOf = (report: Report, plan: Plan): Twin | string => {
  const markdown = reportMarkdown(report, plan)
  let read: ReadBack
  try {
    read = reportFromMarkdown(markdown)
  } catch (error) {
    return `its Markdown twin does not read back (${error instanceof Error ? error.message : String(error)})`
  }
  return reportMarkdown(read.report, read.plan) === markdown
    ? { markdown, ...read }
    : 'its Markdown twin reads back as a different report'
}
```

Notes for the implementer:
- `gap()` at the end of the plan reads the `''` after the final `\n`, so `at === lines.length` means the twin ended where `reportMarkdown` ends it.
- `slide('Files')` after the parts loop: the loop stops at the first block that is not a part, so `## Files` here is the fixed slide.
- The comma expression in `files` keeps `files` a `const`; if a reviewer prefers, an `if` with `let files: FileChange[] = []` is equivalent (import `FileChange` then).

- [ ] **Step 4: Run the tests**

Run: `claude plugin test claude/refine-mod`
Expected: PASS, `0 fail`. If a round-trip test fails, the `toEqual` diff names the field; fix the reader, not the expectation.

- [ ] **Step 5: Commit**

```bash
git add claude/refine-mod/hooks/markdown.ts claude/refine-mod/tests/markdown.test.ts
git commit -m "$(cat <<'EOF'
refactor(refine): read a report back from its Markdown twin

The twin now keeps every field the slides use: each part's lens as a
hidden <!-- lens --> mark and More detail as a <details> block.
reportFromMarkdown reads it back into the report and plan it was
written from, and twinOf checks the read-back writes the same twin.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Build the page from the twin, and say so in the docs

**Files:**
- Modify: `claude/refine-mod/hooks/refine.ts:27` (import), `:371-379` (report mode), `:408-415` (refine mode)
- Test: `claude/refine-mod/tests/refine.test.ts`
- Modify: `README.md:306-308`, `CONTEXT.md:60-62`

**Interfaces:**
- Consumes: `twinOf(report, plan): Twin | string`, `Twin = { markdown, report, plan }` and `markdownPath` from `markdown.ts` (Task 2).

- [ ] **Step 1: Write the failing engine test**

In `claude/refine-mod/tests/refine.test.ts`, add after the test `'report mode: one report a session; the second names the first'`:

```ts
// The page is built from the twin as it reads back: a task note on two lines
// cannot be one numbered line of the twin, so nothing is written.
test('report mode: a plan whose twin would not read back writes nothing', { options: REPORT_MODE }, async ($, on) => {
  const torn = { ...PLANNED, annotations: [{ entry: '20260101T000001Z', description: 'first\nnote' }] }
  const { runs, writes } = world(on, SANDBOX, WRITE, [torn])
  await $.session.start(START)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })

  expect(shown.deny).toStartWith('show_task_report: nothing written: its Markdown twin does not read back (line ')
  expect(writes).toEqual([])
  expect(runs.map(run => run.argv.at(-1))).toEqual(['export'])
})
```

- [ ] **Step 2: Run the test to see it fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL — `shown.deny` is `undefined` (the report is written today), and `writes` holds the page and twin.

- [ ] **Step 3: Build both report paths from the twin**

In `claude/refine-mod/hooks/refine.ts`, replace

```ts
import { markdownPath, reportMarkdown } from './markdown'
```

with

```ts
import { markdownPath, twinOf } from './markdown'
```

In the report-mode branch, replace

```ts
      const shown = { description: task.description, notes: notesOf(task) }
      const path = reportPath(reports, uuid, await $.clock.now())
      await $.fs.write(path, reportPage(report, shown, await readFonts($, fonts), await drawDiagrams($, report)))
      // Its Markdown twin, which herdr's task-docs plugin opens in
      // plannotator-tui so comments on it reach the task's Claude.
      await $.fs.write(markdownPath(path), reportMarkdown(report, shown))
```

with

```ts
      const shown = { description: task.description, notes: notesOf(task) }
      // The page is built from the report's Markdown twin as it reads back,
      // never from the fields beside it, so the two cannot disagree.
      const twin = twinOf(report, shown)
      if (typeof twin === 'string') return { deny: `show_task_report: nothing written: ${twin}.` }
      const path = reportPath(reports, uuid, await $.clock.now())
      const page = reportPage(twin.report, twin.plan, await readFonts($, fonts), await drawDiagrams($, twin.report))
      await $.fs.write(path, page)
      // The twin itself, which herdr's task-docs plugin opens in
      // plannotator-tui so comments on it reach the task's Claude.
      await $.fs.write(markdownPath(path), twin.markdown)
```

In the refine-mode branch, replace

```ts
    const report = parseReport(e)
    if (typeof report === 'string') return { deny: `show_task_report: ${report}` }

    const shown = askedFor
    askedFor = undefined
    const path = reportPath(reports, uuid, await $.clock.now())
    await $.fs.write(path, reportPage(report, shown, await readFonts($, fonts), await drawDiagrams($, report)))
    await $.fs.write(markdownPath(path), reportMarkdown(report, shown))
```

with

```ts
    const report = parseReport(e)
    if (typeof report === 'string') return { deny: `show_task_report: ${report}` }
    const twin = twinOf(report, askedFor)
    if (typeof twin === 'string') return { deny: `show_task_report: nothing written: ${twin}.` }

    const shown = askedFor
    askedFor = undefined
    const path = reportPath(reports, uuid, await $.clock.now())
    const page = reportPage(twin.report, twin.plan, await readFonts($, fonts), await drawDiagrams($, twin.report))
    await $.fs.write(path, page)
    await $.fs.write(markdownPath(path), twin.markdown)
```

(`askedFor` is a `Plan` here: the `askedFor === undefined` check returns just above.)

- [ ] **Step 4: Run the tests**

Run: `claude plugin test claude/refine-mod`
Expected: PASS, `0 fail`; every existing report test still sees `[REPORT_PATH, REPORT_MD]` written in that order.

- [ ] **Step 5: Say it in the README and CONTEXT.md**

In `README.md`, replace

```
Each refine report is written twice, as its HTML slides and as `<report>.md`
beside them, from the same fields; the Markdown is what plannotator-tui
opens.
```

with

```
Each refine report is written first as `<report>.md`, its Markdown twin,
diagrams kept as Mermaid blocks; its HTML slides are then built from what
that twin reads back as, so the two never disagree. The Markdown is what
plannotator-tui opens.
```

In `CONTEXT.md`, in the **Refine report** entry, replace

```
holds. It is saved under the reviews folder, with a Markdown twin,
`<report>.md`, beside it for herdr's task-docs plugin to open in
plannotator-tui, and linked from the task's notes as `Report: <path>`; once
```

with

```
holds. It is saved under the reviews folder, with a Markdown twin,
`<report>.md`, beside it: the slides are built from the twin, and herdr's
task-docs plugin opens it in plannotator-tui. It is linked from the task's
notes as `Report: <path>`; once
```

- [ ] **Step 6: Commit**

```bash
git add claude/refine-mod/hooks/refine.ts claude/refine-mod/tests/refine.test.ts README.md CONTEXT.md
git commit -m "$(cat <<'EOF'
refactor(refine): build the report HTML from its Markdown twin

Both show_task_report paths now write the twin, read it back and build
the slides and diagrams from what was read, so the HTML and the .md
cannot disagree. A plan whose twin would not read back, such as a task
note on two lines, writes nothing and says why.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 7: Check a real report after landing**

The refine mod and the refine-task skill run from `~/Projects/niri-tasks`, never a worktree, so this check runs once the branch has landed through `finish-worktree`. On a pending task tagged `planned` whose plan has a diagram-worthy change:

Run: `niritasks task report <uuid> --fresh`
Expected: the report opens in a new browser window with its diagrams drawn (no Mermaid script fetched). Then, with `<report>` the new `Report:` note's path:

Run: `grep -c 'class="drawn"' <report>.html; grep -c '^```mermaid$' <report>.md; grep -c '^<!-- lens' <report>.md`
Expected: the first two counts are equal and non-zero (one drawn SVG per Mermaid block, unless a state diagram with marks was left to the browser), and the third equals the number of parts.
