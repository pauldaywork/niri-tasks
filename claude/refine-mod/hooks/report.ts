// The pure half of show_task_report: the fields it takes from the model, the
// limits that keep a report short enough to take in, and where it is saved.
// No `$`, so the unit tests reach it directly.

import { HIDDEN, taskDate } from './plan'
import type { Plan, Task } from './plan'

export type Change = 'new' | 'change' | 'remove'
// A diagram is Mermaid source: text the Markdown twin keeps as it is, and the
// page draws before it opens.
export type Diagram = { mermaid: string }
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
  // A branch report's own parts; absent from a plan report.
  branch?: Branch
}

// Which report a session makes: of the task's plan, or of what its branch
// has changed so far.
export type ReportKind = 'plan' | 'branch'

// A branch report's parts each look through a named lens: it has no plain
// part, so its slides keep one order.
export const BRANCH_LENSES: readonly Lens[] = LENSES.filter(lens => lens !== 'part')

export type Layer = 'unit' | 'integration' | 'e2e'
export const LAYERS: readonly string[] = ['unit', 'integration', 'e2e']
export type Result = 'pass' | 'fail' | 'not run'
export const RESULTS: readonly string[] = ['pass', 'fail', 'not run']
export type PlanKind = 'step' | 'done when'
export const PLAN_KINDS: readonly string[] = ['step', 'done when']
export type Progress = 'done' | 'partial' | 'not yet'
export const PROGRESS: readonly string[] = ['done', 'partial', 'not yet']

// Where a reviewer should look hardest, and the files it is in.
export type HotSpot = { spot: string; why: string; files: string[] }
// A commit as the deck shows it: its short sha and its subject, from git.
export type Commit = { sha: string; subject: string }
// A step of the commit story: its commits, by sha as the model names them,
// or as commits once git has named them.
export type Step<C> = { step: string; commits: C[]; points: string[] }
export type MapItem = { name: string; does: string }
// A changed file, and what each function or part of it the branch changes
// now does.
export type MapEntry = { path: string; change: Change; items: MapItem[] }
export type SuiteRun = { suite: string; command: string; passed: number; failed: number }
export type NotRun = { suite: string; why: string }
export type Failing = { test: string; file: string; why: string }
// The whole test suite as the session ran it.
export type Suite = { runs: SuiteRun[]; not_run: NotRun[]; failing: Failing[] }
// A test the branch adds or changes.
export type NewTest = { test: string; file: string; layer: Layer; pins: string; result: Result }
export type Untested = { code: string; file: string; why: string }
// One of the task's Steps or Done when notes, and how far the branch has
// got with it.
export type PlanItem = { item: string; kind: PlanKind; progress: Progress; shown_by: string }
// What git says of the branch: read by the mod, never given by the model.
export type Facts = {
  branch: string
  base: string
  merge_base: string
  commits: number
  files: number
  added: number
  removed: number
  uncommitted: number
}

// A branch report's own fields, as the model gives them.
export type BranchInput = {
  hot_spots: HotSpot[]
  story: Step<string>[]
  map: MapEntry[]
  suite: Suite
  new_tests: NewTest[]
  untested: Untested[]
  plan_vs_done: PlanItem[]
}

// The project's tests as Rust ran them, outside the sandbox, before an
// Explain branch session started: the results file it wrote beside the
// run's log.
export type TestRun = { command: string; exit_code: number; log: string; started: string; finished: string }

// What a report keeps of the run: what the suite slide and its twin name.
// Its times are left in the file, so the twin reads back the whole of it.
export type RunLine = Pick<TestRun, 'command' | 'exit_code' | 'log'>

// As a report holds them: each commit with its subject, and the branch's
// facts, both from git (branch.ts); and the test run, when there was one.
export type Branch = Omit<BranchInput, 'story'> & { facts: Facts; story: Step<Commit>[]; run?: RunLine }

// The test run from its results file, or why it cannot be read.
export const parseTestRun = (text: string): TestRun | string => {
  let value: unknown
  try {
    value = JSON.parse(text)
  } catch (error) {
    return `it is not JSON (${error instanceof Error ? error.message : String(error)})`
  }
  if (!isObject(value) || Array.isArray(value)) return 'it is not a JSON object'
  for (const field of ['command', 'log', 'started', 'finished'] as const) {
    if (typeof value[field] !== 'string') return `${field} must be a string`
  }
  if (typeof value.exit_code !== 'number' || !Number.isInteger(value.exit_code)) return 'exit_code must be a whole number'
  const { command, exit_code, log, started, finished } = value as TestRun
  return { command, exit_code, log, started, finished }
}

