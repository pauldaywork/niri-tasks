# Report Notes Under Home Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `Report:` note survives a username or home folder change: the refine mod writes `Report: ~/...` for a report under home, and the notes already stored are rewritten to match.

**Architecture:** `src/refine.rs` already holds the home folder in its `Fence`; it passes it to the mod as a new `home` option beside `reports`. A pure helper in `claude/refine-mod/hooks/report.ts` turns an absolute path under home into `~/...` and leaves any other path alone. `hooks/refine.ts` writes and opens the HTML and Markdown files by their absolute path, and uses the `~/` form everywhere a path is *named*: the task note, the transcript's `Report:` line, the tool's answers, and the paths `ownNotes` matches. `niritasks` already reads `~/` notes (`report_file` in `src/refine.rs`), so no reader changes. The old notes are rewritten once by hand with `task export` → `jq` → `task import`.

**Tech Stack:** Rust (`cargo test`), the TypeScript Claude Code mod in `claude/refine-mod` (tests through `claude-code/testing`, run with `claude plugin test`), Taskwarrior 3.5, `jq`.

**Spec:** Taskwarrior task `2fc9356b-da9b-460d-bd6d-ad121fd6d7fe`, its description and notes (`task rc.json.array=on 2fc9356b-da9b-460d-bd6d-ad121fd6d7fe export`). There is no separate spec file.

## Global Constraints

