import type { EngineInterface, Register } from 'claude-code'
import {
  CHANGE,
  HEADER,
  NOT_ASKED_FOR,
  QUESTION,
  REPORT,
  REPORT_FIRST,
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
import { reportPage } from './page'
import { openArgv, parseReport, reportPath, reportsDir, withReportNotes } from './report'
import type { MadeReport } from './report'

// The tool's listed name: mcp__<plugin>__<name>, hyphens kept.
export const TOOL = 'mcp__niri-tasks-refine__write_task_plan'
export const REPORT_TOOL = 'mcp__niri-tasks-refine__show_task_report'

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

const TEXT = { type: 'string' }
const TEXTS = { type: 'array', items: TEXT }

const REPORT_SPEC = {
  name: 'show_task_report',
  description:
    'Writes an HTML report of the plan in one fixed, easy-to-read layout and opens it in the browser, once the ' +
    'person has chosen "Show me a report first" in write_task_plan. Fill the fields as the refine-task skill\'s ' +
    'report-catalogue.md says. Every text field is one line of plain text; a report over the limits is refused ' +
    'with every reason at once.',
  inputSchema: {
    type: 'object',
    properties: {
      title: { ...TEXT, description: "The task's new description." },
      summary: { ...TEXT, description: 'One sentence, at most 35 words: what the task does.' },
      changes: { ...TEXTS, description: '1-3 lines, at most 20 words each: what will be different.' },
      unchanged: { ...TEXTS, description: '0-3 lines, at most 20 words each: what stays the same.' },
      needs_your_eye: { ...TEXTS, description: '0-3 lines, at most 30 words each: where the person should judge.' },
      terms: {
        type: 'array',
        description: '0-5 words the report uses, each with a plain meaning.',
        items: { type: 'object', properties: { term: TEXT, meaning: TEXT }, required: ['term', 'meaning'] },
      },
      sections: {
        type: 'array',
        description: '1-6 parts: a heading, 1-5 points, optionally one diagram with a look_at line, and collapsed detail.',
        items: {
          type: 'object',
          properties: {
            heading: TEXT,
            look_at: TEXT,
            diagram: { type: 'object', properties: { mermaid: TEXT, svg: TEXT } },
            points: TEXTS,
            detail: { ...TEXT, description: 'HTML, shown collapsed under "More detail".' },
          },
          required: ['heading', 'points'],
        },
      },
      files: {
        type: 'array',
        items: {
          type: 'object',
          properties: { path: TEXT, change: { type: 'string', enum: ['new', 'change', 'remove'] }, why: TEXT },
          required: ['path', 'change', 'why'],
        },
      },
      checks: {
        type: 'array',
        items: { type: 'object', properties: { check: TEXT, how: TEXT }, required: ['check', 'how'] },
      },
    },
    required: ['title', 'summary', 'changes', 'sections', 'files', 'checks'],
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
  // Where reports go, or undefined when this session offers none.
  const reports = reportsDir(options.reports)
  // The plan the person was shown when they chose REPORT in answer to the
  // latest question, spent by the first well-formed report. Module state: a
  // reload forgets it, and the model is told to ask again.
  let askedFor: Plan | undefined
  // The reports made this session, linked from the task's notes when it is
  // written. Module state: a reload forgets it, so a report made before a
  // reload is not linked (its path is still in the transcript's `Report:`
  // line).
  const made: MadeReport[] = []

  on('session.start', async ($, e, next) => {
    if ((await armedUuid($, options.uuid)) !== undefined) {
      await $.tool.register(SPEC)
      if (reports !== undefined) await $.tool.register(REPORT_SPEC)
    }
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
    // what the model printed, with a note linking each report made this
    // session; refused whole before a line could be cut.
    const plan = { description: input.description, notes: withReportNotes(made, input) }
    const lines = planLines(plan)
    const tooLong = overlong(lines)
    if (tooLong !== undefined) return { deny: `Nothing written: ${tooLong}` }
    for (const line of lines) $.ui.log(line)

    // A new question supersedes any earlier REPORT choice.
    askedFor = undefined
    const choices = reports === undefined ? [WRITE, CHANGE] : [WRITE, REPORT, CHANGE]
    let answer: string
    try {
      answer = await $.ui.ask(QUESTION, { options: choices, header: HEADER })
    } catch (error) {
      return { deny: notAsked(error instanceof Error ? error.message : String(error)) }
    }
    if (answer === REPORT && reports !== undefined) {
      askedFor = { description: input.description, notes: input.notes }
      return { deny: REPORT_FIRST }
    }
    if (answer !== WRITE) return { deny: notApproved(answer) }

    // The person may have taken minutes: read the task again.
    const now = await current($, uuid, input.expected)
    if ('deny' in now) return now

    const planned = merge(now.task, { ...input, notes: plan.notes }, await $.clock.now())
    const imported = await $.process.run([...TASK, 'rc.verbose=nothing', 'import'], {
      stdin: JSON.stringify([planned]),
    })
    if (imported.exitCode !== 0) {
      return { deny: `task import failed (${imported.exitCode}): ${imported.stderr.trim()}` }
    }
    return {
      result: `Wrote the plan to task ${uuid}: ${plan.notes.length} note(s), tagged planned.`,
    }
  }).catch($ => ({ deny: `${$.plugin.name}: write_task_plan failed before writing.` }))

  // The report the person asked for: written outside the sandbox, which
  // cannot write the reports folder, and opened in their browser.
  on('tool.call', { tool: REPORT_TOOL }, async ($, e) => {
    const uuid = await armedUuid($, options.uuid)
    if (uuid === undefined || reports === undefined) {
      return { deny: 'show_task_report is not armed in this session.' }
    }
    if (askedFor === undefined) return { deny: NOT_ASKED_FOR }

    const report = parseReport(e)
    if (typeof report === 'string') return { deny: `show_task_report: ${report}` }

    const shown = askedFor
    askedFor = undefined
    const path = reportPath(reports, uuid, await $.clock.now())
    await $.fs.write(path, reportPage(report, shown))
    made.push({ path, plan: shown })
    $.ui.log(`Report: ${path}`)
    await $.process.run(openArgv(path))
    return {
      result:
        `Wrote the report to ${path} and opened it in the browser. ` +
        'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
        'and the task will link the report in its notes.',
    }
  }).catch($ => ({ deny: `${$.plugin.name}: show_task_report failed.` }))
}