// Why the model's suite cannot be the run's: a run that passed had no
// failure, and one that failed had at least one, which its log names.
// Nothing when they agree.
export const checkSuite = (suite: Suite, run: TestRun): string[] => {
  const failed = suite.runs.some(r => r.failed > 0) || suite.failing.length > 0
  if (run.exit_code === 0 && failed) return ['the test run passed (exit 0), so no suite can have failed']
  if (run.exit_code !== 0 && !failed) {
    return [`the test run failed (exit ${run.exit_code}): name the suite and the tests that failed, from its log ${run.log}`]
  }
  return []
}

// The branch fields' names, refused in a plan report.
const BRANCH_FIELDS = ['hot_spots', 'story', 'map', 'suite', 'new_tests', 'untested', 'plan_vs_done'] as const

// The branch slides' headings, the same in the page and its twin.
export const BRANCH_HEADINGS = {
  hotSpots: 'Where to look hardest',
  story: 'How the branch got here',
  map: 'What each file now does',
  suite: 'Did anything break?',
  newTests: 'The tests this branch adds',
  untested: 'Changed code with no test',
  planVsDone: 'Plan vs done',
} as const

// What keeps a report easy to take in: about four things held in mind at
// once, one idea per line, short plain sentences. A report over them is
// refused with every reason at once, so it is fixed in one go; a line only
// a word or two over its word limit is taken, and named, so one word does
// not cost a whole report resent.
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

// What keeps a branch report's own slides easy to take in. A test list may
// run long: the reader asked for every test the branch adds.
export const BRANCH_LIMITS = {
  hotSpots: { min: 1, max: 4, spotWords: 10, whyWords: 30, files: { min: 1, max: 4 } },
  story: { min: 1, max: 8, stepWords: 10, shaChars: 40, commits: { min: 1, max: 60 }, points: { min: 1, max: 3, words: 25 } },
  map: { min: 1, max: 20, items: { min: 1, max: 6, nameChars: 80, words: 20 } },
  suite: {
    runs: { min: 1, max: 8, suiteWords: 6, commandChars: 200 },
    notRun: { min: 0, max: 6, words: 25 },
    failing: { min: 0, max: 10, testChars: 200, words: 25 },
  },
  newTests: { min: 0, max: 30, testChars: 200, pinsWords: 25 },
  untested: { min: 0, max: 8, codeWords: 8, whyWords: 20 },
  planVsDone: { min: 0, max: 20, itemWords: 30, shownByWords: 25 },
} as const

// How many words a line may run over its word limit and still be taken.
// Not in the tool's descriptions or the catalogue: the model aims for the
// limits themselves.
export const WORD_GRACE = 2

export const CHANGES: readonly string[] = ['new', 'change', 'remove']

// Elements refused by name: a <meta> refresh navigates and a <meta> policy
// could be added to, which the page's own policy does not stop; the rest would
// reach outside or run script, and fail quietly under it. Refused so the model
// hears why rather than finding a blank figure.
const FORBIDDEN = /<\s*(script|meta|base|link|iframe|frameset|frame|object|embed|form|portal)(?![\w-])/i

// What would break the page's one layout, refused in detail, which the page
// shows as given (Mermaid source is escaped, so it cannot): a comment left
// open hides the rest of the page, and closing one of the page's own
// elements moves what follows out of its place. Drawn diagrams are held to
// them too.
export const COMMENT = /<!--/
export const CLOSES = /<\s*\/\s*(details|section|main|footer|nav|figure|header|body|html)(?![\w-])/i

// A drawn diagram is one whole <svg> element, nothing before or after.
export const WHOLE_SVG = /^\s*<svg[\s>][\s\S]*<\/svg>\s*$/i

// Elements that swallow the rest of the page as text, or restyle it.
const SWALLOWS_DETAIL = /<\s*(style|plaintext|xmp|textarea|title|noscript|template|html|head|body)(?![\w-])/i

