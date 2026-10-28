import { expect, mock, test } from 'claude-code/testing'
import type { On, ProcessRunInit } from 'claude-code'
import { BRANCH_MIN, SHA_A, SHA_B } from './branch-fixtures'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'
const TOOL = 'mcp__niri-tasks-refine__write_task_plan'
const SANDBOX = { sandbox: { enabled: true, failIfUnavailable: true } }
const START = { cwd: '/tmp', surface: null, isInteractive: false } as const
const WRITE = 'Write it to the task'
const REPORT = 'Show me a report first'
const REPORT_TOOL = 'mcp__niri-tasks-refine__show_task_report'
const REPORTS = { uuid: UUID, reports: '/r' }
const REPORT_PATH = '/r/refine-0b8f6a52-20261006-010203.html'
// The report's Markdown twin, written beside it.
const REPORT_MD = '/r/refine-0b8f6a52-20261006-010203.md'
// A Report session: the Go to list's Generate report, not a refine.
const REPORT_MODE = { uuid: UUID, reports: '/r', report: true }
// The smallest report show_task_report takes.
const MINIMAL = {
  title: 'feat: New words',
  summary: 'The task gets new words.',
  changes: ['Words: the description changes.'],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
}

// A needs_your_eye line one word over its limit of 30, which the tool takes
// and names; and one three over, which it refuses.
const OVER = Array.from({ length: 31 }, () => 'word').join(' ')
const TOO_FAR_OVER = Array.from({ length: 33 }, () => 'word').join(' ')
const OVERAGE = 'needs_your_eye[0] has 31 words; at most 30'

const TASK = {
  uuid: UUID,
  status: 'pending',
  description: 'feat: Old words',
  entry: '20260101T000000Z',
  annotations: [{ entry: '20260101T000001Z', description: 'first note' }],
  tags: ['zeta'],
}

// TASK once a refine has written it: the plan a Report reports on.
const PLANNED = { ...TASK, tags: ['planned', 'zeta'] }

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
// it is refused, as when the mod's fonts are missing. A path in `files` is
// read as its text there.
const engine = (
  on: On,
  settings: Record<string, unknown>,
  answer: Answers = WRITE,
  writeFails?: string,
  fonts = false,
  files: Record<string, string> = {},
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
    const file = files[e.path]
    if (file !== undefined) return { value: file }
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
  draw?: (argv: readonly string[]) => ReturnType<typeof ran>,
) => {
  const { registered, seen, writes, reads } = engine(on, settings, answer, undefined, fonts)
  const runs: Run[] = []
  let exported = 0
  on('process.run', ($, e) => {
    runs.push(e)
    // The renderer: finding its runtime, then running it in the fence.
    if (e.argv[0] === 'bwrap' || e.argv[2]?.includes('command -v')) return draw?.(e.argv) ?? ran('', 1)
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
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(writes[1]?.text).toStartWith('# feat: New words\n\nThe task gets new words.\n')
  expect(writes[0]?.text).toContain('<p class="lede">The task gets new words.</p>')
  expect(writes[0]?.text).toContain('Content-Security-Policy')
  // The plan the person was shown, from the tool's own arguments.
  const page = writes[0]?.text ?? ''
  const block = page.slice(page.indexOf('id="decision"'))
  expect(block).toContain('<p><strong>Description:</strong> feat: New words</p>')
  for (const note of CALL.notes) expect(block).toContain(`<li>${note}</li>`)
  expect(seen).toContain(`log: Report: ${REPORT_PATH}`)
  expect(runs.at(-1)?.argv).toEqual(['sh', '-c', '{ niritasks task open-report "$1" || xdg-open "$1"; } >/dev/null 2>&1 </dev/null &', 'sh', REPORT_PATH])

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

// TASK refined once before, in an earlier session that made a report.
const OLD_REPORT = '/r/refine-0b8f6a52-20260101-000000.html'
const REPORTED = {
  ...PLANNED,
  annotations: [
    { entry: '20260101T000001Z', description: 'first note' },
    { entry: '20260101T000002Z', description: `Report: ${OLD_REPORT}` },
  ],
}
const REREFINE = {
  ...CALL,
  expected: { description: 'feat: Old words', notes: ['first note', `Report: ${OLD_REPORT}`] },
}

test("a re-refine that changes the plan relabels an earlier session's report", { options: { uuid: UUID } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, WRITE, [REPORTED])
  await $.session.start(START)
  const written = await $.tool.call({ ...REREFINE, notes: ['step one', `Report: ${OLD_REPORT}`] })
  expect(written.deny).toBeUndefined()
  expect(seen).toContain(`log: Note 2: Report (earlier draft): ${OLD_REPORT}`)
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.map((a: { description: string }) => a.description)).toEqual([
    'step one',
    `Report (earlier draft): ${OLD_REPORT}`,
  ])
})

test("a re-refine that keeps the plan keeps an earlier session's report", { options: { uuid: UUID } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, WRITE, [REPORTED])
  await $.session.start(START)
  const written = await $.tool.call({ ...REREFINE, ...REREFINE.expected })
  expect(written.deny).toBeUndefined()
  expect(seen).toContain(`log: Note 2: Report: ${OLD_REPORT}`)
  const [imported] = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported.annotations.at(-1).description).toBe(`Report: ${OLD_REPORT}`)
})

