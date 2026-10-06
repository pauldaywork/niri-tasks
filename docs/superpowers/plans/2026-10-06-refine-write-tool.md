# Write a Refined Task Through a Mod Tool — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A refine session writes its approved plan through a `write_task_plan` tool served by a Claude Code mod, instead of the model pasting JSON into a `python3` heredoc.

**Architecture:** A TypeScript mod, `niri-tasks-refine`, lives in `claude/refine-mod/`. At session start it registers `write_task_plan` only if `--settings` gave it a valid task uuid and the Bash sandbox is on with `failIfUnavailable`. The tool exports that one task, refuses if the description or notes differ from what the skill read, and imports the new description and notes plus `+planned` through a fixed `task` argv with `rc.hooks=off`. `install.sh` links the folder to a fixed path; `refine.rs` puts the uuid in `--settings` `pluginConfigs` and refuses to launch without the mod; `herdr.rs` adds `--plugin-dir`. The `refine-task` skill's step 5 calls the tool, and `Bash(python3 *)` goes.

**Tech Stack:** Rust 2021 (serde_json), Claude Code 2.1.291 mods (TypeScript hooks module, `claude plugin validate` / `claude plugin test`), Taskwarrior 2.6.2, bash.

**Spec:** Taskwarrior task `5ff28e6f-a47e-4999-8789-892b397e6619` (read it with `task rc.json.array=on 5ff28e6f export`; its notes are the spec) and `docs/adr/0002-refine-keeps-its-fence-mod-takes-the-write.md`, section "A — A registered `write_task_plan` tool" and "Consequences". Every file below was prototyped and verified on this machine first (Claude Code 2.1.291): `claude plugin validate` clean, `claude plugin test` 18/18, and a live `claude -p --plugin-dir` run against a throwaway Taskwarrior store wrote, refused a stale write, and left the real `~/.task` untouched.

## Global Constraints

