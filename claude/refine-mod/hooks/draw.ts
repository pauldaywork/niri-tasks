// The pure half of drawing a report's diagrams before the page opens: what
// the mod hands the shipped renderer, the fence it runs it in, and what it
// keeps of what comes back. No `$`, so the unit tests reach it directly.

import { withMarks } from './page'
import type { Report } from './report'

// The renderer the mod ships: beautiful-mermaid, bundled (render/README.md).
export const RENDERER = 'render/diagrams.mjs'

// The renderer runs outside the session's sandbox, on text the model wrote,
// so in a fence of its own: the whole file system read-only, a blank /tmp,
// no network and no namespace shared, gone with the mod.
export const drawArgv = (runtime: 'bun' | 'node', script: string): string[] => [
  'bwrap', '--ro-bind', '/', '/', '--dev', '/dev', '--proc', '/proc', '--tmpfs', '/tmp',
  '--unshare-all', '--die-with-parent', '--new-session', runtime, script,
]

// Each part's Mermaid source as it is to be drawn, flowcharts and state
// diagrams with the change colours; undefined for a part with none.
export const drawSources = (report: Report): (string | undefined)[] =>
  report.sections.map(s => (s.diagram !== undefined && 'mermaid' in s.diagram ? withMarks(s.diagram.mermaid) : undefined))

// What an SVG from the renderer may not hold: it is put in the page as it is,
// so nothing that runs, loads or links. beautiful-mermaid draws plain shapes
// and escapes its labels; this holds it to that.
const UNSAFE = /<\s*\/?\s*(script|foreignObject|iframe|object|embed|image|use|a)\b|\son[a-z]+\s*=|javascript:/i

// The web font beautiful-mermaid asks for, which the page's policy would
// refuse anyway, and its font, swapped for the one the page embeds.
export const cleanSvg = (svg: string): string =>
  svg.replace(/@import\s+url\([^)]*\)\s*;?/g, '').replace(/'Inter',/g, "'Report Body', 'Inter',")

// The SVG for each part, from the renderer's answer for the sources that
// were drawn, in order; undefined where it could not draw one, or drew
// something the page will not take.
export const drawnSvgs = (stdout: string, sources: readonly (string | undefined)[]): (string | undefined)[] => {
  let svgs: unknown
  try {
    svgs = (JSON.parse(stdout) as { svgs?: unknown }).svgs
  } catch {
    return sources.map(() => undefined)
  }
  const answers = Array.isArray(svgs) ? svgs : []
  let next = 0
  return sources.map(source => {
    if (source === undefined) return undefined
    const svg: unknown = answers[next++]
    if (typeof svg !== 'string' || !/^\s*<svg[\s>]/.test(svg) || UNSAFE.test(svg)) return undefined
    return cleanSvg(svg)
  })
}