// How many problems one refusal lists before it says how many more.
const MAX_ERRORS = 20

// The problems found, refused all at once, and beside them the lines taken
// though over their word limits, which the tool names.
type Errors = string[] & { warnings: string[] }
const findings = (): Errors => Object.assign([] as string[], { warnings: [] as string[] })
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
  if (limit.words !== undefined && n > limit.words) {
    const over = `${at} has ${n} words; at most ${limit.words}`
    if (n - limit.words <= WORD_GRACE) errors.warnings.push(over)
    else errors.push(over)
  }
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
  if (!isObject(value) || !('mermaid' in value)) {
    errors.push(`${at} must be { "mermaid": "…" }`)
    return undefined
  }
  const source = markup(errors, `${at}.mermaid`, value.mermaid, LIMITS.diagramChars)
  const lines = source.split('\n').length
  if (lines > LIMITS.mermaidLines) {
    errors.push(`${at}.mermaid has ${lines} lines; at most ${LIMITS.mermaidLines}: split it into two diagrams`)
  }
  return { mermaid: source }
}

export const SWAPS: readonly string[] = ['easy', 'medium', 'hard']

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

const section = (errors: Errors, at: string, value: unknown, kind: ReportKind): Section => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { heading: '', points: [] }
  }
  const lens = value.kind
  if (kind === 'branch' && (lens === undefined || lens === 'part')) {
    errors.push(`${at}.kind is needed in a branch report: one of ${BRANCH_LENSES.join(', ')}`)
  } else if (lens !== undefined && (typeof lens !== 'string' || !(LENSES as readonly string[]).includes(lens))) {
    errors.push(`${at}.kind must be one of ${LENSES.join(', ')}`)
  }
  const part: Section = {
    ...(lens !== undefined ? { kind: lens as Lens } : {}),
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

// A whole number, 0 or more.
const whole = (errors: Errors, at: string, value: unknown): number => {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 0) {
    errors.push(`${at} must be a whole number, 0 or more`)
    return 0
  }
  return value
}

// One of a few words, `said` naming them for the refusal.
const oneOf = <T extends string>(errors: Errors, at: string, value: unknown, allowed: readonly string[], said: string): T => {
  if (typeof value !== 'string' || !allowed.includes(value)) errors.push(`${at} must be ${said}`)
  return value as T
}

// An object's fields, or none when it is not one.
const fieldsOf = (errors: Errors, at: string, value: unknown): Record<string, unknown> => {
  if (isObject(value)) return value
  errors.push(`${at} must be an object`)
  return {}
}

const pathText = (errors: Errors, at: string, value: unknown): string =>
  text(errors, at, value, { chars: LIMITS.files.pathChars })

