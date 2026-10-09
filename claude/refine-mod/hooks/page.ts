// The page a report is written as: the mod's own head, whose policy lets the
// report run no script, then the report's fields in one fixed layout, so
// every report reads the same way: what it does and what to decide first,
// detail last. No `$`, so the unit tests reach it directly.

import { WRITE } from './plan'
import type { Plan } from './plan'
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
.muted { color: var(--muted); }
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
    ['plan', 'What will be written'],
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

// The description and notes as the tool will write them, from the plan the
// person was shown, never from the report's fields: what they approve is
// what they read here.
const written = (plan: Plan): string => {
  const notes =
    plan.notes.length === 0
      ? '<p>No notes.</p>'
      : `<ol>${plan.notes.map(n => `<li>${escapeHtml(n)}</li>`).join('')}</ol>`
  return (
    '<section id="plan"><h2>Exactly what will be written</h2><details>' +
    '<summary>The description and notes you approve in the terminal</summary>' +
    `<p><strong>Description:</strong> ${escapeHtml(plan.description)}</p>${notes}` +
    '<p class="muted">The tool also adds a note linking this report.</p></details></section>'
  )
}

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
// stylesheet and Mermaid, then the report in its fixed order, with the plan
// the person was shown just before the files.
export const reportPage = (report: Report, plan: Plan): string => {
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
    written(plan),
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