- Plugin name, exactly: `niri-tasks-refine`. The tool's listed name, exactly: `mcp__niri-tasks-refine__write_task_plan` (hyphens kept; verified live).
- The task's argv, exactly: `["task", "rc.hooks=off", "rc.json.array=on", <uuid>, "export"]`, then `["task", "rc.hooks=off", "rc.verbose=nothing", "import"]` with the task JSON on stdin. No shell. `rc.hooks=off` on both, because a refine session can write `~/.task/hooks` (ADR 0002; task `b268d068`).
- The task uuid reaches the mod only through `--settings` `pluginConfigs["niri-tasks-refine"].options.uuid`, never from the model.
- The mod registers no tool unless the uuid is a uuid and the merged settings have `sandbox.enabled === true` and `sandbox.failIfUnavailable === true`.
- The tool takes `expected: { description, notes }` (what the skill read in step 1) and refuses, writing nothing, if the task is gone, not pending, or its description or notes differ.
- The tool writes only `description`, `annotations` (replaced, one per note, one second apart) and adds `planned` to `tags`; every other field is passed back as exported.
- The mod's fixed install path: `${XDG_DATA_HOME:-$HOME/.local/share}/niri-tasks/refine-mod` (an absolute `XDG_DATA_HOME` only, per the XDG spec). Never under `~/.claude/skills` — a plugin there loads in every session.
- `refine::launch` refuses to start, before anything opens, if the mod folder has no `.claude-plugin/plugin.json`, saying to run `install.sh`.
- `Bash(python3 *)` leaves the allow list; `Bash(task *)`, `WebSearch`, `WebFetch` and `allowWrite` on the task data stay.
- Out of scope: the approval change (`$.ui.ask`, task `dc32d340`), the hooks `denyWrite` (task `b268d068`), Start working.
- Match the surrounding code's comment style: a doc comment on every public Rust item saying *why*. TypeScript comments in the same plain voice.
- Commits: Conventional Commits, `feat(refine): …` (or `docs(…)`), body wrapped at 72, ending `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Land the branch with the `finish-worktree` skill.

## Facts the implementer needs (verified)

- A mod is a folder with `.claude-plugin/plugin.json`, `hooks/hooks.json` (`{"modules": ["./refine.ts"]}`, paths relative to `hooks/`), and the module exporting `register(on, options)`. `options` carries the manifest's `userConfig` values; a `required` field with no value fails the load, so `uuid` is `required: false, default: ""`.
- On a `--plugin-dir` load the engine lays its typings into `<folder>/.claude-plugin/types/` with its own `.gitignore` of `*`. Never commit that folder.
- `$.tool.register(spec)` from `session.start` lists the tool by turn one. The tool is *deferred*: the model finds it through ToolSearch, so the skill names its full name.
- A `tool.call` hook for a tool the plugin serves never calls `next` (nothing beneath answers). Success is `{ result: string }`; any refusal is `{ deny: string }`, which the model receives as an error result. The guard is `.catch($ => ({ deny: … }))`.
- `$.process.run(argv, { stdin })` runs with no shell, as the user, **outside** the Bash sandbox, with Claude's environment (so `TASKRC`/`TASKDATA` are honoured). It resolves `{ exitCode, stdout, stderr, … }` for any exit code.
- `task … export` of an unknown uuid prints `[]` and exits 0. `task import` of a known uuid replaces its annotations. Taskwarrior keys an annotation by its entry second, so notes must be spaced one second apart.
- In `claude plugin test`, the test's own `on` hooks sit beneath the plugin and must answer every engine op it touches: `session.start`, `tool.register`, `settings.read`, `process.run`, `clock.now` (`mock.clock`). Assert on the plugin's own answer (`result`/`deny`), not `text`/`isError`.
- Run `claude -p` with `< /dev/null`, or it waits 3s for stdin.

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `claude/refine-mod/.claude-plugin/plugin.json` | Create | manifest, `uuid` user option |
| `claude/refine-mod/hooks/hooks.json` | Create | names the module |
| `claude/refine-mod/hooks/plan.ts` | Create | pure parts: input checks, stale check, merge |
| `claude/refine-mod/hooks/refine.ts` | Create | `register`: arm at session start, serve the tool |
| `claude/refine-mod/tests/plan.test.ts` | Create | unit tests for `plan.ts` |
| `claude/refine-mod/tests/refine.test.ts` | Create | engine tests through `claude-code/testing` |
| `claude/refine-mod/tsconfig.json` | Create | type-check against the engine's laid-down typings |
| `src/refine.rs` | Modify | `REFINE_MOD`, `refine_mod_dir`, uuid in `session_settings`, no `python3` allow, launch check |
| `src/herdr.rs` | Modify | `--plugin-dir` in `agent_start_claude_refiner` |
| `install.sh` | Modify | link the mod folder to its fixed path |
| `.claude/skills/refine-task/SKILL.md` | Modify | step 5 calls the tool |
| `README.md` | Modify | the install line names the mod |

---

### Task 1: The `niri-tasks-refine` mod

**Files:**
- Create: every `claude/refine-mod/…` file in the table above.

**Interfaces:**
- Consumes: nothing.
- Produces: a plugin folder named `niri-tasks-refine` whose tool `mcp__niri-tasks-refine__write_task_plan` takes `{ expected: { description: string, notes: string[] }, description: string, notes: string[] }`, armed by the user option `uuid`. Task 2 passes `--plugin-dir <folder>` and `pluginConfigs["niri-tasks-refine"].options.uuid`; Task 3 links the folder and calls the tool from the skill.

- [ ] **Step 1: Write the manifest and the hooks list**

`claude/refine-mod/.claude-plugin/plugin.json`:

```json
{
  "name": "niri-tasks-refine",
  "version": "0.1.0",
  "description": "Gives a refine session one tool, write_task_plan, that rewrites one Taskwarrior task's description and notes.",
  "author": {
    "name": "Paul Day"
  },
  "userConfig": {
    "uuid": {
      "type": "string",
      "title": "Task UUID",
      "description": "The Taskwarrior task this session may rewrite. Unset, the tool is not offered.",
      "required": false,
      "default": ""
    }
  }
}
```

`claude/refine-mod/hooks/hooks.json`:

```json
{
  "modules": ["./refine.ts"]
}
```

`claude/refine-mod/tsconfig.json`:

```json
{
  "extends": "./.claude-plugin/types/tsconfig.json"
}
```

- [ ] **Step 2: Write the failing unit tests**

`claude/refine-mod/tests/plan.test.ts`:

```ts
import { describe, expect, test } from 'claude-code/testing'
import { isSandboxed, isUuid, merge, parseExport, parseInput, refusal, taskDate } from '../hooks/plan'
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
```

- [ ] **Step 3: Run them to see them fail**

Run: `claude plugin test claude/refine-mod`
Expected: FAIL — `../hooks/plan` cannot be found.

- [ ] **Step 4: Write `plan.ts`**

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
```

