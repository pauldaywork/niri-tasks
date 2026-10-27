// A branch report as slides: the plan report's renderer, styles and lenses,
// in the branch's own order, with the slides only a branch has: where to
// look hardest, how the commits got here, what each file now does, whether
// anything broke, the tests it adds, and its plan against what is done.
// No `$`, so the unit tests reach it directly.

import type { Plan } from './plan'
import { BRANCH_HEADINGS as H } from './report'
import type { Commit, Facts, HotSpot, Lens, MapEntry, NewTest, PlanItem, Progress, Report, Result, Step, Suite, Untested } from './report'
import {
  badge, bullets, count, deck, decision, escapeHtml, files, glance, inline, lensSlides, needsBrowser, path, plain,
  readingMinutes, terms,
} from './page'
import type { Slide } from './page'
import { BRANCH_CSS } from './style'
import type { Fonts } from './style'

const RESULT_CLASS: Record<Result, string> = { pass: 'pass', fail: 'fail', 'not run': 'notrun' }
const PROGRESS_CLASS: Record<Progress, string> = { done: 'done', partial: 'partial', 'not yet': 'notyet' }

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
      ? `<p class="verdict"><span class="badge pass">nothing broke</span> ${number(passed)} ${passed === 1 ? 'test' : 'tests'} passed ${across}</p>`
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
  return deck(report.title, all, needsBrowser(report.sections, drawn), fonts, BRANCH_CSS)
}
