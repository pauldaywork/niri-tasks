// The page a report is written as: the mod's own head, whose policy lets the
// report run no script, then the report's fields as slides in one fixed
// order: what it is and what to decide first, a slide per idea, the decision
// last. A slide fills the screen, and the page snaps from one to the next as
// it scrolls, so moving through it needs no script. No `$`, so the unit tests
// reach it directly.

import type { Plan } from './plan'
import { LENSES, SLOTS } from './report'
import type { Change, Check, CodePair, Diagram, FileChange, Lens, Report, Section, Swap, Term, Tool } from './report'
import { slideCss } from './style'
import type { Fonts } from './style'

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

// The policy when every diagram was drawn before the page opened: no script
// at all, Mermaid's included.
export const CSP_STATIC = CSP.replace(`script-src ${MERMAID_URL}`, "script-src 'none'")

// The colours `new`, `change` and `remove` take in a flowchart or a state
// diagram, the same fills as the badges: a diagram marks what the plan
// changes with `class <node> new` or `:::new`.
const MARKS = [
  'classDef new fill:#99E885,stroke:#000000,stroke-width:3px,color:#000000',
  'classDef change fill:#F7CB46,stroke:#000000,stroke-width:3px,color:#000000',
  'classDef remove fill:#FE90E8,stroke:#000000,stroke-width:3px,stroke-dasharray:6 4,color:#000000',
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

// Minutes to read what is shown, at 200 words a minute, plus half a minute a
// diagram; at least one. Detail, shown collapsed, is not counted. Shown
// first, so the reader knows what they are taking on.
export const readingMinutes = (report: Report): number => {
  const prose = [
    report.summary,
    ...report.changes,
    ...report.unchanged,
    ...report.needs_your_eye,
    ...report.terms.flatMap(t => [t.term, t.meaning]),
    ...report.sections.flatMap(s => [s.heading, s.look_at ?? '', ...s.points]),
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

const pad = (n: number): string => String(n).padStart(2, '0')

// What the colours mean, under a drawing that takes them; under a sequence
// diagram, whose only mark is a highlighted block of new steps, what that
// block means.
const STEPS_LEGEND = `<p class="legend">${badge('new')} Highlighted steps are new.</p>`

// What kind of Mermaid diagram a source is: its first word, past any front
// matter and `%%` comment or directive lines; '' when it has none.
export const diagramKind = (source: string): string => {
  const body = source.replace(/^\s*---[^\S\n]*\n[\s\S]*?\n\s*---[^\S\n]*(?:\n|$)/, '')
  return /^(?:\s*%%[^\n]*(?:\n|$))*\s*([A-Za-z][\w-]*)/.exec(body)?.[1] ?? ''
}

// Whether a Mermaid diagram takes the change colours: a flowchart or a state
// diagram.
export const takesMarks = (source: string): boolean => /^(?:flowchart|graph|stateDiagram)/.test(diagramKind(source))

// Flowcharts and state diagrams get the change colours after their own lines.
export const withMarks = (source: string): string => (takesMarks(source) ? `${source.trimEnd()}\n${MARKS}` : source)

// The legend a diagram carries: the change colours under a flowchart or a
// state diagram, the highlighted block under a sequence diagram with a
// `rect`, and none under any other.
// The change marks a diagram uses, in the legend's order: the legend names
// only those, so it never explains a colour the reader cannot see.
const marksUsed = (source: string): Change[] =>
  (['new', 'change', 'remove'] as const).filter(mark =>
    new RegExp(`(?::::|\\bclass\\s+[\\w,-]+\\s+)${mark}\\b`).test(source),
  )

const legendFor = (diagram: Diagram): string => {
  if (!('mermaid' in diagram)) return ''
  if (takesMarks(diagram.mermaid)) {
    const used = marksUsed(diagram.mermaid)
    return used.length === 0
      ? ''
      : `<p class="legend">${used.map(badge).join(' ')} Everything else is unchanged.</p>`
  }
  return /^\s*rect\b/m.test(diagram.mermaid) ? STEPS_LEGEND : ''
}

// A drawn diagram's width: its own width at one and a half times, so its
// labels (11 px in the renderer) read at about 16 px; the stylesheet caps it
// at the figure's width.
const drawnWidth = (svg: string): string => {
  const width = Number(/viewBox="\s*[-\d.]+\s+[-\d.]+\s+([\d.]+)\s+[\d.]+\s*"/.exec(svg)?.[1])
  return Number.isFinite(width) && width > 0 ? ` style="--w: ${Math.round(width * 1.5)}px"` : ''
}

// Whether a drawn diagram is wide and short: it then takes the card's whole
// width, the points below it, or its labels would shrink to fit a column.
const isWide = (svg?: string): boolean => {
  const box = svg && /viewBox="\s*[-\d.]+\s+[-\d.]+\s+([\d.]+)\s+([\d.]+)\s*"/.exec(svg)
  return box ? Number(box[1]) > 1.8 * Number(box[2]) : false
}

// One diagram as the page shows it: drawn before the page opened, else
// Mermaid in the browser, else the model's own SVG.
const drawing = (diagram: Diagram, svg?: string): string =>
  svg !== undefined
    ? `<div class="drawn"${drawnWidth(svg)}>${svg}</div>`
    : 'mermaid' in diagram
      ? `<pre class="mermaid">${escapeHtml(withMarks(diagram.mermaid))}</pre>`
      : diagram.svg

const figure = (diagram: Diagram, lookAt: string, svg?: string): string =>
  [
    '<figure>',
    `<figcaption class="look"><strong>Look at:</strong> ${plain(lookAt)}</figcaption>`,
    drawing(diagram, svg),
    legendFor(diagram),
    '</figure>',
  ]
    .filter(Boolean)
    .join('\n')

// The same thing as it is and as it will be, side by side, so the difference
// is seen rather than read.
const pairFigure = (before: Diagram, after: Diagram, lookAt: string, svgs: readonly (string | undefined)[]): string =>
  [
    '<figure class="pair-figure">',
    `<figcaption class="look"><strong>Look at:</strong> ${plain(lookAt)}</figcaption>`,
    `<div class="pair${isWide(svgs[1]) || isWide(svgs[2]) ? ' rows' : ''}">`,
    `<div class="pane"><p class="pane-label">Before</p>\n${drawing(before, svgs[1])}\n</div>`,
    `<div class="pane"><p class="pane-label">After</p>\n${drawing(after, svgs[2])}\n</div>`,
    '</div>',
    legendFor(after),
    '</figure>',
  ]
    .filter(Boolean)
    .join('\n')

// Code as it is and as it will be, a pair of panes per file.
const codePanes = (pairs: readonly CodePair[]): string =>
  pairs
    .map(c =>
      [
        '<div class="code-pair">',
        `<p class="code-file"><code class="path">${escapeHtml(c.file).replace(/\//g, '/<wbr>')}</code></p>`,
        '<div class="pair">',
        `<div class="pane"><p class="pane-label">Before</p><pre class="before">${c.before === '' ? '<span class="muted">(new file)</span>' : escapeHtml(c.before)}</pre></div>`,
        `<div class="pane"><p class="pane-label">After</p><pre class="after">${c.after === '' ? '<span class="muted">(removed)</span>' : escapeHtml(c.after)}</pre></div>`,
        '</div>',
        '</div>',
      ].join('\n'),
    )
    .join('\n')

const SWAP_LABELS: Record<Swap, string> = { easy: 'easy to swap', medium: 'some work to swap', hard: 'hard to swap' }

// The outside tools the change touches, how each is connected, and how hard
// it would be to swap for another.
const toolTable = (list: readonly Tool[]): string => {
  const rows = list
    .map(
      t =>
        `<tr><td><strong>${plain(t.tool)}</strong></td><td>${inline(t.how)}</td>` +
        `<td><span class="badge ${t.swap}">${SWAP_LABELS[t.swap]}</span></td><td>${inline(t.why)}</td></tr>`,
    )
    .join('')
  return [
    '<table class="tools">',
    '<thead><tr><th>Tool</th><th>How it connects</th><th>Swap</th><th>Why</th></tr></thead>',
    `<tbody>${rows}</tbody>`,
    '</table>',
  ].join('\n')
}

// A ground colour per kind of slide, so each chunk is told apart at a glance.
type Ground = 'cream' | 'yellow' | 'pink' | 'blue' | 'green' | 'paper' | 'ink'

// A slide: its id, the kind its pill names, the name the slide before links
// to it by, its ground, and its card's content.
type Slide = { id: string; pill: string; name: string; ground: Ground; body: string }

const cover = (report: Report, meta: string): string =>
  [
    `<h1>${plain(report.title)}</h1>`,
    `<p class="meta">${meta}</p>`,
    '<p class="hint">Scroll, or press Page Down, to go through it one slide at a time.</p>',
  ].join('\n')

const glance = (report: Report): string =>
  [
    `<p class="lede">${plain(report.summary)}</p>`,
    '<div class="cols">',
    `<div>\n<h3>What changes</h3>\n${bullets(report.changes)}\n</div>`,
    report.unchanged.length > 0 ? `<div>\n<h3>What stays the same</h3>\n${bullets(report.unchanged)}\n</div>` : '',
    '</div>',
  ]
    .filter(Boolean)
    .join('\n')

const eye = (lines: readonly string[]): string => `<h2>Where your judgement is needed</h2>\n${bullets(lines)}`

const terms = (list: readonly Term[]): string =>
  `<h2>Words used here</h2>\n<dl>${list.map(t => `<dt>${plain(t.term)}</dt><dd>${inline(t.meaning)}</dd>`).join('')}</dl>`

const part = (s: Section, svgs: readonly (string | undefined)[]): string => {
  const points = bullets(s.points)
  const extras = [
    s.code !== undefined ? codePanes(s.code) : '',
    s.tools !== undefined ? toolTable(s.tools) : '',
  ].filter(Boolean)
  // A pair, code panes or a table take the card's width, the points below.
  const visual =
    s.before !== undefined && s.after !== undefined
      ? `<div class="stack">\n${pairFigure(s.before, s.after, s.look_at ?? '', svgs)}\n${extras.join('\n')}\n<div class="points">${points}</div>\n</div>`
      : s.diagram !== undefined && extras.length === 0
        ? `<div class="split${isWide(svgs[0]) ? ' wide' : ''}">\n${figure(s.diagram, s.look_at ?? '', svgs[0])}\n<div class="points">${points}</div>\n</div>`
        : s.diagram !== undefined || extras.length > 0
          ? `<div class="stack">\n${s.diagram !== undefined ? figure(s.diagram, s.look_at ?? '', svgs[0]) : ''}\n${extras.join('\n')}\n<div class="points">${points}</div>\n</div>`
          : `<div class="points solo">${points}</div>`
  return [
    `<h2>${plain(s.heading)}</h2>`,
    visual,
    s.detail !== undefined ? `<details>\n<summary>More detail</summary>\n${s.detail}\n</details>` : '',
  ]
    .filter(Boolean)
    .join('\n')
}

// Each lens's slide name and ground: the same every report, so the reader
// learns where to look.
const LENS_SLIDES: Record<Lens, { pill: string; ground: Ground }> = {
  'before-after': { pill: 'Before → after', ground: 'cream' },
  part: { pill: 'Part', ground: 'paper' },
  structure: { pill: 'Structure', ground: 'blue' },
  'data-flow': { pill: 'Data flow', ground: 'paper' },
  'outside-tools': { pill: 'Outside tools', ground: 'cream' },
  styling: { pill: 'Styling', ground: 'pink' },
  code: { pill: 'Code changes', ground: 'paper' },
  newcomer: { pill: 'For a newcomer', ground: 'green' },
}

const files = (list: readonly FileChange[]): string => {
  const rows = list
    .map(
      f =>
        `<tr><td>${badge(f.change)}</td><td><code class="path">${escapeHtml(f.path).replace(/\//g, '/<wbr>')}</code></td>` +
        `<td>${inline(f.why)}</td></tr>`,
    )
    .join('')
  return [
    '<h2>Files</h2>',
    '<table>',
    '<thead><tr><th>Change</th><th>File</th><th>Why</th></tr></thead>',
    `<tbody>${rows}</tbody>`,
    '</table>',
  ].join('\n')
}

const checks = (list: readonly Check[]): string => {
  const rows = list.map(c => `<tr><td>${inline(c.check)}</td><td>${inline(c.how)}</td></tr>`).join('')
  return [
    '<h2>Done when</h2>',
    '<table>',
    '<thead><tr><th>Check</th><th>How it is checked</th></tr></thead>',
    `<tbody>${rows}</tbody>`,
    '</table>',
  ].join('\n')
}

// The plan itself, description and notes, from the plan the person was
// shown, never from the report's fields.
const decision = (plan: Plan): string => {
  const notes =
    plan.notes.length === 0
      ? '<p>No notes.</p>'
      : `<ol>${plan.notes.map(n => `<li>${inline(n)}</li>`).join('')}</ol>`
  return [
    '<h2>The plan</h2>',
    '<div class="written">',
    `<p><strong>Description:</strong> ${escapeHtml(plan.description)}</p>`,
    notes,
    '</div>',
  ].join('\n')
}

// One slide on the screen: what kind it is and where, its card, and the way
// on to the next, named.
const frame = (slide: Slide, i: number, all: readonly Slide[]): string => {
  const next = all[i + 1]
  return [
    `<section class="slide ${slide.ground}" id="${slide.id}" aria-label="Slide ${i + 1} of ${all.length}">`,
    '<div class="bar">',
    `<span class="pill">${escapeHtml(slide.pill)}</span>`,
    `<span class="count">${pad(i + 1)} / ${pad(all.length)}</span>`,
    '</div>',
    `<div class="card">\n${slide.body}\n</div>`,
    next !== undefined ? `<a class="next" href="#${next.id}">Next: ${plain(next.name)} ↓</a>` : '',
    '</section>',
  ]
    .filter(Boolean)
    .join('\n')
}

// Every slide's number, pinned to the side: where you are, and a way back.
const rail = (all: readonly Slide[]): string => {
  const items = all
    .map((s, i) => `<li><a href="#${s.id}" title="${escapeHtml(s.name)}">${pad(i + 1)}</a></li>`)
    .join('')
  return `<nav class="rail" aria-label="Slides"><ol>${items}</ol></nav>`
}

// The whole page: the policy before anything it governs, then the title, the
// stylesheet and Mermaid, then the slides in their fixed order.
// The parts as slides, in lens order (the model's order within a lens): plain
// parts numbered among themselves, every other lens under its own name.
const lensSlides = (parts: readonly Section[], drawn: readonly (string | undefined)[]): Slide[] => {
  const order = parts
    .map((s, i) => ({ s, i }))
    .sort((a, b) => LENSES.indexOf(a.s.kind ?? 'part') - LENSES.indexOf(b.s.kind ?? 'part') || a.i - b.i)
  const numbered = order.filter(o => (o.s.kind ?? 'part') === 'part')
  const seen = new Map<Lens, number>()
  return order.map(({ s, i }): Slide => {
    const kind = s.kind ?? 'part'
    const n = (seen.get(kind) ?? 0) + 1
    seen.set(kind, n)
    const lens = LENS_SLIDES[kind]
    const svgs = drawn.slice(i * SLOTS, i * SLOTS + SLOTS)
    return kind === 'part'
      ? {
          id: `part-${n}`,
          pill: `Part ${n} of ${numbered.length}`,
          name: s.heading,
          ground: n % 2 === 1 ? 'paper' : 'blue',
          body: part(s, svgs),
        }
      : { id: n === 1 ? kind : `${kind}-${n}`, pill: lens.pill, name: s.heading, ground: lens.ground, body: part(s, svgs) }
  })
}

export const reportPage = (
  report: Report,
  plan: Plan,
  fonts?: Fonts,
  drawn: readonly (string | undefined)[] = [],
): string => {
  const parts = report.sections
  const touched = report.files.length > 0
  const rest: Slide[] = [
    { id: 'glance', pill: 'At a glance', name: 'At a glance', ground: 'yellow', body: glance(report) },
    ...(report.needs_your_eye.length > 0
      ? [{ id: 'eye', pill: 'Needs your eye', name: 'Needs your eye', ground: 'pink', body: eye(report.needs_your_eye) } as Slide]
      : []),
    ...(report.terms.length > 0
      ? [{ id: 'terms', pill: 'Words used here', name: 'Words used here', ground: 'blue', body: terms(report.terms) } as Slide]
      : []),
    ...lensSlides(parts, drawn),
    ...(touched ? [{ id: 'files', pill: 'Files', name: 'Files', ground: 'paper', body: files(report.files) } as Slide] : []),
    { id: 'checks', pill: 'Done when', name: 'Done when', ground: 'green', body: checks(report.checks) },
    { id: 'decision', pill: 'The plan', name: 'The plan', ground: 'ink', body: decision(plan) },
  ]
  const total = rest.length + 1
  const meta =
    `About ${readingMinutes(report)} min to read · ${count(total, 'slide')}` +
    (touched ? ` · ${count(report.files.length, 'file')}` : '')
  // Mermaid in the browser only for a diagram the renderer did not draw.
  const browser = parts.some((s, i) =>
    [s.diagram, s.before, s.after].some((d, slot) => d !== undefined && 'mermaid' in d && drawn[i * SLOTS + slot] === undefined),
  )
  const all: Slide[] = [{ id: 'cover', pill: 'Refine report', name: 'Start', ground: 'cream', body: cover(report, meta) }, ...rest]
  return [
    '<!doctype html>',
    '<html lang="en">',
    '<head>',
    `<meta http-equiv="Content-Security-Policy" content="${browser ? CSP : CSP_STATIC}">`,
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${escapeHtml(report.title)}</title>`,
    `<style>${slideCss(fonts)}</style>`,
    browser ? `<script src="${MERMAID_URL}" integrity="${MERMAID_SRI}" crossorigin="anonymous"></script>` : '',
    '</head>',
    '<body>',
    rail(all),
    '<main class="deck">',
    ...all.map((s, i) => frame(s, i, all)),
    '</main>',
    '</body>',
    '</html>',
  ]
    .filter(Boolean)
    .join('\n') + '\n'
}
