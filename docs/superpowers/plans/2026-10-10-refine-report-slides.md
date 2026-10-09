# The Refine Report as Slides Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Break the refine report up into full-screen slides in the BlockFrame style. The page snaps one slide at a time as it scrolls, with no script, and uses two embedded open fonts.

**Architecture:** `hooks/page.ts` keeps the report's fields, limits, policy and Mermaid as they are. It renders them as a fixed sequence of `<section class="slide">`, each a full screen with:
- a kind pill and an `NN / NN` counter;
- one framed card;
- a "Next: <name> ↓" link.

A CSS-only `scroll-snap-type: y mandatory` stops the scroll on each slide, so Page Down, Space, ↓ and the wheel move one slide with nothing running. A new `hooks/style.ts` holds the BlockFrame stylesheet and the `@font-face` rules. The mod ships Space Grotesk and Inter (variable, latin, SIL OFL) in `claude/refine-mod/fonts/`, reads them on the first report with `$.fs.read`, and embeds them as `data:` URLs, which the existing policy allows (`font-src data:`).

**Tech Stack:** TypeScript Claude Code mod (`claude plugin test`, `claude plugin validate`), Mermaid 11.17.2 (unchanged), `@fontsource-variable/inter@5.3.0` and `@fontsource-variable/space-grotesk@5.3.0` woff2 files, bun (render by hand only), headless Chrome (render and screenshot checks).

**Spec:** Taskwarrior task `691724fc-d9ff-408f-b00e-a9bcc693ef18`, plus the user's requirements from conversation:
- 2026-10-09: "I want these reports to be really easy for people with adhd to understand." and "make sure any generated reports are linked in the notes of the niritask task".
- 2026-10-10: "can the report be in slides so its broken up a bit", taking ideas from the beautiful-html-templates BlockFrame, Capsule, Creative Mode, Monochrome, Cobalt Grid and Pin & Paper templates.
- The user's choices on 2026-10-10: the **BlockFrame** look; **no script, snap scroll** navigation; **embed two open fonts**.

This plan follows `docs/superpowers/plans/2026-10-09-readable-refine-report.md`, which is built on this branch: the report's fields, limits, refusals, the "what will be written" block and the report notes all stay as they are.

## Global Constraints

- **Unchanged:** the report fields, `LIMITS`, the refusals, `CSP`, `MERMAID_URL`, `MERMAID_SRI`, the report notes and the one-report-per-ask rule. No script runs but Mermaid.
- **Slide order:**
  1. **Refine report**: the cover, cream.
  2. **At a glance**: yellow.
  3. **Needs your eye**: pink, only when `needs_your_eye` is non-empty.
  4. **Words used here**: blue, only when `terms` is non-empty.
  5. One slide per part, `Part N of M`, alternating paper and blue.
  6. **Files**: paper, only when `files` is non-empty.
  7. **Done when**: green.
  8. **Your decision**: ink, the last slide.
- **Slide ids**, in that order: `cover`, `glance`, `eye`, `terms`, `part-1` … `part-M`, `files`, `checks`, `decision`.
- **Every slide has:**
  - a white pill naming its kind;
  - a counter `NN / NN`, zero-padded;
  - one card: off-white, a 4px black frame and a hard 10px shadow;
  - a link `Next: <next slide's name> ↓` on every slide but the last.
- **The rail:** a fixed column of numbered links on the right, one per slide, hidden below 64rem.
- **Navigation is CSS only:** `scroll-snap-type: y mandatory` on `html`, and `scroll-snap-align: start` with `min-height: 100svh` on each slide.
- **The next step** (`Back in the terminal: choose <strong>Write it to the task</strong> to save this plan, or type what to change.`) is on the cover and on **Your decision**.
- **Your decision** shows "Exactly what will be written", taken from the plan the person was shown, never from the report's fields.
- **Palette (BlockFrame):**
  - black `#000000`, white `#FFFFFF`, off-white `#FFFDF5`;
  - pink `#FE90E8`, blue `#C0F7FE`, green `#99E885`, yellow `#F7CB46`, cream `#FFDC8B`, ink `#111111`.
  - Change marks: `new` green, `change` yellow, `remove` pink with a dashed border, always with a text label. The badges and the Mermaid `classDef`s use the same fills.
- **Type:**
  - Headings in "Report Display" (Space Grotesk), sentence case, weight 700, letter-spacing −0.02em.
  - Body in "Report Body" (Inter) at weight 500, `clamp(18px, 0.9rem + 0.45vw, 22px)`, line height 1.55.
  - Text inside a card at most 62ch wide.
  - No italics. Capitals only in the pills, the table headers and the badges, never in the report's own words.
- **Fonts:** `@fontsource-variable/space-grotesk@5.3.0` `files/space-grotesk-latin-wght-normal.woff2`, sha256 `0640890476fc1198ab4de571fb658de443c4d85b66466ec09534a8737ab1ce9d`, and `@fontsource-variable/inter@5.3.0` `files/inter-latin-wght-normal.woff2`, sha256 `3100e775e8616cd2611beecfa23a4263d7037586789b43f035236a2e6fbd4c62`. Both are SIL OFL 1.1, and their licence texts ship beside them. If the fonts cannot be read, the report is still written, in system fonts.
- **Light only.** BlockFrame is a light system.
- **Commits:** Conventional Commits, scope `refine`, subject ≤ 72 characters, each ending with a `Co-Authored-By:` trailer naming the model that wrote it.

## Why these choices

- **Slides.** The user asked for the report to be "broken up a bit". One idea per screen is Mayer's segmenting principle, and W3C's cognitive-accessibility guide asks for content in small chunks that a distracted reader can come back to. The `NN / NN` counter and the rail are the "where am I" signposts that guide recommends.
- **Snap scroll, no script.** The page's policy allows no script but pinned Mermaid; the user chose to keep it that way. CSS scroll snap moves one slide per Page Down, Space, ↓ or wheel step in every current browser, and needs nothing to run.
- **BlockFrame.** Chosen by the user. Thick frames and a ground colour per kind of slide make each chunk unmistakable. Its all-caps display headings are dropped: capitals are harder to read (UK Home Office accessibility posters).
- **Embedded fonts.** The policy blocks Google Fonts, and only JetBrains Mono is installed here. Variable latin subsets keep both fonts to about 70 KB, or about 94 KB as base64.

## File Structure

