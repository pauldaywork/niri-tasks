# Report What a Task's Branch Has Changed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A task card's Go to list gets **Explain branch**: a Claude in its own herdr tab reads the task's worktree branch against the default branch, runs the test suite, and writes an HTML deck (with its Markdown twin) explaining what the branch changed, how it works and how it is tested, linked from the task as `Branch report: <path>` and opened again by a **Branch report** row.

**Architecture:** The refine mod's `show_task_report` gets a branch variant. In a branch session (`report` and `branch` options set by Rust), the tool takes the plan report's fields minus `needs_your_eye` and `checks`, plus seven branch fields. The mod reads the branch's facts and each commit's subject from git itself, outside the sandbox, and checks the model's commit story against them. It writes the deck from the Markdown twin through the same renderer, styles, limits and reviews folder, in the branch's own slide order. Rust adds `Mode::Branch`, `niritasks task report <uuid> --branch [--fresh]`, the two Go to rows and the `Branch report:` note. A new `report-branch` skill and its catalogue tell Claude how to fill the fields.

**Tech Stack:** TypeScript Claude Code mod in `claude/refine-mod` (`claude plugin test .`, `claude plugin validate .`), Rust (clap, serde_json, anyhow; `cargo test`), Python 3 `unittest` (herdr task-docs plugin), git, Taskwarrior 3.5, herdr.

**Spec:** Taskwarrior task `b5447e66-d3fa-410a-b70e-99dcddf259f1`, "feat: Report what a task's branch has changed". Read it with `task rc.json.array=on b5447e66-d3fa-410a-b70e-99dcddf259f1 export`; its notes are the spec. The look was settled with the user on a mock deck of the peek branch (`7e677bd^..47c4989`), saved as `~/.local/share/niri-tasks/reviews/branch-report-mock-cbb49ff3-20261027.html`: open it to see every new slide as it must look.

## Global Constraints

- **Explain branch:** "A new Go to row, Explain branch, on any task with a worktree, starts a Claude in its own herdr tab like Generate report does; diff and log are taken against the merge-base with the default branch."
- **One renderer:** "show_task_report gets a branch variant with its own slide order, sharing the renderer, styles, limits, reviews folder and Markdown twin; refine reports stay as they are." A plan report's page, twin and answers must not change by one character.
- **Deck order:** "Cover (branch, base, commits, lines) - At a glance - Review hot spots - Words - Commit story - Before/after - Changes map - Structure - Data flow - Outside tools - Styling - Code - Tests - Newcomer - Plan vs done - Files - The plan". Agreed with the user on the mock: **Tests is two slides**, *Test suite* ("Did anything break?": every suite run, its counts, those not run and why) then *New tests* (each test the branch adds or changes, its layer, what it pins, its result, then changed code with no test).
- **Tests:** "Tests summarises the tests relevant to what the diff touches: each test's layer and the behaviour it pins, runs the cargo and bun ones (targets in $TMPDIR) with pass or fail, names e2e tests without running them, and lists changed code with no test." Agreed on the mock: the suite slide counts every suite that runs headless, Python's `unittest` included; a suite the sandbox stops is listed as not run, with why.
- **Plan vs done:** "Plan vs done replaces Done when: each of the task's Steps and Done when notes marked done, partial or not yet, with the commit or test that shows it."
- **The note:** "Link it as 'Branch report: <path>', not 'Report:', so it never turns the plan's report into an earlier draft; the Go to list gets its own row to open it."
- **Commit shas:** the model names commits by sha alone; the mod fills in each subject and the cover's numbers from git (agreed with the user).
- **Done when:** "Explain branch on a started task opens a deck with every branch slide filled from its commits and diff, its Tests slide shows the relevant tests and their results, the task carries a Branch report note the Go to list opens, and the plan's Report note is untouched."
- **Out of scope:** "Reports for branches no task owns; changing the refine report's deck; running the nested-niri e2e tests from the report."
- **Commits:** Conventional Commits, subject at most 72 characters, body wrapped at 72 saying what and why, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Tests stay green at every commit: `cargo test` checks that README's Commands block and llms.txt name every subcommand and flag, so `--branch`'s doc lines land in the same commit as the flag (Task 8).
- **Where things run:** a refine or report session loads skills from `~/Projects/niri-tasks` (linked by `install.sh`) and the mod from `~/.local/share/niri-tasks/refine-mod`, never from this worktree. The end-to-end check of Explain branch (Task 10, last step) can only run after the branch lands.

## File structure

| File | Change | Responsibility |
|---|---|---|
| `claude/refine-mod/hooks/report.ts` | modify | Branch types, `BRANCH_LIMITS`, `BRANCH_HEADINGS`, `parseReport(e, kind)`, note/path/answer helpers per kind |
| `claude/refine-mod/hooks/branch.ts` | create | Git argv builders and parsers, `Target`, `withGit` (shas → commits, coverage, map paths, facts) |
| `claude/refine-mod/hooks/markdown.ts` | modify | The branch twin: written and read back |
| `claude/refine-mod/hooks/page.ts` | modify | Export the shared pieces; `deck`, `needsBrowser`; branch prose in `readingMinutes` |
| `claude/refine-mod/hooks/branch-page.ts` | create | The branch deck's own slides and order |
| `claude/refine-mod/hooks/style.ts` | modify | The branch slides' CSS |
| `claude/refine-mod/hooks/refine.ts` | modify | Branch options, the branch tool spec, git reads, the branch call |
| `claude/refine-mod/.claude-plugin/plugin.json` | modify | `branch`, `worktree`, `base` options |
| `claude/refine-mod/tests/branch-fixtures.ts` | create | Small branch report and git fixtures |
| `claude/refine-mod/tests/branch.test.ts` | create | `branch.ts` |
| `claude/refine-mod/tests/example-branch-report.ts` | create | The catalogue's worked example (the peek branch) |
| `claude/refine-mod/tests/{report,markdown,page,refine}.test.ts` | modify | Branch cases |
| `.claude/skills/report-branch/SKILL.md` | create | The skill Explain branch prompts |
| `.claude/skills/report-branch/branch-catalogue.md` | create | How to fill the branch fields; worked example |
| `install.sh` | modify | Link the skill and catalogue |
| `src/task.rs` | modify | `Branch report:` note, `branch_report_path/link`, `ReportNote.branch` |
| `src/docs.rs` | modify | List a branch report as one |
| `src/actions.rs`, `src/panel/model.rs` | modify | `has_branch_report`, `Card.branch_report` |
| `src/names.rs` | modify | `branch_agent`, `Role::Branch` |
| `src/work.rs` | modify | `default_branch` |
| `src/refine.rs` | modify | `Mode::Branch`, `Branch`, `launch_branch`, settings options |
| `src/main.rs` | modify | `task report --branch` |
| `src/panel/goto.rs`, `src/panel/state.rs` | modify | Explain branch and Branch report rows |
| `herdr/task-docs/task_docs.py` (+ test) | modify | `branch-<uuid8>` agents are a task's |
| `CONTEXT.md`, `README.md`, `llms.txt` | modify | Docs |

## How to run the tests

- Mod: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15` (every file under `tests/`; there is no filter, so read the failures by name), and `claude plugin validate .` after touching `plugin.json` or `hooks/refine.ts`.
- Rust: `cargo test <name>` for one test, `cargo test` for all.
- herdr plugin: `python3 -m unittest discover -s herdr/task-docs/tests`.
- Everything this machine runs, from this worktree: `cargo build && NIRITASKS="$PWD/target/debug/niritasks" bash tests/all.sh`.

---

### Task 1: The branch fields and their limits

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts`
- Create: `claude/refine-mod/tests/branch-fixtures.ts`
- Test: `claude/refine-mod/tests/report.test.ts`

**Interfaces:**
- Produces (in `report.ts`):
  - `export type ReportKind = 'plan' | 'branch'`
  - `export const BRANCH_LENSES: readonly Lens[]` (LENSES without `part`)
  - `export type Layer = 'unit' | 'integration' | 'e2e'`, `Result = 'pass' | 'fail' | 'not run'`, `PlanKind = 'step' | 'done when'`, `Progress = 'done' | 'partial' | 'not yet'`, and `LAYERS`, `RESULTS`, `PLAN_KINDS`, `PROGRESS` as `readonly string[]`
  - `HotSpot { spot; why; files: string[] }`, `Commit { sha; subject }`, `Step<C> { step; commits: C[]; points: string[] }`, `MapItem { name; does }`, `MapEntry { path; change: Change; items: MapItem[] }`, `SuiteRun { suite; command; passed: number; failed: number }`, `NotRun { suite; why }`, `Failing { test; file; why }`, `Suite { runs; not_run; failing }`, `NewTest { test; file; layer; pins; result }`, `Untested { code; file; why }`, `PlanItem { item; kind; progress; shown_by }`, `Facts { branch; base; merge_base; commits; files; added; removed; uncommitted }`
  - `BranchInput { hot_spots; story: Step<string>[]; map; suite; new_tests; untested; plan_vs_done }`
  - `Branch = Omit<BranchInput, 'story'> & { facts: Facts; story: Step<Commit>[] }`
  - `Report` gains `branch?: Branch`
  - `BRANCH_LIMITS`, `BRANCH_HEADINGS`
  - `parseReport(e, kind: ReportKind = 'plan'): Parsed | string`, where `Parsed = { report: Report; warnings: string[]; branch?: BranchInput }` (`branch` present only for kind `'branch'`)

- [ ] **Step 1: Write the fixtures**

Create `claude/refine-mod/tests/branch-fixtures.ts`:

```ts
// The smallest branch report show_task_report takes, as the model gives it:
// its commits named by sha, which the git fixture (Task 2) knows.
export const BRANCH_MIN = {
  title: 'feat: New words',
  summary: 'The branch adds new words.',
  changes: ['Words: the description changes.'],
  sections: [{ kind: 'code', heading: 'The words in code', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  hot_spots: [{ spot: 'The words', why: 'They are new.', files: ['src/words.rs'] }],
  story: [{ step: 'Wrote the words', commits: ['abc1234', 'abd5678'], points: ['The words went in.'] }],
  map: [{ path: 'src/words.rs', change: 'change', items: [{ name: 'words()', does: 'Answers the words.' }] }],
  suite: { runs: [{ suite: 'cargo test', command: 'cargo test', passed: 3, failed: 0 }] },
  new_tests: [{ test: 'words_are_new', file: 'src/words.rs', layer: 'unit', pins: 'The words are new.', result: 'pass' }],
}
```

- [ ] **Step 2: Write the failing tests**

Add to `claude/refine-mod/tests/report.test.ts`, importing `BRANCH_MIN` from `./branch-fixtures`:

```ts
describe('parseReport for a branch', () => {
  test('takes the smallest branch report, its optional lists empty', () => {
    expect(parseReport(BRANCH_MIN, 'branch')).toEqual({
      report: {
        title: BRANCH_MIN.title,
        summary: BRANCH_MIN.summary,
        changes: BRANCH_MIN.changes,
        unchanged: [],
        needs_your_eye: [],
        terms: [],
        sections: BRANCH_MIN.sections,
        files: BRANCH_MIN.files,
        checks: [],
      },
      warnings: [],
      branch: {
        hot_spots: BRANCH_MIN.hot_spots,
        story: BRANCH_MIN.story,
        map: BRANCH_MIN.map,
        suite: { ...BRANCH_MIN.suite, not_run: [], failing: [] },
        new_tests: BRANCH_MIN.new_tests,
        untested: [],
        plan_vs_done: [],
      },
    })
  })

  test('refuses needs_your_eye and checks, naming what takes their place', () => {
    expect(parseReport({ ...BRANCH_MIN, needs_your_eye: ['x'], checks: [{ check: 'a', how: 'b' }] }, 'branch')).toBe(
      'needs_your_eye is not used in a branch report: hot_spots takes its place; ' +
        'checks is not used in a branch report: plan_vs_done takes its place',
    )
  })

  test('needs a lens on every part', () => {
    expect(parseReport({ ...BRANCH_MIN, sections: [{ heading: 'h', points: ['p'] }] }, 'branch')).toBe(
      'sections[0].kind is needed in a branch report: one of before-after, structure, data-flow, outside-tools, styling, code, newcomer',
    )
    expect(parseReport({ ...BRANCH_MIN, sections: [{ kind: 'part', heading: 'h', points: ['p'] }] }, 'branch')).toBe(
      'sections[0].kind is needed in a branch report: one of before-after, structure, data-flow, outside-tools, styling, code, newcomer',
    )
  })

  test('refuses a branch field in a plan report', () => {
    expect(parseReport({ ...MINIMAL, story: [] })).toBe('story is only for a branch report')
  })

  test('checks counts, layers, results, kinds and progress', () => {
    const [newTest] = BRANCH_MIN.new_tests
    expect(
      parseReport(
        {
          ...BRANCH_MIN,
          suite: { runs: [{ suite: 'cargo test', command: 'cargo test', passed: -1, failed: 1.5 }] },
          new_tests: [{ ...newTest, layer: 'system', result: 'skipped' }],
          plan_vs_done: [{ item: 'a', kind: 'goal', progress: 'half', shown_by: 'b' }],
        },
        'branch',
      ),
    ).toBe(
      'suite.runs[0].passed must be a whole number, 0 or more; ' +
        'suite.runs[0].failed must be a whole number, 0 or more; ' +
        'new_tests[0].layer must be unit, integration or e2e; ' +
        'new_tests[0].result must be pass, fail or not run; ' +
        'plan_vs_done[0].kind must be step or done when; ' +
        'plan_vs_done[0].progress must be done, partial or not yet',
    )
  })

  test('names a missing suite', () => {
    const { suite: _, ...rest } = BRANCH_MIN
    expect(parseReport(rest, 'branch')).toBe('suite must be an object; suite.runs must be an array')
  })

  test('holds the branch lists to their counts', () => {
    expect(parseReport({ ...BRANCH_MIN, hot_spots: [] }, 'branch')).toBe('hot_spots must have 1 to 4 items, not 0')
    expect(parseReport({ ...BRANCH_MIN, story: [] }, 'branch')).toBe('story must have 1 to 8 items, not 0')
  })
})
```

- [ ] **Step 3: Run them to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: the new `parseReport for a branch` tests fail (`parseReport` ignores its second argument, so the first answers without `branch` and the refusals never come); every other test passes.

- [ ] **Step 4: Add the types, limits and headings**

In `claude/refine-mod/hooks/report.ts`, after the `Report` type, add:

```ts
// Which report a session makes: of the task's plan, or of what its branch
// has changed so far.
export type ReportKind = 'plan' | 'branch'

// A branch report's parts each look through a named lens: it has no plain
// part, so its slides keep one order.
export const BRANCH_LENSES: readonly Lens[] = LENSES.filter(lens => lens !== 'part')

export type Layer = 'unit' | 'integration' | 'e2e'
export const LAYERS: readonly string[] = ['unit', 'integration', 'e2e']
export type Result = 'pass' | 'fail' | 'not run'
export const RESULTS: readonly string[] = ['pass', 'fail', 'not run']
export type PlanKind = 'step' | 'done when'
export const PLAN_KINDS: readonly string[] = ['step', 'done when']
export type Progress = 'done' | 'partial' | 'not yet'
export const PROGRESS: readonly string[] = ['done', 'partial', 'not yet']

// Where a reviewer should look hardest, and the files it is in.
export type HotSpot = { spot: string; why: string; files: string[] }
// A commit as the deck shows it: its short sha and its subject, from git.
export type Commit = { sha: string; subject: string }
// A step of the commit story: its commits, by sha as the model names them,
// or as commits once git has named them.
export type Step<C> = { step: string; commits: C[]; points: string[] }
export type MapItem = { name: string; does: string }
// A changed file, and what each function or part of it the branch changes
// now does.
export type MapEntry = { path: string; change: Change; items: MapItem[] }
export type SuiteRun = { suite: string; command: string; passed: number; failed: number }
export type NotRun = { suite: string; why: string }
export type Failing = { test: string; file: string; why: string }
// The whole test suite as the session ran it.
export type Suite = { runs: SuiteRun[]; not_run: NotRun[]; failing: Failing[] }
// A test the branch adds or changes.
export type NewTest = { test: string; file: string; layer: Layer; pins: string; result: Result }
export type Untested = { code: string; file: string; why: string }
// One of the task's Steps or Done when notes, and how far the branch has
// got with it.
export type PlanItem = { item: string; kind: PlanKind; progress: Progress; shown_by: string }
// What git says of the branch: read by the mod, never given by the model.
export type Facts = {
  branch: string
  base: string
  merge_base: string
  commits: number
  files: number
  added: number
  removed: number
  uncommitted: number
}

// A branch report's own fields, as the model gives them.
export type BranchInput = {
  hot_spots: HotSpot[]
  story: Step<string>[]
  map: MapEntry[]
  suite: Suite
  new_tests: NewTest[]
  untested: Untested[]
  plan_vs_done: PlanItem[]
}

// As a report holds them: each commit with its subject, and the branch's
// facts, both from git (branch.ts).
export type Branch = Omit<BranchInput, 'story'> & { facts: Facts; story: Step<Commit>[] }

// The branch fields' names, refused in a plan report.
const BRANCH_FIELDS = ['hot_spots', 'story', 'map', 'suite', 'new_tests', 'untested', 'plan_vs_done'] as const

// The branch slides' headings, the same in the page and its twin.
export const BRANCH_HEADINGS = {
  hotSpots: 'Where to look hardest',
  story: 'How the branch got here',
  map: 'What each file now does',
  suite: 'Did anything break?',
  newTests: 'The tests this branch adds',
  untested: 'Changed code with no test',
  planVsDone: 'Plan vs done',
} as const
```

Add `branch?: Branch` as the last field of `Report`:

```ts
export type Report = {
  title: string
  summary: string
  changes: string[]
  unchanged: string[]
  needs_your_eye: string[]
  terms: Term[]
  sections: Section[]
  files: FileChange[]
  checks: Check[]
  // A branch report's own parts; absent from a plan report.
  branch?: Branch
}
```

After `LIMITS`, add:

```ts
// What keeps a branch report's own slides easy to take in. A test list may
// run long: the reader asked for every test the branch adds.
export const BRANCH_LIMITS = {
  hotSpots: { min: 1, max: 4, spotWords: 10, whyWords: 30, files: { min: 1, max: 4 } },
  story: { min: 1, max: 8, stepWords: 10, shaChars: 40, commits: { min: 1, max: 60 }, points: { min: 1, max: 3, words: 25 } },
  map: { min: 1, max: 20, items: { min: 1, max: 6, nameChars: 80, words: 20 } },
  suite: {
    runs: { min: 1, max: 8, suiteWords: 6, commandChars: 200 },
    notRun: { min: 0, max: 6, words: 25 },
    failing: { min: 0, max: 10, testChars: 200, words: 25 },
  },
  newTests: { min: 0, max: 30, testChars: 200, pinsWords: 25 },
  untested: { min: 0, max: 8, codeWords: 8, whyWords: 20 },
  planVsDone: { min: 0, max: 20, itemWords: 30, shownByWords: 25 },
} as const
```

- [ ] **Step 5: Parse the branch fields**

In `report.ts`, give `section` the report's kind and refuse a plain part in a branch report. Change its signature and add the check at its top:

```ts
const section = (errors: Errors, at: string, value: unknown, kind: ReportKind): Section => {
  if (!isObject(value)) {
    errors.push(`${at} must be an object`)
    return { heading: '', points: [] }
  }
  const lens = value.kind
  if (kind === 'branch' && (lens === undefined || lens === 'part')) {
    errors.push(`${at}.kind is needed in a branch report: one of ${BRANCH_LENSES.join(', ')}`)
  } else if (lens !== undefined && (typeof lens !== 'string' || !(LENSES as readonly string[]).includes(lens))) {
    errors.push(`${at}.kind must be one of ${LENSES.join(', ')}`)
  }
  const part: Section = {
    ...(lens !== undefined ? { kind: lens as Lens } : {}),
```

(rename the old local `kind` to `lens` through the rest of `section`; nothing else in it changes).

Above `parseReport`, add the branch parsers:

