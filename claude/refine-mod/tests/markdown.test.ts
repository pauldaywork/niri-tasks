import { describe, expect, test } from 'claude-code/testing'
import { markdownPath, reportMarkdown } from '../hooks/markdown'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'
import { EXAMPLE } from './example-report'

// The smallest report the tool takes, as parseReport answers it.
const MINIMAL = parseReport({
  title: 'feat: New words',
  summary: 'The task gets new words.',
  changes: ['Words: the description changes.'],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
}) as Report

const PLAN = { description: 'feat: New words', notes: ['step one', 'step two'] }

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
    expect(md).toContain('## Flow\n\n_The arrow._\n\n```mermaid\nflowchart LR\n  a --> b\n```\n\n- p')
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
    expect(md).toContain('More detail:\n\n<p>More.</p>')
  })

  test('the worked example has every part as a heading', () => {
    const report = parseReport(EXAMPLE) as Report
    const md = reportMarkdown(report, PLAN)
    for (const s of report.sections) expect(md).toContain(`## ${s.heading}`)
  })

  test('a plan with no notes says so', () => {
    expect(reportMarkdown(MINIMAL, { description: 'd', notes: [] })).toContain('**Description:** d\n\nNo notes.\n')
  })
})
