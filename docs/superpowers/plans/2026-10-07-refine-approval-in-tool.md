# Approve a Refine's Write Inside the Tool — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The `write_task_plan` tool asks the person itself, and writes only when they choose **Write it to the task**, so approval is enforced by the mod instead of trusted to the model.

**Architecture:** Inside the tool's `tool.call` hook, after the input and stale checks, the mod logs the plan from its own arguments with `$.ui.log` (dim, never sent to the model), asks with `$.ui.ask` in the engine's AskUserQuestion dialog, and only on the exact label **Write it to the task** re-reads the task, re-checks it and imports. Any other answer, or no one to ask, returns a `deny` carrying the person's choice or words. The `refine-task` skill drops its own AskUserQuestion step: step 4 shows the proposal, step 5 calls the tool and reacts to its answer.

**Tech Stack:** Claude Code 2.1.292 mods (TypeScript hooks module; `claude plugin validate` / `claude plugin test`), Taskwarrior 2.6.2, Markdown skill.

**Spec:** Taskwarrior task `dc32d340-d2bb-414b-83e9-45b6c47fb446` (read it with `task rc.json.array=on dc32d340 export`; its notes are the spec) and `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, section "B — A pane with Write and Change buttons" (its Verdict: the approval moves into the write tool as a `$.ui.ask`). Every file below was prototyped and verified on this machine first: `claude plugin validate` clean, `claude plugin test` 29/29, and live runs over `claude -p --input-format stream-json --permission-prompt-tool stdio` that answered the dialog with each of Write, Change something, typed text, a mid-answer task edit and a 20-second wait.

## Global Constraints

- Question, options and header, exactly: `Write this to the task?`; `Write it to the task`, `Change something`; header `Task plan` (12 characters at most).
- Only the exact answer `Write it to the task` writes. `$.ui.ask` resolves to the label chosen or the text typed under Other, so compare exactly.
- The plan shown is built from the tool's own arguments, one `$.ui.log` line each: `Description: …`, then `Note 1: …`, `Note 2: …`, or `Notes: none (the task keeps no notes)`. Refuse the call before asking if any line is over 2000 characters (the most the terminal draws).
- The description and every note must be one line; the tool refuses otherwise.
- The task is exported and checked against `expected` before asking, and again after **Write it to the task**, before importing: the person may take minutes.
- Every "nothing written" outcome is a `{ deny }` (an error result to the model), never a `{ result }`.
- If the ask cannot be made or is dismissed, the tool writes nothing and says the person could not be asked; the message must not depend on the engine's error wording.
- The task's argv, the uuid from `pluginConfigs`, the arming checks and `rc.hooks=off` are unchanged from today's mod.
- Out of scope: the hooks `denyWrite` (task `b268d068`), Start working, anything in `src/`.
- TypeScript comments in the mod's existing plain voice. Commits: Conventional Commits, `feat(refine): …` / `docs(refine): …`, body wrapped at 72, ending with a `Co-Authored-By:` trailer naming the model that wrote it. Land with the `finish-worktree` skill.

## Facts the implementer needs (verified on 2.1.292)

- `$.ui.ask(question, { options, header })` → `Promise<string>` (d.ts `ask`, ~2420; `AskOptions` ~665). It rejects when the dialog is dismissed or no one can answer. In plain `claude -p` the AskUserQuestion tool is absent, so it rejects with `$.tool.call: no tool named "AskUserQuestion" in this session`. Its time does not count against the hook's 10-second budget (`HookBudget`: the clock stops while a `$` call is in flight); a 20 s answer worked live.
- `$.ui.log(text)` → `void`: one dim transcript line, not sent to the model; a `-p` host gets it as `{"type":"system","subtype":"ui_log",…}`.
- Test kit: a test answers the ask with `on('tool.call', { tool: 'AskUserQuestion' }, ($, e) => ({ result: { questions: e.questions, answers: { [e.questions[0].question]: '<label or text>' } } }))`; returning `{ deny }` makes the ask reject. A test sees log lines with `on('ui.log', ($, e) => { …; return { value: undefined } })` — it must return exactly `{ value: undefined }`. Tests never logged before, so earlier tests needed no `ui.log` hook; now every test that reaches the ask needs one.
- `claude --plugin-dir` writes `.claude-plugin/types/` into the plugin folder (self-ignoring); never commit it.
- `--allowedTools` is variadic: write `--allowedTools=<tool>` or it swallows the prompt. Run `claude -p` with `< /dev/null` unless it reads stream-json from stdin.

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `claude/refine-mod/hooks/plan.ts` | Modify | one-line notes; the question's words; `planLines`, `overlong`, `notApproved`, `notAsked` |
| `claude/refine-mod/hooks/refine.ts` | Modify | `current` (export + stale check), log, ask, re-check, import |
| `claude/refine-mod/tests/plan.test.ts` | Modify | unit tests for the new pure parts |
| `claude/refine-mod/tests/refine.test.ts` | Modify | engine tests for every answer |
| `.claude/skills/refine-task/SKILL.md` | Modify | no AskUserQuestion; step 5 reacts to the tool's answer |
| `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` | Modify | one line under Adopted: B built, with the commit |

---

### Task 1: The tool asks before it writes

**Files:**
- Modify: `claude/refine-mod/hooks/plan.ts`, `claude/refine-mod/hooks/refine.ts`
- Modify: `claude/refine-mod/tests/plan.test.ts`, `claude/refine-mod/tests/refine.test.ts`

**Interfaces:**
- Consumes: today's mod (`isSandboxed`, `isUuid`, `merge`, `parseExport`, `parseInput`, `refusal`, `TOOL`, `SPEC`, `armedUuid`, `TASK`).
- Produces: the tool's answers, which Task 2's skill text reacts to, verbatim prefixes: `Wrote the plan to task …` (success); `Nothing written: the person chose "Change something". …`; `Nothing written: the person answered "<words>" instead of approving. …`; `Nothing written: the person could not be asked …`; `Nothing written: the task changed since it was read: …`; `Nothing written: … would show as a line of …`.

Each file below is the whole file after the change. Replace the file's contents with it.

- [ ] **Step 1: Write the failing unit tests**

`claude/refine-mod/tests/plan.test.ts`:

```ts
import { describe, expect, test } from 'claude-code/testing'
import {
  CHANGE,
  MAX_LINE,
  WRITE,
  isSandboxed,
  isUuid,
  merge,
  notApproved,
  notAsked,
  overlong,
  parseExport,
  parseInput,
  planLines,
  refusal,
  taskDate,
} from '../hooks/plan'
import type { Task } from '../hooks/plan'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'

