# A Readable Refine Report, Linked From Its Task Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every refine report easy to take in for a reader with ADHD. The tool takes the report as fixed, length-limited fields and lays them out the same way every time: conclusion first, one idea per line, every diagram saying what to look at. Every report the person asks for is also linked from the task's notes when the task is written.

**Architecture:** `show_task_report` stops taking free HTML. It takes structured fields (`summary`, `changes`, `needs_your_eye`, `sections` with an optional diagram, `files`, `checks`, …), which `hooks/report.ts` checks against fixed limits, listing every problem at once. A new `hooks/page.ts` turns them into one fixed layout, under the same Content-Security-Policy and pinned Mermaid as before. The mod remembers each report it writes in the session, with the plan the person was shown when they asked for it. `write_task_plan` then adds a `Report: <path>` note per report, or `Report (earlier draft): <path>` when the plan changed since, and shows those notes with the rest before asking. The catalogue and the skill teach the session to fill the fields.

**Tech Stack:** TypeScript Claude Code mod (`claude plugin test`, `claude plugin validate`, Claude Code 2.1.295), Mermaid 11.17.2 (pinned, unchanged), bun (only to render the example page by hand), headless Chrome (render checks).

**Spec:** Taskwarrior task `691724fc-d9ff-408f-b00e-a9bcc693ef18` (`task rc.json.array=on 691724fc-d9ff-408f-b00e-a9bcc693ef18 export`), plus two requirements the user added in conversation on 2026-10-09:
- "I want these reports to be really easy for people with adhd to understand."
- "make sure any generated reports are linked in the notes of the niritask task".

This plan follows `docs/superpowers/plans/2026-10-09-refine-report.md`, which is fully built on this branch. Read its "Background for the implementer" for the mod, the sandbox and why the page's policy exists.

## Global Constraints

- The report is still on demand only, and still one report per time the person chooses **Show me a report first**. That logic in `refine.ts` is kept as it is.
- The page's security is unchanged: the same `CSP`, `MERMAID_URL` and `MERMAID_SRI`, and the same refused elements. The refused-element check now covers each `detail`, each SVG and each Mermaid source.
- The limits are exactly these (the `LIMITS` constant):
  - `title` 120 characters; `summary` 35 words;
  - `changes` 1–3 lines of 20 words; `unchanged` 0–3 lines of 20 words; `needs_your_eye` 0–3 lines of 30 words;
  - `terms` 0–5, each term 4 words and its meaning 20 words;
  - `sections` 1–6, each heading 10 words and `look_at` 25 words;
  - `points` 1–5 per section, 25 words each;
  - a Mermaid diagram 40 lines, a diagram 50,000 characters, a `detail` 20,000 characters;
  - `files` 1–40, each path 200 characters and `why` 20 words;
  - `checks` 1–6, 20 words each.
- Every text field except `detail` and the diagram is one line of plain text, with no control or invisible characters.
- The page layout, in this order:
  1. a header (the "Refine report" kicker, the title, and "About N min to read · P parts · F files");
  2. a sticky contents bar;
  3. **At a glance**: the summary, "What changes", then "What stays the same" and "Needs your eye" when present, then the next step;
  4. **Words used here**, when terms are given;
  5. the diagram legend, when any part has a diagram;
  6. the numbered parts;
  7. **Files**;
  8. **Done when**;
  9. the next step again, in the footer.
- The next step reads exactly: `Back in the terminal: choose <strong>Write it to the task</strong> to save this plan, or type what to change.`
- Typography: 18px text, line height 1.6, text column at most 40rem (about 75 characters a line), left-aligned, no italics.
- Change colours are the same in the badges and the diagrams: `new` green, `change` amber, `remove` red with a dashed border. A colour always comes with a text label.
- Report notes read exactly `Report: <path>`, or `Report (earlier draft): <path>` when the plan written differs from the plan the person was shown when they asked for that report. They go after the plan's own notes, in the order the reports were made, and a path the notes already link is not linked again.
- Commits: Conventional Commits, scope `refine`, subject ≤ 72 characters, each ending with a `Co-Authored-By:` trailer naming the model that wrote it.

## Why these choices

The research behind the layout, so a reviewer can check the reasoning:

- **Fixed layout, conclusion first, a summary.** W3C's cognitive-accessibility design guide (`https://www.w3.org/TR/coga-usable/`) calls for a clear purpose, a consistent visual design, the key information easy to find, and a summary of long content. It names AD(H)D readers in its headings and breadcrumbs patterns. Nielsen Norman Group found that concise, scannable, objective text was 124% more usable (`https://www.nngroup.com/articles/how-users-read-on-the-web/`).
- **Few things at once.** Working memory holds about four chunks (Cowan, 2001). Hence at most three changes, three "needs your eye" lines and five points a part.
- **Signals and pictures.** Headings and other signals improve memory for what they mark, especially in complex texts (Lorch). Mayer's multimedia principles favour words with pictures, nothing extraneous, signalling, labels placed by the picture, user-paced segments, and key terms introduced first. Hence "Look at:" above every diagram, the "Words used here" list, the numbered parts and the collapsed detail.
- **Small diagrams, few symbols.** Moody's "Physics of Notations" favours modular diagrams, few distinct symbols and text alongside graphics. Hence one idea per diagram, about nine boxes, and three change classes.
- **Line length and type.** WCAG 1.4.8 asks for at most 80 characters a line, 1.5 line spacing and no justified text. The UK Home Office posters say no italics, no walls of text, a consistent layout, and no making people remember things from earlier.
- **Evidence specific to ADHD is thin.** The Swedish "Understandable Text" work found ADHD readers prefer headings that describe content, bullet lists and summaries. Bionic Reading showed no effect on speed in a 2,074-reader test, so it is not used. The last task puts the example in front of the actual reader for exactly this reason.

## File Structure

- `claude/refine-mod/hooks/report.ts` (rewrite): the report's types, `LIMITS`, `parseReport`, `reportsDir`, `reportPath`, `openArgv`, and from Task 2 `MadeReport` and `reportNotes`. It no longer holds the page.
- `claude/refine-mod/hooks/page.ts` (new): `MERMAID_URL`, `MERMAID_SRI` and `CSP` (moved from `report.ts`), the stylesheet, `inline`, `readingMinutes` and `reportPage`.
- `claude/refine-mod/hooks/refine.ts` (modify): the report tool's input schema and the import of `reportPage`; from Task 2, the remembered reports and the report notes.
- `claude/refine-mod/tests/example-report.ts` (new): the worked example, which tests render and the catalogue copies.
- `claude/refine-mod/tests/report.test.ts` (rewrite), `claude/refine-mod/tests/page.test.ts` (new) and `claude/refine-mod/tests/refine.test.ts` (modify).
- `.claude/skills/refine-task/report-catalogue.md` (rewrite) and `.claude/skills/refine-task/SKILL.md` (modify).
- `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` (a dated note) and `CONTEXT.md` (a glossary entry).

---

### Task 1: A fixed, readable report page

**Files:**
- Rewrite: `claude/refine-mod/hooks/report.ts`
- Create: `claude/refine-mod/hooks/page.ts`
- Create: `claude/refine-mod/tests/example-report.ts`
- Rewrite: `claude/refine-mod/tests/report.test.ts`
- Create: `claude/refine-mod/tests/page.test.ts`
- Modify: `claude/refine-mod/hooks/refine.ts` (the `./report` import, `REPORT_SPEC`)
- Modify: `claude/refine-mod/tests/refine.test.ts` (the report calls)

**Interfaces:**
- Consumes: `HIDDEN` and `WRITE` from `./plan`.
- Produces:
  - from `hooks/report.ts`: `type Change = 'new' | 'change' | 'remove'`, `type Diagram = { mermaid: string } | { svg: string }`, `type Section = { heading: string; look_at?: string; diagram?: Diagram; points: string[]; detail?: string }`, `type Term`, `type FileChange`, `type Check`, `type Report`, `LIMITS`, `parseReport(e: Record<string, unknown>): Report | string`, plus `reportsDir`, `reportPath` and `openArgv` unchanged;
  - from `hooks/page.ts`: `MERMAID_URL`, `MERMAID_SRI`, `CSP`, `inline(text: string): string`, `readingMinutes(report: Report): number` and `reportPage(report: Report): string`;
  - from `tests/example-report.ts`: `EXAMPLE`.

- [ ] **Step 1: Write the worked example**

Create `claude/refine-mod/tests/example-report.ts`:

