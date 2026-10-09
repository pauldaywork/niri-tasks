// The refine report's diagram renderer, run by the mod in a bwrap fence:
// reads {"diagrams": [Mermaid source, ...]} on stdin and writes
// {"svgs": [svg or null, ...], "errors": [message or null, ...]} on stdout,
// one of each per diagram, in order. Black on white, every text colour
// black, so labels stay readable; the page styles the rest.
import { renderMermaidSVG } from 'beautiful-mermaid'

const THEME = {
  bg: '#FFFFFF',
  fg: '#000000',
  line: '#000000',
  accent: '#000000',
  muted: '#000000',
  border: '#000000',
  transparent: true,
}

let input = ''
for await (const chunk of process.stdin) input += chunk
const { diagrams } = JSON.parse(input) as { diagrams: string[] }
const svgs: (string | null)[] = []
const errors: (string | null)[] = []
for (const diagram of diagrams) {
  try {
    svgs.push(renderMermaidSVG(diagram, THEME))
    errors.push(null)
  } catch (error) {
    svgs.push(null)
    errors.push(error instanceof Error ? error.message : String(error))
  }
}
process.stdout.write(JSON.stringify({ svgs, errors }))