- The note form is `Report: ~/<rest>` only when the absolute path starts with `<home>/`; a path outside home (a data folder elsewhere) stays absolute.
- `/home/xy/...` is *not* under home `/home/x`: match `<home>/`, never a bare prefix.
- The HTML report and its Markdown twin are still written, and opened, by the absolute path.
- The note, the transcript's `Report:` log line, the tool's answers (`Wrote the report to ...`, `the note "Report: ..."`, the second-report refusal) and `ownNotes`' matching all use the `~/` form.
- No `home` option (or one that is not absolute) means every path stays absolute: the existing tests that set only `reports: '/r'` must pass unchanged.
- Old notes are rewritten once by hand, not by a shipped command. `Report:` and `Report (earlier draft):` notes on every task, whatever its status (pending, completed, deleted). `file://$HOME/` and `$HOME/` become `~/`. Each note keeps its `entry` date.
- Out of scope: syncing the reviews folder to other machines; `Spec:` and `Plan:` notes; making `file:///` notes with `#fragments` openable (they become `~/...#c3 (prose)` and stay as unopenable as before).
- Commits: Conventional Commits, type `refactor` (this task's type) unless a step says otherwise, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Files

| File | Change | Responsibility |
|---|---|---|
| `src/refine.rs` | Modify | `session_settings` adds `"home"` to the mod's options; doc comments on `session_settings` and `report_file`; one Rust test |
| `claude/refine-mod/.claude-plugin/plugin.json` | Modify | declare the `home` option |
| `claude/refine-mod/hooks/report.ts` | Modify | `homeDir`, `notePath`; `MadeReport.path` documented as the note form |
| `claude/refine-mod/tests/report.test.ts` | Modify | unit tests for `homeDir`, `notePath`, and `withReportNotes` on `~/` paths |
| `claude/refine-mod/hooks/refine.ts` | Modify | read `options.home`; name reports by `notePath` |
| `claude/refine-mod/tests/refine.test.ts` | Modify | engine tests for a session told its home folder |

---

### Task 1: Tell the mod the home folder

**Files:**
- Modify: `src/refine.rs:140-157` (the `report_file` doc comment), `src/refine.rs:218-255` (`session_settings` doc comment and `pluginConfigs`), tests module near `src/refine.rs:1156`
- Modify: `claude/refine-mod/.claude-plugin/plugin.json`

**Interfaces:**
- Consumes: `Fence { home: PathBuf, .. }` (already a parameter of `session_settings`; the test helper `fence(&[])` gives home `/home/x`).
- Produces: the mod's options gain `home: string`, the absolute home folder, e.g. `"/home/x"`. `session_settings`' signature does not change.

- [ ] **Step 1: Write the failing test**

In `src/refine.rs`'s tests module, directly after `the_mod_is_told_which_task_it_may_write_and_where_reports_go`, add:

```rust
    /// The mod is told the home folder, so the note it writes names a report
    /// under it as `~/...`, which survives a username or home folder change.
    #[test]
    fn the_mod_is_told_the_home_folder() {
        let json = session_settings(Path::new("/p"), Path::new("/t"), &fence(&[]), "u", Path::new("/home/x/.local/share/niri-tasks/reviews"), Mode::Quick, false, None);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["pluginConfigs"][REFINE_MOD]["options"]["home"], "/home/x");
    }
```

- [ ] **Step 2: Run it to make sure it fails**

Run: `cargo test the_mod_is_told_the_home_folder`
Expected: FAIL, `left: Null`, `right: "/home/x"`.

- [ ] **Step 3: Pass the home folder**

In `session_settings`, change the `pluginConfigs` line to:

```rust
            (REFINE_MOD): { "options": { "uuid": uuid, "reports": reports, "home": fence.home, "report": (mode == Mode::Report), "unattended": unattended } },
```

In its doc comment, replace the line

```rust
/// It is told `reports` too, the folder its report tool writes to, and
```

with

```rust
/// It is told `reports` too, the folder its report tool writes to, `home`,
/// so the note linking a report under it names it as `~/...`, and
```

Replace the `report_file` doc comment (the three lines above `pub fn report_file`) with:

```rust
/// The file a `Report: <path>` note names. The mod writes `~/...` for a
/// report under the home folder and an absolute path for one outside it; an
/// absolute path is used as written, `~` or `~/...` is under the home
/// folder, and a relative path is in [`reports_dir`].
```

- [ ] **Step 4: Declare the option in the mod**

In `claude/refine-mod/.claude-plugin/plugin.json`, add to `userConfig`, after `"reports"`:

```json
    "home": {
      "type": "string",
      "title": "Home folder",
      "description": "The absolute home folder. A report under it is named from ~/ in the task's Report: note and in the tool's answers, so the note survives a username or home folder change; the files are still written and opened by their absolute path. Unset, or for a report outside it, the absolute path is named.",
      "required": false,
      "default": ""
    },
```

Check it is still valid JSON: `jq . claude/refine-mod/.claude-plugin/plugin.json >/dev/null && echo ok` → `ok`.

- [ ] **Step 5: Run the Rust tests**

Run: `cargo test`
Expected: all pass, including `the_mod_is_told_the_home_folder`.

- [ ] **Step 6: Commit**

```bash
git add src/refine.rs claude/refine-mod/.claude-plugin/plugin.json
git commit -m "$(cat <<'EOF'
refactor(refine): tell the refine mod the home folder

The mod gets a home option beside reports, so it can name a report
under home as ~/... in the task's note.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: The `~/` helper in report.ts

**Files:**
- Modify: `claude/refine-mod/hooks/report.ts` (the `reportsDir` block near the end, and the `MadeReport` comment)
- Test: `claude/refine-mod/tests/report.test.ts`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `homeDir(value: unknown): string | undefined` — an absolute folder without its trailing slash, or `undefined` (same rule as `reportsDir`).
  - `notePath(path: string, home: string | undefined): string` — `~/<rest>` when `path` starts with `` `${home}/` ``, else `path` unchanged.
  - `MadeReport.path` is now documented as the note form (what `notePath` gives); its type is unchanged (`string`).

- [ ] **Step 1: Write the failing tests**

In `claude/refine-mod/tests/report.test.ts`, add `homeDir` and `notePath` to the import from `'../hooks/report'` (keep it sorted as it is: `LIMITS, WORD_GRACE, homeDir, madeAlready, notePath, openArgv, ...`). After the `describe('reportsDir', ...)` block add:

```ts
describe('homeDir', () => {
  test('takes an absolute folder, without a trailing slash', () => {
    expect(homeDir('/home/x')).toBe('/home/x')
    expect(homeDir('/home/x/')).toBe('/home/x')
  })
  test('refuses the rest', () => {
    for (const value of [undefined, '', 'x', '~', '/', 7]) expect(homeDir(value)).toBeUndefined()
  })
})

describe('notePath', () => {
  test('names a path under home from ~/', () => {
    expect(notePath('/home/x/.local/share/niri-tasks/reviews/r.html', '/home/x')).toBe(
      '~/.local/share/niri-tasks/reviews/r.html',
    )
  })
  test('leaves a path outside home absolute', () => {
    expect(notePath('/data/niri-tasks/reviews/r.html', '/home/x')).toBe('/data/niri-tasks/reviews/r.html')
    // A neighbour whose name starts with home's is not under it.
    expect(notePath('/home/xy/r.html', '/home/x')).toBe('/home/xy/r.html')
  })
  test('leaves every path absolute without a home', () => {
    expect(notePath('/home/x/r.html', undefined)).toBe('/home/x/r.html')
  })
})
```

Inside `describe('withReportNotes', ...)`, after the test `'keeps a kept note for an unrevised plan once, as it was'`, add:

```ts
  test('a kept ~/ note for a report of this session is kept once, or relabelled once', () => {
    const made = [{ path: '~/r/a.html', plan }]
    const kept = { ...plan, notes: ['Goal: x', 'Report: ~/r/a.html'] }
    expect(withReportNotes(made, kept, plan)).toEqual(['Goal: x', 'Report: ~/r/a.html'])
    const revised = { ...plan, notes: ['Goal: y', 'Report: ~/r/a.html'] }
    expect(withReportNotes(made, revised, plan)).toEqual(['Goal: y', 'Report (earlier draft): ~/r/a.html'])
  })
```

(The `withReportNotes` test passes already, since `ownNotes` matches whatever form `made` holds; it pins that the `~/` form works. The `homeDir`/`notePath` tests are the failing ones.)

- [ ] **Step 2: Run them to make sure they fail**

Run: `cd claude/refine-mod && claude plugin test`
Expected: FAIL in `report.test.ts`: `homeDir` and `notePath` are not exported (an import/type error, or `is not a function`).

- [ ] **Step 3: Implement**

In `claude/refine-mod/hooks/report.ts`, replace the `reportsDir` block:

```ts
// The folder the reports go in, from the session's settings: absolute, or
// none, and then the tool is not offered.
export const reportsDir = (value: unknown): string | undefined => {
  if (typeof value !== 'string' || !value.startsWith('/')) return undefined
  const dir = value.replace(/\/+$/, '')
  return dir === '' ? undefined : dir
}
```

with:

```ts
// An absolute folder from the session's settings, without a trailing slash,
// or undefined.
const absoluteDir = (value: unknown): string | undefined => {
  if (typeof value !== 'string' || !value.startsWith('/')) return undefined
  const dir = value.replace(/\/+$/, '')
  return dir === '' ? undefined : dir
}

// The folder the reports go in, from the session's settings: absolute, or
// none, and then the tool is not offered.
export const reportsDir = absoluteDir

// The home folder, from the session's settings: absolute, or none, and then
// every report is named by its absolute path.
export const homeDir = absoluteDir

// A report's path as the task's note and the tool's answers name it: from
// `~/` under `home`, so the note survives a username or home folder change
// (`niritasks` reads a `~/` note back under the home folder); absolute
// elsewhere, as for a data folder outside home. The files themselves are
// written and opened by the absolute path.
export const notePath = (path: string, home: string | undefined): string =>
  home !== undefined && path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path
```

Replace the `MadeReport` comment:

```ts
// A report made in this session, and the plan the person was shown when
// they asked for it.
```

with:

```ts
// A report made in this session, by the path its note names (`notePath`),
// and the plan the person was shown when they asked for it.
```

- [ ] **Step 4: Run them to make sure they pass**

Run: `cd claude/refine-mod && claude plugin test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add claude/refine-mod/hooks/report.ts claude/refine-mod/tests/report.test.ts
git commit -m "$(cat <<'EOF'
refactor(refine): name a report under home from ~/

notePath turns a report's absolute path under the home folder into
~/..., and leaves one outside it absolute.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Write `Report: ~/...` notes

**Files:**
- Modify: `claude/refine-mod/hooks/refine.ts` (the import from `./report`, `register`, both branches of the `REPORT_TOOL` hook)
- Test: `claude/refine-mod/tests/refine.test.ts`

**Interfaces:**
- Consumes: `homeDir(value: unknown): string | undefined` and `notePath(path: string, home: string | undefined): string` from Task 2; `options.home` from Task 1.
- Produces: nothing later tasks use.

- [ ] **Step 1: Write the failing tests**

In `claude/refine-mod/tests/refine.test.ts`, after the test `"a re-refine that keeps the plan keeps an earlier session's report"`, add:

```ts
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
```

After the test `'report mode: one report a session; the second names the first'`, add:

```ts
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
```

`Run` is the type already declared near the top of the file; `notesOf` is a new local test helper (the hooks' `notesOf` is not imported into this file).

- [ ] **Step 2: Run them to make sure they fail**

Run: `cd claude/refine-mod && claude plugin test`
Expected: the four `under home` tests FAIL, each naming `/home/x/r/...` where `~/r/...` was expected. The outside-home test passes already.

- [ ] **Step 3: Implement**

In `claude/refine-mod/hooks/refine.ts`, extend the import from `./report`:

```ts
import { homeDir, madeAlready, notePath, openArgv, overLimits, parseReport, reportPath, reportsDir, unreportable, withReportNote, withReportNotes } from './report'
```

In `register`, after `const reports = reportsDir(options.reports)`, add:

```ts
  // The home folder: a report under it is named from ~/ in its note and the
  // answers, so the note survives a username or home folder change.
  const home = homeDir(options.home)
```

In the report-mode branch, replace from `const path = reportPath(reports, uuid, await $.clock.now())` to the end of that branch's `return { result: ... }` with:

```ts
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
```

In the refine branch, replace from `const path = reportPath(reports, uuid, await $.clock.now())` to the end of its `return { result: ... }` with:

```ts
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
```

- [ ] **Step 4: Run the tests to make sure they pass**

Run: `cd claude/refine-mod && claude plugin test`
Expected: all pass, the existing `reports: '/r'` tests unchanged.

- [ ] **Step 5: Commit**

```bash
git add claude/refine-mod/hooks/refine.ts claude/refine-mod/tests/refine.test.ts
git commit -m "$(cat <<'EOF'
refactor(refine): link a report under home as Report: ~/...

The task's note, the transcript's Report: line and the tool's answers
name a report under the home folder from ~/, so the note survives a
username or home folder change. The files are still written and opened
by their absolute path.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Rewrite the stored notes, once

This changes the user's live Taskwarrior data, which every worktree and the main checkout share. It is independent of Tasks 1–3 but runs last. **Ask the user before Step 4's import**, showing them Step 3's diff.

**Files:** none in the repo. A backup at `~/.local/share/niri-tasks/report-notes-before-tilde.json` (never `/tmp`).

**Interfaces:** none.

- [ ] **Step 1: Back up and count**

```bash
BACKUP=~/.local/share/niri-tasks/report-notes-before-tilde.json
task rc.hooks=off rc.json.array=on export > "$BACKUP"
jq -r --arg h "$HOME/" '
  [.[] | .annotations[]? | .description | select(test("^Report( \\(earlier draft\\))?: "))]
  | "absolute: \(map(select(startswith("Report: " + $h) or startswith("Report (earlier draft): " + $h))) | length)",
    "file:    \(map(select(contains("file://" + $h))) | length)",
    "tilde:   \(map(select(contains(": ~/"))) | length)"' "$BACKUP"
jq -r '[.[].status] | group_by(.) | map("\(.[0]): \(length)") | .[]' "$BACKUP"
```

Expected: about 17 absolute, 3 file, 4 tilde (more if reports were made since 2026-10-13); and the status counts include `deleted` and `completed`, proving the unfiltered export holds every status. If `deleted` is missing, export again with the filter `'status:deleted or status:completed or status:pending or status:waiting or status:recurring'` and repeat.

- [ ] **Step 2: Build the rewritten tasks**

```bash
OUT=~/.local/share/niri-tasks/report-notes-tilde.json
jq --arg h "$HOME/" '
  def report: .description | test("^Report( \\(earlier draft\\))?: ");
  def tilde: .description |= (split("file://" + $h) | join("~/") | split($h) | join("~/"));
  [ .[]
    | select(any(.annotations[]?; report and (.description | contains($h))))
    | .annotations |= map(if report then tilde else . end) ]' "$BACKUP" > "$OUT"
jq length "$OUT"
```

Expected: the number of tasks holding such a note (about 19).

- [ ] **Step 3: Check the diff**

```bash
lines() { jq -r '.[] | .uuid[:8] as $u | .annotations[]? | select(.description | test("^Report")) | "\($u) \(.entry) \(.description)"'; }
diff <(jq --slurpfile out "$OUT" '[.[] | select(.uuid as $u | $out[0] | any(.uuid == $u))]' "$BACKUP" | lines) \
     <(lines < "$OUT")
```

Expected: each changed line differs only in `/home/<user>/` or `file:///home/<user>/` becoming `~/`, with the same uuid prefix and the same entry date. A `file:///` note keeps its `#c3 (prose)` tail. Show this diff to the user and ask before importing.

- [ ] **Step 4: Import (after the user says yes)**

```bash
task rc.hooks=off rc.verbose=nothing import < "$OUT"
```

Expected: exit 0.

- [ ] **Step 5: Verify**

```bash
task rc.hooks=off rc.json.array=on export | jq -r --arg h "$HOME/" '
  .[] | .annotations[]? | .description
  | select(test("^Report( \\(earlier draft\\))?: ")) | select(contains($h) or contains("file:///"))'
```

Expected: no output.

```bash
niritasks task docs e1aaecb7
```

Expected: it lists both of that task's reports (the `Report:` and the earlier draft), each resolved to an existing file under the home folder (`ls` the paths it prints). Open one pending task's card (for example `9686dc96` or `dc042cca`) and check its Go to… opens the report and shows it dated as before (the note's entry, 2026-10-10 or 2026-10-13), not today.

- [ ] **Step 6: Nothing to commit**

The rewrite touches no repo file. Leave the backup in place; tell the user its path.

---

## After landing

The mod runs from `~/.local/share/niri-tasks/refine-mod`, a link to the main checkout, so only after `finish-worktree` lands this branch: press Generate report on a planned task and check its new note reads `Report: ~/.local/share/niri-tasks/reviews/refine-<uuid8>-<stamp>.html` and the card's Go to… opens it.