// A session told its home folder, with the reports folder under it: the
// files are written and opened by their absolute paths, and named from ~/
// everywhere else.
const HOME_REPORTS = { uuid: UUID, reports: '/home/x/r', home: '/home/x' }
const HOME_PATH = '/home/x/r/refine-0b8f6a52-20261006-010203.html'
const HOME_MD = '/home/x/r/refine-0b8f6a52-20261006-010203.md'
const HOME_NOTE = '~/r/refine-0b8f6a52-20261006-010203.html'
const OPEN = (path: string) => [
  'sh',
  '-c',
  '{ niritasks task open-report "$1" || xdg-open "$1"; } >/dev/null 2>&1 </dev/null &',
  'sh',
  path,
]
const notesOf = (run: Run | undefined): string[] =>
  JSON.parse(run?.init?.stdin ?? 'null')[0].annotations.map((a: { description: string }) => a.description)

test('under home: the report is written and opened by its path, and named from ~/', { options: HOME_REPORTS }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, [REPORT, WRITE])
  await $.session.start(START)
  await $.tool.call(CALL)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(shown.result).toStartWith(`Wrote the report to ${HOME_NOTE} and opened it in the browser. `)
  expect(writes.map(w => w.path)).toEqual([HOME_PATH, HOME_MD])
  expect(runs.at(-1)?.argv).toEqual(OPEN(HOME_PATH))
  expect(seen).toContain(`log: Report: ${HOME_NOTE}`)
  await $.tool.call(CALL)
  expect(notesOf(runs.at(-1))).toEqual(['step one', 'step two', `Report: ${HOME_NOTE}`])
})

test('under home: a report note the model keeps is not doubled', { options: HOME_REPORTS }, async ($, on) => {
  const { runs } = world(on, SANDBOX, [REPORT, WRITE])
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  await $.tool.call({ ...CALL, notes: [...CALL.notes, `Report: ${HOME_NOTE}`] })
  expect(notesOf(runs.at(-1))).toEqual(['step one', 'step two', `Report: ${HOME_NOTE}`])
})

