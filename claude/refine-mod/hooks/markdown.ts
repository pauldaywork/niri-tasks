// The Markdown twin of a report: the same fields as the HTML slides, top to
// bottom, as a document plannotator-tui can open in herdr, where the comments
// made on it go back to the task's Claude. It holds every field the slides
// use, so it reads back into the report it was written from, and the slides
// are built from what reads back: the two cannot disagree. Pure, like
// report.ts, so the unit tests reach it directly.

import type { Plan } from './plan'
import { BRANCH_HEADINGS as H, CHANGES, LAYERS, LENSES, PLAN_KINDS, PROGRESS, RESULTS, SWAPS } from './report'
import type {
  Branch, Change, CodePair, Commit, Diagram, Facts, FileChange, HotSpot, Layer, Lens, MapEntry, PlanKind,
  Progress, Report, Result, Section, Step, Suite, Swap, Tool,
} from './report'

// The twin's path: the report's, with .md for .html.
export const markdownPath = (htmlPath: string): string => `${htmlPath.replace(/\.html$/, '')}.md`

// A fence longer than any run of backticks in the text, so code that holds
// ``` stays one block.
const fence = (text: string, lang = ''): string => {
  const longest = Math.max(2, ...(text.match(/`+/g) ?? []).map(run => run.length))
  const bar = '`'.repeat(longest + 1)
  return `${bar}${lang}\n${text}\n${bar}`
}

const cell = (text: string): string => text.replace(/\|/g, '\\|')

// The tables' heads, as written and as read back.
const TOOL_HEAD = ['Tool', 'How it connects', 'Swap', 'Why']
const FILE_HEAD = ['Change', 'File', 'Why']
const CHECK_HEAD = ['Check', 'How it is checked']

const headRow = (head: readonly string[]): string => `| ${head.join(' | ')} |`

const table = (head: readonly string[], rows: readonly (readonly string[])[]): string =>
  [
    headRow(head),
    `|${head.map(() => ' --- ').join('|')}|`,
    ...rows.map(row => `| ${row.map(cell).join(' | ')} |`),
  ].join('\n')

const bullets = (lines: readonly string[]): string => lines.map(line => `- ${line}`).join('\n')

// A part's lens, as a comment under its heading, which the rendered Markdown
// hides: `<!-- lens: <kind> -->`, or `<!-- lens -->` for a plain part given
// none. It also tells a part from a fixed slide that shares its heading.
const lensMark = (s: Section): string => (s.kind === undefined ? '<!-- lens -->' : `<!-- lens: ${s.kind} -->`)
const LENS_MARK = /^<!-- lens(?:: (\S+))? -->$/

// More detail, collapsed, as the slide shows it.
const MORE = '<summary>More detail</summary>'

const drawing = (diagram: Diagram): string => fence(diagram.mermaid, 'mermaid')

const code = (pairs: readonly CodePair[]): string =>
  pairs
    .map(pair =>
      [
        `### ${pair.file}`,
        pair.before === '' ? '_New file._' : `Before:\n\n${fence(pair.before)}`,
        pair.after === '' ? '_Removed._' : `After:\n\n${fence(pair.after)}`,
      ].join('\n\n'),
    )
    .join('\n\n')

const tools = (list: readonly Tool[]): string => table(TOOL_HEAD, list.map(t => [t.tool, t.how, t.swap, t.why]))

const part = (s: Section): string => {
  const out = [`## ${s.heading}`, lensMark(s)]
  if (s.look_at !== undefined) out.push(`_${s.look_at}_`)
  if (s.diagram !== undefined) out.push(drawing(s.diagram))
  if (s.before !== undefined && s.after !== undefined) {
    out.push(`Before:\n\n${drawing(s.before)}`, `After:\n\n${drawing(s.after)}`)
  }
  out.push(bullets(s.points))
  if (s.code !== undefined) out.push(code(s.code))
  if (s.tools !== undefined) out.push(tools(s.tools))
  if (s.detail !== undefined) out.push(`<details>\n${MORE}\n${s.detail}\n</details>`)
  return out.join('\n\n')
}

// A branch report's facts, from git, under its title.
const FACTS =
  /^_Branch `([^`]+)` from `([^`]+)` at `([^`]+)`: (\d+) commits, (\d+) files, \+(\d+) -(\d+), (\d+) uncommitted\._$/
const factsLine = (f: Facts): string =>
  `_Branch \`${f.branch}\` from \`${f.base}\` at \`${f.merge_base}\`: ${f.commits} commits, ${f.files} files, ` +
  `+${f.added} -${f.removed}, ${f.uncommitted} uncommitted._`

const SUITE_HEAD = ['Suite', 'Command', 'Passed', 'Failed']
const NOT_RUN_HEAD = ['Not run', 'Why']
const FAILING_HEAD = ['Failing test', 'File', 'Why']
const TESTS_HEAD = ['Test', 'File', 'Layer', 'What it pins', 'Result']
const UNTESTED_HEAD = ['Code', 'File', 'Why']
const PLAN_HEAD = ['From the plan', 'Kind', 'Progress', 'Shown by']
const NO_TESTS = '_None._'

const ticked = (text: string): string => `\`${text}\``

const hotSpots = (list: readonly HotSpot[]): string =>
  list.map(h => [`### ${h.spot}`, h.why, `Files: ${h.files.map(ticked).join(', ')}`].join('\n\n')).join('\n\n')

const story = (steps: readonly Step<Commit>[]): string =>
  steps
    .map(s =>
      [`### ${s.step}`, bullets(s.commits.map(c => `${ticked(c.sha)} ${c.subject}`)), bullets(s.points)].join('\n\n'),
    )
    .join('\n\n')

const changesMap = (entries: readonly MapEntry[]): string =>
  entries
    .map(m => [`### ${m.change} ${ticked(m.path)}`, bullets(m.items.map(i => `**${i.name}**: ${i.does}`))].join('\n\n'))
    .join('\n\n')

const suite = (s: Suite): string =>
  [
    table(SUITE_HEAD, s.runs.map(r => [r.suite, r.command, String(r.passed), String(r.failed)])),
    s.not_run.length > 0 ? table(NOT_RUN_HEAD, s.not_run.map(n => [n.suite, n.why])) : '',
    s.failing.length > 0 ? table(FAILING_HEAD, s.failing.map(f => [f.test, ticked(f.file), f.why])) : '',
  ]
    .filter(Boolean)
    .join('\n\n')

const newTests = (b: Branch): string =>
  [
    b.new_tests.length === 0
      ? NO_TESTS
      : table(TESTS_HEAD, b.new_tests.map(t => [t.test, ticked(t.file), t.layer, t.pins, t.result])),
    b.untested.length > 0
      ? `### ${H.untested}\n\n${table(UNTESTED_HEAD, b.untested.map(u => [u.code, ticked(u.file), u.why]))}`
      : '',
  ]
    .filter(Boolean)
    .join('\n\n')

// The report as Markdown, ending with the plan the person was shown, as the
// HTML's last slide does. A branch report's own blocks sit where its slides
// do; its parts stay in the model's order, as a plan report's do.
export const reportMarkdown = (report: Report, plan: Plan): string => {
  const b = report.branch
  const out = [`# ${report.title}`]
  if (b !== undefined) out.push(factsLine(b.facts))
  out.push(report.summary, `## What changes\n\n${bullets(report.changes)}`)
  if (report.unchanged.length > 0) out.push(`## What stays the same\n\n${bullets(report.unchanged)}`)
  if (b !== undefined) out.push(`## ${H.hotSpots}\n\n${hotSpots(b.hot_spots)}`)
  if (report.needs_your_eye.length > 0) {
    out.push(`## Where your judgement is needed\n\n${bullets(report.needs_your_eye)}`)
  }
  if (report.terms.length > 0) {
    out.push(`## Words used here\n\n${bullets(report.terms.map(t => `**${t.term}**: ${t.meaning}`))}`)
  }
  if (b !== undefined) out.push(`## ${H.story}\n\n${story(b.story)}`)
  out.push(...report.sections.map(part))
  if (b !== undefined) {
    out.push(`## ${H.map}\n\n${changesMap(b.map)}`, `## ${H.suite}\n\n${suite(b.suite)}`, `## ${H.newTests}\n\n${newTests(b)}`)
    if (b.plan_vs_done.length > 0) {
      out.push(
        `## ${H.planVsDone}\n\n${table(PLAN_HEAD, b.plan_vs_done.map(p => [p.item, p.kind, p.progress, p.shown_by]))}`,
      )
    }
  }
  if (report.files.length > 0) {
    out.push(`## Files\n\n${table(FILE_HEAD, report.files.map(f => [f.change, `\`${f.path}\``, f.why]))}`)
  }
  if (b === undefined) out.push(`## Done when\n\n${table(CHECK_HEAD, report.checks.map(c => [c.check, c.how]))}`)
  const notes = plan.notes.length === 0 ? 'No notes.' : plan.notes.map((n, i) => `${i + 1}. ${n}`).join('\n')
  out.push(`## The plan\n\n**Description:** ${plan.description}\n\n${notes}`)
  return `${out.join('\n\n')}\n`
}

