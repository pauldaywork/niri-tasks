// The pure half of show_task_report: the fields it takes from the model, the
// limits that keep a report short enough to take in, and where it is saved.
// No `$`, so the unit tests reach it directly.

import { HIDDEN, taskDate } from './plan'
import type { Plan, Task } from './plan'

export type Change = 'new' | 'change' | 'remove'
export type Diagram = { mermaid: string } | { svg: string }
// What a part looks at the change through. A report looks through every lens
// that applies; each has its own slide name and place in the order.
export const LENSES = [
  'before-after',
  'part',
  'structure',
  'data-flow',
  'outside-tools',
  'styling',
  'code',
  'newcomer',
] as const
export type Lens = (typeof LENSES)[number]
export type Swap = 'easy' | 'medium' | 'hard'

// The diagram slots of each part, in order: its diagram, its before, its
// after. A part's drawn diagram is found at part * SLOTS + slot.
export const SLOTS = 3
// A piece of code as it is and as it will be: '' before for a new file, ''
// after for a removed one.
export type CodePair = { file: string; before: string; after: string }
// An outside tool the change touches: how it is connected, and how hard it
// would be to swap for another.
export type Tool = { tool: string; how: string; swap: Swap; why: string }
export type Section = {
  // Absent for a plain part.
  kind?: Lens
  heading: string
  look_at?: string
  diagram?: Diagram
  before?: Diagram
  after?: Diagram
  points: string[]
  detail?: string
  code?: CodePair[]
  tools?: Tool[]
}
export type Term = { term: string; meaning: string }
export type FileChange = { path: string; change: Change; why: string }
export type Check = { check: string; how: string }

export type Report = {
  title: string
  summary: string
  changes: string[]
  unchanged: string[]
  needs_your_eye: string[]
  terms: Term[]
  sections: Section[]
  files: FileChange[]
  checks: Check[]
}

// What keeps a report easy to take in: about four things held in mind at
// once, one idea per line, short plain sentences. A report over them is
// refused with every reason at once, so it is fixed in one go.
export const LIMITS = {
  titleChars: 120,
  summaryWords: 35,
  changes: { min: 1, max: 3, words: 20 },
  unchanged: { min: 0, max: 3, words: 20 },
  needsYourEye: { min: 0, max: 3, words: 30 },
  terms: { min: 0, max: 5, termWords: 4, meaningWords: 20 },
  sections: { min: 1, max: 9, headingWords: 10, lookAtWords: 25 },
  code: { min: 1, max: 3, lines: 16, chars: 1200, fileChars: 200 },
  tools: { min: 1, max: 8, toolWords: 4, words: 20 },
  points: { min: 1, max: 5, words: 25 },
  mermaidLines: 40,
  diagramChars: 50_000,
  detailChars: 20_000,
  files: { min: 0, max: 40, pathChars: 200, whyWords: 20 },
  checks: { min: 1, max: 6, words: 20 },
} as const

const CHANGES: readonly string[] = ['new', 'change', 'remove']

// Elements refused by name: a <meta> refresh navigates and a <meta> policy
// could be added to, which the page's own policy does not stop; the rest would
// reach outside or run script, and fail quietly under it. Refused so the model
// hears why rather than finding a blank figure.
const FORBIDDEN = /<\s*(script|meta|base|link|iframe|frameset|frame|object|embed|form|portal)(?![\w-])/i

// What would break the page's one layout, refused in detail and SVG, which
// the page shows as given (Mermaid source is escaped, so it cannot): a
// comment left open hides the rest of the page, and closing one of the
// page's own elements moves what follows out of its place. Drawn diagrams
// are held to them too.
export const COMMENT = /<!--/
export const CLOSES = /<\s*\/\s*(details|section|main|footer|nav|figure|header|body|html)(?![\w-])/i

// An SVG shown as given is one whole <svg> element, nothing before or after.
export const WHOLE_SVG = /^\s*<svg[\s>][\s\S]*<\/svg>\s*$/i

// Elements that swallow the rest of the page as text, or restyle it. An SVG
// may keep its <title>, which names the drawing.
const SWALLOWS_DETAIL = /<\s*(style|plaintext|xmp|textarea|title|noscript|template|html|head|body)(?![\w-])/i
const SWALLOWS_SVG = /<\s*(style)(?![\w-])/i

// How many problems one refusal lists before it says how many more.
const MAX_ERRORS = 20

