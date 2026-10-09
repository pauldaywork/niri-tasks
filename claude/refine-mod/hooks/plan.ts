// The pure half of write_task_plan: no `$`, so the unit tests reach it
// directly.

export type Plan = { description: string; notes: string[] }

export type PlanInput = { expected: Plan; description: string; notes: string[] }

export type Annotation = { entry: string; description: string }

export type Task = {
  uuid: string
  status: string
  description: string
  annotations?: Annotation[]
  tags?: string[]
  [field: string]: unknown
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

export const isUuid = (value: unknown): value is string =>
  typeof value === 'string' && UUID.test(value)

// The tool is offered only where the sandbox is on and a missing sandbox
// fails the session instead of running unsandboxed.
export const isSandboxed = (settings: Readonly<Record<string, unknown>>): boolean => {
  const sandbox = settings.sandbox as Record<string, unknown> | undefined
  return sandbox?.enabled === true && sandbox?.failIfUnavailable === true
}

// Control and invisible characters, which could make text shown to the
// person look unlike what is written.
export const HIDDEN = /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}\p{Cs}]/u

const isStrings = (value: unknown): value is string[] =>
  Array.isArray(value) && value.every(item => typeof item === 'string')

const toPlan = (value: unknown, where: string): Plan | string => {
  if (typeof value !== 'object' || value === null) return `${where} must be an object`
  const { description, notes } = value as Record<string, unknown>
  if (typeof description !== 'string') return `${where}.description must be a string`
  if (!isStrings(notes)) return `${where}.notes must be an array of strings`
  return { description, notes }
}

// Answers the input, or a string saying what is wrong with it.
export const parseInput = (e: Record<string, unknown>): PlanInput | string => {
  const expected = toPlan(e.expected, 'expected')
  if (typeof expected === 'string') return expected
  const plan = toPlan(e, 'input')
  if (typeof plan === 'string') return plan
  if (plan.description.trim() === '') return 'description must not be empty'
  if (/[\r\n]/.test(plan.description)) return 'description must be one line'
  if (plan.notes.some(note => note.trim() === '')) return 'notes must not be empty'
  if (plan.notes.some(note => /[\r\n]/.test(note))) return 'each note must be one line'
  // Escape sequences, tabs, bidi overrides and zero-width characters could make
  // the logged line look unlike what `task import` writes.
  if (HIDDEN.test(plan.description)) {
    return 'description must be plain text: no control or invisible characters'
  }
  if (plan.notes.some(note => HIDDEN.test(note))) {
    return 'notes must be plain text: no control or invisible characters'
  }
  return { expected, ...plan }
}

// `task <uuid> export` with rc.json.array=on: an array of zero or one task.
export const parseExport = (stdout: string): Task | undefined => {
  const tasks = JSON.parse(stdout) as unknown
  if (!Array.isArray(tasks)) throw new Error('task export did not answer an array')
  return tasks[0] as Task | undefined
}

const notesOf = (task: Task): string[] =>
  (task.annotations ?? []).map(annotation => annotation.description)

const same = (a: readonly string[], b: readonly string[]): boolean =>
  a.length === b.length && a.every((item, i) => item === b[i])

// Why the task may not be written, or undefined when it still matches what
// the skill read.
export const refusal = (task: Task | undefined, expected: Plan): string | undefined => {
  if (task === undefined) return 'the task no longer exists'
  if (task.status !== 'pending') return `the task is ${task.status}, not pending`
  const changes: string[] = []
  if (task.description !== expected.description) {
    changes.push(
      `its description is now ${JSON.stringify(task.description)}, ` +
        `not ${JSON.stringify(expected.description)}`,
    )
  }
  const notes = notesOf(task)
  if (!same(notes, expected.notes)) {
    changes.push(`its notes are now ${JSON.stringify(notes)}, not ${JSON.stringify(expected.notes)}`)
  }
  if (changes.length === 0) return undefined
  return `the task changed since it was read: ${changes.join('; ')}. Read it again before planning.`
}

// Taskwarrior's date form, YYYYMMDDTHHMMSSZ, in UTC.
export const taskDate = (ms: number): string =>
  new Date(ms).toISOString().replace(/\.\d{3}Z$/, 'Z').replace(/[-:]/g, '')

// The task as it is to be imported. Each note is one second after the one
// before: Taskwarrior keys an annotation by its entry time, so two notes at
// the same second would collapse into one.
export const merge = (task: Task, plan: PlanInput, nowMs: number): Task => {
  const base = Math.floor(nowMs / 1000) * 1000
  return {
    ...task,
    description: plan.description,
    annotations: plan.notes.map((description, i) => ({
      entry: taskDate(base + i * 1000),
      description,
    })),
    tags: [...new Set([...(task.tags ?? []), 'planned'])].sort(),
  }
}

// The approval the tool asks for before it writes. Only WRITE, compared
// exactly, approves: `$.ui.ask` answers the label chosen or the words typed
// under Other.
export const QUESTION = 'Write this to the task?'
export const WRITE = 'Write it to the task'
export const CHANGE = 'Change something'
export const HEADER = 'Task plan'

// The most of one `$.ui.log` line the terminal draws.
export const MAX_LINE = 2000

// What the person reads before the question: the plan from the tool's own
// arguments, one line for the description and one per note.
export const planLines = (plan: Plan): string[] => [
  `Description: ${plan.description}`,
  ...(plan.notes.length === 0
    ? ['Notes: none (the task keeps no notes)']
    : plan.notes.map((note, i) => `Note ${i + 1}: ${note}`)),
]

// Why the lines may not be shown, or undefined when the terminal draws each
// whole: the person must not approve what was cut from view.
export const overlong = (lines: readonly string[]): string | undefined => {
  const i = lines.findIndex(line => line.length > MAX_LINE)
  if (i === -1) return undefined
  const what = i === 0 ? 'the description' : `note ${i}`
  return (
    `${what} would show as a line of ${lines[i]?.length} characters, ` +
    `over the ${MAX_LINE} the terminal draws. Shorten it or split it into notes, then call again.`
  )
}

// What the model reads when the person did not choose WRITE: their choice,
// or the words they typed under Other, so it can revise and call again.
export const notApproved = (answer: string): string =>
  answer === CHANGE
    ? `Nothing written: the person chose "${CHANGE}". Ask them what to change, ` +
      'revise the plan, and call write_task_plan again.'
    : `Nothing written: the person answered ${JSON.stringify(answer)} instead of approving. ` +
      'Revise the plan with that, and call write_task_plan again.'

// What the model reads when no one could be asked (a `-p` run, a dismissed
// dialog): the tool writes only on the person's own "Write it to the task".
export const notAsked = (reason: string): string =>
  `Nothing written: the person could not be asked, or dismissed the question (${reason}). ` +
  `write_task_plan writes only after they choose "${WRITE}".`