// What reads back from a twin: the report and the plan it was written from.
export type ReadBack = { report: Report; plan: Plan }

// The twin read back into the fields it was written from, the inverse of
// reportMarkdown: block by block, in the order it writes them, never as
// Markdown in general, so a line that only looks like a heading or a list
// reads back as written. Throws, naming the line, at anything reportMarkdown
// would not have written.
export const reportFromMarkdown = (markdown: string): ReadBack => {
  const lines = markdown.split('\n')
  let at = 0
  const fail = (wanted: string): never => {
    throw new Error(`line ${at + 1}: expected ${wanted}`)
  }
  const take = (): string => {
    const line = lines[at]
    if (line === undefined) return fail('more lines')
    at += 1
    return line
  }
  const line = (exact: string): void => {
    if (lines[at] !== exact) fail(JSON.stringify(exact))
    at += 1
  }
  const match = (pattern: RegExp, wanted: string): RegExpExecArray => {
    const found = pattern.exec(lines[at] ?? '')
    if (found === null) return fail(wanted)
    at += 1
    return found
  }
  const gap = (): void => line('')
  // The first line of the next block, past the blank line that ends this one.
  const next = (): string | undefined => (lines[at] === '' ? lines[at + 1] : undefined)
  // A part starts at line i: its heading, then its lens mark.
  const partAt = (i: number): boolean =>
    lines[i]?.startsWith('## ') === true && lines[i + 1] === '' && LENS_MARK.test(lines[i + 2] ?? '')
  // The fixed slide headed `heading` comes next, not a part of that name.
  const slide = (heading: string): boolean => next() === `## ${heading}` && !partAt(at + 1)
  const oneOf = <T extends string>(value: string, allowed: readonly string[], wanted: string): T =>
    allowed.includes(value) ? (value as T) : fail(wanted)

  const list = (): string[] => {
    const items: string[] = []
    while (lines[at]?.startsWith('- ')) items.push(take().slice(2))
    return items.length > 0 ? items : fail('a "- " line')
  }
  const block = (lang: '' | 'mermaid'): string => {
    const open = lang === 'mermaid' ? /^(`{3,})mermaid$/ : /^(`{3,})$/
    const bar = match(open, `a ${lang === '' ? 'code' : lang} fence`)[1] ?? ''
    const body: string[] = []
    while (lines[at] !== bar) body.push(take())
    at += 1
    return body.join('\n')
  }
  // A table's rows, each cell as written before `cell` escaped its pipes.
  const rows = (head: readonly string[]): string[][] => {
    line(headRow(head))
    line(`|${head.map(() => ' --- ').join('|')}|`)
    const out: string[][] = []
    while (lines[at]?.startsWith('| ')) {
      const row = lines[at] ?? ''
      const cells = row.endsWith(' |') ? row.slice(2, -2).split(' | ').map(c => c.replace(/\\\|/g, '|')) : []
      if (cells.length !== head.length) fail(`a row of ${head.length} cells`)
      out.push(cells)
      at += 1
    }
    return out.length > 0 ? out : fail('a table row')
  }
  const diagram = (): Diagram => ({ mermaid: block('mermaid') })
  const codePair = (): CodePair => {
    const file = match(/^### (.+)$/, 'a "### <file>" heading')[1] ?? ''
    gap()
    let before = ''
    if (lines[at] === '_New file._') at += 1
    else {
      line('Before:')
      gap()
      before = block('')
    }
    gap()
    let after = ''
    if (lines[at] === '_Removed._') at += 1
    else {
      line('After:')
      gap()
      after = block('')
    }
    return { file, before, after }
  }
  const part = (): Section => {
    const heading = match(/^## (.+)$/, 'a part heading')[1] ?? ''
    gap()
    const kind = match(LENS_MARK, 'a <!-- lens --> mark')[1]
    const s: Section = {
      ...(kind === undefined ? {} : { kind: oneOf<Lens>(kind, LENSES, 'a lens') }),
      heading,
      points: [],
    }
    if (/^_.+_$/.test(next() ?? '')) {
      gap()
      s.look_at = take().slice(1, -1)
    }
    if (/^`{3,}mermaid$/.test(next() ?? '')) {
      gap()
      s.diagram = diagram()
    }
    if (next() === 'Before:') {
      gap()
      line('Before:')
      gap()
      s.before = diagram()
      gap()
      line('After:')
      gap()
      s.after = diagram()
    }
    gap()
    s.points = list()
    while (next()?.startsWith('### ')) {
      gap()
      s.code = [...(s.code ?? []), codePair()]
    }
    if (next() === headRow(TOOL_HEAD)) {
      gap()
      s.tools = rows(TOOL_HEAD).map(([tool = '', how = '', swap = '', why = '']) => ({
        tool,
        how,
        swap: oneOf<Swap>(swap, SWAPS, 'easy, medium or hard'),
        why,
      }))
    }
    if (next() === '<details>') {
      gap()
      line('<details>')
      line(MORE)
      const body: string[] = []
      while (lines[at] !== '</details>') body.push(take())
      at += 1
      s.detail = body.join('\n')
    }
    return s
  }
  // A fixed slide's list, or none when the slide was left out.
  const optional = (heading: string): string[] => {
    if (!slide(heading)) return []
    gap()
    line(`## ${heading}`)
    gap()
    return list()
  }
  // A fixed slide that must come next: its heading, then its body.
  const heading = (text: string): void => {
    gap()
    line(`## ${text}`)
    gap()
  }
  // `### ` blocks, one after another, each read by `read`.
  const blocks = <T>(read: () => T): T[] => {
    const out = [read()]
    while (next()?.startsWith('### ')) {
      gap()
      out.push(read())
    }
    return out
  }
  const quoted = (cell: string): string => (/^`.*`$/.test(cell) ? cell.slice(1, -1) : fail('a `path`'))
  const count = (cell: string): number => (/^\d+$/.test(cell) ? Number(cell) : fail('a whole number'))
  const hotSpot = (): HotSpot => {
    const spot = match(/^### (.+)$/, 'a "### <spot>" heading')[1] ?? ''
    gap()
    const why = take()
    gap()
    const files = (match(/^Files: (.+)$/, 'a "Files: " line')[1] ?? '').split(', ').map(quoted)
    return { spot, why, files }
  }
  const step = (): Step<Commit> => {
    const name = match(/^### (.+)$/, 'a "### <step>" heading')[1] ?? ''
    gap()
    const commits = list().map(c => {
      const found = /^`([^`]+)` (.*)$/.exec(c) ?? fail('a "`<sha>` <subject>" line')
      return { sha: found[1] ?? '', subject: found[2] ?? '' }
    })
    gap()
    return { step: name, commits, points: list() }
  }
  const mapEntry = (): MapEntry => {
    const found = match(/^### (\S+) `(.+)`$/, 'a "### <change> `<path>`" heading')
    gap()
    const items = list().map(i => {
      const m = /^\*\*(.+?)\*\*: (.+)$/.exec(i) ?? fail('a "**<name>**: <does>" line')
      return { name: m[1] ?? '', does: m[2] ?? '' }
    })
    return { path: found[2] ?? '', change: oneOf<Change>(found[1] ?? '', CHANGES, 'new, change or remove'), items }
  }
  // A table that may follow, by its head, or none.
  const maybeRows = (head: readonly string[]): string[][] => {
    if (next() !== headRow(head)) return []
    gap()
    return rows(head)
  }

  const title = match(/^# (.+)$/, 'the "# <title>" line')[1] ?? ''
  gap()
  const f = FACTS.exec(lines[at] ?? '')
  let facts: Facts | undefined
  if (f !== null) {
    at += 1
    facts = {
      branch: f[1] ?? '',
      base: f[2] ?? '',
      merge_base: f[3] ?? '',
      commits: Number(f[4]),
      files: Number(f[5]),
      added: Number(f[6]),
      removed: Number(f[7]),
      uncommitted: Number(f[8]),
    }
    gap()
  }
  const summary = take()
  gap()
  line('## What changes')
  gap()
  const changes = list()
  const unchanged = optional('What stays the same')
  let hot_spots: HotSpot[] = []
  if (facts !== undefined) {
    heading(H.hotSpots)
    hot_spots = blocks(hotSpot)
  }
  const needs_your_eye = optional('Where your judgement is needed')
  const terms = optional('Words used here').map(entry => {
    const t = /^\*\*(.+?)\*\*: (.+)$/.exec(entry) ?? fail('a "**<term>**: <meaning>" line')
    return { term: t[1] ?? '', meaning: t[2] ?? '' }
  })
  let steps: Step<Commit>[] = []
  if (facts !== undefined) {
    heading(H.story)
    steps = blocks(step)
  }
  const sections: Section[] = []
  while (lines[at] === '' && partAt(at + 1)) {
    gap()
    sections.push(part())
  }
  let branch: Branch | undefined
  if (facts !== undefined) {
    heading(H.map)
    const map = blocks(mapEntry)
    heading(H.suite)
    const runs = rows(SUITE_HEAD).map(([suite = '', command = '', passed = '', failed = '']) => ({
      suite,
      command,
      passed: count(passed),
      failed: count(failed),
    }))
    const not_run = maybeRows(NOT_RUN_HEAD).map(([suite = '', why = '']) => ({ suite, why }))
    const failing = maybeRows(FAILING_HEAD).map(([test = '', file = '', why = '']) => ({
      test,
      file: quoted(file),
      why,
    }))
    heading(H.newTests)
    let new_tests: Branch['new_tests'] = []
    if (lines[at] === NO_TESTS) at += 1
    else {
      new_tests = rows(TESTS_HEAD).map(([test = '', file = '', layer = '', pins = '', result = '']) => ({
        test,
        file: quoted(file),
        layer: oneOf<Layer>(layer, LAYERS, 'unit, integration or e2e'),
        pins,
        result: oneOf<Result>(result, RESULTS, 'pass, fail or not run'),
      }))
    }
    let untested: Branch['untested'] = []
    if (next() === `### ${H.untested}`) {
      gap()
      line(`### ${H.untested}`)
      gap()
      untested = rows(UNTESTED_HEAD).map(([c = '', file = '', why = '']) => ({ code: c, file: quoted(file), why }))
    }
    let plan_vs_done: Branch['plan_vs_done'] = []
    if (slide(H.planVsDone)) {
      heading(H.planVsDone)
      plan_vs_done = rows(PLAN_HEAD).map(([item = '', kind = '', progress = '', shown_by = '']) => ({
        item,
        kind: oneOf<PlanKind>(kind, PLAN_KINDS, 'step or done when'),
        progress: oneOf<Progress>(progress, PROGRESS, 'done, partial or not yet'),
        shown_by,
      }))
    }
    branch = { facts, hot_spots, story: steps, map, suite: { runs, not_run, failing }, new_tests, untested, plan_vs_done }
  }
  let files: FileChange[] = []
  if (slide('Files')) {
    gap()
    line('## Files')
    gap()
    files = rows(FILE_HEAD).map(([change = '', path = '', why = '']) => ({
      change: oneOf<Change>(change, CHANGES, 'new, change or remove'),
      path: quoted(path),
      why,
    }))
  }
  let checks: { check: string; how: string }[] = []
  if (branch === undefined) {
    gap()
    line('## Done when')
    gap()
    checks = rows(CHECK_HEAD).map(([check = '', how = '']) => ({ check, how }))
  }
  gap()
  line('## The plan')
  gap()
  const description = match(/^\*\*Description:\*\* (.*)$/, 'the "**Description:** " line')[1] ?? ''
  gap()
  const notes: string[] = []
  if (lines[at] === 'No notes.') at += 1
  else {
    for (let n = 1; lines[at]?.startsWith(`${n}. `); n += 1) notes.push(take().slice(`${n}. `.length))
    if (notes.length === 0) fail('the notes')
  }
  gap()
  if (at !== lines.length) fail('the end')
  return {
    report: {
      title, summary, changes, unchanged, needs_your_eye, terms, sections, files, checks,
      ...(branch === undefined ? {} : { branch }),
    },
    plan: { description, notes },
  }
}

// Plain data equal field by field, whatever the key order; a key on one side
// only is a difference.
const same = (a: unknown, b: unknown): boolean => {
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return a === b
  if (Array.isArray(a) !== Array.isArray(b)) return false
  const ka = Object.keys(a)
  const kb = Object.keys(b)
  return (
    ka.length === kb.length &&
    ka.every(k => Object.hasOwn(b, k) && same((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]))
  )
}

// A report's twin, and what reads back from it, which the slides are built
// from.
export type Twin = ReadBack & { markdown: string }

// The twin and what reads back from it; or why the fields do not come back as
// given, as for a plan note on two lines or a term holding "**: ", and then the
// slides would not match the report and nothing is written.
export const twinOf = (report: Report, plan: Plan): Twin | string => {
  const markdown = reportMarkdown(report, plan)
  let read: ReadBack
  try {
    read = reportFromMarkdown(markdown)
  } catch (error) {
    return `its Markdown twin does not read back (${error instanceof Error ? error.message : String(error)})`
  }
  return same(read, { report, plan })
    ? { markdown, ...read }
    : 'its Markdown twin reads back as a different report'
}