const task = (over: Partial<Task> = {}): Task => ({
  uuid: UUID,
  status: 'pending',
  description: 'feat: Old words',
  entry: '20260101T000000Z',
  annotations: [{ entry: '20260101T000001Z', description: 'first note' }],
  tags: ['zeta', 'alpha'],
  ...over,
})

const EXPECTED = { description: 'feat: Old words', notes: ['first note'] }

describe('isUuid', () => {
  test('takes a uuid, any case', () => {
    expect(isUuid(UUID)).toBe(true)
    expect(isUuid(UUID.toUpperCase())).toBe(true)
  })
  test('refuses the rest', () => {
    for (const value of ['', 'abc', `${UUID}x`, undefined, 7]) expect(isUuid(value)).toBe(false)
  })
})

describe('isSandboxed', () => {
  test('needs both flags true', () => {
    expect(isSandboxed({ sandbox: { enabled: true, failIfUnavailable: true } })).toBe(true)
    expect(isSandboxed({ sandbox: { enabled: true } })).toBe(false)
    expect(isSandboxed({ sandbox: { enabled: 'true', failIfUnavailable: true } })).toBe(false)
    expect(isSandboxed({})).toBe(false)
  })
})

describe('parseInput', () => {
  test('takes a well-formed input', () => {
    const input = { expected: EXPECTED, description: 'feat: New', notes: ['a', 'b'] }
    expect(parseInput(input)).toEqual(input)
  })
  test('says what is wrong', () => {
    expect(parseInput({ description: 'x', notes: [] })).toBe('expected must be an object')
    expect(parseInput({ expected: EXPECTED, description: 'x', notes: [1] })).toBe(
      'input.notes must be an array of strings',
    )
    expect(parseInput({ expected: EXPECTED, description: ' ', notes: [] })).toBe(
      'description must not be empty',
    )
    expect(parseInput({ expected: EXPECTED, description: 'a\nb', notes: [] })).toBe(
      'description must be one line',
    )
    expect(parseInput({ expected: EXPECTED, description: 'x', notes: ['a\nb'] })).toBe(
      'each note must be one line',
    )
  })
})

describe('parseExport', () => {
  test('answers the one task, or undefined', () => {
    expect(parseExport(JSON.stringify([task()]))?.uuid).toBe(UUID)
    expect(parseExport('[]')).toBeUndefined()
  })
})

describe('refusal', () => {
  test('none while the task matches', () => {
    expect(refusal(task(), EXPECTED)).toBeUndefined()
  })
  test('a missing or finished task', () => {
    expect(refusal(undefined, EXPECTED)).toBe('the task no longer exists')
    expect(refusal(task({ status: 'completed' }), EXPECTED)).toBe('the task is completed, not pending')
  })
  test('says what changed', () => {
    const why = refusal(task({ description: 'feat: Edited' }), EXPECTED)
    expect(why).toContain('its description is now "feat: Edited", not "feat: Old words"')
    expect(refusal(task({ annotations: [] }), EXPECTED)).toContain(
      'its notes are now [], not ["first note"]',
    )
  })
})

