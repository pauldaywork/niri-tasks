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

// A drawn diagram's colours and fonts are set here, from the stylesheet
// beautiful-mermaid puts in each SVG, which the mod removes (draw.ts): its
// colour variables, scoped to drawn diagrams, and its fonts, the page's
// own body face first.
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
.solo { font-size: 1.3em; }
.drawn svg { display: block; width: 100%; height: auto; max-height: 45vh; }
.drawn svg { --_text: var(--fg);
  --_text-sec: var(--muted, color-mix(in srgb, var(--fg) 60%, var(--bg)));
  --_text-muted: var(--muted, color-mix(in srgb, var(--fg) 40%, var(--bg)));
  --_text-faint: color-mix(in srgb, var(--fg) 25%, var(--bg));
  --_line: var(--line, color-mix(in srgb, var(--fg) 50%, var(--bg)));
  --_arrow: var(--accent, color-mix(in srgb, var(--fg) 85%, var(--bg)));
  --_node-fill: var(--surface, color-mix(in srgb, var(--fg) 3%, var(--bg)));
  --_node-stroke: var(--border, color-mix(in srgb, var(--fg) 20%, var(--bg)));
  --_group-fill: var(--bg);
  --_group-hdr: color-mix(in srgb, var(--fg) 5%, var(--bg));
  --_inner-stroke: color-mix(in srgb, var(--fg) 12%, var(--bg));
  --_key-badge: color-mix(in srgb, var(--fg) 10%, var(--bg)); }
.drawn svg text { font-family: 'Report Body', 'Inter', system-ui, sans-serif; }
.drawn svg .mono { font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', ui-monospace, monospace; }
@media (max-width: 56rem) { .drawn svg { min-width: 34rem; max-height: none; } }
.drawn g.block[data-type="rect"] > rect:first-of-type { fill: var(--green); stroke: var(--black); stroke-width: 1.5px; }
.drawn g.block[data-type="rect"] > rect:nth-of-type(2), .drawn g.block[data-type="rect"] > text { display: none; }
code.path { overflow-wrap: normal; word-break: normal; }
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