```ts
// The report catalogue's worked example, as show_task_report takes it: this
// very feature, explained. Tests render it; keep the catalogue's copy in step.
export const EXAMPLE = {
  title: 'feat: Let a refine show an HTML report of its plan',
  summary:
    'Before you approve a refined task, you can ask for a page that explains the plan with pictures, then approve or change it as before.',
  changes: [
    'New answer: the question gains "Show me a report first", which writes nothing to the task.',
    'New page: the report opens in your browser and is saved under reviews, never in /tmp.',
    'New note: the task keeps a link to each report made while refining it.',
  ],
  unchanged: [
    'Approval: the task is still written only when you choose Write it to the task.',
    'Every refine: no report is made unless you ask for one.',
  ],
  needs_your_eye: [
    'Diagrams need the internet: offline, Mermaid diagrams show as their source text.',
    'Script: the page runs no script but pinned Mermaid, so the model cannot act in your browser.',
  ],
  terms: [
    { term: 'Refine', meaning: 'Claude turning a terse task into a plan, then writing it back.' },
    { term: 'The mod', meaning: 'Code inside Claude Code that asks you before writing the task.' },
    { term: 'Mermaid', meaning: 'A tool that draws diagrams from short text descriptions.' },
  ],
  sections: [
    {
      heading: 'What you will see',
      look_at: 'The new middle answer, and the loop back to the same question.',
      diagram: {
        mermaid: [
          'flowchart TD',
          '  ask["Write this to the task?"] --> write["Write it to the task"]',
          '  ask --> report["Show me a report first"]:::new',
          '  report --> page["Report opens in your browser"]:::new',
          '  page --> ask',
          '  ask --> change["Change something"]',
        ].join('\n'),
      },
      points: [
        'Choose the report: nothing is written yet.',
        'Read the page: it opens by itself in your browser.',
        'Back in the terminal: the same question comes back.',
      ],
    },
    {
      heading: 'How the report is made',
      look_at: 'The mod, not Claude, writes the page and opens it.',
      diagram: {
        mermaid: [
          'sequenceDiagram',
          '  actor You',
          '  participant C as Claude',
          '  participant M as The mod',
          '  C->>M: write_task_plan(plan)',
          '  M->>You: Write this to the task?',
          '  rect rgb(220, 252, 231)',
          '  You-->>M: Show me a report first',
          '  M-->>C: Build the report',
          '  C->>M: show_task_report(report)',
          '  M->>You: Opens the page',
          '  end',
          '  C->>M: write_task_plan(plan)',
        ].join('\n'),
      },
      points: [
        'Claude fills in fixed fields: summary, changes, diagrams, files and checks.',
        'The mod builds the page, so every report has the same layout.',
        "The page runs no script of Claude's: only the pinned Mermaid library.",
      ],
    },
    {
      heading: 'What the task remembers',
      points: [
        'Report note: each report becomes a note, Report: followed by its path.',
        'Earlier draft: a report made before you changed the plan is marked as an earlier draft.',
        'You approve it: the note is shown with the plan before you choose.',
      ],
    },
  ],
  files: [
    { path: 'claude/refine-mod/hooks/report.ts', change: 'change', why: "Checks the report's fields and their length limits." },
    { path: 'claude/refine-mod/hooks/page.ts', change: 'new', why: 'Builds the page in one fixed, readable layout.' },
    { path: 'claude/refine-mod/hooks/refine.ts', change: 'change', why: 'Adds the answer, the report tool and the report notes.' },
    { path: '.claude/skills/refine-task/SKILL.md', change: 'change', why: 'Tells Claude when and how to make a report.' },
    { path: '.claude/skills/refine-task/report-catalogue.md', change: 'change', why: 'Explains how to write a report that is easy to read.' },
  ],
  checks: [
    { check: 'Choosing the report opens a page in the browser', how: 'A live refine, by hand' },
    { check: 'Approving afterwards writes the task with a report note', how: 'A live refine, then `task export`' },
    { check: "The mod's tests pass", how: '`claude plugin test claude/refine-mod`' },
  ],
}
```

- [ ] **Step 2: Write the failing tests for the fields**

Replace the whole of `claude/refine-mod/tests/report.test.ts` with:

```ts
import { describe, expect, test } from 'claude-code/testing'
import { LIMITS, openArgv, parseReport, reportPath, reportsDir } from '../hooks/report'
import { EXAMPLE } from './example-report'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'

// The smallest report the tool takes.
const MINIMAL = {
  title: 'feat: New words',
  summary: 'The task gets new words.',
  changes: ['Words: the description changes.'],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
}

const words = (n: number) => Array.from({ length: n }, () => 'word').join(' ')

// MINIMAL with one part that carries this diagram and a look_at line.
const drawn = (diagram: unknown) => ({
  ...MINIMAL,
  sections: [{ heading: 'h', points: ['p'], look_at: 'x', diagram }],
})

// MINIMAL with one part that carries this detail.
const detailed = (detail: string) => ({
  ...MINIMAL,
  sections: [{ heading: 'h', points: ['p'], detail }],
})

describe('parseReport', () => {
  test('takes the smallest report, with the optional lists empty', () => {
    expect(parseReport(MINIMAL)).toEqual({ ...MINIMAL, unchanged: [], needs_your_eye: [], terms: [] })
  })

  test('takes the worked example', () => {
    expect(typeof parseReport(EXAMPLE)).toBe('object')
  })

  test('refuses a missing field by name', () => {
    const { summary: _, ...rest } = MINIMAL
    expect(parseReport(rest)).toBe('summary must be a non-empty string')
  })

  test('refuses text over its word limit, saying the count and the limit', () => {
    expect(parseReport({ ...MINIMAL, summary: words(36) })).toBe('summary has 36 words; at most 35')
    expect(parseReport({ ...MINIMAL, changes: [words(21)] })).toBe('changes[0] has 21 words; at most 20')
  })

  test('refuses too many or too few lines', () => {
    expect(parseReport({ ...MINIMAL, changes: [] })).toBe('changes must have 1 to 3 items, not 0')
    expect(parseReport({ ...MINIMAL, needs_your_eye: ['a', 'b', 'c', 'd'] })).toBe(
      'needs_your_eye must have 0 to 3 items, not 4',
    )
  })

  test('refuses hidden characters and line breaks in plain text', () => {
    expect(parseReport({ ...MINIMAL, title: 'a‮b' })).toBe(
      'title must be plain text: no control or invisible characters',
    )
    expect(parseReport({ ...MINIMAL, title: 'a\nb' })).toBe('title must be one line')
  })

  test('lists every problem at once', () => {
    const answer = parseReport({
      ...MINIMAL,
      summary: 'two\nlines',
      changes: [],
      files: [{ path: 'a', change: 'edit', why: 'x' }],
    })
    expect(answer).toBe(
      'summary must be one line; changes must have 1 to 3 items, not 0; files[0].change must be new, change or remove',
    )
  })

  test('caps a long list of problems at twenty', () => {
    const files = Array.from({ length: 25 }, () => ({ path: 'a', change: 'x', why: 'y' }))
    expect(parseReport({ ...MINIMAL, files })).toEndWith('; and 5 more')
  })

  test('a diagram needs a look_at line, and a look_at line needs a diagram', () => {
    const flow = { mermaid: 'flowchart LR\n a --> b' }
    expect(parseReport({ ...MINIMAL, sections: [{ heading: 'h', points: ['p'], diagram: flow }] })).toBe(
      'sections[0].look_at must be a non-empty string',
    )
    expect(parseReport({ ...MINIMAL, sections: [{ heading: 'h', points: ['p'], look_at: 'the arrow' }] })).toBe(
      'sections[0].look_at needs a diagram to point at',
    )
  })

  test('a diagram is Mermaid or SVG, not both, and not too long', () => {
    expect(parseReport(drawn({ mermaid: 'a', svg: '<svg></svg>' }))).toBe(
      'sections[0].diagram must have exactly one of mermaid or svg',
    )
    const long = Array.from({ length: LIMITS.mermaidLines + 1 }, () => 'a --> b').join('\n')
    expect(parseReport(drawn({ mermaid: long }))).toBe(
      `sections[0].diagram.mermaid has ${LIMITS.mermaidLines + 1} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`,
    )
    expect(parseReport(drawn({ svg: '<p>no</p>' }))).toBe('sections[0].diagram.svg must be one <svg> element')
    expect(typeof parseReport(drawn({ svg: '<svg viewBox="0 0 1 1"><rect/></svg>' }))).toBe('object')
  })

  test('refuses every element the page must not hold, in detail, SVG and Mermaid', () => {
    const tags = ['script', 'meta', 'base', 'link', 'iframe', 'frame', 'frameset', 'object', 'embed', 'form', 'portal']
    for (const tag of tags) {
      expect(parseReport(detailed(`<p>x</p><${tag} a="b">`))).toBe(
        `sections[0].detail must not contain <${tag}>: the page runs no script but Mermaid, loads nothing and sends nothing`,
      )
    }
    for (const detail of ['<script/src=x>', '<script\n>', '< SCRIPT>']) {
      expect(parseReport(detailed(detail))).toContain('must not contain <script>')
    }
    expect(parseReport(drawn({ svg: '<svg><script>x</script></svg>' }))).toContain(
      'sections[0].diagram.svg must not contain <script>',
    )
    expect(parseReport(drawn({ mermaid: 'flowchart LR\n a["< META http-equiv=refresh>"]' }))).toContain(
      'sections[0].diagram.mermaid must not contain <meta>',
    )
  })

  test('keeps words that only begin like a refused element', () => {
    for (const detail of ['<p>a <code>&lt;script&gt;</code></p>', '<formula>x</formula>', '<p>metadata</p>', '<baseline/>']) {
      expect(typeof parseReport(detailed(detail))).toBe('object')
    }
  })
})

describe('reportsDir', () => {
  test('takes an absolute folder, without a trailing slash', () => {
    expect(reportsDir('/home/x/.local/share/niri-tasks/reviews')).toBe('/home/x/.local/share/niri-tasks/reviews')
    expect(reportsDir('/r/')).toBe('/r')
  })
  test('refuses the rest', () => {
    for (const value of [undefined, '', 'reviews', '~/reviews', '/', 7]) expect(reportsDir(value)).toBeUndefined()
  })
})

test('reportPath names the task and the UTC time', () => {
  expect(reportPath('/r', UUID, Date.UTC(2026, 9, 6, 1, 2, 3, 456))).toBe('/r/refine-0b8f6a52-20261006-010203.html')
  expect(reportPath('/r', UUID.toUpperCase(), 0)).toBe('/r/refine-0b8f6a52-19700101-000000.html')
})

test('openArgv passes the path as an argument, never as script', () => {
  const path = '/r/it\'s "here" $(x).html'
  expect(openArgv(path)).toEqual(['sh', '-c', 'xdg-open "$1" >/dev/null 2>&1 </dev/null &', 'sh', path])
})
```

- [ ] **Step 3: Write the failing tests for the page**

