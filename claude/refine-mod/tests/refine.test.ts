import { expect, mock, test } from 'claude-code/testing'
import type { On, ProcessRunInit } from 'claude-code'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'
const TOOL = 'mcp__niri-tasks-refine__write_task_plan'
const SANDBOX = { sandbox: { enabled: true, failIfUnavailable: true } }
const START = { cwd: '/tmp', surface: null, isInteractive: false } as const
const WRITE = 'Write it to the task'
const REPORT = 'Show me a report first'
const REPORT_TOOL = 'mcp__niri-tasks-refine__show_task_report'
const REPORTS = { uuid: UUID, reports: '/r' }
const REPORT_PATH = '/r/refine-0b8f6a52-20261006-010203.html'
// The smallest report show_task_report takes.
const MINIMAL = {
  title: 'feat: New words',
  summary: 'The task gets new words.',
  changes: ['Words: the description changes.'],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
}

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
// under Other, or `{ deny }` for a dialog no one answered. A list answers
// one question each, in order, the last one after that.
type Answer = string | { deny: string }
type Answers = Answer | readonly Answer[]

type Write = { path: string; text: string }

const ran = (stdout = '', exitCode = 0) => ({
  value: { exitCode, stdout, stderr: '', isStdoutTruncated: false, isStderrTruncated: false },
})

// The engine beneath the plugin, which a test must stand in for: the
// session's start, the tool registry, the clock, the settings, the
// transcript's log lines and the AskUserQuestion dialog. `seen` records the
// log lines and the questions in the order they reached the engine.
// `writeFails`, when given, is what `$.fs.write` is refused with. With
// `fonts`, `$.fs.read` answers each font file with a short stand-in; without,
// it is refused, as when the mod's fonts are missing.
const engine = (
  on: On,
  settings: Record<string, unknown>,
  answer: Answers = WRITE,
  writeFails?: string,
  fonts = false,
) => {
  const registered: string[] = []
  const seen: string[] = []
  const writes: Write[] = []
  const reads: string[] = []
  let asked = 0
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
  on('fs.write', ($, e) => {
    if (writeFails !== undefined) return { deny: writeFails }
    writes.push({ path: e.path, text: e.text })
    return { value: undefined }
  })
  on('fs.read', ($, e) => {
    reads.push(e.path)
    if (!fonts) return { deny: 'no fonts here' }
    return { value: { base64: e.path.endsWith('space-grotesk-latin-wght-normal.woff2') ? 'RElTUA==' : 'Qk9EWQ==' } }
  })
  // `$.ui.ask` is a tool.call of AskUserQuestion; the dialog's answer is
  // `answers`, keyed by the question.
  on('tool.call', { tool: 'AskUserQuestion' }, ($, e) => {
    const [question] = e.questions
    seen.push(`ask: ${question?.question} [${question?.options.map(o => o.label).join(' | ')}] (${question?.header})`)
    const list: readonly Answer[] = Array.isArray(answer) ? answer : [answer as Answer]
    const given = list[Math.min(asked++, list.length - 1)] ?? WRITE
    if (typeof given !== 'string') return given
    return { result: { questions: e.questions, answers: { [question?.question ?? '']: given } } }
  })
  mock.clock(on, { now: Date.UTC(2026, 9, 6, 1, 2, 3) })
  return { registered, seen, writes, reads }
}

