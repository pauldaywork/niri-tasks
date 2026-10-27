import { describe, expect, test } from 'claude-code/testing'
import { drawSources } from '../hooks/draw'
import { markdownPath, reportFromMarkdown, reportMarkdown, twinOf } from '../hooks/markdown'
import { reportPage } from '../hooks/page'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'
import { BRANCH_MIN, branched } from './branch-fixtures'
import { EXAMPLE, EXAMPLE_PLAN } from './example-report'

// The report parseReport takes, failing the test where it is refused.
const parsed = (input: Record<string, unknown>): Report => {
  const answer = parseReport(input)
  if (typeof answer === 'string') throw new Error(answer)
  return answer.report
}

// The smallest report the tool takes, as parseReport answers it.
const MINIMAL = parsed({
  title: 'feat: New words',
  summary: 'The task gets new words.',
  changes: ['Words: the description changes.'],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
})

const PLAN = { description: 'feat: New words', notes: ['step one', 'step two'] }

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
const EDGES = parsed(EDGES_INPUT)

describe('markdownPath', () => {
  test('is the report with .md for .html', () => {
    expect(markdownPath('/r/refine-0b8f6a52-20261006-010203.html')).toBe('/r/refine-0b8f6a52-20261006-010203.md')
  })
})

describe('reportMarkdown', () => {
  test('the smallest report reads top to bottom as its slides do', () => {
    expect(reportMarkdown(MINIMAL, PLAN)).toBe(
      [
        '# feat: New words',
        '',
        'The task gets new words.',
        '',
        '## What changes',
        '',
        '- Words: the description changes.',
        '',
        '## What changes',
        '',
        '<!-- lens -->',
        '',
        '- The words.',
        '',
        '## Files',
        '',
        '| Change | File | Why |',
        '| --- | --- | --- |',
        '| change | `src/words.rs` | Holds the words. |',
        '',
        '## Done when',
        '',
        '| Check | How it is checked |',
        '| --- | --- |',
        '| The words show | By eye |',
        '',
        '## The plan',
        '',
        '**Description:** feat: New words',
        '',
        '1. step one',
        '2. step two',
        '',
      ].join('\n'),
    )
  })

  test('the optional lists show only when they hold something', () => {
    const md = reportMarkdown(
      { ...MINIMAL, unchanged: ['The panel.'], needs_your_eye: ['The key.'], terms: [{ term: 'twin', meaning: 'The .md copy.' }] },
      PLAN,
    )
    expect(md).toContain('## What stays the same\n\n- The panel.')
    expect(md).toContain('## Where your judgement is needed\n\n- The key.')
    expect(md).toContain('## Words used here\n\n- **twin**: The .md copy.')
    expect(reportMarkdown(MINIMAL, PLAN)).not.toContain('## What stays the same')
  })

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
    expect(md).toContain('## Flow\n\n<!-- lens -->\n\n_The arrow._\n\n```mermaid\nflowchart LR\n  a --> b\n```\n\n- p')
    expect(md).not.toContain('see the HTML report')
    expect(md).toContain('Before:\n\n```mermaid\nflowchart LR\n  a\n```\n\nAfter:\n\n```mermaid\nflowchart LR\n  b\n```')
  })

  test('code holding a fence gets a longer one, and a new file says so', () => {
    const md = reportMarkdown(
      { ...MINIMAL, sections: [{ heading: 'Code', points: ['p'], code: [{ file: 'a.md', before: '', after: 'x ``` y' }] }] },
      PLAN,
    )
    expect(md).toContain('### a.md\n\n_New file._\n\nAfter:\n\n````\nx ``` y\n````')
  })

  test('a pipe in a table cell is escaped', () => {
    const md = reportMarkdown({ ...MINIMAL, checks: [{ check: 'a | b', how: 'By eye' }] }, PLAN)
    expect(md).toContain('| a \\| b | By eye |')
  })

  test('the outside tools are a table, and detail follows the points', () => {
    const md = reportMarkdown(
      {
        ...MINIMAL,
        sections: [
          {
            heading: 'Tools',
            points: ['p'],
            tools: [{ tool: 'herdr', how: 'Its CLI.', swap: 'hard', why: 'Panes.' }],
            detail: '<p>More.</p>',
          },
        ],
      },
      PLAN,
    )
    expect(md).toContain('| Tool | How it connects | Swap | Why |\n| --- | --- | --- | --- |\n| herdr | Its CLI. | hard | Panes. |')
    expect(md).toContain('| herdr | Its CLI. | hard | Panes. |\n\n<details>\n<summary>More detail</summary>\n<p>More.</p>\n</details>')
  })

  test('the worked example has every part as a heading', () => {
    const report = parsed(EXAMPLE)
    const md = reportMarkdown(report, PLAN)
    for (const s of report.sections) expect(md).toContain(`## ${s.heading}`)
  })

  test('a plan with no notes says so', () => {
    expect(reportMarkdown(MINIMAL, { description: 'd', notes: [] })).toContain('**Description:** d\n\nNo notes.\n')
  })
})