Create `claude/refine-mod/tests/page.test.ts`:

```ts
import { describe, expect, test } from 'claude-code/testing'
import { CSP, MERMAID_SRI, MERMAID_URL, inline, readingMinutes, reportPage } from '../hooks/page'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'
import { EXAMPLE } from './example-report'

const report = (over: Partial<Report> = {}): Report => ({
  title: 'feat: <A> & "B"',
  summary: 'The task gets <new> words.',
  changes: ['Words: the description changes.'],
  unchanged: [],
  needs_your_eye: [],
  terms: [],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
  ...over,
})

const words = (n: number) => Array.from({ length: n }, () => 'word').join(' ')
const NEXT = 'Back in the terminal: choose <strong>Write it to the task</strong> to save this plan, or type what to change.'

describe('the head', () => {
  const page = reportPage(report())

  test('puts the policy first, before anything it governs', () => {
    const csp = page.indexOf(`<meta http-equiv="Content-Security-Policy" content="${CSP}">`)
    expect(csp).toBeGreaterThan(-1)
    expect(csp).toBeLessThan(page.indexOf('<title>'))
    expect(csp).toBeLessThan(page.indexOf('<style>'))
    expect(csp).toBeLessThan(page.indexOf('<script'))
    expect(csp).toBeLessThan(page.indexOf('<body>'))
  })

  test('allows no script but pinned Mermaid, and nothing fetched or sent', () => {
    expect(CSP).toBe(
      `default-src 'none'; script-src ${MERMAID_URL}; style-src 'unsafe-inline'; ` +
        "img-src data:; font-src data:; form-action 'none'; base-uri 'none'",
    )
    expect(MERMAID_URL).toBe('https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js')
    expect(page).toContain(`<script src="${MERMAID_URL}" integrity="${MERMAID_SRI}" crossorigin="anonymous"></script>`)
  })

  test("escapes the model's text", () => {
    expect(page).toContain('<title>feat: &lt;A&gt; &amp; &quot;B&quot;</title>')
    expect(page).toContain('<h1>feat: &lt;A&gt; &amp; &quot;B&quot;</h1>')
    expect(page).toContain('The task gets &lt;new&gt; words.')
  })
})

describe('the layout', () => {
  test('glance, parts, files, checks, in that order, and the next step at both ends', () => {
    const page = reportPage(report())
    const at = (id: string) => page.indexOf(`id="${id}"`)
    expect(at('glance')).toBeGreaterThan(-1)
    expect(at('glance')).toBeLessThan(at('part-1'))
    expect(at('part-1')).toBeLessThan(at('files'))
    expect(at('files')).toBeLessThan(at('checks'))
    expect(page.split(NEXT).length - 1).toBe(2)
  })

  test('says how long it takes and how much there is', () => {
    expect(reportPage(report())).toContain('<p class="meta">About 1 min to read · 1 part · 1 file</p>')
  })

  test('lists the parts, numbered, in the contents bar', () => {
    const page = reportPage(report({ sections: [{ heading: 'One', points: ['a'] }, { heading: 'Two', points: ['b'] }] }))
    expect(page).toContain('<a href="#part-1">1. One</a>')
    expect(page).toContain('<a href="#part-2">2. Two</a>')
    expect(page).toContain('<span class="num">2/2</span> Two')
  })

  test('shows the optional blocks only when given', () => {
    const bare = reportPage(report())
    for (const text of ['What stays the same', 'Needs your eye', 'Words used here', 'class="legend"']) {
      expect(bare).not.toContain(text)
    }
    const full = reportPage(
      report({
        unchanged: ['Approval: still asked.'],
        needs_your_eye: ['Offline: diagrams show as text.'],
        terms: [{ term: 'Refine', meaning: 'Turning a task into a plan.' }],
        sections: [{ heading: 'h', points: ['p'], look_at: 'x', diagram: { mermaid: 'flowchart LR\n a --> b' } }],
      }),
    )
    for (const text of ['What stays the same', 'Needs your eye', 'Words used here', 'class="legend"']) {
      expect(full).toContain(text)
    }
    expect(full).toContain('<dt>Refine</dt><dd>Turning a task into a plan.</dd>')
  })

  test('shows the files with a labelled badge each', () => {
    expect(reportPage(report())).toContain(
      '<tr><td><span class="badge change">changed</span></td><td><code>src/words.rs</code></td><td>Holds the words.</td></tr>',
    )
  })
})

describe('diagrams', () => {
  const part = (diagram: { mermaid: string } | { svg: string }) =>
    reportPage(report({ sections: [{ heading: 'h', points: ['p'], look_at: 'The new box.', diagram }] }))

  test('say what to look at, just above the drawing', () => {
    const page = part({ mermaid: 'flowchart LR\n a --> b' })
    const look = page.indexOf('<figcaption class="look"><strong>Look at:</strong> The new box.</figcaption>')
    expect(look).toBeGreaterThan(-1)
    expect(look).toBeLessThan(page.indexOf('<pre class="mermaid">'))
  })

  test('escape Mermaid source, and give flowcharts and state diagrams the change colours', () => {
    const flow = part({ mermaid: 'flowchart LR\n a["x"] --> b' })
    expect(flow).toContain('a[&quot;x&quot;] --&gt; b')
    expect(flow).toContain('classDef new fill:#dcfce7')
    expect(part({ mermaid: 'stateDiagram-v2\n [*] --> A' })).toContain('classDef change fill:#fef3c7')
    expect(part({ mermaid: 'sequenceDiagram\n A->>B: hi' })).not.toContain('classDef')
  })

  test('pass SVG through as given', () => {
    expect(part({ svg: '<svg viewBox="0 0 1 1"><rect/></svg>' })).toContain('<svg viewBox="0 0 1 1"><rect/></svg>')
  })

  test('put detail in a collapsed block', () => {
    const page = reportPage(report({ sections: [{ heading: 'h', points: ['p'], detail: '<p>deep</p>' }] }))
    expect(page).toContain('<details>\n<summary>More detail</summary>\n<p>deep</p>\n</details>')
  })
})

describe('inline', () => {
  test('bolds a short label at the start', () => {
    expect(inline('Label: rest')).toBe('<strong>Label:</strong> rest')
  })
  test('shows backticks as code, and escapes the rest', () => {
    expect(inline('run `task export` now')).toBe('run <code>task export</code> now')
    expect(inline('a <b> & c')).toBe('a &lt;b&gt; &amp; c')
  })
  test('leaves a colon that is not a label alone', () => {
    expect(inline('see https://x.y: it')).toBe('see https://x.y: it')
  })
})

describe('readingMinutes', () => {
  test('is at least one', () => {
    expect(readingMinutes(report())).toBe(1)
  })
  test('counts detail at 200 words a minute', () => {
    expect(readingMinutes(report({ sections: [{ heading: 'h', points: ['p'], detail: `<p>${words(600)}</p>` }] }))).toBe(4)
  })
})

test('the worked example renders every part', () => {
  const parsed = parseReport(EXAMPLE)
  if (typeof parsed === 'string') throw new Error(parsed)
  const page = reportPage(parsed)
  for (const section of parsed.sections) expect(page).toContain(section.heading)
  expect(page.split('<pre class="mermaid">').length - 1).toBe(2)
})
```

In `readingMinutes`'s second test the words are 600 from the detail, plus 'h' and 'p' and the rest of `report()`'s text (about 18), so 618 / 200 rounds up to 4.

- [ ] **Step 4: Run them to verify they fail**

Run: `claude plugin test claude/refine-mod`
Expected: `report.test.ts` fails (no `LIMITS` export, and `parseReport` still takes `title` and `body`), and `page.test.ts` fails to load (`../hooks/page` does not exist).

- [ ] **Step 5: Rewrite report.ts**

Replace the whole of `claude/refine-mod/hooks/report.ts` with:

```ts
// The pure half of show_task_report: the fields it takes from the model, the
// limits that keep a report short enough to take in, and where it is saved.
// No `$`, so the unit tests reach it directly.

import { HIDDEN } from './plan'

export type Change = 'new' | 'change' | 'remove'
export type Diagram = { mermaid: string } | { svg: string }
export type Section = {
  heading: string
  look_at?: string
  diagram?: Diagram
  points: string[]
  detail?: string
}
export type Term = { term: string; meaning: string }
export type FileChange = { path: string; change: Change; why: string }
export type Check = { check: string; how: string }

export type Report = {
  title: string
  summary: string
  changes: string[]
  unchanged: string[]
  needs_your_eye: string[]
  terms: Term[]
  sections: Section[]
  files: FileChange[]
  checks: Check[]
}

// What keeps a report easy to take in: about four things held in mind at
// once, one idea per line, short plain sentences. A report over them is
// refused with every reason at once, so it is fixed in one go.
export const LIMITS = {
  titleChars: 120,
  summaryWords: 35,
  changes: { min: 1, max: 3, words: 20 },
  unchanged: { min: 0, max: 3, words: 20 },
  needsYourEye: { min: 0, max: 3, words: 30 },
  terms: { min: 0, max: 5, termWords: 4, meaningWords: 20 },
  sections: { min: 1, max: 6, headingWords: 10, lookAtWords: 25 },
  points: { min: 1, max: 5, words: 25 },
  mermaidLines: 40,
  diagramChars: 50_000,
  detailChars: 20_000,
  files: { min: 1, max: 40, pathChars: 200, whyWords: 20 },
  checks: { min: 1, max: 6, words: 20 },
} as const

const CHANGES: readonly string[] = ['new', 'change', 'remove']

// Elements refused by name: a <meta> refresh navigates and a <meta> policy
// could be added to, which the page's own policy does not stop; the rest would
// reach outside or run script, and fail quietly under it. Refused so the model
// hears why rather than finding a blank figure.
const FORBIDDEN = /<\s*(script|meta|base|link|iframe|frameset|frame|object|embed|form|portal)(?![\w-])/i

// How many problems one refusal lists before it says how many more.
const MAX_ERRORS = 20

type Errors = string[]
type Count = { readonly min: number; readonly max: number }

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value)

const wordCount = (text: string): number => text.split(/\s+/).filter(Boolean).length

// One line of plain text within its limit. A problem is noted, never thrown,
// so the refusal can list them all.
const text = (errors: Errors, at: string, value: unknown, limit: { words?: number; chars?: number }): string => {
  if (typeof value !== 'string' || value.trim() === '') {
    errors.push(`${at} must be a non-empty string`)
    return ''
  }
  if (/[\r\n]/.test(value)) errors.push(`${at} must be one line`)
  else if (HIDDEN.test(value)) errors.push(`${at} must be plain text: no control or invisible characters`)
  const n = wordCount(value)
  if (limit.words !== undefined && n > limit.words) errors.push(`${at} has ${n} words; at most ${limit.words}`)
  if (limit.chars !== undefined && value.length > limit.chars) {
    errors.push(`${at} has ${value.length} characters; at most ${limit.chars}`)
  }
  return value
}

// Markup the page shows as given: within its size, and holding nothing the
// page must not.
const markup = (errors: Errors, at: string, value: unknown, maxChars: number): string => {
  if (typeof value !== 'string' || value.trim() === '') {
    errors.push(`${at} must be a non-empty string`)
    return ''
  }
  if (value.length > maxChars) errors.push(`${at} has ${value.length} characters; at most ${maxChars}`)
  const tag = FORBIDDEN.exec(value)?.[1]?.toLowerCase()
  if (tag !== undefined) {
    errors.push(`${at} must not contain <${tag}>: the page runs no script but Mermaid, loads nothing and sends nothing`)
  }
  return value
}

// A list within its count; an optional list may be left out.
const list = <T>(
  errors: Errors,
  at: string,
  value: unknown,
  count: Count,
  item: (value: unknown, at: string) => T,
): T[] => {
  if (value === undefined && count.min === 0) return []
  if (!Array.isArray(value)) {
    errors.push(`${at} must be an array`)
    return []
  }
  if (value.length < count.min || value.length > count.max) {
    errors.push(`${at} must have ${count.min} to ${count.max} items, not ${value.length}`)
  }
  return value.map((v, i) => item(v, `${at}[${i}]`))
}

const term = (errors: Errors, at: string, value: unknown): Term => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { term: '', meaning: '' }
  }
  return {
    term: text(errors, `${at}.term`, value.term, { words: LIMITS.terms.termWords }),
    meaning: text(errors, `${at}.meaning`, value.meaning, { words: LIMITS.terms.meaningWords }),
  }
}

const diagram = (errors: Errors, at: string, value: unknown): Diagram | undefined => {
  if (!isObject(value) || ('mermaid' in value) === ('svg' in value)) {
    errors.push(`${at} must have exactly one of mermaid or svg`)
    return undefined
  }
  if ('mermaid' in value) {
    const source = markup(errors, `${at}.mermaid`, value.mermaid, LIMITS.diagramChars)
    const lines = source.split('\n').length
    if (lines > LIMITS.mermaidLines) {
      errors.push(`${at}.mermaid has ${lines} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`)
    }
    return { mermaid: source }
  }
  const svg = markup(errors, `${at}.svg`, value.svg, LIMITS.diagramChars)
  if (svg !== '' && !/^\s*<svg[\s>][\s\S]*<\/svg>\s*$/i.test(svg)) errors.push(`${at}.svg must be one <svg> element`)
  return { svg }
}

const section = (errors: Errors, at: string, value: unknown): Section => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { heading: '', points: [] }
  }
  const part: Section = {
    heading: text(errors, `${at}.heading`, value.heading, { words: LIMITS.sections.headingWords }),
    points: list(errors, `${at}.points`, value.points, LIMITS.points, (v, i) =>
      text(errors, i, v, { words: LIMITS.points.words }),
    ),
  }
  if (value.diagram !== undefined) {
    const drawn = diagram(errors, `${at}.diagram`, value.diagram)
    if (drawn !== undefined) part.diagram = drawn
    part.look_at = text(errors, `${at}.look_at`, value.look_at, { words: LIMITS.sections.lookAtWords })
  } else if (value.look_at !== undefined) {
    errors.push(`${at}.look_at needs a diagram to point at`)
  }
  if (value.detail !== undefined) part.detail = markup(errors, `${at}.detail`, value.detail, LIMITS.detailChars)
  return part
}

const file = (errors: Errors, at: string, value: unknown): FileChange => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { path: '', change: 'change', why: '' }
  }
  const path = text(errors, `${at}.path`, value.path, { chars: LIMITS.files.pathChars })
  const change = value.change
  if (typeof change !== 'string' || !CHANGES.includes(change)) errors.push(`${at}.change must be new, change or remove`)
  const why = text(errors, `${at}.why`, value.why, { words: LIMITS.files.whyWords })
  return { path, change: change as Change, why }
}

const check = (errors: Errors, at: string, value: unknown): Check => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { check: '', how: '' }
  }
  return {
    check: text(errors, `${at}.check`, value.check, { words: LIMITS.checks.words }),
    how: text(errors, `${at}.how`, value.how, { words: LIMITS.checks.words }),
  }
}

// Answers the report, or every problem with it, joined.
export const parseReport = (e: Record<string, unknown>): Report | string => {
  const errors: Errors = []
  const lines = (at: string, value: unknown, count: Count & { readonly words: number }): string[] =>
    list(errors, at, value, count, (v, i) => text(errors, i, v, { words: count.words }))
  const report: Report = {
    title: text(errors, 'title', e.title, { chars: LIMITS.titleChars }),
    summary: text(errors, 'summary', e.summary, { words: LIMITS.summaryWords }),
    changes: lines('changes', e.changes, LIMITS.changes),
    unchanged: lines('unchanged', e.unchanged, LIMITS.unchanged),
    needs_your_eye: lines('needs_your_eye', e.needs_your_eye, LIMITS.needsYourEye),
    terms: list(errors, 'terms', e.terms, LIMITS.terms, (v, at) => term(errors, at, v)),
    sections: list(errors, 'sections', e.sections, LIMITS.sections, (v, at) => section(errors, at, v)),
    files: list(errors, 'files', e.files, LIMITS.files, (v, at) => file(errors, at, v)),
    checks: list(errors, 'checks', e.checks, LIMITS.checks, (v, at) => check(errors, at, v)),
  }
  if (errors.length === 0) return report
  const more = errors.length > MAX_ERRORS ? `; and ${errors.length - MAX_ERRORS} more` : ''
  return errors.slice(0, MAX_ERRORS).join('; ') + more
}

// The folder the reports go in, from the session's settings: absolute, or
// none, and then the tool is not offered.
export const reportsDir = (value: unknown): string | undefined => {
  if (typeof value !== 'string' || !value.startsWith('/')) return undefined
  const dir = value.replace(/\/+$/, '')
  return dir === '' ? undefined : dir
}

// <dir>/refine-<uuid8>-<YYYYMMDD-HHMMSS>.html in UTC: a second report on the
// same task, after a revision, does not overwrite the first.
export const reportPath = (dir: string, uuid: string, nowMs: number): string => {
  const stamp = new Date(nowMs).toISOString().slice(0, 19).replace(/-|:/g, '').replace('T', '-')
  return `${dir}/refine-${uuid.slice(0, 8).toLowerCase()}-${stamp}.html`
}

// xdg-open in the background with its streams closed: a browser it starts
// would otherwise hold `$.process.run` until its timeout. The path is `$1`,
// an argument, never part of the script.
export const openArgv = (path: string): string[] => [
  'sh',
  '-c',
  'xdg-open "$1" >/dev/null 2>&1 </dev/null &',
  'sh',
  path,
]
```

- [ ] **Step 6: Write page.ts**

Create `claude/refine-mod/hooks/page.ts`:

```ts
// The page a report is written as: the mod's own head, whose policy lets the
// report run no script, then the report's fields in one fixed layout, so
// every report reads the same way: what it does and what to decide first,
// detail last. No `$`, so the unit tests reach it directly.

import { WRITE } from './plan'
import type { Change, Check, Diagram, FileChange, Report, Section, Term } from './report'

// Mermaid's own bundle, pinned and checked: it renders every
// `<pre class="mermaid">` on load, so the page needs no script of its own.
// Mermaid decodes entities in a block before sanitizing, so its sanitizer is
// the only barrier against an entity-encoded <meta> or <base> inside one:
// re-check that on any version bump.
export const MERMAID_URL = 'https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js'
export const MERMAID_SRI = 'sha384-EOXBFmc3gx5mb+vn0vPvvGqACToJD24hhacX5Yx+8NUUQrHIle/Qi5Bg9o3zKwW2'

// The page is the model's words in the person's own browser, outside the
// sandbox: it runs no script but Mermaid, fetches nothing, and sends nothing.
export const CSP = [
  "default-src 'none'",
  `script-src ${MERMAID_URL}`,
  "style-src 'unsafe-inline'",
  'img-src data:',
  'font-src data:',
  "form-action 'none'",
  "base-uri 'none'",
].join('; ')

// Easy to read: 18px text, a column of about 75 characters, generous line
// and paragraph spacing, left-aligned, no italics. Light and dark from the
// system. Diagrams sit on a light card, since Mermaid draws its light theme.
const BASE_CSS = `
:root { color-scheme: light dark;
  --bg: #fbfaf7; --fg: #1c1917; --muted: #57534e; --line: #d6d3d1; --card: #ffffff; --code: #f1efe9;
  --accent: #1d4ed8; --eye: #eff6ff; --new: #15803d; --change: #b45309; --remove: #b91c1c; }
@media (prefers-color-scheme: dark) { :root {
  --bg: #1c1917; --fg: #f5f5f4; --muted: #b7b0a9; --line: #44403c; --card: #262220; --code: #2f2a27;
  --accent: #93c5fd; --eye: #172554; --new: #4ade80; --change: #fbbf24; --remove: #f87171; } }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); text-align: left;
  font: 18px/1.6 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; }
em, i { font-style: normal; font-weight: 600; }
main { max-width: 40rem; margin: 0 auto; padding: 0 1rem 4rem; }
header { padding-top: 2rem; }
.kicker, .meta { color: var(--muted); margin: 0; font-size: 0.95rem; }
h1 { font-size: 1.9rem; line-height: 1.25; margin: 0.25rem 0 0.5rem; }
h2 { font-size: 1.35rem; line-height: 1.3; margin: 0 0 0.75rem; }
h3 { font-size: 1.1rem; margin: 1.25rem 0 0.4rem; }
p, ul, dl, table, figure, details { margin: 0 0 1.2rem; }
ul { padding-left: 1.4rem; }
li + li { margin-top: 0.5rem; }
section { margin: 2.5rem 0; scroll-margin-top: 4rem; }
nav.contents { position: sticky; top: 0; z-index: 1; background: var(--bg); border-bottom: 1px solid var(--line);
  margin: 1rem -1rem 0; padding: 0.5rem 1rem; overflow-x: auto; white-space: nowrap; }
nav.contents ol { list-style: none; margin: 0; padding: 0; display: flex; gap: 1.25rem; }
nav.contents li + li { margin-top: 0; }
nav.contents a { font-size: 0.95rem; }
a { color: var(--accent); }
.glance { background: var(--card); border: 2px solid var(--line); border-radius: 12px; padding: 1.25rem 1.5rem; }
.lede { font-size: 1.2rem; font-weight: 500; }
.eye { background: var(--eye); border-left: 5px solid var(--accent); border-radius: 0 8px 8px 0;
  padding: 0.75rem 1rem; margin: 0 0 1.2rem; }
.eye h3 { margin-top: 0; }
.next { font-weight: 500; border-top: 1px solid var(--line); padding-top: 0.9rem; margin: 0; }
footer { margin: 3rem 0 0; }
.num { display: inline-block; min-width: 2.6rem; color: var(--muted); font-size: 0.9rem; font-weight: 600; }
figure { background: #ffffff; color: #1c1917; border: 1px solid var(--line); border-radius: 10px;
  padding: 0.75rem; overflow-x: auto; }
figcaption.look { margin: 0 0 0.5rem; font-size: 1rem; }
pre.mermaid { margin: 0; text-align: center; font-family: inherit; }
svg { max-width: 100%; height: auto; }
code, pre { font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 0.88em; }
code { background: var(--code); padding: 0.1em 0.35em; border-radius: 4px; overflow-wrap: anywhere; }
figure code { background: #f1efe9; }
table { width: 100%; border-collapse: collapse; display: block; overflow-x: auto; }
th, td { text-align: left; vertical-align: top; padding: 0.5rem 0.75rem 0.5rem 0; border-top: 1px solid var(--line); }
th { color: var(--muted); font-weight: 600; border-top: 0; }
dt { font-weight: 700; }
dd { margin: 0 0 0.75rem; }
.badge { display: inline-block; font-size: 0.8rem; font-weight: 700; padding: 0.05rem 0.55rem;
  border-radius: 999px; border: 2px solid currentColor; white-space: nowrap; }
.badge.new { color: var(--new); }
.badge.change { color: var(--change); }
.badge.remove { color: var(--remove); border-style: dashed; }
.legend { color: var(--muted); }
details { border: 1px solid var(--line); border-radius: 8px; padding: 0.5rem 0.9rem; }
summary { cursor: pointer; font-weight: 600; }
`

// The colours `new`, `change` and `remove` take in a flowchart or a state
// diagram, the same as the badges: a diagram marks what the plan changes with
// `class <node> new` or `:::new`.
const MARKS = [
  'classDef new fill:#dcfce7,stroke:#15803d,stroke-width:3px,color:#14532d',
  'classDef change fill:#fef3c7,stroke:#b45309,stroke-width:3px,color:#78350f',
  'classDef remove fill:#fee2e2,stroke:#b91c1c,stroke-width:3px,stroke-dasharray:6 4,color:#7f1d1d',
].join('\n')

const LABELS: Record<Change, string> = { new: 'new', change: 'changed', remove: 'removed' }

const escapeHtml = (text: string): string =>
  text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

// The model's text as HTML: escaped, with `backticks` shown as code.
const plain = (text: string): string => escapeHtml(text).replace(/`([^`]+)`/g, '<code>$1</code>')

// A line the reader scans: as `plain`, with a short "Label: " at its start
// in bold, so the first words say what the line is about.
export const inline = (text: string): string => {
  const label = /^([^:`]{1,40}): (.+)$/.exec(text)
  return label === null ? plain(text) : `<strong>${plain(label[1] ?? '')}:</strong> ${plain(label[2] ?? '')}`
}

// Minutes to read at 200 words a minute, plus half a minute a diagram; at
// least one. Shown first, so the reader knows what they are taking on.
export const readingMinutes = (report: Report): number => {
  const prose = [
    report.summary,
    ...report.changes,
    ...report.unchanged,
    ...report.needs_your_eye,
    ...report.terms.flatMap(t => [t.term, t.meaning]),
    ...report.sections.flatMap(s => [s.heading, s.look_at ?? '', ...s.points, (s.detail ?? '').replace(/<[^>]*>/g, ' ')]),
    ...report.files.map(f => f.why),
    ...report.checks.flatMap(c => [c.check, c.how]),
  ].join(' ')
  const words = prose.split(/\s+/).filter(Boolean).length
  const diagrams = report.sections.filter(s => s.diagram !== undefined).length
  return Math.max(1, Math.ceil(words / 200 + diagrams / 2))
}

const count = (n: number, one: string): string => `${n} ${one}${n === 1 ? '' : 's'}`

const badge = (change: Change): string => `<span class="badge ${change}">${LABELS[change]}</span>`

const bullets = (lines: readonly string[]): string => `<ul>${lines.map(l => `<li>${inline(l)}</li>`).join('')}</ul>`

// What to do with the page, said where it starts and again where it ends.
const NEXT =
  `<p class="next">Back in the terminal: choose <strong>${escapeHtml(WRITE)}</strong> ` +
  'to save this plan, or type what to change.</p>'

const LEGEND =
  `<p class="legend">In the diagrams: ${badge('new')} ${badge('change')} ${badge('remove')}. ` +
  'Everything else is unchanged.</p>'

const contents = (report: Report): string => {
  const links: [string, string][] = [
    ['glance', 'At a glance'],
    ...(report.terms.length > 0 ? [['terms', 'Words used here'] as [string, string]] : []),
    ...report.sections.map((s, i): [string, string] => [`part-${i + 1}`, `${i + 1}. ${s.heading}`]),
    ['files', 'Files'],
    ['checks', 'Done when'],
  ]
  const items = links.map(([id, label]) => `<li><a href="#${id}">${escapeHtml(label)}</a></li>`).join('')
  return `<nav class="contents" aria-label="Contents"><ol>${items}</ol></nav>`
}

const glance = (report: Report): string =>
  [
    '<section class="glance" id="glance">',
    '<h2>At a glance</h2>',
    `<p class="lede">${plain(report.summary)}</p>`,
    '<h3>What changes</h3>',
    bullets(report.changes),
    report.unchanged.length > 0 ? `<h3>What stays the same</h3>\n${bullets(report.unchanged)}` : '',
    report.needs_your_eye.length > 0
      ? `<div class="eye">\n<h3>Needs your eye</h3>\n${bullets(report.needs_your_eye)}\n</div>`
      : '',
    NEXT,
    '</section>',
  ]
    .filter(Boolean)
    .join('\n')

const terms = (list: readonly Term[]): string =>
  [
    '<section id="terms">',
    '<h2>Words used here</h2>',
    `<dl>${list.map(t => `<dt>${plain(t.term)}</dt><dd>${inline(t.meaning)}</dd>`).join('')}</dl>`,
    '</section>',
  ].join('\n')

// Flowcharts and state diagrams get the change colours after their own lines.
const withMarks = (source: string): string =>
  /^\s*(flowchart|graph|stateDiagram)/.test(source) ? `${source.trimEnd()}\n${MARKS}` : source

const figure = (diagram: Diagram, lookAt: string): string => {
  const drawing =
    'mermaid' in diagram ? `<pre class="mermaid">${escapeHtml(withMarks(diagram.mermaid))}</pre>` : diagram.svg
  return [
    '<figure>',
    `<figcaption class="look"><strong>Look at:</strong> ${plain(lookAt)}</figcaption>`,
    drawing,
    '</figure>',
  ].join('\n')
}

const part = (s: Section, n: number, of: number): string =>
  [
    `<section class="part" id="part-${n}">`,
    `<h2><span class="num">${n}/${of}</span> ${plain(s.heading)}</h2>`,
    s.diagram !== undefined ? figure(s.diagram, s.look_at ?? '') : '',
    bullets(s.points),
    s.detail !== undefined ? `<details>\n<summary>More detail</summary>\n${s.detail}\n</details>` : '',
    '</section>',
  ]
    .filter(Boolean)
    .join('\n')

const files = (list: readonly FileChange[]): string => {
  const rows = list
    .map(f => `<tr><td>${badge(f.change)}</td><td><code>${escapeHtml(f.path)}</code></td><td>${inline(f.why)}</td></tr>`)
    .join('')
  return [
    '<section id="files">',
    '<h2>Files</h2>',
    '<table>',
    '<thead><tr><th>Change</th><th>File</th><th>Why</th></tr></thead>',
    `<tbody>${rows}</tbody>`,
    '</table>',
    '</section>',
  ].join('\n')
}

const checks = (list: readonly Check[]): string => {
  const rows = list.map(c => `<tr><td>${inline(c.check)}</td><td>${inline(c.how)}</td></tr>`).join('')
  return [
    '<section id="checks">',
    '<h2>Done when</h2>',
    '<table>',
    '<thead><tr><th>Check</th><th>How it is checked</th></tr></thead>',
    `<tbody>${rows}</tbody>`,
    '</table>',
    '</section>',
  ].join('\n')
}

// The whole page: the policy before anything it governs, then the title, the
// stylesheet and Mermaid, then the report in its fixed order.
export const reportPage = (report: Report): string => {
  const parts = report.sections
  const meta =
    `About ${readingMinutes(report)} min to read · ` +
    `${count(parts.length, 'part')} · ${count(report.files.length, 'file')}`
  return [
    '<!doctype html>',
    '<html lang="en">',
    '<head>',
    `<meta http-equiv="Content-Security-Policy" content="${CSP}">`,
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${escapeHtml(report.title)}</title>`,
    `<style>${BASE_CSS}</style>`,
    `<script src="${MERMAID_URL}" integrity="${MERMAID_SRI}" crossorigin="anonymous"></script>`,
    '</head>',
    '<body>',
    '<main>',
    '<header>',
    '<p class="kicker">Refine report</p>',
    `<h1>${plain(report.title)}</h1>`,
    `<p class="meta">${meta}</p>`,
    '</header>',
    contents(report),
    glance(report),
    report.terms.length > 0 ? terms(report.terms) : '',
    parts.some(s => s.diagram !== undefined) ? LEGEND : '',
    ...parts.map((s, i) => part(s, i + 1, parts.length)),
    files(report.files),
    checks(report.checks),
    `<footer>${NEXT}</footer>`,
    '</main>',
    '</body>',
    '</html>',
  ]
    .filter(Boolean)
    .join('\n') + '\n'
}
```

`.filter(Boolean)` drops the optional blocks a report leaves out.

- [ ] **Step 7: Point refine.ts at the new modules**

In `claude/refine-mod/hooks/refine.ts`, replace

```ts
import { openArgv, parseReport, reportPage, reportPath, reportsDir } from './report'
```

with

```ts
import { reportPage } from './page'
import { openArgv, parseReport, reportPath, reportsDir } from './report'
```

and replace the whole `const REPORT_SPEC = { … }` with:

```ts
const TEXT = { type: 'string' }
const TEXTS = { type: 'array', items: TEXT }

const REPORT_SPEC = {
  name: 'show_task_report',
  description:
    'Writes an HTML report of the plan in one fixed, easy-to-read layout and opens it in the browser, once the ' +
    'person has chosen "Show me a report first" in write_task_plan. Fill the fields as the refine-task skill\'s ' +
    'report-catalogue.md says. Every text field is one line of plain text; a report over the limits is refused ' +
    'with every reason at once.',
  inputSchema: {
    type: 'object',
    properties: {
      title: { ...TEXT, description: "The task's new description." },
      summary: { ...TEXT, description: 'One sentence, at most 35 words: what the task does.' },
      changes: { ...TEXTS, description: '1-3 lines, at most 20 words each: what will be different.' },
      unchanged: { ...TEXTS, description: '0-3 lines, at most 20 words each: what stays the same.' },
      needs_your_eye: { ...TEXTS, description: '0-3 lines, at most 30 words each: where the person should judge.' },
      terms: {
        type: 'array',
        description: '0-5 words the report uses, each with a plain meaning.',
        items: { type: 'object', properties: { term: TEXT, meaning: TEXT }, required: ['term', 'meaning'] },
      },
      sections: {
        type: 'array',
        description: '1-6 parts: a heading, 1-5 points, optionally one diagram with a look_at line, and collapsed detail.',
        items: {
          type: 'object',
          properties: {
            heading: TEXT,
            look_at: TEXT,
            diagram: { type: 'object', properties: { mermaid: TEXT, svg: TEXT } },
            points: TEXTS,
            detail: { ...TEXT, description: 'HTML, shown collapsed under "More detail".' },
          },
          required: ['heading', 'points'],
        },
      },
      files: {
        type: 'array',
        items: {
          type: 'object',
          properties: { path: TEXT, change: { type: 'string', enum: ['new', 'change', 'remove'] }, why: TEXT },
          required: ['path', 'change', 'why'],
        },
      },
      checks: {
        type: 'array',
        items: { type: 'object', properties: { check: TEXT, how: TEXT }, required: ['check', 'how'] },
      },
    },
    required: ['title', 'summary', 'changes', 'sections', 'files', 'checks'],
  },
}
```

- [ ] **Step 8: Move refine.test.ts to the new fields**

In `claude/refine-mod/tests/refine.test.ts`:

1. Replace the line `const BODY = '<h1>feat: New words</h1><pre class="mermaid">flowchart LR\n a --> b</pre>'` with:

```ts
// The smallest report show_task_report takes.
const MINIMAL = {
  title: 'feat: New words',
  summary: 'The task gets new words.',
  changes: ['Words: the description changes.'],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
}
```

2. Replace every `{ tool: REPORT_TOOL, title: 'feat: New words', body: BODY }` and every `{ tool: REPORT_TOOL, title: 't', body: BODY }` with `{ tool: REPORT_TOOL, ...MINIMAL }`.
3. In `report, then approve: …`, replace `expect(writes[0]?.text).toContain(BODY)` with `expect(writes[0]?.text).toContain('<p class="lede">The task gets new words.</p>')`.
4. Replace the whole test `a body with a script is refused, and the person may still get a report` with:

```ts
test('a report with a script is refused, and the person may still get one', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const refused = await $.tool.call({
    tool: REPORT_TOOL,
    ...MINIMAL,
    sections: [{ heading: 'What changes', points: ['The words.'], detail: '<script>alert(1)</script>' }],
  })
  expect(refused.deny).toBe(
    'show_task_report: sections[0].detail must not contain <script>: ' +
      'the page runs no script but Mermaid, loads nothing and sends nothing',
  )
  expect(writes).toEqual([])
  const fixed = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(fixed.result).toContain(`Wrote the report to ${REPORT_PATH}`)
})

test('a report over its limits is refused with every reason', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, changes: [], checks: [] })
  expect(answer.deny).toBe(
    'show_task_report: changes must have 1 to 3 items, not 0; checks must have 1 to 6 items, not 0',
  )
  expect(writes).toEqual([])
})
```

- [ ] **Step 9: Run the tests to verify they pass**

Run: `claude plugin test claude/refine-mod`
Expected: every test in `plan.test.ts`, `report.test.ts`, `page.test.ts` and `refine.test.ts` passes.

- [ ] **Step 10: Check the example in a browser engine**

Render the worked example and confirm both diagrams draw (Mermaid reads the escaped quotes and the appended `classDef` lines) and nothing reports a syntax error:

```bash
S="$(mktemp -d)"
OUT="$S/example.html" bun -e '
import { EXAMPLE } from "./claude/refine-mod/tests/example-report.ts"
import { parseReport } from "./claude/refine-mod/hooks/report.ts"
import { reportPage } from "./claude/refine-mod/hooks/page.ts"
const report = parseReport(EXAMPLE)
if (typeof report === "string") throw new Error(report)
await Bun.write(process.env.OUT, reportPage(report))
'
timeout 60 google-chrome --headless=new --disable-gpu --virtual-time-budget=10000 --dump-dom "file://$S/example.html" > "$S/out.html"
grep -c '<svg id="mermaid' "$S/out.html"; grep -c 'Syntax error' "$S/out.html"
rm -r "$S"
```

Expected: `2`, then `0`. If a diagram fails, the likeliest cause is the appended `classDef` lines: check `MARKS` against https://mermaid.js.org/syntax/flowchart.html#classes before changing the example.

- [ ] **Step 11: Validate, type-check and commit**

Run: `claude plugin validate claude/refine-mod`
Expected: `✔ Validation passed`.

Type-check against this build's API, from a scratch folder. `claude-code.d.ts` is written when the `plugin-authoring` skill loads; take the newest:

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

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/hooks/page.ts claude/refine-mod/hooks/refine.ts \
  claude/refine-mod/tests/example-report.ts claude/refine-mod/tests/report.test.ts \
  claude/refine-mod/tests/page.test.ts claude/refine-mod/tests/refine.test.ts
git commit -m "$(cat <<'EOF'
feat(refine): lay every report out the same, easy-to-read way

show_task_report takes fixed, length-limited fields instead of free
HTML, and the mod lays them out conclusion first: what changes, what
needs the reader's eye and the next step, then numbered parts whose
diagrams each say what to look at, then the files and the checks.

Co-Authored-By: <the model that wrote it> <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Link each report from the task's notes

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts` (add `MadeReport`, `reportNotes`)
- Modify: `claude/refine-mod/hooks/refine.ts` (the report state, both tool hooks)
- Test: `claude/refine-mod/tests/report.test.ts`, `claude/refine-mod/tests/refine.test.ts`

**Interfaces:**
- Consumes: `Plan` from `./plan`; Task 1's `report.ts`.
- Produces: `type MadeReport = { path: string; plan: Plan }` and `reportNotes(made: readonly MadeReport[], plan: Plan): string[]` in `hooks/report.ts`. The note formats `Report: <path>` and `Report (earlier draft): <path>`, which the skill (Task 3) names. The report tool's result text gains `, and the task will link the report in its notes.`.

- [ ] **Step 1: Write the failing unit tests**

In `claude/refine-mod/tests/report.test.ts`, add `reportNotes` to the `../hooks/report` import, and append:

```ts
describe('reportNotes', () => {
  const plan = { description: 'feat: A', notes: ['Goal: x'] }

  test('links a report made for this very plan', () => {
    expect(reportNotes([{ path: '/r/a.html', plan }], plan)).toEqual(['Report: /r/a.html'])
  })

  test('marks a report made before the plan changed', () => {
    expect(reportNotes([{ path: '/r/a.html', plan }], { ...plan, notes: ['Goal: y'] })).toEqual([
      'Report (earlier draft): /r/a.html',
    ])
  })

  test('links each report once, in the order made', () => {
    const made = [
      { path: '/r/a.html', plan: { ...plan, description: 'feat: Old' } },
      { path: '/r/b.html', plan },
    ]
    expect(reportNotes(made, plan)).toEqual(['Report (earlier draft): /r/a.html', 'Report: /r/b.html'])
    expect(reportNotes(made, { ...plan, notes: [...plan.notes, 'Report: /r/b.html'] })).toEqual([
      'Report (earlier draft): /r/a.html',
    ])
  })

  test('none made, none linked', () => {
    expect(reportNotes([], plan)).toEqual([])
  })
})
```

- [ ] **Step 2: Write the failing tool tests**

In `claude/refine-mod/tests/refine.test.ts`, in the test `report, then approve: writes the page, opens it, then writes the task`:

1. Replace the `expect(shown.result).toBe(…)` block with:

```ts
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH} and opened it in the browser. ` +
      'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
      'and the task will link the report in its notes.',
  )
