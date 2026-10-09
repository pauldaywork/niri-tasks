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