describe('merge', () => {
  test('sets description and notes, adds planned, keeps the rest', () => {
    const now = Date.UTC(2026, 9, 6, 1, 2, 3, 456)
    const merged = merge(task(), { expected: EXPECTED, description: 'feat: New', notes: ['a', 'b'] }, now)
    expect(taskDate(now)).toBe('20261006T010203Z')
    expect(merged).toEqual({
      ...task(),
      description: 'feat: New',
      annotations: [
        { entry: '20261006T010203Z', description: 'a' },
        { entry: '20261006T010204Z', description: 'b' },
      ],
      tags: ['alpha', 'planned', 'zeta'],
    })
  })
  test('does not repeat planned', () => {
    const merged = merge(task({ tags: ['planned'] }), { expected: EXPECTED, description: 'x', notes: [] }, 0)
    expect(merged.tags).toEqual(['planned'])
    expect(merged.annotations).toEqual([])
  })
})

describe('planLines', () => {
  test('one line for the description, one per note, in order', () => {
    expect(planLines({ description: 'feat: New', notes: ['a', 'b'] })).toEqual([
      'Description: feat: New',
      'Note 1: a',
      'Note 2: b',
    ])
  })
  test('says when the task will keep no notes', () => {
    expect(planLines({ description: 'feat: New', notes: [] })).toEqual([
      'Description: feat: New',
      'Notes: none (the task keeps no notes)',
    ])
  })
})

describe('overlong', () => {
  test('none while every line fits', () => {
    expect(overlong(planLines({ description: 'd', notes: ['x'.repeat(MAX_LINE - 'Note 1: '.length)] }))).toBeUndefined()
  })
  test('names the first line past the most the terminal draws', () => {
    const lines = planLines({ description: 'd', notes: ['ok', 'x'.repeat(MAX_LINE - 'Note 2: '.length + 1)] })
    expect(overlong(lines)).toBe(
      'note 2 would show as a line of 2001 characters, over the 2000 the terminal draws. ' +
        'Shorten it or split it into notes, then call again.',
    )
    expect(overlong([`Description: ${'y'.repeat(MAX_LINE)}`])).toStartWith('the description would show')
  })
})

describe('notApproved', () => {
  test('carries the choice, or the typed words', () => {
    expect(notApproved(CHANGE)).toContain(`the person chose "${CHANGE}"`)
    expect(notApproved('fewer notes')).toContain('the person answered "fewer notes" instead of approving')
    expect(notApproved(`${WRITE}, ${CHANGE}`)).toContain('instead of approving')
  })
})

describe('notAsked', () => {
  test('says nothing was written and why', () => {
    expect(notAsked('no one to ask')).toBe(
      'Nothing written: the person could not be asked to approve the plan (no one to ask). ' +
        'write_task_plan writes only after they choose "Write it to the task", ' +
        'which needs someone at an interactive session.',
    )
  })
})
```

- [ ] **Step 2: Run them to see them fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL — `planLines`, `overlong`, `notApproved`, `notAsked`, `CHANGE`, `WRITE` and `MAX_LINE` are not exported from `../hooks/plan`, and `'each note must be one line'` is not returned.

- [ ] **Step 3: Write `plan.ts`**

`claude/refine-mod/hooks/plan.ts`:

```ts
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
  if (plan.notes.some(note => /[\r\n]/.test(note))) return 'each note must be one line'
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
  `Nothing written: the person could not be asked to approve the plan (${reason}). ` +
  `write_task_plan writes only after they choose "${WRITE}", which needs someone at an interactive session.`
```

- [ ] **Step 4: Run the unit tests**

Run: `claude plugin test claude/refine-mod`
Expected: every `plan.test.ts` test passes. The old `refine.test.ts` tests still pass too: `refine.ts` has not changed yet.

- [ ] **Step 5: Write the failing engine tests**

`claude/refine-mod/tests/refine.test.ts`:

```ts
import { expect, mock, test } from 'claude-code/testing'
import type { On, ProcessRunInit } from 'claude-code'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'
const TOOL = 'mcp__niri-tasks-refine__write_task_plan'
const SANDBOX = { sandbox: { enabled: true, failIfUnavailable: true } }
const START = { cwd: '/tmp', surface: null, isInteractive: false } as const
const WRITE = 'Write it to the task'

const TASK = {
  uuid: UUID,
  status: 'pending',
  description: 'feat: Old words',
  entry: '20260101T000000Z',
  annotations: [{ entry: '20260101T000001Z', description: 'first note' }],
  tags: ['zeta'],
}