// The engine, and a `task` that exports `exports[i]` at the i-th export (the
// last one after that) and records every command it was asked to run.
const world = (
  on: On,
  settings: Record<string, unknown>,
  answer: Answers = WRITE,
  exports: readonly object[] = [TASK],
  fonts = false,
) => {
  const { registered, seen, writes, reads } = engine(on, settings, answer, undefined, fonts)
  const runs: Run[] = []
  let exported = 0
  on('process.run', ($, e) => {
    runs.push(e)
    if (!e.argv.includes('export')) return ran('')
    const task = exports[Math.min(exported++, exports.length - 1)]
    return ran(JSON.stringify([task]))
  })
  return { registered, runs, seen, writes, reads }
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

test('an empty answer: writes nothing', { options: { uuid: UUID } }, async ($, on) => {
  const { runs } = world(on, SANDBOX, '')
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toStartWith('Nothing written: the person answered "" instead of approving.')
  expect(verbs(runs)).toEqual(['export'])
})

test('no one to ask: writes nothing and says why', { options: { uuid: UUID } }, async ($, on) => {
  const { runs } = world(on, SANDBOX, { deny: 'no one is at the keyboard' })
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toStartWith('Nothing written: the person could not be asked')
  expect(answer.deny).toContain('no one is at the keyboard')
  expect(answer.deny).toEndWith(
    'write_task_plan writes only after they choose "Write it to the task".',
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

test('with a reports folder: both tools, and the report among the answers', { options: REPORTS }, async ($, on) => {
  const { registered, seen } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([TOOL, REPORT_TOOL])
  await $.tool.call(CALL)
  expect(seen.at(-1)).toBe(`ask: Write this to the task? [${WRITE} | ${REPORT} | Change something] (Task plan)`)
})

test('a relative reports folder: no report tool, two answers', { options: { uuid: UUID, reports: 'r' } }, async ($, on) => {
  const { registered, seen } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([TOOL])
  await $.tool.call(CALL)
  expect(seen.at(-1)).toBe(`ask: Write this to the task? [${WRITE} | Change something] (Task plan)`)
})

test('"Show me a report first": writes nothing, the model is told to build it', { options: REPORTS }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toBe(
    'Nothing written: the person chose "Show me a report first". Build the HTML report of this plan ' +
      'as the refine-task skill\'s "Report, when asked" says, show it with show_task_report, ' +
      'then call write_task_plan again with the same plan.',
  )
  expect(verbs(runs)).toEqual(['export'])
  expect(writes).toEqual([])
})

test('a report no one asked for is refused', { options: REPORTS }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX)
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe(
    'show_task_report: the person has not asked for a report. Call write_task_plan; ' +
      'they can choose "Show me a report first" there.',
  )
  expect(writes).toEqual([])
  expect(runs).toEqual([])
})

test('report, then approve: writes the page, opens it, then writes the task', { options: REPORTS }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, [REPORT, WRITE])
  await $.session.start(START)

  expect((await $.tool.call(CALL)).deny).toContain(`the person chose "${REPORT}"`)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH} and opened it in the browser. ` +
      'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
      'and the task will link the report in its notes.',
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH])
  expect(writes[0]?.text).toContain('<p class="lede">The task gets new words.</p>')
  expect(writes[0]?.text).toContain('Content-Security-Policy')
  // The plan the person was shown, from the tool's own arguments.
  const page = writes[0]?.text ?? ''
  const block = page.slice(page.indexOf('id="decision"'))
  expect(block).toContain('<p><strong>Description:</strong> feat: New words</p>')
  for (const note of CALL.notes) expect(block).toContain(`<li>${note}</li>`)
  expect(seen).toContain(`log: Report: ${REPORT_PATH}`)
  expect(runs.at(-1)?.argv).toEqual(['sh', '-c', 'xdg-open "$1" >/dev/null 2>&1 </dev/null &', 'sh', REPORT_PATH])

  // One report per ask: a second needs the person to choose it again.
  const again = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(again.deny).toStartWith('show_task_report: the person has not asked for a report.')

  const written = await $.tool.call(CALL)
  expect(written.result).toBe(`Wrote the plan to task ${UUID}: 3 note(s), tagged planned.`)
  expect(seen).toContain(`log: Note 3: Report: ${REPORT_PATH}`)
  expect(verbs(runs).at(-1)).toBe('import')
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.map((a: { description: string }) => a.description)).toEqual([
    'step one',
    'step two',
    `Report: ${REPORT_PATH}`,
  ])
})

test('a report made before the plan changed is linked as an earlier draft', { options: REPORTS }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, [REPORT, WRITE])
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  const written = await $.tool.call({ ...CALL, notes: ['step one', 'step three'] })
  expect(written.result).toContain('3 note(s)')
  expect(seen).toContain(`log: Note 3: Report (earlier draft): ${REPORT_PATH}`)
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.at(-1).description).toBe(`Report (earlier draft): ${REPORT_PATH}`)
})

test('a report choice is superseded by the next question', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, [REPORT, 'Change something'])
  await $.session.start(START)
  expect((await $.tool.call(CALL)).deny).toContain(`the person chose "${REPORT}"`)
  expect((await $.tool.call(CALL)).deny).toContain('the person chose "Change something"')
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(shown.deny).toStartWith('show_task_report: the person has not asked for a report.')
  expect(writes).toEqual([])
})

test('a report with a script is refused, and the person may still get one', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const refused = await $.tool.call({
    tool: REPORT_TOOL,
    ...MINIMAL,
    sections: [{ heading: 'What changes', points: ['The words.'], detail: '<script>alert(1)</script>' }],
  })
  expect(refused.deny).toBe(
    'show_task_report: sections[0].detail must not contain <script>: ' +
      'the page runs no script but Mermaid, loads nothing and sends nothing',
  )
  expect(writes).toEqual([])
  const fixed = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(fixed.result).toContain(`Wrote the report to ${REPORT_PATH}`)
})

test('a report over its limits is refused with every reason', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, changes: [], checks: [] })
  expect(answer.deny).toBe(
    'show_task_report: changes must have 1 to 3 items, not 0; checks must have 1 to 6 items, not 0',
  )
  expect(writes).toEqual([])
})

test('a report that cannot be written is denied by its .catch', { options: REPORTS }, async ($, on) => {
  engine(on, SANDBOX, REPORT, 'disk full')
  on('process.run', ($, e) => (e.argv.includes('export') ? ran(JSON.stringify([TASK])) : ran('')))
  await $.session.start(START)
  await $.tool.call(CALL)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe('niri-tasks-refine: show_task_report failed.')
})

test('sandbox off: no report tool either', { options: REPORTS }, async ($, on) => {
  const { registered } = world(on, { sandbox: { enabled: true, failIfUnavailable: false } })
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('a report embeds the fonts the mod ships, read once a session', { options: REPORTS }, async ($, on) => {
  const { reads, writes } = world(on, SANDBOX, [REPORT, REPORT], [TASK], true)
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(writes).toHaveLength(2)
  for (const page of writes.map(w => w.text)) {
    expect(page).toContain('src: url(data:font/woff2;base64,RElTUA==) format("woff2")')
    expect(page).toContain('src: url(data:font/woff2;base64,Qk9EWQ==) format("woff2")')
  }
  expect(reads).toHaveLength(2)
  expect(reads[0]).toEndWith('/fonts/space-grotesk-latin-wght-normal.woff2')
  expect(reads[1]).toEndWith('/fonts/inter-latin-wght-normal.woff2')
})

test('a report is still written when its fonts cannot be read', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(shown.result).toContain(`Wrote the report to ${REPORT_PATH}`)
  expect(writes[0]?.text).not.toContain('@font-face')
})