test('under home: a kept note of a plan since changed is relabelled, not doubled', { options: HOME_REPORTS }, async ($, on) => {
  const { runs } = world(on, SANDBOX, [REPORT, WRITE])
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  await $.tool.call({ ...CALL, notes: ['step one', 'step three', `Report: ${HOME_NOTE}`] })
  expect(notesOf(runs.at(-1))).toEqual(['step one', 'step three', `Report (earlier draft): ${HOME_NOTE}`])
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

test('a report a word over its limit is written, and the overage named', { options: REPORTS }, async ($, on) => {
  const { seen, writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, needs_your_eye: [OVER] })
  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH} and opened it in the browser. ` +
      'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
      `and the task will link the report in its notes. Taken, though over its word limits: ${OVERAGE}.`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(seen).toContain(`log: Over its word limit: ${OVERAGE}`)
})

test('a report three words over its limit is still refused', { options: REPORTS }, async ($, on) => {
  const { writes } = world(on, SANDBOX, REPORT)
  await $.session.start(START)
  await $.tool.call(CALL)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, needs_your_eye: [TOO_FAR_OVER] })
  expect(answer.deny).toBe('show_task_report: needs_your_eye[0] has 33 words; at most 30')
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
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD, REPORT_PATH, REPORT_MD])
  for (const page of writes.filter(w => w.path.endsWith('.html')).map(w => w.text)) {
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

const DRAWN = {
  ...MINIMAL,
  sections: [{ heading: 'What changes', points: ['The words.'], look_at: 'The arrow.', diagram: { mermaid: 'flowchart LR\n a --> b' } }],
}

test('diagrams are drawn in the fenced renderer, and the page then runs no script', { options: REPORTS }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, REPORT, [TASK], false, argv => {
    if (argv[0] !== 'bwrap') return argv.at(-1) === 'bun' ? ran('/opt/bun\n') : ran('', 1)
    return ran(JSON.stringify({ svgs: ['<svg viewBox="0 0 1 1"><rect/></svg>'], errors: [null] }))
  })
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...DRAWN })
  const draw = runs.find(r => r.argv[0] === 'bwrap')
  const bind = (inside: string) => draw?.argv[(draw?.argv.indexOf(inside) ?? 0) - 1]
  expect(bind('/run/r/runtime')).toBe('/opt/bun')
  expect(bind('/run/r/diagrams.mjs')).toEqual(expect.stringMatching(/\/render\/diagrams\.mjs$/))
  expect(draw?.argv.slice(-2)).toEqual(['/run/r/runtime', '/run/r/diagrams.mjs'])
  expect(JSON.parse(draw?.init?.stdin ?? 'null').diagrams[0]).toStartWith('flowchart LR\n a --> b\nclassDef new')
  expect(writes[0]?.text).toContain('<div class="drawn" style="--w: 2px"><svg viewBox="0 0 1 1"><rect/></svg></div>')
  expect(writes[0]?.text).not.toContain('<script')
  expect(seen.filter(line => line.includes('Diagrams'))).toEqual([])
})

test('without bun the renderer runs on node', { options: REPORTS }, async ($, on) => {
  const { runs } = world(on, SANDBOX, REPORT, [TASK], false, argv => {
    if (argv[0] !== 'bwrap') return argv.at(-1) === 'node' ? ran('/usr/bin/node\n') : ran('', 1)
    return ran('', 1)
  })
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...DRAWN })
  const draws = runs.filter(r => r.argv[0] === 'bwrap')
  expect(draws.length).toBe(1)
  expect(draws[0]?.argv).toContain('/usr/bin/node')
})

test('a renderer that fails is not retried on node; the browser draws', { options: REPORTS }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, REPORT, [TASK], false, argv =>
    argv[0] !== 'bwrap' ? ran(argv.at(-1) === 'bun' ? '/opt/bun\n' : '/usr/bin/node\n') : ran('', 1),
  )
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...DRAWN })
  expect(runs.filter(r => r.argv[0] === 'bwrap').length).toBe(1)
  expect(runs.filter(r => r.argv[2]?.includes('command -v')).map(r => r.argv.at(-1))).toEqual(['bun'])
  expect(writes[0]?.text).toContain('<pre class="mermaid">')
  expect(seen).toContain('log: Diagrams left to Mermaid in the browser: the diagram renderer failed (exit 1).')
})

test("a drawing the page will not take is left to the browser, and said so", { options: REPORTS }, async ($, on) => {
  const { seen, writes } = world(on, SANDBOX, REPORT, [TASK], false, argv => {
    if (argv[0] !== 'bwrap') return argv.at(-1) === 'bun' ? ran('/opt/bun\n') : ran('', 1)
    return ran(JSON.stringify({ svgs: ['<svg><!-- open</svg>'], errors: [null] }))
  })
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...DRAWN })
  expect(writes[0]?.text).toContain('<pre class="mermaid">')
  expect(seen).toContain("log: Diagrams left to Mermaid in the browser: the renderer's output was refused for 1 diagram(s).")
})

test('with neither bun nor node the browser draws', { options: REPORTS }, async ($, on) => {
  const tried: string[] = []
  const { runs, seen, writes } = world(on, SANDBOX, REPORT, [TASK], false, argv => {
    tried.push(argv.at(-1) ?? '')
    return ran('', 1)
  })
  await $.session.start(START)
  await $.tool.call(CALL)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...DRAWN })
  expect(shown.result).toContain('Wrote the report to')
  expect(tried).toEqual(['bun', 'node'])
  expect(runs.some(r => r.argv[0] === 'bwrap')).toBe(false)
  expect(writes[0]?.text).toContain('<pre class="mermaid">')
  expect(writes[0]?.text).toContain(`<script src="https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js"`)
  expect(seen).toContain('log: Diagrams left to Mermaid in the browser: no bun or node found.')
})

test('report mode: only the report tool, armed from the start', { options: REPORT_MODE }, async ($, on) => {
  const { registered } = world(on, SANDBOX, WRITE, [PLANNED])
  await $.session.start(START)
  expect(registered).toEqual([REPORT_TOOL])
})

test('report mode without a reports folder: no tool at all', { options: { uuid: UUID, report: true } }, async ($, on) => {
  const { registered } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('report mode: a report a word over its limit is written, and the overage named', { options: REPORT_MODE }, async ($, on) => {
  const { seen, writes } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, needs_your_eye: [OVER] })
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH}, opened it in the browser, and linked it from the task ` +
      `as the note "Report: ${REPORT_PATH}". Taken, though over its word limits: ${OVERAGE}.`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(seen).toEqual([`log: Report: ${REPORT_PATH}`, `log: Over its word limit: ${OVERAGE}`])
})

