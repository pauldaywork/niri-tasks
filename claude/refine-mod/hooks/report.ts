// The pure half of show_task_report: the fields it takes from the model, the
// limits that keep a report short enough to take in, and where it is saved.
// No `$`, so the unit tests reach it directly.

import { HIDDEN } from './plan'
import type { Plan } from './plan'

export type Change = 'new' | 'change' | 'remove'
export type Diagram = { mermaid: string } | { svg: string }
export type Section = {
  heading: string
  look_at?: string
  diagram?: Diagram
  points: string[]
  detail?: string
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
  sections: { min: 1, max: 6, headingWords: 10, lookAtWords: 25 },
  points: { min: 1, max: 5, words: 25 },
  mermaidLines: 40,
  diagramChars: 50_000,
  detailChars: 20_000,
  files: { min: 1, max: 40, pathChars: 200, whyWords: 20 },
  checks: { min: 1, max: 6, words: 20 },
} as const

const CHANGES: readonly string[] = ['new', 'change', 'remove']

// Elements refused by name: a <meta> refresh navigates and a <meta> policy
// could be added to, which the page's own policy does not stop; the rest would
// reach outside or run script, and fail quietly under it. Refused so the model
// hears why rather than finding a blank figure.
const FORBIDDEN = /<\s*(script|meta|base|link|iframe|frameset|frame|object|embed|form|portal)(?![\w-])/i

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

// Markup the page shows as given: within its size, and holding nothing the
// page must not.
const markup = (errors: Errors, at: string, value: unknown, maxChars: number): string => {
  if (typeof value !== 'string' || value.trim() === '') {
    errors.push(`${at} must be a non-empty string`)
    return ''
  }
  if (value.length > maxChars) errors.push(`${at} has ${value.length} characters; at most ${maxChars}`)
  const tag = FORBIDDEN.exec(value)?.[1]?.toLowerCase()
  if (tag !== undefined) {
    errors.push(`${at} must not contain <${tag}>: the page runs no script but Mermaid, loads nothing and sends nothing`)
  }
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
  const svg = markup(errors, `${at}.svg`, value.svg, LIMITS.diagramChars)
  if (svg !== '' && !/^\s*<svg[\s>][\s\S]*<\/svg>\s*$/i.test(svg)) errors.push(`${at}.svg must be one <svg> element`)
  return { svg }
}

const section = (errors: Errors, at: string, value: unknown): Section => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { heading: '', points: [] }
  }
  const part: Section = {
    heading: text(errors, `${at}.heading`, value.heading, { words: LIMITS.sections.headingWords }),
    points: list(errors, `${at}.points`, value.points, LIMITS.points, (v, i) =>
      text(errors, i, v, { words: LIMITS.points.words }),
    ),
  }
  if (value.diagram !== undefined) {
    const drawn = diagram(errors, `${at}.diagram`, value.diagram)
    if (drawn !== undefined) part.diagram = drawn
    part.look_at = text(errors, `${at}.look_at`, value.look_at, { words: LIMITS.sections.lookAtWords })
  } else if (value.look_at !== undefined) {
    errors.push(`${at}.look_at needs a diagram to point at`)
  }
  if (value.detail !== undefined) part.detail = markup(errors, `${at}.detail`, value.detail, LIMITS.detailChars)
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

// The notes that link the session's reports from the task, after the plan's
// own: marked as an earlier draft when the plan changed since. A report the
// plan's notes already link is not linked twice.
export const reportNotes = (made: readonly MadeReport[], plan: Plan): string[] =>
  made
    .filter(m => !plan.notes.some(note => note.startsWith('Report') && note.endsWith(m.path)))
    .map(m => (samePlan(m.plan, plan) ? `Report: ${m.path}` : `Report (earlier draft): ${m.path}`))
