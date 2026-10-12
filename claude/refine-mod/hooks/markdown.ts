// The Markdown twin of a report: the same fields as the HTML slides, top to
// bottom, as a document plannotator-tui can open in herdr, where the comments
// made on it go back to the task's Claude. It holds every field the slides
// use, so it reads back into the report it was written from, and the slides
// are built from what reads back: the two cannot disagree. Pure, like
// report.ts, so the unit tests reach it directly.

import type { Plan } from './plan'
import { CHANGES, LENSES, SWAPS } from './report'
import type { Change, CodePair, Diagram, Lens, Report, Section, Swap, Tool } from './report'

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

// The report as Markdown, in the slides' order, ending with the plan the
// person was shown, as the HTML's last slide does.
export const reportMarkdown = (report: Report, plan: Plan): string => {
  const out = [`# ${report.title}`, report.summary, `## What changes\n\n${bullets(report.changes)}`]
  if (report.unchanged.length > 0) out.push(`## What stays the same\n\n${bullets(report.unchanged)}`)
  if (report.needs_your_eye.length > 0) {
    out.push(`## Where your judgement is needed\n\n${bullets(report.needs_your_eye)}`)
  }
  if (report.terms.length > 0) {
    out.push(`## Words used here\n\n${bullets(report.terms.map(t => `**${t.term}**: ${t.meaning}`))}`)
  }
  out.push(...report.sections.map(part))
  if (report.files.length > 0) {
    out.push(`## Files\n\n${table(FILE_HEAD, report.files.map(f => [f.change, `\`${f.path}\``, f.why]))}`)
  }
  out.push(`## Done when\n\n${table(CHECK_HEAD, report.checks.map(c => [c.check, c.how]))}`)
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

  const title = match(/^# (.+)$/, 'the "# <title>" line')[1] ?? ''
  gap()
  const summary = take()
  gap()
  line('## What changes')
  gap()
  const changes = list()
  const unchanged = optional('What stays the same')
  const needs_your_eye = optional('Where your judgement is needed')
  const terms = optional('Words used here').map(entry => {
    const t = /^\*\*(.+?)\*\*: (.+)$/.exec(entry) ?? fail('a "**<term>**: <meaning>" line')
    return { term: t[1] ?? '', meaning: t[2] ?? '' }
  })
  const sections: Section[] = []
  while (lines[at] === '' && partAt(at + 1)) {
    gap()
    sections.push(part())
  }
  const files = !slide('Files')
    ? []
    : (gap(),
      line('## Files'),
      gap(),
      rows(FILE_HEAD).map(([change = '', path = '', why = '']) => ({
        change: oneOf<Change>(change, CHANGES, 'new, change or remove'),
        path: /^`.*`$/.test(path) ? path.slice(1, -1) : fail('a `path`'),
        why,
      })))
  gap()
  line('## Done when')
  gap()
  const checks = rows(CHECK_HEAD).map(([check = '', how = '']) => ({ check, how }))
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
    report: { title, summary, changes, unchanged, needs_your_eye, terms, sections, files, checks },
    plan: { description, notes },
  }
}

// A report's twin, and what reads back from it, which the slides are built
// from.
export type Twin = ReadBack & { markdown: string }

// The twin and what reads back from it; or why it does not read back as the
// report it was written from, as for a plan note on two lines, and then the
// slides would not match it and nothing is written.
export const twinOf = (report: Report, plan: Plan): Twin | string => {
  const markdown = reportMarkdown(report, plan)
  let read: ReadBack
  try {
    read = reportFromMarkdown(markdown)
  } catch (error) {
    return `its Markdown twin does not read back (${error instanceof Error ? error.message : String(error)})`
  }
  return reportMarkdown(read.report, read.plan) === markdown
    ? { markdown, ...read }
    : 'its Markdown twin reads back as a different report'
}