- `claude/refine-mod/fonts/` (new): the two woff2 files, `OFL-Inter.txt`, `OFL-SpaceGrotesk.txt` and `README.md` (source, version, sha256).
- `claude/refine-mod/hooks/style.ts` (new): `Fonts`, `FONT_FILES` and `slideCss(fonts?)`, the BlockFrame stylesheet and the `@font-face` rules.
- `claude/refine-mod/hooks/page.ts` (rewrite): the same exports, with `reportPage(report, plan, fonts?)` rendering slides.
- `claude/refine-mod/hooks/refine.ts` (modify): read the fonts once and pass them to `reportPage`.
- Tests: `claude/refine-mod/tests/page.test.ts` and `claude/refine-mod/tests/refine.test.ts` (modify).
- Docs: `claude/refine-mod/tests/example-report.ts`, `.claude/skills/refine-task/report-catalogue.md` and `CONTEXT.md` (modify).

---

### Task 1: Slides in the BlockFrame style

**Files:**
- Create: `claude/refine-mod/fonts/inter-latin-wght-normal.woff2`, `claude/refine-mod/fonts/space-grotesk-latin-wght-normal.woff2`, `claude/refine-mod/fonts/OFL-Inter.txt`, `claude/refine-mod/fonts/OFL-SpaceGrotesk.txt`, `claude/refine-mod/fonts/README.md`
- Create: `claude/refine-mod/hooks/style.ts`
- Rewrite: `claude/refine-mod/hooks/page.ts`
- Modify: `claude/refine-mod/tests/page.test.ts`
- Modify: `claude/refine-mod/tests/refine.test.ts` (one slice in the report-then-approve test)

**Interfaces:**
- Consumes: `Report`, `Section`, `Diagram`, `FileChange`, `Check`, `Term` and `Change` from `./report`; `Plan` and `WRITE` from `./plan`.
- Produces:
  - from `hooks/style.ts`: `type Fonts = { display: string; body: string }` (standard base64 of each woff2), `FONT_FILES = { display: 'space-grotesk-latin-wght-normal.woff2', body: 'inter-latin-wght-normal.woff2' }` and `slideCss(fonts?: Fonts): string`;
  - from `hooks/page.ts`: `reportPage(report: Report, plan: Plan, fonts?: Fonts): string`, plus `MERMAID_URL`, `MERMAID_SRI`, `CSP`, `inline` and `readingMinutes`, all unchanged.

- [ ] **Step 1: Ship the fonts**

```bash
F=claude/refine-mod/fonts
mkdir -p "$F"
curl -fsSL -o "$F/inter-latin-wght-normal.woff2" \
  https://cdn.jsdelivr.net/npm/@fontsource-variable/inter@5.3.0/files/inter-latin-wght-normal.woff2
curl -fsSL -o "$F/space-grotesk-latin-wght-normal.woff2" \
  https://cdn.jsdelivr.net/npm/@fontsource-variable/space-grotesk@5.3.0/files/space-grotesk-latin-wght-normal.woff2
sha256sum -c <<EOF
3100e775e8616cd2611beecfa23a4263d7037586789b43f035236a2e6fbd4c62  $F/inter-latin-wght-normal.woff2
0640890476fc1198ab4de571fb658de443c4d85b66466ec09534a8737ab1ce9d  $F/space-grotesk-latin-wght-normal.woff2
EOF
curl -fsSL -o "$F/OFL-Inter.txt" https://cdn.jsdelivr.net/npm/@fontsource-variable/inter@5.3.0/LICENSE
curl -fsSL -o "$F/OFL-SpaceGrotesk.txt" https://cdn.jsdelivr.net/npm/@fontsource-variable/space-grotesk@5.3.0/LICENSE
head -3 "$F/OFL-Inter.txt" "$F/OFL-SpaceGrotesk.txt"
```

Expected: both lines `OK`, and each licence opens with its copyright line and "licensed under the SIL Open Font License, Version 1.1". Stop if a checksum fails.

Create `claude/refine-mod/fonts/README.md`:

```markdown
# Report fonts

The refine report embeds these two typefaces in each page it writes
(`hooks/style.ts`), since the page's policy loads no font from the web.

| File | Typeface | From | sha256 |
|---|---|---|---|
| `space-grotesk-latin-wght-normal.woff2` | Space Grotesk, variable 300–700, latin | `@fontsource-variable/space-grotesk@5.3.0` | `0640890476fc1198ab4de571fb658de443c4d85b66466ec09534a8737ab1ce9d` |
| `inter-latin-wght-normal.woff2` | Inter, variable 100–900, latin | `@fontsource-variable/inter@5.3.0` | `3100e775e8616cd2611beecfa23a4263d7037586789b43f035236a2e6fbd4c62` |

Both are under the SIL Open Font License 1.1: `OFL-SpaceGrotesk.txt`,
`OFL-Inter.txt`.
```

- [ ] **Step 2: Write the failing page tests**

In `claude/refine-mod/tests/page.test.ts`:

1. Under `const NEXT = …`, add:

```ts
// The ids of a page's slides, in order.
const slides = (page: string): string[] =>
  [...page.matchAll(/<section class="slide [^"]*" id="([^"]+)"/g)].map(m => m[1] ?? '')
```

2. Add to `describe('the head', …)`:

```ts
  test('embeds the fonts it is given, and none otherwise', () => {
    expect(reportPage(report(), PLAN)).not.toContain('@font-face')
    const page = reportPage(report(), PLAN, { display: 'RElTUA==', body: 'Qk9EWQ==' })
    expect(page).toContain('src: url(data:font/woff2;base64,RElTUA==) format("woff2")')
    expect(page).toContain('src: url(data:font/woff2;base64,Qk9EWQ==) format("woff2")')
  })

  test('snaps one slide to the screen, with nothing to run', () => {
    const page = reportPage(report(), PLAN)
    expect(page).toContain('scroll-snap-type: y mandatory')
    expect(page).toContain('scroll-snap-align: start')
    expect(page.split('<script').length - 1).toBe(1)
  })
```

3. Replace the whole `describe('the layout', …)` block and the whole `describe('what will be written', …)` block with:

```ts
describe('the slides', () => {
  test('come in a fixed order, the optional ones only when given', () => {
    expect(slides(reportPage(report(), PLAN))).toEqual(['cover', 'glance', 'part-1', 'files', 'checks', 'decision'])
    const full = reportPage(
      report({
        needs_your_eye: ['Offline: diagrams show as text.'],
        terms: [{ term: 'Refine', meaning: 'Turning a task into a plan.' }],
        sections: [
          { heading: 'One', points: ['a'] },
          { heading: 'Two', points: ['b'] },
        ],
      }),
      PLAN,
    )
    expect(slides(full)).toEqual(['cover', 'glance', 'eye', 'terms', 'part-1', 'part-2', 'files', 'checks', 'decision'])
    expect(slides(reportPage(report({ files: [] }), PLAN))).toEqual(['cover', 'glance', 'part-1', 'checks', 'decision'])
  })

  test('each says where it is, and links to the next by name', () => {
    const page = reportPage(report(), PLAN)
    expect(page).toContain('<span class="count">01 / 06</span>')
    expect(page).toContain('<span class="count">06 / 06</span>')
    expect(page).toContain('<a class="next" href="#glance">Next: At a glance ↓</a>')
    expect(page).toContain('<a class="next" href="#part-1">Next: What changes ↓</a>')
    expect(page).toContain('<a class="next" href="#decision">Next: Your decision ↓</a>')
    expect(page.split('<a class="next"').length - 1).toBe(5)
  })

  test('the rail links every slide', () => {
    const page = reportPage(report(), PLAN)
    expect(page).toContain('<nav class="rail" aria-label="Slides">')
    for (const id of slides(page)) expect(page).toContain(`<a href="#${id}"`)
  })

  test('a part has its own slide, numbered, its heading shown with code', () => {
    const page = reportPage(report({ sections: [{ heading: 'The `task` command', points: ['a'] }] }), PLAN)
    expect(page).toContain('<span class="pill">Part 1 of 1</span>')
    expect(page).toContain('<h2>The <code>task</code> command</h2>')
    expect(page).toContain('Next: The <code>task</code> command ↓')
  })

  test('the next step is said on the first slide and the last', () => {
    const page = reportPage(report(), PLAN)
    expect(page.split(NEXT).length - 1).toBe(2)
    expect(page.indexOf(NEXT)).toBeLessThan(page.indexOf('id="glance"'))
    expect(page.lastIndexOf(NEXT)).toBeGreaterThan(page.indexOf('id="decision"'))
  })

  test('the cover says how long it takes and how much there is', () => {
    expect(reportPage(report(), PLAN)).toContain('<p class="meta">About 1 min to read · 6 slides · 1 file</p>')
    expect(reportPage(report({ files: [] }), PLAN)).toContain('<p class="meta">About 1 min to read · 5 slides</p>')
  })

  test('the glance puts what changes beside what stays the same', () => {
    const page = reportPage(report({ unchanged: ['Approval: still asked.'] }), PLAN)
    expect(page).toContain('<p class="lede">The task gets &lt;new&gt; words.</p>')
    expect(page).toContain('<h3>What changes</h3>')
    expect(page).toContain('<h3>What stays the same</h3>')
    expect(reportPage(report(), PLAN)).not.toContain('What stays the same')
  })

  test('words used here are a definition list', () => {
    const page = reportPage(report({ terms: [{ term: 'Refine', meaning: 'Turning a task into a plan.' }] }), PLAN)
    expect(page).toContain('<dt>Refine</dt><dd>Turning a task into a plan.</dd>')
  })

  test('shows the files with a labelled badge each', () => {
    expect(reportPage(report(), PLAN)).toContain(
      '<tr><td><span class="badge change">changed</span></td><td><code>src/words.rs</code></td><td>Holds the words.</td></tr>',
    )
  })
})

describe('the decision slide', () => {
  test('repeats the plan, escaped, from the plan the person was shown', () => {
    const page = reportPage(report({ summary: 'Something else entirely.' }), PLAN)
    const decision = page.slice(page.indexOf('id="decision"'))
    expect(decision).toContain('<h3>Exactly what will be written</h3>')
    expect(decision).toContain('<p><strong>Description:</strong> feat: &lt;A&gt; &amp; &quot;B&quot;</p>')
    expect(decision).toContain('<li>Goal: &lt;x&gt; &amp; y</li>')
    expect(decision).toContain('<li>Done when: `z` shows</li>')
    expect(decision).toContain('<p class="muted">The tool also adds a note linking this report.</p>')
  })

  test('says so when the plan has no notes', () => {
    expect(reportPage(report(), { ...PLAN, notes: [] })).toContain('<p>No notes.</p>')
  })
})
```

4. In `describe('diagrams', …)`:
   - In `escape Mermaid source, and give flowcharts and state diagrams the change colours` and `find the kind of diagram past comments, directives and front matter`, replace every `classDef new fill:#dcfce7` with `classDef new fill:#99E885`, and `classDef change fill:#fef3c7` with `classDef change fill:#F7CB46`.
   - Append:

```ts
  test('sit beside the points, and a part without one has its points alone', () => {
    expect(part({ mermaid: 'flowchart LR\n a --> b' })).toContain('<div class="split">')
    const bare = reportPage(report(), PLAN)
    expect(bare).not.toContain('<div class="split">')
    expect(bare).toContain('<div class="points solo">')
  })

  test('carry the legend under the drawing, only where the change colours apply', () => {
    const flow = part({ mermaid: 'flowchart LR\n a --> b' })
    expect(flow.indexOf('<p class="legend">')).toBeGreaterThan(flow.indexOf('<pre class="mermaid">'))
    expect(part({ mermaid: 'sequenceDiagram\n A->>B: hi' })).not.toContain('<p class="legend">')
    expect(part({ svg: '<svg viewBox="0 0 1 1"><rect/></svg>' })).not.toContain('<p class="legend">')
  })
```

Keep every other test in the file as it is.

5. In `claude/refine-mod/tests/refine.test.ts`, in the test `report, then approve: …`, replace

```ts
  const block = page.slice(page.indexOf('<h2>Exactly what will be written</h2>'), page.indexOf('<section id="files">'))
```

with

```ts
  const block = page.slice(page.indexOf('id="decision"'))
```

and in the lines after it, replace any assertion that looks for `<h2>Exactly what will be written</h2>` with one that looks for `<h3>Exactly what will be written</h3>`.

- [ ] **Step 3: Run them to verify they fail**

Run: `claude plugin test claude/refine-mod`
Expected: the new and replaced tests in `page.test.ts` fail: no slides, no `fonts` parameter, the old colours. Every other test still passes.

- [ ] **Step 4: Write style.ts**

Create `claude/refine-mod/hooks/style.ts`:

