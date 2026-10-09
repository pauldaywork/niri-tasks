import { describe, expect, test } from 'claude-code/testing'
import { RENDERER, cleanSvg, drawArgv, drawSources, drawnSvgs } from '../hooks/draw'
import type { Report } from '../hooks/report'

const report = (sections: Report['sections']): Report => ({
  title: 't',
  summary: 's',
  changes: ['c'],
  unchanged: [],
  needs_your_eye: [],
  terms: [],
  sections,
  files: [],
  checks: [{ check: 'c', how: 'h' }],
})

test('the renderer runs fenced: read-only, no network, nothing else shared', () => {
  expect(RENDERER).toBe('render/diagrams.mjs')
  expect(drawArgv('bun', '/m/render/diagrams.mjs')).toEqual([
    'bwrap', '--ro-bind', '/', '/', '--dev', '/dev', '--proc', '/proc', '--tmpfs', '/tmp',
    '--unshare-all', '--die-with-parent', '--new-session', 'bun', '/m/render/diagrams.mjs',
  ])
})

describe('drawSources', () => {
  test('gives each Mermaid part its source, flowcharts with the change colours', () => {
    const sources = drawSources(
      report([
        { heading: 'a', points: ['p'], look_at: 'x', diagram: { mermaid: 'flowchart LR\n a --> b' } },
        { heading: 'b', points: ['p'] },
        { heading: 'c', points: ['p'], look_at: 'x', diagram: { svg: '<svg></svg>' } },
        { heading: 'd', points: ['p'], look_at: 'x', diagram: { mermaid: 'sequenceDiagram\n A->>B: hi' } },
      ]),
    )
    expect(sources[0]).toStartWith('flowchart LR\n a --> b\nclassDef new fill:#99E885')
    expect(sources[1]).toBeUndefined()
    expect(sources[2]).toBeUndefined()
    expect(sources[3]).toBe('sequenceDiagram\n A->>B: hi')
  })
})

describe('drawnSvgs', () => {
  const sources = ['flowchart LR\n a --> b', undefined, 'sequenceDiagram\n A->>B: hi']

  test('maps the answers back to the parts they were drawn for', () => {
    const out = JSON.stringify({ svgs: ['<svg id="one"></svg>', '<svg id="two"></svg>'] })
    expect(drawnSvgs(out, sources)).toEqual(['<svg id="one"></svg>', undefined, '<svg id="two"></svg>'])
  })

  test('leaves a part undrawn when the renderer could not draw it', () => {
    const out = JSON.stringify({ svgs: [null, '<svg id="two"></svg>'] })
    expect(drawnSvgs(out, sources)).toEqual([undefined, undefined, '<svg id="two"></svg>'])
  })

  test('drops anything that could run or load, or is not an SVG', () => {
    for (const bad of [
      '<svg><script>x</script></svg>',
      '<svg><foreignObject></foreignObject></svg>',
      '<svg onload="x"></svg>',
      '<svg><a href="javascript:x"></a></svg>',
      '<svg><image href="x"/></svg>',
      '<svg><use href="#x"/></svg>',
      '<div></div>',
    ]) {
      expect(drawnSvgs(JSON.stringify({ svgs: [bad, null] }), sources)[0]).toBeUndefined()
    }
  })

  test('draws nothing from output that is not its JSON', () => {
    expect(drawnSvgs('', sources)).toEqual([undefined, undefined, undefined])
    expect(drawnSvgs('{"svgs": 7}', sources)).toEqual([undefined, undefined, undefined])
  })
})

test('cleanSvg drops the web font and uses the report body font', () => {
  const svg =
    "<svg><style>\n  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500&display=swap');\n" +
    "  text { font-family: 'Inter', system-ui, sans-serif; }\n</style></svg>"
  const clean = cleanSvg(svg)
  expect(clean).not.toContain('@import')
  expect(clean).not.toContain('googleapis')
  expect(clean).toContain("font-family: 'Report Body', 'Inter', system-ui, sans-serif;")
})
