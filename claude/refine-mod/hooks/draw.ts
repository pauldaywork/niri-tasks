// The pure half of drawing a report's diagrams before the page opens: what
// the mod hands the shipped renderer, the fence it runs it in, and what it
// keeps of what comes back. No `$`, so the unit tests reach it directly.

import { diagramKind, withMarks } from './page'
import { CLOSES, COMMENT, WHOLE_SVG } from './report'
import type { Report } from './report'

// The renderer the mod ships: beautiful-mermaid, bundled (render/README.md).
export const RENDERER = 'render/diagrams.mjs'

// Finds a runtime by name and resolves it to its real path, the name passed
// as an argument, never as script.
export const whichArgv = (runtime: 'bun' | 'node'): string[] => [
  'sh', '-c', 'readlink -f "$(command -v "$1")"', 'sh', runtime,
]

// The runtime's real path from whichArgv's answer; undefined when it has none.
export const runtimePath = (run: { exitCode: number; stdout: string }): string | undefined => {
  const path = run.stdout.trim()
  return run.exitCode === 0 && path.startsWith('/') ? path : undefined
}

// The renderer runs outside the session's sandbox, on text the model wrote,
// so in a fence of its own: only /usr, the runtime and the script visible,
// all read-only, a blank /tmp, no network, no host sockets, no environment
// and no namespace shared, gone with the mod.
export const drawArgv = (runtimePath: string, script: string): string[] => [
  'bwrap', '--ro-bind', '/usr', '/usr',
  '--symlink', 'usr/lib', '/lib', '--symlink', 'usr/lib64', '/lib64',
  '--symlink', 'usr/bin', '/bin', '--symlink', 'usr/sbin', '/sbin',
  '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp',
  '--ro-bind', runtimePath, '/run/r/runtime', '--ro-bind', script, '/run/r/diagrams.mjs',
  '--unshare-all', '--die-with-parent', '--new-session', '--clearenv', '--setenv', 'PATH', '/usr/bin',
  '/run/r/runtime', '/run/r/diagrams.mjs',
]

// A change mark in a diagram: a `class <ids> new` line, or `:::new` after a
// node, for each of the three changes.
const MARKED = /^\s*class\s+\S[^\n]*\s(?:new|change|remove)\s*;?\s*$|:::(?:new|change|remove)(?![\w-])/m

// beautiful-mermaid draws a state diagram without its classes, so one that
// marks a change would lose its colours: Mermaid in the browser draws it.
const drawable = (source: string): boolean => !(diagramKind(source).startsWith('stateDiagram') && MARKED.test(source))

// Each part's Mermaid source as it is to be drawn, flowcharts and state
// diagrams with the change colours; undefined for a part with none, or with
// one left to the browser.
export const drawSources = (report: Report): (string | undefined)[] =>
  report.sections.map(s =>
    s.diagram !== undefined && 'mermaid' in s.diagram && drawable(s.diagram.mermaid)
      ? withMarks(s.diagram.mermaid)
      : undefined,
  )

// What an SVG from the renderer may not hold: it is put in the page as it is,
// so nothing that runs, loads or links. beautiful-mermaid draws plain shapes
// and escapes its labels; this holds it to that.
const UNSAFE = /<\s*\/?\s*(script|foreignObject|iframe|object|embed|image|use|a|meta|link|base)\b|\son[a-z]+\s*=|javascript:/i

// beautiful-mermaid's own stylesheet, in every SVG it draws: its colours
// and font, which the page's stylesheet carries for drawn diagrams instead
// (style.ts), and a web font the page's policy would refuse anyway.
const RENDERER_STYLE = /<\s*style\b[^>]*>[\s\S]*?<\s*\/\s*style\s*>/gi

// What an SVG may not hold once its stylesheet is gone: a <style> would
// restyle the whole page.
const STYLE = /<\s*\/?\s*style(?![\w-])/i

// An SVG from the renderer without its stylesheet.
export const cleanSvg = (svg: string): string => svg.replace(RENDERER_STYLE, '')

// An SVG from the renderer as the page will show it, held to what a model's
// SVG is; undefined when it is not one the page will take.
const keptSvg = (svg: unknown): string | undefined => {
  if (typeof svg !== 'string') return undefined
  const clean = cleanSvg(svg)
  const refused =
    !WHOLE_SVG.test(clean) || STYLE.test(clean) || UNSAFE.test(clean) || COMMENT.test(clean) || CLOSES.test(clean)
  return refused ? undefined : clean
}

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
  return sources.map(source => (source === undefined ? undefined : keptSvg(answers[next++])))
}