type Errors = string[]
type Count = { readonly min: number; readonly max: number }

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value)

const wordCount = (text: string): number => text.split(/\s+/).filter(Boolean).length

// One line of plain text within its limit. A problem is noted, never thrown,
// so the refusal can list them all.
const text = (errors: Errors, at: string, value: unknown, limit: { words?: number; chars?: number }): string => {
  if (typeof value !== 'string' || value.trim() === '') {
    errors.push(`${at} must be a non-empty string`)
    return ''
  }
  if (/[\r\n]/.test(value)) errors.push(`${at} must be one line`)
  else if (HIDDEN.test(value)) errors.push(`${at} must be plain text: no control or invisible characters`)
  const n = wordCount(value)
  if (limit.words !== undefined && n > limit.words) errors.push(`${at} has ${n} words; at most ${limit.words}`)
  if (limit.chars !== undefined && value.length > limit.chars) {
    errors.push(`${at} has ${value.length} characters; at most ${limit.chars}`)
  }
  return value
}

// Markup within its size, holding nothing the page must not. Markup the page
// shows as given passes the elements that would swallow it, and is held to
// the page's one layout too.
const markup = (errors: Errors, at: string, value: unknown, maxChars: number, swallows?: RegExp): string => {
  if (typeof value !== 'string' || value.trim() === '') {
    errors.push(`${at} must be a non-empty string`)
    return ''
  }
  if (value.length > maxChars) errors.push(`${at} has ${value.length} characters; at most ${maxChars}`)
  const tag = FORBIDDEN.exec(value)?.[1]?.toLowerCase()
  if (tag !== undefined) {
    errors.push(`${at} must not contain <${tag}>: the page runs no script but Mermaid, loads nothing and sends nothing`)
  }
  if (swallows === undefined) return value
  if (COMMENT.test(value)) errors.push(`${at} must not contain an HTML comment: the page keeps one layout`)
  const closed = CLOSES.exec(value)?.[1]?.toLowerCase()
  if (closed !== undefined) errors.push(`${at} must not close the page's <${closed}>: the page keeps one layout`)
  const swallowed = swallows.exec(value)?.[1]?.toLowerCase()
  if (swallowed !== undefined) errors.push(`${at} must not contain <${swallowed}>: the page keeps one layout`)
  return value
}

// A list within its count; an optional list may be left out.
const list = <T>(
  errors: Errors,
  at: string,
  value: unknown,
  count: Count,
  item: (value: unknown, at: string) => T,
): T[] => {
  if (value === undefined && count.min === 0) return []
  if (!Array.isArray(value)) {
    errors.push(`${at} must be an array`)
    return []
  }
  if (value.length < count.min || value.length > count.max) {
    errors.push(`${at} must have ${count.min} to ${count.max} items, not ${value.length}`)
  }
  return value.map((v, i) => item(v, `${at}[${i}]`))
}

const term = (errors: Errors, at: string, value: unknown): Term => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { term: '', meaning: '' }
  }
  return {
    term: text(errors, `${at}.term`, value.term, { words: LIMITS.terms.termWords }),
    meaning: text(errors, `${at}.meaning`, value.meaning, { words: LIMITS.terms.meaningWords }),
  }
}

const diagram = (errors: Errors, at: string, value: unknown): Diagram | undefined => {
  if (!isObject(value) || ('mermaid' in value) === ('svg' in value)) {
    errors.push(`${at} must have exactly one of mermaid or svg`)
    return undefined
  }
  if ('mermaid' in value) {
    const source = markup(errors, `${at}.mermaid`, value.mermaid, LIMITS.diagramChars)
    const lines = source.split('\n').length
    if (lines > LIMITS.mermaidLines) {
      errors.push(`${at}.mermaid has ${lines} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`)
    }
    return { mermaid: source }
  }
  const svg = markup(errors, `${at}.svg`, value.svg, LIMITS.diagramChars, SWALLOWS_SVG)
  if (svg !== '' && !WHOLE_SVG.test(svg)) errors.push(`${at}.svg must be one <svg> element`)
  return { svg }
}

const SWAPS: readonly string[] = ['easy', 'medium', 'hard']

// Code as the page shows it, in a pane: any lines, but no control characters
// besides tabs and line breaks, and within its size.
const CONTROL = /[\u0000-\u0008\u000B-\u001F\u007F]/

