// The page a report is written as: the mod's own head, whose policy lets the
// report run no script, then the report's fields as slides in one fixed
// order: what it is and what to decide first, a slide per idea, the decision
// last. A slide fills the screen, and the page snaps from one to the next as
// it scrolls, so moving through it needs no script. No `$`, so the unit tests
// reach it directly.

import { WRITE } from './plan'
import type { Plan } from './plan'
import type { Change, Check, Diagram, FileChange, Report, Section, Term } from './report'
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

// What to do with the report, said on the first slide and again on the last.
const NEXT =
  `<p class="next-step">Back in the terminal: choose <strong>${escapeHtml(WRITE)}</strong> ` +
  'to save this plan, or type what to change.</p>'

// What the colours mean, under a drawing that takes them.
const LEGEND = `<p class="legend">${badge('new')} ${badge('change')} ${badge('remove')} Everything else is unchanged.</p>`

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

const figure = (diagram: Diagram, lookAt: string, svg?: string): string => {
  // The change colours, or a highlighted block, show on a flowchart, a state
  // diagram, or a sequence diagram with a `rect`.
  const marked = 'mermaid' in diagram && (takesMarks(diagram.mermaid) || /^\s*rect\b/m.test(diagram.mermaid))
  const drawing =
    svg !== undefined
      ? `<div class="drawn">${svg}</div>`
      : 'mermaid' in diagram
        ? `<pre class="mermaid">${escapeHtml(withMarks(diagram.mermaid))}</pre>`
        : diagram.svg
  return [
    '<figure>',
    `<figcaption class="look"><strong>Look at:</strong> ${plain(lookAt)}</figcaption>`,
    drawing,
    marked ? LEGEND : '',
    '</figure>',
  ]
    .filter(Boolean)
    .join('\n')
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
    NEXT,
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

const part = (s: Section, svg?: string): string => {
  const points = bullets(s.points)
  return [
    `<h2>${plain(s.heading)}</h2>`,
    s.diagram !== undefined
      ? `<div class="split">\n${figure(s.diagram, s.look_at ?? '', svg)}\n<div class="points">${points}</div>\n</div>`
      : `<div class="points solo">${points}</div>`,
    s.detail !== undefined ? `<details>\n<summary>More detail</summary>\n${s.detail}\n</details>` : '',
  ]
    .filter(Boolean)
    .join('\n')
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

// The description and notes as the tool will write them, from the plan the
// person was shown, never from the report's fields: what they approve is
// what they read here, on the slide where they decide.
const decision = (plan: Plan): string => {
  const notes =
    plan.notes.length === 0
      ? '<p>No notes.</p>'
      : `<ol>${plan.notes.map(n => `<li>${escapeHtml(n)}</li>`).join('')}</ol>`
  return [
    '<h2>Your decision</h2>',
    '<h3>Exactly what will be written</h3>',
    `<p><strong>Description:</strong> ${escapeHtml(plan.description)}</p>`,
    notes,
    '<p class="muted">The tool also adds a note linking this report.</p>',
    NEXT,
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
    ...parts.map(
      (s, i): Slide => ({
        id: `part-${i + 1}`,
        pill: `Part ${i + 1} of ${parts.length}`,
        name: s.heading,
        ground: i % 2 === 0 ? 'paper' : 'blue',
        body: part(s, drawn[i]),
      }),
    ),
    ...(touched ? [{ id: 'files', pill: 'Files', name: 'Files', ground: 'paper', body: files(report.files) } as Slide] : []),
    { id: 'checks', pill: 'Done when', name: 'Done when', ground: 'green', body: checks(report.checks) },
    { id: 'decision', pill: 'Your decision', name: 'Your decision', ground: 'ink', body: decision(plan) },
  ]
  const total = rest.length + 1
  const meta =
    `About ${readingMinutes(report)} min to read · ${count(total, 'slide')}` +
    (touched ? ` · ${count(report.files.length, 'file')}` : '')
  // Mermaid in the browser only for a diagram the renderer did not draw.
  const browser = parts.some((s, i) => s.diagram !== undefined && 'mermaid' in s.diagram && drawn[i] === undefined)
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