```

2. Replace the last three lines (from `const written = await $.tool.call(CALL)` to the end of the test) with:

```ts
  const written = await $.tool.call(CALL)
  expect(written.result).toBe(`Wrote the plan to task ${UUID}: 3 note(s), tagged planned.`)
  expect(seen).toContain(`log: Note 3: Report: ${REPORT_PATH}`)
  expect(verbs(runs).at(-1)).toBe('import')
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.map((a: { description: string }) => a.description)).toEqual([
    'step one',
    'step two',
    `Report: ${REPORT_PATH}`,
  ])
```

Then append this test:

```ts
test('a report made before the plan changed is linked as an earlier draft', { options: REPORTS }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, [REPORT, WRITE])
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  const written = await $.tool.call({ ...CALL, notes: ['step one', 'step three'] })
  expect(written.result).toContain('3 note(s)')
  expect(seen).toContain(`log: Note 3: Report (earlier draft): ${REPORT_PATH}`)
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.at(-1).description).toBe(`Report (earlier draft): ${REPORT_PATH}`)
})
```

- [ ] **Step 3: Run them to verify they fail**

Run: `claude plugin test claude/refine-mod`
Expected: the four `reportNotes` tests fail (no export), and both tool tests fail (no report note; the old result text).

- [ ] **Step 4: Add reportNotes to report.ts**

In `claude/refine-mod/hooks/report.ts`, change the first import to:

```ts
import { HIDDEN } from './plan'
import type { Plan } from './plan'
```

and add, after `openArgv`:

```ts
// A report made in this session, and the plan the person was shown when
// they asked for it.
export type MadeReport = { path: string; plan: Plan }