- [ ] **Step 5: Write the failing engine tests**

`claude/refine-mod/tests/refine.test.ts`:

```ts
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
```

- [ ] **Step 6: Run them to see the engine tests fail**

Run: `claude plugin test claude/refine-mod`
Expected: the 11 `plan.test.ts` tests pass; `refine.test.ts` fails (the module `./refine.ts` named in `hooks.json` does not exist).

- [ ] **Step 7: Write `refine.ts`**

`claude/refine-mod/hooks/refine.ts`:

```ts
import type { EngineInterface, Register } from 'claude-code'
import { isSandboxed, isUuid, merge, parseExport, parseInput, refusal } from './plan'

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

    const exported = await $.process.run([...TASK, 'rc.json.array=on', uuid, 'export'])
    if (exported.exitCode !== 0) {
      return { deny: `task export failed (${exported.exitCode}): ${exported.stderr.trim()}` }
    }
    const task = parseExport(exported.stdout)
    const why = refusal(task, input.expected)
    if (why !== undefined || task === undefined) return { deny: `Nothing written: ${why}` }

    const planned = merge(task, input, await $.clock.now())
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

Import `./plan` with no extension: `./plan.ts` loads but fails `tsc` with TS5097.

- [ ] **Step 8: Run validate and the tests**

```bash
claude plugin validate claude/refine-mod
claude plugin test claude/refine-mod
```

Expected: validate prints `✔ Validation passed` (no warnings) and lists `hooks: session.start, tool.call{tool=mcp__niri-tasks-refine__write_task_plan}` and `gating hook with .catch: tool.call{…}`; test prints `18 pass`, `0 fail`.

- [ ] **Step 9: Live check against a throwaway store, and type-check**

This proves the listed tool name and the real `task` round trip, which the kit cannot. It never touches `~/.task`: `TASKRC` and `TASKDATA` point into a scratch folder, and the mod's `task` inherits them.

```bash
S=$(mktemp -d); mkdir -p "$S/data"
printf 'data.location=%s/data\nconfirmation=off\n' "$S" > "$S/taskrc"
export TASKRC="$S/taskrc" TASKDATA="$S/data"
task rc.hooks=off add "feat: Old words" +zeta
task rc.hooks=off 1 annotate "first note"
U=$(task rc.hooks=off _get 1.uuid)
stat -c '%Y %n' ~/.task/pending.data
(cd "$S" && claude -p --model haiku --plugin-dir "$OLDPWD/claude/refine-mod" \
  --settings "{\"sandbox\":{\"enabled\":true,\"failIfUnavailable\":true},\"pluginConfigs\":{\"niri-tasks-refine\":{\"options\":{\"uuid\":\"$U\"}}}}" \
  --allowedTools mcp__niri-tasks-refine__write_task_plan \
  'Call mcp__niri-tasks-refine__write_task_plan exactly once with expected = {"description":"feat: Old words","notes":["first note"]}, description = "feat: New words", notes = ["step one","step two"]. Reply with the tool result verbatim.' < /dev/null)