```ts
// How a report looks: BlockFrame-style slides. Pastel grounds, off-white
// cards in thick black frames with hard shadows, heavy sentence-case
// headings. Built for a reader who scans: big type, short lines, one framed
// block per idea, a colour per kind of slide, and colour always paired with a
// text label. Light only, like BlockFrame. No `$`.

// The two typefaces, as standard base64 of their woff2 files.
export type Fonts = { display: string; body: string }

// The files the mod ships in fonts/: Space Grotesk for headings and chrome,
// Inter for reading (both SIL OFL 1.1; see fonts/README.md).
export const FONT_FILES = {
  display: 'space-grotesk-latin-wght-normal.woff2',
  body: 'inter-latin-wght-normal.woff2',
} as const

const fontFaces = (fonts: Fonts): string => `
@font-face { font-family: "Report Display"; src: url(data:font/woff2;base64,${fonts.display}) format("woff2");
  font-weight: 300 700; font-display: block; }
@font-face { font-family: "Report Body"; src: url(data:font/woff2;base64,${fonts.body}) format("woff2");
  font-weight: 100 900; font-display: block; }
`

const CSS = `
:root { color-scheme: light;
  --black: #000000; --white: #FFFFFF; --paper: #FFFDF5; --ink: #111111; --muted: #3D3D3D;
  --pink: #FE90E8; --blue: #C0F7FE; --green: #99E885; --yellow: #F7CB46; --cream: #FFDC8B;
  --display: "Report Display", "Space Grotesk", system-ui, sans-serif;
  --body: "Report Body", "Inter", system-ui, sans-serif;
  --mono: "JetBrains Mono", "JetBrainsMono NF", ui-monospace, monospace; }
html { scroll-snap-type: y mandatory; scroll-behavior: smooth; }
@media (prefers-reduced-motion: reduce) { html { scroll-behavior: auto; } }
body { margin: 0; background: var(--paper); color: var(--black);
  font: 500 clamp(18px, 0.9rem + 0.45vw, 22px)/1.55 var(--body); }
em, i { font-style: normal; font-weight: 700; }
.slide { box-sizing: border-box; min-height: 100vh; min-height: 100svh; scroll-snap-align: start;
  display: flex; flex-direction: column; gap: 1.25rem;
  padding: clamp(1rem, 3vw, 2.5rem) clamp(1rem, 5vw, 5rem) clamp(1rem, 2.5vw, 2rem); }
.cream { background: var(--cream); }
.yellow { background: var(--yellow); }
.pink { background: var(--pink); }
.blue { background: var(--blue); }
.green { background: var(--green); }
.paper { background: var(--paper); }
.ink { background: var(--ink); }
.bar { display: flex; justify-content: space-between; align-items: center; gap: 1rem; }
.pill, .count { background: var(--white); border: 3px solid var(--black); color: var(--black); }
.pill { font: 700 0.8rem/1 var(--display); letter-spacing: 0.08em; text-transform: uppercase;
  box-shadow: 4px 4px 0 var(--black); padding: 0.5rem 0.8rem; }
.count { font: 600 0.9rem/1 var(--mono); padding: 0.45rem 0.7rem; }
.card { box-sizing: border-box; width: 100%; max-width: 68rem; margin: auto; background: var(--paper);
  border: 4px solid var(--black); box-shadow: 10px 10px 0 var(--black); padding: clamp(1.25rem, 3vw, 3rem); }
.ink .card { box-shadow: 10px 10px 0 var(--yellow); }
.card > * { max-width: 62ch; }
.card > .split, .card > table, .card > details, .card > figure { max-width: none; }
h1, h2, h3 { font-family: var(--display); font-weight: 700; letter-spacing: -0.02em; line-height: 1.1;
  margin: 0 0 0.75em; }
h1 { font-size: clamp(2.2rem, 1.4rem + 3.5vw, 4.5rem); }
h2 { font-size: clamp(1.6rem, 1.1rem + 2vw, 2.8rem); }
h3 { font-size: clamp(1.15rem, 1rem + 0.6vw, 1.5rem); margin-top: 1.25em; }
.cols h3, .split h3 { margin-top: 0; }
p, ul, ol, dl, table, figure, details { margin: 0 0 1.1em; }
.kicker { font: 700 0.85rem/1 var(--display); letter-spacing: 0.08em; text-transform: uppercase; }
.lede { font-size: 1.25em; font-weight: 600; }
.meta { font: 600 0.95rem/1.4 var(--mono); }
.hint, .muted { color: var(--muted); }
ul { list-style: none; padding: 0; }
ul > li { position: relative; padding-left: 1.6em; }
ul > li::before { content: ""; position: absolute; left: 0; top: 0.45em; width: 0.7em; height: 0.7em;
  background: var(--black); }
li + li { margin-top: 0.6em; }
ol { padding-left: 1.4em; }
.cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr)); gap: 0 2.5rem; max-width: none; }
.split { display: grid; grid-template-columns: minmax(0, 3fr) minmax(16rem, 2fr); gap: 2rem; align-items: start; }
@media (max-width: 56rem) { .split { grid-template-columns: 1fr; } }
.solo { font-size: 1.15em; }
figure { background: var(--white); border: 3px solid var(--black); padding: 1rem; overflow-x: auto; margin: 0; }
figcaption.look { font-weight: 600; margin: 0 0 0.75rem; }
pre.mermaid { margin: 0; text-align: center; font-family: inherit; }
.legend { font-size: 0.85rem; margin: 0.75rem 0 0; }
svg { max-width: 100%; height: auto; }
code, pre { font-family: var(--mono); font-size: 0.88em; }
code { background: var(--cream); border: 1.5px solid var(--black); padding: 0.05em 0.3em; overflow-wrap: anywhere; }
table { width: 100%; border-collapse: collapse; display: block; overflow-x: auto; }
th, td { text-align: left; vertical-align: top; padding: 0.55rem 0.9rem 0.55rem 0; border-top: 2px solid var(--black); }
th { font: 700 0.8rem/1.2 var(--display); letter-spacing: 0.06em; text-transform: uppercase; border-top: 0; }
dt { font-family: var(--display); font-weight: 700; font-size: 1.1em; }
dd { margin: 0 0 1em; }
.badge { display: inline-block; font: 700 0.8rem/1.3 var(--display); letter-spacing: 0.04em; text-transform: uppercase;
  padding: 0.1rem 0.55rem; border: 2px solid var(--black); background: var(--white); white-space: nowrap; }
