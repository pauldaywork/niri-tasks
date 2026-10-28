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
import { branchPage } from './branch-page'
import {
  branchArgv,
  branchTarget,
  countStatus,
  gitCommand,
  logArgv,
  mergeBaseArgv,
  numstatArgv,
  parseLog,
  parseNumstat,
  statusArgv,
  withGit,
} from './branch'
import type { Git, Target } from './branch'
import { FONT_FILES } from './style'
import type { Fonts } from './style'
import { markdownPath, twinOf } from './markdown'
import {
  BRANCH_LENSES,
  BRANCH_REPORT_NOTE,
  REPORT_NOTE,
  checkSuite,
  homeDir,
  madeAlready,
  notePath,
  openArgv,
  overLimits,
  parseReport,
  parseTestRun,
  reportPath,
  reportsDir,
  unexplainable,
  unreportable,
  withReportNote,
  withReportNotes,
} from './report'
import type { MadeReport, Parsed, Report, ReportKind, TestRun } from './report'

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

const ITEMS = (properties: Record<string, unknown>, required: string[]) => ({
  type: 'array',
  items: { type: 'object', properties, required },
})
const COUNT = { type: 'integer', minimum: 0 }

// A branch report's schema: the plan report's, bar needs_your_eye and
// checks, with no plain part, and the branch's own fields.
const branchSpec = () => {
  const { checks: _checks, needs_your_eye: _eye, sections, title, ...shared } = reportSpec(true).inputSchema.properties
  return {
    name: 'show_task_report',
    description:
      "Writes an HTML report of what this task's worktree branch has changed so far, how it works and how it is " +
      'tested, in one fixed, easy-to-read layout, opens it in the browser and links it from the task as a ' +
      "Branch report: <path> note. Fill the fields as the report-branch skill's branch-catalogue.md says. Name " +
      'commits by sha alone: the tool reads each subject, and the branch, its base and its numbers, from git. ' +
      "When the session's prompt names a test run, the suite comes from its log, and the tool checks it against " +
      "the run's exit code. " +
      'Every text field is one line of plain text; a report over the limits is refused with every reason at once. ' +
      'One report a session.',
    inputSchema: {
      type: 'object',
      properties: {
        ...shared,
        title: { ...TEXT, description: "The task's description." },
        sections: {
          ...sections,
          description:
            '1-9 parts, each looking at the branch through one lens (kind, always given): before-after, structure, ' +
            'data-flow, outside-tools, styling, code, newcomer.',
          items: {
            ...sections.items,
            properties: { ...sections.items.properties, kind: { type: 'string', enum: [...BRANCH_LENSES] } },
            required: ['kind', 'heading', 'points'],
          },
        },
        hot_spots: {
          ...ITEMS({ spot: TEXT, why: TEXT, files: TEXTS }, ['spot', 'why', 'files']),
          description: '1-4 places a reviewer should look hardest: what, why it is risky, and its files.',
        },
        story: {
          ...ITEMS({ step: TEXT, commits: TEXTS, points: TEXTS }, ['step', 'commits', 'points']),
          description: '1-8 steps the commits fall into, oldest first; every commit on the branch in exactly one step, named by sha.',
        },
        map: {
          ...ITEMS(
            {
              path: TEXT,
              change: { type: 'string', enum: ['new', 'change', 'remove'] },
              items: ITEMS({ name: TEXT, does: TEXT }, ['name', 'does']),
            },
            ['path', 'change', 'items'],
          ),
          description: 'Each changed file worth naming, and each function or part of it the branch changes, with what it now does.',
        },
        suite: {
          type: 'object',
          description: 'The whole test suite as you ran it: each suite with its counts, each not run and why, each failing test.',
          properties: {
            runs: ITEMS({ suite: TEXT, command: TEXT, passed: COUNT, failed: COUNT }, ['suite', 'command', 'passed', 'failed']),
            not_run: ITEMS({ suite: TEXT, why: TEXT }, ['suite', 'why']),
            failing: ITEMS({ test: TEXT, file: TEXT, why: TEXT }, ['test', 'file', 'why']),
          },
          required: ['runs'],
        },
        new_tests: {
          ...ITEMS(
            {
              test: TEXT,
              file: TEXT,
              layer: { type: 'string', enum: ['unit', 'integration', 'e2e'] },
              pins: TEXT,
              result: { type: 'string', enum: ['pass', 'fail', 'not run'] },
            },
            ['test', 'file', 'layer', 'pins', 'result'],
          ),
          description: "The tests this branch adds or changes: each one's layer, the behaviour it pins, and its result.",
        },
        untested: {
          ...ITEMS({ code: TEXT, file: TEXT, why: TEXT }, ['code', 'file', 'why']),
          description: 'Changed code no test covers, and why that matters.',
        },
        plan_vs_done: {
          ...ITEMS(
            {
              item: TEXT,
              kind: { type: 'string', enum: ['step', 'done when'] },
              progress: { type: 'string', enum: ['done', 'partial', 'not yet'] },
              shown_by: TEXT,
            },
            ['item', 'kind', 'progress', 'shown_by'],
          ),
          description: "Each of the task's Steps and Done when notes: done, partial or not yet, and the commit or test that shows it.",
        },
      },
      required: ['title', 'summary', 'changes', 'sections', 'files', 'hot_spots', 'story', 'map', 'suite', 'new_tests'],
    },
  }
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

// The branch's facts, read from the worktree with git, outside the
// sandbox: read-only commands (branch.ts). The refusal names the command
// that failed and git's own error.
const readGit = async ($: EngineInterface, target: Target): Promise<Git | string> => {
  const run = async (argv: string[]): Promise<string> => {
    const answer = await $.process.run(argv)
    if (answer.exitCode !== 0) {
      throw new Error(`${gitCommand(argv)} failed (${answer.exitCode}): ${answer.stderr.trim()}`)
    }
    return answer.stdout
  }
  try {
    const branch = (await run(branchArgv(target.worktree))).trim()
    const mergeBase = (await run(mergeBaseArgv(target.worktree, target.base))).trim()
    const log = parseLog(await run(logArgv(target.worktree, mergeBase)))
    const stat = parseNumstat(await run(numstatArgv(target.worktree, mergeBase)))
    const uncommitted = countStatus(await run(statusArgv(target.worktree)))
    return { branch, base: target.base, merge_base: mergeBase, log, ...stat, uncommitted }
  } catch (error) {
    return error instanceof Error ? error.message : String(error)
  }
}

// The project's tests as Rust ran them before the session, outside the
// sandbox, read from the results file it wrote to the reports folder,
// which the sandbox can read but not write; or why it cannot be read.
const readTestRun = async ($: EngineInterface, path: string): Promise<TestRun | string> => {
  let text: string
  try {
    text = await $.fs.read(path)
  } catch (error) {
    return error instanceof Error ? error.message : String(error)
  }
  return parseTestRun(text)
}

// A branch report's fields, each commit named by git and the branch's facts
// read from it, and its suite held to the test run when Rust ran one; or
// the refusal to answer the call with.
const withBranch = async (
  $: EngineInterface,
  parsed: Parsed,
  target: Target,
  tests: string | undefined,
): Promise<Report | { deny: string }> => {
  if (parsed.branch === undefined) return { deny: 'show_task_report: nothing written: the report has no branch fields.' }
  const git = await readGit($, target)
  if (typeof git === 'string') return { deny: `show_task_report: nothing written: ${git}.` }
  const branch = withGit(parsed.branch, git)
  if (typeof branch === 'string') return { deny: `show_task_report: ${branch}` }
  if (tests === undefined) return { ...parsed.report, branch }
  const run = await readTestRun($, tests)
  if (typeof run === 'string') return { deny: `show_task_report: nothing written: could not read the test run ${tests}: ${run}` }
  const disagree = checkSuite(branch.suite, run)
  if (disagree.length > 0) return { deny: `show_task_report: ${disagree.join('; ')}` }
  const { command, exit_code, log } = run
  return { ...parsed.report, branch: { ...branch, run: { command, exit_code, log } } }
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

// A report session's settings, and the report it has made: one a session,
// as a refine makes one per ask. Module state, like `askedFor`.
type ReportSession = {
  kind: ReportKind
  // The worktree and base a branch report reads; undefined in a plan's.
  target: Target | undefined
  // The results file of the tests Rust ran before a branch session, which
  // its suite is held to; undefined when none ran, or in a plan's.
  tests: string | undefined
  home: string | undefined
  unattended: boolean
  fonts: { read?: Fonts }
  linked?: string
}

// A report session's one report, of the task's plan or of its branch: the
// task read as it is now, the fields checked, for a branch git read for
// its facts and its commits' subjects, the deck built from its twin,
// opened, and linked from the task: a plan's as a Report: note, a
// branch's as a Branch report: note, which leaves the plan's alone.
const reportSession = async (
  $: EngineInterface,
  at: ReportSession,
  e: Record<string, unknown>,
  uuid: string,
  folder: string,
): Promise<{ deny: string } | { result: string }> => {
  const { kind, target, tests, home, unattended, fonts } = at
  if (at.linked !== undefined) return { deny: madeAlready(at.linked, kind) }
  // The plan is the task as it is now: read here, not at session start,
  // so an edit made since the button was pressed is what is reported.
  const exported = await $.process.run([...TASK, 'rc.json.array=on', uuid, 'export'])
  if (exported.exitCode !== 0) {
    return { deny: `task export failed (${exported.exitCode}): ${exported.stderr.trim()}` }
  }
  const task = parseExport(exported.stdout)
  const why = kind === 'branch' ? unexplainable(task) : unreportable(task)
  if (why !== undefined || task === undefined) return { deny: `show_task_report: nothing written: ${why}.` }

  const parsed = parseReport(e, kind)
  if (typeof parsed === 'string') return { deny: `show_task_report: ${parsed}` }
  let report = parsed.report
  if (kind === 'branch') {
    if (target === undefined) return { deny: 'show_task_report is not armed in this session.' }
    const branched = await withBranch($, parsed, target, tests)
    if ('deny' in branched) return branched
    report = branched
  }

  const shown = { description: task.description, notes: notesOf(task) }
  // The page is built from the report's Markdown twin as it reads back,
  // never from the fields beside it, so the two cannot disagree.
  const twin = twinOf(report, shown)
  if (typeof twin === 'string') {
    return {
      deny:
        `show_task_report: nothing written: ${twin}. The task's own description and notes are reported ` +
        'as they are; one on several lines must be made one line first.',
    }
  }
  const path = reportPath(folder, uuid, await $.clock.now(), kind)
  // What the note and the answers name; the files are written and
  // opened by `path`.
  const note = notePath(path, home)
  const draw = kind === 'branch' ? branchPage : reportPage
  const page = draw(twin.report, twin.plan, await readFonts($, fonts), await drawDiagrams($, twin.report))
  await $.fs.write(path, page)
  // The twin itself, which herdr's task-docs plugin opens in
  // plannotator-tui so comments on it reach the task's Claude.
  await $.fs.write(markdownPath(path), twin.markdown)
  const label = kind === 'branch' ? BRANCH_REPORT_NOTE : REPORT_NOTE
  const named = kind === 'branch' ? 'branch report' : 'report'
  $.ui.log(`${label}${note}`)
  for (const warning of parsed.warnings) $.ui.log(`Over its word limit: ${warning}`)
  // Unattended, nothing opens: no one is at the screen, and the task's
  // note is how the report is found later.
  if (!unattended) await $.process.run(openArgv(path))
  const opened = unattended ? '' : ' and opened it'

  // The session's one write to the task, and the mod's, not the model's:
  // the note's path is the one just written.
  const imported = await $.process.run([...TASK, 'rc.verbose=nothing', 'import'], {
    stdin: JSON.stringify([withReportNote(task, note, await $.clock.now(), kind)]),
  })
  if (imported.exitCode !== 0) {
    return {
      deny:
        `Wrote the ${named} to ${note}${opened}, but task import failed (${imported.exitCode}): ` +
        `${imported.stderr.trim()} The task does not link it.`,
    }
  }
  at.linked = note
  return {
    result:
      (unattended
        ? `Wrote the ${named} to ${note} and linked it from the task as the note "${label}${note}".`
        : `Wrote the ${named} to ${note}, opened it in the browser, and linked it from the task ` +
          `as the note "${label}${note}".`) + overLimits(parsed.warnings),
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
  // Explain branch: a report session on what the task's worktree branch has
  // changed, not on its plan. Rust names the worktree and the branch it is
  // measured against; without both, no report tool is offered.
  const kind: ReportKind = reportOnly && options.branch === true ? 'branch' : 'plan'
  const target = kind === 'branch' ? branchTarget(options.worktree, options.base) : undefined
  // The tests Rust ran before a branch session, outside its sandbox: their
  // results file, read when the report is shown. Empty when none ran, and
  // then the session runs what it finds itself, as before.
  const tests = kind === 'branch' && typeof options.tests === 'string' && options.tests !== '' ? options.tests : undefined
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
  // A report session's state: its kind, and the report it has linked.
  const session: ReportSession = { kind, target, tests, home, unattended, fonts }

  on('session.start', async ($, e, next) => {
    if ((await armedUuid($, options.uuid)) !== undefined) {
      if (!reportOnly) await $.tool.register(SPEC)
      if (reports !== undefined && (kind === 'plan' || target !== undefined)) {
        await $.tool.register(kind === 'branch' ? branchSpec() : reportSpec(reportOnly))
      }
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
    if (reportOnly) return reportSession($, session, e, uuid, reports)
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
