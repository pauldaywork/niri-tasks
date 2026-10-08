# One Session module for refine, start working and go to session — design

Date: 2026-10-03

Decisions from the architecture review of 2026-10-03 (candidate 1) and the
grilling that followed. Everything here is settled except Q19, the shape of
the Session interface, which has its own comparison report.

Tasks: `0cb3900d` refactor: move session opening behind one Session module;
`2ee54dab` fix: keep two quick refines to one agent.

## Goal

Refine, Start working and Go to session each open a workspace's herdr
session and then put or find a Claude in it. Today that logic is spread over
`refine.rs` (which owns `open_session` and lends it to the others), `work.rs`,
`link.rs`, `session.rs` and `herdr.rs`, with the agent-exists-then-focus and
open-a-tab halves copied between the start and refine flows, six `HOME`
lookups, three hand-written fake-herdr scripts in the tests, and no way to run
any flow without a PATH shim. One deep module, `session`, absorbs it, behind a
seam the tests can cross with an in-crate fake.

## Decisions

| # | Question | Decision |
|---|---|---|
| Q1 | Scope | Both pieces, two commits: (a) move and dedupe into one module; (b) the runner seam, fake and tests |
| Q2 | Where | `session.rs` grows into the deep module; its naming rules stay as implementation and keep their tests |
| Q3 | The value | One `Session { name, dir }`, no `Project`. `Session::for_workspace(ws)` is pure (HOME → start_dir, herdr_session_name). The `.git` check stays in `work.rs` on `session.dir` |
| Q4 | Behind the seam | Opening (three paths), focus-an-existing-agent, open-a-tab-or-create-the-workspace, `live_agent_names`/`live_agent`, `wait_for_session`, `show_session_window`. `link.rs` keeps only the naming rules |
| Q5 | Current session | `Session::current() -> Option<Session>` from `HERDR_SESSION` / `HERDR_SOCKET_PATH` (and the pane and tab ids); replaces the env reads in `lib.rs` and `link.rs` |
| Q6 | Agent names | `task-`/`work-` and uuid8 in one place in `link.rs`; the session module does not know what a task is |
| Q7 | Double-refine race | Test first against the new seam, expected to fail; fix in its own commit on task `2ee54dab` |
| Q8 | Runner in Rust | A trait, held as `Box<dyn …>` given at construction. `for_workspace` and `current` use the process adapter; `with_runner` for tests |
| Q9 | Reach of the port | The module's whole way out: herdr, niri windows list, niri focus_window, niri spawn, sleep (Q16), notify (Q22), clipboard (Q26). Nothing else leaves the module |
| Q10 | The test adapter | A stateful fake herdr (workspaces, tabs with panes, agents, a window list), in-crate under `cfg(test)`, not an argv script |
| Q11 | PATH-shim tests | `typed_task_number.rs`, `link_pane.rs` and `e2e-tag.sh` stay as they are; no new tests of that kind; no shared `tests/common` in this work |
| Q12 | `herdr.rs` | Becomes private to the session module (as `session/herdr.rs` or the process adapter, per Q19); `run` is one line over `run_coded` |
| Q13 | Errors | herdr's coded errors come through as `HerdrError { code, message }`; "cannot run herdr" is always `Err`, never "no" |
| Q14 | Glossary | **Session** added to CONTEXT.md (commit `1bec5b9`) |
| Q15 | Interface | Designed twice (four sub-agents); choice pending, see Q19 |
| Q16 | Time | `sleep(Duration)` is on the port, so the fake counts polls and the "did not start" deadline tests in microseconds |
| Q17 | Tests | See *Tests* below |
| Q18 | Delivery | Two tasks, a worktree, a written plan first; glossary committed on main |
| Q19 | Interface shape | **Open.** C (one deep `claude(...)` call) versus D (step methods), and the port cut (raw argv versus typed operations). The hybrid, C over D's port, was recommended |
| Q20 | Start working, existing worktree | Simplified, see *Start working*; no resume kind, no spike |
| Q21 | Refine's delivery | Refine gets the worker's: a blocked start is waited out, the prompt is confirmed delivered with resends on a stall |
| Q22 | Notifications | `notify(text)` is on the port, so the fake sees the blocked-start notice and the clipboard notice |
| Q23 | Error type | `anyhow::Result` with a downcastable `HerdrError`, as the rest of the crate |
| Q24 | Active, no worktree | Rule 3 below; the live-agent check comes first |
| Q25 | Clipboard command | `niritasks task start --here <uuid>`; the tab close moves to a hidden `--close-tab` the launcher passes; `--workspace` is dropped (`Session::current()` makes it redundant) |
| Q26 | Clipboard | `clipboard(text)` on the port. **Research first**, in the plan, before choosing between `wl-copy` (installed by install.sh), a Rust crate on the Wayland data-control protocol, or GTK from the daemon; the deciding fact is whether the text survives the process exiting |
| Q27 | Rule 4 | A not-active task with no worktree still gets the setup tab that makes one |
| Q28 | Rule 2 notice | "No Claude on this task. Its start command is in the clipboard; paste it here, or claude --resume to pick up a session." |

