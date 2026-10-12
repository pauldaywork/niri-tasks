# Annotate a Task's Documents in herdr Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** From the herdr pane where a task's Claude runs, one key lists that task's documents and opens the pick in plannotator-tui beside the pane, so the comments made there become that Claude's next message.

**Architecture:** Three pieces. The refine mod's report tool also writes a Markdown twin of each report beside its HTML. A new `niritasks task docs <uuid>` prints the task's documents, one `label<TAB>path` per line, so Report-note parsing stays in Rust. A small herdr plugin in this repo, `herdr/task-docs/`, finds the task from the pane, lists those documents in an fzf overlay, and opens the pick in plannotator-tui as a split right of the task's Claude, with delivery set to that Claude's pane.

**Tech Stack:** Rust (clap, anyhow, serde; `cargo test`), the TypeScript Claude Code mod in `claude/refine-mod` (`claude plugin test`, `claude plugin validate`), Python 3 for the herdr plugin (`unittest`, as `~/Projects/herdr-speak` does), herdr 0.9.3, plannotator-tui 0.9.4 (shipped by herdr-annotate Full), fzf, bash (`install.sh`, `tests/all.sh`).

**Spec:** Taskwarrior task `35735579-9f97-42c2-af3d-f65e6719c8f3`, read with `task rc.json.array=on 35735579-9f97-42c2-af3d-f65e6719c8f3 export`. Its description and notes are the spec.

## Global Constraints

- herdr 0.9.3: a plugin is a `herdr-plugin.toml` of `[[actions]]` and `[[panes]]`; an action gets `HERDR_PANE_ID` and `HERDR_PLUGIN_CONTEXT_JSON` (`focused_pane_id`, `focused_pane_cwd`, `focused_pane_agent`); `herdr plugin pane open --direction` takes only `right` or `down`.
- The plugin lives in this repo at `herdr/task-docs/`. It hands the pick to plannotator-tui. It never reviews by itself and never changes herdr-annotate or plannotator-tui.
- Without plannotator-tui it notifies that herdr-annotate Full is needed (`herdr plugin install plannotator/herdr-annotate`).
- The Markdown twin is `<report>.md` beside `<report>.html`, written from the same fields. A report with no twin opens in the browser, as the card's Report does.
- The list comes from `niritasks task docs <uuid>`, one document per line, label then path. Report-note parsing stays in `src/task.rs`.
- The reviewer opens as a split to the right of the agent's pane, with focus.
- `install.sh` links the plugin with `herdr plugin link`. The key is a suggested `[[keys.command]]` in the README, as herdr-speak does.
- A task's Claude is the herdr agent `work-<uuid8>` or `task-<uuid8>`. A task's worktree branch is `task/<slug>-<uuid8>`.
- Out of scope: a pane on the left, annotating the HTML slides, changes to herdr-annotate or plannotator-tui, agents other than the task's Claude.
- Commits follow Conventional Commits (`feat(refine): …`, `feat(task): …`, `feat(herdr): …`, `docs: …`), subject at most 72 characters, body wrapped at 72, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## What research settled

These facts come from plannotator-tui's source (`crates/plannotator-tui/src/herdr/{context,launch}.rs`, `cli.rs`, commit `f35f70e`) and the installed herdr-annotate 0.9.0. The executor does not need to re-derive them.

- plannotator-tui's pane mode is `plannotator-tui herdr pane`. It opens the file in `PLANNOTATOR_TUI_FILE` and sends Send's comments to the pane in `PLANNOTATOR_TUI_DELIVER_TO` with `herdr agent prompt`. It never delivers to `HERDR_PANE_ID`, which is its own pane.
- `plannotator-tui herdr open <file>` would open its pane under `$HERDR_PLUGIN_ID`'s `doc` entrypoint, and its split direction comes from the user's plannotator-tui config. So the plugin opens its own `review` pane with `herdr plugin pane open` directly, which pins `--direction right` and `--target-pane`. This is the "raw herdr command" plannotator-tui's own skill documents.
- herdr-annotate Full stages the binary at `<herdr plugin dir>/bin/plannotator-tui.exe`, not on `PATH`. The plugin looks on `PATH` first, then for `~/.config/herdr/plugins/github/annotate-*/bin/plannotator-tui.exe`. This machine has herdr-annotate **Lite** installed, so neither exists yet; see Task 5.
- `herdr agent get <pane-id or name>` prints `{"result":{"agent":{"name":"work-35735579","pane_id":"w1Q:p1",…}}}`, and exits 1 with an `agent_not_found` error when there is none. `focused_pane_agent` in the context is the agent kind (`claude`), not its name, so the name comes from `agent get`.
- herdr shows no notification for the tab in front, which is where this key is pressed. The plugin notifies with `notify-send Tasks <body>`, as `src/notify.rs` does, and falls back to `herdr notification show` without it.

## Decisions made while planning

- **Which plans count as the task's.** `docs/superpowers/plans` holds every plan the repo ever had, and `~/.claude/plans` holds every project's. The list keeps only Markdown plans changed at or after the task's `entry`, newest first. A plan for a task cannot predate the task.
- **Earlier drafts.** `Report (earlier draft): <path>` notes are listed too, labelled as earlier drafts, after nothing else changes about them. The newest report comes first.
- **The notes document.** `task docs --notes-dir <dir>` writes `<dir>/task-<uuid8>.md` (the description as a heading, then one bullet per note) and lists it. The plugin passes its state folder.
- **Which Claude gets the comments.** The pane the key was pressed in, when its agent is named for the task; else the `work-<uuid8>` agent; else the `task-<uuid8>` refine. With none running, the plugin says so and opens nothing.
- **The key.** `prefix+shift+f` (files). herdr binds `shift+d g n p r t w x` and `shift+tab` by default (`t` is rename tab), and this machine's `~/.config/herdr/config.toml` binds `shift+g c o m e v a b`.

## File Structure

- `claude/refine-mod/hooks/markdown.ts` (create): the pure Markdown twin of a report, and its path.
- `claude/refine-mod/tests/markdown.test.ts` (create): its unit tests.
- `claude/refine-mod/hooks/refine.ts` (modify): write the twin after each HTML report.
- `claude/refine-mod/tests/refine.test.ts` (modify): the sessions write both files.
- `claude/refine-mod/.claude-plugin/plugin.json` (modify): its description names the twin.
- `src/task.rs` (modify): `Task::report_notes`, `EARLIER_DRAFT_NOTE`, `stamp_secs` made public.
- `src/docs.rs` (create): a task's documents, its notes as Markdown, and the plans since it was added.
- `src/lib.rs`, `src/main.rs` (modify): the module and `task docs`.
- `README.md`, `llms.txt`, `CONTEXT.md` (modify): the command, the plugin, the key, the twin.
- `herdr/task-docs/herdr-plugin.toml` (create): one action, a `pick` overlay pane, a `review` split pane.
- `herdr/task-docs/task_docs.py` (create): the action, the picker and the review pane.
- `herdr/task-docs/tests/test_task_docs.py` (create): its tests, with fake `herdr`, `notify-send`, `xdg-open` and `niritasks`.
- `tests/all.sh` (modify): run the plugin's tests.
- `install.sh` (modify): link the plugin.

---

### Task 1: The report's Markdown twin

**Files:**
- Create: `claude/refine-mod/hooks/markdown.ts`
- Create: `claude/refine-mod/tests/markdown.test.ts`
- Modify: `claude/refine-mod/hooks/refine.ts` (imports at the top; the two `await $.fs.write(path, reportPage(…))` lines in the `REPORT_TOOL` handler, about lines 373 and 409)
- Modify: `claude/refine-mod/tests/refine.test.ts:12` (constants), `:347`, `:447-448`, `:568`, `:727`
- Modify: `claude/refine-mod/.claude-plugin/plugin.json` (`description`)

**Interfaces:**
- Consumes: `Report`, `Section`, `Diagram`, `CodePair`, `Tool` from `hooks/report.ts`; `Plan` from `hooks/plan.ts`.
- Produces: `markdownPath(htmlPath: string): string` and `reportMarkdown(report: Report, plan: Plan): string` in `hooks/markdown.ts`. Every report the mod writes at `<dir>/refine-<uuid8>-<stamp>.html` now has `<dir>/refine-<uuid8>-<stamp>.md` beside it. Task 2 relies on that file name.

- [ ] **Step 1: Write the failing unit tests**

Create `claude/refine-mod/tests/markdown.test.ts`:

```ts
import { describe, expect, test } from 'claude-code/testing'
import { markdownPath, reportMarkdown } from '../hooks/markdown'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'
import { EXAMPLE } from './example-report'

// The smallest report the tool takes, as parseReport answers it.
const MINIMAL = parseReport({
  title: 'feat: New words',
  summary: 'The task gets new words.',
  changes: ['Words: the description changes.'],
  sections: [{ heading: 'What changes', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  checks: [{ check: 'The words show', how: 'By eye' }],
}) as Report

const PLAN = { description: 'feat: New words', notes: ['step one', 'step two'] }

describe('markdownPath', () => {
  test('is the report with .md for .html', () => {
    expect(markdownPath('/r/refine-0b8f6a52-20261006-010203.html')).toBe('/r/refine-0b8f6a52-20261006-010203.md')
  })
})

describe('reportMarkdown', () => {
  test('the smallest report reads top to bottom as its slides do', () => {
    expect(reportMarkdown(MINIMAL, PLAN)).toBe(
      [
        '# feat: New words',
        '',
        'The task gets new words.',
        '',
        '## What changes',
        '',
        '- Words: the description changes.',
        '',
        '## What changes',
        '',
        '- The words.',
        '',
        '## Files',
        '',
        '| Change | File | Why |',
        '| --- | --- | --- |',
        '| change | `src/words.rs` | Holds the words. |',
        '',
        '## Done when',
        '',
        '| Check | How it is checked |',
        '| --- | --- |',
        '| The words show | By eye |',
        '',
        '## The plan',
        '',
        '**Description:** feat: New words',
        '',
        '1. step one',
        '2. step two',
        '',
      ].join('\n'),
    )
  })

  test('the optional lists show only when they hold something', () => {
    const md = reportMarkdown(
      { ...MINIMAL, unchanged: ['The panel.'], needs_your_eye: ['The key.'], terms: [{ term: 'twin', meaning: 'The .md copy.' }] },
      PLAN,
    )
    expect(md).toContain('## What stays the same\n\n- The panel.')
    expect(md).toContain('## Where your judgement is needed\n\n- The key.')
    expect(md).toContain('## Words used here\n\n- **twin**: The .md copy.')
    expect(reportMarkdown(MINIMAL, PLAN)).not.toContain('## What stays the same')
  })

  test('a Mermaid diagram is a mermaid block; a drawn SVG points at the HTML', () => {
    const md = reportMarkdown(
      {
        ...MINIMAL,
        sections: [
          { heading: 'Flow', points: ['p'], look_at: 'The arrow.', diagram: { mermaid: 'flowchart LR\n  a --> b' } },
          { heading: 'Shape', points: ['p'], look_at: 'The box.', diagram: { svg: '<svg></svg>' } },
          { heading: 'Pair', points: ['p'], look_at: 'Both.', before: { mermaid: 'flowchart LR\n  a' }, after: { mermaid: 'flowchart LR\n  b' } },
        ],
      },
      PLAN,
    )
    expect(md).toContain('## Flow\n\n_The arrow._\n\n```mermaid\nflowchart LR\n  a --> b\n```\n\n- p')
    expect(md).toContain('## Shape\n\n_The box._\n\n_A drawn diagram: see the HTML report._')
    expect(md).toContain('Before:\n\n```mermaid\nflowchart LR\n  a\n```\n\nAfter:\n\n```mermaid\nflowchart LR\n  b\n```')
  })

  test('code holding a fence gets a longer one, and a new file says so', () => {
    const md = reportMarkdown(
      { ...MINIMAL, sections: [{ heading: 'Code', points: ['p'], code: [{ file: 'a.md', before: '', after: 'x ``` y' }] }] },
      PLAN,
    )
    expect(md).toContain('### a.md\n\n_New file._\n\nAfter:\n\n````\nx ``` y\n````')
  })

  test('a pipe in a table cell is escaped', () => {
    const md = reportMarkdown({ ...MINIMAL, checks: [{ check: 'a | b', how: 'By eye' }] }, PLAN)
    expect(md).toContain('| a \\| b | By eye |')
  })

  test('the outside tools are a table, and detail follows the points', () => {
    const md = reportMarkdown(
      {
        ...MINIMAL,
        sections: [
          {
            heading: 'Tools',
            points: ['p'],
            tools: [{ tool: 'herdr', how: 'Its CLI.', swap: 'hard', why: 'Panes.' }],
            detail: '<p>More.</p>',
          },
        ],
      },
      PLAN,
    )
    expect(md).toContain('| Tool | How it connects | Swap | Why |\n| --- | --- | --- | --- |\n| herdr | Its CLI. | hard | Panes. |')
    expect(md).toContain('More detail:\n\n<p>More.</p>')
  })

  test('the worked example has every part as a heading', () => {
    const report = parseReport(EXAMPLE) as Report
    const md = reportMarkdown(report, PLAN)
    for (const s of report.sections) expect(md).toContain(`## ${s.heading}`)
  })

  test('a plan with no notes says so', () => {
    expect(reportMarkdown(MINIMAL, { description: 'd', notes: [] })).toContain('**Description:** d\n\nNo notes.\n')
  })
})
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: FAIL in `markdown.test.ts`, which cannot import `../hooks/markdown`.

- [ ] **Step 3: Write `hooks/markdown.ts`**

