// The pure half of show_task_report: what it takes from the model, and the
// page it writes around it. No `$`, so the unit tests reach it directly.

import { HIDDEN } from './plan'

export type Report = { title: string; body: string }

// Mermaid's own bundle, pinned and checked: it renders every
// `<pre class="mermaid">` on load, so the page needs no script of its own.
export const MERMAID_URL = 'https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js'
export const MERMAID_SRI = 'sha384-EOXBFmc3gx5mb+vn0vPvvGqACToJD24hhacX5Yx+8NUUQrHIle/Qi5Bg9o3zKwW2'

// The page is the model's HTML in the person's own browser, outside the
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

export const MAX_TITLE = 120
export const MAX_BODY = 1_000_000

// Elements refused by name: a <meta> refresh navigates and a <meta> policy
// could be added to, which the page's own policy does not stop; the rest would
// reach outside or run script, and fail quietly under it. Refused so the model
// hears why rather than finding a blank figure.
const FORBIDDEN = /<\s*(script|meta|base|link|iframe|frameset|frame|object|embed|form|portal)(?![\w-])/i

// Answers the report, or a string saying what is wrong with it.
export const parseReport = (e: Record<string, unknown>): Report | string => {
  const { title, body } = e
  if (typeof title !== 'string' || title.trim() === '') return 'title must be a non-empty string'
  if (/[\r\n]/.test(title)) return 'title must be one line'
  if (HIDDEN.test(title)) return 'title must be plain text: no control or invisible characters'
  if (title.length > MAX_TITLE) return `title must be at most ${MAX_TITLE} characters`
  if (typeof body !== 'string' || body.trim() === '') return 'body must be a non-empty string'
  if (body.length > MAX_BODY) return `body must be at most ${MAX_BODY} characters`
  const tag = FORBIDDEN.exec(body)?.[1]?.toLowerCase()
  if (tag !== undefined) {
    return `body must not contain <${tag}>: the page runs no script but Mermaid, loads nothing and sends nothing`
  }
  return { title, body }
}

const escapeHtml = (text: string): string =>
  text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

// Light and dark from the system; the classes the report catalogue names.
// Mermaid draws its default (light) theme, so its figures sit on a light card.
const BASE_CSS = `
:root { color-scheme: light dark; --bg: #fafaf9; --fg: #1c1917; --muted: #57534e; --line: #d6d3d1;
  --card: #ffffff; --code: #f5f5f4; --add: #15803d; --change: #b45309; --remove: #b91c1c; --note: #1d4ed8; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #1c1917; --fg: #f5f5f4; --muted: #a8a29e; --line: #44403c;
    --card: #292524; --code: #292524; --add: #4ade80; --change: #fbbf24; --remove: #f87171; --note: #93c5fd; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg);
  font: 16px/1.55 system-ui, -apple-system, "Segoe UI", sans-serif; }
main { max-width: 64rem; margin: 0 auto; padding: 2rem 1rem 4rem; }
h1 { font-size: 1.8rem; line-height: 1.25; margin: 0 0 1rem; }
h2 { font-size: 1.3rem; margin: 2.5rem 0 0.75rem; padding-bottom: 0.25rem; border-bottom: 1px solid var(--line); }
h3 { font-size: 1.05rem; margin: 1.5rem 0 0.5rem; }
.lede { font-size: 1.15rem; }
.muted { color: var(--muted); }
code, pre { font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 0.9em; }
code { background: var(--code); padding: 0.1em 0.3em; border-radius: 4px; }
pre { background: var(--code); padding: 0.75rem 1rem; border-radius: 8px; overflow-x: auto; }
table { width: 100%; border-collapse: collapse; margin: 0.75rem 0; display: block; overflow-x: auto; }
th, td { text-align: left; vertical-align: top; padding: 0.4rem 0.75rem 0.4rem 0; border-top: 1px solid var(--line); }
th { color: var(--muted); font-weight: 600; border-top: 0; }
figure { margin: 1rem 0; }
figcaption { color: var(--muted); font-size: 0.9rem; margin-top: 0.4rem; }
pre.mermaid { background: #ffffff; color: #1c1917; border: 1px solid var(--line); text-align: center; }
svg { max-width: 100%; height: auto; }
.cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr)); gap: 1rem; }
.callout { border-left: 4px solid var(--note); background: var(--card); padding: 0.75rem 1rem; margin: 1rem 0; border-radius: 0 8px 8px 0; }
.badge, .risk { display: inline-block; font-size: 0.8rem; font-weight: 600; padding: 0.05rem 0.5rem;
  border-radius: 999px; border: 1px solid currentColor; white-space: nowrap; }
.badge.add, .risk.low { color: var(--add); }
.badge.change, .risk.medium { color: var(--change); }
.badge.remove, .risk.high { color: var(--remove); }
details { margin: 0.75rem 0; }
summary { cursor: pointer; font-weight: 600; }
`

// The whole page: the policy before anything it governs, then the title,
// the stylesheet and Mermaid, then the model's body as given.
export const reportPage = (report: Report): string =>
  [
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
    report.body,
    '</main>',
    '</body>',
    '</html>',
    '',
  ].join('\n')

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
