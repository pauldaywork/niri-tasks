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