test('report mode: writes the page, opens it, and links it from the task, asking nothing', { options: REPORT_MODE }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })

  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH}, opened it in the browser, and linked it from the task ` +
      `as the note "Report: ${REPORT_PATH}".`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  // The plan the page shows is the task's own description and notes.
  const page = writes[0]?.text ?? ''
  const block = page.slice(page.indexOf('id="decision"'))
  expect(block).toContain('<p><strong>Description:</strong> feat: Old words</p>')
  expect(block).toContain('<li>first note</li>')
  expect(seen).toEqual([`log: Report: ${REPORT_PATH}`])
  expect(runs.map(run => run.argv)).toEqual([
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    ['sh', '-c', '{ niritasks task open-report "$1" || xdg-open "$1"; } >/dev/null 2>&1 </dev/null &', 'sh', REPORT_PATH],
    ['task', 'rc.hooks=off', 'rc.verbose=nothing', 'import'],
  ])
  // One note added; the description, the other notes and the tags kept.
  expect(JSON.parse(runs[2]?.init?.stdin ?? 'null')).toEqual([
    {
      ...PLANNED,
      annotations: [
        { entry: '20260101T000001Z', description: 'first note' },
        { entry: '20261006T010203Z', description: `Report: ${REPORT_PATH}` },
      ],
    },
  ])
})

test('report mode: one report a session; the second names the first', { options: REPORT_MODE }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX, WRITE, [PLANNED])
  await $.session.start(START)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  const again = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(again.deny).toBe(
    `show_task_report: this session's report is made and linked from the task: ${REPORT_PATH}. ` +
      'Close this tab and press Report on the card again for another.',
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(verbs(runs).filter(v => v === 'import')).toHaveLength(1)
})

test('report mode under home: the note and the answers name the report from ~/', { options: { ...HOME_REPORTS, report: true } }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(shown.result).toBe(
    `Wrote the report to ${HOME_NOTE}, opened it in the browser, and linked it from the task ` +
      `as the note "Report: ${HOME_NOTE}".`,
  )
  expect(writes.map(w => w.path)).toEqual([HOME_PATH, HOME_MD])
  expect(seen).toEqual([`log: Report: ${HOME_NOTE}`])
  expect(runs[1]?.argv).toEqual(OPEN(HOME_PATH))
  expect(JSON.parse(runs[2]?.init?.stdin ?? 'null')[0].annotations.at(-1)).toEqual({
    entry: '20261006T010203Z',
    description: `Report: ${HOME_NOTE}`,
  })
  const again = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(again.deny).toStartWith(`show_task_report: this session's report is made and linked from the task: ${HOME_NOTE}. `)
})

test('report mode: a reports folder outside home is named by its absolute path', { options: { ...REPORT_MODE, home: '/home/x' } }, async ($, on) => {
  const { runs } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(JSON.parse(runs[2]?.init?.stdin ?? 'null')[0].annotations.at(-1).description).toBe(`Report: ${REPORT_PATH}`)
})

// The page is built from the twin as it reads back: a task note on two lines
// cannot be one numbered line of the twin, so nothing is written.
test('report mode: a plan whose twin would not read back writes nothing', { options: REPORT_MODE }, async ($, on) => {
  const torn = { ...PLANNED, annotations: [{ entry: '20260101T000001Z', description: 'first\nnote' }] }
  const { runs, writes } = world(on, SANDBOX, WRITE, [torn])
  await $.session.start(START)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })

  expect(shown.deny).toStartWith('show_task_report: nothing written: its Markdown twin does not read back (line ')
  expect(shown.deny).toContain('must be made one line first')
  expect(writes).toEqual([])
  expect(runs.map(run => run.argv.at(-1))).toEqual(['export'])
})

test('report mode: an unplanned task is refused before anything is written', { options: REPORT_MODE }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX, WRITE, [TASK])
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe('show_task_report: nothing written: the task has no plan: it is not tagged planned. Refine it first.')
  expect(writes).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
})

test('report mode: a task no longer pending is refused', { options: REPORT_MODE }, async ($, on) => {
  const { writes } = world(on, SANDBOX, WRITE, [{ ...PLANNED, status: 'completed' }])
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe('show_task_report: nothing written: the task is completed, not pending.')
  expect(writes).toEqual([])
})