task rc.hooks=off rc.json.array=on "$U" export
stat -c '%Y %n' ~/.task/pending.data
unset TASKRC TASKDATA
npx -p typescript@5 tsc -p claude/refine-mod/.claude-plugin/types
git status --short claude/refine-mod
```

Expected: Claude replies `Wrote the plan to task <U>: 2 note(s), tagged planned.`; the export shows description `feat: New words`, two annotations one second apart, tags `["planned","zeta"]`; both `stat` lines are identical; `tsc` prints nothing; `git status` lists no `.claude-plugin/types/` files (its own `.gitignore` hides it). If `npx` is unavailable offline, say so in the report rather than skipping silently.

- [ ] **Step 10: Commit**

```bash
git add claude/refine-mod
git commit -m "$(cat <<'EOF'
feat(refine): add a mod that writes a refined task through a tool

The niri-tasks-refine mod registers write_task_plan when the session
was given a task uuid in its settings and runs with an enforced
sandbox. The tool refuses if the task changed since the skill read it,
and writes only the description, the notes and +planned, through a
fixed task argv with hooks off.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Refine launches Claude with the mod

**Files:**
- Modify: `src/refine.rs` (the `CREDENTIALS` block ends at line 83; `session_settings` at 85-126; `launch` at 298-344; tests from line 395)
- Modify: `src/herdr.rs` (`agent_start_claude_refiner` at 95-106; its test `claude_starts_unable_to_edit_files_or_plan` at ~307-322)

**Interfaces:**
- Consumes: Task 1's plugin name `niri-tasks-refine` and its user option `uuid`.
- Produces: `pub const REFINE_MOD: &str`, `pub fn refine_mod_dir(home: &Path, xdg_data_home: Option<&Path>) -> PathBuf`, `session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str) -> String`, `herdr::agent_start_claude_refiner(session: &str, name: &str, pane: &str, settings: &str, mod_dir: &Path) -> Vec<String>`. Task 3's `install.sh` links to the path `refine_mod_dir` returns.

- [ ] **Step 1: Write the failing tests**

In `src/refine.rs`'s `mod tests`, replace `the_session_may_search_the_web_but_not_read_credentials` and add three tests:

```rust
    /// The web and the skill's reads run unasked; credentials cannot be read
    /// through the Read tool, the one reader the Bash sandbox does not cover.
    /// No `python3`: the write is the mod's tool now.
    #[test]
    fn the_session_may_search_the_web_but_not_read_credentials() {
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], "u");
        let v: Value = serde_json::from_str(&json).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        for tool in ["WebSearch", "WebFetch", "Bash(task *)"] {
            assert!(allow.iter().any(|a| a == tool), "{tool} allowed");
        }
        assert!(!allow.iter().any(|a| a.as_str().is_some_and(|s| s.contains("python3"))));
        let deny = v["permissions"]["deny"].as_array().unwrap();
        assert!(deny.iter().any(|d| d == "Read(~/.ssh/**)"));
        assert!(deny.iter().any(|d| d == "Read(~/.claude/.credentials.json)"));
        assert_eq!(deny.len(), CREDENTIALS.len());
    }

    /// The task the mod may write comes from these settings, never from the
    /// model.
    #[test]
    fn the_mod_is_told_which_task_it_may_write() {
        let u = "d9f76b94-e0ff-44df-85b4-060be4219169";
        let json = session_settings(Path::new("/p"), Path::new("/t"), &[], u);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["uuid"], u);
        assert_eq!(REFINE_MOD, "niri-tasks-refine", "the name in claude/refine-mod's plugin.json");
    }

    /// Where install.sh links the mod: under the XDG data folder, never under
    /// ~/.claude/skills, where a plugin would load in every session.
    #[test]
    fn the_mod_lives_under_the_xdg_data_folder() {
        let home = Path::new("/home/x");
        assert_eq!(refine_mod_dir(home, None), PathBuf::from("/home/x/.local/share/niri-tasks/refine-mod"));
        assert_eq!(
            refine_mod_dir(home, Some(Path::new("/data"))),
            PathBuf::from("/data/niri-tasks/refine-mod")
        );
        // The XDG spec says a relative path is to be ignored.
        assert_eq!(
            refine_mod_dir(home, Some(Path::new("rel"))),
            PathBuf::from("/home/x/.local/share/niri-tasks/refine-mod")
        );
    }
```