const snippet = (errors: Errors, at: string, value: unknown): string => {
  if (typeof value !== 'string') {
    errors.push(`${at} must be a string ('' for none)`)
    return ''
  }
  if (CONTROL.test(value)) errors.push(`${at} must hold no control characters but tabs and line breaks`)
  const lines = value === '' ? 0 : value.split('\n').length
  if (lines > LIMITS.code.lines) errors.push(`${at} has ${lines} lines; at most ${LIMITS.code.lines}`)
  if (value.length > LIMITS.code.chars) errors.push(`${at} has ${value.length} characters; at most ${LIMITS.code.chars}`)
  return value
}

const codePair = (errors: Errors, at: string, value: unknown): CodePair => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { file: '', before: '', after: '' }
  }
  const pair = {
    file: text(errors, `${at}.file`, value.file, { chars: LIMITS.code.fileChars }),
    before: snippet(errors, `${at}.before`, value.before),
    after: snippet(errors, `${at}.after`, value.after),
  }
  if (pair.before === '' && pair.after === '') errors.push(`${at} needs a before or an after`)
  return pair
}

const tool = (errors: Errors, at: string, value: unknown): Tool => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { tool: '', how: '', swap: 'medium', why: '' }
  }
  const swap = value.swap
  if (typeof swap !== 'string' || !SWAPS.includes(swap)) errors.push(`${at}.swap must be easy, medium or hard`)
  return {
    tool: text(errors, `${at}.tool`, value.tool, { words: LIMITS.tools.toolWords }),
    how: text(errors, `${at}.how`, value.how, { words: LIMITS.tools.words }),
    swap: swap as Swap,
    why: text(errors, `${at}.why`, value.why, { words: LIMITS.tools.words }),
  }
}

const section = (errors: Errors, at: string, value: unknown): Section => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { heading: '', points: [] }
  }
  const kind = value.kind
  if (kind !== undefined && (typeof kind !== 'string' || !(LENSES as readonly string[]).includes(kind))) {
    errors.push(`${at}.kind must be one of ${LENSES.join(', ')}`)
  }
  const part: Section = {
    ...(kind !== undefined ? { kind: kind as Lens } : {}),
    heading: text(errors, `${at}.heading`, value.heading, { words: LIMITS.sections.headingWords }),
    points: list(errors, `${at}.points`, value.points, LIMITS.points, (v, i) =>
      text(errors, i, v, { words: LIMITS.points.words }),
    ),
  }
  const paired = value.before !== undefined || value.after !== undefined
  if (paired && value.diagram !== undefined) errors.push(`${at} has a diagram and a before/after pair; give one`)
  if (paired && (value.before === undefined || value.after === undefined)) {
    errors.push(`${at} needs both before and after`)
  }
  if (value.diagram !== undefined) {
    const drawn = diagram(errors, `${at}.diagram`, value.diagram)
    if (drawn !== undefined) part.diagram = drawn
  }
  if (value.before !== undefined) {
    const drawn = diagram(errors, `${at}.before`, value.before)
    if (drawn !== undefined) part.before = drawn
  }
  if (value.after !== undefined) {
    const drawn = diagram(errors, `${at}.after`, value.after)
    if (drawn !== undefined) part.after = drawn
  }
  if (value.diagram !== undefined || paired) {
    part.look_at = text(errors, `${at}.look_at`, value.look_at, { words: LIMITS.sections.lookAtWords })
  } else if (value.look_at !== undefined) {
    errors.push(`${at}.look_at needs a diagram to point at`)
  }
  if (value.code !== undefined) {
    part.code = list(errors, `${at}.code`, value.code, LIMITS.code, (v, i) => codePair(errors, i, v))
  }
  if (value.tools !== undefined) {
    part.tools = list(errors, `${at}.tools`, value.tools, LIMITS.tools, (v, i) => tool(errors, i, v))
  }
  if (value.detail !== undefined) {
    part.detail = markup(errors, `${at}.detail`, value.detail, LIMITS.detailChars, SWALLOWS_DETAIL)
  }
  return part
}

const file = (errors: Errors, at: string, value: unknown): FileChange => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { path: '', change: 'change', why: '' }
  }
  const path = text(errors, `${at}.path`, value.path, { chars: LIMITS.files.pathChars })
  const change = value.change
  if (typeof change !== 'string' || !CHANGES.includes(change)) errors.push(`${at}.change must be new, change or remove`)
  const why = text(errors, `${at}.why`, value.why, { words: LIMITS.files.whyWords })
  return { path, change: change as Change, why }
}

