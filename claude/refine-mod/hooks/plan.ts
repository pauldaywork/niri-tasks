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
