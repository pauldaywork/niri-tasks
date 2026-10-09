# Refine report catalogue

How to fill `show_task_report` when the user chooses **Show me a report
first**. The tool builds the page itself, in one fixed layout, from the
fields you give it. Your job is the words and the diagrams.

## Who reads it, and what for

One person, deciding one thing: approve this plan, or say what to change.
Write for a reader who may have ADHD. They scan rather than read, hold about
four things in mind at once, lose their place when interrupted, and give up
on walls of text. What helps them helps every reader:

- **Conclusion first.** The top of the page answers "what does this do, and
  what am I deciding?" before any detail.
- **Short and plain.** One idea per line, everyday words, no idioms, no
  double negatives. Say "the panel", not `src/panel/surface.rs`, except in
  `files`.
- **Point at what matters.** Each diagram says what to look at, and
  `needs_your_eye` says where the reader's judgement is needed.
- **Pictures with words, not instead of them.** Every diagram has points
  beside it.
- **The same shape every time.** The layout never changes, so the reader
  learns it once.

The tool enforces the limits below and refuses a report that breaks them,
listing every reason, so fix them all and call again.

## The fields

| Field | What it holds | Limit |
|---|---|---|
| `title` | The task's new description. | 120 characters |
| `summary` | One sentence: what the task does, for the reader. | 35 words |
| `changes` | What will be different once it is done. | 1–3 lines, 20 words each |
| `unchanged` | What stays the same, or is out of scope, that the reader might worry about. | 0–3 lines, 20 words each |
| `needs_your_eye` | Where the reader's judgement is needed: an assumption, a trade-off, a risk, an open question. | 0–3 lines, 30 words each |
| `terms` | Words the report uses that the reader may not know, each with a plain `meaning`. | 0–5; term 4 words, meaning 20 |
| `sections` | The parts that explain the plan, in reading order. | 1–6 |
| `sections[].heading` | A statement of what the part shows: "What you will see", not "Overview". | 10 words |
| `sections[].diagram` | One diagram: `{ "mermaid": "…" }` or `{ "svg": "<svg …>…</svg>" }`. | Mermaid 40 lines |
| `sections[].look_at` | With a diagram, and only then: what to look at in it. | 25 words |
| `sections[].points` | What the part says, one idea per line. | 1–5 lines, 25 words each |
| `sections[].detail` | Optional HTML, shown collapsed under "More detail", for whoever wants it. | 20,000 characters |
| `files` | Every file the plan touches: `path`, `change` (`new`, `change` or `remove`) and `why`. | 1–40; why 20 words |
| `checks` | The Done when checks: the `check`, and `how` it is checked. | 1–6; 20 words each |

Every field except `detail` and the diagram is one line of plain text. Two
marks are drawn specially: text in backticks as code, and a short label
before the first `: ` in bold. Start a line with a label when it helps the
reader scan: `"Approval: the task is still written only when you choose."`

`detail`, an SVG and a Mermaid diagram may not contain `<script>`, `<meta>`,
`<link>`, `<base>`, `<iframe>`, `<frame>`, `<frameset>`, `<object>`,
`<embed>`, `<form>` or `<portal>`: the page runs no script but Mermaid,
loads nothing but Mermaid, and sends nothing.

## Choosing the parts

Choose parts by the question they answer for the reader. Most tasks need two
to four. Put the one about what the reader will notice first.

| The reader asks | Make a part with | When |
|---|---|---|
| What will I see change? | A **before and after**: one diagram with the new parts marked `new`, or two small ones. | Anything the user sees or does changes. |
| What happens, in order? | A **sequence** diagram (`sequenceDiagram`). | Several actors — the user, a CLI, a daemon, Claude, a service — take turns. |
| What states can it be in? | A **state** diagram (`stateDiagram-v2`). | Something gains, loses or guards a state: a task's status, a mode, a flag. |
| Where does the data go? | A **data flow** (`flowchart LR`), stores as `[( )]`, arrows labelled with what moves. | Data is read, changed, stored or sent in a new way. |
| What talks to what? | A **component map** (`flowchart TB`, a `subgraph` per process). | A module, process or service is added, or a job moves between them. |
| What does each case do? | Points, one per case, or a table in `detail`. | Behaviour branches on two or more inputs. |
| What shape is the data? | A `classDiagram`, or a field table in `detail`. | A type, a JSON shape or a config's keys change. |
| What else was considered? | Points: each option, and why it was or was not chosen. | A Decided note chose between alternatives. |
| What could go wrong? | Points starting "Risk: …", and the worst one in `needs_your_eye`. | Something working could break, data could be lost, or a security boundary moves. |

The files and the checks are always there: they are fields, not parts.

## Drawing diagrams

- **One idea per diagram, at most about nine boxes.** Split a bigger one in
  two parts.
- **Plain names on the boxes:** "the panel", "Claude", "the task". File
  names belong in `files`.
- **Mark what the plan changes.** In a flowchart or a state diagram, add
  `class <node> new`, `change` or `remove`, or `:::new` after a node. The page
  draws them green, amber and dashed red, the same as the file badges, and
  says so above the first part. Do not define those classes yourself. In a
  sequence diagram, wrap the new steps in `rect rgb(220, 252, 231)` … `end`.