```ts
// A whole number, 0 or more.
const whole = (errors: Errors, at: string, value: unknown): number => {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 0) {
    errors.push(`${at} must be a whole number, 0 or more`)
    return 0
  }
  return value
}

// One of a few words, `said` naming them for the refusal.
const oneOf = <T extends string>(errors: Errors, at: string, value: unknown, allowed: readonly string[], said: string): T => {
  if (typeof value !== 'string' || !allowed.includes(value)) errors.push(`${at} must be ${said}`)
  return value as T
}

// An object's fields, or none when it is not one.
const fieldsOf = (errors: Errors, at: string, value: unknown): Record<string, unknown> => {
  if (isObject(value)) return value
  errors.push(`${at} must be an object`)
  return {}
}

const pathText = (errors: Errors, at: string, value: unknown): string =>
  text(errors, at, value, { chars: LIMITS.files.pathChars })

// A branch report's own fields, every problem noted.
const branchInput = (errors: Errors, e: Record<string, unknown>): BranchInput => {
  const B = BRANCH_LIMITS
  const suite = fieldsOf(errors, 'suite', e.suite)
  return {
    hot_spots: list(errors, 'hot_spots', e.hot_spots, B.hotSpots, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        spot: text(errors, `${at}.spot`, o.spot, { words: B.hotSpots.spotWords }),
        why: text(errors, `${at}.why`, o.why, { words: B.hotSpots.whyWords }),
        files: list(errors, `${at}.files`, o.files, B.hotSpots.files, (f, i) => pathText(errors, i, f)),
      }
    }),
    story: list(errors, 'story', e.story, B.story, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        step: text(errors, `${at}.step`, o.step, { words: B.story.stepWords }),
        commits: list(errors, `${at}.commits`, o.commits, B.story.commits, (c, i) =>
          text(errors, i, c, { chars: B.story.shaChars }),
        ),
        points: list(errors, `${at}.points`, o.points, B.story.points, (p, i) =>
          text(errors, i, p, { words: B.story.points.words }),
        ),
      }
    }),
    map: list(errors, 'map', e.map, B.map, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        path: pathText(errors, `${at}.path`, o.path),
        change: oneOf<Change>(errors, `${at}.change`, o.change, CHANGES, 'new, change or remove'),
        items: list(errors, `${at}.items`, o.items, B.map.items, (it, i) => {
          const m = fieldsOf(errors, i, it)
          return {
            name: text(errors, `${i}.name`, m.name, { chars: B.map.items.nameChars }),
            does: text(errors, `${i}.does`, m.does, { words: B.map.items.words }),
          }
        }),
      }
    }),
    suite: {
      runs: list(errors, 'suite.runs', suite.runs, B.suite.runs, (v, at) => {
        const o = fieldsOf(errors, at, v)
        return {
          suite: text(errors, `${at}.suite`, o.suite, { words: B.suite.runs.suiteWords }),
          command: text(errors, `${at}.command`, o.command, { chars: B.suite.runs.commandChars }),
          passed: whole(errors, `${at}.passed`, o.passed),
          failed: whole(errors, `${at}.failed`, o.failed),
        }
      }),
      not_run: list(errors, 'suite.not_run', suite.not_run, B.suite.notRun, (v, at) => {
        const o = fieldsOf(errors, at, v)
        return {
          suite: text(errors, `${at}.suite`, o.suite, { words: B.suite.runs.suiteWords }),
          why: text(errors, `${at}.why`, o.why, { words: B.suite.notRun.words }),
        }
      }),
      failing: list(errors, 'suite.failing', suite.failing, B.suite.failing, (v, at) => {
        const o = fieldsOf(errors, at, v)
        return {
          test: text(errors, `${at}.test`, o.test, { chars: B.suite.failing.testChars }),
          file: pathText(errors, `${at}.file`, o.file),
          why: text(errors, `${at}.why`, o.why, { words: B.suite.failing.words }),
        }
      }),
    },
    new_tests: list(errors, 'new_tests', e.new_tests, B.newTests, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        test: text(errors, `${at}.test`, o.test, { chars: B.newTests.testChars }),
        file: pathText(errors, `${at}.file`, o.file),
        layer: oneOf<Layer>(errors, `${at}.layer`, o.layer, LAYERS, 'unit, integration or e2e'),
        pins: text(errors, `${at}.pins`, o.pins, { words: B.newTests.pinsWords }),
        result: oneOf<Result>(errors, `${at}.result`, o.result, RESULTS, 'pass, fail or not run'),
      }
    }),
    untested: list(errors, 'untested', e.untested, B.untested, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        code: text(errors, `${at}.code`, o.code, { words: B.untested.codeWords }),
        file: pathText(errors, `${at}.file`, o.file),
        why: text(errors, `${at}.why`, o.why, { words: B.untested.whyWords }),
      }
    }),
    plan_vs_done: list(errors, 'plan_vs_done', e.plan_vs_done, B.planVsDone, (v, at) => {
      const o = fieldsOf(errors, at, v)
      return {
        item: text(errors, `${at}.item`, o.item, { words: B.planVsDone.itemWords }),
        kind: oneOf<PlanKind>(errors, `${at}.kind`, o.kind, PLAN_KINDS, 'step or done when'),
        progress: oneOf<Progress>(errors, `${at}.progress`, o.progress, PROGRESS, 'done, partial or not yet'),
        shown_by: text(errors, `${at}.shown_by`, o.shown_by, { words: B.planVsDone.shownByWords }),
      }
    }),
  }
}
```

Replace `Parsed` and `parseReport` with:

```ts
// A report the tool takes, the lines in it over their word limits, and, for
// a branch report, its own fields as the model gave them.
export type Parsed = { report: Report; warnings: string[]; branch?: BranchInput }

// Answers the report, or every problem with it, joined. A branch report has
// no needs_your_eye or checks: its hot spots and its plan vs done take their
// places.
export const parseReport = (e: Record<string, unknown>, kind: ReportKind = 'plan'): Parsed | string => {
  const errors = findings()
  const lines = (at: string, value: unknown, count: Count & { readonly words: number }): string[] =>
    list(errors, at, value, count, (v, i) => text(errors, i, v, { words: count.words }))
  const branch = kind === 'branch'
  // A plan report's field a branch report leaves out, refused if given.
  const none = (at: string, value: unknown, instead: string): [] => {
    if (value !== undefined && !(Array.isArray(value) && value.length === 0)) {
      errors.push(`${at} is not used in a branch report: ${instead} takes its place`)
    }
    return []
  }
  const report: Report = {
    title: text(errors, 'title', e.title, { chars: LIMITS.titleChars }),
    summary: text(errors, 'summary', e.summary, { words: LIMITS.summaryWords }),
    changes: lines('changes', e.changes, LIMITS.changes),
    unchanged: lines('unchanged', e.unchanged, LIMITS.unchanged),
    needs_your_eye: branch
      ? none('needs_your_eye', e.needs_your_eye, 'hot_spots')
      : lines('needs_your_eye', e.needs_your_eye, LIMITS.needsYourEye),
    terms: list(errors, 'terms', e.terms, LIMITS.terms, (v, at) => term(errors, at, v)),
    sections: list(errors, 'sections', e.sections, LIMITS.sections, (v, at) => section(errors, at, v, kind)),
    files: list(errors, 'files', e.files, LIMITS.files, (v, at) => file(errors, at, v)),
    checks: branch
      ? none('checks', e.checks, 'plan_vs_done')
      : list(errors, 'checks', e.checks, LIMITS.checks, (v, at) => check(errors, at, v)),
  }
  const input = branch ? branchInput(errors, e) : undefined
  if (!branch) {
    for (const field of BRANCH_FIELDS) if (e[field] !== undefined) errors.push(`${field} is only for a branch report`)
  }
  if (errors.length === 0) {
    return input === undefined ? { report, warnings: errors.warnings } : { report, warnings: errors.warnings, branch: input }
  }
  const more = errors.length > MAX_ERRORS ? `; and ${errors.length - MAX_ERRORS} more` : ''
  return errors.slice(0, MAX_ERRORS).join('; ') + more
}
```

- [ ] **Step 6: Run the tests**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: every test passes, the plan report's tests unchanged.

- [ ] **Step 7: Commit**

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/tests/report.test.ts claude/refine-mod/tests/branch-fixtures.ts
git commit -m "feat(report): parse a branch report's own fields

A branch report takes the plan report's fields bar needs_your_eye
and checks, which its hot spots and plan vs done replace, plus its
hot spots, commit story, changes map, test suite, new tests,
untested code and plan vs done, held to limits of their own.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The branch's facts from git

**Files:**
- Create: `claude/refine-mod/hooks/branch.ts`
- Modify: `claude/refine-mod/tests/branch-fixtures.ts`
- Test: `claude/refine-mod/tests/branch.test.ts`

**Interfaces:**
- Consumes: `BranchInput`, `Branch`, `Commit`, `Facts` from `report.ts` (Task 1).
- Produces (in `branch.ts`):
  - `export type Target = { worktree: string; base: string }`, `branchTarget(worktree: unknown, base: unknown): Target | undefined`
  - `gitArgv(worktree, ...args): string[]`, `branchArgv(wt)`, `mergeBaseArgv(wt, base)`, `logArgv(wt, mergeBase)`, `numstatArgv(wt, mergeBase)`, `statusArgv(wt)`, `gitCommand(argv): string`
  - `export type Logged = { sha: string; subject: string }`, `export type Git = { branch; base; merge_base; log: Logged[]; paths: string[]; added; removed; uncommitted }`
  - `parseLog(stdout): Logged[]`, `parseNumstat(stdout): { paths: string[]; added: number; removed: number }`, `countStatus(stdout): number`, `factsOf(git): Facts`
  - `withGit(input: BranchInput, git: Git): Branch | string`
- Produces (in `branch-fixtures.ts`): `GIT: Git`, `SHA_A`, `SHA_B`, `branched(input?, git?): Report` (a parsed branch report with its git facts, failing the test where refused).

- [ ] **Step 1: Extend the fixtures**

Append to `claude/refine-mod/tests/branch-fixtures.ts`:

```ts
import { withGit } from '../hooks/branch'
import type { Git } from '../hooks/branch'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'

export const SHA_A = `abc1234${'0'.repeat(33)}`
export const SHA_B = `abd5678${'1'.repeat(33)}`

// What git says of BRANCH_MIN's branch: its two commits, the files it
// changes, and one file left uncommitted.
export const GIT: Git = {
  branch: 'task/feat-new-words-0b8f6a52',
  base: 'main',
  merge_base: `f00dfee${'2'.repeat(33)}`,
  log: [
    { sha: SHA_A, subject: 'feat: add the words' },
    { sha: SHA_B, subject: 'test: pin the words' },
  ],
  paths: ['src/words.rs', 'README.md'],
  added: 12,
  removed: 3,
  uncommitted: 1,
}

// A branch report as the tool holds it once git has named its commits,
// failing the test where it is refused.
export const branched = (input: Record<string, unknown> = BRANCH_MIN, git: Git = GIT): Report => {
  const parsed = parseReport(input, 'branch')
  if (typeof parsed === 'string') throw new Error(parsed)
  if (parsed.branch === undefined) throw new Error('no branch fields')
  const branch = withGit(parsed.branch, git)
  if (typeof branch === 'string') throw new Error(branch)
  return { ...parsed.report, branch }
}
```

