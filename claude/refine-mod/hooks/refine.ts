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
  notesOf,
  overlong,
  parseExport,
  parseInput,
  planLines,
  refusal,
} from './plan'
import type { Plan, Task } from './plan'
import { RENDERER, drawArgv, drawSources, drawnSvgs, runtimePath, whichArgv } from './draw'
import { reportPage } from './page'
import { FONT_FILES } from './style'
import type { Fonts } from './style'
import { markdownPath, twinOf } from './markdown'
import { homeDir, madeAlready, notePath, openArgv, overLimits, parseReport, reportPath, reportsDir, unreportable, withReportNote, withReportNotes } from './report'
import type { MadeReport, Report } from './report'

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
const DIAGRAM = {
  type: 'object',
  description: 'A Mermaid diagram: { "mermaid": "<source>" }.',
  properties: { mermaid: TEXT },
  required: ['mermaid'],
}

// The report tool's schema is one; what the model is told differs by mode:
// in a refine the report follows the person's "Show me a report first", in a
// report session it is the whole job.
const reportSpec = (reportOnly: boolean) => ({
  name: 'show_task_report',
  description: reportOnly
    ? "Writes an HTML report of this task's plan, its own description and notes as they are now, in one fixed, " +
      'easy-to-read layout, opens it in the browser and links it from the task as a Report: <path> note. ' +
      "Fill the fields as the refine-task skill's report-catalogue.md says. Every text field is one line of " +
      'plain text; a report over the limits is refused with every reason at once. One report a session.'
    : 'Writes an HTML report of the plan in one fixed, easy-to-read layout and opens it in the browser, once the ' +
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
        description:
          '1-9 parts, each looking at the change through one lens (kind): before-after (always, unless nothing ' +
          'changes), part, structure, data-flow, outside-tools, styling, code, newcomer. Each has a heading, 1-5 ' +
          'points, and a diagram (or a before/after pair) with a look_at line wherever one helps.',
        items: {
          type: 'object',
          properties: {
            kind: {
              type: 'string',
              enum: ['before-after', 'part', 'structure', 'data-flow', 'outside-tools', 'styling', 'code', 'newcomer'],
            },
            heading: TEXT,
            look_at: TEXT,
            diagram: DIAGRAM,
            before: DIAGRAM,
            after: DIAGRAM,
            points: TEXTS,
            code: {
              type: 'array',
              description: '1-3 files: the key code as it is and as it will be, at most 16 lines each.',
              items: {
                type: 'object',
                properties: { file: TEXT, before: TEXT, after: TEXT },
                required: ['file', 'before', 'after'],
              },
            },
            tools: {
              type: 'array',
              description: 'For outside-tools: each tool, how it connects, and how hard it is to swap.',
              items: {
                type: 'object',
                properties: { tool: TEXT, how: TEXT, swap: { type: 'string', enum: ['easy', 'medium', 'hard'] }, why: TEXT },
                required: ['tool', 'how', 'swap', 'why'],
              },
            },
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
})

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

// The report's two typefaces, shipped in the mod's fonts/ folder, read on
// the first report and kept in `kept`. If they cannot be read, the report is
// still written, in the system's fonts.
const readFonts = async ($: EngineInterface, kept: { read?: Fonts }): Promise<Fonts | undefined> => {
  if (kept.read !== undefined) return kept.read
  try {
    const display = await $.fs.read(`${$.plugin.root}/fonts/${FONT_FILES.display}`, { as: 'bytes' })
    const body = await $.fs.read(`${$.plugin.root}/fonts/${FONT_FILES.body}`, { as: 'bytes' })
    kept.read = { display: display.base64, body: body.base64 }
    return kept.read
  } catch {
    return undefined
  }
}

// The real path of the runtime the renderer runs on: bun if there is one,
// else node; undefined when there is neither.
const findRuntime = async ($: EngineInterface): Promise<string | undefined> => {
  for (const runtime of ['bun', 'node'] as const) {
    try {
      const path = runtimePath(await $.process.run(whichArgv(runtime)))
      if (path !== undefined) return path
    } catch {
      // This one cannot be looked up: try the next.
    }
  }
  return undefined
}

// Says in the transcript why diagrams were left to Mermaid in the browser,
// which needs the internet, so the fallback is seen.
const leftToBrowser = ($: EngineInterface, why: string): void => {
  $.ui.log(`Diagrams left to Mermaid in the browser: ${why}.`)
}

// Draws each Mermaid diagram before the page opens, in the shipped renderer
// inside its fence, on the one runtime found: a renderer that fails or times
// out is not tried again. Undefined for any it could not draw, which the page
// leaves to Mermaid in the browser, and says why.
const drawDiagrams = async ($: EngineInterface, report: Report): Promise<(string | undefined)[]> => {
  const sources = drawSources(report)
  const diagrams = sources.filter((s): s is string => s !== undefined)
  const undrawn = sources.map(() => undefined)
  if (diagrams.length === 0) return undrawn
  const runtime = await findRuntime($)
  if (runtime === undefined) {
    leftToBrowser($, 'no bun or node found')
    return undrawn
  }
  let run: Awaited<ReturnType<EngineInterface['process']['run']>>
  try {
    run = await $.process.run(drawArgv(runtime, `${$.plugin.root}/${RENDERER}`), {
      stdin: JSON.stringify({ diagrams }),
      timeoutMs: 20_000,
    })
  } catch {
    leftToBrowser($, 'the diagram renderer could not run (no bwrap, or it timed out)')
    return undrawn
  }
  if (run.exitCode !== 0) {
    leftToBrowser($, `the diagram renderer failed (exit ${run.exitCode})`)
    return undrawn
  }
  const svgs = drawnSvgs(run.stdout, sources)
  const refused = sources.filter((s, i) => s !== undefined && svgs[i] === undefined).length
  if (refused > 0) leftToBrowser($, `the renderer's output was refused for ${refused} diagram(s)`)
  return svgs
}

// The person's answer to "Write this to the task?", or the refusal to answer
// the call with when they could not be asked. Unattended, there is no one to
// ask and the command was the ask: WRITE, with nothing shown but the lines
// already logged.
const approval = async (
  $: EngineInterface,
  reports: string | undefined,
  unattended: boolean,
): Promise<string | { deny: string }> => {
  if (unattended) return WRITE
  const choices = reports === undefined ? [WRITE, CHANGE] : [WRITE, REPORT, CHANGE]
  try {
    return await $.ui.ask(QUESTION, { options: choices, header: HEADER })
  } catch (error) {
    return { deny: notAsked(error instanceof Error ? error.message : String(error)) }
  }
}

export const register: Register = (on, options) => {
  // Where reports go, or undefined when this session offers none.
  const reports = reportsDir(options.reports)
  // The home folder: a report under it is named from ~/ in its note and the
  // answers, so the note survives a username or home folder change.
  const home = homeDir(options.home)
  // Report mode: the Go to list's Generate report on a planned task, not a refine.
  // Only the report tool is offered, armed from the start on the task's own
  // description and notes, and the mod links the report from the task
  // itself: the button press was the ask, so no question is put. One a
  // session, as a refine makes one per ask. Module state, like `askedFor`.
  const reportOnly = options.report === true
  // Unattended: a `claude -p` run with no one at the keyboard
  // (`niritasks task refine --unattended`, the task box's Add & all). The
  // command was the ask, as the button press is in report mode: the write
  // tool writes with no question, its lines still logged and the task still
  // read again first, and the report tool opens nothing. $.ui.ask rejects in
  // a -p run anyway; this is what makes the run do its job rather than fail
  // at the question.
  const unattended = options.unattended === true
  let linked: string | undefined
  // The plan the person was shown when they chose REPORT in answer to the
  // latest question, spent by the first well-formed report. Module state: a
  // reload forgets it, and the model is told to ask again.
  let askedFor: Plan | undefined
  // The reports made this session, linked from the task's notes when it is
  // written. Module state: a reload forgets it, so a report made before a
  // reload is not linked (its path is still in the transcript's `Report:`
  // line).
  const made: MadeReport[] = []
  // The report's two typefaces, read on the first report and kept.
  const fonts: { read?: Fonts } = {}

  on('session.start', async ($, e, next) => {
    if ((await armedUuid($, options.uuid)) !== undefined) {
      if (!reportOnly) await $.tool.register(SPEC)
      if (reports !== undefined) await $.tool.register(reportSpec(reportOnly))
    }
    return next(e)
  })

  // The plugin serves its own tool: no hook beneath answers it, so this one
  // never calls next.
  on('tool.call', { tool: TOOL }, async ($, e) => {
    const uuid = await armedUuid($, options.uuid)
    if (uuid === undefined) return { deny: 'write_task_plan is not armed in this session.' }
    if (reportOnly) return { deny: 'write_task_plan is not offered in a report session: nothing here refines.' }

    const input = parseInput(e)
    if (typeof input === 'string') return { deny: `write_task_plan: ${input}` }

    // Checked before the person is asked, so they are never asked about a
    // plan that could not be written.
    const before = await current($, uuid, input.expected)
    if ('deny' in before) return before

    // The plan as the tool will write it, from its own arguments, not from
    // what the model printed, with a note linking each report made this
    // session, and an earlier session's report marked as an earlier draft
    // when the plan differs from the task as read; refused whole before a
    // line could be cut.
    const plan = { description: input.description, notes: withReportNotes(made, input, input.expected) }
    const lines = planLines(plan)
    const tooLong = overlong(lines)
    if (tooLong !== undefined) return { deny: `Nothing written: ${tooLong}` }
    for (const line of lines) $.ui.log(line)

    // A new question supersedes any earlier REPORT choice.
    askedFor = undefined
    const answer = await approval($, reports, unattended)
    if (typeof answer !== 'string') return answer
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
    if (reportOnly) {
      if (linked !== undefined) return { deny: madeAlready(linked) }
      // The plan is the task as it is now: read here, not at session start,
      // so an edit made since the button was pressed is what is reported.
      const exported = await $.process.run([...TASK, 'rc.json.array=on', uuid, 'export'])
      if (exported.exitCode !== 0) {
        return { deny: `task export failed (${exported.exitCode}): ${exported.stderr.trim()}` }
      }
      const task = parseExport(exported.stdout)
      const why = unreportable(task)
      if (why !== undefined || task === undefined) return { deny: `show_task_report: nothing written: ${why}.` }

      const parsed = parseReport(e)
      if (typeof parsed === 'string') return { deny: `show_task_report: ${parsed}` }

      const shown = { description: task.description, notes: notesOf(task) }
      // The page is built from the report's Markdown twin as it reads back,
      // never from the fields beside it, so the two cannot disagree.
      const twin = twinOf(parsed.report, shown)
      if (typeof twin === 'string') {
        return {
          deny:
            `show_task_report: nothing written: ${twin}. The task's own description and notes are reported ` +
            'as they are; one on several lines must be made one line first.',
        }
      }
      const path = reportPath(reports, uuid, await $.clock.now())
      // What the note and the answers name; the files are written and
      // opened by `path`.
      const note = notePath(path, home)
      const page = reportPage(twin.report, twin.plan, await readFonts($, fonts), await drawDiagrams($, twin.report))
      await $.fs.write(path, page)
      // The twin itself, which herdr's task-docs plugin opens in
      // plannotator-tui so comments on it reach the task's Claude.
      await $.fs.write(markdownPath(path), twin.markdown)
      $.ui.log(`Report: ${note}`)
      for (const warning of parsed.warnings) $.ui.log(`Over its word limit: ${warning}`)
      // Unattended, nothing opens: no one is at the screen, and the task's
      // Report: note is how the report is found later.
      if (!unattended) await $.process.run(openArgv(path))
      const opened = unattended ? '' : ' and opened it'

      // The session's one write to the task, and the mod's, not the model's:
      // the note's path is the one just written.
      const imported = await $.process.run([...TASK, 'rc.verbose=nothing', 'import'], {
        stdin: JSON.stringify([withReportNote(task, note, await $.clock.now())]),
      })
      if (imported.exitCode !== 0) {
        return {
          deny:
            `Wrote the report to ${note}${opened}, but task import failed (${imported.exitCode}): ` +
            `${imported.stderr.trim()} The task does not link it.`,
        }
      }
      linked = note
      return {
        result:
          (unattended
            ? `Wrote the report to ${note} and linked it from the task as the note "Report: ${note}".`
            : `Wrote the report to ${note}, opened it in the browser, and linked it from the task ` +
              `as the note "Report: ${note}".`) + overLimits(parsed.warnings),
      }
    }
    if (askedFor === undefined) return { deny: NOT_ASKED_FOR }

    const parsed = parseReport(e)
    if (typeof parsed === 'string') return { deny: `show_task_report: ${parsed}` }

    const twin = twinOf(parsed.report, askedFor)
    if (typeof twin === 'string') return { deny: `show_task_report: nothing written: ${twin}.` }

    const shown = askedFor
    askedFor = undefined
    const path = reportPath(reports, uuid, await $.clock.now())
    // What the note and the answers name; the files are written and opened
    // by `path`.
    const note = notePath(path, home)
    const page = reportPage(twin.report, twin.plan, await readFonts($, fonts), await drawDiagrams($, twin.report))
    await $.fs.write(path, page)
    await $.fs.write(markdownPath(path), twin.markdown)
    made.push({ path: note, plan: shown })
    $.ui.log(`Report: ${note}`)
    for (const warning of parsed.warnings) $.ui.log(`Over its word limit: ${warning}`)
    await $.process.run(openArgv(path))
    return {
      result:
        `Wrote the report to ${note} and opened it in the browser. ` +
        'Now call write_task_plan again with the same plan; the person approves or changes it there, ' +
        'and the task will link the report in its notes.' +
        overLimits(parsed.warnings),
    }
  }).catch($ => ({ deny: `${$.plugin.name}: show_task_report failed.` }))
}