const check = (errors: Errors, at: string, value: unknown): Check => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { check: '', how: '' }
  }
  return {
    check: text(errors, `${at}.check`, value.check, { words: LIMITS.checks.words }),
    how: text(errors, `${at}.how`, value.how, { words: LIMITS.checks.words }),
  }
}

// Answers the report, or every problem with it, joined.
export const parseReport = (e: Record<string, unknown>): Report | string => {
  const errors: Errors = []
  const lines = (at: string, value: unknown, count: Count & { readonly words: number }): string[] =>
    list(errors, at, value, count, (v, i) => text(errors, i, v, { words: count.words }))
  const report: Report = {
    title: text(errors, 'title', e.title, { chars: LIMITS.titleChars }),
    summary: text(errors, 'summary', e.summary, { words: LIMITS.summaryWords }),
    changes: lines('changes', e.changes, LIMITS.changes),
    unchanged: lines('unchanged', e.unchanged, LIMITS.unchanged),
    needs_your_eye: lines('needs_your_eye', e.needs_your_eye, LIMITS.needsYourEye),
    terms: list(errors, 'terms', e.terms, LIMITS.terms, (v, at) => term(errors, at, v)),
    sections: list(errors, 'sections', e.sections, LIMITS.sections, (v, at) => section(errors, at, v)),
    files: list(errors, 'files', e.files, LIMITS.files, (v, at) => file(errors, at, v)),
    checks: list(errors, 'checks', e.checks, LIMITS.checks, (v, at) => check(errors, at, v)),
  }
  if (errors.length === 0) return report
  const more = errors.length > MAX_ERRORS ? `; and ${errors.length - MAX_ERRORS} more` : ''
  return errors.slice(0, MAX_ERRORS).join('; ') + more
}

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

// A report made in this session, and the plan the person was shown when
// they asked for it.
export type MadeReport = { path: string; plan: Plan }

const samePlan = (a: Plan, b: Plan): boolean =>
  a.description === b.description && a.notes.length === b.notes.length && a.notes.every((n, i) => n === b.notes[i])

// The plan's own notes: those that link one of the session's reports are
// the mod's to write, so they are dropped here and written afresh. A note
// linking a report from an earlier session is the plan's own.
const ownNotes = (made: readonly MadeReport[], notes: readonly string[]): string[] =>
  notes.filter(note => !(note.startsWith('Report') && made.some(m => note.endsWith(m.path))))

// The notes the task is written with: the plan's own, then one linking each
// report made this session, in the order made, marked as an earlier draft
// when the plan changed since. A report note the model kept is relabelled,
// never doubled.
export const withReportNotes = (made: readonly MadeReport[], plan: Plan): string[] => {
  const own = ownNotes(made, plan.notes)
  const now = { description: plan.description, notes: own }
  return [
    ...own,
    ...made.map(m =>
      samePlan({ description: m.plan.description, notes: ownNotes(made, m.plan.notes) }, now)
        ? `Report: ${m.path}`
        : `Report (earlier draft): ${m.path}`,
    ),
  ]
}

// Report mode: the card's Report button on a planned task, with no refine
// before it. Why the task may not be reported on, or undefined when it is
// pending and has a plan.
export const unreportable = (task: Task | undefined): string | undefined => {
  if (task === undefined) return 'the task no longer exists'
  if (task.status !== 'pending') return `the task is ${task.status}, not pending`
  if (!(task.tags ?? []).includes('planned')) return 'the task has no plan: it is not tagged planned. Refine it first'
  return undefined
}

// The task with one more note, linking the report, after the others: at
// `nowMs`, or the first later second no note has yet, since Taskwarrior keys
// a note by its entry time and two at one second would collapse into one.
// Everything else about the task is kept.
export const withReportNote = (task: Task, path: string, nowMs: number): Task => {
  const notes = task.annotations ?? []
  const taken = new Set(notes.map(note => note.entry))
  let at = Math.floor(nowMs / 1000) * 1000
  while (taken.has(taskDate(at))) at += 1000
  return { ...task, annotations: [...notes, { entry: taskDate(at), description: `Report: ${path}` }] }
}

// What the model reads when it calls show_task_report a second time in a
// report session: one report per press of the button.
export const madeAlready = (path: string): string =>
  `show_task_report: this session's report is made and linked from the task: ${path}. ` +
  'Close this tab and press Report on the card again for another.'