(Move the file's imports to its top.)

- [ ] **Step 2: Write the failing tests**

Create `claude/refine-mod/tests/branch.test.ts`:

```ts
import { describe, expect, test } from 'claude-code/testing'
import {
  branchArgv,
  branchTarget,
  countStatus,
  factsOf,
  gitArgv,
  gitCommand,
  logArgv,
  mergeBaseArgv,
  numstatArgv,
  parseLog,
  parseNumstat,
  statusArgv,
  withGit,
} from '../hooks/branch'
import { parseReport } from '../hooks/report'
import type { BranchInput } from '../hooks/report'
import { BRANCH_MIN, GIT, SHA_A, SHA_B } from './branch-fixtures'

const input = (): BranchInput => {
  const parsed = parseReport(BRANCH_MIN, 'branch')
  if (typeof parsed === 'string' || parsed.branch === undefined) throw new Error(String(parsed))
  return parsed.branch
}

describe('the target', () => {
  test('takes an absolute worktree and a branch name', () => {
    expect(branchTarget('/w/t', 'main')).toEqual({ worktree: '/w/t', base: 'main' })
  })

  test('refuses a relative worktree, an empty base, an option and a space', () => {
    expect(branchTarget('w/t', 'main')).toBeUndefined()
    expect(branchTarget('/w/t', '')).toBeUndefined()
    expect(branchTarget('/w/t', '--output=/x')).toBeUndefined()
    expect(branchTarget('/w/t', 'ma in')).toBeUndefined()
    expect(branchTarget(undefined, 'main')).toBeUndefined()
  })
})

describe('the git commands', () => {
  test('read the worktree with nothing in its config run', () => {
    expect(gitArgv('/w', 'status')).toEqual(['git', '--no-optional-locks', '-C', '/w', '-c', 'core.fsmonitor=false', 'status'])
    expect(branchArgv('/w').slice(6)).toEqual(['rev-parse', '--abbrev-ref', 'HEAD'])
    expect(mergeBaseArgv('/w', 'main').slice(6)).toEqual(['merge-base', 'HEAD', 'main'])
    expect(logArgv('/w', 'mb').slice(6)).toEqual(['log', '--reverse', '--format=%H%x09%s', 'mb..HEAD'])
    expect(numstatArgv('/w', 'mb').slice(6)).toEqual([
      'diff', '--numstat', '--no-renames', '--no-ext-diff', '--no-textconv', 'mb', 'HEAD',
    ])
    expect(statusArgv('/w').slice(6)).toEqual(['status', '--porcelain', '--untracked-files=normal'])
  })

  test('a command is named by its own words in a refusal', () => {
    expect(gitCommand(mergeBaseArgv('/w', 'main'))).toBe('git merge-base HEAD main')
  })
})

describe('reading what git answers', () => {
  test('the log, oldest first, a subject keeping its tabs', () => {
    expect(parseLog(`${SHA_A}\tfeat: a\tb\n${SHA_B}\tfix: c\n`)).toEqual([
      { sha: SHA_A, subject: 'feat: a\tb' },
      { sha: SHA_B, subject: 'fix: c' },
    ])
    expect(parseLog('')).toEqual([])
  })

  test('numstat counts a binary file as no lines, and keeps a path with spaces', () => {
    expect(parseNumstat('3\t1\tsrc/a.rs\n-\t-\timg.png\n10\t0\tdocs/a b.md\n')).toEqual({
      paths: ['src/a.rs', 'img.png', 'docs/a b.md'],
      added: 13,
      removed: 1,
    })
  })

  test('status counts each file with changes not committed', () => {
    expect(countStatus(' M a\n?? b\n')).toBe(2)
    expect(countStatus('')).toBe(0)
  })

  test('the facts name the merge base short', () => {
    expect(factsOf(GIT)).toEqual({
      branch: 'task/feat-new-words-0b8f6a52',
      base: 'main',
      merge_base: 'f00dfee',
      commits: 2,
      files: 2,
      added: 12,
      removed: 3,
      uncommitted: 1,
    })
  })
})

describe('withGit', () => {
  test('gives each commit its short sha and subject, and the branch its facts', () => {
    const given = { ...input(), story: [{ step: 'Wrote the words', commits: ['abc1234', SHA_B.toUpperCase()], points: ['p'] }] }
    expect(withGit(given, GIT)).toEqual({
      ...given,
      story: [
        {
          step: 'Wrote the words',
          commits: [
            { sha: 'abc1234', subject: 'feat: add the words' },
            { sha: 'abd5678', subject: 'test: pin the words' },
          ],
          points: ['p'],
        },
      ],
      facts: factsOf(GIT),
    })
  })

  test('refuses a sha too short, one not on the branch, one told twice, and a commit left out', () => {
    const story = [{ step: 's', commits: ['abc1234', 'abc12', 'fff9999', SHA_A], points: ['p'] }]
    expect(withGit({ ...input(), story }, GIT)).toBe(
      'story[0].commits[1]: abc12 is too short; give at least 7 characters; ' +
        'story[0].commits[2]: fff9999 is not a commit on the branch; ' +
        `story[0].commits[3]: ${SHA_A} is told already, at story[0].commits[0]; ` +
        'the story leaves out 1 commit(s): abd5678; every commit goes in one step',
    )
  })

  test('refuses a prefix two commits share', () => {
    const twin = { sha: `abc1234${'9'.repeat(33)}`, subject: 'x' }
    const story = [{ step: 's', commits: ['abc1234', SHA_B], points: ['p'] }]
    expect(withGit({ ...input(), story }, { ...GIT, log: [...GIT.log, twin] })).toBe(
      'story[0].commits[0]: abc1234 names more than one commit; give more of it; ' +
        'the story leaves out 2 commit(s): abc1234, abc1234; every commit goes in one step',
    )
  })

  test('refuses a map file the branch does not change', () => {
    const map = [{ path: 'src/other.rs', change: 'change' as const, items: [{ name: 'f', does: 'd' }] }]
    expect(withGit({ ...input(), map }, GIT)).toBe('map[0].path: src/other.rs is not a file the branch changes')
  })

  test('refuses a branch with no commits', () => {
    expect(withGit(input(), { ...GIT, log: [] })).toBe('the branch has no commits since main: there is nothing to report')
  })
})
```

- [ ] **Step 3: Run them to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: `branch.test.ts` fails to load: `hooks/branch` does not exist.

- [ ] **Step 4: Write `branch.ts`**

Create `claude/refine-mod/hooks/branch.ts`:

```ts
// The pure half of a branch report's facts: the git commands the mod runs
// on the task's worktree, outside the sandbox, what it keeps of their
// answers, and how the model's commit story is checked against them. The
// model names commits by sha alone; their subjects and the branch's numbers
// come from git, so the deck cannot misstate them. No `$`, so the unit
// tests reach it directly.

import type { Branch, BranchInput, Commit, Facts } from './report'

// The worktree a branch report reads and the branch it is measured
// against, from the session's settings.
export type Target = { worktree: string; base: string }

// The target, or undefined unless the worktree is absolute and the base a
// plain branch name: never an option, so it cannot change what git does.
export const branchTarget = (worktree: unknown, base: unknown): Target | undefined =>
  typeof worktree === 'string' &&
  worktree.startsWith('/') &&
  typeof base === 'string' &&
  /^[^-\s][^\s]*$/.test(base) &&
  !/[\u0000-\u001F\u007F]/.test(base)
    ? { worktree, base }
    : undefined

// Read only, with nothing in the repository's config run for it: no
// fsmonitor hook, and no optional lock taken. Diffs ask for no external
// diff and no textconv (numstatArgv).
export const gitArgv = (worktree: string, ...args: string[]): string[] => [
  'git', '--no-optional-locks', '-C', worktree, '-c', 'core.fsmonitor=false', ...args,
]

// The words gitArgv puts before the command's own.
const PREFIX = 6

export const branchArgv = (worktree: string): string[] => gitArgv(worktree, 'rev-parse', '--abbrev-ref', 'HEAD')
export const mergeBaseArgv = (worktree: string, base: string): string[] => gitArgv(worktree, 'merge-base', 'HEAD', base)
// Oldest first, as the story tells them: the full sha, a tab, the subject.
export const logArgv = (worktree: string, mergeBase: string): string[] =>
  gitArgv(worktree, 'log', '--reverse', '--format=%H%x09%s', `${mergeBase}..HEAD`)
// What is committed on the branch since the merge base; not the worktree's
// uncommitted changes, which statusArgv counts.
export const numstatArgv = (worktree: string, mergeBase: string): string[] =>
  gitArgv(worktree, 'diff', '--numstat', '--no-renames', '--no-ext-diff', '--no-textconv', mergeBase, 'HEAD')
export const statusArgv = (worktree: string): string[] =>
  gitArgv(worktree, 'status', '--porcelain', '--untracked-files=normal')

// A command as a refusal names it: `git` and its own words.
export const gitCommand = (argv: readonly string[]): string => ['git', ...argv.slice(PREFIX)].join(' ')

export type Logged = { sha: string; subject: string }

// Everything the mod reads from git for a branch report.
export type Git = {
  branch: string
  base: string
  merge_base: string
  log: Logged[]
  paths: string[]
  added: number
  removed: number
  uncommitted: number
}

const nonEmpty = (stdout: string): string[] => stdout.split('\n').filter(line => line !== '')

export const parseLog = (stdout: string): Logged[] =>
  nonEmpty(stdout).map(line => {
    const tab = line.indexOf('\t')
    return tab < 0 ? { sha: line, subject: '' } : { sha: line.slice(0, tab), subject: line.slice(tab + 1) }
  })

// `git diff --numstat`: added, removed, path, a tab between; a binary file
// says `-` for both, and counts as no lines.
export const parseNumstat = (stdout: string): { paths: string[]; added: number; removed: number } => {
  const rows = nonEmpty(stdout).map(line => line.split('\t'))
  const lines = (n: string | undefined): number => (n === undefined || n === '-' ? 0 : Number(n))
  return {
    paths: rows.map(([, , ...path]) => path.join('\t')),
    added: rows.reduce((sum, [a]) => sum + lines(a), 0),
    removed: rows.reduce((sum, [, r]) => sum + lines(r), 0),
  }
}

export const countStatus = (stdout: string): number => nonEmpty(stdout).length

// A sha as the deck shows it, and the least of one the model may give.
const SHORT = 7

export const factsOf = (git: Git): Facts => ({
  branch: git.branch,
  base: git.base,
  merge_base: git.merge_base.slice(0, SHORT),
  commits: git.log.length,
  files: git.paths.length,
  added: git.added,
  removed: git.removed,
  uncommitted: git.uncommitted,
})

// The model's branch fields with each commit named by git, and the
// branch's facts; or every problem with them, joined. Every commit on the
// branch goes in exactly one step of the story, and every file the map
// names is one the branch changes.
export const withGit = (input: BranchInput, git: Git): Branch | string => {
  if (git.log.length === 0) return `the branch has no commits since ${git.base}: there is nothing to report`
  const errors: string[] = []
  // Each commit told, by its full sha, and where.
  const told = new Map<string, string>()
  const story = input.story.map((step, i) => ({
    ...step,
    commits: step.commits.map((given, j): Commit => {
      const at = `story[${i}].commits[${j}]`
      const sha = given.trim().toLowerCase()
      if (sha.length < SHORT) {
        errors.push(`${at}: ${given} is too short; give at least ${SHORT} characters`)
        return { sha: given, subject: '' }
      }
      const found = git.log.filter(c => c.sha.startsWith(sha))
      const [commit] = found
      if (found.length !== 1 || commit === undefined) {
        errors.push(
          found.length === 0
            ? `${at}: ${given} is not a commit on the branch`
            : `${at}: ${given} names more than one commit; give more of it`,
        )
        return { sha: given, subject: '' }
      }
      const before = told.get(commit.sha)
      if (before !== undefined) errors.push(`${at}: ${given} is told already, at ${before}`)
      else told.set(commit.sha, at)
      return { sha: commit.sha.slice(0, SHORT), subject: commit.subject }
    }),
  }))
  const left = git.log.filter(c => !told.has(c.sha))
  if (left.length > 0) {
    const named = left.slice(0, 10).map(c => c.sha.slice(0, SHORT)).join(', ')
    errors.push(
      `the story leaves out ${left.length} commit(s): ${named}${left.length > 10 ? ', …' : ''}; every commit goes in one step`,
    )
  }
  input.map.forEach((m, i) => {
    if (!git.paths.includes(m.path)) errors.push(`map[${i}].path: ${m.path} is not a file the branch changes`)
  })
  if (errors.length > 0) return errors.join('; ')
  return { ...input, story, facts: factsOf(git) }
}
```

- [ ] **Step 5: Run the tests**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: every test passes.

- [ ] **Step 6: Commit**

```bash
git add claude/refine-mod/hooks/branch.ts claude/refine-mod/tests/branch.test.ts claude/refine-mod/tests/branch-fixtures.ts
git commit -m "feat(report): read a branch's facts and commits from git

The mod names each commit the story gives by sha, and counts the
branch's commits, files and lines, from git itself, so the deck
cannot misstate them. A story that leaves out a commit, or a map
naming a file the branch does not change, is refused.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The branch report's Markdown twin

**Files:**
- Modify: `claude/refine-mod/hooks/markdown.ts`
- Test: `claude/refine-mod/tests/markdown.test.ts`

**Interfaces:**
- Consumes: `Report.branch`, `BRANCH_HEADINGS`, `LAYERS`, `RESULTS`, `PLAN_KINDS`, `PROGRESS`, `CHANGES` (Task 1); `branched`, `GIT`, `BRANCH_MIN` (Task 2 fixtures).
- Produces: `reportMarkdown(report, plan)` and `reportFromMarkdown(markdown)` handle `report.branch`; `twinOf(report, plan)` round-trips a branch report unchanged in signature.

The branch twin, top to bottom (a plan report's twin is unchanged):

```text
# <title>

_Branch `<branch>` from `<base>` at `<merge_base>`: <n> commits, <n> files, +<n> -<n>, <n> uncommitted._

<summary>

## What changes
…
## What stays the same          (when any)
…
## Where to look hardest

### <spot>

<why>

Files: `<path>`, `<path>`

## Words used here              (when any)
…
## How the branch got here

### <step>

- `<sha>` <subject>
- `<sha>` <subject>

- <point>

## <part heading>                (each part, in the model's order, as now)
…
## What each file now does

### <change> `<path>`

- **<name>**: <does>

## Did anything break?

| Suite | Command | Passed | Failed |
…
| Not run | Why |                 (when any)
…
| Failing test | File | Why |     (when any)
…
## The tests this branch adds

| Test | File | Layer | What it pins | Result |      (or `_None._`)
…
### Changed code with no test   (when any)

| Code | File | Why |
…
## Plan vs done                 (when any)

| From the plan | Kind | Progress | Shown by |
…
## Files                        (when any)
…
## The plan
…
```

- [ ] **Step 1: Write the failing tests**

Add to `claude/refine-mod/tests/markdown.test.ts`, importing `BRANCH_MIN`, `GIT` and `branched` from `./branch-fixtures`:

```ts
describe('a branch report', () => {
  const full = branched({
    ...BRANCH_MIN,
    unchanged: ['The panel.'],
    terms: [{ term: 'twin', meaning: 'The .md copy.' }],
    hot_spots: [
      { spot: 'Pipes | in cells', why: 'A | b stays one cell.', files: ['src/words.rs', 'README.md'] },
      { spot: 'Second spot', why: 'Why two.', files: ['README.md'] },
    ],
    story: [
      { step: 'Wrote the words', commits: ['abc1234'], points: ['One.', 'Two.'] },
      { step: 'Pinned them', commits: ['abd5678'], points: ['Three.'] },
    ],
    map: [
      { path: 'src/words.rs', change: 'change', items: [{ name: 'words()', does: 'Answers.' }, { name: 'WORDS', does: 'Holds.' }] },
      { path: 'README.md', change: 'new', items: [{ name: 'Words', does: 'Says so.' }] },
    ],
    suite: {
      runs: [
        { suite: 'cargo test', command: 'CARGO_TARGET_DIR=$TMPDIR/t cargo test', passed: 777, failed: 1 },
        { suite: 'plugin tests', command: 'python3 -m unittest', passed: 40, failed: 0 },
      ],
      not_run: [{ suite: 'e2e-panel.sh', why: 'Nests a niri on your screen.' }],
      failing: [{ test: 'words_are_new', file: 'src/words.rs', why: 'It expects old words.' }],
    },
    untested: [{ code: 'WORDS', file: 'src/words.rs', why: 'A constant.' }],
    plan_vs_done: [{ item: 'Add the words', kind: 'step', progress: 'partial', shown_by: 'abc1234' }],
  })
  const PLAN = { description: 'feat: New words', notes: ['Steps: Add the words'] }

  test('reads back as the report it was written from', () => {
    expect(reportFromMarkdown(reportMarkdown(full, PLAN))).toEqual({ report: full, plan: PLAN })
    expect(typeof twinOf(full, PLAN)).toBe('object')
  })

  test('the smallest branch report reads back too, its optional slides left out', () => {
    const min = branched()
    const md = reportMarkdown(min, PLAN)
    expect(reportFromMarkdown(md)).toEqual({ report: min, plan: PLAN })
    expect(md).not.toContain('## Plan vs done')
    expect(md).not.toContain('## Done when')
    expect(md).not.toContain('### Changed code with no test')
  })

  test('opens with the branch facts git gave, then the summary', () => {
    const lines = reportMarkdown(branched(), PLAN).split('\n')
    expect(lines.slice(0, 5)).toEqual([
      '# feat: New words',
      '',
      '_Branch `task/feat-new-words-0b8f6a52` from `main` at `f00dfee`: 2 commits, 2 files, +12 -3, 1 uncommitted._',
      '',
      'The branch adds new words.',
    ])
  })

  test('names each commit with its subject from git', () => {
    expect(reportMarkdown(branched(), PLAN)).toContain(
      '## How the branch got here\n\n### Wrote the words\n\n- `abc1234` feat: add the words\n- `abd5678` test: pin the words\n\n- The words went in.',
    )
  })

  test('says so when the branch adds no test', () => {
    const md = reportMarkdown(branched({ ...BRANCH_MIN, new_tests: [] }), PLAN)
    expect(md).toContain('## The tests this branch adds\n\n_None._')
    expect(reportFromMarkdown(md).report.branch?.new_tests).toEqual([])
  })

  test('a plan report keeps its twin as it was', () => {
    expect(reportMarkdown(MINIMAL, PLAN)).not.toContain('_Branch `')
    expect(reportFromMarkdown(reportMarkdown(MINIMAL, PLAN)).report.branch).toBeUndefined()
  })

  test('refuses a branch whose name would not read back', () => {
    const odd = { ...full, branch: { ...full.branch!, facts: { ...full.branch!.facts, branch: 'a`b' } } }
    expect(typeof twinOf(odd, PLAN)).toBe('string')
  })
})
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: the `a branch report` tests fail: the twin carries no branch blocks, so it reads back without `branch`.

- [ ] **Step 3: Write the branch twin**

In `claude/refine-mod/hooks/markdown.ts`, import the new names:

```ts
import { BRANCH_HEADINGS as H, CHANGES, LAYERS, LENSES, PLAN_KINDS, PROGRESS, RESULTS, SWAPS } from './report'
import type {
  Branch, Change, CodePair, Commit, Diagram, Facts, FileChange, HotSpot, Layer, Lens, MapEntry, PlanKind,
  Progress, Report, Result, Section, Step, Suite, Swap, Tool,
} from './report'
```

Add the writers above `reportMarkdown`:

```ts
// A branch report's facts, from git, under its title.
const FACTS =
  /^_Branch `([^`]+)` from `([^`]+)` at `([^`]+)`: (\d+) commits, (\d+) files, \+(\d+) -(\d+), (\d+) uncommitted\._$/
const factsLine = (f: Facts): string =>
  `_Branch \`${f.branch}\` from \`${f.base}\` at \`${f.merge_base}\`: ${f.commits} commits, ${f.files} files, ` +
  `+${f.added} -${f.removed}, ${f.uncommitted} uncommitted._`

const SUITE_HEAD = ['Suite', 'Command', 'Passed', 'Failed']
const NOT_RUN_HEAD = ['Not run', 'Why']
const FAILING_HEAD = ['Failing test', 'File', 'Why']
const TESTS_HEAD = ['Test', 'File', 'Layer', 'What it pins', 'Result']
const UNTESTED_HEAD = ['Code', 'File', 'Why']
const PLAN_HEAD = ['From the plan', 'Kind', 'Progress', 'Shown by']
const NO_TESTS = '_None._'

const code = (text: string): string => `\`${text}\``

const hotSpots = (list: readonly HotSpot[]): string =>
  list.map(h => [`### ${h.spot}`, h.why, `Files: ${h.files.map(code).join(', ')}`].join('\n\n')).join('\n\n')

const story = (steps: readonly Step<Commit>[]): string =>
  steps
    .map(s => [`### ${s.step}`, bullets(s.commits.map(c => `${code(c.sha)} ${c.subject}`)), bullets(s.points)].join('\n\n'))
    .join('\n\n')

const changesMap = (entries: readonly MapEntry[]): string =>
  entries
    .map(m => [`### ${m.change} ${code(m.path)}`, bullets(m.items.map(i => `**${i.name}**: ${i.does}`))].join('\n\n'))
    .join('\n\n')

const suite = (s: Suite): string =>
  [
    table(SUITE_HEAD, s.runs.map(r => [r.suite, r.command, String(r.passed), String(r.failed)])),
    s.not_run.length > 0 ? table(NOT_RUN_HEAD, s.not_run.map(n => [n.suite, n.why])) : '',
    s.failing.length > 0 ? table(FAILING_HEAD, s.failing.map(f => [f.test, code(f.file), f.why])) : '',
  ]
    .filter(Boolean)
    .join('\n\n')

const newTests = (b: Branch): string =>
  [
    b.new_tests.length === 0
      ? NO_TESTS
      : table(TESTS_HEAD, b.new_tests.map(t => [t.test, code(t.file), t.layer, t.pins, t.result])),
    b.untested.length > 0
      ? `### ${H.untested}\n\n${table(UNTESTED_HEAD, b.untested.map(u => [u.code, code(u.file), u.why]))}`
      : '',
  ]
    .filter(Boolean)
    .join('\n\n')
```

Replace `reportMarkdown` with:

```ts
// The report as Markdown, ending with the plan the person was shown, as the
// HTML's last slide does. A branch report's own blocks sit where its slides
// do; its parts stay in the model's order, as a plan report's do.
export const reportMarkdown = (report: Report, plan: Plan): string => {
  const b = report.branch
  const out = [`# ${report.title}`]
  if (b !== undefined) out.push(factsLine(b.facts))
  out.push(report.summary, `## What changes\n\n${bullets(report.changes)}`)
  if (report.unchanged.length > 0) out.push(`## What stays the same\n\n${bullets(report.unchanged)}`)
  if (b !== undefined) out.push(`## ${H.hotSpots}\n\n${hotSpots(b.hot_spots)}`)
  if (report.needs_your_eye.length > 0) {
    out.push(`## Where your judgement is needed\n\n${bullets(report.needs_your_eye)}`)
  }
  if (report.terms.length > 0) {
    out.push(`## Words used here\n\n${bullets(report.terms.map(t => `**${t.term}**: ${t.meaning}`))}`)
  }
  if (b !== undefined) out.push(`## ${H.story}\n\n${story(b.story)}`)
  out.push(...report.sections.map(part))
  if (b !== undefined) {
    out.push(`## ${H.map}\n\n${changesMap(b.map)}`, `## ${H.suite}\n\n${suite(b.suite)}`, `## ${H.newTests}\n\n${newTests(b)}`)
    if (b.plan_vs_done.length > 0) {
      out.push(
        `## ${H.planVsDone}\n\n${table(PLAN_HEAD, b.plan_vs_done.map(p => [p.item, p.kind, p.progress, p.shown_by]))}`,
      )
    }
  }
  if (report.files.length > 0) {
    out.push(`## Files\n\n${table(FILE_HEAD, report.files.map(f => [f.change, `\`${f.path}\``, f.why]))}`)
  }
  if (b === undefined) out.push(`## Done when\n\n${table(CHECK_HEAD, report.checks.map(c => [c.check, c.how]))}`)
  const notes = plan.notes.length === 0 ? 'No notes.' : plan.notes.map((n, i) => `${i + 1}. ${n}`).join('\n')
  out.push(`## The plan\n\n**Description:** ${plan.description}\n\n${notes}`)
  return `${out.join('\n\n')}\n`
}
```

- [ ] **Step 4: Read the branch twin back**

In `reportFromMarkdown`, add these readers after `optional` (they use the closures already there):

```ts
  // A fixed slide that must come next: its heading, then its body.
  const heading = (text: string): void => {
    gap()
    line(`## ${text}`)
    gap()
  }
  // `### ` blocks, one after another, each read by `read`.
  const blocks = <T>(read: () => T): T[] => {
    const out = [read()]
    while (next()?.startsWith('### ')) {
      gap()
      out.push(read())
    }
    return out
  }
  const quoted = (cell: string): string => (/^`.+`$/.test(cell) ? cell.slice(1, -1) : fail('a `path`'))
  const count = (cell: string): number => (/^\d+$/.test(cell) ? Number(cell) : fail('a whole number'))
  const hotSpot = (): HotSpot => {
    const spot = match(/^### (.+)$/, 'a "### <spot>" heading')[1] ?? ''
    gap()
    const why = take()
    gap()
    const files = (match(/^Files: (.+)$/, 'a "Files: " line')[1] ?? '').split(', ').map(quoted)
    return { spot, why, files }
  }
  const step = (): Step<Commit> => {
    const name = match(/^### (.+)$/, 'a "### <step>" heading')[1] ?? ''
    gap()
    const commits = list().map(c => {
      const found = /^`([^`]+)` (.*)$/.exec(c) ?? fail('a "`<sha>` <subject>" line')
      return { sha: found[1] ?? '', subject: found[2] ?? '' }
    })
    gap()
    return { step: name, commits, points: list() }
  }
  const mapEntry = (): MapEntry => {
    const found = match(/^### (\S+) `(.+)`$/, 'a "### <change> `<path>`" heading')
    gap()
    const items = list().map(i => {
      const m = /^\*\*(.+?)\*\*: (.+)$/.exec(i) ?? fail('a "**<name>**: <does>" line')
      return { name: m[1] ?? '', does: m[2] ?? '' }
    })
    return { path: found[2] ?? '', change: oneOf<Change>(found[1] ?? '', CHANGES, 'new, change or remove'), items }
  }
  // A table that may follow, by its head, or none.
  const maybeRows = (head: readonly string[]): string[][] => {
    if (next() !== headRow(head)) return []
    gap()
    return rows(head)
  }
```

Then change the body of `reportFromMarkdown` from the title on to:

```ts
  const title = match(/^# (.+)$/, 'the "# <title>" line')[1] ?? ''
  gap()
  const f = FACTS.exec(lines[at] ?? '')
  let facts: Facts | undefined
  if (f !== null) {
    at += 1
    facts = {
      branch: f[1] ?? '',
      base: f[2] ?? '',
      merge_base: f[3] ?? '',
      commits: Number(f[4]),
      files: Number(f[5]),
      added: Number(f[6]),
      removed: Number(f[7]),
      uncommitted: Number(f[8]),
    }
    gap()
  }
  const summary = take()
  gap()
  line('## What changes')
  gap()
  const changes = list()
  const unchanged = optional('What stays the same')
  let hot_spots: HotSpot[] = []
  if (facts !== undefined) {
    heading(H.hotSpots)
    hot_spots = blocks(hotSpot)
  }
  const needs_your_eye = optional('Where your judgement is needed')
  const terms = optional('Words used here').map(entry => {
    const t = /^\*\*(.+?)\*\*: (.+)$/.exec(entry) ?? fail('a "**<term>**: <meaning>" line')
    return { term: t[1] ?? '', meaning: t[2] ?? '' }
  })
  let steps: Step<Commit>[] = []
  if (facts !== undefined) {
    heading(H.story)
    steps = blocks(step)
  }
  const sections: Section[] = []
  while (lines[at] === '' && partAt(at + 1)) {
    gap()
    sections.push(part())
  }
  let branch: Branch | undefined
  if (facts !== undefined) {
    heading(H.map)
    const map = blocks(mapEntry)
    heading(H.suite)
    const runs = rows(SUITE_HEAD).map(([suite = '', command = '', passed = '', failed = '']) => ({
      suite,
      command,
      passed: count(passed),
      failed: count(failed),
    }))
    const not_run = maybeRows(NOT_RUN_HEAD).map(([suite = '', why = '']) => ({ suite, why }))
    const failing = maybeRows(FAILING_HEAD).map(([test = '', file = '', why = '']) => ({ test, file: quoted(file), why }))
    heading(H.newTests)
    let new_tests: Branch['new_tests'] = []
    if (lines[at] === NO_TESTS) at += 1
    else {
      new_tests = rows(TESTS_HEAD).map(([test = '', file = '', layer = '', pins = '', result = '']) => ({
        test,
        file: quoted(file),
        layer: oneOf<Layer>(layer, LAYERS, 'unit, integration or e2e'),
        pins,
        result: oneOf<Result>(result, RESULTS, 'pass, fail or not run'),
      }))
    }
    let untested: Branch['untested'] = []
    if (next() === `### ${H.untested}`) {
      gap()
      line(`### ${H.untested}`)
      gap()
      untested = rows(UNTESTED_HEAD).map(([c = '', file = '', why = '']) => ({ code: c, file: quoted(file), why }))
    }
    let plan_vs_done: Branch['plan_vs_done'] = []
    if (slide(H.planVsDone)) {
      heading(H.planVsDone)
      plan_vs_done = rows(PLAN_HEAD).map(([item = '', kind = '', progress = '', shown_by = '']) => ({
        item,
        kind: oneOf<PlanKind>(kind, PLAN_KINDS, 'step or done when'),
        progress: oneOf<Progress>(progress, PROGRESS, 'done, partial or not yet'),
        shown_by,
      }))
    }
    branch = { facts, hot_spots, story: steps, map, suite: { runs, not_run, failing }, new_tests, untested, plan_vs_done }
  }
  let files: FileChange[] = []
  if (slide('Files')) {
    gap()
    line('## Files')
    gap()
    files = rows(FILE_HEAD).map(([change = '', path = '', why = '']) => ({
      change: oneOf<Change>(change, CHANGES, 'new, change or remove'),
      path: /^`.*`$/.test(path) ? path.slice(1, -1) : fail('a `path`'),
      why,
    }))
  }
  let checks: { check: string; how: string }[] = []
  if (branch === undefined) {
    gap()
    line('## Done when')
    gap()
    checks = rows(CHECK_HEAD).map(([check = '', how = '']) => ({ check, how }))
  }
  gap()
  line('## The plan')
```

…keeping the rest (the plan's description and notes, the end check) as it is, and return:

```ts
  return {
    report: {
      title, summary, changes, unchanged, needs_your_eye, terms, sections, files, checks,
      ...(branch === undefined ? {} : { branch }),
    },
    plan: { description, notes },
  }
```

Note: `slide(heading)` already checks the next block is `## heading` and not a part; `heading()` reads past it.

- [ ] **Step 5: Run the tests**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: every test passes, the plan report's twin tests unchanged.

- [ ] **Step 6: Commit**

```bash
git add claude/refine-mod/hooks/markdown.ts claude/refine-mod/tests/markdown.test.ts
git commit -m "feat(report): write and read back a branch report's twin

The branch report's Markdown twin carries its facts, hot spots,
commit story, changes map, test suite, new tests and plan vs done,
and reads back into the report it was written from, so its slides
are built from the twin as a plan report's are.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The branch deck

**Files:**
- Modify: `claude/refine-mod/hooks/page.ts`, `claude/refine-mod/hooks/style.ts`
- Create: `claude/refine-mod/hooks/branch-page.ts`
- Test: `claude/refine-mod/tests/page.test.ts`

**Interfaces:**
- Consumes: `Report.branch`, `BRANCH_HEADINGS` (Task 1); `branched`, `BRANCH_MIN` (fixtures).
- Produces:
  - from `page.ts` (newly exported, unchanged in behaviour): `escapeHtml`, `plain`, `badge`, `bullets`, `count`, `glance`, `terms`, `files`, `decision`, `lensSlides`, `type Slide`, `type Ground`; new `needsBrowser(parts, drawn): boolean` and `deck(title, all, browser, fonts?): string`
  - `branchPage(report: Report, plan: Plan, fonts?: Fonts, drawn?: readonly (string | undefined)[]): string` in `branch-page.ts`

The look is the approved mock's: open `~/.local/share/niri-tasks/reviews/branch-report-mock-cbb49ff3-20261027.html` beside you.

- [ ] **Step 1: Write the failing tests**

Add to `claude/refine-mod/tests/page.test.ts` (import `branchPage` from `../hooks/branch-page`, `branched` and `BRANCH_MIN` from `./branch-fixtures`):

```ts
describe('a branch report', () => {
  const PLAN = { description: 'feat: New words', notes: ['Steps: Add the words'] }
  const every = branched({
    ...BRANCH_MIN,
    terms: [{ term: 'twin', meaning: 'The .md copy.' }],
    sections: ['newcomer', 'code', 'before-after', 'data-flow', 'structure', 'outside-tools', 'styling'].map(kind => ({
      kind,
      heading: `The ${kind}`,
      points: ['p'],
    })),
    plan_vs_done: [{ item: 'Add the words', kind: 'step', progress: 'partial', shown_by: 'abc1234' }],
  })

  test('keeps the branch order, whatever order the parts came in', () => {
    expect(slides(branchPage(every, PLAN))).toEqual([
      'cover', 'glance', 'hot-spots', 'terms', 'story', 'before-after', 'map', 'structure', 'data-flow',
      'outside-tools', 'styling', 'code', 'suite', 'new-tests', 'newcomer', 'plan-vs-done', 'files', 'decision',
    ])
  })

  test('leaves out Words, Plan vs done and Files when there are none', () => {
    const page = branchPage(branched({ ...BRANCH_MIN, files: [] }), PLAN)
    expect(slides(page)).toEqual(['cover', 'glance', 'hot-spots', 'story', 'map', 'code', 'suite', 'new-tests', 'decision'])
  })

  test('the cover gives the branch, its base and its numbers from git', () => {
    const page = branchPage(branched(), PLAN)
    const cover = page.slice(page.indexOf('id="cover"'), page.indexOf('id="glance"'))
    expect(cover).toContain('<span class="pill">Branch report</span>')
    expect(cover).toContain('task/feat-new-words-0b8f6a52 ← main at f00dfee')
    expect(cover).toContain('<b>2</b><span>commits</span>')
    expect(cover).toContain('<b class="plus">+12</b>')
    expect(cover).toContain('<b class="minus">−3</b>')
    expect(cover).toContain('1 file has changes not yet committed; this report leaves them out.')
  })

  test('the commit story shows each commit with its subject', () => {
    expect(branchPage(branched(), PLAN)).toContain('<li><code>abc1234</code>feat: add the words</li>')
  })

  test('the suite says nothing broke, or how many failed', () => {
    expect(branchPage(branched(), PLAN)).toContain('nothing broke</span> 3 tests passed across 1 suite.')
    const failing = branched({ ...BRANCH_MIN, suite: { runs: [{ suite: 'cargo test', command: 'cargo test', passed: 1200, failed: 2 }] } })
    expect(branchPage(failing, PLAN)).toContain('2 failing</span> 1,200 passed, 2 failed across 1 suite.')
  })

  test('a suite not run is shown dashed, with why', () => {
    const page = branchPage(
      branched({ ...BRANCH_MIN, suite: { ...BRANCH_MIN.suite, not_run: [{ suite: 'e2e-panel.sh', why: 'Nests a niri.' }] } }),
      PLAN,
    )
    expect(page).toContain('<div class="run skipped"><p class="head"><span class="badge notrun">not run</span> <strong>e2e-panel.sh</strong></p>')
  })

  test('the new tests name their layer and result', () => {
    const page = branchPage(branched(), PLAN)
    expect(page).toContain('<code>words_are_new</code>')
    expect(page).toContain('<span class="badge unit">unit</span>')
    expect(page).toContain('<span class="badge pass">pass</span>')
  })

  test('escapes what git and the model give', () => {
    const report = branched()
    const odd = { ...report, branch: { ...report.branch!, story: [{ step: '<b>', commits: [{ sha: 'abc1234', subject: '<script>' }], points: ['p'] }] } }
    const page = branchPage(odd, PLAN)
    expect(page).not.toContain('<script>')
    expect(page).toContain('&lt;script&gt;')
  })

  test('a plan report renders as it did', () => {
    expect(slides(reportPage(report(), PLAN))[0]).toBe('cover')
    expect(reportPage(report(), PLAN)).toContain('<span class="pill">Refine report</span>')
  })
})
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: `page.test.ts` fails to load: `hooks/branch-page` does not exist.

- [ ] **Step 3: Share the plan deck's pieces**

In `claude/refine-mod/hooks/page.ts`:

1. Put `export` before `escapeHtml`, `plain`, `count`, `badge`, `bullets`, `glance`, `terms`, `part`, `files`, `decision`, `lensSlides`, and the `Ground` and `Slide` types. Nothing else about them changes.
2. Count a branch report's own words in `readingMinutes`: add `...branchProse(report.branch),` to the `prose` list, with, above it:

```ts
// A branch report's own words, read as the rest are.
const branchProse = (b?: Branch): string[] =>
  b === undefined
    ? []
    : [
        ...b.hot_spots.flatMap(h => [h.spot, h.why]),
        ...b.story.flatMap(s => [s.step, ...s.points]),
        ...b.map.flatMap(m => m.items.map(i => i.does)),
        ...b.new_tests.map(t => t.pins),
        ...b.plan_vs_done.flatMap(p => [p.item, p.shown_by]),
      ]
```

(import `Branch` as a type from `./report`).
3. Move the page's frame out of `reportPage` into two exports, and use them there:

```ts
// Whether any diagram is left to Mermaid in the browser: one the renderer
// did not draw.
export const needsBrowser = (parts: readonly Section[], drawn: readonly (string | undefined)[]): boolean =>
  parts.some((s, i) =>
    [s.diagram, s.before, s.after].some((d, slot) => d !== undefined && drawn[i * SLOTS + slot] === undefined),
  )

// The whole page: the policy before anything it governs, then the title,
// the stylesheet and Mermaid when a diagram needs it, then the slides.
export const deck = (title: string, all: readonly Slide[], browser: boolean, fonts?: Fonts): string =>
  [
    '<!doctype html>',
    '<html lang="en">',
    '<head>',
    `<meta http-equiv="Content-Security-Policy" content="${browser ? CSP : CSP_STATIC}">`,
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${escapeHtml(title)}</title>`,
    `<style>${slideCss(fonts)}</style>`,
    browser ? `<script src="${MERMAID_URL}" integrity="${MERMAID_SRI}" crossorigin="anonymous"></script>` : '',
    '</head>',
    '<body>',
    rail(all),
    '<main class="deck">',
    ...all.map((s, i) => frame(s, i, all)),
    '</main>',
    '</body>',
    '</html>',
  ]
    .filter(Boolean)
    .join('\n') + '\n'
```

and end `reportPage` with `return deck(report.title, all, needsBrowser(parts, drawn), fonts)` in place of its own head-to-`</html>` list and its `browser` computation.

- [ ] **Step 4: Add the branch slides' styles**

In `claude/refine-mod/hooks/style.ts`, append to the `CSS` template, before its closing backtick:

```css
.facts { display: grid; grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr)); gap: 1rem; max-width: 62rem; margin: 0 0 1.25rem; }
.fact { background: var(--white); border: 3px solid var(--black); box-shadow: 4px 4px 0 var(--black); padding: 0.7rem 0.9rem; }
.fact b { display: block; font: 700 clamp(1.5rem, 1rem + 1.5vw, 2.4rem)/1.1 var(--display); }
.fact span { font: 700 0.75rem/1.2 var(--display); letter-spacing: 0.08em; text-transform: uppercase; }
.fact .plus { color: #1F7A12; } .fact .minus { color: #B0127F; }
.branchline { font: 600 0.95rem/1.5 var(--mono); margin: 0 0 1rem; }
.spots { display: grid; gap: 1rem; max-width: none; }
.spot { border: 3px solid var(--black); background: var(--white); padding: 0.8rem 1rem; box-shadow: 4px 4px 0 var(--black); }
.spot h3 { margin: 0 0 0.35rem; }
.spot p { margin: 0 0 0.5rem; }
.spot .where code { margin-right: 0.4rem; font-size: 0.8em; }
.timeline { list-style: none; padding: 0 0 0 2.2rem; margin: 0; position: relative; max-width: none; counter-reset: step; }
.timeline::before { content: ""; position: absolute; left: 0.7rem; top: 0.3rem; bottom: 0.3rem; width: 4px; background: var(--black); }
.timeline > li { position: relative; padding: 0 0 0.9rem 0; counter-increment: step; }
.timeline > li::before { content: counter(step); position: absolute; left: -2.2rem; top: 0; width: 1.8rem; height: 1.8rem;
  background: var(--yellow); border: 3px solid var(--black); font: 700 0.95rem/1.3rem var(--display); text-align: center; box-sizing: border-box; }
.timeline h3 { margin: 0 0 0.4rem; font-size: 1.15rem; }
.commits { list-style: none; padding: 0; margin: 0 0 0.5rem; display: flex; flex-wrap: wrap; gap: 0.4rem; }
.commits li { margin: 0; padding: 0.1rem 0.45rem; background: var(--white); border: 2px solid var(--black); font-size: 0.72em; }
.commits li::before { display: none; }
.commits code { background: none; border: 0; padding: 0 0.35rem 0 0; font-weight: 700; }
.timeline .points ul { columns: 2 22rem; column-gap: 2.5rem; font-size: 0.9em; margin: 0; }
.map { display: grid; grid-template-columns: repeat(auto-fill, minmax(22rem, 1fr)); gap: 1rem; max-width: none; }
.mapfile { border: 3px solid var(--black); background: var(--white); padding: 0.7rem 0.9rem; }
.mapfile .head { display: flex; gap: 0.5rem; align-items: baseline; margin: 0 0 0.5rem; flex-wrap: wrap; }
.mapfile dl { margin: 0; font-size: 0.9em; }
.mapfile dt { font: 600 0.85em/1.3 var(--mono); }
.mapfile dd { margin: 0 0 0.5em; }
.verdict { font: 700 clamp(1.3rem, 1rem + 1vw, 1.9rem)/1.3 var(--display); max-width: none; }
.verdict .badge { font-size: 0.6em; vertical-align: middle; margin-right: 0.4rem; }
.suites { display: grid; grid-template-columns: repeat(auto-fill, minmax(19rem, 1fr)); gap: 1rem; max-width: none; }
.run { border: 3px solid var(--black); box-shadow: 4px 4px 0 var(--black); padding: 0.6rem 0.9rem; background: var(--white); margin: 0; }
.run p { margin: 0 0 0.4rem; }
.run code { font-size: 0.75em; }
.run .nums b { font: 700 1.6rem/1 var(--display); }
.run.skipped { background: var(--paper); border-style: dashed; box-shadow: none; }
.badge.pass, .badge.done { background: var(--green); }
.badge.fail { background: var(--pink); }
.badge.notrun, .badge.notyet { background: var(--white); border-style: dashed; }
.badge.partial { background: var(--yellow); }
.badge.unit { background: var(--blue); } .badge.integration { background: var(--cream); } .badge.e2e { background: var(--pink); }
table.tests { font-size: 0.85em; }
table.tests td:first-child { width: 34%; }
table.tests td:first-child code { background: none; border: 0; padding: 0; font-size: 0.8em; font-weight: 700; overflow-wrap: anywhere; }
table.tests td:nth-child(2), table.tests td:nth-child(4) { white-space: nowrap; }
table.tests .muted { font-size: 0.85em; }
.gaps { margin-top: 1.25rem; }
```

(The badge classes `pass`, `fail` and the rest share names with no plan-report badge, so a plan report's look is unchanged.)

- [ ] **Step 5: Write `branch-page.ts`**

Create `claude/refine-mod/hooks/branch-page.ts`:

```ts
// A branch report as slides: the plan report's renderer, styles and lenses,
// in the branch's own order, with the slides only a branch has: where to
// look hardest, how the commits got here, what each file now does, whether
// anything broke, the tests it adds, and its plan against what is done.
// No `$`, so the unit tests reach it directly.

import type { Plan } from './plan'
import { BRANCH_HEADINGS as H } from './report'
import type { Commit, Facts, HotSpot, Lens, MapEntry, NewTest, PlanItem, Progress, Report, Result, Step, Suite, Untested } from './report'
import {
  badge, bullets, count, deck, decision, escapeHtml, files, glance, inline, lensSlides, needsBrowser, plain,
  readingMinutes, terms,
} from './page'
import type { Slide } from './page'
import type { Fonts } from './style'

const RESULT_CLASS: Record<Result, string> = { pass: 'pass', fail: 'fail', 'not run': 'notrun' }
const PROGRESS_CLASS: Record<Progress, string> = { done: 'done', partial: 'partial', 'not yet': 'notyet' }

const path = (p: string): string => `<code class="path">${escapeHtml(p).replace(/\//g, '/<wbr>')}</code>`
const number = (n: number): string => n.toLocaleString('en-AU')

const cover = (report: Report, f: Facts, meta: string): string =>
  [
    '<p class="kicker">Branch report</p>',
    `<h1>${plain(report.title)}</h1>`,
    `<p class="branchline">${escapeHtml(f.branch)} ← ${escapeHtml(f.base)} at ${escapeHtml(f.merge_base)}</p>`,
    '<div class="facts">',
    `<div class="fact"><b>${number(f.commits)}</b><span>commits</span></div>`,
    `<div class="fact"><b>${number(f.files)}</b><span>files</span></div>`,
    `<div class="fact"><b class="plus">+${number(f.added)}</b><span>lines added</span></div>`,
    `<div class="fact"><b class="minus">−${number(f.removed)}</b><span>lines removed</span></div>`,
    '</div>',
    f.uncommitted > 0
      ? `<p class="hint">${count(f.uncommitted, 'file')} ${f.uncommitted === 1 ? 'has' : 'have'} changes not yet committed; this report leaves them out.</p>`
      : '',
    `<p class="meta">${meta}</p>`,
  ]
    .filter(Boolean)
    .join('\n')

const hotSpots = (list: readonly HotSpot[]): string =>
  `<h2>${H.hotSpots}</h2>\n<div class="spots">${list
    .map(h => `<div class="spot"><h3>${plain(h.spot)}</h3><p>${inline(h.why)}</p><p class="where">${h.files.map(path).join(' ')}</p></div>`)
    .join('\n')}</div>`

const story = (steps: readonly Step<Commit>[]): string =>
  `<h2>${H.story}</h2>\n<ol class="timeline">${steps
    .map(
      s =>
        `<li><h3>${plain(s.step)}</h3><ul class="commits">${s.commits
          .map(c => `<li><code>${escapeHtml(c.sha)}</code>${escapeHtml(c.subject)}</li>`)
          .join('')}</ul><div class="points">${bullets(s.points)}</div></li>`,
    )
    .join('\n')}</ol>`

const changesMap = (entries: readonly MapEntry[]): string =>
  `<h2>${H.map}</h2>\n<div class="map">${entries
    .map(
      m =>
        `<div class="mapfile"><p class="head">${badge(m.change)} ${path(m.path)}</p><dl>${m.items
          .map(i => `<dt>${escapeHtml(i.name)}</dt><dd>${inline(i.does)}</dd>`)
          .join('')}</dl></div>`,
    )
    .join('\n')}</div>`

// Did anything break: one line that answers it, then each suite run and
// each not run, and the failing tests by name.
const suite = (s: Suite): string => {
  const passed = s.runs.reduce((n, r) => n + r.passed, 0)
  const failed = s.runs.reduce((n, r) => n + r.failed, 0)
  const across = `across ${count(s.runs.length, 'suite')}.`
  const verdict =
    failed === 0
      ? `<p class="verdict"><span class="badge pass">nothing broke</span> ${number(passed)} tests passed ${across}</p>`
      : `<p class="verdict"><span class="badge fail">${number(failed)} failing</span> ${number(passed)} passed, ${number(failed)} failed ${across}</p>`
  const runs = s.runs.map(
    r =>
      `<div class="run"><p class="head"><span class="badge ${r.failed === 0 ? 'pass' : 'fail'}">${r.failed === 0 ? 'passed' : 'failed'}</span> <strong>${plain(r.suite)}</strong></p>` +
      `<p class="nums"><b>${number(r.passed)}</b> passed · <b>${number(r.failed)}</b> failed</p><code>${escapeHtml(r.command)}</code></div>`,
  )
  const notRun = s.not_run.map(
    n =>
      `<div class="run skipped"><p class="head"><span class="badge notrun">not run</span> <strong>${plain(n.suite)}</strong></p><p>${inline(n.why)}</p></div>`,
  )
  return [
    `<h2>${H.suite}</h2>`,
    verdict,
    `<div class="suites">${[...runs, ...notRun].join('')}</div>`,
    s.failing.length > 0
      ? `<h3>Failing</h3><ul>${s.failing.map(f => `<li><code>${escapeHtml(f.test)}</code> ${path(f.file)}: ${plain(f.why)}</li>`).join('')}</ul>`
      : '',
  ]
    .filter(Boolean)
    .join('\n')
}

const newTests = (tests: readonly NewTest[], untested: readonly Untested[]): string =>
  [
    `<h2>${H.newTests}</h2>`,
    tests.length === 0
      ? '<p class="lede">This branch adds no tests.</p>'
      : [
          '<table class="tests">',
          '<thead><tr><th>Test</th><th>Layer</th><th>What it pins</th><th>Result</th></tr></thead>',
          `<tbody>${tests
            .map(
              t =>
                `<tr><td><code>${escapeHtml(t.test)}</code><br><span class="muted">${escapeHtml(t.file)}</span></td>` +
                `<td><span class="badge ${t.layer}">${t.layer}</span></td><td>${inline(t.pins)}</td>` +
                `<td><span class="badge ${RESULT_CLASS[t.result]}">${t.result}</span></td></tr>`,
            )
            .join('')}</tbody>`,
          '</table>',
        ].join('\n'),
    untested.length > 0
      ? `<div class="gaps"><h3>${H.untested}</h3><ul>${untested
          .map(u => `<li>${path(u.file)} <strong>${plain(u.code)}</strong>: ${plain(u.why)}</li>`)
          .join('')}</ul></div>`
      : '',
  ]
    .filter(Boolean)
    .join('\n')

const planVsDone = (items: readonly PlanItem[]): string =>
  [
    `<h2>${H.planVsDone}</h2>`,
    '<table>',
    '<thead><tr><th>From the plan</th><th>Progress</th><th>Shown by</th></tr></thead>',
    `<tbody>${items
      .map(
        i =>
          `<tr><td><span class="muted">${i.kind === 'step' ? 'Step' : 'Done when'}</span><br>${inline(i.item)}</td>` +
          `<td><span class="badge ${PROGRESS_CLASS[i.progress]}">${i.progress}</span></td><td>${inline(i.shown_by)}</td></tr>`,
      )
      .join('')}</tbody>`,
    '</table>',
  ].join('\n')

// The branch deck, in the branch's own order: the cover, at a glance,
// where to look hardest, the words, the commit story, before and after,
// the changes map, the structural lenses and the code, the test suite and
// the new tests, the newcomer, plan vs done, the files and the plan.
export const branchPage = (
  report: Report,
  plan: Plan,
  fonts?: Fonts,
  drawn: readonly (string | undefined)[] = [],
): string => {
  const b = report.branch
  if (b === undefined) throw new Error('branchPage needs a branch report')
  const lenses = lensSlides(report.sections, drawn)
  const of = (kind: Lens): Slide[] => lenses.filter(s => s.id === kind || s.id.startsWith(`${kind}-`))
  const rest: Slide[] = [
    { id: 'glance', pill: 'At a glance', name: 'At a glance', ground: 'yellow', body: glance(report) },
    { id: 'hot-spots', pill: 'Review hot spots', name: 'Review hot spots', ground: 'pink', body: hotSpots(b.hot_spots) },
    ...(report.terms.length > 0
      ? [{ id: 'terms', pill: 'Words used here', name: 'Words used here', ground: 'blue', body: terms(report.terms) } as Slide]
      : []),
    { id: 'story', pill: 'Commit story', name: 'Commit story', ground: 'cream', body: story(b.story) },
    ...of('before-after'),
    { id: 'map', pill: 'Changes map', name: 'Changes map', ground: 'paper', body: changesMap(b.map) },
    ...of('structure'),
    ...of('data-flow'),
    ...of('outside-tools'),
    ...of('styling'),
    ...of('code'),
    { id: 'suite', pill: 'Test suite', name: 'Test suite', ground: 'green', body: suite(b.suite) },
    { id: 'new-tests', pill: 'New tests', name: 'New tests', ground: 'blue', body: newTests(b.new_tests, b.untested) },
    ...of('newcomer'),
    ...(b.plan_vs_done.length > 0
      ? [{ id: 'plan-vs-done', pill: 'Plan vs done', name: 'Plan vs done', ground: 'cream', body: planVsDone(b.plan_vs_done) } as Slide]
      : []),
    ...(report.files.length > 0 ? [{ id: 'files', pill: 'Files', name: 'Files', ground: 'paper', body: files(report.files) } as Slide] : []),
    { id: 'decision', pill: 'The plan', name: 'The plan', ground: 'ink', body: decision(plan) },
  ]
  const meta = `About ${readingMinutes(report)} min to read · ${count(rest.length + 1, 'slide')}`
  const all: Slide[] = [
    { id: 'cover', pill: 'Branch report', name: 'Start', ground: 'cream', body: cover(report, b.facts, meta) },
    ...rest,
  ]
  return deck(report.title, all, needsBrowser(report.sections, drawn), fonts)
}
```

- [ ] **Step 6: Run the tests**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: every test passes; the plan report's page tests unchanged.

- [ ] **Step 7: Look at it**

Render the fixtures' fullest report and screenshot each new slide; compare with the mock:

```bash
S="$(mktemp -d)"
cat > "$S/render.ts" <<EOF
import { branchPage } from '$PWD/claude/refine-mod/hooks/branch-page'
import { BRANCH_MIN, branched } from '$PWD/claude/refine-mod/tests/branch-fixtures'
await Bun.write('$S/deck.html', branchPage(branched(), { description: 'feat: New words', notes: [] }))
EOF
bun "$S/render.ts" && xdg-open "$S/deck.html"
```

Expected: every slide from the mock is there with the mock's look (the fixtures are smaller). Fix anything that differs before committing.

- [ ] **Step 8: Commit**

```bash
git add claude/refine-mod/hooks/page.ts claude/refine-mod/hooks/style.ts claude/refine-mod/hooks/branch-page.ts claude/refine-mod/tests/page.test.ts
git commit -m "feat(report): draw a branch report as its own deck

The branch deck shares the plan report's renderer, styles and
lenses, in the branch's order: hot spots, the commit story as a
timeline, a map of each changed file, whether anything broke, the
tests the branch adds, and the plan against what is done.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The branch session in the mod

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts`, `claude/refine-mod/hooks/refine.ts`, `claude/refine-mod/.claude-plugin/plugin.json`
- Test: `claude/refine-mod/tests/report.test.ts`, `claude/refine-mod/tests/refine.test.ts`

**Interfaces:**
- Consumes: everything from Tasks 1–4.
- Produces (in `report.ts`):
  - `reportPath(dir, uuid, nowMs, kind: ReportKind = 'plan')`: `<dir>/branch-<uuid8>-<stamp>.html` for a branch
  - `BRANCH_REPORT_NOTE = 'Branch report: '`; `withReportNote(task, path, nowMs, kind: ReportKind = 'plan')`
  - `madeAlready(path, kind: ReportKind = 'plan')`
  - `unexplainable(task): string | undefined`
- Session options read by `refine.ts`: `branch: boolean`, `worktree: string`, `base: string` (Rust sends these in Task 8).

- [ ] **Step 1: Write the failing pure tests**

Add to `claude/refine-mod/tests/report.test.ts`:

```ts
describe('a branch report on disk and in the notes', () => {
  test('is named branch-<uuid8>-<stamp>.html', () => {
    expect(reportPath('/r', UUID, Date.UTC(2026, 9, 6, 1, 2, 3), 'branch')).toBe('/r/branch-0b8f6a52-20261006-010203.html')
    expect(reportPath('/r', UUID, Date.UTC(2026, 9, 6, 1, 2, 3))).toBe('/r/refine-0b8f6a52-20261006-010203.html')
  })

  test('is linked as a Branch report: note, never a Report: one', () => {
    const task: Task = { uuid: UUID, status: 'pending', description: 'd', annotations: [{ entry: '20261006T010203Z', description: 'Report: /r/plan.html' }] }
    expect(withReportNote(task, '/r/b.html', Date.UTC(2026, 9, 6, 1, 2, 3), 'branch').annotations).toEqual([
      { entry: '20261006T010203Z', description: 'Report: /r/plan.html' },
      { entry: '20261006T010204Z', description: 'Branch report: /r/b.html' },
    ])
  })

  test('one a session, the second naming Explain branch', () => {
    expect(madeAlready('/r/b.html', 'branch')).toBe(
      "show_task_report: this session's branch report is made and linked from the task: /r/b.html. " +
        "Close this tab and choose Explain branch in the card's Go to list again for another.",
    )
  })

  test('any task but a deleted or gone one may have its branch explained', () => {
    const task: Task = { uuid: UUID, status: 'pending', description: 'd' }
    expect(unexplainable(task)).toBeUndefined()
    expect(unexplainable({ ...task, status: 'completed' })).toBeUndefined()
    expect(unexplainable({ ...task, status: 'deleted' })).toBe('the task is deleted')
    expect(unexplainable(undefined)).toBe('the task no longer exists')
  })
})
```

(add `unexplainable` to the file's import from `../hooks/report`.)

- [ ] **Step 2: Write the failing session tests**

Add to `claude/refine-mod/tests/refine.test.ts` (import `SHA_A`, `SHA_B`, `BRANCH_MIN` from `./branch-fixtures`):

```ts
// An Explain branch session: Rust sets report and branch, and names the
// worktree and the branch it is measured against.
const BRANCH_MODE = { uuid: UUID, reports: '/r', report: true, branch: true, worktree: '/w', base: 'main' }
const BRANCH_PATH = '/r/branch-0b8f6a52-20261006-010203.html'
const BRANCH_MD = '/r/branch-0b8f6a52-20261006-010203.md'
const MERGE_BASE = `f00dfee${'2'.repeat(33)}`
const GIT_PREFIX = ['git', '--no-optional-locks', '-C', '/w', '-c', 'core.fsmonitor=false']

// What git answers for BRANCH_MIN's branch, by the command's first word; a
// command in `fails` exits 128 with git's error.
const gitAnswer = (argv: readonly string[], fails?: string) => {
  const command = argv[6]
  if (command === fails) return { value: { exitCode: 128, stdout: '', stderr: `fatal: ${command} broke\n`, isStdoutTruncated: false, isStderrTruncated: false } }
  if (command === 'rev-parse') return ran('task/feat-new-words-0b8f6a52\n')
  if (command === 'merge-base') return ran(`${MERGE_BASE}\n`)
  if (command === 'log') return ran(`${SHA_A}\tfeat: add the words\n${SHA_B}\ttest: pin the words\n`)
  if (command === 'diff') return ran('10\t2\tsrc/words.rs\n2\t1\tREADME.md\n')
  if (command === 'status') return ran(' M README.md\n')
  return ran('')
}

// The world, with git answering for the worktree.
const branchWorld = (on: On, exports: readonly object[] = [TASK], fails?: string) => {
  const { registered, seen, writes } = engine(on, SANDBOX, { deny: 'nobody should be asked' })
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
  return { registered, runs, seen, writes }
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

test('branch mode: a plan report\'s fields are refused', { options: BRANCH_MODE }, async ($, on) => {
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
```

- [ ] **Step 3: Run them to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -20`
Expected: the new `report.test.ts` cases fail (no `kind` arguments, no `unexplainable`), and the `branch mode:` tests fail (the session treats it as a plan report session: the first registers the tool, but the call asks for a planned task, so `shown.deny` names the missing plan).

- [ ] **Step 4: The pure helpers per kind**

In `claude/refine-mod/hooks/report.ts`:

```ts
// <dir>/<refine|branch>-<uuid8>-<YYYYMMDD-HHMMSS>.html in UTC: a second
// report on the same task, after a revision, does not overwrite the first.
export const reportPath = (dir: string, uuid: string, nowMs: number, kind: ReportKind = 'plan'): string => {
  const stamp = new Date(nowMs).toISOString().slice(0, 19).replace(/-|:/g, '').replace('T', '-')
  return `${dir}/${kind === 'branch' ? 'branch' : 'refine'}-${uuid.slice(0, 8).toLowerCase()}-${stamp}.html`
}
```

```ts
// What a note linking a branch report starts with. Never `Report: `, so a
// branch report never makes the plan's report an earlier draft.
export const BRANCH_REPORT_NOTE = 'Branch report: '
```

Change `withReportNote` to take the kind, and write its prefix:

```ts
export const withReportNote = (task: Task, path: string, nowMs: number, kind: ReportKind = 'plan'): Task => {
  …
  const prefix = kind === 'branch' ? BRANCH_REPORT_NOTE : REPORT_NOTE
  return { ...task, annotations: [...notes, { entry: taskDate(at), description: `${prefix}${path}` }] }
}
```

```ts
export const madeAlready = (path: string, kind: ReportKind = 'plan'): string =>
  kind === 'branch'
    ? `show_task_report: this session's branch report is made and linked from the task: ${path}. ` +
      "Close this tab and choose Explain branch in the card's Go to list again for another."
    : `show_task_report: this session's report is made and linked from the task: ${path}. ` +
      'Close this tab and press Report on the card again for another.'

// Explain branch: why the task's branch may not be reported on, or
// undefined. Any task but a deleted one: a branch needs no plan, and a
// finished task's branch can still be read. Rust checks it has a worktree.
export const unexplainable = (task: Task | undefined): string | undefined => {
  if (task === undefined) return 'the task no longer exists'
  if (task.status === 'deleted') return 'the task is deleted'
  return undefined
}
```

- [ ] **Step 5: The branch tool spec and call**

In `claude/refine-mod/hooks/refine.ts`:

1. Imports: add `branchPage` from `./branch-page`; `branchArgv, branchTarget, countStatus, gitCommand, logArgv, mergeBaseArgv, numstatArgv, parseLog, parseNumstat, statusArgv, withGit` and the types `Git`, `Target` from `./branch`; `unexplainable` and the type `ReportKind` from `./report`; `BRANCH_LENSES` from `./report`.

2. After `reportSpec`, add the branch spec, built from the plan report's:

```ts
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
          description: 'The tests this branch adds or changes: each one\'s layer, the behaviour it pins, and its result.',
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
```

3. Above `register`, add the git read:

```ts
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
```

4. In `register`, after `const unattended = …`, add:

```ts
  // Explain branch: a report session on what the task's worktree branch has
  // changed, not on its plan. Rust names the worktree and the branch it is
  // measured against; without both, no report tool is offered.
  const kind: ReportKind = reportOnly && options.branch === true ? 'branch' : 'plan'
  const target = kind === 'branch' ? branchTarget(options.worktree, options.base) : undefined
```

and in `session.start` register the report tool only when it can work:

```ts
      if (reports !== undefined && (kind === 'plan' || target !== undefined)) {
        await $.tool.register(kind === 'branch' ? branchSpec() : reportSpec(reportOnly))
      }
```

5. Add the branch call as a closure in `register`, after `fonts`:

```ts
  // The branch report: the task read as it is now, the fields checked, git
  // read for the branch's facts and its commits' subjects, the deck built
  // from its twin, opened, and linked as a Branch report: note, never as the
  // plan's Report:.
  const explainBranch = async (
    $: EngineInterface,
    e: Record<string, unknown>,
    uuid: string,
    folder: string,
    at: Target,
  ) => {
    if (linked !== undefined) return { deny: madeAlready(linked, 'branch') }
    const exported = await $.process.run([...TASK, 'rc.json.array=on', uuid, 'export'])
    if (exported.exitCode !== 0) {
      return { deny: `task export failed (${exported.exitCode}): ${exported.stderr.trim()}` }
    }
    const task = parseExport(exported.stdout)
    const why = unexplainable(task)
    if (why !== undefined || task === undefined) return { deny: `show_task_report: nothing written: ${why}.` }

    const parsed = parseReport(e, 'branch')
    if (typeof parsed === 'string') return { deny: `show_task_report: ${parsed}` }
    const git = await readGit($, at)
    if (typeof git === 'string') return { deny: `show_task_report: nothing written: ${git}.` }
    const branch = withGit(parsed.branch ?? ({} as never), git)
    if (typeof branch === 'string') return { deny: `show_task_report: ${branch}` }

    const shown = { description: task.description, notes: notesOf(task) }
    const twin = twinOf({ ...parsed.report, branch }, shown)
    if (typeof twin === 'string') {
      return {
        deny:
          `show_task_report: nothing written: ${twin}. The task's own description and notes are reported ` +
          'as they are; one on several lines must be made one line first.',
      }
    }
    const path = reportPath(folder, uuid, await $.clock.now(), 'branch')
    const note = notePath(path, home)
    const page = branchPage(twin.report, twin.plan, await readFonts($, fonts), await drawDiagrams($, twin.report))
    await $.fs.write(path, page)
    await $.fs.write(markdownPath(path), twin.markdown)
    $.ui.log(`Branch report: ${note}`)
    for (const warning of parsed.warnings) $.ui.log(`Over its word limit: ${warning}`)
    if (!unattended) await $.process.run(openArgv(path))
    const opened = unattended ? '' : ', opened it in the browser,'

    const imported = await $.process.run([...TASK, 'rc.verbose=nothing', 'import'], {
      stdin: JSON.stringify([withReportNote(task, note, await $.clock.now(), 'branch')]),
    })
    if (imported.exitCode !== 0) {
      return {
        deny:
          `Wrote the branch report to ${note}${opened} but task import failed (${imported.exitCode}): ` +
          `${imported.stderr.trim()} The task does not link it.`,
      }
    }
    linked = note
    return {
      result:
        `Wrote the branch report to ${note}${opened} and linked it from the task as the note ` +
        `"Branch report: ${note}".` + overLimits(parsed.warnings),
    }
  }
```

`parsed.branch` is always present for kind `'branch'` (Task 1); the `?? ({} as never)` only satisfies the type checker.

6. In the report tool's `tool.call` handler, right after the `armedUuid` / `reports` check, add:

```ts
    if (kind === 'branch') {
      if (target === undefined) return { deny: 'show_task_report is not armed in this session.' }
      return explainBranch($, e, uuid, reports, target)
    }
```

- [ ] **Step 6: Declare the options**

In `claude/refine-mod/.claude-plugin/plugin.json`, add to `userConfig`:

```json
    "branch": {
      "type": "boolean",
      "title": "Branch report",
      "description": "With report set: the session is an Explain branch on the task's worktree branch, not a report of its plan. show_task_report takes the branch report's fields, reads the branch's facts and commits from git in worktree, measured against base, and links the report as a Branch report: <path> note. Unset, a report session reports on the plan.",
      "required": false,
      "default": false
    },
    "worktree": {
      "type": "string",
      "title": "Worktree",
      "description": "With branch set: the absolute folder of the task's worktree, which the mod reads with git. Unset, no report tool is offered in a branch session.",
      "required": false,
      "default": ""
    },
    "base": {
      "type": "string",
      "title": "Base branch",
      "description": "With branch set: the branch the task's branch is measured against, the repository's default branch. Unset, no report tool is offered in a branch session.",
      "required": false,
      "default": ""
    }
```

and append to its `description`: ` With \`branch\` set too, an Explain branch session: show_task_report reports on what the task's worktree branch has changed, read from git, and links it as a Branch report: note.`

- [ ] **Step 7: Run the tests and validate**

Run: `cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -15`
Expected: it validates; every test passes, the plan report and refine tests unchanged.

- [ ] **Step 8: Commit**

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/hooks/refine.ts claude/refine-mod/.claude-plugin/plugin.json claude/refine-mod/tests/report.test.ts claude/refine-mod/tests/refine.test.ts
git commit -m "feat(report): explain a task's branch in a session of its own

With the branch option, a report session's show_task_report takes
the branch report's fields, reads the branch's facts and commit
subjects from git in the task's worktree, writes the branch deck
and its twin, and links it as a Branch report: note, leaving the
plan's Report: note as it is.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The report-branch skill, its catalogue and worked example

**Files:**
- Create: `claude/refine-mod/tests/example-branch-report.ts`
- Create: `.claude/skills/report-branch/SKILL.md`
- Create: `.claude/skills/report-branch/branch-catalogue.md`
- Modify: `install.sh`
- Test: `claude/refine-mod/tests/report.test.ts`, `claude/refine-mod/tests/markdown.test.ts`, `claude/refine-mod/tests/page.test.ts`

**Interfaces:**
- Consumes: `parseReport`, `withGit`, `twinOf`, `branchPage`.
- Produces: `EXAMPLE_BRANCH` (tool input), `EXAMPLE_BRANCH_GIT: Git`, `EXAMPLE_BRANCH_PLAN: Plan`; the skill `/report-branch <uuid> <base>`, which Rust prompts (Task 8).

- [ ] **Step 1: Write the worked example**

Create `claude/refine-mod/tests/example-branch-report.ts`. It is the approved mock's content, the peek branch (`99dc3a3..47c4989`), as `show_task_report` takes it. Tests render it; the catalogue shows it. Keep the two in step.

```ts
// The branch catalogue's worked example, as show_task_report takes it in an
// Explain branch session: the peek branch, task cbb49ff3, "Peek only the
// focused herdr pane's task card", 99dc3a3..47c4989. Tests render it; the
// catalogue shows it. Keep the two in step.
import type { Git } from '../hooks/branch'

export const EXAMPLE_BRANCH = {
  "title": "feat: Peek only the focused herdr pane's task card",
  "summary": "Selecting a herdr pane now peeks that pane's task, alone and whole, out of the panel's edge; the old one-line list of every task is gone.",
  "changes": [
    "Peek: the edge shows one card, the focused pane's task, with its notes and buttons.",
    "New command: `niritasks task peek <uuid8>` or `--clear`, run by herdr on every focus.",
    "Buttons: the peeked card's buttons work by click; Go to and Move take the keyboard first."
  ],
  "unchanged": [
    "Mod+Alt+Ctrl+T still opens the full list with its filter tabs.",
    "The task-docs.panel action and its key are as they were."
  ],
  "terms": [
    { "term": "Peek", "meaning": "The card that pokes out of the tucked-away panel's edge." },
    { "term": "Tucked away", "meaning": "The panel hidden past the screen edge, without the keyboard." },
    { "term": "Focus hook", "meaning": "A command herdr runs when a pane, tab or workspace is selected." }
  ],
  "hot_spots": [
    {
      "spot": "Clicks on a card that never has the keyboard",
      "why": "Go to and Move now take the keyboard mid-click; a stale peek must do nothing.",
      "files": ["src/panel/state.rs", "src/daemon.rs"]
    },
    {
      "spot": "Three focus events per switch",
      "why": "One switch can fire all three; correctness rests on the daemon ignoring a repeat.",
      "files": ["herdr/task-docs/task_docs.py", "src/panel/state.rs"]
    },
    {
      "spot": "Surface input region",
      "why": "Only the one card may take clicks; the e2e checks this by screenshot, not by click.",
      "files": ["src/panel/surface.rs", "tests/e2e-panel.sh"]
    }
  ],
  "story": [
    { "step": "Planned the peek", "commits": ["7e677bd"], "points": ["A 2,000-line plan: hooks, IPC, daemon, panel, docs."] },
    {
      "step": "The panel shows one card; the CLI asks for it",
      "commits": ["3b5f037", "c27b9d7", "9b9dda1"],
      "points": ["State first, with unit tests, then the request that sets it.", "A fix kept a tucked card from stealing focus."]
    },
    {
      "step": "The e2e test checks the peek on screen",
      "commits": ["ef204a9", "49ec6e6"],
      "points": ["Screenshots now measure one card; a peek that failed now fails the test."]
    },
    {
      "step": "herdr drives it",
      "commits": ["36f9d36", "3bf8fde"],
      "points": ["The plugin's own repeat check was dropped: the daemon already ignores repeats."]
    },
    {
      "step": "Docs catch up",
      "commits": ["8576568", "2b2b7c1", "079ce8d", "47c4989"],
      "points": ["CONTEXT.md, README and llms.txt name the peek; old hover wording gone."]
    }
  ],
  "sections": [
    {
      "kind": "before-after",
      "heading": "The panel's edge, before and after",
      "look_at": "Before, every task poked out a line; after, only the selected pane's task does.",
      "before": { "mermaid": "flowchart LR\n  edge[Panel edge] --> l1[Task A, one line]\n  edge --> l2[Task B, one line]\n  edge --> l3[Task C, one line]" },
      "after": { "mermaid": "flowchart LR\n  pane[Focused herdr pane] --> peek[Its task card, whole]:::new\n  peek --> notes[Notes and buttons]:::new" },
      "points": ["Nothing peeks until a pane on a task is selected.", "A pane on no task clears the edge.", "A task leaving pending drops its peek."]
    },
    {
      "kind": "data-flow",
      "heading": "From a pane switch to the card on screen",
      "look_at": "Three focus events all resolve to the one focused pane, then one IPC line.",
      "diagram": { "mermaid": "sequenceDiagram\n  participant H as herdr\n  participant P as task_docs.py\n  participant N as niritasks\n  participant D as daemon\n  H->>P: pane/tab/workspace.focused\n  rect rgb(153,232,133)\n  P->>H: pane list (find focused pane)\n  P->>N: task peek uuid8 | --clear\n  N->>D: peek <tag> <uuid>\n  D->>D: remember per tag, redraw panels showing it\n  end" },
      "points": [
        "The plugin remembers nothing: the daemon ignores a peek it already shows.",
        "The peek is kept per workspace tag, so a monitor switched later shows it.",
        "Failures go to herdr's plugin log, never a notification."
      ]
    },
    {
      "kind": "outside-tools",
      "heading": "herdr is now relied on for focus events",
      "points": ["The plugin needs herdr 0.9.3, the first with pane, tab and workspace focus events."],
      "tools": [
        { "tool": "herdr", "how": "Plugin [[events]] hooks run `task_docs.py focused` on each focus.", "swap": "medium", "why": "One plugin file and one Python function speak herdr; the daemon never does." }
      ]
    },
    {
      "kind": "code",
      "heading": "The panel decides what to show from the peek",
      "points": ["Tucked away, `visible` is just the peeked card, with its buttons.", "The same peek twice returns no effects, so nothing redraws."],
      "code": [
        {
          "file": "src/panel/state.rs",
          "before": "let picked = self.shown_filter().pick(&self.all);",
          "after": "if !self.keyboard {\n    return self.peeked().map(|card| shown_card(card.clone(), &[])).into_iter().collect();\n}\nlet picked = self.filter.pick(&self.all);"
        }
      ]
    },
    {
      "kind": "newcomer",
      "heading": "Could a newcomer follow it?",
      "points": [
        "Yes: CONTEXT.md now defines Peek, and the README shows the focus hooks.",
        "The state.rs tests read as a spec of what the peek does.",
        "Watch out: `row_card` and `shows_notes` now branch on the keyboard."
      ]
    }
  ],
  "map": [
    {
      "path": "herdr/task-docs/task_docs.py",
      "change": "change",
      "items": [
        { "name": "focused_pane()", "does": "Asks herdr for its panes and picks the focused one and its folder." },
        { "name": "peek(*args)", "does": "Runs `niritasks task peek`; logs a failure to stderr." },
        { "name": "focused_action()", "does": "The hook: peek the pane's task, or clear." }
      ]
    },
    { "path": "src/main.rs", "change": "change", "items": [{ "name": "TaskCommand::Peek", "does": "Resolves uuid8, finds the session's workspace tag, sends the request." }] },
    { "path": "src/ipc.rs", "change": "change", "items": [{ "name": "Request::Peek", "does": "`peek <tag> [uuid]` on the wire, encoded and decoded." }] },
    {
      "path": "src/daemon.rs",
      "change": "change",
      "items": [
        { "name": "PEEKS", "does": "The peek per workspace tag, in the daemon's memory." },
        { "name": "remember_peek()", "does": "Sets or clears a tag's peek." }
      ]
    },
    {
      "path": "src/panel/state.rs",
      "change": "change",
      "items": [
        { "name": "set_peek()", "does": "Changes the peek; a repeat returns no effects." },
        { "name": "take_keyboard_for()", "does": "Go to or Move on the peek: takes the keyboard, then presses." },
        { "name": "row_card() / peeked()", "does": "Which card shows its button row, by keyboard or peek." }
      ]
    },
    {
      "path": "src/panel/surface.rs",
      "change": "change",
      "items": [
        { "name": "Panel::peek()", "does": "Hands the peek to the state and redraws." },
        { "name": "Panel::shows()", "does": "Whether this monitor's panel shows a workspace tag." }
      ]
    },
    { "path": "src/link.rs", "change": "change", "items": [{ "name": "live_agents_named()", "does": "Moved here from daemon.rs to be shared." }] }
  ],
  "suite": {
    "runs": [
      { "suite": "cargo test", "command": "CARGO_TARGET_DIR=$TMPDIR/target cargo test --locked", "passed": 777, "failed": 0 },
      { "suite": "Refine mod", "command": "claude plugin test claude/refine-mod", "passed": 202, "failed": 0 },
      { "suite": "task-docs plugin", "command": "python3 -m unittest discover -s herdr/task-docs/tests", "passed": 40, "failed": 0 }
    ],
    "not_run": [
      { "suite": "e2e-panel.sh, e2e-box.sh", "why": "Each nests a niri on your screen: run `bash tests/all.sh` yourself." },
      { "suite": "e2e-tag.sh", "why": "Renames a real workspace: left to `tests/all.sh`." }
    ]
  },
  "new_tests": [
    { "test": "tucked_away_only_the_peeked_card_shows_with_its_buttons", "file": "src/panel/state.rs", "layer": "unit", "pins": "Nothing peeked shows nothing; a peek shows that one card with buttons.", "result": "pass" },
    { "test": "the_same_peek_again_changes_nothing", "file": "src/panel/state.rs", "layer": "unit", "pins": "A repeat focus redraws nothing.", "result": "pass" },
    { "test": "the_keyboard_shows_every_card_and_keeps_the_peek_for_after", "file": "src/panel/state.rs", "layer": "unit", "pins": "The keyboard still lists all; Escape restores the peek.", "result": "pass" },
    { "test": "the_peeks_go_to_and_move_take_the_keyboard_on_its_card", "file": "src/panel/state.rs", "layer": "unit", "pins": "Go to and Move ask for the keyboard first.", "result": "pass" },
    { "test": "a_peek_is_kept_per_workspace_tag", "file": "src/daemon.rs", "layer": "unit", "pins": "Each workspace keeps its own peek.", "result": "pass" },
    { "test": "peek_carries_a_tag_and_an_optional_uuid", "file": "src/ipc.rs", "layer": "unit", "pins": "The wire format round-trips.", "result": "pass" },
    { "test": "test_a_focused_tasks_pane_peeks_at_its_task", "file": "herdr/task-docs/tests/test_task_docs.py", "layer": "integration", "pins": "A task pane runs `task peek <uuid8>`, against a fake herdr.", "result": "pass" },
    { "test": "test_a_pane_on_no_task_clears_the_peek", "file": "herdr/task-docs/tests/test_task_docs.py", "layer": "integration", "pins": "An unlinked pane runs `--clear`.", "result": "pass" },
    { "test": "e2e-panel.sh: the peek", "file": "tests/e2e-panel.sh", "layer": "e2e", "pins": "One card on screen in a nested niri; Escape puts it back.", "result": "not run" }
  ],
  "untested": [
    { "code": "Panel::peek and the input region", "file": "src/panel/surface.rs", "why": "GTK drawing; only the e2e screenshots cover it." },
    { "code": "TaskCommand::Peek outside a herdr session", "file": "src/main.rs", "why": "The refusal is not tested." }
  ],
  "plan_vs_done": [
    { "item": "herdr/task-docs focus hooks running `task_docs.py focused`", "kind": "step", "progress": "done", "shown_by": "36f9d36, 3bf8fde; plugin tests" },
    { "item": "An ipc Peek request", "kind": "step", "progress": "done", "shown_by": "c27b9d7; `peek_carries_a_tag_and_an_optional_uuid`" },
    { "item": "Daemon keeps the peek per tag; panel draws that card only", "kind": "step", "progress": "done", "shown_by": "3b5f037; `a_peek_is_kept_per_workspace_tag`" },
    { "item": "Update CONTEXT.md, README and the tests", "kind": "step", "progress": "done", "shown_by": "8576568, 2b2b7c1, 079ce8d" },
    { "item": "Selecting a pane, tab or workspace peeks only its task", "kind": "done when", "progress": "done", "shown_by": "Plugin tests; e2e peek check" },
    { "item": "Hovering slides it out with notes and working buttons", "kind": "done when", "progress": "partial", "shown_by": "Clicks are unit-tested; the slide-out is checked by hand" },
    { "item": "An unlinked pane leaves the edge empty", "kind": "done when", "progress": "done", "shown_by": "`test_a_pane_on_no_task_clears_the_peek`" },
    { "item": "Mod+Alt+Ctrl+T is unchanged", "kind": "done when", "progress": "done", "shown_by": "`the_keyboard_shows_every_card_and_keeps_the_peek_for_after`" },
    { "item": "cargo test and the task-docs plugin tests pass", "kind": "done when", "progress": "done", "shown_by": "Test suite: every suite passed" }
  ],
  "files": [
    { "path": "herdr/task-docs/task_docs.py", "change": "change", "why": "The focus hook: finds the focused pane's task and peeks it." },
    { "path": "herdr/task-docs/herdr-plugin.toml", "change": "change", "why": "Hooks pane, tab and workspace focus; needs herdr 0.9.3." },
    { "path": "src/ipc.rs", "change": "change", "why": "The peek request on the daemon's socket." },
    { "path": "src/main.rs", "change": "change", "why": "`niritasks task peek`." },
    { "path": "src/daemon.rs", "change": "change", "why": "Keeps the peek per tag and hands it to panels." },
    { "path": "src/panel/state.rs", "change": "change", "why": "What a tucked-away panel shows and does." },
    { "path": "src/panel/surface.rs", "change": "change", "why": "Draws the one peeked card and passes clicks." },
    { "path": "src/link.rs", "change": "change", "why": "Live agents of a named workspace, shared." },
    { "path": "tests/e2e-panel.sh", "change": "change", "why": "Screenshots the peek in a nested niri." }
  ]
}

// What git said of the peek branch: `git log --reverse --format=%H%x09%s
// 99dc3a3..47c4989` and `git diff --numstat 99dc3a3 47c4989`.
export const EXAMPLE_BRANCH_GIT: Git = {
  branch: 'task/feat-peek-only-the-focused-herdr-pane-s-cbb49ff3',
  base: 'main',
  merge_base: '99dc3a3d4b64241b80e578cfcd5260a2f4d54381',
  log: [
    { sha: '7e677bda5a93514e922edf7ceca5ff29a58c1d1a', subject: "docs: plan peeking at the focused herdr pane's task" },
    { sha: '3b5f037c07c9c0c6809c651296971364e6e32d86', subject: 'feat(panel): show only the peeked card when tucked away' },
    { sha: 'c27b9d7c7d2998c535d0281cd1c8af1bf9fbee77', subject: 'feat(ipc): add niritasks task peek' },
    { sha: '9b9dda13074ca3ca918589dbed9e267d6801b26e', subject: 'fix(panel): keep the focus off the peeked card when tucked away' },
    { sha: 'ef204a9225325127e25f35c7461dc1f0d549f9a8', subject: 'test(e2e): measure the peeked card instead of the stack' },
    { sha: '49ec6e6461d762423c9cbdd5d23351be2ec01b7e', subject: 'test(e2e): fail a peek that never happened' },
    { sha: '36f9d361c58152313f9d7dbe93282c250a7ccd38', subject: "feat(task-docs): peek at the focused pane's task" },
    { sha: '85765680889d596b6b81aa6a75333bf3b4f3a3e1', subject: "docs: describe the peek of the focused herdr pane's task" },
    { sha: '2b2b7c1ec782ca146e1a3e5082464b14a5a4f5ed', subject: 'docs: drop the old stacked-peek wording from the README' },
    { sha: '3bf8fdef6fd7b5efc6deee9b9bbc3acb7272f7d3', subject: 'fix(task-docs): peek on every focus event' },
    { sha: '079ce8dfb352ba777fed1065172b2acfe6202d4b', subject: 'docs(panel): drop the hover and one-line folding wording' },
    { sha: '47c4989d7f781756bc11a6887ca31217783facf4', subject: 'docs: say a repeated focus changes nothing on the panel' },
  ],
  paths: [
    'CONTEXT.md', 'README.md', 'docs/superpowers/plans/2026-10-26-peek-the-focused-herdr-panes-task.md',
    'herdr/task-docs/herdr-plugin.toml', 'herdr/task-docs/task_docs.py', 'herdr/task-docs/tests/test_task_docs.py',
    'llms.txt', 'src/daemon.rs', 'src/ipc.rs', 'src/link.rs', 'src/main.rs', 'src/panel/model.rs',
    'src/panel/state.rs', 'src/panel/surface.rs', 'tests/e2e-panel.sh',
  ],
  added: 3006,
  removed: 232,
  uncommitted: 0,
}

// The task the example explains, as the deck's last slide shows it.
export const EXAMPLE_BRANCH_PLAN = {
  description: "feat: Peek only the focused herdr pane's task card",
  notes: [
    "Goal: While working in herdr, the task panel's edge shows only the task whose herdr pane is selected, as one expanded card with its notes and action row, instead of the list of every task.",
    'Steps: 1) herdr/task-docs: [[events]] hooks running `task_docs.py focused`, which finds the focused pane\'s task and runs `niritasks task peek <uuid8>` or `niritasks task peek --clear`; 2) an ipc Peek request; 3) the daemon keeps the peeked task per workspace tag and the panel of the monitor showing that workspace draws only that card, expanded; 4) update CONTEXT.md (Peek, Task panel, Task card), the README and the tests.',
    "Done when: Selecting a task's herdr pane, tab or workspace leaves only that task's card peeking on its workspace's monitor; hovering slides it out with notes and working buttons; selecting an unlinked pane leaves the edge empty; the task leaving pending clears it; Mod+Alt+Ctrl+T is unchanged; cargo test and the task-docs plugin tests pass.",
  ],
}
```

- [ ] **Step 2: Test it end to end through the pure half**

Add to `report.test.ts`:

```ts
test('takes the branch catalogue\'s worked example, within every limit', () => {
  const parsed = parseReport(EXAMPLE_BRANCH, 'branch')
  expect(typeof parsed).toBe('object')
  expect(typeof parsed === 'object' && parsed.warnings).toEqual([])
})
```

Add to `markdown.test.ts`:

```ts
test('the branch worked example reads back from its twin', () => {
  const parsed = parseReport(EXAMPLE_BRANCH, 'branch')
  if (typeof parsed === 'string' || parsed.branch === undefined) throw new Error(String(parsed))
  const branch = withGit(parsed.branch, EXAMPLE_BRANCH_GIT)
  if (typeof branch === 'string') throw new Error(branch)
  expect(typeof twinOf({ ...parsed.report, branch }, EXAMPLE_BRANCH_PLAN)).toBe('object')
})
```

Add to `page.test.ts`:

```ts
test('the branch worked example renders every branch slide', () => {
  const parsed = parseReport(EXAMPLE_BRANCH, 'branch')
  if (typeof parsed === 'string' || parsed.branch === undefined) throw new Error(String(parsed))
  const branch = withGit(parsed.branch, EXAMPLE_BRANCH_GIT)
  if (typeof branch === 'string') throw new Error(branch)
  expect(slides(branchPage({ ...parsed.report, branch }, EXAMPLE_BRANCH_PLAN))).toEqual([
    'cover', 'glance', 'hot-spots', 'terms', 'story', 'before-after', 'map', 'data-flow', 'outside-tools', 'code',
    'suite', 'new-tests', 'newcomer', 'plan-vs-done', 'files', 'decision',
  ])
})
```

(import `EXAMPLE_BRANCH`, `EXAMPLE_BRANCH_GIT`, `EXAMPLE_BRANCH_PLAN` and `withGit` where each file needs them.)

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: every test passes. A limit the example breaks shows as a warning or a refusal naming the field: shorten that line in the example, never the limit.

- [ ] **Step 3: Write the skill**

Create `.claude/skills/report-branch/SKILL.md`:

````markdown
---
name: report-branch
description: >
  Explain what a Taskwarrior task's worktree branch has changed so far, how
  it works and how it is tested, as an HTML deck opened in the browser and
  linked from the task as a Branch report: note, after running the project's
  test suite. Invoked by the Explain branch row of niri-tasks' task card's Go
  to list as `/report-branch <uuid> <base>`.
disable-model-invocation: true
argument-hint: <uuid> <base>
---

# report-branch: explain what a task's branch has changed

Arguments: `$ARGUMENTS`: a task uuid, then the base branch the task's
branch is measured against (the repository's default branch).

This session runs in the task's worktree. Your job is to explain its branch
to someone who has not read the diff: what it changes, how the new code
works, how it is tested, where a reviewer should look hardest, and how far it
has got with the task's plan. You do not review it for merging, fix it, or
carry on the task.

**You report on the branch; you never change it.** No file edits, no
commits, no config changes. The task's one change is the `Branch report:
<path>` note the report tool adds itself. You must not change the task any
other way: no `task modify`, no `task annotate`, no `task import`.

Your Bash commands run in a sandbox that can read anything but secrets and
write only to the task database and `$TMPDIR`. The worktree is read-only to
you: every build and test output goes under `$TMPDIR`. Commands reach a
fixed list of source and docs hosts and no others. A command that fails with
`Read-only file system`, `Operation not permitted`, or a refused connection
hit that fence on purpose: do not look for a way around it, and do not ask
the user to lift it.

## 1. Read the task and the branch

```bash
task rc.json.array=on <uuid> export
```

Stop and say so if it returns `[]` or its `status` is `deleted`. Show the
user the description and every note. Then select the report tool with
ToolSearch (`select:mcp__niri-tasks-refine__show_task_report`). If it is not
there, stop and tell the user the refine mod did not load: they should run
`install.sh` and check `claude --version` is 2.1.287 or later.

Measure the branch against the merge base with the base branch, committed
work only:

```bash
mb=$(git merge-base HEAD <base>)
git log --reverse --format='%h %s' "$mb"..HEAD
git diff --stat "$mb" HEAD
git diff "$mb" HEAD -- <file>       # each file worth reading
git status --short                   # uncommitted: the report leaves it out
```

Stop and say so if the log is empty: the branch has nothing to report yet.

## 2. Ground it

Read what the diff touches until you could explain it: `CONTEXT.md`,
`AGENTS.md`/`CLAUDE.md`, the code around each change, and the plan or spec
the task's notes link (`Plan:`, `Spec:`). Facts you can look up are yours to
find; never ask the user for one.

## 3. Run the tests

Find the project's suites: the README's testing section, `AGENTS.md` or
`CLAUDE.md`, a `tests/all.sh` or `Makefile` target, CI config, and the
manifests (`Cargo.toml`, `package.json`, `pyproject.toml`). Run every suite
that runs headless, with its build outputs under `$TMPDIR`:

- Rust: `CARGO_TARGET_DIR="$TMPDIR/target" cargo test --locked`
- Python: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s <dir>`
- Bun or a Claude Code mod: `bun test` or `claude plugin test <dir>`, with any
  cache pointed under `$TMPDIR`

Never run a suite that needs a display, a nested compositor, a real
workspace or a network host outside the fence: an e2e script is named, not
run. A suite the sandbox stops (a write outside `$TMPDIR`, a refused host)
is not run either, with the reason; never count it as failed. For each suite
run, take the passed and failed counts from its own summary, and name each
failing test.

Then find the tests the branch adds or changes (`git diff "$mb" HEAD` on the
test files and the `#[test]`, `def test_` or `test(` lines it adds), and for
each: its layer (unit, integration, e2e), the behaviour it pins, and its
result from the runs. Note any changed code that no test covers.

## 4. Fill the report

Read `branch-catalogue.md` beside this skill
(`~/.claude/skills/report-branch/branch-catalogue.md` when installed), and
the lenses, diagram and writing sections of the refine report catalogue it
points to. Then fill every field from what you read and ran:

- `title` is the task's description, as read in step 1.
- Name commits by sha alone: the tool reads each subject, the branch, its
  base and its numbers from git, and refuses a story that leaves a commit
  out.
- `plan_vs_done` takes each `Steps:` and `Done when:` note the task has,
  split into its items; none when it has neither.
- The tool shows the task's description and notes on the last slide; do not
  repeat them.

## 5. Show it

Call `mcp__niri-tasks-refine__show_task_report` with the fields. The tool
reads git, builds the deck, saves it under the reviews folder, opens it in
the browser, and adds a `Branch report: <path>` note to the task. Its answer
says what happened:

- **Wrote the branch report to … and linked it from the task**: go to
  step 6.
- **show_task_report: nothing written: the task is deleted** or **… no
  longer exists**: stop and tell the user.
- **show_task_report: nothing written: git … failed**: stop and tell the
  user the tool's answer.
- **show_task_report: this session's branch report is made and linked**:
  the report is done already; go to step 6.
- Any other **show_task_report: …** answer lists every field over its limit,
  not allowed, or not matching git: fix them all and call again.
- **Wrote the branch report to … but task import failed**: tell the user the
  path and the error, and stop.
- **not armed** or **show_task_report failed**: do not retry and do not
  write anything any other way: tell the user the tool's answer and stop.

## 6. Verify and report

Export the task once more and check: the description and the earlier notes
are as you read them in step 1 (a `Report:` note among them untouched), the
last note is `Branch report: <path>` with the path the tool answered, and
the tags are unchanged. Tell the user the path in one line, and stop. Leave
the session open: choosing Branch report in the Go to list opens the deck
again, and to make another they close this tab first.

Always address the task by uuid, never its numeric id.
````

- [ ] **Step 4: Write the catalogue**

Create `.claude/skills/report-branch/branch-catalogue.md`:

````markdown
# Branch report catalogue

How to fill `show_task_report` in an Explain branch session. The tool builds
the deck itself, as full-screen slides in one fixed order, from the fields
you give it and what it reads from git:

1. the cover: the title, the branch and its base, and its commits, files
   and lines, from git;
2. **At a glance**: `summary`, `changes` and `unchanged`;
3. **Review hot spots**: `hot_spots`;
4. **Words used here**: `terms`, when you give any;
5. **Commit story**: `story`, a timeline of steps;
6. **Before → after**: the `before-after` parts;
7. **Changes map**: `map`;
8. **Structure**, **Data flow**, **Outside tools**, **Styling** and **Code
   changes**: the parts with those lenses, when you give any;
9. **Test suite**: `suite`, whether anything broke;
10. **New tests**: `new_tests` and `untested`;
11. **For a newcomer**: the `newcomer` part;
12. **Plan vs done**: `plan_vs_done`, when the task has Steps or Done when
    notes;
13. **Files**: `files`;
14. **The plan**: the task's description and notes, taken from the task.

## Who reads it, and what for

One person who has not read the diff and wants to know what the branch has
done so far: to review it, to pick it up, or to decide whether it is nearly
finished. Write for them as the refine report catalogue says
(`~/.claude/skills/refine-task/report-catalogue.md`, "Who reads it, and what
for" and "Writing the lines"): conclusion first, one idea per line, plain
words, every diagram with points beside it.

The report explains; it never asks the reader to do anything.

## The fields

The plan report's fields work as its catalogue says, with three differences:
`title` is the task's description as it is; there is no `needs_your_eye`
(the hot spots take its place) and no `checks` (plan vs done takes its
place); and every part needs a `kind` other than `part`. The branch's own
fields:

| Field | What it holds | Limit |
|---|---|---|
| `hot_spots` | Where a reviewer should look hardest: a risky change, a subtle rule, something only checked by hand. | 1–4; spot 10 words, why 30, 1–4 `files` |
| `story` | The commits as steps, oldest first: what each step achieved. Every commit on the branch goes in exactly one step, named by sha (at least 7 characters). | 1–8 steps; step 10 words; 1–3 `points`, 25 words each |
| `map` | Each changed file worth naming, and each function, type or part of it the branch changes, with what it now does. Only files the branch changes. | 1–20 files; 1–6 `items`; name 80 characters, does 20 words |
| `suite.runs` | Each suite you ran: a short name, the command, and the passed and failed counts from its own summary. | 1–8; suite 6 words, command 200 characters |
| `suite.not_run` | Each suite you did not run, and why: an e2e that needs a display, or one the sandbox stopped. | 0–6; why 25 words |
| `suite.failing` | Each failing test, its file, and what it says went wrong. | 0–10; why 25 words |
| `new_tests` | Each test the branch adds or changes: its name, file, `layer` (`unit`, `integration`, `e2e`), the behaviour it pins, and its `result` (`pass`, `fail`, `not run`). | 0–30; pins 25 words |
| `untested` | Changed code no test covers, and why that matters. | 0–8; code 8 words, why 20 |
| `plan_vs_done` | Each item of the task's `Steps:` and `Done when:` notes: `kind` (`step`, `done when`), `progress` (`done`, `partial`, `not yet`), and the commit or test that shows it. | 0–20; item 30 words, shown_by 25 |

A line a word or two over its word limit is taken and named; further over,
the whole report is refused with every reason, so fix them all and call
again. A story that leaves out a commit, or names one twice, and a map that
names a file the branch does not change, are refused too.

## Filling each slide

- **Hot spots**: pick what could break quietly: a rule that rests on
  another part's behaviour, a path only an e2e or a hand check covers, a
  change to something shared. Say why in the reader's words.
- **Commit story**: group commits by what they achieved, not by type; a
  fix belongs with the feature it fixes. Two or three points per step at
  most.
- **Changes map**: name what a reader would search for: the function, the
  type, the command. Say what it does now, not how it changed.
- **Test suite**: the suites as you ran them. A suite you could not run is
  not a failure; say why it was not run.
- **New tests**: say what each test pins as a behaviour ("A repeat focus
  redraws nothing"), not what it calls.
- **Plan vs done**: split a `Steps:` note into its numbered items. Partial
  means started but not finished, or only checked by hand.
- **Lenses**: look through every lens the refine report catalogue lists
  ("The lenses", "Drawing diagrams") that applies to what the branch
  changed; leave out one with nothing to say.

## Worked example

The peek branch, task cbb49ff3, "Peek only the focused herdr pane's task
card" (`99dc3a3..47c4989`, 12 commits). It is
`claude/refine-mod/tests/example-branch-report.ts`'s `EXAMPLE_BRANCH`; the
tests render it, so keep the two in step:

```json
<EXAMPLE_BRANCH, printed by the command below>
```
````

Then fill the worked example's fence with the example exactly as the tests hold it:

```bash
bun -e 'import { EXAMPLE_BRANCH } from "./claude/refine-mod/tests/example-branch-report.ts"; console.log(JSON.stringify(EXAMPLE_BRANCH, null, 2))' \
  | python3 -c 'import sys; p = ".claude/skills/report-branch/branch-catalogue.md"; s = open(p).read(); open(p, "w").write(s.replace("<EXAMPLE_BRANCH, printed by the command below>", sys.stdin.read().rstrip("\n")))'
grep -n '<EXAMPLE' .claude/skills/report-branch/branch-catalogue.md
```

Expected: the `grep` prints nothing; the fence holds the example's JSON.

- [ ] **Step 5: Link the skill on install**

In `install.sh`, after the `report-task` link (line 93), add:

```bash
# report-branch: what a card's Explain branch opens Claude with, in the
# task's worktree. Its catalogue sits beside it.
link "$REPO/.claude/skills/report-branch/SKILL.md" "$CLAUDE_SKILLS/report-branch/SKILL.md"
link "$REPO/.claude/skills/report-branch/branch-catalogue.md" "$CLAUDE_SKILLS/report-branch/branch-catalogue.md"
```

Run: `bash -n install.sh`
Expected: no output.

- [ ] **Step 6: Run the tests**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: every test passes.

- [ ] **Step 7: Commit**

```bash
git add claude/refine-mod/tests/example-branch-report.ts claude/refine-mod/tests/report.test.ts claude/refine-mod/tests/markdown.test.ts claude/refine-mod/tests/page.test.ts .claude/skills/report-branch install.sh
git commit -m "feat(report): add the report-branch skill and its catalogue

Explain branch prompts /report-branch <uuid> <base>: read the
branch against its merge base, run every headless test suite with
its outputs under TMPDIR, and fill the branch deck as the catalogue
says. The catalogue's worked example is the peek branch, which the
tests render.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The Branch report note

**Files:**
- Modify: `src/task.rs`, `src/docs.rs`, `src/actions.rs`, `src/panel/model.rs`
- Test: the same files' `mod tests`

**Interfaces:**
- Produces:
  - `task::BRANCH_REPORT_NOTE: &str = "Branch report: "`
  - `Task::branch_report_path(&self) -> Option<&str>`, `Task::branch_report_link(&self) -> Option<Link>`
  - `ReportNote` gains `pub branch: bool` (a `Branch report:` note); `Task::report_notes()` lists them too, so `docs::reports` and `backup::missing_reports` cover branch reports
  - `TaskState.has_branch_report: bool`; `Card.branch_report: Option<Link>`

- [ ] **Step 1: Write the failing tests**

In `src/task.rs`'s tests:

```rust
    /// A branch report is linked by its own note: it never stands for the
    /// plan's report, and the plan's never stands for it.
    #[test]
    fn a_branch_report_note_is_its_own_link() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Report: /r/plan.html"},
                {"entry":"20261007T010000Z","description":"Branch report: /r/branch.html"}]}"#,
        )
        .unwrap();
        assert_eq!(t.report_path(), Some("/r/plan.html"));
        assert_eq!(t.branch_report_path(), Some("/r/branch.html"));
        assert_eq!(
            t.branch_report_link(),
            Some(Link { path: "/r/branch.html".into(), entry: "20261007T010000Z".into() })
        );
        let listed: Vec<(&str, bool, bool)> =
            t.report_notes().iter().map(|n| (n.path.as_str(), n.draft, n.branch)).collect();
        assert_eq!(listed, vec![("/r/branch.html", false, true), ("/r/plan.html", false, false)]);
    }
```

In `src/docs.rs`'s tests:

```rust
    /// A branch report is listed as one, by its twin as a plan report is.
    #[test]
    fn a_branch_report_is_listed_as_one() {
        let dir = scratch("branch-reports");
        let d = dir.display();
        file_at(&dir.join("b.md"), "# b", 1);
        let t: Task = serde_json::from_str(&format!(
            r#"{{"uuid":"u","description":"d","annotations":[
                {{"entry":"20261008T010000Z","description":"Branch report: {d}/b.html"}}]}}"#
        ))
        .unwrap();
        assert_eq!(
            reports(&t, &Dirs::at(&dir)),
            vec![Doc { label: "Branch report · 2026-10-08".into(), path: dir.join("b.md") }]
        );
    }