const CALL = {
  tool: TOOL as typeof TOOL,
  expected: { description: 'feat: Old words', notes: ['first note'] },
  description: 'feat: New words',
  notes: ['step one', 'step two'],
}

type Run = { argv: readonly string[]; init?: ProcessRunInit }

// How the person answers the question: the label chosen or the words typed
// under Other, or `{ deny }` for a dialog no one answered.
type Answer = string | { deny: string }

const ran = (stdout = '', exitCode = 0) => ({
  value: { exitCode, stdout, stderr: '', isStdoutTruncated: false, isStderrTruncated: false },
})

// The engine beneath the plugin, which a test must stand in for: the
// session's start, the tool registry, the clock, the settings, the
// transcript's log lines and the AskUserQuestion dialog. `seen` records the
// log lines and the questions in the order they reached the engine.
const engine = (on: On, settings: Record<string, unknown>, answer: Answer = WRITE) => {
  const registered: string[] = []
  const seen: string[] = []
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('tool.register', ($, e) => {
    const tool = `mcp__niri-tasks-refine__${e.name}`
    registered.push(tool)
    return { value: { tool } }
  })
  on('settings.read', () => ({ value: settings }))
  on('ui.log', ($, e) => {
    seen.push(`log: ${e.text}`)
    return { value: undefined }
  })
  // `$.ui.ask` is a tool.call of AskUserQuestion; the dialog's answer is
  // `answers`, keyed by the question.
  on('tool.call', { tool: 'AskUserQuestion' }, ($, e) => {
    const [question] = e.questions
    seen.push(`ask: ${question?.question} [${question?.options.map(o => o.label).join(' | ')}] (${question?.header})`)
    if (typeof answer !== 'string') return answer
    return { result: { questions: e.questions, answers: { [question?.question ?? '']: answer } } }
  })
  mock.clock(on, { now: Date.UTC(2026, 9, 6, 1, 2, 3) })
  return { registered, seen }
}

// The engine, and a `task` that exports `exports[i]` at the i-th export (the
// last one after that) and records every command it was asked to run.
const world = (
  on: On,
  settings: Record<string, unknown>,
  answer: Answer = WRITE,
  exports: readonly object[] = [TASK],
) => {
  const { registered, seen } = engine(on, settings, answer)
  const runs: Run[] = []
  let exported = 0
  on('process.run', ($, e) => {
    runs.push(e)
    if (!e.argv.includes('export')) return ran('')
    const task = exports[Math.min(exported++, exports.length - 1)]
    return ran(JSON.stringify([task]))
  })
  return { registered, runs, seen }
}

const verbs = (runs: readonly Run[]) => runs.map(run => run.argv.at(-1))

test('armed, approved: shows the plan, asks, re-reads, imports', { options: { uuid: UUID } }, async ($, on) => {
  const { registered, runs, seen } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([TOOL])

  const answer = await $.tool.call(CALL)

  // A test's `$.tool.call` gets the plugin's answer as given: `result`, and
  // no `text`, which only core sets.
  expect(answer.deny).toBeUndefined()
  expect(answer.result).toContain(`Wrote the plan to task ${UUID}: 2 note(s)`)
  expect(seen).toEqual([
    'log: Description: feat: New words',
    'log: Note 1: step one',
    'log: Note 2: step two',
    `ask: Write this to the task? [${WRITE} | Change something] (Task plan)`,
  ])
  expect(runs.map(run => run.argv)).toEqual([
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    ['task', 'rc.hooks=off', 'rc.verbose=nothing', 'import'],
  ])
  expect(JSON.parse(runs[2]?.init?.stdin ?? 'null')).toEqual([
    {
      ...TASK,
      description: 'feat: New words',
      annotations: [
        { entry: '20261006T010203Z', description: 'step one' },
        { entry: '20261006T010204Z', description: 'step two' },
      ],
      tags: ['planned', 'zeta'],
    },
  ])
})

test('"Change something": writes nothing, the model gets the choice', { options: { uuid: UUID } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, 'Change something')
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toBe(
    'Nothing written: the person chose "Change something". Ask them what to change, ' +
      'revise the plan, and call write_task_plan again.',
  )
  expect(seen.at(-1)).toContain('ask: Write this to the task?')
  expect(verbs(runs)).toEqual(['export'])
})

test('typed Other: writes nothing, the model gets the words', { options: { uuid: UUID } }, async ($, on) => {
  const { runs } = world(on, SANDBOX, 'Split step two into two notes')
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toBe(
    'Nothing written: the person answered "Split step two into two notes" instead of approving. ' +
      'Revise the plan with that, and call write_task_plan again.',
  )
  expect(verbs(runs)).toEqual(['export'])
})

