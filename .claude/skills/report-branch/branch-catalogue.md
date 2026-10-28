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
| `suite.runs` | Each suite that ran: a short name, the command, and the passed and failed counts from its own summary (in the test log, when you were given one). | 1–8; suite 6 words, command 200 characters |
| `suite.not_run` | Each suite that did not run, and why: an e2e that needs a display, one the log marks `SKIP:`, or one the sandbox stopped. | 0–6; why 25 words |
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
- **Test suite**: the suites as they ran. A suite you could not run is
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
{
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
    {
      "term": "Peek",
      "meaning": "The card that pokes out of the tucked-away panel's edge."
    },
    {
      "term": "Tucked away",
      "meaning": "The panel hidden past the screen edge, without the keyboard."
    },
    {
      "term": "Focus hook",
      "meaning": "A command herdr runs when a pane, tab or workspace is selected."
    }
  ],
  "hot_spots": [
    {
      "spot": "Clicks on a card that never has the keyboard",
      "why": "Go to and Move now take the keyboard mid-click; a stale peek must do nothing.",
      "files": [
        "src/panel/state.rs",
        "src/daemon.rs"
      ]
    },
    {
      "spot": "Three focus events per switch",
      "why": "One switch can fire all three; correctness rests on the daemon ignoring a repeat.",
      "files": [
        "herdr/task-docs/task_docs.py",
        "src/panel/state.rs"
      ]
    },
    {
      "spot": "Surface input region",
      "why": "Only the one card may take clicks; the e2e checks this by screenshot, not by click.",
      "files": [
        "src/panel/surface.rs",
        "tests/e2e-panel.sh"
      ]
    }
  ],
  "story": [
    {
      "step": "Planned the peek",
      "commits": [
        "7e677bd"
      ],
      "points": [
        "A 2,000-line plan: hooks, IPC, daemon, panel, docs."
      ]
    },
    {
      "step": "The panel shows one card; the CLI asks for it",
      "commits": [
        "3b5f037",
        "c27b9d7",
        "9b9dda1"
      ],
      "points": [
        "State first, with unit tests, then the request that sets it.",
        "A fix kept a tucked card from stealing focus."
      ]
    },
    {
      "step": "The e2e test checks the peek on screen",
      "commits": [
        "ef204a9",
        "49ec6e6"
      ],
      "points": [
        "Screenshots now measure one card; a peek that failed now fails the test."
      ]
    },
    {
      "step": "herdr drives it",
      "commits": [
        "36f9d36",
        "3bf8fde"
      ],
      "points": [
        "The plugin's own repeat check was dropped: the daemon already ignores repeats."
      ]
    },
    {
      "step": "Docs catch up",
      "commits": [
        "8576568",
        "2b2b7c1",
        "079ce8d",
        "47c4989"
      ],
      "points": [
        "CONTEXT.md, README and llms.txt name the peek; old hover wording gone."
      ]
    }
  ],
  "sections": [
    {
      "kind": "before-after",
      "heading": "The panel's edge, before and after",
      "look_at": "Before, every task poked out a line; after, only the selected pane's task does.",
      "before": {
        "mermaid": "flowchart LR\n  edge[Panel edge] --> l1[Task A, one line]\n  edge --> l2[Task B, one line]\n  edge --> l3[Task C, one line]"
      },
      "after": {
        "mermaid": "flowchart LR\n  pane[Focused herdr pane] --> peek[Its task card, whole]:::new\n  peek --> notes[Notes and buttons]:::new"
      },
      "points": [
        "Nothing peeks until a pane on a task is selected.",
        "A pane on no task clears the edge.",
        "A task leaving pending drops its peek."
      ]
    },
    {
      "kind": "data-flow",
      "heading": "From a pane switch to the card on screen",
      "look_at": "Three focus events all resolve to the one focused pane, then one IPC line.",
      "diagram": {
        "mermaid": "sequenceDiagram\n  participant H as herdr\n  participant P as task_docs.py\n  participant N as niritasks\n  participant D as daemon\n  H->>P: pane/tab/workspace.focused\n  rect rgb(153,232,133)\n  P->>H: pane list (find focused pane)\n  P->>N: task peek uuid8 | --clear\n  N->>D: peek <tag> <uuid>\n  D->>D: remember per tag, redraw panels showing it\n  end"
      },
      "points": [
        "The plugin remembers nothing: the daemon ignores a peek it already shows.",
        "The peek is kept per workspace tag, so a monitor switched later shows it.",
        "Failures go to herdr's plugin log, never a notification."
      ]
    },
    {
      "kind": "outside-tools",
      "heading": "herdr is now relied on for focus events",
      "points": [
        "The plugin needs herdr 0.9.3, the first with pane, tab and workspace focus events."
      ],
      "tools": [
        {
          "tool": "herdr",
          "how": "Plugin [[events]] hooks run `task_docs.py focused` on each focus.",
          "swap": "medium",
          "why": "One plugin file and one Python function speak herdr; the daemon never does."
        }
      ]
    },
    {
      "kind": "code",
      "heading": "The panel decides what to show from the peek",
      "points": [
        "Tucked away, `visible` is just the peeked card, with its buttons.",
        "The same peek twice returns no effects, so nothing redraws."
      ],
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
        {
          "name": "focused_pane()",
          "does": "Asks herdr for its panes and picks the focused one and its folder."
        },
        {
          "name": "peek(*args)",
          "does": "Runs `niritasks task peek`; logs a failure to stderr."
        },
        {
          "name": "focused_action()",
          "does": "The hook: peek the pane's task, or clear."
        }
      ]
    },
    {
      "path": "src/main.rs",
      "change": "change",
      "items": [
        {
          "name": "TaskCommand::Peek",
          "does": "Resolves uuid8, finds the session's workspace tag, sends the request."
        }
      ]
    },
    {
      "path": "src/ipc.rs",
      "change": "change",
      "items": [
        {
          "name": "Request::Peek",
          "does": "`peek <tag> [uuid]` on the wire, encoded and decoded."
        }
      ]
    },
    {
      "path": "src/daemon.rs",
      "change": "change",
      "items": [
        {
          "name": "PEEKS",
          "does": "The peek per workspace tag, in the daemon's memory."
        },
        {
          "name": "remember_peek()",
          "does": "Sets or clears a tag's peek."
        }
      ]
    },
    {
      "path": "src/panel/state.rs",
      "change": "change",
      "items": [
        {
          "name": "set_peek()",
          "does": "Changes the peek; a repeat returns no effects."
        },
        {
          "name": "take_keyboard_for()",
          "does": "Go to or Move on the peek: takes the keyboard, then presses."
        },
        {
          "name": "row_card() / peeked()",
          "does": "Which card shows its button row, by keyboard or peek."
        }
      ]
    },
    {
      "path": "src/panel/surface.rs",
      "change": "change",
      "items": [
        {
          "name": "Panel::peek()",
          "does": "Hands the peek to the state and redraws."
        },
        {
          "name": "Panel::shows()",
          "does": "Whether this monitor's panel shows a workspace tag."
        }
      ]
    },
    {
      "path": "src/link.rs",
      "change": "change",
      "items": [
        {
          "name": "live_agents_named()",
          "does": "Moved here from daemon.rs to be shared."
        }
      ]
    }
  ],
  "suite": {
    "runs": [
      {
        "suite": "cargo test",
        "command": "CARGO_TARGET_DIR=$TMPDIR/target cargo test --locked --no-fail-fast",
        "passed": 777,
        "failed": 0
      },
      {
        "suite": "Refine mod",
        "command": "claude plugin test claude/refine-mod",
        "passed": 202,
        "failed": 0
      },
      {
        "suite": "task-docs plugin",
        "command": "python3 -m unittest discover -s herdr/task-docs/tests",
        "passed": 40,
        "failed": 0
      }
    ],
    "not_run": [
      {
        "suite": "e2e-panel.sh, e2e-box.sh",
        "why": "Each nests a niri on your screen: run `bash tests/all.sh` yourself."
      },
      {
        "suite": "e2e-tag.sh",
        "why": "Renames a real workspace: left to `tests/all.sh`."
      }
    ]
  },
  "new_tests": [
    {
      "test": "tucked_away_only_the_peeked_card_shows_with_its_buttons",
      "file": "src/panel/state.rs",
      "layer": "unit",
      "pins": "Nothing peeked shows nothing; a peek shows that one card with buttons.",
      "result": "pass"
    },
    {
      "test": "the_same_peek_again_changes_nothing",
      "file": "src/panel/state.rs",
      "layer": "unit",
      "pins": "A repeat focus redraws nothing.",
      "result": "pass"
    },
    {
      "test": "the_keyboard_shows_every_card_and_keeps_the_peek_for_after",
      "file": "src/panel/state.rs",
      "layer": "unit",
      "pins": "The keyboard still lists all; Escape restores the peek.",
      "result": "pass"
    },
    {
      "test": "the_peeks_go_to_and_move_take_the_keyboard_on_its_card",
      "file": "src/panel/state.rs",
      "layer": "unit",
      "pins": "Go to and Move ask for the keyboard first.",
      "result": "pass"
    },
    {
      "test": "a_peek_is_kept_per_workspace_tag",
      "file": "src/daemon.rs",
      "layer": "unit",
      "pins": "Each workspace keeps its own peek.",
      "result": "pass"
    },
    {
      "test": "peek_carries_a_tag_and_an_optional_uuid",
      "file": "src/ipc.rs",
      "layer": "unit",
      "pins": "The wire format round-trips.",
      "result": "pass"
    },
    {
      "test": "test_a_focused_tasks_pane_peeks_at_its_task",
      "file": "herdr/task-docs/tests/test_task_docs.py",
      "layer": "integration",
      "pins": "A task pane runs `task peek <uuid8>`, against a fake herdr.",
      "result": "pass"
    },
    {
      "test": "test_a_pane_on_no_task_clears_the_peek",
      "file": "herdr/task-docs/tests/test_task_docs.py",
      "layer": "integration",
      "pins": "An unlinked pane runs `--clear`.",
      "result": "pass"
    },
    {
      "test": "e2e-panel.sh: the peek",
      "file": "tests/e2e-panel.sh",
      "layer": "e2e",
      "pins": "One card on screen in a nested niri; Escape puts it back.",
      "result": "not run"
    }
  ],
  "untested": [
    {
      "code": "Panel::peek and the input region",
      "file": "src/panel/surface.rs",
      "why": "GTK drawing; only the e2e screenshots cover it."
    },
    {
      "code": "TaskCommand::Peek outside a herdr session",
      "file": "src/main.rs",
      "why": "The refusal is not tested."
    }
  ],
  "plan_vs_done": [
    {
      "item": "herdr/task-docs focus hooks running `task_docs.py focused`",
      "kind": "step",
      "progress": "done",
      "shown_by": "36f9d36, 3bf8fde; plugin tests"
    },
    {
      "item": "An ipc Peek request",
      "kind": "step",
      "progress": "done",
      "shown_by": "c27b9d7; `peek_carries_a_tag_and_an_optional_uuid`"
    },
    {
      "item": "Daemon keeps the peek per tag; panel draws that card only",
      "kind": "step",
      "progress": "done",
      "shown_by": "3b5f037; `a_peek_is_kept_per_workspace_tag`"
    },
    {
      "item": "Update CONTEXT.md, README and the tests",
      "kind": "step",
      "progress": "done",
      "shown_by": "8576568, 2b2b7c1, 079ce8d"
    },
    {
      "item": "Selecting a pane, tab or workspace peeks only its task",
      "kind": "done when",
      "progress": "done",
      "shown_by": "Plugin tests; e2e peek check"
    },
    {
      "item": "Hovering slides it out with notes and working buttons",
      "kind": "done when",
      "progress": "partial",
      "shown_by": "Clicks are unit-tested; the slide-out is checked by hand"
    },
    {
      "item": "An unlinked pane leaves the edge empty",
      "kind": "done when",
      "progress": "done",
      "shown_by": "`test_a_pane_on_no_task_clears_the_peek`"
    },
    {
      "item": "Mod+Alt+Ctrl+T is unchanged",
      "kind": "done when",
      "progress": "done",
      "shown_by": "`the_keyboard_shows_every_card_and_keeps_the_peek_for_after`"
    },
    {
      "item": "cargo test and the task-docs plugin tests pass",
      "kind": "done when",
      "progress": "done",
      "shown_by": "Test suite: every suite passed"
    }
  ],
  "files": [
    {
      "path": "herdr/task-docs/task_docs.py",
      "change": "change",
      "why": "The focus hook: finds the focused pane's task and peeks it."
    },
    {
      "path": "herdr/task-docs/herdr-plugin.toml",
      "change": "change",
      "why": "Hooks pane, tab and workspace focus; needs herdr 0.9.3."
    },
    {
      "path": "src/ipc.rs",
      "change": "change",
      "why": "The peek request on the daemon's socket."
    },
    {
      "path": "src/main.rs",
      "change": "change",
      "why": "`niritasks task peek`."
    },
    {
      "path": "src/daemon.rs",
      "change": "change",
      "why": "Keeps the peek per tag and hands it to panels."
    },
    {
      "path": "src/panel/state.rs",
      "change": "change",
      "why": "What a tucked-away panel shows and does."
    },
    {
      "path": "src/panel/surface.rs",
      "change": "change",
      "why": "Draws the one peeked card and passes clicks."
    },
    {
      "path": "src/link.rs",
      "change": "change",
      "why": "Live agents of a named workspace, shared."
    },
    {
      "path": "tests/e2e-panel.sh",
      "change": "change",
      "why": "Screenshots the peek in a nested niri."
    }
  ]
}
```