test('report mode: a report over its limits is refused with every reason, and nothing is linked', { options: REPORT_MODE }, async ($, on) => {
  const { runs, writes } = world(on, SANDBOX, WRITE, [PLANNED])
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL, changes: [], checks: [] })
  expect(answer.deny).toBe(
    'show_task_report: changes must have 1 to 3 items, not 0; checks must have 1 to 6 items, not 0',
  )
  expect(writes).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
  // Fixed, it goes through: the refusal did not spend the session's one report.
  expect((await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })).result).toContain(`Wrote the report to ${REPORT_PATH}`)
})

test('report mode: a failed import is reported, with the page already open', { options: REPORT_MODE }, async ($, on) => {
  engine(on, SANDBOX)
  let imports = 0
  on('process.run', ($, e) => {
    if (e.argv.includes('export')) return ran(JSON.stringify([PLANNED]))
    if (e.argv.includes('import')) {
      imports += 1
      return imports === 1 ? { value: { ...ran('', 2).value, stderr: 'Not a valid JSON value.' } } : ran('')
    }
    return ran('')
  })
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe(
    `Wrote the report to ${REPORT_PATH} and opened it, but task import failed (2): Not a valid JSON value. ` +
      'The task does not link it.',
  )
  const again = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(again.result).toContain(`Wrote the report to ${REPORT_PATH}`)
  expect(imports).toBe(2)
})

test('report mode: sandbox off, no tool', { options: REPORT_MODE }, async ($, on) => {
  const { registered } = world(on, { sandbox: { enabled: true, failIfUnavailable: false } })
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('refine mode is unchanged by the option being false', { options: { ...REPORTS, report: false } }, async ($, on) => {
  const { registered, seen } = world(on, SANDBOX)
  await $.session.start(START)
  expect(registered).toEqual([TOOL, REPORT_TOOL])
  await $.tool.call(CALL)
  expect(seen.at(-1)).toBe(`ask: Write this to the task? [${WRITE} | ${REPORT} | Change something] (Task plan)`)
})

// An unattended run: `niritasks task refine --unattended`'s `claude -p`,
// where no one is at the keyboard. The write tool writes without asking and
// the report tool opens nothing.
const UNATTENDED = { uuid: UUID, reports: '/r', unattended: true }
const UNATTENDED_REPORT = { uuid: UUID, reports: '/r', report: true, unattended: true }

test('unattended: shows the plan and writes it, asking nothing', { options: UNATTENDED }, async ($, on) => {
  const { registered, runs, seen } = world(on, SANDBOX, { deny: 'nobody should be asked' })
  await $.session.start(START)
  expect(registered).toEqual([TOOL, REPORT_TOOL])

  const answer = await $.tool.call(CALL)

  expect(answer.deny).toBeUndefined()
  expect(answer.result).toBe(`Wrote the plan to task ${UUID}: 2 note(s), tagged planned.`)
  expect(seen).toEqual(['log: Description: feat: New words', 'log: Note 1: step one', 'log: Note 2: step two'])
  expect(verbs(runs)).toEqual(['export', 'export', 'import'])
  expect(JSON.parse(runs[2]?.init?.stdin ?? 'null')[0].tags).toEqual(['planned', 'zeta'])
})

test('unattended: the task changing under the write still refuses it', { options: UNATTENDED }, async ($, on) => {
  const edited = { ...TASK, annotations: [...TASK.annotations, { entry: '20260101T000002Z', description: 'added' }] }
  const { runs } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [TASK, edited])
  await $.session.start(START)
  const answer = await $.tool.call(CALL)
  expect(answer.deny).toContain('Nothing written: the task changed since it was read')
  expect(verbs(runs)).toEqual(['export', 'export'])
})

test('unattended: a line too long is still refused before anything', { options: UNATTENDED }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, { deny: 'nobody should be asked' })
  await $.session.start(START)
  const answer = await $.tool.call({ ...CALL, notes: ['x'.repeat(2000)] })
  expect(answer.deny).toStartWith('Nothing written: note 1 would show as a line of 2008 characters')
  expect(seen).toEqual([])
  expect(verbs(runs)).toEqual(['export'])
})

test('unattended: a report no one asked for is refused, as nobody can ask', { options: UNATTENDED }, async ($, on) => {
  const { writes } = world(on, SANDBOX, { deny: 'nobody should be asked' })
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toStartWith('show_task_report: the person has not asked for a report.')
  expect(writes).toEqual([])
})