```

In `src/actions.rs`'s tests:

```rust
    #[test]
    fn of_reads_has_branch_report_from_the_tasks_notes() {
        let with: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","status":"pending",
                "annotations":[{"entry":"20261006T010000Z","description":"Branch report: /r/b.html"}]}"#,
        )
        .unwrap();
        assert_eq!(TaskState::of(&with, false, &[]), TaskState { has_branch_report: true, ..TaskState::default() });
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test branch_report 2>&1 | tail -15`
Expected: compile errors: no `branch_report_path`, `branch_report_link`, `ReportNote::branch`, `has_branch_report`.

- [ ] **Step 3: The note**

In `src/task.rs`, after `PLAN_NOTE`:

```rust
/// What a note linking a branch report starts with, as the refine mod
/// writes it after Explain branch: `Branch report: <path>`. Never
/// `Report: `, so a branch report never makes the plan's report an earlier
/// draft. The card's Go to list opens the newest.
pub const BRANCH_REPORT_NOTE: &str = "Branch report: ";
```

Give `ReportNote` its kind:

```rust
    /// Linked as `Report (earlier draft):`.
    pub draft: bool,
    /// Linked as `Branch report:`: what the task's branch has changed.
    pub branch: bool,
```

Add beside `report_path`/`report_link`:

```rust
    /// The branch report linked from the task's notes: the newest
    /// `Branch report: <path>` note's path. None on a task with no such note.
    pub fn branch_report_path(&self) -> Option<&str> {
        self.linked(BRANCH_REPORT_NOTE).map(|(path, _)| path)
    }

    /// [`Task::branch_report_path`] with the date its note was added.
    pub fn branch_report_link(&self) -> Option<Link> {
        self.link(BRANCH_REPORT_NOTE)
    }