// A branch report's own fields, every problem noted.
const branchInput = (errors: Errors, e: Record<string, unknown>): BranchInput => {
  const B = BRANCH_LIMITS
  const suite = fieldsOf(errors, 'suite', e.suite)
  return {
    hot_spots: list(errors, 'hot_spots', e.hot_spots, B.hotSpots, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        spot: text(errors, `${at}.spot`, o.spot, { words: B.hotSpots.spotWords }),
        why: text(errors, `${at}.why`, o.why, { words: B.hotSpots.whyWords }),
        files: list(errors, `${at}.files`, o.files, B.hotSpots.files, (f, i) => pathText(errors, i, f)),
      }
    }),
    story: list(errors, 'story', e.story, B.story, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        step: text(errors, `${at}.step`, o.step, { words: B.story.stepWords }),
        commits: list(errors, `${at}.commits`, o.commits, B.story.commits, (c, i) =>
          text(errors, i, c, { chars: B.story.shaChars }),
        ),
        points: list(errors, `${at}.points`, o.points, B.story.points, (p, i) =>
          text(errors, i, p, { words: B.story.points.words }),
        ),
      }
    }),
    map: list(errors, 'map', e.map, B.map, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        path: pathText(errors, `${at}.path`, o.path),
        change: oneOf<Change>(errors, `${at}.change`, o.change, CHANGES, 'new, change or remove'),
        items: list(errors, `${at}.items`, o.items, B.map.items, (it, i) => {
          const m = fieldsOf(errors, i, it)
          return {
            name: text(errors, `${i}.name`, m.name, { chars: B.map.items.nameChars }),
            does: text(errors, `${i}.does`, m.does, { words: B.map.items.words }),
          }
        }),
      }
    }),
    suite: {
      runs: list(errors, 'suite.runs', suite.runs, B.suite.runs, (v, at) => {
        const o = fieldsOf(errors, at, v)
        return {
          suite: text(errors, `${at}.suite`, o.suite, { words: B.suite.runs.suiteWords }),
          command: text(errors, `${at}.command`, o.command, { chars: B.suite.runs.commandChars }),
          passed: whole(errors, `${at}.passed`, o.passed),
          failed: whole(errors, `${at}.failed`, o.failed),
        }
      }),
      not_run: list(errors, 'suite.not_run', suite.not_run, B.suite.notRun, (v, at) => {
        const o = fieldsOf(errors, at, v)
        return {
          suite: text(errors, `${at}.suite`, o.suite, { words: B.suite.runs.suiteWords }),
          why: text(errors, `${at}.why`, o.why, { words: B.suite.notRun.words }),
        }
      }),
      failing: list(errors, 'suite.failing', suite.failing, B.suite.failing, (v, at) => {
        const o = fieldsOf(errors, at, v)
        return {
          test: text(errors, `${at}.test`, o.test, { chars: B.suite.failing.testChars }),
          file: pathText(errors, `${at}.file`, o.file),
          why: text(errors, `${at}.why`, o.why, { words: B.suite.failing.words }),
        }
      }),
    },
    new_tests: list(errors, 'new_tests', e.new_tests, B.newTests, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        test: text(errors, `${at}.test`, o.test, { chars: B.newTests.testChars }),
        file: pathText(errors, `${at}.file`, o.file),
        layer: oneOf<Layer>(errors, `${at}.layer`, o.layer, LAYERS, 'unit, integration or e2e'),
        pins: text(errors, `${at}.pins`, o.pins, { words: B.newTests.pinsWords }),
        result: oneOf<Result>(errors, `${at}.result`, o.result, RESULTS, 'pass, fail or not run'),
      }
    }),
    untested: list(errors, 'untested', e.untested, B.untested, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        code: text(errors, `${at}.code`, o.code, { words: B.untested.codeWords }),
        file: pathText(errors, `${at}.file`, o.file),
        why: text(errors, `${at}.why`, o.why, { words: B.untested.whyWords }),
      }
    }),
    plan_vs_done: list(errors, 'plan_vs_done', e.plan_vs_done, B.planVsDone, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        item: text(errors, `${at}.item`, o.item, { words: B.planVsDone.itemWords }),
        kind: oneOf<PlanKind>(errors, `${at}.kind`, o.kind, PLAN_KINDS, 'step or done when'),
        progress: oneOf<Progress>(errors, `${at}.progress`, o.progress, PROGRESS, 'done, partial or not yet'),
        shown_by: text(errors, `${at}.shown_by`, o.shown_by, { words: B.planVsDone.shownByWords }),
      }
    }),
  }
}

// A report the tool takes, the lines in it over their word limits, and, for
// a branch report, its own fields as the model gave them.
export type Parsed = { report: Report; warnings: string[]; branch?: BranchInput }