const samePlan = (a: Plan, b: Plan): boolean =>
  a.description === b.description && a.notes.length === b.notes.length && a.notes.every((n, i) => n === b.notes[i])

// The notes that link the session's reports from the task, after the plan's
// own: marked as an earlier draft when the plan changed since. A report the
// plan's notes already link is not linked twice.
export const reportNotes = (made: readonly MadeReport[], plan: Plan): string[] =>
  made
    .filter(m => !plan.notes.some(note => note.startsWith('Report') && note.endsWith(m.path)))
    .map(m => (samePlan(m.plan, plan) ? `Report: ${m.path}` : `Report (earlier draft): ${m.path}`))
```

- [ ] **Step 5: Wire it into refine.ts**

In `claude/refine-mod/hooks/refine.ts`:

1. Change the report import to:

```ts
import { openArgv, parseReport, reportNotes, reportPath, reportsDir } from './report'
import type { MadeReport } from './report'
```

2. Replace

```ts
  // Set by the person's REPORT answer to the latest question, spent by the
  // first well-formed report. Module state: a reload forgets it, and the
  // model is told to ask again.
  let reportAsked = false
```

with

```ts
  // The plan the person was shown when they chose REPORT in answer to the
  // latest question, spent by the first well-formed report. Module state: a
  // reload forgets it, and the model is told to ask again.
  let askedFor: Plan | undefined
  // The reports made this session, linked from the task's notes when it is
  // written.
  const made: MadeReport[] = []
```

3. In the `write_task_plan` hook, replace

```ts
    // The plan as the tool will write it, from its own arguments, not from
    // what the model printed; refused whole before a line could be cut.
    const lines = planLines(input)
```

with

```ts
    // The plan as the tool will write it, from its own arguments, not from
    // what the model printed, with a note linking each report made this
    // session; refused whole before a line could be cut.
    const plan = { description: input.description, notes: [...input.notes, ...reportNotes(made, input)] }
    const lines = planLines(plan)
```

4. In the same hook, replace `reportAsked = false` (just before `const choices`) with `askedFor = undefined`. Replace

```ts
    if (answer === REPORT && reports !== undefined) {
      reportAsked = true
      return { deny: REPORT_FIRST }
    }
```

with

```ts
    if (answer === REPORT && reports !== undefined) {
      askedFor = { description: input.description, notes: input.notes }
      return { deny: REPORT_FIRST }
    }
```

5. In the same hook, replace

```ts
    const planned = merge(now.task, input, await $.clock.now())
```

with

```ts
    const planned = merge(now.task, { ...input, notes: plan.notes }, await $.clock.now())
```

and in its result, `${input.notes.length} note(s)` with `${plan.notes.length} note(s)`.

6. In the `show_task_report` hook, replace

```ts
    if (!reportAsked) return { deny: NOT_ASKED_FOR }
```

with

```ts
    if (askedFor === undefined) return { deny: NOT_ASKED_FOR }
```

replace

```ts
    reportAsked = false
```

with

```ts
    const shown = askedFor
    askedFor = undefined
```

add `made.push({ path, plan: shown })` on the line after `await $.fs.write(path, reportPage(report))`, and make the result:

```ts
    return {
      result:
        `Wrote the report to ${path} and opened it in the browser. ` +
        'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
        'and the task will link the report in its notes.',
    }
```

The report is remembered only after `$.fs.write` succeeds, so a failed write links nothing.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `claude plugin test claude/refine-mod`
Expected: every test passes.

- [ ] **Step 7: Validate and commit**

Run: `claude plugin validate claude/refine-mod`. Expected: `✔ Validation passed`.

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/hooks/refine.ts \
  claude/refine-mod/tests/report.test.ts claude/refine-mod/tests/refine.test.ts
git commit -m "$(cat <<'EOF'
feat(refine): link each report from the task's notes

The write tool adds a note per report made in the session, after the
plan's own notes and shown with them before the question: Report: and
its path, or Report (earlier draft): when the plan changed since.

Co-Authored-By: <the model that wrote it> <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Teach the session to fill the report

**Files:**
- Rewrite: `.claude/skills/refine-task/report-catalogue.md`
- Modify: `.claude/skills/refine-task/SKILL.md`
- Modify: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`
- Modify: `CONTEXT.md`

**Interfaces:**
- Consumes: the field names, `LIMITS`, the classes `new`, `change` and `remove`, and the note formats from Tasks 1–2; `tests/example-report.ts`.
- Produces: nothing code depends on.

- [ ] **Step 1: Rewrite the catalogue**

Replace the whole of `.claude/skills/refine-task/report-catalogue.md` with the text below. The worked example's JSON must equal `EXAMPLE` in `claude/refine-mod/tests/example-report.ts`. Generate it rather than retyping it:

```bash
bun -e 'import { EXAMPLE } from "./claude/refine-mod/tests/example-report.ts"; console.log(JSON.stringify(EXAMPLE, null, 2))'
```

