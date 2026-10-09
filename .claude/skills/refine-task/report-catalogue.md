# Refine report catalogue

How to fill `show_task_report` when the user chooses **Show me a report
first**. The tool builds the report itself, as full-screen slides in one
fixed order, from the fields you give it:

1. the cover: the title and how long it takes;
2. **At a glance**: `summary`, `changes` and `unchanged`;
3. **Needs your eye**: `needs_your_eye`, when you give any;
4. **Words used here**: `terms`, when you give any;
5. one slide per part in `sections`, in lens order (below);
6. **Files**: `files`, when the plan touches any;
7. **Done when**: `checks`;
8. **The plan**: the description and notes, taken from the plan itself.

The report explains; it never asks the reader to do anything. Do not tell
them to write the task, approve it or go back to the terminal: the
terminal asks them that itself.

Your job is the words and the diagrams. Each part is one screen: a heading,
a diagram (or a before/after pair), and a few points beside it. Write it so
it reads whole with as little scrolling as you can.

## Who reads it, and what for

One person, deciding one thing: approve this plan, or say what to change.
Write for a reader who may have ADHD. They scan rather than read, hold about
four things in mind at once, lose their place when interrupted, and give up
on walls of text. What helps them helps every reader:

- **Conclusion first.** The top of the report answers "what does this do,
  and what changes?" before any detail.
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
| `sections` | The parts: each looks at the plan through one lens. | 1–9 |
| `sections[].kind` | The lens: `before-after`, `part`, `structure`, `data-flow`, `outside-tools`, `styling`, `code` or `newcomer`. Leave it out for a plain `part`. | |
| `sections[].heading` | A statement of what the part shows: "Where the new pieces sit", not "Overview". | 10 words |
| `sections[].diagram` | One diagram: `{ "mermaid": "…" }` or `{ "svg": "<svg …>…</svg>" }`. | Mermaid 40 lines; 50,000 characters |
| `sections[].before`, `.after` | Instead of `diagram`: the same view as it is and as it will be, drawn side by side. Give both. | as `diagram` |
| `sections[].look_at` | With a diagram or a pair, and only then: what to look at in it. | 25 words |
| `sections[].points` | What the part says, one idea per line. | 1–5 lines, 25 words each |
| `sections[].code` | For `code`: `file`, `before` and `after`, the key lines as they are and as they will be. `before` is `""` for a new file, `after` `""` for a removed one. | 1–3 files; 16 lines, 1,200 characters each |
| `sections[].tools` | For `outside-tools`: `tool`, `how` it connects, `swap` (`easy`, `medium` or `hard`) and `why`. | 1–8; tool 4 words, others 20 |
| `sections[].detail` | Optional HTML, shown collapsed under "More detail", for whoever wants it. | 20,000 characters |
| `files` | Every file the plan touches: `path`, `change` (`new`, `change` or `remove`) and `why`. Leave it empty only when the plan touches no file. | 0–40; path 200 characters, why 20 words |
| `checks` | The Done when checks: the `check`, and `how` it is checked. | 1–6; 20 words each |

Every field except `detail` and the diagram is one line of plain text. Two
marks are drawn specially: text in backticks as code, and a short label
before the first `: ` in bold. Start a line with a label when it helps the
reader scan: `"Approval: the task is still written only when you choose."`
Use `Label: ` only when you mean a label; reword any other `: ` in the
first 40 characters.

`detail`, an SVG and a Mermaid diagram may not contain `<script>`, `<meta>`,
`<link>`, `<base>`, `<iframe>`, `<frame>`, `<frameset>`, `<object>`,
`<embed>`, `<form>` or `<portal>`: the page runs no script when every
diagram is drawn, loads only pinned Mermaid for one that is not, and sends
nothing.

`detail` and an SVG are shown as given, so they may not break the page's one
layout either: no HTML comment (`<!--`), and no closing tag of the page's own
`<details>`, `<section>`, `<main>`, `<footer>`, `<nav>`, `<figure>`,
`<header>`, `<body>` or `<html>`. `detail` may not contain `<style>`,
`<plaintext>`, `<xmp>`, `<textarea>`, `<title>`, `<noscript>`, `<template>`,
`<html>`, `<head>` or `<body>`, which swallow or restyle the page. An SVG may
not contain `<style>`: use attributes and `currentColor`. An SVG `<title>` is
fine.

