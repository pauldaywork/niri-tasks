// The report catalogue's worked example, as show_task_report takes it: task 45,
// "Try a worktree build in a nested niri", seen through every lens that
// applies. Tests render it; the catalogue shows it. Keep the two in step.
export const EXAMPLE = {
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

// The plan the reader is shown with it: the task's own notes.
export const EXAMPLE_PLAN = {
  "description": "feat: Try a worktree build in a nested niri",
  "notes": [
    "Goal: From any worktree, run its own compiled niritasks by hand without disturbing the installed binary, the live daemon, the real task database, herdr, or e2e runs and agents in other worktrees.",
    "Context: The README says to try a change with install.sh or cargo install plus a daemon restart; both replace the shared ~/.cargo/bin/niritasks that the systemd daemon and other worktrees' e2e runs (NIRITASKS defaults to it) use, and install.sh relinks skills and rules.",
    "Context: tests/lib/nested-niri.sh already isolates an e2e run: its own nested niri and XDG_RUNTIME_DIR (so its own daemon socket), a sandbox TASKDATA, herdr stripped from PATH, and the parent lock for running beside other worktrees.",
    "Context: Inside a nested niri Mod defaults to Alt; the parent niri takes any combination it binds before the nested window sees it, so a Super Mod would open the real box. The parent binds no Alt combination without Mod except Alt+Print and Ctrl+Alt+Delete.",
    "Decided: Reuse nested-niri.sh through an option on nested_start (such as --focused) that skips parking and the focus watcher; the e2e tests keep using it unchanged, so there is still one sandbox.",
    "Decided: The nested window opens focused on the current workspace and niri sizes it like any other window, so the cards wrap at a realistic width.",
    "Decided: Seed the sandbox TASKDATA with an SQLite online backup (Python's sqlite3) of the real taskchampion.sqlite3, consistent even while the live daemon writes; writes go only to the copy.",
    "Decided: Name the nested niri's workspace after the worktree's project (niri-tasks gives tag niri_tasks) so the panel shows its real tasks; other projects come up with project open.",
    "Decided: Drive it mainly from a try shell in the launching terminal whose environment targets the nested niri, with the worktree's target/ binary first on PATH; exiting the shell tears everything down.",
    "Decided: Also include niri/niri-tasks.kdl as-is in the nested niri, its spawns finding the worktree binary, with Mod as Right Alt (xkb lv3:ralt_switch, mod-key-nested ISO_Level3_Shift); if niri will not take that, fall back to plain Alt.",
    "Decided: Notifications go to the e2e notify-send stub and the try shell shows its log, so no sandbox popup reaches the real desktop.",
    "Decided: A reload command in the try shell runs cargo build and restarts only the nested daemon, keeping the window and the sandbox copy.",
    "Decided: Build debug by default; a --release flag builds and runs release, for checking how the panel animates.",
    "Decided: The script is try.sh at the repo root, beside install.sh.",
    "Decided: Never touch ~/.cargo/bin, systemctl --user or install.sh's links.",
    "Steps: Add the nested_start option; write try.sh to build, seed the copy, start the nested niri with the project workspace and the Right Alt binds, start its daemon from target/ and open the try shell with reload; replace the README's try-by-hand install advice with try.sh.",
    "Done when: Two worktrees each have a try window open while tests/all.sh runs in a third, every run passes, and the real daemon's PID, the real task database and ~/.cargo/bin/niritasks are unchanged.",
    "Done when: In a try window the niri-tasks binds work with Right Alt (or the plain Alt fallback), reload picks up an edit, the panel shows the project's real tasks from the copy, and notifications appear only in the try shell.",
    "Out of scope: Exercising herdr actions (Start working, Go to session) against real herdr; installing a build for daily use, which stays install.sh on main."
  ]
}
