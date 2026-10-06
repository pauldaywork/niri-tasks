import { expect, mock, test } from 'claude-code/testing'
import type { On, ProcessRunInit } from 'claude-code'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'
const TOOL = 'mcp__niri-tasks-refine__write_task_plan'
const SANDBOX = { sandbox: { enabled: true, failIfUnavailable: true } }
const START = { cwd: '/tmp', surface: null, isInteractive: false } as const

const TASK = {
  uuid: UUID,
  status: 'pending',
  description: 'feat: Old words',
  entry: '20260101T000000Z',
  annotations: [{ entry: '20260101T000001Z', description: 'first note' }],
  tags: ['zeta'],
}

type Run = { argv: readonly string[]; init?: ProcessRunInit }

const ran = (stdout = '', exitCode = 0) => ({
  value: { exitCode, stdout, stderr: '', isStdoutTruncated: false, isStderrTruncated: false },
})

// The engine beneath the plugin, which a test must stand in for: the
// session's start, the tool registry, the clock and the settings.
const engine = (on: On, settings: Record<string, unknown>) => {
  const registered: string[] = []
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('tool.register', ($, e) => {
    const tool = `mcp__niri-tasks-refine__${e.name}`
    registered.push(tool)
    return { value: { tool } }
  })
  on('settings.read', () => ({ value: settings }))
  mock.clock(on, { now: Date.UTC(2026, 9, 6, 1, 2, 3) })
  return registered
}

// The engine, and a `task` that exports TASK and records every command it
// was asked to run.
const world = (on: On, settings: Record<string, unknown>) => {
  const registered = engine(on, settings)
  const runs: Run[] = []
  on('process.run', ($, e) => {
    runs.push(e)
    return ran(e.argv.includes('export') ? JSON.stringify([TASK]) : '')
  })
  return { registered, runs }
}

test('armed: registers the tool, exports then imports the plan', { options: { uuid: UUID } }, async ($, on) => {
  const { registered, runs } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([TOOL])

  const answer = await $.tool.call({
    tool: TOOL,
    expected: { description: 'feat: Old words', notes: ['first note'] },
    description: 'feat: New words',
    notes: ['step one', 'step two'],
  })

  // A test's `$.tool.call` gets the plugin's answer as given: `result`, and
  // no `text`, which only core sets.
  expect(answer.deny).toBeUndefined()
  expect(answer.result).toContain(`Wrote the plan to task ${UUID}: 2 note(s)`)
  expect(runs.map(run => run.argv)).toEqual([
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    ['task', 'rc.hooks=off', 'rc.verbose=nothing', 'import'],
  ])
  expect(JSON.parse(runs[1]?.init?.stdin ?? 'null')).toEqual([
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

test('stale expected: refuses and imports nothing', { options: { uuid: UUID } }, async ($, on) => {
  const { runs } = world(on, SANDBOX)
  await $.session.start(START)

  const answer = await $.tool.call({
    tool: TOOL,
    expected: { description: 'feat: What I read', notes: ['first note'] },
    description: 'feat: New words',
    notes: [],
  })

  expect(answer.deny).toContain('Nothing written: the task changed since it was read')
  expect(answer.deny).toContain('its description is now "feat: Old words", not "feat: What I read"')
  expect(runs.map(run => run.argv.at(-1))).toEqual(['export'])
})

test('a failed import is reported', { options: { uuid: UUID } }, async ($, on) => {
  engine(on, SANDBOX)
  on('process.run', ($, e) =>
    e.argv.includes('export')
      ? ran(JSON.stringify([TASK]))
      : { value: { ...ran('', 2).value, stderr: 'Not a valid JSON value.' } },
  )
  await $.session.start(START)
  const answer = await $.tool.call({
    tool: TOOL,
    expected: { description: 'feat: Old words', notes: ['first note'] },
    description: 'feat: New words',
    notes: [],
  })
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
  const answer = await $.tool.call({
    tool: TOOL,
    expected: { description: 'feat: Old words', notes: [] },
    description: 'feat: New words',
    notes: [],
  })
  expect(answer.deny).toBe('niri-tasks-refine: write_task_plan failed before writing.')
})