test('no one to ask: writes nothing and says why', { options: { uuid: UUID } }, async ($, on) => {
  const { runs } = world(on, SANDBOX, { deny: 'no one is at the keyboard' })
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toStartWith('Nothing written: the person could not be asked to approve the plan (')
  expect(answer.deny).toContain('no one is at the keyboard')
  expect(answer.deny).toEndWith(
    'write_task_plan writes only after they choose "Write it to the task", ' +
      'which needs someone at an interactive session.',
  )
  expect(verbs(runs)).toEqual(['export'])
})

test('a line past 2,000 characters: refused before asking', { options: { uuid: UUID } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX)
  await $.session.start(START)
  const answer = await $.tool.call({ ...CALL, notes: ['short', 'x'.repeat(2000)] })
  expect(answer.deny).toBe(
    'Nothing written: note 2 would show as a line of 2008 characters, over the 2000 the terminal draws. ' +
      'Shorten it or split it into notes, then call again.',
  )
  expect(seen).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
})

test('the task changes while the person answers: writes nothing', { options: { uuid: UUID } }, async ($, on) => {
  const edited = { ...TASK, annotations: [...TASK.annotations, { entry: '20260101T000002Z', description: 'added' }] }
  const { runs, seen } = world(on, SANDBOX, WRITE, [TASK, edited])
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(seen.at(-1)).toContain('ask: ')
  expect(answer.deny).toContain('Nothing written: the task changed since it was read')
  expect(answer.deny).toContain('its notes are now ["first note","added"], not ["first note"]')
  expect(verbs(runs)).toEqual(['export', 'export'])
})

test('stale expected: refuses before asking and imports nothing', { options: { uuid: UUID } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX)
  await $.session.start(START)

  const answer = await $.tool.call({
    ...CALL,
    expected: { description: 'feat: What I read', notes: ['first note'] },
    notes: [],
  })

  expect(answer.deny).toContain('Nothing written: the task changed since it was read')
  expect(answer.deny).toContain('its description is now "feat: Old words", not "feat: What I read"')
  expect(seen).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
})

test('a failed import is reported', { options: { uuid: UUID } }, async ($, on) => {
  engine(on, SANDBOX)
  on('process.run', ($, e) =>
    e.argv.includes('export')
      ? ran(JSON.stringify([TASK]))
      : { value: { ...ran('', 2).value, stderr: 'Not a valid JSON value.' } },
  )
  await $.session.start(START)
  const answer = await $.tool.call({ ...CALL, notes: [] })
  expect(answer.deny).toBe('task import failed (2): Not a valid JSON value.')
})