test('unattended report mode: writes the page and links it, opening nothing', { options: UNATTENDED_REPORT }, async ($, on) => {
  const { runs, seen, writes } = world(on, SANDBOX, { deny: 'nobody should be asked' }, [PLANNED])
  await $.session.start(START)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })

  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the report to ${REPORT_PATH} and linked it from the task as the note "Report: ${REPORT_PATH}".`,
  )
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(writes[1]?.text).toContain(`**Description:** ${PLANNED.description}`)
  expect(seen).toEqual([`log: Report: ${REPORT_PATH}`])
  expect(runs.map(run => run.argv)).toEqual([
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    ['task', 'rc.hooks=off', 'rc.verbose=nothing', 'import'],
  ])
  const [imported] = JSON.parse(runs[1]?.init?.stdin ?? 'null')
  expect(imported.annotations.at(-1).description).toBe(`Report: ${REPORT_PATH}`)
})

test('unattended report mode: a failed import says nothing was opened', { options: UNATTENDED_REPORT }, async ($, on) => {
  engine(on, SANDBOX)
  on('process.run', ($, e) =>
    e.argv.includes('export')
      ? ran(JSON.stringify([PLANNED]))
      : { value: { ...ran('', 2).value, stderr: 'Not a valid JSON value.' } },
  )
  await $.session.start(START)
  const answer = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(answer.deny).toBe(
    `Wrote the report to ${REPORT_PATH}, but task import failed (2): Not a valid JSON value. The task does not link it.`,
  )
})

test('attended sessions are unchanged by the option being false', { options: { ...REPORTS, unattended: false } }, async ($, on) => {
  const { runs, seen } = world(on, SANDBOX, [REPORT, WRITE], [TASK])
  await $.session.start(START)
  expect((await $.tool.call(CALL)).deny).toContain(`the person chose "${REPORT}"`)
  expect(seen.at(-1)).toBe(`ask: Write this to the task? [${WRITE} | ${REPORT} | Change something] (Task plan)`)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(runs.at(-1)?.argv[0]).toBe('sh')
})

// An Explain branch session: Rust sets report and branch, and names the
// worktree and the branch it is measured against.
const BRANCH_MODE = { uuid: UUID, reports: '/r', report: true, branch: true, worktree: '/w', base: 'main' }
const BRANCH_PATH = '/r/branch-0b8f6a52-20261006-010203.html'
const BRANCH_MD = '/r/branch-0b8f6a52-20261006-010203.md'
const MERGE_BASE = `f00dfee${'2'.repeat(33)}`
const GIT_PREFIX = ['git', '--no-optional-locks', '-C', '/w', '-c', 'core.fsmonitor=false', '-c', 'core.quotePath=false']

// What git answers for BRANCH_MIN's branch, by the command's first word; a
// command in `fails` exits 128 with git's error.
const gitAnswer = (argv: readonly string[], fails?: string) => {
  const command = argv[GIT_PREFIX.length]
  if (command === fails) return { value: { exitCode: 128, stdout: '', stderr: `fatal: ${command} broke\n`, isStdoutTruncated: false, isStderrTruncated: false } }
  if (command === 'rev-parse') return ran('task/feat-new-words-0b8f6a52\n')
  if (command === 'merge-base') return ran(`${MERGE_BASE}\n`)
  if (command === 'log') return ran(`${SHA_A}\tfeat: add the words\n${SHA_B}\ttest: pin the words\n`)
  if (command === 'diff') return ran('10\t2\tsrc/words.rs\n2\t1\tREADME.md\n')
  if (command === 'status') return ran(' M README.md\n')
  return ran('')
}

// The world, with git answering for the worktree, and `files` read as
// their text there.
const branchWorld = (on: On, exports: readonly object[] = [TASK], fails?: string, files: Record<string, string> = {}) => {
  const { registered, seen, writes, reads } = engine(on, SANDBOX, { deny: 'nobody should be asked' }, undefined, false, files)
  const runs: Run[] = []
  let exported = 0
  on('process.run', ($, e) => {
    runs.push(e)
    if (e.argv[0] === 'git') return gitAnswer(e.argv, fails)
    if (e.argv[0] === 'bwrap' || e.argv[2]?.includes('command -v')) return ran('', 1)
    if (!e.argv.includes('export')) return ran('')
    const task = exports[Math.min(exported++, exports.length - 1)]
    return ran(JSON.stringify([task]))
  })
  return { registered, runs, seen, writes, reads }
}

test('branch mode: only the report tool', { options: BRANCH_MODE }, async ($, on) => {
  const { registered } = branchWorld(on)
  await $.session.start(START)
  expect(registered).toEqual([REPORT_TOOL])
})

test('branch mode without a worktree or base: no tool at all', { options: { ...BRANCH_MODE, worktree: '' } }, async ($, on) => {
  const { registered } = branchWorld(on)
  await $.session.start(START)
  expect(registered).toEqual([])
})

test('branch mode: reads git, writes the deck, opens it, and links it as a Branch report', { options: BRANCH_MODE }, async ($, on) => {
  // An unplanned task: a branch needs no plan.
  const { runs, seen, writes } = branchWorld(on)
  await $.session.start(START)

  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })

  expect(shown.deny).toBeUndefined()
  expect(shown.result).toBe(
    `Wrote the branch report to ${BRANCH_PATH}, opened it in the browser, and linked it from the task ` +
      `as the note "Branch report: ${BRANCH_PATH}".`,
  )
  expect(writes.map(w => w.path)).toEqual([BRANCH_PATH, BRANCH_MD])
  const page = writes[0]?.text ?? ''
  expect(page).toContain('<span class="pill">Branch report</span>')
  expect(page).toContain('<li><code>abc1234</code>feat: add the words</li>')
  expect(writes[1]?.text).toContain('_Branch `task/feat-new-words-0b8f6a52` from `main` at `f00dfee`: 2 commits, 2 files, +12 -3, 1 uncommitted._')
  expect(seen).toEqual([`log: Branch report: ${BRANCH_PATH}`])
  expect(runs.map(run => run.argv)).toEqual([
    ['task', 'rc.hooks=off', 'rc.json.array=on', UUID, 'export'],
    [...GIT_PREFIX, 'rev-parse', '--abbrev-ref', 'HEAD'],
    [...GIT_PREFIX, 'merge-base', 'HEAD', 'main'],
    [...GIT_PREFIX, 'log', '--reverse', '--format=%H%x09%s', `${MERGE_BASE}..HEAD`],
    [...GIT_PREFIX, 'diff', '--numstat', '--no-renames', '--no-ext-diff', '--no-textconv', MERGE_BASE, 'HEAD'],
    [...GIT_PREFIX, 'status', '--porcelain', '--untracked-files=normal'],
    ['sh', '-c', '{ niritasks task open-report "$1" || xdg-open "$1"; } >/dev/null 2>&1 </dev/null &', 'sh', BRANCH_PATH],
    ['task', 'rc.hooks=off', 'rc.verbose=nothing', 'import'],
  ])
  expect(JSON.parse(runs[7]?.init?.stdin ?? 'null')).toEqual([
    {
      ...TASK,
      annotations: [
        { entry: '20260101T000001Z', description: 'first note' },
        { entry: '20261006T010203Z', description: `Branch report: ${BRANCH_PATH}` },
      ],
    },
  ])
})

test("branch mode: a plan's Report: note is left as it is", { options: BRANCH_MODE }, async ($, on) => {
  const reported = { ...PLANNED, annotations: [...PLANNED.annotations, { entry: '20260101T000002Z', description: 'Report: /r/plan.html' }] }
  const { runs } = branchWorld(on, [reported])
  await $.session.start(START)
  await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  const imported = JSON.parse(runs.at(-1)?.init?.stdin ?? 'null')
  expect(imported[0].annotations.map((a: { description: string }) => a.description)).toEqual([
    'first note',
    'Report: /r/plan.html',
    `Branch report: ${BRANCH_PATH}`,
  ])
  expect(imported[0].tags).toEqual(['planned', 'zeta'])
})

test('branch mode: a story that leaves out a commit is refused, and nothing written', { options: BRANCH_MODE }, async ($, on) => {
  const { writes, runs } = branchWorld(on)
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN, story: [{ step: 's', commits: ['abc1234'], points: ['p'] }] })
  expect(shown.deny).toBe('show_task_report: the story leaves out 1 commit(s): abd5678; every commit goes in one step')
  expect(writes).toEqual([])
  expect(verbs(runs)).not.toContain('import')
})

test('branch mode: a git command that fails is named, and nothing written', { options: BRANCH_MODE }, async ($, on) => {
  const { writes } = branchWorld(on, [TASK], 'merge-base')
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(shown.deny).toBe('show_task_report: nothing written: git merge-base HEAD main failed (128): fatal: merge-base broke.')
  expect(writes).toEqual([])
})

test('branch mode: a deleted task is refused before git is read', { options: BRANCH_MODE }, async ($, on) => {
  const { runs } = branchWorld(on, [{ ...TASK, status: 'deleted' }])
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(shown.deny).toBe('show_task_report: nothing written: the task is deleted.')
  expect(runs.some(r => r.argv[0] === 'git')).toBe(false)
})

test("branch mode: a plan report's fields are refused", { options: BRANCH_MODE }, async ($, on) => {
  branchWorld(on)
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(shown.deny).toContain('checks is not used in a branch report: plan_vs_done takes its place')
})

test('branch mode: one report a session', { options: BRANCH_MODE }, async ($, on) => {
  const { writes } = branchWorld(on)
  await $.session.start(START)
  await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  const again = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(again.deny).toBe(
    `show_task_report: this session's branch report is made and linked from the task: ${BRANCH_PATH}. ` +
      "Close this tab and choose Explain branch in the card's Go to list again for another.",
  )
  expect(writes.map(w => w.path)).toEqual([BRANCH_PATH, BRANCH_MD])
})