describe('reportFromMarkdown', () => {
  test('the edge cases are a report the tool takes', () => {
    expect(typeof parseReport(EDGES_INPUT)).toBe('object')
  })

  test('reads the worked example back into the fields it was written from', () => {
    const report = parsed(EXAMPLE)
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
    const report = parsed(EXAMPLE)
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
  test('refuses a term the twin reads back as another', () => {
    const input = { ...MINIMAL, terms: [{ term: 'a**: b', meaning: 'c' }] }
    expect(twinOf(parsed(input), PLAN)).toBe('its Markdown twin reads back as a different report')
  })
})

describe('a branch report', () => {
  const full = branched({
    ...BRANCH_MIN,
    unchanged: ['The panel.'],
    terms: [{ term: 'twin', meaning: 'The .md copy.' }],
    hot_spots: [
      { spot: 'Pipes | in cells', why: 'A | b stays one cell.', files: ['src/words.rs', 'README.md'] },
      { spot: 'Second spot', why: 'Why two.', files: ['README.md'] },
    ],
    story: [
      { step: 'Wrote the words', commits: ['abc1234'], points: ['One.', 'Two.'] },
      { step: 'Pinned them', commits: ['abd5678'], points: ['Three.'] },
    ],
    map: [
      { path: 'src/words.rs', change: 'change', items: [{ name: 'words()', does: 'Answers.' }, { name: 'WORDS', does: 'Holds.' }] },
      { path: 'README.md', change: 'new', items: [{ name: 'Words', does: 'Says so.' }] },
    ],
    suite: {
      runs: [
        { suite: 'cargo test', command: 'CARGO_TARGET_DIR=$TMPDIR/t cargo test', passed: 777, failed: 1 },
        { suite: 'plugin tests', command: 'python3 -m unittest', passed: 40, failed: 0 },
      ],
      not_run: [{ suite: 'e2e-panel.sh', why: 'Nests a niri on your screen.' }],
      failing: [{ test: 'words_are_new', file: 'src/words.rs', why: 'It expects old words.' }],
    },
    untested: [{ code: 'WORDS', file: 'src/words.rs', why: 'A constant.' }],
    plan_vs_done: [{ item: 'Add the words', kind: 'step', progress: 'partial', shown_by: 'abc1234' }],
  })
  const PLAN = { description: 'feat: New words', notes: ['Steps: Add the words'] }

  test('reads back as the report it was written from', () => {
    expect(reportFromMarkdown(reportMarkdown(full, PLAN))).toEqual({ report: full, plan: PLAN })
    expect(typeof twinOf(full, PLAN)).toBe('object')
  })

  test('the smallest branch report reads back too, its optional slides left out', () => {
    const min = branched()
    const md = reportMarkdown(min, PLAN)
    expect(reportFromMarkdown(md)).toEqual({ report: min, plan: PLAN })
    expect(md).not.toContain('## Plan vs done')
    expect(md).not.toContain('## Done when')
    expect(md).not.toContain('### Changed code with no test')
  })

  test('opens with the branch facts git gave, then the summary', () => {
    const lines = reportMarkdown(branched(), PLAN).split('\n')
    expect(lines.slice(0, 5)).toEqual([
      '# feat: New words',
      '',
      '_Branch `task/feat-new-words-0b8f6a52` from `main` at `f00dfee`: 2 commits, 2 files, +12 -3, 1 uncommitted._',
      '',
      'The branch adds new words.',
    ])
  })

  test('names each commit with its subject from git', () => {
    expect(reportMarkdown(branched(), PLAN)).toContain(
      '## How the branch got here\n\n### Wrote the words\n\n- `abc1234` feat: add the words\n- `abd5678` test: pin the words\n\n- The words went in.',
    )
  })

  test('says so when the branch adds no test', () => {
    const md = reportMarkdown(branched({ ...BRANCH_MIN, new_tests: [] }), PLAN)
    expect(md).toContain('## The tests this branch adds\n\n_None._')
    expect(reportFromMarkdown(md).report.branch?.new_tests).toEqual([])
  })

  test('a plan report keeps its twin as it was', () => {
    expect(reportMarkdown(MINIMAL, PLAN)).not.toContain('_Branch `')
    expect(reportFromMarkdown(reportMarkdown(MINIMAL, PLAN)).report.branch).toBeUndefined()
  })

  test('refuses a branch whose name would not read back', () => {
    const odd = { ...full, branch: { ...full.branch!, facts: { ...full.branch!.facts, branch: 'a`b' } } }
    expect(typeof twinOf(odd, PLAN)).toBe('string')
  })
})