test('no uuid: no tool', async ($, on) => {
  const { registered } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('not a uuid: no tool', { options: { uuid: 'nope' } }, async ($, on) => {
  const { registered } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('sandbox off: no tool', { options: { uuid: UUID } }, async ($, on) => {
  const { registered } = world(on, { sandbox: { enabled: true, failIfUnavailable: false } })
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('a hook that throws is denied by its .catch', { options: { uuid: UUID } }, async ($, on) => {
  engine(on, SANDBOX)
  on('process.run', () => ({ deny: 'no task here' }))
  await $.session.start(START)
  const answer = await $.tool.call({ ...CALL, expected: { description: 'feat: Old words', notes: [] }, notes: [] })
  expect(answer.deny).toBe('niri-tasks-refine: write_task_plan failed before writing.')
})
```

- [ ] **Step 6: Run them to see them fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL in `refine.test.ts` — no log lines or question are seen, and the approved test sees one export where it expects two.

- [ ] **Step 7: Write `refine.ts`**

`claude/refine-mod/hooks/refine.ts`:

```ts
import type { EngineInterface, Register } from 'claude-code'
import {
  CHANGE,
  HEADER,
  QUESTION,
  WRITE,
  isSandboxed,
  isUuid,
  merge,
  notApproved,
  notAsked,
  overlong,
  parseExport,
  parseInput,
  planLines,
  refusal,
} from './plan'
import type { Plan, Task } from './plan'

// The tool's listed name: mcp__<plugin>__<name>, hyphens kept.
export const TOOL = 'mcp__niri-tasks-refine__write_task_plan'

const PLAN = {
  type: 'object',
  properties: {
    description: { type: 'string' },
    notes: { type: 'array', items: { type: 'string' } },
  },
  required: ['description', 'notes'],
}

const SPEC = {
  name: 'write_task_plan',
  description:
    "Rewrites this session's Taskwarrior task: its description and notes, and tags it planned. " +
    '`expected` is the description and notes as you read them; the write is refused if the task changed since.',
  inputSchema: {
    type: 'object',
    properties: {
      expected: PLAN,
      description: { type: 'string', description: 'The new one-line description.' },
      notes: { type: 'array', items: { type: 'string' }, description: 'The new notes, in order.' },
    },
    required: ['expected', 'description', 'notes'],
  },
}

// The task this session may write, or undefined when the tool must not be
// offered: no valid uuid, or the sandbox is not on and enforced. Asked again
// at every call, so a reload or a settings change between the two counts.
const armedUuid = async ($: EngineInterface, uuid: unknown): Promise<string | undefined> =>
  isUuid(uuid) && isSandboxed(await $.settings.read()) ? uuid : undefined

// Hooks off: the session can write ~/.task/hooks, and this command runs
// outside its sandbox.
const TASK = ['task', 'rc.hooks=off']

// The task as it is now, if it still matches what the skill read; else the
// refusal the call answers with.
const current = async (
  $: EngineInterface,
  uuid: string,
  expected: Plan,
): Promise<{ task: Task } | { deny: string }> => {
  const exported = await $.process.run([...TASK, 'rc.json.array=on', uuid, 'export'])
  if (exported.exitCode !== 0) {
    return { deny: `task export failed (${exported.exitCode}): ${exported.stderr.trim()}` }
  }
  const task = parseExport(exported.stdout)
  const why = refusal(task, expected)
  if (why !== undefined || task === undefined) return { deny: `Nothing written: ${why}` }
  return { task }
}

export const register: Register = (on, options) => {
  on('session.start', async ($, e, next) => {
    if ((await armedUuid($, options.uuid)) !== undefined) await $.tool.register(SPEC)
    return next(e)
  })

  // The plugin serves its own tool: no hook beneath answers it, so this one
  // never calls next.
  on('tool.call', { tool: TOOL }, async ($, e) => {
    const uuid = await armedUuid($, options.uuid)
    if (uuid === undefined) return { deny: 'write_task_plan is not armed in this session.' }

    const input = parseInput(e)
    if (typeof input === 'string') return { deny: `write_task_plan: ${input}` }

    // Checked before the person is asked, so they are never asked about a
    // plan that could not be written.
    const before = await current($, uuid, input.expected)
    if ('deny' in before) return before

    // The plan as the tool will write it, from its own arguments, not from
    // what the model printed; refused whole before a line could be cut.
    const lines = planLines(input)
    const tooLong = overlong(lines)
    if (tooLong !== undefined) return { deny: `Nothing written: ${tooLong}` }
    for (const line of lines) $.ui.log(line)

    let answer: string
    try {
      answer = await $.ui.ask(QUESTION, { options: [WRITE, CHANGE], header: HEADER })
    } catch (error) {
      return { deny: notAsked(error instanceof Error ? error.message : String(error)) }
    }
    if (answer !== WRITE) return { deny: notApproved(answer) }

    // The person may have taken minutes: read the task again.
    const now = await current($, uuid, input.expected)
    if ('deny' in now) return now

    const planned = merge(now.task, input, await $.clock.now())
    const imported = await $.process.run([...TASK, 'rc.verbose=nothing', 'import'], {
      stdin: JSON.stringify([planned]),
    })
    if (imported.exitCode !== 0) {
      return { deny: `task import failed (${imported.exitCode}): ${imported.stderr.trim()}` }
    }
    return {
      result: `Wrote the plan to task ${uuid}: ${input.notes.length} note(s), tagged planned.`,
    }
  }).catch($ => ({ deny: `${$.plugin.name}: write_task_plan failed before writing.` }))
}
```

- [ ] **Step 8: Run validate and the tests**

```bash
claude plugin validate claude/refine-mod
claude plugin test claude/refine-mod
```

Expected: `✔ Validation passed`, with `$.ui.ask` and `$.ui.log` among the calls listed; `29 pass`, `0 fail`.

- [ ] **Step 9: Live check over stream-json, against a throwaway store**

`claude -p --input-format stream-json --permission-prompt-tool stdio` hands the AskUserQuestion dialog to its host as a `can_use_tool` control request, so a script can answer it. Save this driver as `drive.py` in the scratch folder `$S` made below — never in the repo:

```python
# Drives `claude -p` over stream-json and answers can_use_tool requests:
# AskUserQuestion gets ANSWER; anything else is allowed unchanged.
import json, os, subprocess, sys
answer, prompt, log = sys.argv[1], sys.argv[2], sys.argv[3]
cmd = sys.argv[4:]
p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(log + '.err', 'w'), text=True)
out = open(log, 'w')
def send(m):
    p.stdin.write(json.dumps(m) + '\n'); p.stdin.flush()
send({'type': 'control_request', 'request_id': 'init-1', 'request': {'subtype': 'initialize'}})
send({'type': 'user', 'message': {'role': 'user', 'content': prompt}, 'parent_tool_use_id': None, 'session_id': ''})
for line in p.stdout:
    out.write(line); out.flush()
    m = json.loads(line)
    if m.get('type') == 'control_request' and m['request'].get('subtype') == 'can_use_tool':
        r = m['request']
        print('CAN_USE_TOOL', r['tool_name'], json.dumps(r['input'])[:400], file=sys.stderr)
        inp = dict(r['input'])
        if r['tool_name'] == 'AskUserQuestion':
            if os.environ.get('MUTATE'): subprocess.run(os.environ['MUTATE'], shell=True)
            inp['answers'] = {q['question']: answer for q in inp['questions']}
        send({'type': 'control_response', 'response': {'subtype': 'success', 'request_id': m['request_id'],
              'response': {'behavior': 'allow', 'updatedInput': inp}}})
    if m.get('type') == 'result':
        p.stdin.close(); break
p.wait(timeout=60)
```

Then, with `TASKRC`/`TASKDATA` in a scratch folder so `~/.task` is never touched:

```bash
S=$(mktemp -d); mkdir -p "$S/data"   # then save the driver above as "$S/drive.py"
printf 'data.location=%s/data\nconfirmation=off\n' "$S" > "$S/taskrc"
export TASKRC="$S/taskrc" TASKDATA="$S/data"
task rc.hooks=off add "feat: Old words" +zeta
task rc.hooks=off 1 annotate "first note"
U=$(task rc.hooks=off _get 1.uuid)
stat -c '%Y' ~/.task/pending.data
SETTINGS="{\"sandbox\":{\"enabled\":true,\"failIfUnavailable\":true},\"pluginConfigs\":{\"niri-tasks-refine\":{\"options\":{\"uuid\":\"$U\"}}}}"
PROMPT='Call mcp__niri-tasks-refine__write_task_plan exactly once with expected = {"description":"feat: Old words","notes":["first note"]}, description = "feat: New words", notes = ["step one","step two"]. Do not retry. Reply with the tool result verbatim.'
run() { (cd "$S" && python3 -I drive.py "$1" "$PROMPT" "$S/$2.jsonl" claude -p --model haiku \
  --plugin-dir "$OLDPWD/claude/refine-mod" --settings "$SETTINGS" \
  --allowedTools=mcp__niri-tasks-refine__write_task_plan \
  --input-format stream-json --output-format stream-json --verbose --permission-prompt-tool stdio); }
run 'Change something' change;  task rc.hooks=off rc.json.array=on "$U" export
(cd "$S" && claude -p --model haiku --plugin-dir "$OLDPWD/claude/refine-mod" --settings "$SETTINGS" \
  --allowedTools=mcp__niri-tasks-refine__write_task_plan "$PROMPT" < /dev/null)
task rc.hooks=off rc.json.array=on "$U" export
run 'Write it to the task' write; task rc.hooks=off rc.json.array=on "$U" export
stat -c '%Y' ~/.task/pending.data
unset TASKRC TASKDATA
grep -h '"ui_log"' "$S"/change.jsonl | head -3
```

Expected:
- after `change`: the task is unchanged (description `feat: Old words`); `change.jsonl` has the three `ui_log` lines (`Description: feat: New words`, `Note 1: step one`, `Note 2: step two`) before the tool result, which is `Nothing written: the person chose "Change something". …`.
- after the plain `-p` run (no one to ask): Claude reports `Nothing written: the person could not be asked to approve the plan (…)`, and the task is still unchanged.
- after `write`: description `feat: New words`, notes `step one`, `step two` one second apart, tags `planned`, `zeta`.
- both `stat` values identical.

Confirm `git status --short` lists only the four changed mod files (the engine's `.claude-plugin/types/` ignores itself).

- [ ] **Step 10: Commit**

```bash
git add claude/refine-mod
git commit -m "$(cat <<'EOF'
feat(refine): ask the person inside the write tool before writing

write_task_plan now shows the plan from its own arguments as dim
transcript lines, asks "Write this to the task?" in the engine's
dialog, and writes only on "Write it to the task", re-reading the task
first. Any other answer, or no one to ask, writes nothing and hands
the person's choice or words back to the model.

Co-Authored-By: Claude <your model> <noreply@anthropic.com>
EOF
)"
```

The trailer names the model that wrote the commit (for example `Claude Sonnet 5.5`).

---

### Task 2: The skill stops asking and reacts to the tool

**Files:**
- Modify: `.claude/skills/refine-task/SKILL.md` (opening paragraph, §4 "Propose", §5 "Write")
- Modify: `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md` (the `## Adopted` list)

**Interfaces:**
- Consumes: Task 1's tool answers (the prefixes listed in Task 1's Interfaces).
- Produces: a skill with no AskUserQuestion step.

- [ ] **Step 1: Check what refers to the old approval**

Run: `grep -n "AskUserQuestion\|step 4\|says so" .claude/skills/refine-task/SKILL.md`
Expected: the opening paragraph ("only after the user says so in step 4"), §4's "then ask with AskUserQuestion", and §4/§5's "Change something" / "go back to step 4" lines. Those are what the next steps change.

- [ ] **Step 2: Rewrite the opening sentence**

In the paragraph under "**You refine the task; you never do it.**", replace

```markdown
Your one write is the Taskwarrior update in
step 5, and only after the user says so in step 4.
```

(the line break may sit elsewhere; match the words) with

```markdown
Your one write is the Taskwarrior update in
step 5, and the write tool asks the user itself before it writes.
```

Rewrap that paragraph to the file's ~80 columns.

- [ ] **Step 3: Rewrite §4's opening and close**

In `## 4. Propose`, replace the opening sentence

```markdown
Show the proposal in your reply, then ask with AskUserQuestion — one question,
"Write this to the task?", with two options: **Write it to the task** and
**Change something**. The proposal contains:
```

with

```markdown
Show the proposal in your reply. Do not ask the user to approve it yourself —
no AskUserQuestion: the write tool in step 5 shows them exactly what it will
write and asks them. The proposal contains:
```

and replace §4's last paragraph

```markdown
On **Change something** (or any other answer), take their feedback, revise,
print the block again, and ask again. Only **Write it to the task** leads to
step 5.
```

with

```markdown
Then go straight to step 5.
```

Leave the proposal's three parts and the **Will be written to the task** block as they are.

- [ ] **Step 4: Rewrite §5**

Replace §5, from `## 5. Write` up to (not including) `## 6. Verify and report`, with:

````markdown
## 5. Write

Write with the `mcp__niri-tasks-refine__write_task_plan` tool — the session's
one way to change the task. It is deferred: select it with ToolSearch
(`select:mcp__niri-tasks-refine__write_task_plan`) if it is not loaded. Call
it with:

- `expected`: the `description` and the annotations' `description`s, in
  order, exactly as you read them in step 1;
- `description` and `notes`: the **Will be written to the task** block you
  just printed, exactly as printed — no rewording, no reordering, nothing
  added. Each note is one line.

The tool shows the user the description and notes it was given, asks
"Write this to the task?", and writes only if they choose **Write it to the
task**. It writes the description, replaces the notes and adds `+planned` in
one import, and touches nothing else. Its answer says what happened:

- **Wrote the plan …** — go to step 6.
- **Nothing written: the person chose "Change something"** — ask them what to
  change, revise, print the block again (step 4) and call the tool again.
- **Nothing written: the person answered "…"** — those are their words about
  the plan: revise with them, print the block again and call again.
- **Nothing written: the task changed since it was read** — someone else
  edited it meanwhile: show the user what changed and ask before going on.
- **Nothing written: the person could not be asked** — stop, and tell the
  user the plan was not written because the tool could not ask them.
- Any other **Nothing written** — fix what it names (a line too long, a note
  on two lines) and call again.

If the tool is missing, say so and stop — do not write the task any other
way.
````

- [ ] **Step 5: Check the skill reads through**

Run: `grep -n "AskUserQuestion" .claude/skills/refine-task/SKILL.md`
Expected: one hit only, in §4's "no AskUserQuestion". Read §4 into §5 once end to end: the order is propose → print the block → call the tool → react to its answer.

- [ ] **Step 6: Record B as built in the ADR**

In `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, under `## Adopted`, append ` — built` to the B line, so it reads:

```markdown
- B — `feat: Approve a refine's write inside the tool` (task `dc32d340`) — built
```

If the A line has no ` — built` yet, add it there too (A shipped in `4f9ac17`).

- [ ] **Step 7: Commit**

```bash
git add .claude/skills/refine-task/SKILL.md docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md
git commit -m "$(cat <<'EOF'
feat(refine): let the write tool ask instead of the skill

The refine-task skill no longer asks with AskUserQuestion: it shows
the proposal and calls write_task_plan, which asks the user itself,
and reacts to the tool's answer to revise, stop or report.

Co-Authored-By: Claude <your model> <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 8: End to end through Refine (after landing — the user's screen)**

After the branch lands on main (with `finish-worktree`), `claude/refine-mod` is already linked from `~/.local/share/niri-tasks/refine-mod` to the main checkout, so no reinstall is needed. The person running the check:

1. `niritasks task add 'chore: Try the refine approval'`; Refine it.
2. When the dialog appears, check the dim lines above it show the description and each note; answer **Change something** once, give a change, and see the session revise and ask again.
3. Answer **Write it to the task**; `task rc.json.array=on <uuid> export` shows the plan and `planned`.
4. `niritasks task status <uuid> deleted --yes`.