```

In `report_notes`, list branch notes too:

```rust
            .filter_map(|a| {
                let d = &a.description;
                let (rest, draft, branch) = if let Some(rest) = d.strip_prefix(REPORT_NOTE) {
                    (rest, false, false)
                } else if let Some(rest) = d.strip_prefix(EARLIER_DRAFT_NOTE) {
                    (rest, true, false)
                } else {
                    (d.strip_prefix(BRANCH_REPORT_NOTE)?, false, true)
                };
                let path = rest.trim();
                (!path.is_empty()).then(|| ReportNote { path: path.to_string(), draft, branch, date: a.date() })
            })
```

and update its doc comment: "`Report:`, `Report (earlier draft):` and `Branch report:` notes alike, the second and third marked." Fix every other `ReportNote { … }` literal (`grep -n "ReportNote {" src`) to set `branch: false`.

- [ ] **Step 4: The listing, the state and the card**

`src/docs.rs`, in `reports`:

```rust
            let kind = if n.branch {
                "Branch report"
            } else if n.draft {
                "Report, earlier draft"
            } else {
                "Report"
            };
```

`src/actions.rs`: add to `TaskState`, after `has_report`:

```rust
    /// Its notes link a branch report, `Branch report: <path>`: Explain
    /// branch's, which the Go to list's Branch report opens.
    pub has_branch_report: bool,
