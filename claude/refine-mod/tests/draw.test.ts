import { describe, expect, test } from 'claude-code/testing'
import { RENDERER, cleanSvg, drawArgv, drawSources, drawnSvgs, runtimePath, whichArgv } from '../hooks/draw'
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

test('the renderer runs fenced: only /usr, the runtime and the script, no network, no host sockets', () => {
  expect(RENDERER).toBe('render/diagrams.mjs')
  expect(drawArgv('/opt/bun', '/m/render/diagrams.mjs')).toEqual([
    'bwrap', '--ro-bind', '/usr', '/usr',
    '--symlink', 'usr/lib', '/lib', '--symlink', 'usr/lib64', '/lib64',
    '--symlink', 'usr/bin', '/bin', '--symlink', 'usr/sbin', '/sbin',
    '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp',
    '--ro-bind', '/opt/bun', '/run/r/runtime', '--ro-bind', '/m/render/diagrams.mjs', '/run/r/diagrams.mjs',
    '--unshare-all', '--die-with-parent', '--new-session', '--clearenv', '--setenv', 'PATH', '/usr/bin',
    '/run/r/runtime', '/run/r/diagrams.mjs',
  ])
})

test('the runtime is found by name and resolved to its real path, passed as an argument', () => {
  expect(whichArgv('bun')).toEqual(['sh', '-c', 'readlink -f "$(command -v "$1")"', 'sh', 'bun'])
  expect(runtimePath({ exitCode: 0, stdout: '/home/p/.bun/bin/bun\n' })).toBe('/home/p/.bun/bin/bun')
  expect(runtimePath({ exitCode: 1, stdout: '' })).toBeUndefined()
  expect(runtimePath({ exitCode: 0, stdout: '\n' })).toBeUndefined()
  expect(runtimePath({ exitCode: 0, stdout: 'bun\n' })).toBeUndefined()
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
    // Three slots a part: its diagram, its before, its after.
    expect(sources).toHaveLength(12)
    expect(sources[0]).toStartWith('flowchart LR\n a --> b\nclassDef new fill:#99E885')
    expect(sources.slice(1, 9).every(s => s === undefined)).toBe(true)
    expect(sources[9]).toBe('sequenceDiagram\n A->>B: hi')
  })

  // beautiful-mermaid draws a state diagram without the change colours, so
  // one that marks a change is left to Mermaid in the browser.
  test('leaves a state diagram with change marks to the browser', () => {
    const state = (mermaid: string) => drawSources(report([{ heading: 'a', points: ['p'], look_at: 'x', diagram: { mermaid } }]))[0]
    expect(state('stateDiagram-v2\n [*] --> A\n A --> B\n class B new')).toBeUndefined()
    expect(state('stateDiagram-v2\n [*] --> A\n class A,B change;')).toBeUndefined()
    expect(state('stateDiagram\n [*] --> A:::remove')).toBeUndefined()
    expect(state('%% the states\nstateDiagram-v2\n A --> B:::new')).toBeUndefined()
    expect(state('stateDiagram-v2\n [*] --> A\n A --> B')).toStartWith('stateDiagram-v2\n [*] --> A\n A --> B\nclassDef new')
  })

  test('still draws a flowchart with change marks', () => {
    const flow = 'flowchart LR\n a --> b:::new\n class a remove'
    const [source] = drawSources(report([{ heading: 'a', points: ['p'], look_at: 'x', diagram: { mermaid: flow } }]))
    expect(source).toStartWith(`${flow}\nclassDef new`)
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
      '<svg><meta http-equiv="refresh" content="0"/></svg>',
      '<svg><link rel="stylesheet" href="x"/></svg>',
      '<svg><base href="https://x/"/></svg>',
      '<div></div>',
    ]) {
      expect(drawnSvgs(JSON.stringify({ svgs: [bad, null] }), sources)[0]).toBeUndefined()
    }
  })

  test("keeps a renderer SVG with its <style>, the style removed", () => {
    const svg = '<svg viewBox="0 0 1 1">\n<style>\n  svg { --_line: var(--fg); }\n</style>\n<rect/>\n</svg>'
    expect(drawnSvgs(JSON.stringify({ svgs: [svg, null] }), sources)[0]).toBe('<svg viewBox="0 0 1 1">\n\n<rect/>\n</svg>')
  })

  test('drops what would break the page, as for an SVG the model writes', () => {
    for (const bad of [
      '<svg><!-- open',
      '<svg><text>a</text></section><section></svg>',
      '<svg><g></figure></g></svg>',
      '<svg><rect/>',
      '<svg><rect/></svg><p>after</p>',
      '<svg><style>a{}</style><style>b{}</svg>',
      '<svg><style>a{}</style>b{}</style></svg>',
    ]) {
      expect(drawnSvgs(JSON.stringify({ svgs: [bad, null] }), sources)[0]).toBeUndefined()
    }
  })

  test('draws nothing from output that is not its JSON', () => {
    expect(drawnSvgs('', sources)).toEqual([undefined, undefined, undefined])
    expect(drawnSvgs('{"svgs": 7}', sources)).toEqual([undefined, undefined, undefined])
  })
})

test("cleanSvg removes the renderer's stylesheet, whose rules the page carries", () => {
  const svg =
    "<svg><style>\n  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500&display=swap');\n" +
    "  text { font-family: 'Inter', system-ui, sans-serif; }\n</style><rect/><STYLE media=\"x\">svg{}</STYLE ></svg>"
  expect(cleanSvg(svg)).toBe('<svg><rect/></svg>')
})

test('drawSources gives a before/after pair its own two slots', () => {
  const sources = drawSources(
    report([
      {
        kind: 'before-after',
        heading: 'h',
        points: ['p'],
        look_at: 'x',
        before: { mermaid: 'flowchart LR\n a --> b' },
        after: { mermaid: 'flowchart LR\n a --> c:::new' },
      },
    ]),
  )
  expect(sources[0]).toBeUndefined()
  expect(sources[1]).toStartWith('flowchart LR\n a --> b\n')
  expect(sources[2]).toStartWith('flowchart LR\n a --> c:::new\n')
})
