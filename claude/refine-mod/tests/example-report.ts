// The report catalogue's worked example, as show_task_report takes it: this
// very feature, explained. Tests render it; keep the catalogue's copy in step.
export const EXAMPLE = {
  title: 'feat: Let a refine show an HTML report of its plan',
  summary:
    'Before you approve a refined task, you can ask for a page that explains the plan with pictures, then approve or change it as before.',
  changes: [
    'New answer: the question gains "Show me a report first", which writes nothing to the task.',
    'New page: the report opens in your browser and is saved under reviews, never in /tmp.',
    'New note: the task keeps a link to each report made while refining it.',
  ],
  unchanged: [
    'Approval: the task is still written only when you choose Write it to the task.',
    'Every refine: no report is made unless you ask for one.',
  ],
  needs_your_eye: [
    'Diagrams: if the mod cannot run its renderer, they fall back to Mermaid, which needs the internet.',
    'Script: none runs; only an undrawn diagram loads pinned Mermaid.',
  ],
  terms: [
    { term: 'Refine', meaning: 'Claude turning a terse task into a plan, then writing it back.' },
    { term: 'The mod', meaning: 'Code inside Claude Code that asks you before writing the task.' },
    { term: 'Mermaid', meaning: 'A tool that draws diagrams from short text descriptions.' },
  ],
  sections: [
    {
      heading: 'What you will see',
      look_at: 'The new middle answer, and the loop back to the same question.',
      diagram: {
        mermaid: [
          'flowchart TD',
          '  ask["Write this to the task?"] --> write["Write it to the task"]',
          '  ask --> report["Show me a report first"]:::new',
          '  report --> page["Report opens in your browser"]:::new',
          '  page --> ask',
          '  ask --> change["Change something"]',
        ].join('\n'),
      },
      points: [
        'Choose the report: nothing is written yet.',
        'Read the page: it opens by itself in your browser.',
        'Back in the terminal: the same question comes back.',
      ],
    },
    {
      heading: 'How the report is made',
      look_at: 'The mod, not Claude, writes the page and opens it.',
      diagram: {
        mermaid: [
          'sequenceDiagram',
          '  actor You',
          '  participant C as Claude',
          '  participant M as The mod',
          '  C->>M: write_task_plan(plan)',
          '  M->>You: Write this to the task?',
          '  rect rgb(153, 232, 133)',
          '  You-->>M: Show me a report first',
          '  M-->>C: Build the report',
          '  C->>M: show_task_report(report)',
          '  M->>You: Opens the page',
          '  end',
          '  C->>M: write_task_plan(plan)',
        ].join('\n'),
      },
      points: [
        'Claude fills in fixed fields: summary, changes, diagrams, files and checks.',
        'The mod builds the page, so every report has the same layout.',
        'Nothing runs: diagrams are drawn before the page opens.',
      ],
    },
    {
      heading: 'What the task remembers',
      points: [
        'Report note: each report becomes a note, Report: followed by its path.',
        'Earlier draft: a report made before you changed the plan is marked as an earlier draft.',
        'You approve it: the note is shown with the plan before you choose.',
      ],
    },
  ],
  files: [
    { path: 'claude/refine-mod/hooks/report.ts', change: 'change', why: "Checks the report's fields and their length limits." },
    { path: 'claude/refine-mod/hooks/page.ts', change: 'new', why: 'Builds the page in one fixed, readable layout.' },
    { path: 'claude/refine-mod/hooks/refine.ts', change: 'change', why: 'Adds the answer, the report tool and the report notes.' },
    { path: '.claude/skills/refine-task/SKILL.md', change: 'change', why: 'Tells Claude when and how to make a report.' },
    { path: '.claude/skills/refine-task/report-catalogue.md', change: 'change', why: 'Explains how to write a report that is easy to read.' },
  ],
  checks: [
    { check: 'Choosing the report opens a page in the browser', how: 'A live refine, by hand' },
    { check: 'Approving afterwards writes the task with a report note', how: 'A live refine, then `task export`' },
    { check: "The mod's tests pass", how: '`claude plugin test claude/refine-mod`' },
  ],
}

// The plan the person was shown when they asked for the worked example: the
// page repeats it as it will be written.
export const EXAMPLE_PLAN: { description: string; notes: string[] } = {
  description: EXAMPLE.title,
  notes: [
    'Goal: let the person see the plan explained on a page before they approve it.',
    'Decided: the mod builds the page from fixed fields, so every report reads the same way.',
    'Done when: choosing the report opens it, and approving links it from the task.',
  ],
}