```

and in `TaskState::of`: `has_branch_report: task.branch_report_path().is_some(),`. Add `has_branch_report: true` to the "every field" literal in its tests (`grep -n "has_report: true, has_spec" src/actions.rs`).

`src/panel/model.rs`: add to `Card`, after `report`:

```rust
    /// The branch report the task's notes link, `Branch report: <path>`,
    /// and when: its Go to list offers Branch report, with its age. None on
    /// "+N more".
    pub branch_report: Option<Link>,
```

set it in `cards` (`branch_report: t.branch_report_link(),`), in `cap`'s "+N more" card (`branch_report: None,`), and in `Card::state` (`has_branch_report: self.branch_report.is_some(),`). Every other `Card { … }` literal gets `branch_report: None` (`cargo build --tests 2>&1 | grep -A3 "missing field .branch_report."` lists them).

- [ ] **Step 5: Run the tests**

Run: `cargo test 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: every result `ok`. `backup::missing_reports` now also lists a branch note whose file is gone, through `report_notes`.

- [ ] **Step 6: Commit**

```bash
git add src/task.rs src/docs.rs src/actions.rs src/panel/model.rs
git commit -m "feat(task): read a Branch report note apart from the plan's

A Branch report: note links what Explain branch wrote. It is the
task's branch report, listed by task docs and backed up with the
others, and never stands for the plan's Report:.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: `niritasks task report --branch`

**Files:**
- Modify: `src/names.rs`, `src/work.rs`, `src/refine.rs`, `src/main.rs`, `README.md` (Commands block), `llms.txt` (commands table)
- Test: the same files' `mod tests`

**Interfaces:**
- Consumes: `Task::branch_report_path` (Task 7).
- Produces:
  - `names::branch_agent(uuid) -> String` (`branch-<uuid8>`), `names::Role::Branch` (`Branch #61 · …`), `is_task_agent` counts `branch-`
  - `work::default_branch(repo: &Path) -> Result<String>`
  - `refine::Mode::Branch`; `refine::Branch { pub worktree: PathBuf, pub base: String }`; `refine::branch_prompt(uuid, base) -> String` (`/report-branch <uuid> <base>`); `refine::launch_branch(ws, t, &Branch) -> Result<()>`
  - `session_settings(project, task_data, fence, uuid, reports, mode, branch: Option<&Branch>, unattended, status_line)`: mod options `report` (true for `Report` and `Branch`), `branch`, `worktree`, `base`
  - CLI: `niritasks task report <uuid> --branch [--fresh]`