```ts
// The Markdown twin of a report: the same fields as the HTML slides, top to
// bottom, as a document plannotator-tui can open in herdr, where the comments
// made on it go back to the task's Claude. Pure, like report.ts, so the unit
// tests reach it directly.

import type { Plan } from './plan'
import type { CodePair, Diagram, Report, Section, Tool } from './report'

// The twin's path: the report's, with .md for .html.
export const markdownPath = (htmlPath: string): string => `${htmlPath.replace(/\.html$/, '')}.md`

// A fence longer than any run of backticks in the text, so code that holds
// ``` stays one block.
const fence = (text: string, lang = ''): string => {
  const longest = Math.max(2, ...(text.match(/`+/g) ?? []).map(run => run.length))
  const bar = '`'.repeat(longest + 1)
  return `${bar}${lang}\n${text}\n${bar}`
}

const cell = (text: string): string => text.replace(/\|/g, '\\|')

const table = (head: readonly string[], rows: readonly (readonly string[])[]): string =>
  [
    `| ${head.join(' | ')} |`,
    `|${head.map(() => ' --- ').join('|')}|`,
    ...rows.map(row => `| ${row.map(cell).join(' | ')} |`),
  ].join('\n')

const bullets = (lines: readonly string[]): string => lines.map(line => `- ${line}`).join('\n')

const drawing = (diagram: Diagram): string =>
  'mermaid' in diagram ? fence(diagram.mermaid, 'mermaid') : '_A drawn diagram: see the HTML report._'

const code = (pairs: readonly CodePair[]): string =>
  pairs
    .map(pair =>
      [
        `### ${pair.file}`,
        pair.before === '' ? '_New file._' : `Before:\n\n${fence(pair.before)}`,
        pair.after === '' ? '_Removed._' : `After:\n\n${fence(pair.after)}`,
      ].join('\n\n'),
    )
    .join('\n\n')

const tools = (list: readonly Tool[]): string =>
  table(['Tool', 'How it connects', 'Swap', 'Why'], list.map(t => [t.tool, t.how, t.swap, t.why]))

const part = (s: Section): string => {
  const out = [`## ${s.heading}`]
  if (s.look_at !== undefined) out.push(`_${s.look_at}_`)
  if (s.diagram !== undefined) out.push(drawing(s.diagram))
  if (s.before !== undefined && s.after !== undefined) {
    out.push(`Before:\n\n${drawing(s.before)}`, `After:\n\n${drawing(s.after)}`)
  }
  out.push(bullets(s.points))
  if (s.code !== undefined) out.push(code(s.code))
  if (s.tools !== undefined) out.push(tools(s.tools))
  if (s.detail !== undefined) out.push(`More detail:\n\n${s.detail}`)
  return out.join('\n\n')
}

// The report as Markdown, in the slides' order, ending with the plan the
// person was shown, as the HTML's last slide does.
export const reportMarkdown = (report: Report, plan: Plan): string => {
  const out = [`# ${report.title}`, report.summary, `## What changes\n\n${bullets(report.changes)}`]
  if (report.unchanged.length > 0) out.push(`## What stays the same\n\n${bullets(report.unchanged)}`)
  if (report.needs_your_eye.length > 0) {
    out.push(`## Where your judgement is needed\n\n${bullets(report.needs_your_eye)}`)
  }
  if (report.terms.length > 0) {
    out.push(`## Words used here\n\n${bullets(report.terms.map(t => `**${t.term}**: ${t.meaning}`))}`)
  }
  out.push(...report.sections.map(part))
  if (report.files.length > 0) {
    out.push(`## Files\n\n${table(['Change', 'File', 'Why'], report.files.map(f => [f.change, `\`${f.path}\``, f.why]))}`)
  }
  out.push(`## Done when\n\n${table(['Check', 'How it is checked'], report.checks.map(c => [c.check, c.how]))}`)
  const notes = plan.notes.length === 0 ? 'No notes.' : plan.notes.map((n, i) => `${i + 1}. ${n}`).join('\n')
  out.push(`## The plan\n\n**Description:** ${plan.description}\n\n${notes}`)
  return `${out.join('\n\n')}\n`
}
```

- [ ] **Step 4: Run the unit tests to see them pass**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -15`
Expected: every `markdown.test.ts` test passes. The rest still pass, since nothing calls the module yet.

- [ ] **Step 5: Make the session tests expect the twin**

In `claude/refine-mod/tests/refine.test.ts`, under `const REPORT_PATH = …` (line 12), add:

```ts
// The report's Markdown twin, written beside it.
const REPORT_MD = '/r/refine-0b8f6a52-20261006-010203.md'
```

Line 347, in "report, then approve", becomes the two lines:

```ts
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
  expect(writes[1]?.text).toStartWith('# feat: New words\n\nThe task gets new words.\n')
```

Lines 447-448, in "a report embeds the fonts the mod ships", become:

```ts
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD, REPORT_PATH, REPORT_MD])
  for (const page of writes.filter(w => w.path.endsWith('.html')).map(w => w.text)) {
```

Lines 568 and 727 (the report-mode and unattended report-mode sessions) each become:

```ts
  expect(writes.map(w => w.path)).toEqual([REPORT_PATH, REPORT_MD])
```

After line 727, add a check that the twin holds the task's own plan, which report mode reports on:

```ts
  expect(writes[1]?.text).toContain(`**Description:** ${PLANNED.description}`)
```

- [ ] **Step 6: Run them to see them fail**

Run: `cd claude/refine-mod && claude plugin test . 2>&1 | tail -25`
Expected: FAIL in the four tests changed above, each seeing only `[REPORT_PATH]` (or two `.html` writes). Any other failure means an assertion was mistyped.

- [ ] **Step 7: Write the twin in `hooks/refine.ts`**

Add the import beside the other `./report` imports:

```ts
import { markdownPath, reportMarkdown } from './markdown'
```

In the `REPORT_TOOL` handler there are two writes of the page. After each, write the twin from the same report and plan. In report mode:

```ts
      await $.fs.write(path, reportPage(report, shown, await readFonts($, fonts), await drawDiagrams($, report)))
      // Its Markdown twin, which herdr's task-docs plugin opens in
      // plannotator-tui so comments on it reach the task's Claude.
      await $.fs.write(markdownPath(path), reportMarkdown(report, shown))
```

In refine mode, after `await $.fs.write(path, reportPage(report, shown, …))`:

```ts
    await $.fs.write(markdownPath(path), reportMarkdown(report, shown))
```

- [ ] **Step 8: Name the twin in the mod's description**

In `claude/refine-mod/.claude-plugin/plugin.json`, in `description`, change "which writes and opens an HTML report of the plan when the person asks for one" to "which writes and opens an HTML report of the plan, with a Markdown twin beside it, when the person asks for one".

- [ ] **Step 9: Run every mod test and the validator**

Run: `cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -6`
Expected: validation passes and every test passes.

- [ ] **Step 10: Commit**

```bash
git add claude/refine-mod
git commit -m "feat(refine): write a Markdown twin beside each report

show_task_report now writes <report>.md beside <report>.html, from the
same fields, so a report can be opened and annotated in plannotator-tui.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `niritasks task docs`

**Files:**
- Modify: `src/task.rs` (beside `REPORT_NOTE`, line 30; `impl Task`, after `report_path`, line 143; `fn stamp_secs`, line 218; the `#[cfg(test)]` module)
- Create: `src/docs.rs`
- Modify: `src/lib.rs` (module list)
- Modify: `src/main.rs` (imports line 12-15; `TaskCommand` after `Report`; the dispatch after `TaskCommand::Report`; the tests module)
- Modify: `README.md` (the `## Commands` block, after the `task report --fresh` line), `llms.txt` (the commands table, after the `task report --fresh` row)

**Interfaces:**
- Consumes: the `.md` twin beside each report, from Task 1, named by replacing `.html` with `.md`.
- Produces: `niritasks task docs <uuid> [--project <dir>] [--notes-dir <dir>]`, printing one `label\tpath` line per document. Reports come first, newest first. The task notes file comes next, when `--notes-dir` is given. Plans changed since the task was added come last, newest first. Task 3's script runs exactly `niritasks task docs <uuid8> --project <pane cwd> --notes-dir <plugin state dir>` and splits each line at the first tab. Labels never hold a tab.

- [ ] **Step 1: Write the failing `report_notes` test in `src/task.rs`**

In the `#[cfg(test)] mod tests` of `src/task.rs`, after `a_blank_report_note_is_skipped`, add:

```rust
    /// Every report the task links, newest first, earlier drafts marked, for
    /// `task docs`. A mention, a missing space or a blank path is no report.
    #[test]
    fn report_notes_lists_every_report_newest_first() {
        let t: Task = serde_json::from_str(
            r#"{"uuid":"u","description":"d","annotations":[
                {"entry":"20261006T010000Z","description":"Report (earlier draft): /r/old.html"},
                {"entry":"20261006T020000Z","description":"See the Report: /r/not-this.html"},
                {"entry":"20261007T030000Z","description":"Report: /r/new.html "},
                {"entry":"20261007T040000Z","description":"Report:  "}
            ]}"#,
        )
        .unwrap();
        assert_eq!(
            t.report_notes(),
            vec![
                ReportNote { path: "/r/new.html".into(), draft: false, date: "2026-10-07".into() },
                ReportNote { path: "/r/old.html".into(), draft: true, date: "2026-10-06".into() },
            ]
        );
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --lib task::tests::report_notes 2>&1 | tail -5`
Expected: FAIL to compile: no `ReportNote`, no `report_notes`.

- [ ] **Step 3: Add `report_notes` to `src/task.rs`**

Under `pub const REPORT_NOTE: &str = "Report: ";` add:

```rust
/// What a note linking a report of a plan since changed starts with, as the
/// refine mod writes it. Never the task's report, but still one of its
/// documents.
pub const EARLIER_DRAFT_NOTE: &str = "Report (earlier draft): ";

/// One report a task's notes link, as `task docs` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportNote {
    /// The HTML report's path, trimmed.
    pub path: String,
    /// Linked as `Report (earlier draft):`.
    pub draft: bool,
    /// The note's date, as [`Annotation::date`] writes it.
    pub date: String,
}
```

In `impl Task`, after `report_path`:

```rust
    /// Every report the task's notes link, the newest first: `Report:` and
    /// `Report (earlier draft):` notes alike, the second marked. A note with
    /// nothing after its prefix is skipped, as [`Task::report_path`] skips it.
    pub fn report_notes(&self) -> Vec<ReportNote> {
        self.annotations
            .iter()
            .rev()
            .filter_map(|a| {
                let (rest, draft) = match a.description.strip_prefix(REPORT_NOTE) {
                    Some(rest) => (rest, false),
                    None => (a.description.strip_prefix(EARLIER_DRAFT_NOTE)?, true),
                };
                let path = rest.trim();
                (!path.is_empty()).then(|| ReportNote { path: path.to_string(), draft, date: a.date() })
            })
            .collect()
    }
```

Make the stamp parser public, since `docs.rs` compares plan times with the task's `entry`. Change `fn stamp_secs(stamp: &str) -> Option<i64> {` to `pub fn stamp_secs(stamp: &str) -> Option<i64> {`.

- [ ] **Step 4: Run it to see it pass**

Run: `cargo test --lib task:: 2>&1 | tail -3`
Expected: every `task::` test passes.

- [ ] **Step 5: Write the failing `docs` tests**

Create `src/docs.rs` holding only the tests for now, so the failure is about the missing functions:

```rust
//! A task's documents, as `niritasks task docs` lists them for herdr's
//! task-docs plugin.

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// A scratch folder of this test's own.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("niritasks-docs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A file holding `text`, last changed at `secs` (Unix seconds).
    fn file_at(path: &Path, text: &str, secs: u64) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
    }

    fn task(json: &str) -> Task {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_reports_twin_is_beside_it() {
        assert_eq!(twin(Path::new("/r/refine-a-1.html")), PathBuf::from("/r/refine-a-1.md"));
    }

    /// The twin when it is there, the HTML when only that is (a report made
    /// before twins), and nothing for a report whose files are both gone.
    #[test]
    fn reports_prefer_the_twin_and_skip_what_is_gone() {
        let dir = scratch("reports");
        file_at(&dir.join("new.html"), "<html>", 1);
        file_at(&dir.join("new.md"), "# new", 1);
        file_at(&dir.join("old.html"), "<html>", 1);
        let t = task(&format!(
            r#"{{"uuid":"u","description":"d","annotations":[
                {{"entry":"20261005T010000Z","description":"Report: {d}/gone.html"}},
                {{"entry":"20261006T010000Z","description":"Report (earlier draft): {d}/old.html"}},
                {{"entry":"20261007T010000Z","description":"Report: {d}/new.html"}}
            ]}}"#,
            d = dir.display()
        ));
        assert_eq!(
            reports(&t),
            vec![
                Doc { label: "Report · 2026-10-07".into(), path: dir.join("new.md") },
                Doc {
                    label: "Report, earlier draft · 2026-10-06 · opens in the browser".into(),
                    path: dir.join("old.html"),
                },
            ]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_notes_are_the_description_then_one_bullet_each() {
        let t = task(
            r#"{"uuid":"35735579-9f97","description":"feat: Annotate docs","annotations":[
                {"entry":"20261011T064839Z","description":"Goal: one key"},
                {"entry":"20261011T064840Z","description":"Context:\n  herdr 0.9.3"}
            ]}"#,
        );
        assert_eq!(
            notes_markdown(&t),
            "# feat: Annotate docs\n\nTask `35735579-9f97`: its description and notes.\n\n## Notes\n\n\
             - 2026-10-11: Goal: one key\n- 2026-10-11: Context: herdr 0.9.3\n"
        );
        let bare = task(r#"{"uuid":"u","description":"d"}"#);
        assert!(notes_markdown(&bare).ends_with("## Notes\n\nNo notes.\n"));
    }

    #[test]
    fn the_notes_are_written_as_task_uuid8_md() {
        let dir = scratch("notes").join("state");
        let t = task(r#"{"uuid":"35735579-9F97","description":"d"}"#);
        let path = write_notes(&dir, &t).unwrap();
        assert_eq!(path, dir.join("task-35735579.md"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), notes_markdown(&t));
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    /// Only Markdown changed since the task was added, from the project's
    /// plans and Claude's, newest first.
    #[test]
    fn plans_since_the_task_newest_first() {
        let home = scratch("plans");
        let project = home.join("Projects/alpha");
        let ours = project.join("docs/superpowers/plans");
        let claudes = home.join(".claude/plans");
        file_at(&ours.join("before.md"), "x", 99);
        file_at(&ours.join("first.md"), "x", 100);
        file_at(&ours.join("notes.txt"), "x", 300);
        file_at(&claudes.join("second.md"), "x", 200);
        let found = plans(&Dirs::at(&home), &project, 100);
        assert_eq!(
            found,
            vec![
                Doc { label: "Claude plan · second.md".into(), path: claudes.join("second.md") },
                Doc { label: "Plan · first.md".into(), path: ours.join("first.md") },
            ]
        );
        // Neither folder there is no plans, not an error.
        assert!(plans(&Dirs::at(&home.join("none")), &home.join("none"), 0).is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_list_is_reports_then_notes_then_plans() {
        let home = scratch("list");
        let project = home.join("p");
        file_at(&home.join("r.html"), "<html>", 1);
        file_at(&home.join("r.md"), "# r", 1);
        let added = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 60;
        file_at(&project.join("docs/superpowers/plans/plan.md"), "x", added + 30);
        let t = task(&format!(
            r#"{{"uuid":"u","description":"d","entry":"{}","annotations":[
                {{"entry":"20261007T010000Z","description":"Report: {}/r.html"}}]}}"#,
            crate::task::stamp(added as i64),
            home.display()
        ));
        let notes = home.join("task-u.md");
        let lines: Vec<String> =
            list(&t, &Dirs::at(&home), &project, Some(&notes)).iter().map(Doc::line).collect();
        assert_eq!(
            lines,
            vec![
                format!("Report · 2026-10-07\t{}/r.md", home.display()),
                format!("Task · description and notes\t{}", notes.display()),
                format!("Plan · plan.md\t{}/docs/superpowers/plans/plan.md", project.display()),
            ]
        );
        std::fs::remove_dir_all(&home).ok();
    }
}
```

Add `pub mod docs;` to `src/lib.rs`, in alphabetical order after `pub mod dirs;`.

- [ ] **Step 6: Run them to see them fail**

Run: `cargo test --lib docs:: 2>&1 | tail -8`
Expected: FAIL to compile: `twin`, `reports`, `Doc`, `notes_markdown`, `write_notes`, `plans`, `list`, `Task`, `Dirs`, `Path` and `PathBuf` not found.

- [ ] **Step 7: Write `src/docs.rs`**

Replace the two header lines above `#[cfg(test)]` with the module:

```rust
//! A task's documents, as `niritasks task docs` lists them for herdr's
//! task-docs plugin: its refine reports, its own description and notes, and
//! the plans written since it was added. Each is a Markdown file
//! plannotator-tui can open beside the task's Claude, bar a report made
//! before reports had a Markdown twin, which opens in the browser.

use crate::{dirs::Dirs, names, task, task::Task, text};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// One document: what the list shows, and the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    /// Never holds a tab: [`Doc::line`] splits on the first one.
    pub label: String,
    pub path: PathBuf,
}

impl Doc {
    /// One line of `task docs`: the label, a tab, the path.
    pub fn line(&self) -> String {
        format!("{}\t{}", self.label, self.path.display())
    }
}

/// The Markdown twin the refine mod writes beside a report.
pub fn twin(report: &Path) -> PathBuf {
    report.with_extension("md")
}

/// The task's reports, newest first: each one's Markdown twin, or its HTML
/// when it has none. A report whose files are both gone is left out.
pub fn reports(t: &Task) -> Vec<Doc> {
    t.report_notes()
        .into_iter()
        .filter_map(|n| {
            let kind = if n.draft { "Report, earlier draft" } else { "Report" };
            let html = PathBuf::from(&n.path);
            let md = twin(&html);
            if md.is_file() {
                Some(Doc { label: format!("{kind} · {}", n.date), path: md })
            } else if html.is_file() {
                Some(Doc { label: format!("{kind} · {} · opens in the browser", n.date), path: html })
            } else {
                None
            }
        })
        .collect()
}

/// The task's description and notes as Markdown: the description as the
/// heading, then each note, dated, as a bullet.
pub fn notes_markdown(t: &Task) -> String {
    let mut out = format!(
        "# {}\n\nTask `{}`: its description and notes.\n\n## Notes\n\n",
        text::collapse_whitespace(&t.description),
        t.uuid
    );
    if t.annotations.is_empty() {
        out.push_str("No notes.\n");
    }
    for a in &t.annotations {
        out.push_str(&format!("- {}: {}\n", a.date(), text::collapse_whitespace(&a.description)));
    }
    out
}

/// Write [`notes_markdown`] to `<dir>/task-<uuid8>.md`, making `dir`, and
/// answer the file's path.
pub fn write_notes(dir: &Path, t: &Task) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("could not make {}", dir.display()))?;
    let path = dir.join(format!("task-{}.md", names::uuid8(&t.uuid)));
    std::fs::write(&path, notes_markdown(t)).with_context(|| format!("could not write {}", path.display()))?;
    Ok(path)
}

/// The Markdown files directly in `folder` last changed at or after `since`
/// (Unix seconds), each with that time. A folder that is not there has none.
fn plans_in(folder: &Path, kind: &str, since: i64) -> Vec<(i64, Doc)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let Some(changed) = path
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
        else {
            continue;
        };
        if changed < since {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        found.push((changed, Doc { label: format!("{kind} · {name}"), path }));
    }
    found
}

/// The plans written since `since`: the project's `docs/superpowers/plans`
/// and `~/.claude/plans`, newest first. A plan for a task cannot predate it,
/// and both folders hold plans for much else.
pub fn plans(dirs: &Dirs, project: &Path, since: i64) -> Vec<Doc> {
    let mut found = plans_in(&project.join("docs/superpowers/plans"), "Plan", since);
    found.extend(plans_in(&dirs.home().join(".claude/plans"), "Claude plan", since));
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
    found.into_iter().map(|(_, doc)| doc).collect()
}

/// Everything `task docs` lists, in order: the reports, then the notes file
/// when there is one, then the plans since the task was added. A task whose
/// `entry` does not parse counts every plan.
pub fn list(t: &Task, dirs: &Dirs, project: &Path, notes: Option<&Path>) -> Vec<Doc> {
    let mut docs = reports(t);
    if let Some(path) = notes {
        docs.push(Doc { label: "Task · description and notes".into(), path: path.to_path_buf() });
    }
    docs.extend(plans(dirs, project, task::stamp_secs(&t.entry).unwrap_or(0)));
    docs
}
```

- [ ] **Step 8: Run them to see them pass**

Run: `cargo test --lib docs:: 2>&1 | tail -3`
Expected: all six `docs::` tests pass.

- [ ] **Step 9: Write the failing CLI test in `src/main.rs`**

In the tests module of `src/main.rs`, after `a_fresh_report_is_a_flag_on_task_report`, add:

```rust
    /// herdr's task-docs plugin runs this to list a task's documents.
    #[test]
    fn listing_a_tasks_documents_is_a_command() {
        assert!(Cli::try_parse_from(["niritasks", "task", "docs", "c53b6e3d"]).is_ok());
        assert!(Cli::try_parse_from([
            "niritasks", "task", "docs", "c53b6e3d", "--project", "/p", "--notes-dir", "/s"
        ])
        .is_ok());
    }
```

Run: `cargo test --bin niritasks listing_a_tasks 2>&1 | tail -5`
Expected: FAIL: `unrecognized subcommand 'docs'`.

- [ ] **Step 10: Add `task docs` to the CLI**

In `src/main.rs`, change `use std::path::Path;` to `use std::path::{Path, PathBuf};`, and add `docs` to the `niri_tasks::{…}` import list after `dirs`.

In `enum TaskCommand`, after the `Report { … }` variant, add:

```rust
    /// List a task's documents, one per line: a label, a tab, the path
    ///
    /// What herdr's task-docs plugin lists to open in plannotator-tui beside
    /// the task's Claude. First the reports its notes link, newest first:
    /// each one's Markdown twin, or its HTML when it was made before twins.
    /// Then, with --notes-dir, its description and notes, written there as
    /// task-<uuid8>.md. Then the Markdown plans in the project's
    /// docs/superpowers/plans and in ~/.claude/plans changed since the task
    /// was added, newest first.
    Docs {
        /// The task's uuid, or its first 8 characters
        uuid: String,
        /// The project folder whose docs/superpowers/plans to look in; this folder when left out
        #[arg(long)]
        project: Option<PathBuf>,
        /// Write the task's description and notes into this folder as Markdown, and list that file
        #[arg(long)]
        notes_dir: Option<PathBuf>,
    },
```

In the dispatch, after the `TaskCommand::Report { … } => { … }` arm, add:

```rust
        TaskCommand::Docs { uuid, project, notes_dir } => {
            let t = task::get(&uuid)?.with_context(|| format!("No task {uuid}."))?;
            let dirs = dirs::Dirs::from_env()?;
            let project = match project {
                Some(p) => p,
                None => std::env::current_dir().context("could not read this folder")?,
            };
            let notes = notes_dir.map(|d| docs::write_notes(&d, &t)).transpose()?;
            for doc in docs::list(&t, &dirs, &project, notes.as_deref()) {
                println!("{}", doc.line());
            }
        }
```

- [ ] **Step 11: Document the command, then run every test**

In `README.md`'s `## Commands` block, after the `niritasks task report <uuid> --fresh` line, add:

```
niritasks task docs <uuid> --project <dir> --notes-dir <dir>
                                   # list its documents, label<TAB>path: its reports (the Markdown twin, else
                                   #   the HTML), its notes written to --notes-dir as Markdown, and the plans
                                   #   in <dir>/docs/superpowers/plans and ~/.claude/plans since it was added
```

In `llms.txt`'s commands table, after the `task report <uuid> --fresh` row, add:

```
| `niritasks task docs <uuid> --project <dir> --notes-dir <dir>` | Print the task's documents, one `label<TAB>path` per line: the reports its notes link, newest first (each one's Markdown twin, or its HTML when made before twins); with `--notes-dir`, its description and notes written there as `task-<uuid8>.md`; then the Markdown plans in `<dir>/docs/superpowers/plans` (this folder without `--project`) and `~/.claude/plans` changed since the task was added, newest first. What herdr's task-docs plugin lists | scriptable |
```

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head`
Expected: every `test result: ok`, the README and llms.txt coverage tests (`readme_commands_block_runs_every_subcommand_and_flag`, `llms_txt_runs_every_subcommand_and_flag`) included.

- [ ] **Step 12: Try it on this task**

Run: `cargo run -q -- task docs 35735579 --notes-dir /tmp/claude-docs-check | cut -c1-140`
Expected: a `Task · description and notes` line pointing at `/tmp/claude-docs-check/task-35735579.md`, then `Plan · 2026-10-11-annotate-task-documents.md` and any newer plans. Delete `/tmp/claude-docs-check` afterwards.

- [ ] **Step 13: Commit**

```bash
git add src/task.rs src/docs.rs src/lib.rs src/main.rs README.md llms.txt
git commit -m "feat(task): add task docs, which lists a task's documents

One label<TAB>path line per document: the reports its notes link (the
Markdown twin, else the HTML), its notes written out as Markdown, and the
plans changed since it was added. herdr's task-docs plugin lists these.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The herdr plugin

**Files:**
- Create: `herdr/task-docs/herdr-plugin.toml`
- Create: `herdr/task-docs/task_docs.py`
- Create: `herdr/task-docs/tests/__init__.py` (empty)
- Create: `herdr/task-docs/tests/test_task_docs.py`
- Modify: `tests/all.sh` (a `why_plugin` check and a `suite` line)

**Interfaces:**
- Consumes: `niritasks task docs <uuid8> --project <dir> --notes-dir <dir>` from Task 2, printing `label\tpath` lines.
- Produces: the herdr action `task-docs.open`, which Task 4's README binds to a key and `install.sh` links.

- [ ] **Step 1: Write the manifest**

Create `herdr/task-docs/herdr-plugin.toml`:

```toml
# Annotate one of a task's documents from the herdr pane where its Claude
# runs: its refine reports, its own description and notes, and the plans
# written since it was added. The pick opens in plannotator-tui (from
# herdr-annotate Full) beside that Claude, and Send makes the comments its
# next message. Linked by niri-tasks' install.sh; the key is the README's.
id = "task-docs"
name = "Task docs"
version = "0.1.0"
min_herdr_version = "0.9.1"
description = "Annotate a niri-tasks task's reports, notes and plans in plannotator-tui, beside the task's Claude"
platforms = ["linux"]

# Press in a task's pane: finds the task from the pane's agent name
# (work-/task-<uuid8>) or its branch (task/<slug>-<uuid8>), and opens the list.
[[actions]]
id = "open"
title = "Task docs: annotate one of this task's documents"
contexts = ["pane"]
command = ["python3", "task_docs.py", "open"]

# Opened by the action over the pane, in the pane's folder: fzf over
# `niritasks task docs`. Enter opens the pick beside the task's Claude; Esc
# closes it. Reached through HERDR_PLUGIN_ROOT, since its cwd is the pane's.
[[panes]]
id = "pick"
title = "Task docs · enter annotate · esc cancel"
placement = "overlay"
command = ["sh", "-c", "exec python3 \"$HERDR_PLUGIN_ROOT/task_docs.py\" pick"]

# Opened by the picker to the right of the task's Claude, focused:
# plannotator-tui on PLANNOTATOR_TUI_FILE, delivering to
# PLANNOTATOR_TUI_DELIVER_TO, the Claude's pane. q closes it.
[[panes]]
id = "review"
title = "Task docs: annotate · Send goes to the task's Claude"
placement = "split"
command = ["sh", "-c", "exec python3 \"$HERDR_PLUGIN_ROOT/task_docs.py\" review"]
```

- [ ] **Step 2: Write the failing tests**

Create an empty `herdr/task-docs/tests/__init__.py`, and `herdr/task-docs/tests/test_task_docs.py`:

```python
import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import task_docs  # noqa: E402

# herdr: logs each call as a JSON list, and answers `agent get <target>` from
# the FAKE_AGENTS map, exiting 1 for a target it does not hold, as herdr does.
FAKE_HERDR = """#!/usr/bin/env python3
import json, os, sys
with open(os.environ["FAKE_HERDR_LOG"], "a") as log:
    log.write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[1:3] == ["agent", "get"]:
    agent = json.load(open(os.environ["FAKE_AGENTS"])).get(sys.argv[3])
    if agent is None:
        print('{"error":{"code":"agent_not_found"}}', file=sys.stderr)
        sys.exit(1)
    print(json.dumps({"result": {"agent": agent}}))
"""

# A command that writes its arguments, one per line, to the file named by $LOG_VAR.
FAKE_LOGGER = """#!/bin/sh
printf '%s\\n' "$@" >> "$LOG_VAR"
"""

# niritasks: logs its arguments, prints two documents.
FAKE_NIRITASKS = """#!/bin/sh
printf '%s\\n' "$@" > "$FAKE_NIRITASKS_LOG"
printf 'Report · 2026-10-11\\t/r/a.md\\n'
printf 'Plan · b.md\\t/p/b.md\\n'
"""


def fake(folder, name, body):
    path = Path(folder) / name
    path.write_text(body)
    path.chmod(0o755)
    return path


class PluginTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.herdr_log = self.tmp / "herdr.log"
        self.notify_log = self.tmp / "notify.log"
        self.open_log = self.tmp / "xdg-open.log"
        self.agents = self.tmp / "agents.json"
        self.with_agents({})
        fake(self.bin, "herdr", FAKE_HERDR)
        fake(self.bin, "notify-send", FAKE_LOGGER.replace("LOG_VAR", "FAKE_NOTIFY_LOG"))
        fake(self.bin, "xdg-open", FAKE_LOGGER.replace("LOG_VAR", "FAKE_OPEN_LOG"))
        self.folder = self.tmp / "work"
        self.folder.mkdir()
        env = {
            "PATH": f"{self.bin}:/usr/bin:/bin",
            "HERDR_BIN_PATH": str(self.bin / "herdr"),
            "FAKE_HERDR_LOG": str(self.herdr_log),
            "FAKE_AGENTS": str(self.agents),
            "FAKE_NOTIFY_LOG": str(self.notify_log),
            "FAKE_OPEN_LOG": str(self.open_log),
            "XDG_CONFIG_HOME": str(self.tmp / "config"),
            "HERDR_PLUGIN_ID": "task-docs",
            "HERDR_PLUGIN_STATE_DIR": str(self.tmp / "state"),
            "HERDR_PANE_ID": "w1:p1",
            "HERDR_PLUGIN_CONTEXT_JSON": json.dumps(
                {"focused_pane_id": "w1:p1", "focused_pane_cwd": str(self.folder), "focused_pane_agent": "claude"}
            ),
        }
        patcher = mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)

    def with_agents(self, agents):
        self.agents.write_text(json.dumps(agents))

    def with_reviewer(self):
        fake(self.bin, "plannotator-tui", "#!/bin/sh\n")

    def on_branch(self, branch):
        subprocess.run(["git", "init", "-q", "-b", branch, str(self.folder)], check=True)

    def calls(self):
        if not self.herdr_log.exists():
            return []
        return [json.loads(line) for line in self.herdr_log.read_text().splitlines()]

    def pane_opens(self):
        return [c for c in self.calls() if c[:3] == ["plugin", "pane", "open"]]

    def notified(self):
        return self.notify_log.read_text() if self.notify_log.exists() else ""


class FindingTheTask(PluginTest):
    def test_an_agent_named_for_a_task_gives_its_uuid8(self):
        self.assertEqual(task_docs.uuid8_of_agent("work-1234abcd"), "1234abcd")
        self.assertEqual(task_docs.uuid8_of_agent("task-1234abcd"), "1234abcd")
        self.assertIsNone(task_docs.uuid8_of_agent("reviewer"))
        self.assertIsNone(task_docs.uuid8_of_agent(None))

    def test_a_task_branch_gives_its_uuid8(self):
        self.on_branch("task/fix-the-peek-1234abcd")
        self.assertEqual(task_docs.uuid8_of_branch(self.folder), "1234abcd")

    def test_another_branch_or_no_repo_gives_none(self):
        self.assertIsNone(task_docs.uuid8_of_branch(self.folder))
        self.on_branch("main")
        self.assertIsNone(task_docs.uuid8_of_branch(self.folder))

    def test_the_panes_agent_name_comes_before_its_branch(self):
        self.on_branch("task/other-99999999")
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        self.assertEqual(task_docs.find_task("w1:p1", self.folder), "1234abcd")

    def test_the_tasks_claude_is_this_pane_then_work_then_task(self):
        self.with_agents({"w1:p1": {"name": "task-1234abcd", "pane_id": "w1:p1"},
                          "work-1234abcd": {"name": "work-1234abcd", "pane_id": "w2:p1"}})
        self.assertEqual(task_docs.task_claude("w1:p1", "1234abcd"), "w1:p1")
        self.assertEqual(task_docs.task_claude("w1:p9", "1234abcd"), "w2:p1")
        self.with_agents({"task-1234abcd": {"name": "task-1234abcd", "pane_id": "w3:p1"}})
        self.assertEqual(task_docs.task_claude(None, "1234abcd"), "w3:p1")
        self.with_agents({})
        self.assertIsNone(task_docs.task_claude("w1:p1", "1234abcd"))


class TheAction(PluginTest):
    def test_a_pane_with_no_task_says_so_and_opens_nothing(self):
        self.with_reviewer()
        task_docs.open_action()
        self.assertIn("Task docs: no task here", self.notified())
        self.assertEqual(self.pane_opens(), [])

    def test_without_plannotator_tui_it_says_herdr_annotate_full_is_needed(self):
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        task_docs.open_action()
        self.assertIn("plannotator/herdr-annotate", self.notified())
        self.assertEqual(self.pane_opens(), [])

    def test_a_task_with_no_claude_says_so(self):
        self.with_reviewer()
        self.on_branch("task/fix-1234abcd")
        task_docs.open_action()
        self.assertIn("no Claude on this task", self.notified())
        self.assertEqual(self.pane_opens(), [])

    def test_a_tasks_pane_opens_the_list_over_it(self):
        self.with_reviewer()
        self.with_agents({"w1:p1": {"name": "work-1234abcd", "pane_id": "w1:p1"}})
        task_docs.open_action()
        self.assertEqual(self.pane_opens(), [[
            "plugin", "pane", "open", "--plugin", "task-docs", "--entrypoint", "pick",
            "--placement", "overlay", "--cwd", str(self.folder), "--focus",
            "--env", "TASK_DOCS_TASK=1234abcd", "--env", "TASK_DOCS_CLAUDE=w1:p1",
        ]])

    def test_from_a_shell_on_the_tasks_branch_the_comments_go_to_its_working_claude(self):
        self.with_reviewer()
        self.on_branch("task/fix-1234abcd")
        self.with_agents({"work-1234abcd": {"name": "work-1234abcd", "pane_id": "w5:p2"}})
        task_docs.open_action()
        self.assertIn("TASK_DOCS_CLAUDE=w5:p2", self.pane_opens()[0])

    def test_plannotator_tui_from_herdr_annotate_full_counts(self):
        staged = self.tmp / "config/herdr/plugins/github/annotate-abc123/bin/plannotator-tui.exe"
        staged.parent.mkdir(parents=True)
        fake(staged.parent, staged.name, "#!/bin/sh\n")
        self.assertEqual(task_docs.reviewer(), str(staged))


class ThePicker(PluginTest):
    def test_lines_split_at_the_first_tab_and_others_are_skipped(self):
        self.assertEqual(
            task_docs.parse_docs("Report · 2026-10-11\t/r/a\tb.md\nno tab here\n"),
            [("Report · 2026-10-11", "/r/a\tb.md")],
        )

    def test_it_asks_niritasks_for_the_tasks_documents(self):
        fake(self.bin, "niritasks", FAKE_NIRITASKS)
        log = self.tmp / "niritasks.log"
        with mock.patch.dict(os.environ, {"FAKE_NIRITASKS_LOG": str(log)}):
            items = task_docs.docs("1234abcd", self.folder)
        self.assertEqual(items, [("Report · 2026-10-11", "/r/a.md"), ("Plan · b.md", "/p/b.md")])
        self.assertEqual(log.read_text().split("\n")[:-1], [
            "task", "docs", "1234abcd", "--project", str(self.folder), "--notes-dir", str(self.tmp / "state"),
        ])

    def test_the_pick_opens_beside_the_claude_named_by_the_action(self):
        fake(self.bin, "niritasks", FAKE_NIRITASKS)
        env = {"TASK_DOCS_TASK": "1234abcd", "TASK_DOCS_CLAUDE": "w1:p1", "FAKE_NIRITASKS_LOG": str(self.tmp / "n.log")}
        with mock.patch.dict(os.environ, env), \
                mock.patch.object(task_docs, "choose", return_value="/r/a.md") as choose, \
                mock.patch.object(task_docs, "open_doc") as open_doc:
            task_docs.pick()
        choose.assert_called_once_with([("Report · 2026-10-11", "/r/a.md"), ("Plan · b.md", "/p/b.md")])
        open_doc.assert_called_once_with("/r/a.md", "w1:p1")

    def test_esc_opens_nothing(self):
        fake(self.bin, "niritasks", FAKE_NIRITASKS)
        env = {"TASK_DOCS_TASK": "1234abcd", "TASK_DOCS_CLAUDE": "w1:p1", "FAKE_NIRITASKS_LOG": str(self.tmp / "n.log")}
        with mock.patch.dict(os.environ, env), \
                mock.patch.object(task_docs, "choose", return_value=None), \
                mock.patch.object(task_docs, "open_doc") as open_doc:
            task_docs.pick()
        open_doc.assert_not_called()


class OpeningADocument(PluginTest):
    def test_markdown_opens_in_plannotator_tui_right_of_the_claude_and_delivers_to_it(self):
        task_docs.open_doc("/r/refine-a.md", "w1:p1")
        self.assertEqual(self.pane_opens(), [[
            "plugin", "pane", "open", "--plugin", "task-docs", "--entrypoint", "review",
            "--placement", "split", "--direction", "right", "--target-pane", "w1:p1", "--focus",
            "--cwd", "/r", "--env", "PLANNOTATOR_TUI_FILE=/r/refine-a.md",
            "--env", "PLANNOTATOR_TUI_DELIVER_TO=w1:p1",
        ]])

    def test_a_report_with_no_twin_opens_in_the_browser(self):
        task_docs.open_doc("/r/refine-a.html", "w1:p1")
        for _ in range(50):
            if self.open_log.exists():
                break
            time.sleep(0.05)
        self.assertEqual(self.open_log.read_text(), "/r/refine-a.html\n")
        self.assertEqual(self.pane_opens(), [])


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 3: Run them to see them fail**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs 2>&1 | tail -5`
Expected: FAIL with `ModuleNotFoundError: No module named 'task_docs'`.

- [ ] **Step 4: Write `herdr/task-docs/task_docs.py`**

```python
#!/usr/bin/env python3
"""herdr's task-docs plugin: annotate one of a task's documents in plannotator-tui.

`task_docs.py open` is the action. It finds the task of the pane the key was
pressed in, from its agent's name (work-/task-<uuid8>) or its folder's branch
(task/<slug>-<uuid8>), and that task's Claude, then opens the `pick` overlay.
`task_docs.py pick` runs in that overlay: fzf over `niritasks task docs`, and
the pick opens in the `review` pane, right of the task's Claude, where
plannotator-tui's Send makes the comments that Claude's next message. A report
made before reports had a Markdown twin opens in the browser instead.
`task_docs.py review` runs in that pane: plannotator-tui's own pane mode.
"""

import glob
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

PLUGIN = os.environ.get("HERDR_PLUGIN_ID") or "task-docs"
AGENT = re.compile(r"^(?:work|task)-([0-9a-f]{8})$")
BRANCH = re.compile(r"^task/.+-([0-9a-f]{8})$")
FULL = "Install herdr-annotate Full, which ships it: herdr plugin install plannotator/herdr-annotate"
PREVIEW = (
    'case {2} in *.md) head -n 200 {2} ;; '
    '*) echo "Opens in the browser: a report made before reports had a Markdown twin." ;; esac'
)


def state_dir():
    """Where the task's notes are written as Markdown: herdr's state folder for this plugin."""
    return Path(os.environ.get("HERDR_PLUGIN_STATE_DIR") or Path.home() / ".local/state/herdr/plugins/task-docs")


def herdr(*args):
    """Run a herdr CLI command, capturing its output."""
    binary = os.environ.get("HERDR_BIN_PATH") or "herdr"
    return subprocess.run([binary, *args], capture_output=True, text=True, check=False)


def notify(body):
    """A desktop notification, as niritasks sends: herdr shows none for the tab in
    front, which is where this key is pressed. herdr's own without notify-send."""
    try:
        subprocess.run(["notify-send", "Tasks", body], stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, check=False)
    except OSError:
        herdr("notification", "show", "Tasks", "--body", body, "--sound", "none")


def context():
    """herdr's context for this invocation, or {} when it is missing or unreadable."""
    try:
        return json.loads(os.environ.get("HERDR_PLUGIN_CONTEXT_JSON") or "{}")
    except ValueError:
        return {}


def agent(target):
    """herdr's record of the agent in a pane, or of that name, or None."""
    result = herdr("agent", "get", target)
    if result.returncode != 0:
        return None
    try:
        return json.loads(result.stdout)["result"]["agent"]
    except (ValueError, KeyError, TypeError):
        return None


def uuid8_of_agent(name):
    """The task a work-/task-<uuid8> agent is named for, or None."""
    match = AGENT.match(name or "")
    return match.group(1) if match else None


def uuid8_of_branch(cwd):
    """The task a task/<slug>-<uuid8> branch checked out in cwd is for, or None."""
    try:
        out = subprocess.run(["git", "-C", str(cwd), "branch", "--show-current"],
                             capture_output=True, text=True, check=False).stdout.strip()
    except OSError:
        return None
    match = BRANCH.match(out)
    return match.group(1) if match else None


def find_task(pane, cwd):
    """The uuid8 of the pane's task: its agent's name first, else its folder's branch."""
    here = agent(pane) if pane else None
    return uuid8_of_agent((here or {}).get("name")) or (uuid8_of_branch(cwd) if cwd else None)


def task_claude(pane, uuid8):
    """The pane of the task's Claude: this pane when its agent is named for the
    task, else the working Claude's (work-), else the refine's (task-)."""
    for target in (pane, f"work-{uuid8}", f"task-{uuid8}"):
        if not target:
            continue
        found = agent(target)
        if found and uuid8_of_agent(found.get("name")) == uuid8:
            return found.get("pane_id")
    return None


def reviewer():
    """plannotator-tui: on PATH, else the copy herdr-annotate Full stages in its plugin folder."""
    found = shutil.which("plannotator-tui")
    if found:
        return found
    config = Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config")
    for path in sorted(glob.glob(str(config / "herdr/plugins/github/annotate-*/bin/plannotator-tui.exe"))):
        if os.access(path, os.X_OK):
            return path
    return None


def open_action():
    """The task-docs.open action: open the task's documents over its pane, or say why not."""
    ctx = context()
    pane = os.environ.get("HERDR_PANE_ID") or ctx.get("focused_pane_id")
    cwd = ctx.get("focused_pane_cwd") or os.getcwd()
    uuid8 = find_task(pane, cwd)
    if not uuid8:
        notify("Task docs: no task here. This pane runs no task's Claude (work-/task-<uuid8>) "
               "and is not on a task/<slug>-<uuid8> branch.")
        return
    if not reviewer():
        notify(f"Task docs: plannotator-tui is needed. {FULL}")
        return
    claude = task_claude(pane, uuid8)
    if not claude:
        notify(f"Task docs: no Claude on this task. Neither work-{uuid8} nor task-{uuid8} is running, "
               "so there is no one to send comments to.")
        return
    result = herdr("plugin", "pane", "open", "--plugin", PLUGIN, "--entrypoint", "pick",
                   "--placement", "overlay", "--cwd", str(cwd), "--focus",
                   "--env", f"TASK_DOCS_TASK={uuid8}", "--env", f"TASK_DOCS_CLAUDE={claude}")
    if result.returncode != 0:
        notify(f"Task docs: could not open the list. {result.stderr.strip()[-300:]}")


def parse_docs(stdout):
    """`niritasks task docs` lines as (label, path) pairs; a line with no tab is skipped."""
    return [tuple(line.split("\t", 1)) for line in stdout.splitlines() if "\t" in line]


def docs(uuid8, cwd):
    """The task's documents, from niritasks; its notes written into the state folder."""
    result = subprocess.run(
        ["niritasks", "task", "docs", uuid8, "--project", str(cwd), "--notes-dir", str(state_dir())],
        capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip()[-300:] or "niritasks task docs failed")
    return parse_docs(result.stdout)


def choose(items):
    """fzf over the labels, with a preview; the picked path, or None on Esc."""
    result = subprocess.run(
        ["fzf", "--delimiter", "\t", "--with-nth", "1", "--no-sort", "--prompt", "doc> ",
         "--header", "Enter annotates it beside the task's Claude, Esc cancels",
         "--preview", PREVIEW, "--preview-window", "right,60%,wrap"],
        input="\n".join(f"{label}\t{path}" for label, path in items),
        stdout=subprocess.PIPE, text=True, check=False,
    )
    choice = result.stdout.strip()
    if result.returncode != 0 or "\t" not in choice:
        return None
    return choice.split("\t", 1)[1]


def open_doc(path, claude):
    """Markdown in plannotator-tui, right of the task's Claude and delivering to it;
    anything else, a report with no twin, in the browser."""
    if not path.endswith(".md"):
        subprocess.Popen(["xdg-open", path], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                         stderr=subprocess.DEVNULL, start_new_session=True)
        return
    result = herdr("plugin", "pane", "open", "--plugin", PLUGIN, "--entrypoint", "review",
                   "--placement", "split", "--direction", "right", "--target-pane", claude, "--focus",
                   "--cwd", str(Path(path).parent),
                   "--env", f"PLANNOTATOR_TUI_FILE={path}", "--env", f"PLANNOTATOR_TUI_DELIVER_TO={claude}")
    if result.returncode != 0:
        notify(f"Task docs: could not open the reviewer. {result.stderr.strip()[-300:]}")


def pick():
    """Run in the overlay: list the task's documents and open the one picked."""
    uuid8 = os.environ.get("TASK_DOCS_TASK")
    claude = os.environ.get("TASK_DOCS_CLAUDE")
    if not uuid8 or not claude:
        notify("Task docs: the list was opened without a task.")
        return
    try:
        items = docs(uuid8, Path.cwd())
    except (OSError, RuntimeError) as error:
        notify(f"Task docs: could not list the documents. {error}")
        return
    if not items:
        notify(f"Task docs: task {uuid8} has no report, notes or plan to annotate.")
        return
    try:
        path = choose(items)
    except OSError:
        notify("Task docs: fzf not found. Install fzf to pick a document.")
        return
    if path:
        open_doc(path, claude)


def review():
    """Run in the review pane: plannotator-tui's pane mode, which reads
    PLANNOTATOR_TUI_FILE and PLANNOTATOR_TUI_DELIVER_TO."""
    binary = reviewer()
    if not binary:
        print(f"plannotator-tui is not installed. {FULL}\n\npress enter to close", file=sys.stderr)
        notify(f"Task docs: plannotator-tui is needed. {FULL}")
        input()
        return
    os.execv(binary, [binary, "herdr", "pane"])


def main(argv):
    commands = {"open": open_action, "pick": pick, "review": review}
    if len(argv) != 2 or argv[1] not in commands:
        print("usage: task_docs.py open|pick|review", file=sys.stderr)
        return 2
    commands[argv[1]]()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs -v 2>&1 | tail -8`
Expected: every test passes (`OK`).

- [ ] **Step 6: Add the plugin's tests to `tests/all.sh`**

After `why_box() { … }`, add:

```bash
why_plugin() {
    command -v python3 >/dev/null || { echo "no python3"; return; }
    command -v git >/dev/null || { echo "no git"; return; }
}
```

After `suite "tests/e2e-tag.sh" …`, add:

```bash
suite "herdr/task-docs tests" "$(why_plugin)" \
    python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs
```

In the comment block at the top, change "There are four suites" to "There are five suites", and add after the sentence about cargo: "one needs python3 and git, for the herdr plugin's tests, which fake herdr and touch nothing."

Run: `bash tests/all.sh 2>&1 | tail -10`
Expected: `PASS  herdr/task-docs tests`, and no suite fails that passed before.

- [ ] **Step 7: Commit**

```bash
git add herdr/task-docs tests/all.sh
git commit -m "feat(herdr): add task-docs, a plugin to annotate a task's documents

One action finds the pane's task from its agent name or branch, lists
its documents from niritasks task docs in an fzf overlay, and opens the
pick in plannotator-tui right of the task's Claude, delivering to it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Install it and document it

**Files:**
- Modify: `install.sh` (end of section 3, before `# ─── 4. the daemon`)
- Modify: `README.md` (Requirements; a new `### Annotating a task's documents in herdr` subsection after the Privacy subsection; the tests table under "Running the tests"), `CONTEXT.md` (the **Refine report** term), `llms.txt` (the herdr plugin under `## Optional`)

**Interfaces:**
- Consumes: `herdr/task-docs/` from Task 3.
- Produces: the installed plugin, linked as `local:<repo>/herdr/task-docs`, and the documented key `prefix+shift+f` running `task-docs.open`.

- [ ] **Step 1: Link the plugin in `install.sh`**

Before `# ─── 4. the daemon`, add:

```bash
# The herdr plugin: task-docs, a key in a task's herdr tab that lists the
# task's reports, notes and plans and opens the pick in plannotator-tui
# beside its Claude. `herdr plugin link` points herdr at this folder, so an
# edit here is live, as the links above are; linking from another checkout (a
# worktree) replaces the link, as `link` does. Left alone when it already
# points here, so re-runs are quiet. The key is the README's to bind.
if command -v herdr >/dev/null; then
    if herdr plugin list 2>/dev/null | grep -qF "[local:$REPO/herdr/task-docs]"; then
        :
    elif herdr plugin link "$REPO/herdr/task-docs" >/dev/null 2>&1; then
        info "Linked the herdr plugin task-docs"
    else
        warn "Could not link the herdr plugin task-docs — run: herdr plugin link $REPO/herdr/task-docs"
    fi
fi
```

Run: `bash -n install.sh && echo ok`
Expected: `ok`. Do not run `install.sh` from the worktree: it would point the live plugin, skills and refine mod at this worktree. It runs from `main` after landing (Task 5).

- [ ] **Step 2: Document the plugin in `README.md`**

In `### Requirements`, after the paragraph about herdr and Start working, add:

```markdown
**Task docs**, the herdr plugin `install.sh` links, needs herdr 0.9.1 or
later, `python3`, `fzf`, and plannotator-tui from herdr-annotate Full
(`herdr plugin install plannotator/herdr-annotate`; Lite has no
plannotator-tui). Without it the key says so.
```

After the `### Privacy` subsection, add:

````markdown
### Annotating a task's documents in herdr

`install.sh` links a herdr plugin, `herdr/task-docs`. Bind its action to a
key in `~/.config/herdr/config.toml`, then run `herdr config check` and
`herdr server reload-config`:

```toml
[[keys.command]]
key = "prefix+shift+f"
type = "plugin_action"
command = "task-docs.open"
description = "task docs: annotate one of this task's documents"
```

Press it in the pane of a task's Claude, or any pane whose folder is on the
task's `task/<slug>-<uuid8>` branch. A list opens over the pane:

- the task's refine reports, newest first, earlier drafts marked;
- the task's description and notes, as Markdown;
- the plans in the project's `docs/superpowers/plans` and in
  `~/.claude/plans` changed since the task was added, newest first.

Enter opens the pick in plannotator-tui, to the right of the task's Claude.
Comment on it there; **Send** makes the comments that Claude's next message.
A report made before reports had a Markdown twin opens in the browser
instead, as a card's Open report does. A pane with no task, or a task with
no Claude running, gets a notification saying so and nothing opens. The list
is `niritasks task docs <uuid>`, so a script can read it too.

Each refine report is written twice, as its HTML slides and as `<report>.md`
beside them, from the same fields; the Markdown is what plannotator-tui
opens.
````

In the tests table under "Running the tests", add a code line after `cargo test …`:

```
python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs  # the herdr plugin  ~1s
```

and a table row after the `cargo test` row:

```markdown
| `herdr/task-docs` tests | `python3`, `git` | Nothing. A fake `herdr`, `notify-send` and `xdg-open` stand in, in a scratch folder |
```

Change "`tests/all.sh` is the four in one command" to "`tests/all.sh` is the five in one command".

- [ ] **Step 3: Name the twin in `CONTEXT.md` and `llms.txt`**

In `CONTEXT.md`, in **Refine report**, after "It is saved under the reviews folder", insert ", with a Markdown twin, `<report>.md`, beside it for herdr's task-docs plugin to open in plannotator-tui,".

In `llms.txt`, under `## Optional`, add a line in that list's style:

```markdown
- [task-docs](herdr/task-docs/task_docs.py): the herdr plugin that lists a task's documents (`niritasks task docs`) and opens the pick in plannotator-tui beside the task's Claude, whose Send comes back to that Claude
```

- [ ] **Step 4: Run every check**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED" && (cd claude/refine-mod && claude plugin validate . && claude plugin test . 2>&1 | tail -3) && bash tests/all.sh 2>&1 | tail -12`
Expected: every `test result: ok` (the `llms_txt_has_the_llmstxt_shape` test included), the mod's tests pass, and `tests/all.sh` ends `0 failed`.

- [ ] **Step 5: Commit**

```bash
git add install.sh README.md CONTEXT.md llms.txt
git commit -m "docs: describe annotating a task's documents in herdr

install.sh links herdr/task-docs; the README gives its key, what it
lists and what it needs, and the report's Markdown twin is named.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: End-to-end check, after landing

The plugin, the refine mod and the binary run from `~/Projects/niri-tasks`, never from a worktree, so this check runs after `finish-worktree` has landed the branch on `main`. It needs the user at the keyboard in herdr.

**Files:** none in the repo. It changes `~/.config/herdr/config.toml` and may swap herdr-annotate Lite for Full, both on the user's say-so.

- [ ] **Step 1: Install from `main`**

Run: `cd ~/Projects/niri-tasks && bash install.sh 2>&1 | tail -8 && herdr plugin list | grep task-docs`
Expected: `Linked the herdr plugin task-docs`, and the list shows `task-docs (Task docs) enabled [local:/home/paul/Projects/niri-tasks/herdr/task-docs]`.

- [ ] **Step 2: Get plannotator-tui, with the user's agreement**

This machine has herdr-annotate **Lite**, which has no plannotator-tui. Ask the user before running `herdr plugin install plannotator/herdr-annotate`: it replaces Lite under the same id and keeps its terminal-annotation actions. Then check: `ls ~/.config/herdr/plugins/github/annotate-*/bin/plannotator-tui.exe`.

- [ ] **Step 3: Bind the key, with the user's agreement**

Add the README's `[[keys.command]]` block to `~/.config/herdr/config.toml`, with a comment in that file's style saying what it does. Run `herdr config check && herdr server reload-config`. If `prefix+shift+f` turns out to run one of herdr's own actions, use another free letter (`h i j k l q s u y z`), in the config and the README both.

- [ ] **Step 4: Walk the "Done when"**

In a task's herdr tab:

1. Press the key in the task's Claude pane. The list shows that task's documents, its notes among them.
2. Pick a Markdown document. plannotator-tui opens to the right of the Claude, focused.
3. Comment on a line and press Send. The comments arrive as that Claude's next message.
4. Press the key in a pane with no task, such as a plain shell in `~`. A notification says there is no task here, and nothing opens.
5. Press **Report** on a planned card with `--fresh` (`niritasks task report <uuid> --fresh`). Once it is written, `ls ~/.local/share/niri-tasks/reviews/` shows a `.md` beside the new `.html`, and the key lists it first. It opens in plannotator-tui.

Record anything that fails as a `fix:` task with `niritasks task add`.

---

## Self-review

- **Spec coverage.** The twin and its test are Task 1. `task docs` listing report twins, the notes as Markdown in the plugin state dir, and both plan folders newest first is Task 2. The manifest, one action, the fzf overlay and the review pane, task lookup by agent name or branch, delivery to the task's Claude, the plannotator-tui check, and herdr-speak-style tests are Task 3. The `install.sh` link and the README key and requirement are Task 4. Every "Done when" item is walked in Task 5. The spec's "check `plannotator-tui --help`" is settled in "What research settled": its pane mode takes the file and the delivery pane by environment, so no comment export or `herdr agent prompt` fallback is needed.
- **Names across tasks.** `markdownPath` and `reportMarkdown` (Task 1); `ReportNote`, `report_notes`, `EARLIER_DRAFT_NOTE`, `stamp_secs` (Task 2, `task.rs`); `Doc`, `twin`, `reports`, `notes_markdown`, `write_notes`, `plans`, `list` (Task 2, `docs.rs`); `task docs <uuid> --project --notes-dir` (Task 2, used by Task 3's `docs()`); `task-docs.open`, entrypoints `pick` and `review`, env `TASK_DOCS_TASK` and `TASK_DOCS_CLAUDE` (Task 3, used by Task 4's README).