## Start working

Supersedes the "second pick" row of the 2026-09-29 design, which started a
fresh Claude when the old one was gone.

1. **Worktree exists, live agent.** Go to it (open the session, focus the
   agent). Unchanged.
2. **Worktree exists, no agent.** `herdr worktree open --focus` on it (which
   finds or opens its workspace), put `niritasks task start --here <uuid>` in
   the clipboard, and notify as Q28. Nothing is started; the user pastes it,
   or runs `claude --resume` by hand.
3. **No worktree, task active.** Clear active with `task::stop`, then fail:
   "This task was active but has no worktree. It is stopped; Start working
   again to make one." Nothing opens. The live-agent check (rule 1) runs
   first, so a hand-started Claude linked to the task in the project folder is
   still reached.
4. **No worktree, task not active.** The setup tab that makes one, as today.

`niritasks task start --here <uuid>`, run in any pane of the project's
session: find the task's worktree (error if none, pointing at Start working),
open or focus its workspace, open a Claude tab there, start Claude under the
task's `work-` name, deliver the plan prompt, mark the task active. The
worktree is made first only when the task has none; the tab is closed
afterwards only with `--close-tab`. The pane rule inside the module: a
worktree whose workspace was already open gets a new "Claude" tab, so the
user's own pane is left alone; a freshly opened one uses its root pane.

## The port and its adapters

```rust
pub struct HerdrError { pub code: Option<String>, pub message: String }
// herdr: either a raw `herdr(argv) -> Result<Result<Value, HerdrError>>`
//        or typed operations (workspace_list, tab_create, agent_get, …) — Q19
// niri:  windows() -> Vec<WindowInfo>, focus_window(id), spawn(argv)
// time:  sleep(Duration)
// user:  notify(text), clipboard(text)
```

Two adapters, so the seam is real: the process adapter (std::process for
herdr, niri_ipc for the three niri calls, thread::sleep, notify-send, the
clipboard tool the research picks) and the in-crate fake.

The fake models, per session: running or stopped with a boot countdown that
`sleep` advances; herdr workspaces with labels; tabs with root panes;
worktree workspaces keyed by path so `worktree open` can answer
`already_open`; agents with name, pane and status; a scripted `agent start`
outcome (ok, failed, or failed-while-blocked then idle after `agent wait`);
scripted stall codes per `agent prompt --wait` attempt; a niri window list;
and a log of spawns, focused windows, focused agents, closed tabs, pane runs,
renames, notices, clipboard writes and sleeps.

## Tests

At the Session interface, against the fake:

- open on a stopped session starts the project terminal and waits for it;
  on a running session with a window, focuses it; running with no window,
  attaches another terminal; the deadline path errors "did not start"
- focus of a live agent is true; of none, false; herdr not runnable is an error
- a tab goes in the first herdr workspace, or the workspace is created when
  there is none
- a failed Claude start closes the tab the module made; a blocked start
  notifies and waits; a stalled prompt is resent up to four times
- live agents of a stopped session are none, but no herdr is an error

At the flows, against the fake:

- refine twice on one task ends with one agent (fails until `2ee54dab`)
- start working: rule 1 focuses the working Claude; rule 2 opens the
  worktree, writes the clipboard, notifies, starts nothing; rule 3 stops the
  task and errors without opening anything; rule 4 opens the setup tab;
  `--here` makes the worktree only when missing and closes the tab only with
  `--close-tab`
- go to session focuses the working Claude over the refine, errors with none

Kept: `herdr.rs`'s 22 argv and JSON tests move with the code (to the process
adapter under a typed port, or stay with the builders under a raw one).
Kept: the three PATH-shim suites. Deleted afterwards: whatever of their
assertions the fake now covers, as `2fe78b1` did for the panel.

## Plan outline

1. Research the clipboard mechanism (Q26) and record the pick.
2. Commit (a): the move. `session.rs` absorbs opening, tabs, agents;
   `work.rs`, `refine.rs`, `link.rs` call it; `herdr.rs` goes private;
   `lib.rs`'s `herdr_session` becomes `Session::current`. Existing tests pass.
3. Commit (b): the port, the process adapter, the fake, the interface tests.
4. Commit (c): Start working rules 1–4, `--here` without `--workspace`,
   `--close-tab`, the clipboard and the notice, with their tests. README and
   llms.txt rows for the changed command (the command-reference tests
   require it).
5. Task `2ee54dab`: the race test, then the fix.
