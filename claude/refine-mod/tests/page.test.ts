import { describe, expect, test } from 'claude-code/testing'
import { CSP, CSP_STATIC, MERMAID_SRI, MERMAID_URL, inline, readingMinutes, reportPage } from '../hooks/page'
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

// The ids of a page's slides, in order.
const slides = (page: string): string[] =>
  [...page.matchAll(/<section class="slide [^"]*" id="([^"]+)"/g)].map(m => m[1] ?? '')

describe('the head', () => {
  const page = reportPage(report(), PLAN)
  // A page with a Mermaid diagram left to the browser, so it has a script.
  const undrawn = reportPage(
    report({ sections: [{ heading: 'h', points: ['p'], look_at: 'x', diagram: { mermaid: 'flowchart LR\n a --> b' } }] }),
    PLAN,
  )

  test('puts the policy first, before anything it governs', () => {
    const csp = undrawn.indexOf(`<meta http-equiv="Content-Security-Policy" content="${CSP}">`)
    expect(csp).toBeGreaterThan(-1)
    expect(csp).toBeLessThan(undrawn.indexOf('<title>'))
    expect(csp).toBeLessThan(undrawn.indexOf('<style>'))
    expect(csp).toBeLessThan(undrawn.indexOf('<script'))
    expect(csp).toBeLessThan(undrawn.indexOf('<body>'))
    const fixed = page.indexOf(`<meta http-equiv="Content-Security-Policy" content="${CSP_STATIC}">`)
    expect(fixed).toBeGreaterThan(-1)
    expect(fixed).toBeLessThan(page.indexOf('<title>'))
    expect(fixed).toBeLessThan(page.indexOf('<style>'))
  })

  test('allows no script but pinned Mermaid, and nothing fetched or sent', () => {
    expect(CSP).toBe(
      `default-src 'none'; script-src ${MERMAID_URL}; style-src 'unsafe-inline'; ` +
        "img-src data:; font-src data:; form-action 'none'; base-uri 'none'",
    )
    expect(MERMAID_URL).toBe('https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js')
    expect(undrawn).toContain(`<script src="${MERMAID_URL}" integrity="${MERMAID_SRI}" crossorigin="anonymous"></script>`)
    expect(page).not.toContain('<script')
  })

  test("escapes the model's text", () => {
    expect(page).toContain('<title>feat: &lt;A&gt; &amp; &quot;B&quot;</title>')
    expect(page).toContain('<h1>feat: &lt;A&gt; &amp; &quot;B&quot;</h1>')
    expect(page).toContain('The task gets &lt;new&gt; words.')
  })

  test('embeds the fonts it is given, and none otherwise', () => {
    expect(reportPage(report(), PLAN)).not.toContain('@font-face')
    const page = reportPage(report(), PLAN, { display: 'RElTUA==', body: 'Qk9EWQ==' })
    expect(page).toContain('src: url(data:font/woff2;base64,RElTUA==) format("woff2")')
    expect(page).toContain('src: url(data:font/woff2;base64,Qk9EWQ==) format("woff2")')
  })

  test('snaps one slide to the screen, with nothing to run', () => {
    const page = reportPage(report(), PLAN)
    expect(page).toContain('scroll-snap-type: y mandatory')
    expect(page).toContain('scroll-snap-align: start')
    expect(page).not.toContain('<script')
  })
})

describe('the slides', () => {
  test('come in a fixed order, the optional ones only when given', () => {
    expect(slides(reportPage(report(), PLAN))).toEqual(['cover', 'glance', 'part-1', 'files', 'checks', 'decision'])
    const full = reportPage(
      report({
        needs_your_eye: ['Offline: diagrams show as text.'],
        terms: [{ term: 'Refine', meaning: 'Turning a task into a plan.' }],
        sections: [
          { heading: 'One', points: ['a'] },
          { heading: 'Two', points: ['b'] },
        ],
      }),
      PLAN,
    )
    expect(slides(full)).toEqual(['cover', 'glance', 'eye', 'terms', 'part-1', 'part-2', 'files', 'checks', 'decision'])
    expect(slides(reportPage(report({ files: [] }), PLAN))).toEqual(['cover', 'glance', 'part-1', 'checks', 'decision'])
  })

  test('each says where it is, and links to the next by name', () => {
    const page = reportPage(report(), PLAN)
    expect(page).toContain('<span class="count">01 / 06</span>')
    expect(page).toContain('<span class="count">06 / 06</span>')
    expect(page).toContain('<a class="next" href="#glance">Next: At a glance ↓</a>')
    expect(page).toContain('<a class="next" href="#part-1">Next: What changes ↓</a>')
    expect(page).toContain('<a class="next" href="#decision">Next: The plan ↓</a>')
    expect(page.split('<a class="next"').length - 1).toBe(5)
  })

  test('the rail links every slide', () => {
    const page = reportPage(report(), PLAN)
    expect(page).toContain('<nav class="rail" aria-label="Slides">')
    for (const id of slides(page)) expect(page).toContain(`<a href="#${id}"`)
  })

  test('a part has its own slide, numbered, its heading shown with code', () => {
    const page = reportPage(report({ sections: [{ heading: 'The `task` command', points: ['a'] }] }), PLAN)
    expect(page).toContain('<span class="pill">Part 1 of 1</span>')
    expect(page).toContain('<h2>The <code>task</code> command</h2>')
    expect(page).toContain('Next: The <code>task</code> command ↓')
  })

  test('says nothing about writing the task: a report explains, the terminal decides', () => {
    const page = reportPage(report(), PLAN)
    for (const words of [NEXT, 'Back in the terminal', 'Write it to the task', 'Your decision']) {
      expect(page).not.toContain(words)
    }
  })

  test('the cover says how long it takes and how much there is', () => {
    expect(reportPage(report(), PLAN)).toContain('<p class="meta">About 1 min to read · 6 slides · 1 file</p>')
    expect(reportPage(report({ files: [] }), PLAN)).toContain('<p class="meta">About 1 min to read · 5 slides</p>')
  })

  test('the glance puts what changes beside what stays the same', () => {
    const page = reportPage(report({ unchanged: ['Approval: still asked.'] }), PLAN)
    expect(page).toContain('<p class="lede">The task gets &lt;new&gt; words.</p>')
    expect(page).toContain('<h3>What changes</h3>')
    expect(page).toContain('<h3>What stays the same</h3>')
    expect(reportPage(report(), PLAN)).not.toContain('What stays the same')
  })

  test('words used here are a definition list', () => {
    const page = reportPage(report({ terms: [{ term: 'Refine', meaning: 'Turning a task into a plan.' }] }), PLAN)
    expect(page).toContain('<dt>Refine</dt><dd>Turning a task into a plan.</dd>')
  })

  test('shows the files with a labelled badge each, paths breaking only at a slash', () => {
    expect(reportPage(report(), PLAN)).toContain(
      '<tr><td><span class="badge change">changed</span></td><td><code class="path">src/<wbr>words.rs</code></td><td>Holds the words.</td></tr>',
    )
  })

  test('the cover names the report once, in its pill', () => {
    const page = reportPage(report(), PLAN)
    expect(page.split('Refine report').length - 1).toBe(1)
  })
})

describe('the decision slide', () => {
  test('repeats the plan, escaped, from the plan the person was shown', () => {
    const page = reportPage(report({ summary: 'Something else entirely.' }), PLAN)
    const decision = page.slice(page.indexOf('id="decision"'))
    expect(decision).toContain('<h2>The plan</h2>')
    expect(decision).toContain('<span class="pill">The plan</span>')
    expect(decision).toContain('<p><strong>Description:</strong> feat: &lt;A&gt; &amp; &quot;B&quot;</p>')
    expect(decision).toContain('<li><strong>Goal:</strong> &lt;x&gt; &amp; y</li>')
    expect(decision).toContain('<li><strong>Done when:</strong> <code>z</code> shows</li>')
    expect(decision).not.toContain('Exactly what will be written')
    expect(decision).not.toContain('also adds a note')
  })

  test('says so when the plan has no notes', () => {
    expect(reportPage(report(), { ...PLAN, notes: [] })).toContain('<p>No notes.</p>')
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
    expect(flow).toContain('classDef new fill:#99E885')
    expect(part({ mermaid: 'stateDiagram-v2\n [*] --> A' })).toContain('classDef change fill:#F7CB46')
    expect(part({ mermaid: 'sequenceDiagram\n A->>B: hi' })).not.toContain('classDef')
  })

  test('find the kind of diagram past comments, directives and front matter', () => {
    expect(part({ mermaid: "%%{init: {'theme': 'base'}}%%\n%% the flow\nflowchart LR\n a --> b" })).toContain(
      'classDef new fill:#99E885',
    )
    expect(part({ mermaid: '---\ntitle: The flow\n---\nflowchart LR\n a --> b' })).toContain('classDef new fill:#99E885')
    expect(part({ mermaid: '---\ntitle: flowchart\n---\nsequenceDiagram\n A->>B: hi' })).not.toContain('classDef')
    expect(part({ mermaid: '%% flowchart\nsequenceDiagram\n A->>B: hi' })).not.toContain('classDef')
  })

  test('pass SVG through as given', () => {
    expect(part({ svg: '<svg viewBox="0 0 1 1"><rect/></svg>' })).toContain('<svg viewBox="0 0 1 1"><rect/></svg>')
  })

  test('put detail in a collapsed block', () => {
    const page = reportPage(report({ sections: [{ heading: 'h', points: ['p'], detail: '<p>deep</p>' }] }), PLAN)
    expect(page).toContain('<details>\n<summary>More detail</summary>\n<p>deep</p>\n</details>')
  })

  test('sit beside the points, and a part without one has its points alone', () => {
    expect(part({ mermaid: 'flowchart LR\n a --> b' })).toContain('<div class="split">')
    const bare = reportPage(report(), PLAN)
    expect(bare).not.toContain('<div class="split">')
    expect(bare).toContain('<div class="points solo">')
  })

  test('carry the legend under the drawing, only where the change colours apply', () => {
    const flow = part({ mermaid: 'flowchart LR\n a --> b:::new' })
    expect(flow.indexOf('<p class="legend">')).toBeGreaterThan(flow.indexOf('<pre class="mermaid">'))
    expect(part({ mermaid: 'flowchart LR\n a --> b' })).not.toContain('<p class="legend">')
    expect(part({ mermaid: 'sequenceDiagram\n A->>B: hi' })).not.toContain('<p class="legend">')
    expect(part({ svg: '<svg viewBox="0 0 1 1"><rect/></svg>' })).not.toContain('<p class="legend">')
  })

  test('a drawn diagram replaces the Mermaid block, and a page with all drawn runs no script', () => {
    const page = reportPage(
      report({ sections: [{ heading: 'h', points: ['p'], look_at: 'x', diagram: { mermaid: 'flowchart LR\n a --> b' } }] }),
      PLAN,
      undefined,
      ['<svg viewBox="0 0 1 1"></svg>'],
    )
    expect(page).toContain('<div class="drawn" style="--w: 2px"><svg viewBox="0 0 1 1"></svg></div>')
    expect(page).not.toContain('<pre class="mermaid">')
    expect(page).not.toContain('<script')
    expect(page).toContain(`<meta http-equiv="Content-Security-Policy" content="${CSP_STATIC}">`)
    expect(CSP_STATIC).toBe(
      "default-src 'none'; script-src 'none'; style-src 'unsafe-inline'; " +
        "img-src data:; font-src data:; form-action 'none'; base-uri 'none'",
    )
  })

  test("a drawn diagram takes the renderer's colours and fonts from the page, scoped to it", () => {
    const page = reportPage(report(), PLAN)
    expect(page).toContain('.drawn svg { --_text: var(--fg);')
    expect(page).toContain('--_node-fill: var(--surface, color-mix(in srgb, var(--fg) 3%, var(--bg)));')
    expect(page).toContain(".drawn svg text { font-family: 'Report Body', 'Inter', system-ui, sans-serif; }")
    expect(page).not.toContain('@import')
  })

  // beautiful-mermaid keeps a class's fill but not its dash, so the page
  // dashes a drawn node in the `remove` pink, in any case.
  test('a drawn `remove` node keeps its dash', () => {
    expect(reportPage(report(), PLAN)).toContain('.drawn svg [fill="#FE90E8" i] { stroke-dasharray: 6 4; }')
  })

  test('a diagram left undrawn still gets Mermaid in the browser', () => {
    const flow = { mermaid: 'flowchart LR\n a --> b' }
    const page = reportPage(
      report({
        sections: [
          { heading: 'h', points: ['p'], look_at: 'x', diagram: flow },
          { heading: 'i', points: ['p'], look_at: 'x', diagram: flow },
        ],
      }),
      PLAN,
      undefined,
      ['<svg viewBox="0 0 1 1"></svg>', undefined],
    )
    expect(page.split('<pre class="mermaid">').length - 1).toBe(1)
    expect(page).toContain(`<script src="${MERMAID_URL}"`)
    expect(page).toContain(`<meta http-equiv="Content-Security-Policy" content="${CSP}">`)
  })

  test('a highlighted block in a sequence diagram carries a legend for new steps only', () => {
    const page = part({ mermaid: 'sequenceDiagram\n rect rgb(153, 232, 133)\n A->>B: hi\n end' })
    expect(page).toContain('<p class="legend"><span class="badge new">new</span> Highlighted steps are new.</p>')
    const figure = page.slice(page.indexOf('<figure>'), page.indexOf('</figure>'))
    expect(figure).not.toContain('<span class="badge change">')
    expect(figure).not.toContain('<span class="badge remove">')
    expect(part({ mermaid: 'flowchart LR\n a:::new --> b:::change --> c:::remove' })).toContain(
      '<p class="legend"><span class="badge new">new</span> <span class="badge change">changed</span> ' +
        '<span class="badge remove">removed</span> Everything else is unchanged.</p>',
    )
    expect(part({ mermaid: 'flowchart LR\n a --> b:::new' })).toContain(
      '<p class="legend"><span class="badge new">new</span> Everything else is unchanged.</p>',
    )
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
  test('leaves out detail, which is shown collapsed', () => {
    expect(readingMinutes(report({ sections: [{ heading: 'h', points: ['p'], detail: `<p>${words(600)}</p>` }] }))).toBe(1)
  })
  test('counts what is shown at 200 words a minute', () => {
    // Two parts of five 25-word points (250) and their one-word headings (2),
    // with the summary (5), the change (4), the file's why (3) and the check
    // (3 + 2): 269 words, so ceil(269 / 200) = 2.
    const points = Array.from({ length: 5 }, () => words(25))
    const sections = [
      { heading: 'One', points },
      { heading: 'Two', points },
    ]
    expect(readingMinutes(report({ sections }))).toBe(2)
  })
})

test('the worked example renders every part', () => {
  const parsed = parseReport(EXAMPLE)
  if (typeof parsed === 'string') throw new Error(parsed)
  const page = reportPage(parsed, EXAMPLE_PLAN)
  for (const section of parsed.sections) expect(page).toContain(section.heading)
  // A before/after pair and six single diagrams, all left to the browser here.
  expect(page.split('<pre class="mermaid">').length - 1).toBe(8)
})

describe('lens slides', () => {
  const flow = { mermaid: 'flowchart LR\n a --> b:::new' }

  test('come in lens order under their own names, plain parts numbered among themselves', () => {
    const page = reportPage(
      report({
        sections: [
          { kind: 'code', heading: 'The code', points: ['c'], code: [{ file: 'a.sh', before: 'x', after: 'y' }] },
          { heading: 'One', points: ['a'] },
          { kind: 'before-after', heading: 'Then and now', points: ['b'], look_at: 'x', before: flow, after: flow },
          { heading: 'Two', points: ['d'] },
        ],
      }),
      PLAN,
    )
    expect(slides(page)).toEqual(['cover', 'glance', 'before-after', 'part-1', 'part-2', 'code', 'files', 'checks', 'decision'])
    expect(page).toContain('<span class="pill">Before → after</span>')
    expect(page).toContain('<span class="pill">Part 2 of 2</span>')
    expect(page).toContain('<span class="pill">Code changes</span>')
  })

  test('a before/after pair sits side by side, Before first, and stacks when its diagrams are wide', () => {
    const pair = { kind: 'before-after' as const, heading: 'h', points: ['p'], look_at: 'x', before: flow, after: flow }
    const page = reportPage(report({ sections: [pair] }), PLAN)
    expect(page).toContain('<div class="pair">')
    expect(page.indexOf('<p class="pane-label">Before</p>')).toBeLessThan(page.indexOf('<p class="pane-label">After</p>'))
    const wide = '<svg viewBox="0 0 900 100"></svg>'
    expect(reportPage(report({ sections: [pair] }), PLAN, undefined, [undefined, wide, wide])).toContain(
      '<div class="pair rows">',
    )
  })

  test('code panes escape the code, and say when a file is new or removed', () => {
    const page = reportPage(
      report({
        sections: [
          {
            kind: 'code',
            heading: 'h',
            points: ['p'],
            code: [
              { file: 'src/a.rs', before: 'if a < b && c {', after: '' },
              { file: 'b.sh', before: '', after: 'echo "<hi>"' },
            ],
          },
        ],
      }),
      PLAN,
    )
    expect(page).toContain('<pre class="before">if a &lt; b &amp;&amp; c {</pre>')
    expect(page).toContain('<pre class="after"><span class="muted">(removed)</span></pre>')
    expect(page).toContain('<pre class="before"><span class="muted">(new file)</span></pre>')
    expect(page).toContain('<pre class="after">echo &quot;&lt;hi&gt;&quot;</pre>')
    expect(page).toContain('<code class="path">src/<wbr>a.rs</code>')
  })

  test('outside tools come as a table, each with a labelled swap badge', () => {
    const page = reportPage(
      report({
        sections: [
          {
            kind: 'outside-tools',
            heading: 'h',
            points: ['p'],
            tools: [{ tool: 'herdr', how: 'Runs the sessions.', swap: 'medium', why: 'Behind `src/session.rs`.' }],
          },
        ],
      }),
      PLAN,
    )
    expect(page).toContain('<span class="pill">Outside tools</span>')
    expect(page).toContain(
      '<tr><td><strong>herdr</strong></td><td>Runs the sessions.</td>' +
        '<td><span class="badge medium">some work to swap</span></td><td>Behind <code>src/session.rs</code>.</td></tr>',
    )
  })
})