- [ ] **Step 1: Write the failing tests**

`src/names.rs` tests:

```rust
    #[test]
    fn a_branch_report_has_its_own_agent_and_label() {
        assert_eq!(branch_agent("7CD9FD3A-d27b-4387-8249-aaf0d6785f90"), "branch-7cd9fd3a");
        assert!(is_task_agent("branch-7cd9fd3a"));
        let n = PaneName::for_task(Role::Branch, 61, "7b4f4c5c-185a", "fix the peek");
        assert!(n.label.starts_with("Branch #61 · fix the peek"), "{}", n.label);
    }
```

`src/work.rs` tests:

```rust
    /// origin's HEAD names the default branch; without a remote, the main
    /// checkout's own branch does.
    #[test]
    fn the_base_is_origins_head_else_the_checkouts_branch() {
        assert_eq!(base_branch(Some("origin/main"), Some("feature")), Some("main".into()));
        assert_eq!(base_branch(None, Some("main")), Some("main".into()));
        assert_eq!(base_branch(Some("origin/"), Some("trunk")), Some("trunk".into()));
        assert_eq!(base_branch(None, Some("")), None);
        assert_eq!(base_branch(None, None), None);
    }
```

`src/refine.rs` tests:

```rust
    #[test]
    fn a_branch_session_prompts_report_branch_with_its_base() {
        assert_eq!(branch_prompt("u-1", "main"), "/report-branch u-1 main");
        assert_eq!(agent_name(Mode::Branch, "7CD9FD3A-d27b"), "branch-7cd9fd3a");
        assert_eq!(tab_label(Mode::Branch, "fix: a thing"), "Branch: fix: a thing");
    }

    /// An Explain branch session tells the mod it is a report session on a
    /// branch, which worktree and which base; a plan report's says no branch.
    #[test]
    fn a_branch_session_tells_the_mod_its_worktree_and_base() {
        let b = Branch { worktree: PathBuf::from("/w/t"), base: "main".into() };
        let json = session_settings(Path::new("/w/t"), Path::new("/t"), &fence(&[]), "u", Path::new("/r"), Mode::Branch, Some(&b), false, None);
        let v: Value = serde_json::from_str(&json).unwrap();
        let options = &v["pluginConfigs"][REFINE_MOD]["options"];
        assert_eq!(options["report"], true);
        assert_eq!(options["branch"], true);
        assert_eq!(options["worktree"], "/w/t");
        assert_eq!(options["base"], "main");
        assert_eq!(v["sandbox"]["filesystem"]["denyWrite"][0], "/w/t", "the worktree is read-only to it");
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), "u", Path::new("/r"), Mode::Report, None, false, None);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["branch"], false);
        assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["worktree"], "");
    }

    /// Explain branch opens its own tab, in the task's worktree, beside a
    /// plan report's, prompted with its base.
    #[test]
    fn a_branch_session_opens_in_the_worktree_beside_a_report() {
        let fake = Fake::running(&[("w1", "alpha")]);
        open(&fake, Mode::Report).unwrap();
        let session = Session::with_port("alpha", PathBuf::from("/p/alpha"), Box::new(fake.clone()));
        open_in(&session, &tk(), "alpha", &refiner(), Mode::Branch, Path::new("/w/t"), &branch_prompt(&tk().uuid, "main")).unwrap();
        let log = fake.log();
        assert!(log.iter().any(|l| l == "tab_create alpha w1 /w/t Branch: fix: a thing"), "{log:?}");
        assert!(log.iter().any(|l| l.starts_with("agent_start alpha branch-7cd9fd3a ")), "{log:?}");
        assert!(
            log.iter().any(|l| l.starts_with("agent_prompt alpha branch-7cd9fd3a") && l.contains("/report-branch 7cd9fd3a-d27b-4387-8249-aaf0d6785f90 main")),
            "{log:?}"
        );
    }
```