## The lenses

Look at the plan through every lens below, and give a part for each one
that applies. A report that leaves out a lens that applies is incomplete;
one that pads a lens that does not apply wastes the reader's time. Give
every part a diagram or a before/after pair unless it truly has nothing to
draw, as in `code`, whose code panes are its picture.

The slides come in this order, whatever order you give them in; plain
parts are numbered among themselves.

| Lens (`kind`) | When | What it shows | Draw |
|---|---|---|---|
| `before-after` | Always, unless nothing the plan touches changes how anything works. | How it works today against how it will work, for the one change that matters most. | A `before`/`after` pair: the same kind of diagram on both sides, the new parts marked `new` in `after`. |
| `part` | For the plan's own steps, mechanics or choices that no lens covers. | What the plan does, one idea per part. | Whatever fits, from the table below. |
| `structure` | Code, files or modules are added, moved or split. | How the code is broken up, where the new pieces sit, and whether the split stays logical. Say so plainly if it does not. | A component map (`flowchart TB`, a `subgraph` per folder or process), changed parts marked. |
| `data-flow` | Data is read, changed, stored or sent in a new way. | Where the data comes from, how it moves, where it lands, and what never flows back. | `flowchart LR`, stores as `[( )]`, arrows labelled with what moves. |
| `outside-tools` | The plan touches anything outside the app: niri, herdr, Claude Code, Taskwarrior, systemd, a browser, a command-line tool. | Each tool, how it connects (CLI, socket, file, config), and how hard it would be to swap for another. | A diagram of the app and its tools, arrows labelled with how they connect, plus `tools`. |
| `styling` | Anything the user sees changes: a panel, a card, a dialog, a colour, a message. | What looks different. | A `before`/`after` pair of screen mock-ups (SVG) or of the layout. |
| `code` | The plan changes code. | The key lines before and after, for the one to three files that matter most. A sketch is fine; say so. | `code` panes; no diagram needed. |
| `newcomer` | Always, unless the plan is too small to need explaining. | Could someone new to the project follow this: where it lives, what they would read first, what is missing. | A map from the README or the entry point to the new parts. |

### Rating a swap

For each tool in `outside-tools`, rate how hard it would be to replace with
another tool doing the same job, across the whole app, not just this plan:

- **easy** — one call or one file uses it, or another tool would drop in.
- **medium** — it sits behind one module or interface, but its name or its
  ideas reach further.
- **hard** — the app is built on its model: its data, its IPC, its idea of
  workspaces.

Look before you rate: count the files that name it, and say in `why` what
you counted.

### Kinds of diagram for a part

| The reader asks | Make a diagram with | When |
|---|---|---|
| What happens, in order? | A **sequence** diagram (`sequenceDiagram`). | Several actors — the user, a CLI, a daemon, Claude, a service — take turns. |
| What states can it be in? | A **state** diagram (`stateDiagram-v2`). | Something gains, loses or guards a state: a task's status, a mode, a flag. |
| What does each case do? | A decision flowchart, or points, one per case. | Behaviour branches on two or more inputs. |
| What shape is the data? | A `classDiagram`, or a field table in `detail`. | A type, a JSON shape or a config's keys change. |
| What else was considered? | Points: each option, and why it was or was not chosen. | A Decided note chose between alternatives. |
| What could go wrong? | Points starting "Risk: …", and the worst one in `needs_your_eye`. | Something working could break, data could be lost, or a security boundary moves. |

The files and the checks are always there: they are fields, not parts.

## Drawing diagrams

- **The mod draws them before the page opens**, with beautiful-mermaid:
  flowcharts, state, sequence, class and ER diagrams. Write ordinary
  Mermaid. A diagram it cannot draw falls back to Mermaid in the browser.
- **One idea per diagram, at most about nine boxes.** Split a bigger one in
  two parts.