// Answers the report, or every problem with it, joined. A branch report has
// no needs_your_eye or checks: its hot spots and its plan vs done take their
// places.
export const parseReport = (e: Record<string, unknown>, kind: ReportKind = 'plan'): Parsed | string => {
  const errors = findings()
  const lines = (at: string, value: unknown, count: Count & { readonly words: number }): string[] =>
    list(errors, at, value, count, (v, i) => text(errors, i, v, { words: count.words }))
  const branch = kind === 'branch'
  // A plan report's field a branch report leaves out, refused if given.
  const none = (at: string, value: unknown, instead: string): [] => {
    if (value !== undefined && !(Array.isArray(value) && value.length === 0)) {
      errors.push(`${at} is not used in a branch report: ${instead} takes its place`)
    }
    return []
  }
  const report: Report = {
    title: text(errors, 'title', e.title, { chars: LIMITS.titleChars }),
    summary: text(errors, 'summary', e.summary, { words: LIMITS.summaryWords }),
    changes: lines('changes', e.changes, LIMITS.changes),
    unchanged: lines('unchanged', e.unchanged, LIMITS.unchanged),
    needs_your_eye: branch
      ? none('needs_your_eye', e.needs_your_eye, 'hot_spots')
      : lines('needs_your_eye', e.needs_your_eye, LIMITS.needsYourEye),
    terms: list(errors, 'terms', e.terms, LIMITS.terms, (v, at) => term(errors, at, v)),
    sections: list(errors, 'sections', e.sections, LIMITS.sections, (v, at) => section(errors, at, v, kind)),
    files: list(errors, 'files', e.files, LIMITS.files, (v, at) => file(errors, at, v)),
    checks: branch
      ? none('checks', e.checks, 'plan_vs_done')
      : list(errors, 'checks', e.checks, LIMITS.checks, (v, at) => check(errors, at, v)),
  }
  const input = branch ? branchInput(errors, e) : undefined
  if (!branch) {
    for (const field of BRANCH_FIELDS) if (e[field] !== undefined) errors.push(`${field} is only for a branch report`)
  }
  if (errors.length === 0) {
    return input === undefined ? { report, warnings: errors.warnings } : { report, warnings: errors.warnings, branch: input }
  }
  const more = errors.length > MAX_ERRORS ? `; and ${errors.length - MAX_ERRORS} more` : ''
  return errors.slice(0, MAX_ERRORS).join('; ') + more
}

// What the tool's answer adds for the lines taken over their word limits:
// each named, so the next report keeps to the limits. Nothing when none is.
export const overLimits = (warnings: readonly string[]): string =>
  warnings.length === 0 ? '' : ` Taken, though over its word limits: ${warnings.join('; ')}.`

// An absolute folder from the session's settings, without a trailing slash,
// or undefined.
const absoluteDir = (value: unknown): string | undefined => {
  if (typeof value !== 'string' || !value.startsWith('/')) return undefined
  const dir = value.replace(/\/+$/, '')
  return dir === '' ? undefined : dir
}

// The folder the reports go in, from the session's settings: absolute, or
// none, and then the tool is not offered.
export const reportsDir = absoluteDir

// The home folder, from the session's settings: absolute, or none, and then
// every report is named by its absolute path.
export const homeDir = absoluteDir

// A report's path as the task's note and the tool's answers name it: from
// `~/` under `home`, so the note survives a username or home folder change
// (`niritasks` reads a `~/` note back under the home folder); absolute
// elsewhere, as for a data folder outside home. The files themselves are
// written and opened by the absolute path.
export const notePath = (path: string, home: string | undefined): string =>
  home !== undefined && path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path

// <dir>/<refine|branch>-<uuid8>-<YYYYMMDD-HHMMSS>.html in UTC: a second
// report on the same task, after a revision, does not overwrite the first.
export const reportPath = (dir: string, uuid: string, nowMs: number, kind: ReportKind = 'plan'): string => {
  const stamp = new Date(nowMs).toISOString().slice(0, 19).replace(/-|:/g, '').replace('T', '-')
  return `${dir}/${kind === 'branch' ? 'branch' : 'refine'}-${uuid.slice(0, 8).toLowerCase()}-${stamp}.html`
}

// `niritasks task open-report`, which opens a new browser window and moves
// it to this session's workspace however the user has moved since; xdg-open
// where niritasks is not on PATH or fails. In the background with its streams
// closed: a browser it starts would otherwise hold `$.process.run` until its
// timeout. The path is `$1`, an argument, never part of the script.
export const openArgv = (path: string): string[] => [
  'sh',
  '-c',
  '{ niritasks task open-report "$1" || xdg-open "$1"; } >/dev/null 2>&1 </dev/null &',
  'sh',
  path,
]

// A report made in this session, by the path its note names (`notePath`),
// and the plan the person was shown when they asked for it.
export type MadeReport = { path: string; plan: Plan }

const samePlan = (a: Plan, b: Plan): boolean =>
  a.description === b.description && a.notes.length === b.notes.length && a.notes.every((n, i) => n === b.notes[i])

// What a note linking a report starts with, as the tool writes it: the
// report of the plan, or one of a plan since changed. The card opens only
// the first.
export const REPORT_NOTE = 'Report: '
const EARLIER_DRAFT_NOTE = 'Report (earlier draft): '