(Check the fake's `agent_prompt` log line shape with `grep -n 'agent_prompt' src/session/fake.rs` and match it.)

`src/main.rs` tests:

```rust
    #[test]
    fn a_report_can_be_of_the_branch() {
        assert!(Cli::try_parse_from(["niritasks", "task", "report", "c53b6e3d", "--branch"]).is_ok());
        assert!(Cli::try_parse_from(["niritasks", "task", "report", "c53b6e3d", "--branch", "--fresh"]).is_ok());
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test branch 2>&1 | tail -15`
Expected: compile errors for the new names.

- [ ] **Step 3: Names**

`src/names.rs`:

```rust
/// The herdr agent name for a task's Explain branch session. Its own
/// `branch-`, so Explain branch never lands in a plan report's tab, and a
/// second press finds the first.
pub fn branch_agent(uuid: &str) -> String {
    format!("branch-{}", uuid8(uuid))
}
```

`is_task_agent`: `["task-", "work-", "report-", "branch-"]`, and its doc names Explain branch. `Role` gains `Branch`, and `PaneName::for_task`'s match `Role::Branch => "Branch"`.

- [ ] **Step 4: The default branch**

`src/work.rs`:

```rust
/// The branch a task's branch is measured against: the repository's
/// default branch, origin's HEAD when it has one (`origin/main` is `main`),
/// else the branch its main checkout, the workspace's folder, is on.
pub fn default_branch(repo: &Path) -> Result<String> {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let origin = git(&["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"]);
    let current = git(&["branch", "--show-current"]);
    base_branch(origin.as_deref(), current.as_deref()).with_context(|| {
        format!("Could not tell {}'s default branch: it has no origin/HEAD and its checkout is on no branch.", repo.display())
    })
}

/// [`default_branch`]'s choice, from what git answered.
fn base_branch(origin_head: Option<&str>, current: Option<&str>) -> Option<String> {
    origin_head
        .and_then(|h| h.strip_prefix("origin/"))
        .filter(|b| !b.is_empty())
        .or(current.filter(|b| !b.is_empty()))
        .map(String::from)
}
```

- [ ] **Step 5: `Mode::Branch` and `launch_branch`**

`src/refine.rs`:

1. `Mode` gains, after `Report`:

```rust
    /// No refine: the report of what the task's worktree branch has
    /// changed, run in that worktree, opened in the browser and linked from
    /// the task as `Branch report:`.
    Branch,
```

and `Mode`'s doc comment names Explain branch and `report-branch`.

2. Add:

```rust
/// A branch to explain: the task's worktree, and the branch it is
/// measured against ([`crate::work::default_branch`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub worktree: PathBuf,
    pub base: String,
}

/// Explain branch's prompt: [`prompt`]'s, then the base, which the skill
/// measures the branch against, as the mod does.
pub fn branch_prompt(uuid: &str, base: &str) -> String {
    format!("{} {base}", prompt(uuid, Mode::Branch))
}
```

3. In `prompt`: `Mode::Branch => format!("/report-branch {uuid}"),` (its doc: "Explain branch's base follows, from [`branch_prompt`]"). In `tab_label`: `Mode::Branch => "Branch",`. In `agent_name`: `Mode::Branch => names::branch_agent(uuid),`. In `already_open`: `Mode::Branch => "Already explaining this task's branch — switched to its tab.",`. In `open_in`'s role match: `Mode::Branch => names::Role::Branch,`.

4. `session_settings` takes `branch: Option<&Branch>` after `mode`, and its mod options become:

```rust
            (REFINE_MOD): { "options": {
                "uuid": uuid, "reports": reports, "home": fence.home,
                "report": matches!(mode, Mode::Report | Mode::Branch),
                "branch": mode == Mode::Branch,
                "worktree": branch.map(|b| b.worktree.display().to_string()).unwrap_or_default(),
                "base": branch.map(|b| b.base.clone()).unwrap_or_default(),
                "unattended": unattended,
            } },
```

Add to its doc comment: "`branch`, `worktree` and `base` in an Explain branch session, which reads that worktree's branch against that base; `project` is then the worktree, so the session cannot write it." Pass `None` for `branch` at every existing call (`grep -n "session_settings(" src`), the unattended runs included.

5. `open_in` and `launch_in` take the folder the tab opens in and the prompt:

```rust
fn open_in(session: &Session, t: &task::Task, workspace_label: &str, claude: &Claude, mode: Mode, dir: &Path, text: &str) -> Result<()> {
    …
    if launch_in(session, &name, &label, workspace_label, claude, &pane, dir, text)? == Launched::AlreadyRunning {
```

```rust
fn launch_in(session: &Session, name: &str, label: &str, workspace_label: &str, claude: &Claude, pane_name: &names::PaneName, dir: &Path, text: &str) -> Result<Launched> {
    …
    let tab = session.new_tab(&workspaces, dir, label, workspace_label)?;
```

Update the test helpers: `open(fake, mode)` passes `Path::new("/p/alpha"), &prompt(&tk().uuid, mode)`; `refine(fake)` passes `Path::new("/p/alpha")` before its prompt.

6. Split `launch`:

```rust
pub fn launch(ws: &Workspace, t: &task::Task, mode: Mode) -> Result<()> {
    start(ws, t, mode, None)
}

/// Explain branch: Claude in the task's worktree, in a tab of the
/// workspace's herdr session, on what its branch has changed since it left
/// `branch.base`. Fenced as a refine is, the worktree read-only to it.
pub fn launch_branch(ws: &Workspace, t: &task::Task, branch: &Branch) -> Result<()> {
    start(ws, t, Mode::Branch, Some(branch))
}

/// [`launch`]'s body, for a branch too: the project is the worktree, and
/// the prompt names the base.
fn start(ws: &Workspace, t: &task::Task, mode: Mode, branch: Option<&Branch>) -> Result<()> {
    … // as launch's body was, up to `let session = ws.session()?;` and `exe`
    let project = branch.map_or(session.dir(), |b| b.worktree.as_path());
    let settings = session_settings(
        project,
        &task_data,
        &fence,
        &t.uuid,
        &reports_dir(&dirs),
        mode,
        branch,
        false,
        Some(&crate::status_line::command(&exe, &t.uuid)),
    );
    let claude = Claude::Refiner { settings, mod_dir, tmp: fence.tmp.clone() };
    let text = match branch {
        Some(b) => branch_prompt(&t.uuid, &b.base),
        None => prompt(&t.uuid, mode),
    };
    open_in(&session, t, ws.name(), &claude, mode, project, &text)
}
```

- [ ] **Step 6: The command**

`src/main.rs`: give `TaskCommand::Report` the flag, and say so in its doc:

```rust
    /// Open the report of a planned task's plan, or have Claude build it in a new tab of the workspace's herdr session
    ///
    /// … (as now) …
    /// With --branch, the same for the task's branch report, `Branch
    /// report: <path>`: opened if linked, else Claude explains what the
    /// task's worktree branch has changed since it left the default branch,
    /// runs the tests, opens the report and links it; refused for a task
    /// with no worktree or a deleted one. Any status but deleted; no plan
    /// needed.
    Report {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// Build a new report even when the task's notes link one
        #[arg(long)]
        fresh: bool,
        /// The report of what the task's worktree branch has changed, not of its plan
        #[arg(long)]
        branch: bool,
    },
```

In `task_command`, match `TaskCommand::Report { uuid, fresh, branch }`, and right after the processing check:

```rust
            if branch {
                return explain_branch(&t, fresh);
            }
```

and add, beside `task_command`:

```rust
/// `task report <uuid> --branch`: open the task's branch report if linked
/// and there, else have Claude explain its worktree branch.
fn explain_branch(t: &task::Task, fresh: bool) -> Result<()> {
    let dirs = dirs::Dirs::from_env()?;
    let mut gone = None;
    if !fresh {
        if let Some(path) = t.branch_report_path() {
            let file = refine::report_file(path, &dirs);
            if file.is_file() {
                open::open_document(&file, Workspace::of_caller()?.name())?;
                notify::tasks(&format!("Opened the branch report: {}", t.description));
                return Ok(());
            }
            gone = Some(path.to_string());
        }
    }
    let workspace = Workspace::of_caller()?;
    anyhow::ensure!(t.status != "deleted", "A deleted task has no branch to explain.");
    let worktree = work::worktree_path(&workspace, &t.uuid)
        .context("This task has no worktree, so no branch to explain. Get started on it first.")?;
    let base = work::default_branch(&workspace.folder(&dirs))?;
    if gone.is_some() {
        notify::tasks(&format!("Branch report file gone, building a fresh one: {}", t.description));
    }
    refine::launch_branch(&workspace, t, &refine::Branch { worktree, base })
}
```

(add `work` to the `niri_tasks::{…}` import if it is not there.)

- [ ] **Step 7: The docs the command tests read**

`README.md`, Commands block, after the `task report <uuid> --fresh` line:

```text
niritasks task report <uuid> --branch  # open the branch report linked from its notes; without one, Claude explains what its worktree branch has changed, runs the tests, opens the report and links it
niritasks task report <uuid> --branch --fresh  #   explain the branch afresh even when its notes link a branch report
```

`llms.txt`, commands table, after the `task report <uuid> --fresh` row:

```text
| `niritasks task report <uuid> --branch` | On a task whose notes link a branch report as `Branch report: <path>`, open it as `task report` opens a report. Otherwise open a tab in the workspace's herdr session, in the task's worktree, with Claude (the `report-branch` skill) explaining what its branch has changed since the merge-base with the default branch: it runs every headless test suite with outputs under its `$TMPDIR`, names e2e suites without running them, writes the HTML deck and its Markdown twin to the reviews folder, opens it, and adds a `Branch report: <path>` note, leaving any `Report:` note as it is. Refused for a task with no worktree or a deleted one; any other status, planned or not | GUI (browser), or herdr tab, follows the herdr session, else focus |
| `niritasks task report <uuid> --branch --fresh` | Explain the branch afresh even when its notes link a branch report; the same refusals | herdr tab, follows the herdr session, else focus |
```

- [ ] **Step 8: Run the tests**

Run: `cargo test 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: every result `ok`, `readme_commands_block_runs_every_subcommand_and_flag` and `llms_txt_runs_every_subcommand_and_flag` included.

- [ ] **Step 9: Commit**

```bash
git add src/names.rs src/work.rs src/refine.rs src/main.rs README.md llms.txt
git commit -m "feat(report): add niritasks task report --branch

Explain branch opens Claude in the task's worktree, in a tab of its
own, prompted with /report-branch and the default branch it is
measured against, and tells the mod which worktree and base to read.
Without --fresh, a linked branch report opens instead.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: The Go to rows

**Files:**
- Modify: `src/panel/goto.rs`, `src/panel/state.rs`, `herdr/task-docs/task_docs.py`
- Test: `src/panel/goto.rs` tests, `herdr/task-docs/tests/test_task_docs.py`

**Interfaces:**
- Consumes: `TaskState.has_branch_report`, `Card.branch_report` (Task 7); `task report --branch` (Task 8).
- Produces: `Item::Explain` ("Explain branch"), `Item::BranchReport` ("Branch report"); `Since.branch_report`; `Since::of(report, branch_report, plan, spec, modified)`.

- [ ] **Step 1: Write the failing tests**

In `src/panel/goto.rs` tests, add `has_branch_report: true` to `everything()`, and:

```rust
    /// Explain branch shows on any task with a worktree, planned or not,
    /// finished or not; Branch report once its notes link one.
    #[test]
    fn a_worktree_can_have_its_branch_explained_and_opened() {
        let worktree = Found { worktree: true, ..Found::default() };
        assert_eq!(Item::of(on_list(), worktree), vec![Refine, Grill, Start, Explain, Worktree, ReadAloud]);
        assert!(!Item::of(on_list(), Found::default()).contains(&Explain), "no worktree, no branch");
        let finished = TaskState { finished: true, ..TaskState::default() };
        assert!(Item::of(finished, worktree).contains(&Explain));
        let explained = TaskState { has_branch_report: true, ..on_list() };
        assert!(Item::of(explained, Found::default()).contains(&BranchReport));
        assert_eq!(Explain.label(on_list(), worktree), "Explain branch");
        assert_eq!(BranchReport.label(explained, worktree), "Branch report");
        assert_eq!(Explain.args("u"), ["task", "report", "u", "--branch", "--fresh"]);
        assert_eq!(BranchReport.args("u"), ["task", "report", "u", "--branch"]);
        assert_eq!(Explain.group(), Group::Plan);
        assert_eq!(BranchReport.group(), Group::Document);
        let processing = TaskState { processing: true, ..on_list() };
        assert!(Item::of(processing, worktree).is_empty());
    }

    /// The branch report's row shows its age, from its note.
    #[test]
    fn the_branch_report_row_shows_its_age() {
        let link = Link { path: "/r/b.html".into(), entry: "20261006T010000Z".into() };
        let since = Since::of(None, Some(&link), None, None, &|_| None);
        assert_eq!(since.branch_report.as_deref(), Some("20261006T010000Z"));
        let state = TaskState { has_branch_report: true, ..on_list() };
        let list = GoToList::new("u", "t", state, Found::default(), since).unwrap();
        assert!(list.row_age(BranchReport, task::stamp_secs("20261008T010000Z").unwrap()).is_some());
    }
```

In `herdr/task-docs/tests/test_task_docs.py`, in `test_an_agent_named_for_a_task_gives_its_uuid8`, add:

```python
        self.assertEqual(task_docs.uuid8_of_agent("branch-1234abcd"), "1234abcd")
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test goto 2>&1 | tail -15; python3 -m unittest discover -s herdr/task-docs/tests 2>&1 | tail -5`
Expected: Rust fails to compile (no `Explain`, `BranchReport`, `Since::of` arity); Python fails on `branch-1234abcd`.

- [ ] **Step 3: The rows**

`src/panel/goto.rs`:

1. `GOTO_KEYS`' doc: "there are fourteen rows at most".
2. `Since` gains `pub branch_report: Option<String>`; `Since::of` takes `branch_report: Option<&Link>` after `report` and sets `branch_report: since(branch_report)`.
3. `Item` gains, after `Report`:

```rust
    /// Claude explains what its worktree branch has changed, a fresh report
    /// each time.
    Explain,
```

and after `OpenReport`:

```rust
    /// The branch report its notes link, `Branch report: <path>`.
    BranchReport,
```

4. `ALL: [Item; 14] = [Refine, Grill, Start, Report, Explain, OpenReport, BranchReport, Plan, Spec, RefineSession, WorkSession, Worktree, Editor, ReadAloud];`
5. `applies`: `Explain => found.worktree,` and `BranchReport => state.has_branch_report,`; its doc: "explaining the branch whenever there is a worktree, on or off the list".
6. `label`: `Explain => "Explain branch",` and `BranchReport => "Branch report",`.
7. `group`: `Refine | Grill | Report | Explain => Group::Plan,` and `OpenReport | BranchReport | Plan | Spec => Group::Document,`.
8. `icon` (Font Awesome code-fork and file; neither is used yet): `Explain => "\u{f126}",` and `BranchReport => "\u{f15b}",`; add both to its doc's list.
9. `args`: `Explain => &["task", "report", uuid, "--branch", "--fresh"],` and `BranchReport => &["task", "report", uuid, "--branch"],`.
10. `row_age`: `BranchReport => &self.since.branch_report,`; its doc names Branch report.

`src/panel/state.rs`, in `show_goto`:

```rust
        let since = Since::of(card.report.as_ref(), card.branch_report.as_ref(), card.plan.as_ref(), card.spec.as_ref(), modified);
```

Fix the other `Since::of` / `Since { … }` uses `cargo build --tests` names. An existing Go to test whose expected rows were found with `Found { worktree: true, .. }` now also lists `Explain`, just after `Report`'s place: add it there, never drop the row.

`herdr/task-docs/task_docs.py`: `AGENT = re.compile(r"^(?:work|task|report|branch)-([0-9a-f]{8})$")`, and write `work-/task-/report-/branch-<uuid8>` in the module docstring, `uuid8_of_agent`'s docstring and the "no task here" notification.

- [ ] **Step 4: Run the tests**

Run: `cargo test 2>&1 | grep -E 'test result|FAILED|panicked'; python3 -m unittest discover -s herdr/task-docs/tests 2>&1 | tail -3`
Expected: every result `ok`; `OK`.

- [ ] **Step 5: Commit**

```bash
git add src/panel/goto.rs src/panel/state.rs herdr/task-docs/task_docs.py herdr/task-docs/tests/test_task_docs.py
git commit -m "feat(panel): add Explain branch and Branch report to Go to

Any task with a worktree offers Explain branch, which builds a fresh
branch report; once its notes link one, Branch report opens it, with
its age. herdr's task-docs plugin finds the task of a branch- agent.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: The docs, and the end-to-end check

**Files:**
- Modify: `CONTEXT.md`, `README.md`, `llms.txt`

- [ ] **Step 1: CONTEXT.md**

After the **Refine report** entry, add:

```markdown
**Branch report**:
An HTML deck, with its Markdown twin, of what a task's worktree branch has
changed so far, how it works and how it is tested, built by an Explain
branch session (the `report-branch` skill) from the branch's commits and diff
against its merge-base with the default branch, and the test suite as that
session ran it. It shares the refine report's renderer and styles, in its own
slide order: hot spots, the commit story, a changes map, the test suite, the
new tests and plan vs done. Linked from the task's notes as
`Branch report: <path>`, never `Report:`, so the plan's report stays the
task's report; the Go to list's Branch report opens it, and
`niritasks task report <uuid> --branch --fresh` builds another.
_Avoid_: diff report, review report
```

In the **Go to list** entry, add "Explain branch" after "Generate or Regenerate report" and "Branch report" after "Open report" in its list of rows, and say Explain branch shows "whenever the task has a worktree".

- [ ] **Step 2: README**

In the card buttons table's **Go to…** row, add "Explain branch" and "Branch report" to the list of rows. In the Go to rows table (after **Open report**, line ~102), add:

```markdown
| **Explain branch** | the task has a worktree: Claude, in a tab of its own in the worktree, explains what its branch has changed since it left the default branch, runs the tests, and links the deck as `Branch report: <path>` |
| **Branch report** | its notes link a branch report: opened as Open report opens a report |
```

In the paragraph that lists what Refine, Grill me and Generate report need (line ~216), add Explain branch.

- [ ] **Step 3: llms.txt**

In the skills list, after `report-task`:

```markdown
- [report-branch](.claude/skills/report-branch/SKILL.md): explain what a task's worktree branch has changed, how it works and how it is tested, after running its tests, as an HTML deck linked from the task as `Branch report: <path>`; what `niritasks task report --branch` starts
```

- [ ] **Step 4: Run every suite**

Run:

```bash
cargo test 2>&1 | grep -E 'test result|FAILED|panicked'
(cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -4)
python3 -m unittest discover -s herdr/task-docs/tests 2>&1 | tail -3
cargo build && NIRITASKS="$PWD/target/debug/niritasks" bash tests/all.sh
```

Expected: every suite passes (a suite `tests/all.sh` cannot run here is SKIP, not a failure).

- [ ] **Step 5: Commit**

```bash
git add CONTEXT.md README.md llms.txt
git commit -m "docs: describe Explain branch and the branch report

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 6: After landing, check it end to end**

Skills and the mod load from the main checkout, so this runs once the branch has landed with `finish-worktree` and `install.sh` has run there:

1. On a started task with commits on its branch (any task worktree in `git worktree list`), open its card's Go to list: **Explain branch** is there; pick it.
2. A herdr tab `Branch: …` opens in the worktree; Claude runs the suites and calls the tool; the browser opens the deck.
3. Check against the Done when: every branch slide is filled from its commits and diff (the cover's numbers match `git log --oneline $(git merge-base HEAD main)..HEAD | wc -l` and `git diff --shortstat`); the Test suite slide shows the suites with their counts and the e2e ones as not run; New tests lists the tests the branch adds with their results.
4. `task rc.json.array=on <uuid> export`: the last note is `Branch report: <path>`, any `Report:` note is as it was.
5. The Go to list now shows **Branch report**, with its age; picking it opens the same deck.