// An Explain branch session whose tests Rust ran before it started: the
// results file it wrote.
const TESTS = '/r/branch-0b8f6a52-20261006-010000-tests.json'
const TESTS_MODE = { ...BRANCH_MODE, tests: TESTS }
const TEST_LOG = '/r/branch-0b8f6a52-20261006-010000-tests.log'
const testRun = (exit_code: number) =>
  JSON.stringify({
    command: 'bash tests/all.sh',
    exit_code,
    log: TEST_LOG,
    started: '2026-10-06T01:00:00Z',
    finished: '2026-10-06T01:02:00Z',
  })

test('branch mode with a test run: reads it, and the deck and twin name it', { options: TESTS_MODE }, async ($, on) => {
  const { writes, reads, runs } = branchWorld(on, [TASK], undefined, { [TESTS]: testRun(0) })
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(shown.deny).toBeUndefined()
  expect(reads).toContain(TESTS)
  expect(writes.map(w => w.path)).toEqual([BRANCH_PATH, BRANCH_MD])
  expect(writes[0]?.text).toContain('Run before this session: <code>bash tests/all.sh</code>, exit 0, log ')
  expect(writes[1]?.text).toContain(`_Run before this session: \`bash tests/all.sh\`, exit 0, log \`${TEST_LOG}\`._`)
  // Read with the engine's own file system: no command run for it.
  expect(runs.map(r => r.argv[0])).toEqual(['task', 'git', 'git', 'git', 'git', 'git', 'sh', 'task'])
})