.badge.new { background: var(--green); }
.badge.change { background: var(--yellow); }
.badge.remove { background: var(--pink); border-style: dashed; }
details { border: 3px solid var(--black); background: var(--white); padding: 0.6rem 1rem; }
summary { cursor: pointer; font-weight: 700; }
.next-step { font-weight: 700; font-size: 1.05em; background: var(--yellow); border: 3px solid var(--black);
  padding: 0.8rem 1rem; margin: 1.25rem 0 0; }
a.next { align-self: flex-end; font: 700 1rem/1.2 var(--display); color: var(--black); background: var(--white);
  border: 3px solid var(--black); box-shadow: 4px 4px 0 var(--black); padding: 0.6rem 0.9rem; text-decoration: none; }
a.next:hover { background: var(--yellow); }
a:focus-visible { outline: 4px solid var(--blue); outline-offset: 3px; }
.rail { position: fixed; right: 0.75rem; top: 50%; transform: translateY(-50%); z-index: 2; }
.rail ol { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.35rem; }
.rail a { display: block; font: 600 0.7rem/1 var(--mono); color: var(--black); background: var(--white);
  border: 2px solid var(--black); padding: 0.3rem 0.35rem; text-decoration: none; }
.rail a:hover { background: var(--yellow); }
@media (max-width: 64rem) { .rail { display: none; } .card { box-shadow: 6px 6px 0 var(--black); }
  .ink .card { box-shadow: 6px 6px 0 var(--yellow); } }
@media print { html { scroll-snap-type: none; } .slide { min-height: auto; break-after: page; }
  .rail, a.next { display: none; } }
`

// The stylesheet, with the typefaces when the mod could read them; without,
// the page falls back to the system's fonts.
export const slideCss = (fonts?: Fonts): string => (fonts === undefined ? '' : fontFaces(fonts)) + CSS
```

- [ ] **Step 5: Rewrite page.ts**

Replace the whole of `claude/refine-mod/hooks/page.ts`. Keep these unchanged, copying them from the current file:
- the `MERMAID_URL`, `MERMAID_SRI` and `CSP` exports and the comments above them;
- `LABELS`, `escapeHtml`, `plain`, `inline` and `readingMinutes`, with their comments;
- `count`, `badge` and `bullets`;
- `takesMarks` and `withMarks`, with their comments.

Change the file comment, the imports, `MARKS`, `NEXT` and `LEGEND`, drop `BASE_CSS`, `contents`, `glance`, `terms`, `figure`, `part`, `written`, `files` and `checks`, and add the rest, so that the file reads:

```ts
// The page a report is written as: the mod's own head, whose policy lets the
// report run no script, then the report's fields as slides in one fixed
// order: what it is and what to decide first, a slide per idea, the decision
// last. A slide fills the screen, and the page snaps from one to the next as
// it scrolls, so moving through it needs no script. No `$`, so the unit tests
// reach it directly.

import { WRITE } from './plan'
import type { Plan } from './plan'
import type { Change, Check, Diagram, FileChange, Report, Section, Term } from './report'
import { slideCss } from './style'
import type { Fonts } from './style'

// …MERMAID_URL, MERMAID_SRI and CSP, unchanged…

// The colours `new`, `change` and `remove` take in a flowchart or a state
// diagram, the same fills as the badges: a diagram marks what the plan
// changes with `class <node> new` or `:::new`.
const MARKS = [
  'classDef new fill:#99E885,stroke:#000000,stroke-width:3px,color:#000000',
  'classDef change fill:#F7CB46,stroke:#000000,stroke-width:3px,color:#000000',
  'classDef remove fill:#FE90E8,stroke:#000000,stroke-width:3px,stroke-dasharray:6 4,color:#000000',
].join('\n')

// …LABELS, escapeHtml, plain, inline, readingMinutes, count, badge, bullets,
// unchanged…

const pad = (n: number): string => String(n).padStart(2, '0')

// What to do with the report, said on the first slide and again on the last.
const NEXT =
  `<p class="next-step">Back in the terminal: choose <strong>${escapeHtml(WRITE)}</strong> ` +
  'to save this plan, or type what to change.</p>'

// What the colours mean, under a drawing that takes them.
const LEGEND = `<p class="legend">${badge('new')} ${badge('change')} ${badge('remove')} Everything else is unchanged.</p>`

// …takesMarks and withMarks, unchanged…

const figure = (diagram: Diagram, lookAt: string): string => {
  const marked = 'mermaid' in diagram && takesMarks(diagram.mermaid)
  const drawing =
    'mermaid' in diagram ? `<pre class="mermaid">${escapeHtml(withMarks(diagram.mermaid))}</pre>` : diagram.svg
  return [
    '<figure>',
    `<figcaption class="look"><strong>Look at:</strong> ${plain(lookAt)}</figcaption>`,
    drawing,
    marked ? LEGEND : '',
    '</figure>',
  ]
    .filter(Boolean)
    .join('\n')
}

// A ground colour per kind of slide, so each chunk is told apart at a glance.
type Ground = 'cream' | 'yellow' | 'pink' | 'blue' | 'green' | 'paper' | 'ink'

// A slide: its id, the kind its pill names, the name the slide before links
// to it by, its ground, and its card's content.
type Slide = { id: string; pill: string; name: string; ground: Ground; body: string }

const cover = (report: Report, meta: string): string =>
  [
    '<p class="kicker">Refine report</p>',
    `<h1>${plain(report.title)}</h1>`,
    `<p class="meta">${meta}</p>`,
    NEXT,
    '<p class="hint">Scroll, or press Page Down, to go through it one slide at a time.</p>',
  ].join('\n')

const glance = (report: Report): string =>
  [
    `<p class="lede">${plain(report.summary)}</p>`,
    '<div class="cols">',
    `<div>\n<h3>What changes</h3>\n${bullets(report.changes)}\n</div>`,
    report.unchanged.length > 0 ? `<div>\n<h3>What stays the same</h3>\n${bullets(report.unchanged)}\n</div>` : '',
    '</div>',
  ]
    .filter(Boolean)
    .join('\n')

const eye = (lines: readonly string[]): string => `<h2>Where your judgement is needed</h2>\n${bullets(lines)}`

const terms = (list: readonly Term[]): string =>
  `<h2>Words used here</h2>\n<dl>${list.map(t => `<dt>${plain(t.term)}</dt><dd>${inline(t.meaning)}</dd>`).join('')}</dl>`

const part = (s: Section): string => {
  const points = bullets(s.points)
  return [
    `<h2>${plain(s.heading)}</h2>`,
    s.diagram !== undefined
      ? `<div class="split">\n${figure(s.diagram, s.look_at ?? '')}\n<div class="points">${points}</div>\n</div>`
      : `<div class="points solo">${points}</div>`,
    s.detail !== undefined ? `<details>\n<summary>More detail</summary>\n${s.detail}\n</details>` : '',
  ]
    .filter(Boolean)
    .join('\n')
}

const files = (list: readonly FileChange[]): string => {
  const rows = list
    .map(f => `<tr><td>${badge(f.change)}</td><td><code>${escapeHtml(f.path)}</code></td><td>${inline(f.why)}</td></tr>`)
    .join('')
  return [
    '<h2>Files</h2>',
    '<table>',
    '<thead><tr><th>Change</th><th>File</th><th>Why</th></tr></thead>',
    `<tbody>${rows}</tbody>`,
    '</table>',
  ].join('\n')
}

const checks = (list: readonly Check[]): string => {
  const rows = list.map(c => `<tr><td>${inline(c.check)}</td><td>${inline(c.how)}</td></tr>`).join('')
  return [
    '<h2>Done when</h2>',
    '<table>',
    '<thead><tr><th>Check</th><th>How it is checked</th></tr></thead>',
    `<tbody>${rows}</tbody>`,
    '</table>',
  ].join('\n')
}

// The description and notes as the tool will write them, from the plan the
// person was shown, never from the report's fields: what they approve is
// what they read here, on the slide where they decide.
const decision = (plan: Plan): string => {
  const notes =
    plan.notes.length === 0
      ? '<p>No notes.</p>'
      : `<ol>${plan.notes.map(n => `<li>${escapeHtml(n)}</li>`).join('')}</ol>`
  return [
    '<h2>Your decision</h2>',
    '<h3>Exactly what will be written</h3>',
    `<p><strong>Description:</strong> ${escapeHtml(plan.description)}</p>`,
    notes,
    '<p class="muted">The tool also adds a note linking this report.</p>',
    NEXT,
  ].join('\n')
}

// One slide on the screen: what kind it is and where, its card, and the way
// on to the next, named.
const frame = (slide: Slide, i: number, all: readonly Slide[]): string => {
  const next = all[i + 1]
  return [
    `<section class="slide ${slide.ground}" id="${slide.id}" aria-label="Slide ${i + 1} of ${all.length}">`,
    '<div class="bar">',
    `<span class="pill">${escapeHtml(slide.pill)}</span>`,
    `<span class="count">${pad(i + 1)} / ${pad(all.length)}</span>`,
    '</div>',
    `<div class="card">\n${slide.body}\n</div>`,
    next !== undefined ? `<a class="next" href="#${next.id}">Next: ${plain(next.name)} ↓</a>` : '',
    '</section>',
  ]
    .filter(Boolean)
    .join('\n')
}

// Every slide's number, pinned to the side: where you are, and a way back.
const rail = (all: readonly Slide[]): string => {
  const items = all
    .map((s, i) => `<li><a href="#${s.id}" title="${escapeHtml(s.name)}">${pad(i + 1)}</a></li>`)
    .join('')
  return `<nav class="rail" aria-label="Slides"><ol>${items}</ol></nav>`
}

// The whole page: the policy before anything it governs, then the title, the
// stylesheet and Mermaid, then the slides in their fixed order.
export const reportPage = (report: Report, plan: Plan, fonts?: Fonts): string => {
  const parts = report.sections
  const touched = report.files.length > 0
  const rest: Slide[] = [
    { id: 'glance', pill: 'At a glance', name: 'At a glance', ground: 'yellow', body: glance(report) },
    ...(report.needs_your_eye.length > 0
      ? [{ id: 'eye', pill: 'Needs your eye', name: 'Needs your eye', ground: 'pink', body: eye(report.needs_your_eye) } as Slide]
      : []),
    ...(report.terms.length > 0
      ? [{ id: 'terms', pill: 'Words used here', name: 'Words used here', ground: 'blue', body: terms(report.terms) } as Slide]
      : []),
    ...parts.map(
      (s, i): Slide => ({
        id: `part-${i + 1}`,
        pill: `Part ${i + 1} of ${parts.length}`,
        name: s.heading,
        ground: i % 2 === 0 ? 'paper' : 'blue',
        body: part(s),
      }),
    ),
    ...(touched ? [{ id: 'files', pill: 'Files', name: 'Files', ground: 'paper', body: files(report.files) } as Slide] : []),
    { id: 'checks', pill: 'Done when', name: 'Done when', ground: 'green', body: checks(report.checks) },
    { id: 'decision', pill: 'Your decision', name: 'Your decision', ground: 'ink', body: decision(plan) },
  ]
  const total = rest.length + 1
  const meta =
    `About ${readingMinutes(report)} min to read · ${count(total, 'slide')}` +
    (touched ? ` · ${count(report.files.length, 'file')}` : '')
  const all: Slide[] = [{ id: 'cover', pill: 'Refine report', name: 'Start', ground: 'cream', body: cover(report, meta) }, ...rest]
  return [
    '<!doctype html>',
    '<html lang="en">',
    '<head>',
    `<meta http-equiv="Content-Security-Policy" content="${CSP}">`,
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${escapeHtml(report.title)}</title>`,
    `<style>${slideCss(fonts)}</style>`,
    `<script src="${MERMAID_URL}" integrity="${MERMAID_SRI}" crossorigin="anonymous"></script>`,
    '</head>',
    '<body>',
    rail(all),
    '<main class="deck">',
    ...all.map((s, i) => frame(s, i, all)),
    '</main>',
    '</body>',
    '</html>',
  ].join('\n') + '\n'
}
```

The `…unchanged…` comments mark where the kept code goes. Do not leave the comments themselves in the file. If `noUncheckedIndexedAccess` or the `as Slide` casts upset the type-check, give each optional slide a typed `const` first; the output must stay the same.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `claude plugin test claude/refine-mod`
Expected: every test passes.

- [ ] **Step 7: Look at it**

Render the worked example with the shipped fonts, then screenshot each slide by loading the page at its anchor:

```bash
S="$(mktemp -d)"
OUT="$S/example.html" bun -e '
import { readFileSync } from "node:fs"
import { EXAMPLE, EXAMPLE_PLAN } from "./claude/refine-mod/tests/example-report.ts"
import { parseReport } from "./claude/refine-mod/hooks/report.ts"
import { reportPage } from "./claude/refine-mod/hooks/page.ts"
import { FONT_FILES } from "./claude/refine-mod/hooks/style.ts"
const font = (f) => readFileSync(`./claude/refine-mod/fonts/${f}`).toString("base64")
const report = parseReport(EXAMPLE)
if (typeof report === "string") throw new Error(report)
await Bun.write(process.env.OUT, reportPage(report, EXAMPLE_PLAN, { display: font(FONT_FILES.display), body: font(FONT_FILES.body) }))
'
for id in cover glance eye terms part-1 part-2 part-3 files checks decision; do
  timeout 60 google-chrome --headless=new --disable-gpu --hide-scrollbars --window-size=1600,900 \
    --virtual-time-budget=8000 --screenshot="$S/wide-$id.png" "file://$S/example.html#$id" >/dev/null 2>&1