// What a note linking a branch report starts with. Never `Report: `, so a
// branch report never makes the plan's report an earlier draft.
export const BRANCH_REPORT_NOTE = 'Branch report: '

const isReportNote = (note: string): boolean => note.startsWith(REPORT_NOTE) || note.startsWith(EARLIER_DRAFT_NOTE)

// The plan with its report notes dropped: what a refine changes.
const planOnly = (plan: Plan): Plan => ({ description: plan.description, notes: plan.notes.filter(n => !isReportNote(n)) })

// The plan's own notes: those that link one of the session's reports are
// the mod's to write, so they are dropped here and written afresh. A note
// linking a report from an earlier session is the plan's own.
const ownNotes = (made: readonly MadeReport[], notes: readonly string[]): string[] =>
  notes.filter(note => !(note.startsWith('Report') && made.some(m => note.endsWith(m.path))))

// The notes the task is written with: the plan's own, then one linking each
// report made this session, in the order made, marked as an earlier draft
// when the plan changed since. A report note the model kept is relabelled,
// never doubled. When the plan differs from `expected`, the task as read, a
// `Report:` note from an earlier session reports the old plan, so it is
// marked as an earlier draft in its place and the card no longer opens it.
export const withReportNotes = (made: readonly MadeReport[], plan: Plan, expected: Plan): string[] => {
  const own = ownNotes(made, plan.notes)
  const changed = !samePlan(planOnly(expected), planOnly(plan))
  // Each made report is held to the notes before any relabel: the plan it
  // was shown kept the earlier session's note as the model did.
  const now = { description: plan.description, notes: own }
  return [
    ...own.map(note =>
      changed && note.startsWith(REPORT_NOTE) ? EARLIER_DRAFT_NOTE + note.slice(REPORT_NOTE.length) : note,
    ),
    ...made.map(m =>
      samePlan({ description: m.plan.description, notes: ownNotes(made, m.plan.notes) }, now)
        ? `${REPORT_NOTE}${m.path}`
        : `${EARLIER_DRAFT_NOTE}${m.path}`,
    ),
  ]
}

// Report mode: the Go to list's Generate report on a planned task, with no refine
// before it. Why the task may not be reported on, or undefined when it is
// pending and has a plan.
export const unreportable = (task: Task | undefined): string | undefined => {
  if (task === undefined) return 'the task no longer exists'
  if (task.status !== 'pending') return `the task is ${task.status}, not pending`
  if (!(task.tags ?? []).includes('planned')) return 'the task has no plan: it is not tagged planned. Refine it first'
  return undefined
}

// Explain branch: why the task's branch may not be reported on, or
// undefined. Any task but a deleted one: a branch needs no plan, and a
// finished task's branch can still be read. Rust checks it has a worktree.
export const unexplainable = (task: Task | undefined): string | undefined => {
  if (task === undefined) return 'the task no longer exists'
  if (task.status === 'deleted') return 'the task is deleted'
  return undefined
}

// The task with one more note, linking the report, after the others: at
// `nowMs`, or the first later second no note has yet, since Taskwarrior keys
// a note by its entry time and two at one second would collapse into one.
// Everything else about the task is kept. A branch report's note is a
// `Branch report: `, never the plan's `Report: `.
export const withReportNote = (task: Task, path: string, nowMs: number, kind: ReportKind = 'plan'): Task => {
  const notes = task.annotations ?? []
  const taken = new Set(notes.map(note => note.entry))
  let at = Math.floor(nowMs / 1000) * 1000
  while (taken.has(taskDate(at))) at += 1000
  const prefix = kind === 'branch' ? BRANCH_REPORT_NOTE : REPORT_NOTE
  return { ...task, annotations: [...notes, { entry: taskDate(at), description: `${prefix}${path}` }] }
}

// What the model reads when it calls show_task_report a second time in a
// report session: one report per press of the button, or per Explain
// branch.
export const madeAlready = (path: string, kind: ReportKind = 'plan'): string =>
  kind === 'branch'
    ? `show_task_report: this session's branch report is made and linked from the task: ${path}. ` +
      "Close this tab and choose Explain branch in the card's Go to list again for another."
    : `show_task_report: this session's report is made and linked from the task: ${path}. ` +
      'Close this tab and press Report on the card again for another.'