test('branch mode with a test run: a suite that disagrees with it is refused, and nothing written', { options: TESTS_MODE }, async ($, on) => {
  const { writes, runs } = branchWorld(on, [TASK], undefined, { [TESTS]: testRun(1) })
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(shown.deny).toBe(
    `show_task_report: the test run failed (exit 1): name the suite and the tests that failed, from its log ${TEST_LOG}`,
  )
  expect(writes).toEqual([])
  expect(verbs(runs)).not.toContain('import')
})

test('branch mode with a test run: a suite that agrees with a failed run is written', { options: TESTS_MODE }, async ($, on) => {
  const { writes } = branchWorld(on, [TASK], undefined, { [TESTS]: testRun(1) })
  await $.session.start(START)
  const failed = {
    runs: [{ suite: 'all.sh', command: 'bash tests/all.sh', passed: 2, failed: 1 }],
    failing: [{ test: 'words_are_new', file: 'src/words.rs', why: 'It expects old words.' }],
  }
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN, suite: failed })
  expect(shown.deny).toBeUndefined()
  expect(writes[1]?.text).toContain('exit 1, log ')
})

test('branch mode with a test run that is missing: refused, and nothing written', { options: TESTS_MODE }, async ($, on) => {
  const { writes } = branchWorld(on)
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(shown.deny).toStartWith(`show_task_report: nothing written: could not read the test run ${TESTS}: `)
  expect(writes).toEqual([])
})

test('branch mode with a test run that is malformed: refused, and nothing written', { options: TESTS_MODE }, async ($, on) => {
  const { writes } = branchWorld(on, [TASK], undefined, { [TESTS]: '{"command":' })
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(shown.deny).toStartWith(`show_task_report: nothing written: could not read the test run ${TESTS}: it is not JSON (`)
  expect(writes).toEqual([])
})

test('branch mode with no test run: nothing is read for one, and no run is named', { options: { ...BRANCH_MODE, tests: '' } }, async ($, on) => {
  const { writes, reads } = branchWorld(on)
  await $.session.start(START)
  const shown = await $.tool.call({ tool: REPORT_TOOL, ...BRANCH_MIN })
  expect(shown.deny).toBeUndefined()
  expect(reads.filter(r => r.endsWith('.json'))).toEqual([])
  expect(writes[1]?.text).not.toContain('Run before this session')
  expect(writes[0]?.text).not.toContain('Run before this session')
})