Paste the output where the text below says `<the worked example>`, inside a ```` ```json ```` fence.

````markdown
# Refine report catalogue

How to fill `show_task_report` when the user chooses **Show me a report
first**. The tool builds the page itself, in one fixed layout, from the
fields you give it. Your job is the words and the diagrams.

## Who reads it, and what for

One person, deciding one thing: approve this plan, or say what to change.
Write for a reader who may have ADHD. They scan rather than read, hold about
four things in mind at once, lose their place when interrupted, and give up
on walls of text. What helps them helps every reader:

- **Conclusion first.** The top of the page answers "what does this do, and
  what am I deciding?" before any detail.
- **Short and plain.** One idea per line, everyday words, no idioms, no
  double negatives. Say "the panel", not `src/panel/surface.rs`, except in
  `files`.
- **Point at what matters.** Each diagram says what to look at, and
  `needs_your_eye` says where the reader's judgement is needed.
- **Pictures with words, not instead of them.** Every diagram has points
  beside it.
- **The same shape every time.** The layout never changes, so the reader
  learns it once.

The tool enforces the limits below and refuses a report that breaks them,
listing every reason, so fix them all and call again.

## The fields

| Field | What it holds | Limit |
|---|---|---|
| `title` | The task's new description. | 120 characters |
| `summary` | One sentence: what the task does, for the reader. | 35 words |
| `changes` | What will be different once it is done. | 1–3 lines, 20 words each |
| `unchanged` | What stays the same, or is out of scope, that the reader might worry about. | 0–3 lines, 20 words each |
| `needs_your_eye` | Where the reader's judgement is needed: an assumption, a trade-off, a risk, an open question. | 0–3 lines, 30 words each |
| `terms` | Words the report uses that the reader may not know, each with a plain `meaning`. | 0–5; term 4 words, meaning 20 |
| `sections` | The parts that explain the plan, in reading order. | 1–6 |
| `sections[].heading` | A statement of what the part shows: "What you will see", not "Overview". | 10 words |
| `sections[].diagram` | One diagram: `{ "mermaid": "…" }` or `{ "svg": "<svg …>…</svg>" }`. | Mermaid 40 lines |
| `sections[].look_at` | With a diagram, and only then: what to look at in it. | 25 words |
| `sections[].points` | What the part says, one idea per line. | 1–5 lines, 25 words each |
| `sections[].detail` | Optional HTML, shown collapsed under "More detail", for whoever wants it. | 20,000 characters |
| `files` | Every file the plan touches: `path`, `change` (`new`, `change` or `remove`) and `why`. | 1–40; why 20 words |
| `checks` | The Done when checks: the `check`, and `how` it is checked. | 1–6; 20 words each |

Every field except `detail` and the diagram is one line of plain text. Two
marks are drawn specially: text in backticks as code, and a short label
before the first `: ` in bold. Start a line with a label when it helps the
reader scan: `"Approval: the task is still written only when you choose."`

`detail`, an SVG and a Mermaid diagram may not contain `<script>`, `<meta>`,
`<link>`, `<base>`, `<iframe>`, `<frame>`, `<frameset>`, `<object>`,
`<embed>`, `<form>` or `<portal>`: the page runs no script but Mermaid,
loads nothing but Mermaid, and sends nothing.

## Choosing the parts

Choose parts by the question they answer for the reader. Most tasks need two
to four. Put the one about what the reader will notice first.

| The reader asks | Make a part with | When |
|---|---|---|
| What will I see change? | A **before and after**: one diagram with the new parts marked `new`, or two small ones. | Anything the user sees or does changes. |
| What happens, in order? | A **sequence** diagram (`sequenceDiagram`). | Several actors — the user, a CLI, a daemon, Claude, a service — take turns. |
| What states can it be in? | A **state** diagram (`stateDiagram-v2`). | Something gains, loses or guards a state: a task's status, a mode, a flag. |
| Where does the data go? | A **data flow** (`flowchart LR`), stores as `[( )]`, arrows labelled with what moves. | Data is read, changed, stored or sent in a new way. |
| What talks to what? | A **component map** (`flowchart TB`, a `subgraph` per process). | A module, process or service is added, or a job moves between them. |
| What does each case do? | Points, one per case, or a table in `detail`. | Behaviour branches on two or more inputs. |
| What shape is the data? | A `classDiagram`, or a field table in `detail`. | A type, a JSON shape or a config's keys change. |
| What else was considered? | Points: each option, and why it was or was not chosen. | A Decided note chose between alternatives. |
| What could go wrong? | Points starting "Risk: …", and the worst one in `needs_your_eye`. | Something working could break, data could be lost, or a security boundary moves. |

The files and the checks are always there: they are fields, not parts.

## Drawing diagrams

- **One idea per diagram, at most about nine boxes.** Split a bigger one in
  two parts.
- **Plain names on the boxes:** "the panel", "Claude", "the task". File
  names belong in `files`.
- **Mark what the plan changes.** In a flowchart or a state diagram, add
  `class <node> new`, `change` or `remove`, or `:::new` after a node. The page
  draws them green, amber and dashed red, the same as the file badges, and
  says so above the first part. Do not define those classes yourself. In a
  sequence diagram, wrap the new steps in `rect rgb(220, 252, 231)` … `end`.
- **Label the arrows** with what moves or happens.
- **`look_at` points at the change:** "The new middle answer, and the loop
  back to the question", not "The flow".
- Use SVG only for what Mermaid draws badly, such as a screen mock-up. Give
  it a `viewBox`, and draw lines and text in `currentColor`.

## Writing the lines

- Start with the thing that matters, in everyday words.
- One idea per line; no "and also".
- Name things the way the project does (`CONTEXT.md`), and put any word the
  reader may not know in `terms`.
- No italics, capitals for emphasis or exclamation marks; the label is the
  emphasis.
- Say what the plan does, not what it "aims to" or "should" do.

## Worked example

This very feature, explained:

<the worked example>

## Why it is built this way

- W3C, Making Content Usable for People with Cognitive and Learning
  Disabilities: https://www.w3.org/TR/coga-usable/
- Nielsen Norman Group, How Users Read on the Web:
  https://www.nngroup.com/articles/how-users-read-on-the-web/
- WCAG 2.2, Understanding 1.4.8 Visual Presentation:
  https://www.w3.org/WAI/WCAG22/Understanding/visual-presentation.html
- Mayer's principles of multimedia learning, and Lorch on text signals.
- Moody, The "Physics" of Notations (IEEE TSE, 2009).
- Mermaid syntax: https://mermaid.js.org/intro/
````

Then prove the catalogue's example still renders. Run Task 1 Step 10's command and expect `2`, then `0`.

- [ ] **Step 2: Update the skill**

In `.claude/skills/refine-task/SKILL.md`:

1. In step 4's "New notes" item, after `They **replace** the existing notes, so fold in everything the old notes said that still holds.`, add ` Keep any \`Report: …\` or \`Report (earlier draft): …\` notes as they are: they link earlier reports.` Re-wrap the item to the file's width.
2. In step 5, change `It writes the description, replaces the notes and adds \`+planned\` in one import` to `It writes the description, replaces the notes (adding a note that links each report made this session) and adds \`+planned\` in one import`. Re-wrap.
3. Replace the whole `### Report, when asked` subsection, from its heading up to (not including) `## 6. Verify and report`, with:

```markdown
### Report, when asked

The user wants to see the plan explained before they approve it. Write
nothing to the task. Build the report:

1. Read `report-catalogue.md` in this skill's base directory: who the report
   is for, its fields and their limits, how to choose its parts and draw its
   diagrams, and a worked example.
2. Fill the fields from the plan you just proposed and the code you read in
   step 2. Real paths go in `files`; everywhere else, plain words the user
   would use.
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
made before the plan changed. Do not add these notes yourself.

If, after reading the report, the user answers with changes, revise, print
the block again (step 4) and call `write_task_plan` again.

```

4. In step 6, change `the notes are exactly the new list` to `the notes are exactly the new list, followed by any \`Report …\` notes the tool added`. Re-wrap.

- [ ] **Step 3: Record it in ADR 0002 and the glossary**

In `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, after the paragraph that begins `*2026-10-09:* Taskwarrior is now 3.5.0`, add a blank line and:

```markdown
*2026-10-09:* the write tool also adds notes of its own: one per HTML
report the person asked for during the session (`show_task_report`),
`Report: <path>` or `Report (earlier draft): <path>`. They are shown with
the plan before the question, so what the person approves is still what is
written.
```

In `CONTEXT.md`, after the **Planned task** entry (ending `_Avoid_: ready (Taskwarrior's \`+READY\` means something else), refined, groomed`), add a blank line and:

```markdown
**Refine report**:
An HTML page explaining a refine's plan — what changes, what needs the
reader's judgement, diagrams, the files and the checks — that a refine
session makes only when the user chooses Show me a report first. It is saved
under the reviews folder and linked from the task's notes.
_Avoid_: plan report, explainer
```

- [ ] **Step 4: Re-read and run every check**

Read `SKILL.md` top to bottom. Check that every answer either tool gives has a line saying what to do, and that every field named in the catalogue matches `REPORT_SPEC` in `refine.ts`.

Run: `claude plugin validate claude/refine-mod && claude plugin test claude/refine-mod && cargo test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/refine-task/report-catalogue.md .claude/skills/refine-task/SKILL.md \
  docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md CONTEXT.md
git commit -m "$(cat <<'EOF'
docs(refine): teach refine-task to write a report that is easy to read

The catalogue now says who the report is for and why, the fields and
their limits, how to choose parts by the reader's question and draw
small marked diagrams, with a worked example. The skill names the
report notes the write tool adds.

Co-Authored-By: <the model that wrote it> <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Put it in front of the reader

This task needs the user. It is the test the research cannot replace.

- [ ] **Step 1: Render the example for the user**

```bash
OUT="$HOME/.local/share/niri-tasks/reviews/refine-example-$(date -u +%Y%m%d-%H%M%S).html" bun -e '
import { EXAMPLE } from "./claude/refine-mod/tests/example-report.ts"
import { parseReport } from "./claude/refine-mod/hooks/report.ts"
import { reportPage } from "./claude/refine-mod/hooks/page.ts"
const report = parseReport(EXAMPLE)
if (typeof report === "string") throw new Error(report)
await Bun.write(process.env.OUT, reportPage(report))
console.log(process.env.OUT)
'
```

Open the printed path with `xdg-open`, and ask the user to read it as they would before approving a task. Ask what they noticed first, what they skipped, and what was hard. Make what they ask for, with the tests changed to match, before the live refine.

- [ ] **Step 2: A live refine**

1. The user runs `./install.sh` from this worktree. It rebuilds `niritasks` and relinks the skill, the catalogue and the mod.
2. They file a throwaway task, Refine it, and at **Write this to the task?** choose **Show me a report first**.
3. Expected: the page opens in the browser in the fixed layout. The question comes back, and **Write it to the task** writes the task. `task rc.json.array=on <uuid> export` shows a last note `Report: <path>`, and that path opens the same page.
4. Delete the throwaway task. Re-run `./install.sh` from the main checkout after the branch lands.
