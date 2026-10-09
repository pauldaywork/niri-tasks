import { describe, expect, test } from 'claude-code/testing'
import { LIMITS, madeAlready, openArgv, parseReport, reportPath, reportsDir, unreportable, withReportNote, withReportNotes } from '../hooks/report'
import type { Task } from '../hooks/plan'
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

  test('takes a plan that touches no file', () => {
    expect(parseReport({ ...MINIMAL, files: [] })).toEqual({
      ...MINIMAL,
      files: [],
      unchanged: [],
      needs_your_eye: [],
      terms: [],
    })
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

describe('parseReport keeps the page to one layout', () => {
  const LAYOUT = 'the page keeps one layout'

  test('refuses an HTML comment in detail or SVG', () => {
    expect(parseReport(detailed('<p>a</p><!-- b -->'))).toBe(
      `sections[0].detail must not contain an HTML comment: ${LAYOUT}`,
    )
    expect(parseReport(drawn({ svg: '<svg><!-- b --></svg>' }))).toBe(
      `sections[0].diagram.svg must not contain an HTML comment: ${LAYOUT}`,
    )
  })

  test("refuses closing the page's own structure", () => {
    for (const tag of ['details', 'section', 'main', 'footer', 'nav', 'figure', 'header', 'body', 'html']) {
      expect(parseReport(detailed(`<p>a</p></${tag}>`))).toBe(
        `sections[0].detail must not close the page's <${tag}>: ${LAYOUT}`,
      )
    }
    expect(parseReport(detailed('< / Section >'))).toBe(`sections[0].detail must not close the page's <section>: ${LAYOUT}`)
    expect(parseReport(drawn({ svg: '<svg></figure></svg>' }))).toBe(
      `sections[0].diagram.svg must not close the page's <figure>: ${LAYOUT}`,
    )
  })

  test('refuses elements in detail that swallow or restyle the page', () => {
    const tags = ['style', 'plaintext', 'xmp', 'textarea', 'title', 'noscript', 'template', 'html', 'head', 'body']
    for (const tag of tags) {
      expect(parseReport(detailed(`<p>a</p>< ${tag.toUpperCase()} x>`))).toBe(
        `sections[0].detail must not contain <${tag}>: ${LAYOUT}`,
      )
    }
  })

  test('refuses <style> in SVG, and keeps its <title>', () => {
    expect(parseReport(drawn({ svg: '<svg><style>p{}</style></svg>' }))).toBe(
      `sections[0].diagram.svg must not contain <style>: ${LAYOUT}`,
    )
    expect(typeof parseReport(drawn({ svg: '<svg viewBox="0 0 1 1"><title>A box</title><rect/></svg>' }))).toBe('object')
  })

  test('keeps names that only begin like a refused one, and escaped markup', () => {
    for (const detail of ['<titles>x</titles>', '<styled-box/>', '</sections>', '</main-part>', '<p>&lt;!-- x --&gt;</p>']) {
      expect(typeof parseReport(detailed(detail))).toBe('object')
    }
  })

  test('leaves Mermaid, whose source is escaped, to the rules for every diagram', () => {
    expect(typeof parseReport(drawn({ mermaid: 'flowchart LR\n %% <!-- a -->\n a["</section> <style>"] --> b' }))).toBe('object')
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

describe('withReportNotes', () => {
  const plan = { description: 'feat: A', notes: ['Goal: x'] }

  test("adds a note linking a report made for this very plan, after the plan's own", () => {
    expect(withReportNotes([{ path: '/r/a.html', plan }], plan)).toEqual(['Goal: x', 'Report: /r/a.html'])
  })

  test('marks a report made before the plan changed', () => {
    expect(withReportNotes([{ path: '/r/a.html', plan }], { ...plan, notes: ['Goal: y'] })).toEqual([
      'Goal: y',
      'Report (earlier draft): /r/a.html',
    ])
  })

  test('relabels a kept note for a report of this session once the plan is revised', () => {
    const revised = { ...plan, notes: ['Goal: y', 'Report: /r/a.html'] }
    expect(withReportNotes([{ path: '/r/a.html', plan }], revised)).toEqual([
      'Goal: y',
      'Report (earlier draft): /r/a.html',
    ])
  })

  test('keeps a kept note for an unrevised plan once, as it was', () => {
    const kept = { ...plan, notes: ['Goal: x', 'Report: /r/a.html'] }
    expect(withReportNotes([{ path: '/r/a.html', plan }], kept)).toEqual(['Goal: x', 'Report: /r/a.html'])
    // A second report on the same plan: the first's note does not make it differ.
    const made = [
      { path: '/r/a.html', plan },
      { path: '/r/b.html', plan: kept },
    ]
    expect(withReportNotes(made, kept)).toEqual(['Goal: x', 'Report: /r/a.html', 'Report: /r/b.html'])
  })

  test("keeps an earlier session's report note in its place", () => {
    const notes = ['Goal: x', 'Report: /old/z.html', 'Done when: y']
    expect(withReportNotes([{ path: '/r/a.html', plan }], { ...plan, notes })).toEqual([
      ...notes,
      'Report (earlier draft): /r/a.html',
    ])
  })

  test('links each report once, in the order made', () => {
    const made = [
      { path: '/r/a.html', plan: { ...plan, description: 'feat: Old' } },
      { path: '/r/b.html', plan },
    ]
    expect(withReportNotes(made, plan)).toEqual(['Goal: x', 'Report (earlier draft): /r/a.html', 'Report: /r/b.html'])
  })

  test("none made: the plan's own notes", () => {
    expect(withReportNotes([], plan)).toEqual(['Goal: x'])
  })
})

describe('lenses', () => {
  const flow = { mermaid: 'flowchart LR\n a --> b' }
  const at = (over: Record<string, unknown>) => ({
    ...MINIMAL,
    sections: [{ heading: 'h', points: ['p'], ...over }],
  })

  test('a part may name its lens, and one that does not is a plain part', () => {
    const parsed = parseReport(at({ kind: 'data-flow', look_at: 'x', diagram: flow }))
    expect(typeof parsed !== 'string' && parsed.sections[0]?.kind).toBe('data-flow')
    expect(parseReport(at({ kind: 'gossip' }))).toBe(
      'sections[0].kind must be one of before-after, part, structure, data-flow, outside-tools, styling, code, newcomer',
    )
  })

  test('a before/after pair needs both halves, a look_at line, and no diagram beside it', () => {
    expect(typeof parseReport(at({ kind: 'before-after', look_at: 'x', before: flow, after: flow }))).toBe('object')
    expect(parseReport(at({ look_at: 'x', before: flow }))).toBe('sections[0] needs both before and after')
    expect(parseReport(at({ before: flow, after: flow }))).toBe('sections[0].look_at must be a non-empty string')
    expect(parseReport(at({ look_at: 'x', diagram: flow, before: flow, after: flow }))).toBe(
      'sections[0] has a diagram and a before/after pair; give one',
    )
  })

  test('code pairs: a file, a before or an after, and no more than fits a pane', () => {
    const pair = (over: Record<string, unknown>) => at({ kind: 'code', code: [{ file: 'a.sh', before: 'x', after: 'y', ...over }] })
    expect(typeof parseReport(pair({}))).toBe('object')
    expect(typeof parseReport(pair({ before: '' }))).toBe('object')
    expect(parseReport(pair({ before: '', after: '' }))).toBe('sections[0].code[0] needs a before or an after')
    const long = Array.from({ length: LIMITS.code.lines + 1 }, () => 'line').join('\n')
    expect(parseReport(pair({ after: long }))).toBe(
      `sections[0].code[0].after has ${LIMITS.code.lines + 1} lines; at most ${LIMITS.code.lines}`,
    )
    expect(parseReport(pair({ after: 'a\u0007b' }))).toBe(
      'sections[0].code[0].after must hold no control characters but tabs and line breaks',
    )
    expect(typeof parseReport(pair({ after: 'a\tb\nc' }))).toBe('object')
  })

  test('outside tools: each rated easy, medium or hard to swap', () => {
    const tools = (swap: string) =>
      at({ kind: 'outside-tools', tools: [{ tool: 'herdr', how: 'Runs the sessions.', swap, why: 'One module.' }] })
    expect(typeof parseReport(tools('medium'))).toBe('object')
    expect(parseReport(tools('trivial'))).toBe('sections[0].tools[0].swap must be easy, medium or hard')
  })
})

const planned = (over: Partial<Task> = {}): Task => ({
  uuid: UUID,
  status: 'pending',
  description: 'feat: Old words',
  annotations: [{ entry: '20260101T000001Z', description: 'first note' }],
  tags: ['planned', 'zeta'],
  ...over,
})

describe('report mode', () => {
  test('a pending, planned task can be reported on', () => {
    expect(unreportable(planned())).toBeUndefined()
  })

  test('a task that is gone, not pending or not planned cannot, and the reason says which', () => {
    expect(unreportable(undefined)).toBe('the task no longer exists')
    expect(unreportable(planned({ status: 'completed' }))).toBe('the task is completed, not pending')
    expect(unreportable(planned({ tags: ['zeta'] }))).toBe('the task has no plan: it is not tagged planned. Refine it first')
    expect(unreportable(planned({ tags: undefined }))).toBe('the task has no plan: it is not tagged planned. Refine it first')
  })

  test('the report note goes after the others, at now, with everything else kept', () => {
    const now = Date.UTC(2026, 9, 6, 1, 2, 3)
    expect(withReportNote(planned(), '/r/refine-0b8f6a52-20261006-010203.html', now)).toEqual({
      ...planned(),
      annotations: [
        { entry: '20260101T000001Z', description: 'first note' },
        { entry: '20261006T010203Z', description: 'Report: /r/refine-0b8f6a52-20261006-010203.html' },
      ],
    })
  })

  test('a task with no notes yet gets its first', () => {
    const task = withReportNote(planned({ annotations: undefined }), '/r/x.html', Date.UTC(2026, 9, 6, 1, 2, 3))
    expect(task.annotations).toEqual([{ entry: '20261006T010203Z', description: 'Report: /r/x.html' }])
  })

  test('the note never takes a second a note already has: Taskwarrior keys a note by it', () => {
    const now = Date.UTC(2026, 9, 6, 1, 2, 3)
    const task = planned({
      annotations: [
        { entry: '20261006T010203Z', description: 'at now' },
        { entry: '20261006T010204Z', description: 'a second later' },
      ],
    })
    expect(withReportNote(task, '/r/x.html', now).annotations?.at(-1)).toEqual({
      entry: '20261006T010205Z',
      description: 'Report: /r/x.html',
    })
  })

  test('a second report in one session is refused, naming the first', () => {
    expect(madeAlready('/r/x.html')).toBe(
      "show_task_report: this session's report is made and linked from the task: /r/x.html. " +
        'Press Report on the card again for another.',
    )
  })
})
