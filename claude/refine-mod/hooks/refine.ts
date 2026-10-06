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
