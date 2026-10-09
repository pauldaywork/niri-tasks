import { describe, expect, test } from 'claude-code/testing'
import { CSP, MERMAID_SRI, MERMAID_URL, inline, readingMinutes, reportPage } from '../hooks/page'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'
import { EXAMPLE, EXAMPLE_PLAN } from './example-report'

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

// The plan the person was shown, which the page repeats as it will be written.
const PLAN = { description: 'feat: <A> & "B"', notes: ['Goal: <x> & y', 'Done when: `z` shows'] }

const words = (n: number) => Array.from({ length: n }, () => 'word').join(' ')
const NEXT = 'Back in the terminal: choose <strong>Write it to the task</strong> to save this plan, or type what to change.'

describe('the head', () => {
  const page = reportPage(report(), PLAN)

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
    const page = reportPage(report(), PLAN)
    const at = (id: string) => page.indexOf(`id="${id}"`)
    expect(at('glance')).toBeGreaterThan(-1)
    expect(at('glance')).toBeLessThan(at('part-1'))
    expect(at('part-1')).toBeLessThan(at('files'))
    expect(at('files')).toBeLessThan(at('checks'))
    expect(page.split(NEXT).length - 1).toBe(2)
  })

  test('says how long it takes and how much there is', () => {
    expect(reportPage(report(), PLAN)).toContain('<p class="meta">About 1 min to read · 1 part · 1 file</p>')
  })

  test('lists the parts, numbered, in the contents bar', () => {
    const page = reportPage(report({ sections: [{ heading: 'One', points: ['a'] }, { heading: 'Two', points: ['b'] }] }), PLAN)
    expect(page).toContain('<a href="#part-1">1. One</a>')
    expect(page).toContain('<a href="#part-2">2. Two</a>')
    expect(page).toContain('<span class="num">2/2</span> Two')
  })

  test('shows the optional blocks only when given', () => {
    const bare = reportPage(report(), PLAN)
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
      PLAN,
    )
    for (const text of ['What stays the same', 'Needs your eye', 'Words used here', 'class="legend"']) {
      expect(full).toContain(text)
    }
    expect(full).toContain('<dt>Refine</dt><dd>Turning a task into a plan.</dd>')
  })

  test('shows the files with a labelled badge each', () => {
    expect(reportPage(report(), PLAN)).toContain(
      '<tr><td><span class="badge change">changed</span></td><td><code>src/words.rs</code></td><td>Holds the words.</td></tr>',
    )
  })
})

describe('what will be written', () => {
  const BLOCK =
    '<section id="plan"><h2>Exactly what will be written</h2><details>' +
    '<summary>The description and notes you approve in the terminal</summary>'

  test('repeats the plan, escaped, from the plan the person was shown', () => {
    expect(reportPage(report(), PLAN)).toContain(
      `${BLOCK}<p><strong>Description:</strong> feat: &lt;A&gt; &amp; &quot;B&quot;</p>` +
        '<ol><li>Goal: &lt;x&gt; &amp; y</li><li>Done when: `z` shows</li></ol>' +
        '<p class="muted">The tool also adds a note linking this report.</p></details></section>',
    )
  })

  test('sits after the last part and before the files, with its own link', () => {
    const page = reportPage(report({ sections: [{ heading: 'One', points: ['a'] }, { heading: 'Two', points: ['b'] }] }), PLAN)
    expect(page.indexOf('id="plan"')).toBeGreaterThan(page.indexOf('id="part-2"'))
    expect(page.indexOf('id="plan"')).toBeLessThan(page.indexOf('id="files"'))
    const link = page.indexOf('<li><a href="#plan">What will be written</a></li>')
    expect(link).toBeGreaterThan(page.indexOf('href="#part-2"'))
    expect(link).toBeLessThan(page.indexOf('href="#files"'))
  })

  test('says so when the plan has no notes', () => {
    const page = reportPage(report(), { description: 'feat: A', notes: [] })
    expect(page).toContain('<p><strong>Description:</strong> feat: A</p><p>No notes.</p><p class="muted">')
    const block = page.slice(page.indexOf('<section id="plan">'), page.indexOf('<section id="files">'))
    expect(block).not.toContain('<ol>')
  })
})

describe('diagrams', () => {
  const part = (diagram: { mermaid: string } | { svg: string }) =>
    reportPage(report({ sections: [{ heading: 'h', points: ['p'], look_at: 'The new box.', diagram }] }), PLAN)

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
    const page = reportPage(report({ sections: [{ heading: 'h', points: ['p'], detail: '<p>deep</p>' }] }), PLAN)
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
  const page = reportPage(parsed, EXAMPLE_PLAN)
  for (const section of parsed.sections) expect(page).toContain(section.heading)
  expect(page.split('<pre class="mermaid">').length - 1).toBe(2)
})