done
for id in cover glance part-1 decision; do
  timeout 60 google-chrome --headless=new --disable-gpu --hide-scrollbars --window-size=390,844 \
    --virtual-time-budget=8000 --screenshot="$S/phone-$id.png" "file://$S/example.html#$id" >/dev/null 2>&1
done
timeout 60 google-chrome --headless=new --disable-gpu --virtual-time-budget=10000 --dump-dom "file://$S/example.html" > "$S/out.html"
grep -c '<svg id="mermaid' "$S/out.html"; grep -c 'Syntax error' "$S/out.html"
echo "$S"
```

Expected: `2`, then `0`. Then read every PNG with the Read tool and check each:
- the slide's pill, counter and card are visible, and the card's text is not clipped;
- the headings are in Space Grotesk (geometric, a single-storey `a`) and the body text in Inter, not a system serif or the default sans;
- both diagrams are drawn, and the flowchart's new boxes are green, with the legend under it;
- on the phone shots, nothing runs off the right edge.

Fix the CSS for anything that fails, re-run, and look again. Copy the PNGs you judged on into the report folder under `.superpowers/sdd/` as evidence. Then `rm -r "$S"`.

- [ ] **Step 8: Validate, type-check and commit**

Run: `claude plugin validate claude/refine-mod`. Expected: `✔ Validation passed`.

```bash
D="$(dirname "$(ls -t /tmp/claude-1000/bundled-skills/*/*/plugin-authoring/types/claude-code.d.ts | head -1)")"
S="$(mktemp -d)"; M="$PWD/claude/refine-mod"
cat > "$S/tsconfig.json" <<EOF
{ "compilerOptions": { "target": "es2023", "lib": ["es2023"], "types": [], "module": "esnext",
  "moduleResolution": "bundler", "strict": true, "noUncheckedIndexedAccess": true, "noEmit": true, "skipLibCheck": true },
  "include": ["$D/claude-code.d.ts", "$M/hooks", "$M/tests"] }
EOF
(cd "$S" && npx -y -p typescript@5.6.3 tsc -p tsconfig.json); echo "tsc exit $?"; rm -r "$S"
```

Expected: no output, then `tsc exit 0`.

```bash
git add claude/refine-mod/fonts claude/refine-mod/hooks/style.ts claude/refine-mod/hooks/page.ts \
  claude/refine-mod/tests/page.test.ts claude/refine-mod/tests/refine.test.ts
git commit -m "$(cat <<'EOF'
feat(refine): show the report as slides, one idea to a screen

The report becomes BlockFrame-style slides: a framed card per idea on a
ground colour per kind, a counter and a named Next link on each, and
the decision last. The page snaps a slide at a time as it scrolls, so
it still runs no script but Mermaid. Space Grotesk and Inter ship in
fonts/ under the OFL.

Co-Authored-By: <the model that wrote it> <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Embed the shipped fonts in every report

**Files:**
- Modify: `claude/refine-mod/hooks/refine.ts`
- Test: `claude/refine-mod/tests/refine.test.ts`

**Interfaces:**
- Consumes: `FONT_FILES` and `Fonts` from `./style`; `reportPage(report, plan, fonts?)` from `./page` (Task 1).
- Produces: nothing later tasks use.

- [ ] **Step 1: Write the failing tests**

In `claude/refine-mod/tests/refine.test.ts`:

1. Give `engine` a fifth parameter and an `fs.read` stand-in. Change its signature to

```ts
// `writeFails`, when given, is what `$.fs.write` is refused with. With
// `fonts`, `$.fs.read` answers each font file with a short stand-in; without,
// it is refused, as when the mod's fonts are missing.
const engine = (
  on: On,
  settings: Record<string, unknown>,
  answer: Answers = WRITE,
  writeFails?: string,
  fonts = false,
) => {
```

and add `const reads: string[] = []` beside `writes`. Add this hook after the `fs.write` hook:

```ts
  on('fs.read', ($, e) => {
    reads.push(e.path)
    if (!fonts) return { deny: 'no fonts here' }
    return { value: { base64: e.path.endsWith('space-grotesk-latin-wght-normal.woff2') ? 'RElTUA==' : 'Qk9EWQ==' } }
  })
```

Return `reads` from `engine` with the rest (`return { registered, seen, writes, reads }`). In `world`, accept and pass on a fifth parameter `fonts = false` (`engine(on, settings, answer, undefined, fonts)`), and return `reads` too.

2. Append:

```ts
test('a report embeds the fonts the mod ships, read once a session', { options: REPORTS }, async ($, on) => {
  const { reads, writes } = world(on, SANDBOX, [REPORT, REPORT], [TASK], true)
  await $.session.start(START)
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  await $.tool.call(CALL)
  await $.tool.call({ tool: REPORT_TOOL, ...MINIMAL })
  expect(writes).toHaveLength(2)
  for (const page of writes.map(w => w.text)) {
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
```

The fifth argument to `world` is the `exports` list already in its signature (`[TASK]`); check the order of `world`'s parameters before you add `fonts` after them.

If the `fs.read` hook's `e` is typed differently from `{ path, as }` in the declaration file (grep `'fs.read'`), follow the declaration.

- [ ] **Step 2: Run them to verify they fail**

Run: `claude plugin test claude/refine-mod`
Expected: `a report embeds the fonts …` fails (no `@font-face`, no reads). The rest pass.

- [ ] **Step 3: Read the fonts in refine.ts**

In `claude/refine-mod/hooks/refine.ts`, add the imports:

```ts
import { FONT_FILES } from './style'
import type { Fonts } from './style'
```