- **Label the arrows** with what moves or happens.
- **`look_at` points at the change:** "The new middle answer, and the loop
  back to the question", not "The flow".
- Use SVG only for what Mermaid draws badly, such as a screen mock-up. Give
  it a `viewBox`, and draw lines and text in `currentColor`.

## Writing the lines

- Start with the thing that matters, in everyday words.
- One idea per line; no "and also".
- Name things the way the project does (`CONTEXT.md`), and put any word the
  reader may not know in `terms`.
- No italics, capitals for emphasis or exclamation marks; the label is the
  emphasis.
- Say what the plan does, not what it "aims to" or "should" do.

## Worked example

This very feature, explained:

```json
{
  "title": "feat: Let a refine show an HTML report of its plan",
  "summary": "Before you approve a refined task, you can ask for a page that explains the plan with pictures, then approve or change it as before.",
  "changes": [
    "New answer: the question gains \"Show me a report first\", which writes nothing to the task.",
    "New page: the report opens in your browser and is saved under reviews, never in /tmp.",
    "New note: the task keeps a link to each report made while refining it."
  ],
  "unchanged": [
    "Approval: the task is still written only when you choose Write it to the task.",
    "Every refine: no report is made unless you ask for one."
  ],
  "needs_your_eye": [
    "Diagrams need the internet: offline, Mermaid diagrams show as their source text.",
    "Script: the page runs no script but pinned Mermaid, so the model cannot act in your browser."
  ],
  "terms": [
    {
      "term": "Refine",
      "meaning": "Claude turning a terse task into a plan, then writing it back."
    },
    {
      "term": "The mod",
      "meaning": "Code inside Claude Code that asks you before writing the task."
    },
    {
      "term": "Mermaid",
      "meaning": "A tool that draws diagrams from short text descriptions."
    }
  ],
  "sections": [
    {
      "heading": "What you will see",
      "look_at": "The new middle answer, and the loop back to the same question.",
      "diagram": {
        "mermaid": "flowchart TD\n  ask[\"Write this to the task?\"] --> write[\"Write it to the task\"]\n  ask --> report[\"Show me a report first\"]:::new\n  report --> page[\"Report opens in your browser\"]:::new\n  page --> ask\n  ask --> change[\"Change something\"]"
      },
      "points": [
        "Choose the report: nothing is written yet.",
        "Read the page: it opens by itself in your browser.",
        "Back in the terminal: the same question comes back."
      ]
    },
    {
      "heading": "How the report is made",
      "look_at": "The mod, not Claude, writes the page and opens it.",
      "diagram": {
        "mermaid": "sequenceDiagram\n  actor You\n  participant C as Claude\n  participant M as The mod\n  C->>M: write_task_plan(plan)\n  M->>You: Write this to the task?\n  rect rgb(220, 252, 231)\n  You-->>M: Show me a report first\n  M-->>C: Build the report\n  C->>M: show_task_report(report)\n  M->>You: Opens the page\n  end\n  C->>M: write_task_plan(plan)"
      },
      "points": [
        "Claude fills in fixed fields: summary, changes, diagrams, files and checks.",
        "The mod builds the page, so every report has the same layout.",
        "The page runs no script of Claude's: only the pinned Mermaid library."
      ]
    },
    {
      "heading": "What the task remembers",
      "points": [
        "Report note: each report becomes a note, Report: followed by its path.",
        "Earlier draft: a report made before you changed the plan is marked as an earlier draft.",
        "You approve it: the note is shown with the plan before you choose."
      ]
    }
  ],
  "files": [
    {
      "path": "claude/refine-mod/hooks/report.ts",
      "change": "change",
      "why": "Checks the report's fields and their length limits."
    },
    {
      "path": "claude/refine-mod/hooks/page.ts",
      "change": "new",
      "why": "Builds the page in one fixed, readable layout."
    },
    {
      "path": "claude/refine-mod/hooks/refine.ts",
      "change": "change",
      "why": "Adds the answer, the report tool and the report notes."
    },
    {
      "path": ".claude/skills/refine-task/SKILL.md",
      "change": "change",
      "why": "Tells Claude when and how to make a report."
    },
    {
      "path": ".claude/skills/refine-task/report-catalogue.md",
      "change": "change",
      "why": "Explains how to write a report that is easy to read."
    }
  ],
  "checks": [
    {
      "check": "Choosing the report opens a page in the browser",
      "how": "A live refine, by hand"
    },
    {
      "check": "Approving afterwards writes the task with a report note",
      "how": "A live refine, then `task export`"
    },
    {
      "check": "The mod's tests pass",
      "how": "`claude plugin test claude/refine-mod`"
    }
  ]
}
```

## Why it is built this way

- W3C, Making Content Usable for People with Cognitive and Learning
  Disabilities: https://www.w3.org/TR/coga-usable/
- Nielsen Norman Group, How Users Read on the Web:
  https://www.nngroup.com/articles/how-users-read-on-the-web/
- WCAG 2.2, Understanding 1.4.8 Visual Presentation:
  https://www.w3.org/WAI/WCAG22/Understanding/visual-presentation.html
- Mayer's principles of multimedia learning, and Lorch on text signals.
- Moody, The "Physics" of Notations (IEEE TSE, 2009).
- Mermaid syntax: https://mermaid.js.org/intro/