and change the call in `the_sandbox_fences_the_session_to_the_task_database` to pass a fourth argument:

```rust
        let json = session_settings(Path::new("/home/x/Projects/alpha"), Path::new("/home/x/.task"), &hidden, "u");
```

In `src/herdr.rs`'s tests, replace `claude_starts_unable_to_edit_files_or_plan`:

```rust
    /// Claude's own arguments go after `--`, which is how herdr tells them
    /// from its own. No plan mode, and no tools that edit files: see
    /// [`agent_start_claude_refiner`] for why. The mod comes in by folder.
    #[test]
    fn claude_starts_unable_to_edit_files_or_plan() {
        assert_eq!(
            agent_start_claude_refiner("alpha", "task-0123abcd", "w1:p3", "{\"sandbox\":{}}", Path::new("/m/refine-mod")),
            vec![
                "herdr", "--session", "alpha", "agent", "start", "task-0123abcd",
                "--kind", "claude", "--pane", "w1:p3", "--timeout", "60000",
                "--", "--permission-mode", "default",
                "--disallowedTools", "Edit", "Write", "NotebookEdit", "EnterPlanMode", "ExitPlanMode",
                "--append-system-prompt", REFINER_SYSTEM_PROMPT,
                "--settings", "{\"sandbox\":{}}",
                "--plugin-dir", "/m/refine-mod",
            ]
        );
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib refine:: herdr::`
Expected: compile errors — `session_settings` takes 3 arguments, `REFINE_MOD` and `refine_mod_dir` not found, `agent_start_claude_refiner` takes 4 arguments.

- [ ] **Step 3: Implement in `src/refine.rs`**

After `CREDENTIALS` (line 83), add:

```rust
/// The refine mod's plugin name, as `claude/refine-mod/.claude-plugin/plugin.json`
/// declares it: the key its options travel under in `--settings`.
pub const REFINE_MOD: &str = "niri-tasks-refine";

/// Where `install.sh` links the refine mod's folder:
/// `$XDG_DATA_HOME/niri-tasks/refine-mod`, else under `~/.local/share`.
///
/// A fixed path because `niritasks` is `cargo install`ed and does not know
/// where the repo is. Never under `~/.claude/skills`: a plugin there loads in
/// every Claude session, not just a refine. A relative `XDG_DATA_HOME` is
/// ignored, as the XDG spec says.
pub fn refine_mod_dir(home: &Path, xdg_data_home: Option<&Path>) -> PathBuf {
    xdg_data_home
        .filter(|p| p.is_absolute())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("niri-tasks/refine-mod")
}
```

Change `session_settings`: its doc's third paragraph and its signature and JSON. The paragraph that begins "Allowed unasked on top" becomes:

```rust
/// Allowed unasked on top: web search and fetch, which write nothing; and
/// `task`, which Claude Code asks about even inside the sandbox, for reasons
/// it does not log — the skill reads with it, and it still runs sandboxed.
/// The write itself is the refine mod's tool, which `pluginConfigs` tells
/// which task it may write: `uuid`, never anything the model says.
```

and the function:

```rust
pub fn session_settings(project: &Path, task_data: &Path, hidden: &[PathBuf], uuid: &str) -> String {
    let deny: Vec<String> = CREDENTIALS.iter().map(|c| format!("Read({c})")).collect();
    serde_json::json!({
        "permissions": {
            "allow": ["WebSearch", "WebFetch", "Bash(task *)"],
            "deny": deny,
        },
        "sandbox": {
            "enabled": true,
            "autoAllowBashIfSandboxed": true,
            "allowUnsandboxedCommands": false,
            "failIfUnavailable": true,
            "network": { "allowAllUnixSockets": true },
            "filesystem": {
                "denyWrite": [project],
                "allowWrite": [task_data],
                "denyRead": hidden,
            },
        },
        "pluginConfigs": {
            (REFINE_MOD): { "options": { "uuid": uuid } },
        },
    })
    .to_string()
}
```

In `launch`, replace the block from `// Before anything opens` through `let settings = …;` with:

```rust
    // Before anything opens: a refused refine should leave nothing behind.
    let hidden = hidden_paths(home);
    ensure_no_exposed_sockets(&hidden)?;
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let mod_dir = refine_mod_dir(home, xdg.as_deref());
    // Without the mod the session would have no way to write its plan, and
    // the user would find that out only at the end of the interview.
    anyhow::ensure!(
        mod_dir.join(".claude-plugin/plugin.json").is_file(),
        "The refine mod is not installed at {}. Run install.sh from the niri-tasks repo.",
        mod_dir.display()
    );
    let settings = session_settings(&dir, &task::data_location()?, &hidden, &t.uuid);
```

and pass it on:

```rust
    if let Err(e) = herdr::run(&herdr::agent_start_claude_refiner(&s, &name, &pane, &settings, &mod_dir)) {
```

- [ ] **Step 4: Implement in `src/herdr.rs`**

Add a sentence to `agent_start_claude_refiner`'s doc, after the paragraph about `settings`:

```rust
/// `mod_dir` is the refine mod (`refine::refine_mod_dir`), loaded for this
/// session only: it serves the one tool the skill writes the task with.
```

and change the function:

```rust
pub fn agent_start_claude_refiner(session: &str, name: &str, pane: &str, settings: &str, mod_dir: &Path) -> Vec<String> {
    let mod_dir = mod_dir.to_string_lossy();
    cmd(
        session,
        &[
            "agent", "start", name, "--kind", "claude", "--pane", pane, "--timeout", "60000",
            "--", "--permission-mode", "default",
            "--disallowedTools", "Edit", "Write", "NotebookEdit", "EnterPlanMode", "ExitPlanMode",
            "--append-system-prompt", REFINER_SYSTEM_PROMPT,
            "--settings", settings,
            "--plugin-dir", &mod_dir,
        ],
    )
}
```

`herdr.rs` already has `use std::path::Path;`.

- [ ] **Step 5: Run the tests**

Run: `cargo test`
Expected: all pass, including the four tests above; no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/refine.rs src/herdr.rs
git commit -m "$(cat <<'EOF'
feat(refine): start the refine session with the write mod

Refine passes the task uuid to the niri-tasks-refine mod in
pluginConfigs and loads the mod with --plugin-dir from its fixed
install path, refusing to start when it is not installed. The
python3 allow goes: the mod's tool is the write now.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Install the mod and write through it from the skill

**Files:**
- Modify: `install.sh` (section 4, the Claude Code skills, lines 68-92)
- Modify: `.claude/skills/refine-task/SKILL.md` (§5, "Write", lines 105-134)
- Modify: `README.md` (the install paragraph at line 127)

**Interfaces:**
- Consumes: Task 1's tool name `mcp__niri-tasks-refine__write_task_plan` and its input; Task 2's path `${XDG_DATA_HOME:-$HOME/.local/share}/niri-tasks/refine-mod`.
- Produces: an installed mod, and a skill that writes through it.

- [ ] **Step 1: Link the mod in `install.sh`**

After the `finish-worktree` link and before the rules link, add:

```bash
# The refine mod: the one tool a Refine session writes its task with. A
# folder, loaded by `niritasks task refine` with --plugin-dir from this fixed
# path, since the installed binary does not know where the repo is. Not under
# ~/.claude/skills, where a plugin would load in every session. The XDG rule
# matches refine::refine_mod_dir: a relative XDG_DATA_HOME is ignored.
DATA="${XDG_DATA_HOME:-}"
case "$DATA" in /*) ;; *) DATA="$HOME/.local/share" ;; esac
link "$REPO/claude/refine-mod" "$DATA/niri-tasks/refine-mod"
```

- [ ] **Step 2: Run it and check the link loads**

```bash
bash install.sh
ls -l "${XDG_DATA_HOME:-$HOME/.local/share}/niri-tasks/refine-mod"
claude plugin validate "${XDG_DATA_HOME:-$HOME/.local/share}/niri-tasks/refine-mod"
```

Expected: install prints `Linked …/niri-tasks/refine-mod` (silent on a re-run); `ls -l` shows the symlink into the repo; validate passes through the link. `install.sh` also restarts the daemon and rebuilds the binary; that is expected.

- [ ] **Step 3: Rewrite the skill's step 5**

Replace §5 in `.claude/skills/refine-task/SKILL.md`, from `## 5. Write` up to (not including) `## 6. Verify and report`, with:

````markdown
## 5. Write

Write with the `mcp__niri-tasks-refine__write_task_plan` tool — the session's
one way to change the task. It is deferred: select it with ToolSearch
(`select:mcp__niri-tasks-refine__write_task_plan`) if it is not loaded. Call
it once, with:

- `expected`: the `description` and the annotations' `description`s, in
  order, exactly as you read them in step 1;
- `description` and `notes`: the **Will be written to the task** block the
  user approved, exactly as printed — no rewording, no reordering, nothing
  added. If anything needs to change after approval, go back to step 4 and
  print the block again instead.

The tool writes the description, replaces the notes and adds `+planned` in
one import, and touches nothing else. It refuses, writing nothing, if the
task changed since step 1: show the user what it says changed and ask before
going on. If the tool is missing, say so and stop — do not write the task
any other way.
````

Also in the skill's opening section, the sentence "Your one write is the Taskwarrior update in step 5" stays true; leave it.

- [ ] **Step 4: Update the README install line**

In `README.md` line 127, after `the Claude skills (`workspace-tasks`, `refine-task`, `finish-worktree`) into `~/.claude/skills`,` add ` the refine mod (`claude/refine-mod`, which Refine's session writes the task with) into `~/.local/share/niri-tasks/refine-mod`,`. Keep the sentence's existing wrap style.

- [ ] **Step 5: End to end through Refine**

This is the spec's "Refine loads the mod, and the approved write goes through the tool". It needs the user's screen, so it is a manual check: ask the user to run it, or run it yourself only if you are in their niri session.

1. `niritasks task add 'chore: Try the refine write tool'`, and note its uuid.
2. `niritasks task refine <uuid>`. In the new herdr tab, confirm the session's tool list has `mcp__niri-tasks-refine__write_task_plan` (ask Claude "is write_task_plan available?").
3. Let the skill draft, answer **Write it to the task**, and watch it call the tool (not `python3`).
4. `task rc.json.array=on <uuid> export`: the new description and notes are there with `planned` in `tags`.
5. Delete the throwaway task: `niritasks task status <uuid> deleted --yes` (or Remove in the panel).

Record the outcome in the report. If the session started with no tool, check `claude --debug` output in the pane for the plugin load error.

- [ ] **Step 6: Commit**

```bash
git add install.sh .claude/skills/refine-task/SKILL.md README.md
git commit -m "$(cat <<'EOF'
feat(refine): write the refined task through the mod's tool

install.sh links the refine mod to its fixed path, and the
refine-task skill's step 5 calls write_task_plan with what it read in
step 1 and the approved block, instead of a python3 heredoc.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```