Inside `register`, after `const made: MadeReport[] = []`, add:

```ts
  // The report's two typefaces, shipped in the mod's fonts/ folder, read on
  // the first report and kept. If they cannot be read, the report is still
  // written, in the system's fonts.
  let fonts: Fonts | undefined
  const readFonts = async ($: EngineInterface): Promise<Fonts | undefined> => {
    if (fonts !== undefined) return fonts
    try {
      const display = await $.fs.read(`${$.plugin.root}/fonts/${FONT_FILES.display}`, { as: 'bytes' })
      const body = await $.fs.read(`${$.plugin.root}/fonts/${FONT_FILES.body}`, { as: 'bytes' })
      fonts = { display: display.base64, body: body.base64 }
      return fonts
    } catch {
      return undefined
    }
  }
```

In the `show_task_report` hook, change `await $.fs.write(path, reportPage(report, shown))` to:

```ts
    await $.fs.write(path, reportPage(report, shown, await readFonts($)))
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `claude plugin test claude/refine-mod`
Expected: every test passes.

- [ ] **Step 5: Validate and commit**

Run `claude plugin validate claude/refine-mod` (expect `✔ Validation passed`), and the type-check from Task 1 Step 8 (expect `tsc exit 0`).

```bash
git add claude/refine-mod/hooks/refine.ts claude/refine-mod/tests/refine.test.ts
git commit -m "$(cat <<'EOF'
feat(refine): embed the shipped fonts in every report

The mod reads Space Grotesk and Inter from its fonts/ folder on the
first report of a session and embeds them, since the page's policy
loads no font from the web. Without them the report is still written,
in the system's fonts.

Co-Authored-By: <the model that wrote it> <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Say it in the catalogue

**Files:**
- Modify: `claude/refine-mod/tests/example-report.ts`
- Modify: `.claude/skills/refine-task/report-catalogue.md`
- Modify: `CONTEXT.md`

**Interfaces:**
- Consumes: the slide order and colours from Task 1.
- Produces: nothing code depends on.

- [ ] **Step 1: Match the example's highlight to the palette**

In `claude/refine-mod/tests/example-report.ts`, change `'  rect rgb(220, 252, 231)',` to `'  rect rgb(153, 232, 133)',`, the BlockFrame green `#99E885`.

- [ ] **Step 2: Update the catalogue**

In `.claude/skills/refine-task/report-catalogue.md`:

1. Replace the opening paragraph, which begins "How to fill `show_task_report`" and ends "Your job is the words and the diagrams.", with:

```markdown
How to fill `show_task_report` when the user chooses **Show me a report
first**. The tool builds the report itself, as full-screen slides in one
fixed order, from the fields you give it:

1. the cover: the title, how long it takes, and what to do at the end;
2. **At a glance**: `summary`, `changes` and `unchanged`;
3. **Needs your eye**: `needs_your_eye`, when you give any;
4. **Words used here**: `terms`, when you give any;
5. one slide per part in `sections`;
6. **Files**: `files`, when the plan touches any;
7. **Done when**: `checks`;
8. **Your decision**: exactly what will be written, taken from the plan
   itself, and the next step.

Your job is the words and the diagrams. Each part is one screen: a heading,
at most one diagram, and a few points beside it. Write it so it reads
whole without scrolling.
```

2. Delete the paragraph that begins "The page also adds "Exactly what will be written" itself": the list above now says it.
3. In "Drawing diagrams", change "The page draws them green, amber and dashed red, the same as the file badges, and says so above the first part" to "The slides draw them green, yellow and dashed pink, the same as the file badges, with a legend under the diagram", and change `rect rgb(220, 252, 231)` to `rect rgb(153, 232, 133)`. Re-wrap the paragraph.
4. Regenerate the worked example from `EXAMPLE` and replace the JSON in the "Worked example" fence with it:

```bash
bun -e 'import { EXAMPLE } from "./claude/refine-mod/tests/example-report.ts"; console.log(JSON.stringify(EXAMPLE, null, 2))'
```

- [ ] **Step 3: Update the glossary**

In `CONTEXT.md`, in the **Refine report** entry, change `An HTML page explaining a refine's plan` to `HTML slides explaining a refine's plan`. Re-wrap the entry.

- [ ] **Step 4: Check and commit**

Run: `claude plugin validate claude/refine-mod && claude plugin test claude/refine-mod`. Expected: all pass (the example still parses).

```bash
git add claude/refine-mod/tests/example-report.ts .claude/skills/refine-task/report-catalogue.md CONTEXT.md
git commit -m "$(cat <<'EOF'
docs(refine): describe the report's slides in the catalogue

The catalogue lists the slides in their order and asks for each part
to fit one screen. The example's highlight takes the BlockFrame green.

Co-Authored-By: <the model that wrote it> <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Put the slides in front of the reader

This task needs the user.

- [ ] **Step 1: Render the example for the user**

```bash
OUT="$HOME/.local/share/niri-tasks/reviews/refine-example-$(date -u +%Y%m%d-%H%M%S).html" bun -e '
import { readFileSync } from "node:fs"
import { EXAMPLE, EXAMPLE_PLAN } from "./claude/refine-mod/tests/example-report.ts"
import { parseReport } from "./claude/refine-mod/hooks/report.ts"
import { reportPage } from "./claude/refine-mod/hooks/page.ts"
import { FONT_FILES } from "./claude/refine-mod/hooks/style.ts"
const font = (f) => readFileSync(`./claude/refine-mod/fonts/${f}`).toString("base64")
const report = parseReport(EXAMPLE)
if (typeof report === "string") throw new Error(report)
await Bun.write(process.env.OUT, reportPage(report, EXAMPLE_PLAN, { display: font(FONT_FILES.display), body: font(FONT_FILES.body) }))
console.log(process.env.OUT)
'
```

Open it with `sh -c 'xdg-open "$1" >/dev/null 2>&1 </dev/null &' sh <path>`. Ask the user to page through it with Page Down or the wheel. Ask what they noticed first, what they skipped, and what was hard, and make what they ask for.

- [ ] **Step 2: A live refine**

1. The user runs `./install.sh` from this worktree.
2. They file a throwaway task, Refine it, and at **Write this to the task?** choose **Show me a report first**.
3. Expected: the slides open in the browser. The question comes back, and **Write it to the task** writes the task with a last note `Report: <path>`.
4. Delete the throwaway task. Re-run `./install.sh` from the main checkout after the branch lands.