- **Prefer left-to-right (`flowchart LR`) for few boxes in a row** and
  top-down for a tree: the slide scales a diagram to keep its labels
  readable, and a very tall one makes the slide scroll.
- **Plain names on the boxes:** "the panel", "Claude", "the task". File
  names belong in `files`.
- **Mark what the plan changes.** The change colours apply to flowcharts
  and state diagrams only. In one, add `class <node> new`, `change` or
  `remove`, or `:::new` after a node. The slides draw them green, yellow
  and dashed pink, the same as the file badges, with a legend under the
  diagram. Do not define those classes yourself. The mod keeps the
  colours when it draws a flowchart; a state diagram with marks is drawn
  in the browser instead, which needs the internet. In a sequence
  diagram, wrap the new steps in `rect rgb(153, 232, 133)` … `end`.
- **Generics in a class diagram** are written `Vec~Frame~`, not
  `Vec<Frame>`, which is refused.
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

Task 45, "Try a worktree build in a nested niri", seen through every lens
that applies (it changes nothing on screen, so it has no `styling` part):

```json
{
  "title": "feat: Try a worktree build in a nested niri",
  "summary": "A try.sh script runs any worktree's own build in a niri window of its own, on a copy of your tasks, without touching your real setup.",
  "changes": [
    "New script: try.sh builds the worktree and opens it in a nested niri window.",
    "Reload: one command rebuilds and restarts only the nested daemon.",
    "README: the try-by-hand advice points at try.sh instead of install.sh."
  ],
  "unchanged": [
    "Your setup: the installed binary, the live daemon and the real task list are never touched.",
    "Tests: e2e runs keep using the nested-niri helper exactly as now."
  ],
  "needs_your_eye": [
    "Keys: Mod is Right Alt inside the window; if niri refuses that, it falls back to plain Alt.",
    "Your tasks: the window works on a copy, so anything you change there is thrown away.",
    "Helper size: nested-niri.sh is 424 lines and gains an interactive mode beside its test mode."
  ],
  "terms": [
    {
      "term": "Nested niri",
      "meaning": "A niri running as a window inside your real niri, with its own workspaces."
    },
    {
      "term": "Try shell",
      "meaning": "The shell try.sh opens, aimed at the nested niri, where you run commands."
    },
    {
      "term": "Sandbox",
      "meaning": "The copy of your task data and the private sockets the window uses."
    }
  ],
  "sections": [
    {
      "kind": "before-after",
      "heading": "Trying a change: today and with try.sh",
      "look_at": "Today a try replaces the binary everyone shares; with try.sh it stays inside its own window.",
      "before": {
        "mermaid": "flowchart LR\n  you[\"You\"] -->|install.sh or cargo install| bin[\"~/.cargo/bin/niritasks\"]\n  bin --> daemon[\"Live daemon, restarted\"]\n  bin --> e2e[\"Other worktrees' e2e runs\"]\n  daemon --> tasks[(\"Real task list\")]"
      },
      "after": {
        "mermaid": "flowchart LR\n  you[\"You\"] -->|./try.sh| build[\"Worktree build\"]:::new\n  build --> win[\"Nested niri window\"]:::new\n  win --> copy[(\"Copy of your tasks\")]:::new\n  bin[\"~/.cargo/bin/niritasks\"] --> daemon[\"Live daemon, untouched\"]"
      },
      "points": [
        "Before: trying a change swaps the shared binary and restarts the daemon every worktree uses.",
        "After: each worktree tries its own build, in its own window, on its own copy."
      ]
    },
    {
      "kind": "part",
      "heading": "How a try session runs",
      "look_at": "Reload rebuilds and restarts only the nested daemon; the window and the copy stay.",
      "diagram": {
        "mermaid": "sequenceDiagram\n  actor You\n  participant T as try.sh\n  participant N as Nested niri\n  You->>T: ./try.sh\n  T->>T: cargo build, copy your tasks\n  T->>N: open the window, start its daemon\n  T-->>You: try shell\n  rect rgb(153, 232, 133)\n  You->>T: reload\n  T->>N: rebuild, restart the daemon\n  end\n  You->>T: exit\n  T->>N: close everything"
      },
      "points": [
        "Start: run try.sh from the worktree; `--release` builds release, for checking animations.",
        "Work: the panel shows the project's real tasks, from the copy.",
        "Stop: exiting the try shell tears the window and sandbox down."
      ]
    },
    {
      "kind": "part",
      "heading": "Keys inside the window",
      "look_at": "Super combinations reach your real niri first; Right Alt reaches only the nested one.",
      "diagram": {
        "mermaid": "flowchart LR\n  key[\"A key press\"] --> real{\"Your niri binds it?\"}\n  real -->|yes, Super combos| realbox[\"Your real box\"]\n  real -->|no, Right Alt combos| nested[\"Nested niri\"]:::new\n  nested --> cmd[\"Worktree niritasks\"]:::new"
      },
      "points": [
        "Right Alt is Mod: Super would reach your real niri first and open the real box.",
        "Same binds: the window loads niri-tasks.kdl as it is, finding the worktree build.",
        "Fallback: if niri will not take Right Alt, Mod becomes plain Alt."
      ]
    },
    {
      "kind": "structure",
      "heading": "Where the new pieces sit",
      "look_at": "try.sh sits beside install.sh and reuses the test helper; no Rust changes.",
      "diagram": {
        "mermaid": "flowchart TB\n  subgraph repo[\"niri-tasks repo\"]\n    install[\"install.sh\"]\n    try[\"try.sh\"]:::new\n    lib[\"tests/lib/nested-niri.sh\"]:::change\n    e2e[\"tests/e2e-*.sh\"]\n    src[\"src/ (Rust)\"]\n  end\n  try --> lib\n  e2e --> lib\n  try -->|cargo build| src"
      },
      "points": [
        "One sandbox: try.sh and the e2e tests share nested-niri.sh instead of keeping two.",
        "Logical split: building and seeding live in try.sh; isolating niri stays in the helper.",
        "No Rust: the app itself is untouched; only scripts and the README change.",
        "Watch: the helper now serves two callers, so its comments must say which mode does what."
      ]
    },
    {
      "kind": "data-flow",
      "heading": "Where your tasks and messages go",
      "look_at": "Data only flows into the sandbox; nothing flows back to your real task list.",
      "diagram": {
        "mermaid": "flowchart LR\n  real[(\"Real taskchampion.sqlite3\")] -->|SQLite online backup| copy[(\"Sandbox copy\")]:::new\n  copy --> daemon[\"Nested daemon\"]:::new\n  daemon --> panel[\"Panel in the window\"]\n  daemon -->|notify-send| stub[\"Stub log\"]:::new\n  stub --> shell[\"Try shell\"]"
      },
      "points": [
        "Copy: an online backup is consistent even while the live daemon writes.",
        "Writes: everything you do in the window lands in the copy only.",
        "Popups: notifications go to a stub log the try shell shows, never your desktop."
      ]
    },
    {
      "kind": "outside-tools",
      "heading": "The outside tools it leans on",
      "look_at": "Solid lines are used; dotted lines are kept out on purpose.",
      "diagram": {
        "mermaid": "flowchart LR\n  try[\"try.sh\"]:::new --> niri[\"niri, nested\"]\n  try --> cargo[\"cargo\"]\n  try --> py[\"python3 sqlite3\"]\n  try -.->|never| systemd[\"systemctl --user\"]\n  try -.->|kept off PATH| herdr[\"herdr\"]\n  daemon[\"Nested daemon\"] --> task[\"Taskwarrior\"]"
      },
      "tools": [
        {
          "tool": "niri",
          "how": "Runs a second niri as a window, loading niri-tasks.kdl.",
          "swap": "hard",
          "why": "The whole app is built on niri IPC, workspaces and binds."
        },
        {
          "tool": "Taskwarrior",
          "how": "The nested daemon reads the sandbox copy through task.",
          "swap": "hard",
          "why": "Tags, urgency and annotations are its data model, used in src/task.rs."
        },
        {
          "tool": "python3 sqlite3",
          "how": "Takes the online backup of the live database.",
          "swap": "easy",
          "why": "Any SQLite backup, such as the sqlite3 command, would do."
        },
        {
          "tool": "herdr",
          "how": "Kept off the window, as e2e runs do.",
          "swap": "medium",
          "why": "App-wide it sits behind src/session.rs, but its name is in 17 files."
        },
        {
          "tool": "Claude Code",
          "how": "Not reachable: refine needs herdr, which is kept out.",
          "swap": "medium",
          "why": "App-wide it starts through the session layer, plus the refine mod and skills."
        }
      ],
      "points": [
        "New dependency: python3 for the backup, already used elsewhere in the repo.",
        "Kept out: herdr and systemctl, so a try can never reach real sessions or the daemon."
      ]
    },
    {
      "kind": "code",
      "heading": "The code that changes",
      "code": [
        {
          "file": "tests/lib/nested-niri.sh",
          "before": "nested_start() {\n    ...\n    ok \"the nested niri is parked unfocused ...\"\n    watch_focus &\n}",
          "after": "nested_start() {\n    local focused=0\n    [ \"${1:-}\" = --focused ] && { focused=1; shift; }\n    ...\n    if [ \"$focused\" = 1 ]; then\n        ok \"the nested niri is open on this workspace\"\n    else\n        ok \"the nested niri is parked unfocused ...\"\n        watch_focus &\n    fi\n}"
        },
        {
          "file": "try.sh",
          "before": "",
          "after": "#!/usr/bin/env bash\n# try.sh [--release]: this worktree in a nested niri.\nset -euo pipefail\n. tests/lib/nested-niri.sh\ncargo build ${RELEASE:+--release}\nseed_copy ~/.task \"$TASKDATA\"   # SQLite online backup\nnested_start --focused\nstart_daemon \"target/$PROFILE/niritasks\"\ntry_shell                        # reload, exit"
        },
        {
          "file": "README.md",
          "before": "And to try a change by hand rather than under test,\neither `bash install.sh` ... or `cargo install --path .`\nfollowed by `systemctl --user restart niri-tasks`",
          "after": "To try a change by hand, run `./try.sh` from the\nworktree: your build opens in a nested niri on a copy\nof your tasks, and `reload` rebuilds it."
        }
      ],
      "points": [
        "Sketch: the code shows the shape of the change, not the final lines.",
        "Default: without --focused, nested_start behaves exactly as today."
      ]
    },
    {
      "kind": "newcomer",
      "heading": "Could someone new follow this?",
      "look_at": "A newcomer reads the README first; it now leads straight to try.sh.",
      "diagram": {
        "mermaid": "flowchart LR\n  readme[\"README\"]:::change --> try[\"try.sh\"]:::new\n  readme --> install[\"install.sh\"]\n  try --> lib[\"nested-niri.sh comments\"]\n  readme --> context[\"CONTEXT.md glossary\"]"
      },
      "points": [
        "Found easily: the README points at try.sh, next to install.sh, in one sentence.",
        "Explained: nested-niri.sh already explains the sandbox at length in its comments.",
        "Gap: CONTEXT.md has no entry for a try window; adding one would help.",
        "Safe to run: a newcomer cannot break the real setup by trying a change."
      ]
    }
  ],
  "files": [
    {
      "path": "try.sh",
      "change": "new",
      "why": "Builds, copies your tasks, opens the nested window and the try shell with reload."
    },
    {
      "path": "tests/lib/nested-niri.sh",
      "change": "change",
      "why": "Adds the --focused option to nested_start."
    },
    {
      "path": "README.md",
      "change": "change",
      "why": "Replaces the try-by-hand install advice with try.sh."
    }
  ],
  "checks": [
    {
      "check": "Two try windows and tests/all.sh run together, and every run passes",
      "how": "Three worktrees, by hand"
    },
    {
      "check": "Real daemon PID, task database and installed binary are unchanged",
      "how": "Compare before and after"
    },
    {
      "check": "Binds work with Right Alt, or the plain Alt fallback",
      "how": "Press them in a try window"
    },
    {
      "check": "Reload picks up an edit",
      "how": "Change a label, run `reload`"
    },
    {
      "check": "Panel shows the real tasks, and popups stay in the shell",
      "how": "Look in the window and the shell"
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
