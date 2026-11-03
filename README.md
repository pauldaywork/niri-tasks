# niri-tasks

Workspace-scoped Taskwarrior for [niri](https://github.com/YaLTeR/niri).

The niri workspace you are looking at *is* the task filter:

- Name a workspace `website`, and every task you add from it is tagged `+website`.
- The list shortcut shows that workspace's tasks and nothing else.
- Switch workspace, and the same keys show a different set.

Which tasks you see goes purely off the workspace name:

- `niritasks project open` sets it to the folder name.
- That is the only way a workspace gets a name, bar workspace 1, which starts as `general`.
- `~/Projects` is only ever read to offer folders to pick from: opening one, or moving a task to another workspace.

## Keybinds

| Key | Does |
|---|---|
| `Mod+Alt+T` | Add a task to this workspace, with notes. Enter starts each one |
| `Mod+Alt+Shift+T` | The same, with **Add & refine** as the default (below) |
| `Mod+Alt+Ctrl+T` | Hand the [task panel](#task-panel) the keyboard, in the middle of the screen |
| `Mod+Alt+W` | The task panel's project list, in the middle of the screen (below) |

**Add & refine:** once added, the task opens in a "Refine: …" tab of the
workspace's herdr session, where Claude works it up into a plan.

**`Mod+Alt+Ctrl+T`, the panel with the keyboard** (full detail under [Task panel](#task-panel)):

- Every card shows its whole description.
- The focused card has buttons to:
  - open its Go to… list (refine it into a plan with Claude or be grilled about it, its report, plan and spec, its Claude sessions, its worktree, VS Code, read it aloud)
  - start working on it in its own worktree
  - edit it, complete it, or move it to another project's workspace
  - stop it, park it as waiting, or remove it
- Ctrl+Enter refines the focused task, or starts it once it is planned, and keeps the list up. On a task already being worked with a Claude on it, it goes to that session.
- Tabs above the cards narrow them: All, Active, Planned, To refine, Waiting, Finished. Each is labelled with its key, as To refine (4); pick with 1–6 or `[` and `]`.
- The Waiting tab's Clear all (Ctrl+Shift+Delete) deletes every waiting task.
- Finished lists the last 12 finished tasks; Back to list (`b`) reopens one.
- Ctrl+Delete deletes the focused task at once.
- A hint beside the focused card's buttons, and a line under the list, name the keys.
- The last tab, Ideas (7), is a notepad for the workspace's ideas that are not tasks yet.

**`Mod+Alt+W`, the project list:**

- It lists the `~/Projects` folders, then your GitHub repos not cloned yet.
- Type to narrow it.
- Enter or a click puts the project on its own named workspace (cloning a repo first), and opens a terminal and an editor in it.
- When nothing matches, the line under the title says what Enter will do with the name: open a folder of that name, make one, or clone your GitHub repo of that name.
- Escape clears what you typed, then closes it.

## Task panel

### Cards

Each monitor's task panel holds the pending tasks of the workspace it is
displaying, as a stack of cards.

Order:

1. The active tasks (there can be several, one per worktree being worked): ▶ and text in green.
2. Any up next.
3. The rest by priority (H, M, L, none), newest first.
4. The blocked ones last.

Icons:

- ○ for a plain task.
- A dimmed lock for one blocked on another task.
- ● for one already worked up into a plan with Claude, from a card's Go to
  list's **Refine** or **Grill me**.
- A report icon for a planned one whose notes link a refine report, which
  its Go to list's **Open report** opens.

Up to eight cards show, then a "+N more" card. Each card ends in how long
ago its task was added: 5m, 3h, 2d, 4w.

### The peek on the right edge

- Tucked away on the right edge, the panel shows one card at most: the task
  of the herdr pane you have selected, on that workspace's monitor (see
  [Peeking at the focused pane's task](#peeking-at-the-focused-panes-task)).
- Only 30px of it shows.
- The pointer on it slides it out to full width, whole: description, id and
  notes, buttons.
- The buttons work by click while herdr keeps the keyboard. Moving away
  slides it back.
- Go to… and Move to workspace open lists the keyboard drives, so a click on
  either first hands the panel the keyboard on that card.
- The space around the card is click-through, so the panel never gets in the
  way of the windows beneath. A fullscreen window covers it.
- With no task's pane selected, nothing shows at all.
- The rest of the workspace's tasks are on the keyboard's panel (below),
  whose "+N more" card shows any past the first eight.

### With the keyboard (Mod+Alt+Ctrl+T)

`Mod+Alt+Ctrl+T` hands the panel the keyboard.

- It moves to the middle of the screen, leaving no peek on the right edge
  while it is there.
- Every card opens up to its whole description, wrapped.
- The focused card shows a row of buttons under it.

With no tasks at all, finished ones included, or no workspace name, a
notification says so instead.

The buttons:

| Button | Key | Does |
|---|---|---|
| **Go to…** (blue) | `o` | A list of what else can be done with the task and where it can be gone to. First on every card |
| **Start working** (green) | `s` | The task's own worktree, herdr tab and Claude. Not on an active task |
| **Edit** (yellow) | `e` | Open it in the task box. Closing the box, saved or not, hands the keyboard back to the panel on the same tab, on this card |
| **Up next** (yellow) | | Mark it as the one to do next. Not on a waiting or finished task |
| **Complete** (green) | `c` | Mark it done: it leaves the panel, which keeps the keyboard — not on a finished task |
| **Move to workspace** (sapphire) | `m` | Pick another `~/Projects` folder from a list in the panel, and move the task to that folder's workspace — not on a finished task |
| **Stop** (peach) | `t` | Stop it — only on an active task |
| **Waiting** (teal) | | Park it as waiting: off the panel, and on the Waiting tab instead |
| **Back to list** (lavender) | `b` | Off waiting, or reopened from finished, back on the list |
| **Remove** (red) | `Delete` | Turns into **Confirm remove**; a second press deletes it |
| **Clear all** (red) | `Ctrl+Shift+Delete` | On the Waiting tab: deletes every waiting task, after **Confirm clear all** |

- **Go to…**: every row is in the Go to… table further down.
- **Start working**: Claude plans the task first and links the plan it saves
  as a `Plan: docs/superpowers/plans/<file>.md` note. An active task is
  already being worked.
- **Up next**: the card turns yellow and moves up under the active tasks. On
  a task already up next it reads **Not up next**, and clears it.
- **Back to list**: only on a waiting or finished task.
  - A waiting task gets just this, Go to…, Edit, Complete, Move to workspace
    and Remove.
  - A finished one gets just this, Go to…, Edit and Remove.
- **Remove**: moving away puts it back. `Ctrl+Delete` deletes the focused
  task at once, whichever button is focused, and moves to the next card.
- **Clear all**: sits on a strip under the tabs, not a card.
  - Arming it takes the focus off the cards.
  - A second press or `Enter` deletes every waiting task on the workspace
    and goes back to All.
  - `Escape` or moving away puts it back.

### Keys

| Key | Does |
|---|---|
| Up, Down | Move the darker fill and the buttons between cards. Down onto "+N more" shows the rest in its place and carries on to the first of them |
| Left, Right, Tab | Move along the focused card's buttons |
| Enter | Presses the focused button |
| Enter or a click on a card's description | Shows its notes; a second press hides them |
| Escape | Puts the panel back on the right edge, tucked away to the peeked card if there is one |

- The hint: beside the buttons, in smaller dimmed text, the focused card
  names its own keys.
  - The focused button's letter and name (`o: Go to…`, `Del: Remove`).
  - Or on a card's description `Space: view notes` / `Space: hide notes`.
  - And what Ctrl+Enter would do to this task.
- The line under the list names the keys that act the same on every card:
  Enter or Space, and Ctrl+Delete. It is not on Ideas, where the notepad
  takes those keys as typing.
- Notes show, dimmed under the description, the id and uuid (`#48 · <uuid>`,
  the uuid alone on a finished task), then its notes, one per note.
  - They scroll past about twelve lines.
  - Opening another card's notes hides the first's.
  - Giving the keyboard back hides them all.
- Every button that opens something gives the keyboard back as it runs.
  Go to…, Up next, Complete, Move to workspace, Waiting, Back to list and
  Remove keep the list up.
- A list taller than the screen scrolls, keeping the focused card in view.
- Hovering shows the buttons only on the peeked card; on the others only the
  keyboard does.

### Move to workspace

1. The cards swap for a list of your `~/Projects` folders, drawn like cards,
   under a line naming the task. Every folder but this workspace's own is
   listed.
2. Type to narrow it.
   - The field above ranks folders as fzf does, best match first.
   - The highlight goes to the top match.
   - Without fzf it keeps the folders whose names hold what you typed.
3. Up and Down move the highlight.
4. Enter or a click moves the task there.
   - It is retagged to that folder's workspace.
   - The panel goes back to the cards, with the next one focused.
5. Escape clears what you typed. On an empty field, it goes back to the
   cards, on the same one, without moving the task.

### Go to…

It swaps the cards for a list under a line naming the task, of what else can
be done with it and where it can be gone to, each row only when it applies:

| Row | Shows when |
|---|---|
| **Refine**, **Redo refine** once planned, and **Grill me** | on a task on the list, unless its refine Claude is running: Go to refine session then goes there |
| **Get started**, **Back to worktree** once started | on a task on the list, unless its work Claude is running: Go to work session then goes there |
| **Generate report**, **Regenerate report** once one exists | on a planned task on the list |
| **Open report** | its notes link a report (`Report: <path>`) |
| **Explain branch** | the task has a worktree |
| **Branch report** | its notes link a branch report: opened as Open report opens a report |
| **Planning doc**, **Design spec** | its notes link a plan (`Plan: <path>`) or a spec (`Spec: <path>`) |
| **Go to refine session**, **Go to work session** | on a task on the list, that Claude (`task-<uuid8>`, `work-<uuid8>`) is running, checked as the panel slides out |
| **Open worktree folder** | the task has a worktree: a terminal in it |
| **Open in VS Code** | `code` is installed: on the worktree, else the project folder |
| **Read aloud**, **Stop reading aloud** while a speech plays | always: its description and notes, rewritten by Claude for listening |

- **Open report**:
  - The report's path is absolute, `~/...`, or a name in
    `~/.local/share/niri-tasks/reviews`.
  - An HTML report opens in a new browser window on this workspace.
  - A Markdown one opens in a VS Code of its own, as the plan and spec are.
- **Explain branch**:
  1. Its tests run first in a `Tests:` tab, outside Claude's sandbox, when it
     has a test command (`git config niri-tasks.test '<command>'`, else this
     repo's `tests/all.sh` headless).
  2. Claude, in a tab of its own in the worktree, explains what the branch
     has changed since it left the default branch.
  3. Claude reads that test run into its report (or, with no test command,
     runs what tests it can itself).
  4. The deck is linked as `Branch report: <path>`.
- **Planning doc**, **Design spec**: opened in a VS Code of its own, apart
  from your VS Code windows, sharing your extensions, settings and
  keybindings.

Rows of a kind share a colour:

| Colour | Rows |
|---|---|
| mauve | a Claude working on the plan (Refine, Grill me, Generate report, Explain branch) |
| green | Get started |
| blue | going to a running Claude |
| yellow | the documents (Open report, Branch report, Planning doc, Design spec) |
| sapphire | the worktree and VS Code |
| pink | Read aloud |

- There is no field to type in.
- Up and Down move the highlight.
- Enter or a click runs the row and gives the keyboard back. Read aloud keeps
  it, back on the cards.
- Escape goes back to the cards on the same one.
- The plan and the spec are linked by the Claude Start working starts, once
  it has saved its plan.
- Their paths are the repository's, read in the task's worktree while it has
  one.

### Ctrl+Enter and Ctrl+Delete

- Both act on the focused card, whichever of its buttons is focused.
- Ctrl+Enter depends on the task:

| Task | Ctrl+Enter |
|---|---|
| Unplanned | Refines it. The panel keeps the keyboard, same card focused, so you can carry on down the list |
| Planned | Starts working on it. Same: keyboard kept, same card focused |
| Being worked, live Claude, planned or not | Goes to that session, as the Go to list's session rows do, and gives the keyboard back |
| Worked, no Claude | Refines it if it has no plan; does nothing if it has one |
| Waiting or finished | Does nothing |

- Ctrl+Delete deletes the task at once, with no Confirm remove.
- It moves the focus to the next card (the one above, from the last), with
  the list kept up.
- Delete alone still asks first.
- Ctrl still held from the `Mod+Alt+Ctrl+T` chord counts:
  - let go of Ctrl before Enter if you only want the focused button;
  - let go of Ctrl before Delete if you want Remove to ask.
- Every other Ctrl chord passes through untouched, apart from
  Ctrl+Shift+Delete, the Waiting tab's Clear all (below).

### Tabs

Above the cards, six tabs narrow them while the panel has the keyboard:

| Tab | Key | Holds |
|---|---|---|
| **All** | `1` | everything but waiting and finished tasks |
| **Active** | `2` | started tasks |
| **Planned** | `3` | refined or grilled into a plan, not started yet |
| **To refine** | `4` | neither planned nor started yet |
| **Waiting** | `5` | parked tasks |
| **Finished** | `6` | the last 12 completed, the most recently finished first, each aged from when it was finished |

- A tab shows only while it has a task under it, All apart.
- `1`–`6` each always pick the same tab, and do nothing while it is hidden.
- Each label ends in its key, as **To refine (4)**.
- `[` and `]` step to the shown tab either side.
- A click picks one too.
- The arrows and Tab still move only between cards and buttons.
- A started task is under Active alone, not Planned or To refine.
- Waiting tasks are on the Waiting tab only: not on All, and never the peek.
- Finished tasks are on the Finished tab only, the same way. Back to list on
  one reopens it, back on All.
- The panel opens on All every time, and goes back to All when the tab it is
  on runs out of tasks.

#### Clear all

1. It shows only on the Waiting tab, on a strip of its own under the tabs.
   `Ctrl+Shift+Delete` presses it. `Ctrl+Delete` and `Delete` are still the
   focused card's.
2. First press: like Remove, it asks first. It arms as **Confirm clear all**
   and takes the focus off the cards.
   - No card's buttons show.
   - A card's own keys, `Ctrl+Delete` among them, do nothing.
3. Second press, or `Enter`, deletes every waiting task on this workspace,
   one `task status <uuid> deleted --yes` after another. The panel goes back
   to All, keeping the keyboard.
4. `Escape`, an arrow, Tab, or switching tab disarms it instead. The focus
   returns to the card it was on, and the panel stays up.

### Ideas

- The last tab. Not a filter: in place of the cards, one large text area, a
  notepad for this workspace's ideas that are not tasks yet.
- Always there, after Finished. `7` picks it, as `]` does from the last tab
  before it.
- While it is picked every key is typing, the letters, digits and brackets
  included.
- `Escape` saves and goes back to the task list, on the tab you were on
  before Ideas, with the panel still up. A second `Escape` gives the
  keyboard back.
- `Ctrl+[` and `Ctrl+]` switch tab.
- One notepad per workspace tag, plain text in
  `$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md` (under `~/.local/share` without
  it).
- Saved a second after typing stops, and again on leaving Ideas or when the
  panel gives the keyboard back.

### How it looks

- The cards are shaped like mako notifications and take a terminal window's
  font and colours.
- A dark tint over blurred wallpaper, with niri's faint focus-ring outline
  (`src/panel/style.rs` names the configs it copies).
- The blur is asked for by the panel itself, over `ext-background-effect-v1`,
  in the exact shape of the cards.
- A compositor without that protocol shows them unblurred.
- The lock icon comes from a Nerd Font (JetBrainsMono Nerd Font here).
- See `docs/adr/0001-task-panel-in-gtk-not-quickshell.md` for why it is drawn
  the way it is.

## Install

1. Clone and install:

   ```bash
   git clone https://github.com/pauldaywork/niri-tasks ~/Projects/niri-tasks
   cd ~/Projects/niri-tasks
   bash install.sh
   ```

2. Add the include to `~/.config/niri/config.kdl`:

   ```kdl
   include "niri-tasks.kdl"
   ```

To update: `git pull && bash install.sh`.

`install.sh` does this:

- builds `niritasks`;
- symlinks the niri include;
- symlinks the Claude skills (`workspace-tasks`, `refine-task`, `report-task`,
  `finish-worktree`, `reject-worktree`, `private-github-repo`) into
  `~/.claude/skills`;
- symlinks the refine mod (`claude/refine-mod`, which Refine's session writes
  the task with) into `~/.local/share/niri-tasks/refine-mod`;
- symlinks a Claude rule into `~/.claude/rules` (use `finish-worktree` to land
  a worktree branch);
- restarts the daemon onto the new binary;
- reloads niri.

The symlinks make edits in this repo live at once. There is no copy to keep in
sync.

### Requirements

| Needs | For | Without it |
| --- | --- | --- |
| `niri`, `taskwarrior`, a Rust toolchain | Everything | Required |
| `ghostty` 1.2 or later | `+new-window` | Required |
| `notify-send` | Feedback | Feedback degrades to stderr |
| `fzf` | Ranks what you type on the project list | Plain-text matching, and it says so once |
| `tar`, `sqlite3` | `niritasks export` and `import` | — (`sudo apt install sqlite3`) |
| [herdr](https://herdr.dev), optional | `project open` runs the workspace's herdr session in the project terminal | A plain shell in the project folder |
| Claude Code (`claude`, 2.1.287 or later for Refine's mod) + herdr | **Refine**, **Grill me**, **Generate report**, **Explain branch** | — |
| Claude Code alone | **Add & all** (the box) | — |
| `bubblewrap`, `socat` | The Bash sandbox (`sudo apt install bubblewrap socat`) | Claude refuses to start rather than run unfenced |
| [worktrunk](https://worktrunk.dev) (`wt`) + herdr + Claude Code, `~/Projects` a git repo | **Start working** | — |
| herdr 0.9.1+, `python3`, `fzf`, plannotator-tui | **Task docs** | The key says so |
| `claude`, `curl`, `ffplay` (from ffmpeg), a Kokoro server on `127.0.0.1:8880` | **Read aloud** (in a card's Go to list) | — |
| `code` (VS Code), optional | `project open` starts VS Code | Only the terminal starts |

- **Claude Code + herdr:** these open Claude inside the herdr session.
  Add & all needs Claude Code alone, on `$PATH`.
- **`bubblewrap`, `socat`:** the sandbox keeps that Claude to:
  - reading anything but secrets;
  - reaching only a fixed list of source and docs hosts;
  - writing only the task and its own temp folder.
- **Start working:** worktrunk makes the task's worktree and runs the repo's
  `.config/wt.toml` hooks. The first time a repo's hooks need approving, it
  asks in a herdr tab.
- **Task docs:** the herdr plugin `install.sh` links.
  - plannotator-tui comes from herdr-annotate Full
    (`herdr plugin install plannotator/herdr-annotate`).
  - Lite has no plannotator-tui.
- **Read aloud:** if Kokoro is not answering, Read aloud starts it:
  - from the `kokoro-tts` container if there is one;
  - otherwise a new one from `ghcr.io/remsky/kokoro-fastapi-gpu`;
  - that needs `docker`, an NVIDIA GPU and the NVIDIA Container Toolkit;
  - the first start downloads the image;
  - the container is the one
    [herdr-speak](https://github.com/pauldaywork/herdr-speak) uses, so the
    two share one server.
  - Kokoro is [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI).
- **herdr:** with it on `$PATH`, `niritasks project open` runs the workspace's
  herdr session in the project terminal. Without it, the terminal is a plain
  shell in the project folder.
- **VS Code:** `project open` starts VS Code on the project folder when `code`
  is on `$PATH`, else only the terminal.
  - The lookup is there because niri answers a spawn of a missing binary with
    a desktop notification.
  - Without the lookup, it would fire on every project you opened.
- **Tested against:**
  - niri 26.04 and Taskwarrior 3.5.0.
  - It still runs on Taskwarrior 2.6.2, whose data lives in `pending.data`
    and `completed.data` rather than 3.x's `taskchampion.sqlite3`. The panel
    watches both.

- **Pane labels:** a Refine or Start working Claude's pane says which task it
  is on.
  - Border: `Refine #61 · <description>` or `Work #61 · <description>`.
  - Claude's status line, under its prompt:
    `#61 · <description> · <state> · <branch>`, read afresh on each refresh.
  - It replaces any status line of your own in those two Claudes only.
  - herdr draws a border, and the label in it, only around split panes. To
    always draw it, add this to `~/.config/herdr/config.toml` and run
    `herdr server reload-config`:

        [ui]
        pane_borders = "always"

  - niri-tasks never edits that file.
  - The pane also carries `task_id`, `task` and `task_uuid` tokens. An Agent
    row in `[ui.sidebar.agents]` can show them as `$task_id`, `$task` and
    `$task_uuid`.
  - The default sidebar shows none of them.
- **A Claude started by hand** in a herdr pane:
  - is linked to a task when it marks that task active
    (`niritasks task status <uuid> active`);
  - gets the agent name `work-<uuid8>`, the name Start working gives its own;
  - gets the pane label `Work #<id> · <description>`, so the task's Go to
    list can offer Go to work session;
  - keeps its name if Refine or Start working already named the agent.
- **`/finish-worktree`** in that worktree's Claude lands it:
  1. syncs `main` with origin;
  2. rebases the branch and tests the result in a container (`tests/container.sh`,
     every suite, nothing on your desktop touched; `tests/all.sh` on the host
     when the container can't run, and it says so);
  3. fast-forwards `main` and pushes;
  4. marks the task completed with `niritasks task status`;
  5. links any plan or spec doc the branch added that the task doesn't yet
     carry as a `Plan:` or `Spec:` note;
  6. removes the worktree.

  It stops to ask only for a conflict with no obvious resolution, red tests,
  or `main` holding commits origin doesn't have.
- **`/reject-worktree`** in that worktree's Claude throws an attempt away:
  - removes the worktree and its branch, locally and on origin;
  - puts the task back on the list, stopped, to be started fresh;
  - gives the task a `Rejected:` note if you give it a reason;
  - asks first only when there are commits, uncommitted changes or a pushed
    branch to lose.
- **`/private-github-repo`** gives a project with no GitHub repo one:
  - a private repo on your account, named after the folder, set as `origin`
    and pushed;
  - runs `git init` if it has to;
  - asks before making a first commit;
  - stops rather than push a secret, overwrite an existing `origin`, or reuse
    a name already taken on your account.

### Privacy

- Read aloud sends the task's description and notes to Anthropic.
- `claude -p` rewrites them for listening, on Claude Haiku, with no tools and
  no saved session.
- The speech is made locally by Kokoro.
- Nothing is saved, neither the rewrite nor the audio.
- Refine and Start working send the task to Claude too, since a Claude session
  works on it.

### Annotating a task's documents in herdr

`install.sh` links a herdr plugin, `herdr/task-docs`. Bind its action to a
key in `~/.config/herdr/config.toml`, then run `herdr config check` and
`herdr server reload-config`:

```toml
[[keys.command]]
key = "prefix+shift+f"
type = "plugin_action"
command = "task-docs.open"
description = "task docs: annotate one of this task's documents"
```

Press it in the pane of a task's Claude, or any pane whose folder is on the
task's `task/<slug>-<uuid8>` branch. A list opens over the pane:

- the task's refine reports, newest first, earlier drafts marked;
- Markdown files the task's notes name by absolute path, such as a
  handoff, newest note first;
- the task's description and notes, as Markdown;
- the plans in the project's `docs/superpowers/plans` and in
  `~/.claude/plans` changed since the task was added, newest first. A
  project plan counts from its last commit, so a fresh worktree does not
  list every old plan; one that is untracked or edited locally counts from
  its file time.

Enter opens the pick in plannotator-tui, to the right of the task's Claude.
Comment on it there; **Send** makes the comments that Claude's next message.
A report made before reports had a Markdown twin opens in the browser
instead, as a card's Open report does. A pane with no task, or a task with
no Claude running, gets a notification saying so and nothing opens. The list
is `niritasks task docs <uuid>`, so a script can read it too.

Each refine report is written first as `<report>.md`, its Markdown twin,
diagrams kept as Mermaid blocks; its HTML slides are then built from what
that twin reads back as, so the two never disagree. The Markdown is what
plannotator-tui opens.

### Opening the task panel on a herdr pane's task

The same plugin has a second action, `task-docs.panel`. Bind it too, on a
key of its own: not `prefix+shift+t`, which is herdr's rename tab:

```toml
[[keys.command]]
key = "prefix+shift+k"
type = "plugin_action"
command = "task-docs.panel"
description = "task panel: open it on this task's card"
```

Press it in a task's pane, found as for Task docs: the task panel slides
out with the keyboard, on All, the focus on that task's card with its notes
hidden. A task with no card on the focused workspace's panel, one on
another workspace or not pending (waiting or finished, say), opens the panel on its first card
with a notification saying why. A pane with no task gets a notification and
nothing opens. It runs `niritasks task panel <uuid8>`.

### Peeking at the focused pane's task

The plugin also hooks herdr's `pane.focused`, `tab.focused` and
`workspace.focused` events; no key to bind. Selecting a pane, a tab or a
workspace finds the now-focused pane's task, as Task docs does, and that
task's card becomes the peek on its workspace's task panel: the one card the
edge shows. Going to a niri workspace does the same without touching herdr:
the daemon asks that workspace's herdr session for its focused pane each time
a monitor switches to it, and when the daemon starts, so the peek is right
after a restart too. A pane with no task, or a session that is not running,
empties the edge, and so does the task leaving the list (completed, waiting
or removed). Selecting the same pane again changes nothing on the panel. The
hooks run `niritasks task peek --focused` and need herdr 0.9.3. They send no
notification; if the peek does not follow the focus inside herdr,
`herdr plugin log list --plugin task-docs` says why.

## Commands

`niritasks` is usable directly, not just from keybinds. [`llms.txt`](llms.txt)
is the same reference written for agents, with the rules they most often get
wrong.

```
niritasks tag                      # the focused workspace's tag
niritasks tag --session            # the tag of the workspace this terminal was opened on
                                   #   (its herdr session, else its ~/Projects folder)
niritasks task panel [<uuid>]      # hand the task panel the keyboard (Mod+Alt+Ctrl+T); with a uuid or
                                   #   uuid8, focused on that task's card (herdr's task-docs.panel)
niritasks task peek <uuid>         # peek at that task's card on its workspace's panel edge, from a
                                   #   named herdr session's pane
niritasks task peek --clear        # peek at nothing: the edge shows no card
niritasks task peek --focused      # peek at the task of the session's focused pane, or at nothing
                                   #   (herdr's focus hooks run it)
niritasks task status <uuid> <state>  # what a card's Stop, Waiting, Complete and Remove run: active|stopped|waiting|completed|deleted
                                   #   (stopped also brings back a waiting task, and reopens a completed one)
niritasks task status <uuid> deleted --yes  # deleted needs --yes, Remove's confirmation
niritasks task move <uuid> <folder>  # to another ~/Projects folder's workspace, as a card's Move to workspace does
niritasks task add <text>          # honours taskwarrior attributes: due:friday, priority:H
                                   #   to the herdr session's workspace in herdr, else the focused one
                                   #   (task refine, start and session pick their workspace the same way)
niritasks task add                 #   with no text, the task box (Mod+Alt+T)
niritasks task add --refine        # the add box, with Add & refine as its default (Mod+Alt+Shift+T)
                                   #   (with text, adds it and refines it straight away)
niritasks task get-text <uuid>     # print its description
niritasks task edit <uuid> <text>  # replaces the description; attributes stay literal
niritasks task edit <uuid>         #   with no text, the task box on its description and notes
niritasks task get-notes <uuid>    # print its notes, one per line: date, two spaces, text
niritasks task note <uuid> <text>  # attaches an annotation, literal too
niritasks task note <uuid>         #   with no text, the task box with the cursor in a new note
niritasks task speak <uuid>        # read its description and notes aloud, rewritten by Claude for listening;
                                   #   run again, or on any task while one is speaking, to stop
niritasks task speak <uuid> --here # internal: the speaking itself, run in the background by `task speak`
niritasks task status-line <uuid>  # internal: the status line Refine's and Start working's Claude show,
                                   #   #id · description · state · branch, run by Claude Code on each refresh
niritasks task up-next <uuid>      # mark it up next: its card turns yellow and sits under the active tasks;
                                   #   run again to clear it
niritasks task refine <uuid>       # work it up into a plan with Claude, in the workspace's herdr session
niritasks task refine <uuid> --grill  #   the same, interviewing you first
niritasks task refine <uuid> --unattended  # headless: Claude plans it asking nothing, then builds and links its
                                   #   report, in the background; the card shows an hourglass meanwhile
niritasks task report <uuid>       # open the report of its plan linked from its notes; without one, a report
                                   #   built by Claude in a Report: tab of its own in the workspace's herdr session
                                   #   (beside any refine tab), opened in the browser and linked from the task's
                                   #   notes (a planned task only)
niritasks task report <uuid> --fresh  #   build a new report even when its notes link one
niritasks task report <uuid> --branch  # open the branch report linked from its notes; without one, Claude explains what its worktree branch has changed, opens the report and links it; first, a Tests: tab runs the worktree's tests outside Claude's sandbox (git config niri-tasks.test, else NIRITASKS_HEADLESS=1 bash tests/all.sh) and Claude reports on that run
niritasks task report <uuid> --branch --fresh  #   explain the branch afresh even when its notes link a branch report
niritasks task report <uuid> --branch --here --workspace <name>
                                   # internal: the tests step, run inside the Tests: tab `task report --branch` opens
niritasks task docs <uuid> --project <dir> --notes-dir <dir>
                                   # list its documents, label<TAB>path: its reports (the Markdown twin, else
                                   #   the HTML), Markdown its notes name by path, its notes written to
                                   #   --notes-dir as Markdown, and the plans
                                   #   in <dir>/docs/superpowers/plans and ~/.claude/plans since it was added
niritasks task start <uuid>        # its own worktree (task/<slug>-<uuid8>), opened in the herdr session,
                                   #   with Claude planning it; picked again, back to both
niritasks task start <uuid> --here --workspace <name>
                                   # internal: the setup step, run inside the tab `task start` opens
niritasks task session <uuid>      # back to the Claude working on it (work-/task-<uuid8>) in the herdr session
niritasks task session <uuid> --work    #   its working Claude alone (work-<uuid8>): the Go to list's Go to work session
niritasks task session <uuid> --refine  #   its refine Claude alone (task-<uuid8>): Go to refine session
niritasks task open <uuid> spec    # open the design spec its notes link (Spec: <path>) in a VS Code of its own, read in its
                                   #   worktree while it has one, else the project folder
niritasks task open <uuid> plan    #   the same for its plan (Plan: <path>)
niritasks task open <uuid> worktree  # a terminal in its worktree
niritasks task open <uuid> editor  # VS Code on its worktree, else the project folder
niritasks task open-report <path>  # a report just written, in a new browser window on this workspace (what refine runs)
niritasks project open             # the panel's project list: pick a ~/Projects folder or GitHub repo
                                   #   onto its own named workspace, or type a new name (Mod+Alt+W)
niritasks project open <name>      # open that one, cloning or making it first: what a pick runs
niritasks export [<path>]          # every task, any status, and the reviews folder to one .tar.gz, by default
                                   #   ~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz; lists report notes whose file is gone
niritasks import <archive>         # put an export back, into an empty store (see Backing up before a reinstall)
niritasks import <archive> --force #   into a store that already holds tasks; a task in both takes the backup's copy
niritasks terminal                 # a terminal in the focused workspace's ~/Projects folder (Mod+Return)
niritasks daemon                   # internal: the task panels and task-box server, run by the
                                   #   niri-tasks systemd user unit
```

Every `<uuid>` is the full uuid or its first 8 characters, never a task's
number: numbers are renumbered as tasks complete, so a stale one points at
another task. `niritasks <command> --help` says the same as each line here, at
more length.

### Two rules that look alike and are not

`niritasks task add` **word-splits** the description, so taskwarrior parses its own
attribute syntax — `niritasks task add ship it due:friday` sets a due date.

`niritasks task edit` and `niritasks task note` **do not** split. The text goes through as one
argument, so a typed `due:` stays literal text.

Both are covered by tests in `tests/write_path.rs`, against a sandboxed task
database. Getting them backwards fails silently, which is why they are pinned.

The task box follows the same split. Adding word-splits the description, so
`due:friday` in it is a due date; editing an existing task in the box keeps it
literal.

### The task box

One window adds a task and edits one. Edit opens it on the task's description
and every note, and `niritasks task note <uuid>` opens it the same way with the cursor in a
new empty row at the end. Each note is a row: it wraps, you edit it in place,
its date sits small at its right end, and × deletes it. "+ Add note" appends a
row. Adding has two more buttons. **Add & refine** adds the task and hands it
straight to Claude's refine-task skill, in a new tab of the workspace's herdr
session, as a card's Refine does. **Add & all** adds it and plans it in the
background instead: `niritasks task refine <uuid> --unattended` runs Claude
headless in the project folder, behind the same fence as a refine tab, first
to write the plan asking nothing (each question it would have asked is
decided with its recommended answer and recorded as a `Decided:` note), then
to build the report and link it from the task's notes as `Report: <path>`,
without opening it. The card is dimmed with an hourglass meanwhile and offers
nothing; a notification says `Planned and reported` when both are done, or
which step failed and where its log is
(`$XDG_DATA_HOME/niri-tasks/unattended/`, by default
`~/.local/share/niri-tasks/unattended/`).
The log holds each run's final answer and every action Claude was refused,
if any, and the notification counts the refusals and gives the log's path.
Add & all has no key and needs Claude Code but not herdr. If a refine fails,
either way the task stays added and a notification says why.

There is only ever one box. Mod+Alt+T, or Edit or Note on a card, while a box
is open brings that box forward and opens nothing, so nothing typed into it is
lost; if you asked for a different box, a notification says one is already
open. Save or discard it, then ask again.

A box opened by a card's Edit hands the keyboard back to the task panel
when it closes, however it closes, on the tab Edit was pressed on with
that card focused (or the tab's first card, if that one has gone). A box
opened any other way, by Mod+Alt+T or `niritasks task edit`, leaves the
panel tucked away, as does taking or giving back the panel's keyboard
while the box is open.

| Key | Does |
|---|---|
| Enter in the description | Moves to the first note (making one if there are none) |
| Enter in a note | Adds a row below and moves to it |
| Backspace in an empty row | Deletes it and moves up |
| Ctrl+Enter | Saves, from anywhere — Add & refine in the box Mod+Alt+Shift+T opens |
| Ctrl+Shift+Enter | Add & refine, when adding |
| Esc | Discards everything, without asking |

Saving an existing task writes its description and whole note list back in
one `task import`. Untouched and edited notes keep the date they were first
written; new ones are dated when saved, and a save that changed nothing writes
nothing. Taskwarrior keeps notes in date order, so a note typed between two
old ones moves to the end once saved. Empty rows are dropped.

## Backing up before a reinstall

Before a clean install:

1. **Push every task worktree branch.** A task's `Plan:` and `Spec:` notes
   name files in its repo's `docs/superpowers/`, and until the branch lands
   they live only on that branch. In each repo,
   `git branch --list 'task/*'` lists them; `git push origin <branch>` each
   one. `export` does not do this for you.
2. **Run `niritasks export`.** It writes every task, whatever its status,
   to `~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz` (or the path you give;
   it refuses one already there), and prints the path and how many tasks
   it holds. The archive holds `tasks.json`, the store as `task export`
   writes it; `taskchampion.sqlite3`, a copy of the store taken through
   SQLite's own backup; and `reviews/`, the refine reports folder, when
   the folder exists. It then lists each `Report:` or
   `Report (earlier draft):` note whose file is already gone, or is outside
   the reviews folder and so not in the archive — the task's uuid8, status
   and path. Those notes will dangle after the restore whatever you do;
   the backup goes ahead without them.
3. **Copy the archive off the machine.**

After reinstalling Taskwarrior 3.5 and niri-tasks:

1. **Give `task` an rc file.** A fresh install has no `~/.taskrc`, which is
   not backed up, and `task` refuses to run without one ("Cannot proceed
   without rc file."). Restore yours, or run `task` once interactively and
   let it create one; either way `data.location` must point where you want
   the store.
2. **Run `niritasks import <archive>`.** It imports `tasks.json` with hooks
   off, copies `reviews/` back into the reviews folder
   (`~/.local/share/niri-tasks/reviews`, or under `$XDG_DATA_HOME`)
   without overwriting any file already there, and prints how many tasks it
   imported. It refuses a store that already holds tasks (any status,
   deleted ones included), so an old backup cannot undo newer edits;
   `--force` imports anyway, and a task in both takes the backup's copy. If
   it stops after the tasks are in, say while copying reports, rerunning
   with `--force` from the same backup is safe.

If `tasks.json` will not import, restore the SQLite copy by hand, once
`task` has an rc file as above, into a store with nothing in it and no
`task` running. A reinstalled `task` may already have made an empty
`taskchampion.sqlite3` with `-wal` and `-shm` files beside it, so remove
those first:

```bash
tar -xzf niri-tasks-backup-<stamp>.tar.gz taskchampion.sqlite3
mkdir -p "$(task _get rc.data.location)"
rm -f "$(task _get rc.data.location)"/taskchampion.sqlite3*
cp taskchampion.sqlite3 "$(task _get rc.data.location)/"
task count                      # every task, back
```

Not backed up: `~/.taskrc`, and the rest of `~/.local/share/niri-tasks`
(unattended logs, ideas, the VS Code profile).

## Notes

`~/.taskrc` is not managed here. The only requirement is that
`data.location` points somewhere `task` can read and write.

The task panel's blur rule is in `niri-tasks.kdl`: it matches
the panel's own namespace, `niri-tasks-panel`, and sets how the blur looks but
never `blur true`. A rule's blur would fill the panel's whole surface, which is
wider than the cards so they have room to slide. The panel asks for blur behind
the cards alone instead, and niri takes noise and saturation from the rule.

## Development

### Running the tests

```bash
bash tests/all.sh           # everything this machine can run       ~100s
bash tests/container.sh     # everything, in a Docker container     ~460s

cargo test                  # unit and write-path                   ~2s
python3 -m unittest discover -s herdr/task-docs/tests -t herdr/task-docs  # the herdr plugin  ~1s
claude plugin test claude/refine-mod  # the refine mod (needs claude)
bash tests/e2e-tag.sh       # the session's tag, against real niri    ~1s
bash tests/e2e-panel.sh     # the task panel, in a nested niri       ~30s
bash tests/e2e-box.sh       # the task box, in a nested niri         ~35s
```

Each exits non-zero on failure, so any of them can go in a loop or a hook.

`tests/all.sh` is the lot in one command, run cheapest-and-quietest first. It
checks each suite's prerequisites itself and reports one it cannot run as a
**skip, with the reason** — no niri, no Pillow, no Wayland display — rather than
letting it fail. A skip is not a failure: it exits non-zero only when a suite
that actually ran said no, which is what makes it safe on a machine that can
only run half of it. None of the three e2e suites takes the machine over: the panel and box tests
run in a nested niri parked on a spare workspace, and `e2e-tag.sh` at most names
a spare workspace for a moment; `all.sh` tells you which one to keep off.
`NIRITASKS_HEADLESS=1 bash tests/all.sh` skips the three e2e suites and runs the
rest, including the refine mod's tests (`claude plugin test claude/refine-mod`).

| | Needs | Touches |
|---|---|---|
| `tests/all.sh` | Nothing of its own — whatever is missing is skipped and named | Whatever the suites it ends up running touch |
| `tests/container.sh` | Docker, and the user in the `docker` group. The first run builds the image (several minutes, and again when one of its inputs changes) and needs the network; runs do not | Nothing of yours. Docker's own images and volumes (`niritasks-tests*`, pruned as they go stale), and its logs in `target/container-tests/` |
| `cargo test` | `taskwarrior` on `$PATH` | Nothing. The write-path suite points `TASKDATA` at a scratch directory |
| `herdr/task-docs` tests | `python3`, `git` | Nothing. A fake `herdr`, `notify-send` and `xdg-open` stand in, in a scratch folder |
| `claude plugin test claude/refine-mod` | `claude` on `$PATH` | Nothing of yours. The mod's tests run against a fake session |
| `tests/e2e-tag.sh` | niri running with **two named workspaces** — one focused, one not (it borrows the spare empty one if not) | At most the name of that spare workspace, taken off again. No herdr session (a fake `herdr` stands in), and only a scratch task database of its own |
| `tests/e2e-panel.sh` | niri, a Wayland session, `python3-pil`, taskwarrior; `wtype` for its keyboard checks, which are skipped without it | A nested niri window for the run, parked on the spare workspace at the end of your monitor; keep off that workspace until it finishes. Not your clipboard, your screenshots or the `niri-tasks` daemon |
| `tests/e2e-box.sh` | `wtype`, niri, a Wayland session, taskwarrior | A nested niri window for the run, parked like the panel test's; keep off that workspace until it finishes. Its keys go only to that nested niri. Not your keyboard or the `niri-tasks` daemon |

Narrowing `cargo test` works as usual — `cargo test --lib`, `cargo test --test
write_path`, `cargo test tag::` for one module, `-- --nocapture` to see output.

`e2e-panel.sh` runs on a screen of its own: a nested niri, started with its own
runtime directory and a flat 1600x1000 output, with its own daemon against a
sandboxed `TASKDATA`. Your wallpaper, windows, window rules, notifications and
pointer cannot reach its frames, and your `niri-tasks` daemon keeps running
throughout. The nested niri's window opens over yours for a moment, then is
parked floating and unfocused on the last workspace of that monitor. Keep off
that workspace while it runs: going there focuses the window, and the run
fails, saying so, rather than measuring frames your typing could have reached.
Run it with `NIRITASKS=$PWD/target/debug/niritasks`: without that it runs the
installed `niritasks`, which may be an older build.
`NIRITASKS_E2E_KEEP=1` leaves the frames on disk when you need to see what a
failure actually looked like.

It cannot move the pointer, so the hover is checked by hand after a change to
`src/panel/surface.rs`, with a task peeked (select its herdr pane, or run
`niritasks task peek <uuid8>` from a pane of its workspace's session): the peek
slides out when the pointer reaches it and back about 0.4s after it leaves, the
slide is smooth, its notes and buttons show, a click just left of the peek lands
on the window beneath, a click on its description does nothing, Complete
completes it and empties the edge without taking the keyboard from herdr,
Remove asks once and the pointer leaving puts it back, and Go to… slides the
panel to the middle with the keyboard, on that card's Go to list. And the
keyboard, past what the script measures: `Mod+Alt+Ctrl+T` slides the panel from the right edge to the
middle of the screen, smoothly, with every card wrapped and the first
darkened. The pointer passing over it does not move it. Check also active tasks with Stop and no
Start working; Up and Down move the darkening between cards, Left, Right and
Tab along the buttons; `s` starts working,
`e` opens the box, `t` stops; Delete arms Remove and only a second
Delete deletes, while moving away disarms it; Enter on a card's description shows its notes, dimmed under it, with the panel and its blur grown to fit, and a second Enter hides them; a card with more notes than fit stops at about twelve lines and scrolls the rest by wheel and by dragging the scrollbar, neither of which hides them, while a click on the notes does, `c` completes the task, `m` shows the project list and Escape comes back from it while Enter on a folder moves the task there, `project open` on a workspace with no tasks shows the project list and Escape closes it,
Down reaching "+N more" shows the rest, focused on the first card it hid, and a
list taller than the screen scrolls with the focus; a click on a filter tab switches the cards and does not take the focus (the focus still falls back to the first card when the focused one is not under the new tab), a started planned task shows under Active but not Planned, and Back to list on a waiting task brings it back to All; a click on Clear all arms it, darkening no card, a second click deletes the waiting tasks, and Up or Down in between puts it back with the focus on the card it was on; arming Clear all puts an armed Remove back, and arming Remove puts Clear all back; Escape tucks it away, to the peeked card if there is one.

`e2e-box.sh` runs in the same kind of nested niri (`tests/lib/nested-niri.sh`
starts it for both). Its keys are pressed with `wtype` pointed at that nested
niri, so they never reach your windows, and you can keep working while it
runs — keep off the workspace it is parked on, as for the panel test. The
box is served by a daemon started inside the nested niri; once that is
stopped, it checks the CLI refuses with "daemon is not running" rather than
drawing a box of its own. Your own `niri-tasks` daemon is never stopped.
Run from a herdr pane, both nested tests drop every `HERDR_*` variable before
starting the nested niri, so they behave as from a plain terminal and never
touch your herdr session or pane. Either way, the daemon and the commands they
run see a `$PATH` with every directory holding a `herdr` taken out, and the run
stops if `herdr` can still be found, so nothing they start reaches your herdr.

It cannot click, so after a change to the task box (`src/taskbox.rs` or `src/taskbox/`) check the pointer half by
hand: × deletes its row, "+ Add note" appends an empty row with the cursor in
it, and a long note list scrolls, keeping a row made by Enter at the bottom in
view.

> [!IMPORTANT]
> All three scripts run `niritasks` **from `$PATH`** — the installed binary, not the one you
> just built. A green run after an edit you have not installed is testing the
> old code. Point them at a build with `NIRITASKS=`:
>
> ```bash
> cargo build --release
> NIRITASKS=./target/release/niritasks bash tests/e2e-box.sh
> ```
>
> To try a change by hand rather than under test, use `bash try.sh` (below),
> not `install.sh` or `cargo install`: both replace the `niritasks` your
> daemon and every other worktree's test runs use.

### Running the tests in a container

`tests/container.sh` runs `tests/all.sh` in a Docker container, against this
checkout as it stands, uncommitted edits included. Nothing of your desktop is
in reach: the container gets no mount, socket or display of yours and no
network, only a copy of the files git would see, so your niri, workspaces,
task database and installed `niritasks` are never touched, and there is no
workspace to keep off. Run it from any worktree, beside other worktrees' runs,
in the foreground or in the background:

```bash
bash tests/container.sh                          # tests/all.sh
bash tests/container.sh bash tests/e2e-box.sh    # one suite
bash tests/container.sh >/dev/null 2>&1 &        # in the background
tail -f target/container-tests/latest.log
```

The image (`tests/container/Dockerfile`) reproduces what the suites use here,
at this machine's versions and in the same folders: niri and ghostty from the
same PPA, Taskwarrior 3.5.0 compiled from its tarball, rustc 1.95.0, herdr,
worktrunk and Claude Code in `~/.local/bin`, Iosevka Term (in
`~/.local/share/fonts`, as here, because the panel test's own
`XDG_DATA_HOME` hides that folder and its results depend on the font), the GTK
theme and font settings, `en_US.UTF-8`, your timezone and a uid-1000 user. Each
run makes the links `install.sh` makes, from the copy. VS Code and gh are left
out, and Claude Code is never logged in.
`docker run --rm -i "$(bash tests/container/image.sh)" bash -s < tests/container/check-image.sh`
lists what the image holds. Inside, `tests/container/inside.sh` starts a
headless sway, an outer niri in it with two named workspaces, builds the copy
and runs `tests/all.sh` with `NIRITASKS` set to that build; the e2e suites
nest their niri in the outer one as they would in yours. Since GTK on Wayland
reads no GSettings without a desktop portal, `inside.sh` also writes a GTK
`settings.ini` from the image's gsettings.

The image is built on the first run, and again only when one of its four
inputs changes: the Dockerfile, its GTK settings keyfile
(`tests/container/dconf/00-machine`), `Cargo.toml` or `Cargo.lock`
(`tests/container/image.sh`). It holds
the crate's dependencies already compiled, and each checkout's build directory
is a Docker volume filled from them, so a checkout's first run compiles only
niri-tasks and later runs only what changed. Each run starts by saying
whether the image still matches this machine — package versions, programs,
font, GTK settings (`tests/container/parity.sh`); when it does not, rebuild
with `NIRITASKS_CONTAINER_REBUILD=1 bash tests/container.sh`. A second run of
the same checkout while one is going refuses, since the two would share a
build directory. Every run is logged in `target/container-tests/`,
`latest.log` the newest; a failed run's logs and the e2e suites' frames are
kept beside it in `<stamp>-artifacts/`. It exits non-zero when a suite that
ran fails, as `tests/all.sh` does, or when the run cannot get that far: no
Docker, a failed image build, a refused run, or the container's desktop or
build failing.
Each run removes the build volumes nothing will use again — those of
worktrees since removed, and of images since replaced; old images themselves
go only when a new one is built, which keeps the three newest. To reclaim everything:

```bash
docker volume rm $(docker volume ls -q --filter label=niritasks.tests=1)
docker image rm $(docker image ls -q niritasks-tests)
```

### Trying a change by hand

```bash
bash try.sh             # a debug build
bash try.sh --release   # a release build, to see how the panel animates
```

Run from any checkout or worktree, `try.sh` builds it and opens it in a
nested niri — the same sandbox the e2e tests use (`tests/lib/nested-niri.sh`),
but as an ordinary window, focused, on your workspace. Its one workspace is
named after the project, so the panel shows the project's tasks from a copy
of your task database, taken with an SQLite online backup; nothing is written
back. `niri/niri-tasks.kdl` is included with Mod on plain Alt, since your own
niri takes Super combinations first: Alt+T adds a task, Alt+Shift+T adds and
refines, Alt+Ctrl+T shows the panel, Alt+W opens a project and Alt+Return
opens a terminal (stubbed in a try window). Notifications are printed in the terminal
rather than shown on your desktop.

The terminal becomes a try shell whose `niritasks`, `task` and `niri msg`
reach the nested niri. `reload` rebuilds and restarts the nested daemon,
keeping the window and the copy; `exit` closes everything. The installed
binary, the `niri-tasks` service and `install.sh`'s links are never touched,
so tries in several worktrees run beside each other and beside `tests/all.sh`.
herdr is kept out, so Start working and the Go to list's sessions fail there, with the
reason printed in the try shell. Terminals, editors and clones are not opened
either: `ghostty`, `code` and `gh` are stubs that print what would have run
(`[notify] would run: …`), because ghostty and VS Code would otherwise hand
off to your running instances and `gh` would clone into your real
`~/Projects`. The `gh` stub fails, so a clone says it could not and the
project list keeps the GitHub repos it had, from a copy of your
`~/.cache/niritasks` that the try build writes instead of yours. Alt+W can
still make a folder under `~/Projects`, for a new name typed there.
Installing a build for daily use is still `bash install.sh`, on main.

### Why the three scripts are not cargo tests

`tests/e2e-box.sh` is not a cargo test and cannot be: it needs a running niri, a
Wayland display, and `wtype` to press the keys. It opens the box, types into it,
presses Ctrl+Enter, and checks a task was actually written — against a sandboxed
`TASKDATA`, so your real database is untouched. It runs inside a nested niri
of its own, served by a daemon started there, because the daemon is the one
process that draws.

It exists because two bugs reached daily use that no unit test could have caught.
The box opened carrying the daemon's `app_id` instead of its own, so every check
matching on app-id looked straight past it. And Ctrl+Enter did nothing: the key
mapping was correct and tested, but the event controller was in the wrong
propagation phase, so the focused text view swallowed Return before the window
saw it. Both only exist once a compositor is involved.

`tests/e2e-tag.sh` is a script for the same reason: it needs niri to ask for
workspaces. It pins the property `--session` exists for, which no unit test can
see — that the tag follows the terminal rather than the focus. It hands the
binary a session the way herdr hands one to a pane (`HERDR_SESSION`, with
`HERDR_SOCKET_PATH` as the backup) rather than starting herdr, so none of your
sessions is attached to or created; the folder cases run under a throwaway
`$HOME`. It also checks that `niritasks task add` files under the session's
workspace from a herdr pane and under the focused one outside herdr, into a
scratch task database it deletes afterwards; your own is never touched. And
that `task start`, `task refine` and `task session` open in the same
workspace's herdr session, against a fake `herdr` that logs which session it
was asked for and opens nothing.

`tests/e2e-panel.sh` is the third, and the least obvious. The task panel is a
layer-shell surface, so its behaviour is what the compositor puts on screen: the
window can only report the size it *asked* for, and a surface that was never
mapped reports nothing wrong while showing nothing — the overlay the panel
replaced was invisible for whole sessions that way, past every unit test. So it
screenshots a nested niri of its own and measures the panel against a frame taken
with no tasks: nothing on the edge until a task is peeked, the peek's width and
its notes, an empty edge for another tag's task, a task leaving pending and
`--clear`, a restarted daemon and a workspace switch peeking at a stub herdr's
focused pane, and `--focused`, the
keyboard's cards in the middle of the screen and the focused one's buttons, a
filter tab's fill, a waiting task on the Waiting tab and off the tucked panel,
Clear all taking the focus off the cards on its first Ctrl+Shift+Delete and Enter then
deleting both waiting tasks and nothing else, Escape cancelling Clear all,
Ctrl+Enter's refine reaching `task refine`, "+N more" opening on Down, Escape
putting them back, nothing for another tag's task, nothing once they are done,
and a daemon cold-started with nothing to show. Which tab, card and button a key
leads to, and what it arms, are `src/panel/state.rs`'s unit tests, which need no
display.

The nested screen is a flat colour at a fixed size, so its numbers are exact:
the peek, with a task peeked, starts at column 1566 of 1600, the keyboard's cards span 420-1180, and
a frame that should not have changed — another tag's task, Escape, an empty
panel — is compared pixel for pixel. A nested niri parked out of sight draws
only when asked, so each frame is shot until two in a row agree. It counts
lines rather than pixels: the cards' shadows fade into the background over
many pixels, and a column or row counts only once at least 20 of its pixels
changed, which is what puts the panel's edge at the same column every run.
