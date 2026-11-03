# Make the README Short and Simple Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `README.md` (854 lines of long paragraphs) into short bullets, tables and numbered steps, without losing a single fact.

**Architecture:** One file changes: `README.md`. Rewrite one section per task, top to bottom. After each task, run a shape checker (a throwaway script under the git-ignored `target/`), audit the facts against the base commit, run `cargo test` and commit.

**Tech Stack:** Markdown; Python 3 for the shape check; `cargo test` for the README Commands-block test.

**Spec:** Taskwarrior task `e0d8a467` (`task rc.json.array=on e0d8a467-970f-4a2c-bf57-4b333e901a5c export`). Its notes are the spec. They are copied into Global Constraints below.

## Global Constraints

- Every fact in the old README must still be in the new one. The base to check against is commit `cca971a`: `git show cca971a:README.md`.
- Keep everything in `README.md`. Don't move any section into another doc.
- Keep the existing section order and every existing heading's text. The anchor `#peeking-at-the-focused-panes-task` and the code comments that name sections (for example "Running the tests in a container") depend on them. You may add new `###`/`####` subheadings inside a section.
- No paragraph runs past ~3 lines and no table cell past ~2 lines, at 80 columns. A code-block line may be at most 110 characters. `python3 target/readme-shape.py` checks all three.
- `cargo test` passes. `readme_commands_block_runs_every_subcommand_and_flag` in `src/main.rs` needs a `## Commands` heading followed by a fenced block. In that block, every subcommand and long flag must appear on the **same line** as its `niritasks <subcommand>`. Wrap comments only, never the command.
- `llms.txt:74` must stay true: the README covers "install, keybinds, the task panel and task box, requirements, and how to run the tests".
- Style (from the i-have-adhd rules): action first, numbered steps for procedures, one idea per line, no tangents. Cut repeated wording, never facts.
- Out of scope: behaviour changes, `CONTEXT.md`, `llms.txt`, `docs/`.
- Commit subjects: `docs(readme): <imperative summary>`, at most 72 characters, ending with the `Co-Authored-By` trailer the session gives.

## Shared tools (used by every task)

**Shape checker.** Task 1 creates `target/readme-shape.py`. `target/` is git-ignored, so the file is never committed. If it is missing, recreate it from Task 1, Step 1.

**Fact audit.** For the section a task rewrites, print the old text with `git show cca971a:README.md | sed -n '<from>,<to>p'`. (Line numbers in this plan are always `cca971a`'s.) Go through it sentence by sentence and name the new README line that carries each fact. If no line does, put the fact back. A fact that moves to another section, as listed in a task, counts as kept once it lands there.

**Baseline.** At `cca971a`, `python3 target/readme-shape.py` reports 86 problems and `cargo test` passes.

## Style exemplar

Every task follows this before/after pattern, which is Task 1's actual output:

Before (old lines 5-8):

```
The niri workspace you are looking at *is* the task filter. Name a workspace
`website` and every task you add from it is tagged `+website`; the list
shortcut shows that workspace's tasks and nothing else. Switch workspace and the
same keys show a different set.
```

After:

```
The niri workspace you are looking at *is* the task filter:

- Name a workspace `website`, and every task you add from it is tagged `+website`.
- The list shortcut shows that workspace's tasks and nothing else.
- Switch workspace, and the same keys show a different set.
```

Patterns to use:

- A paragraph that lists things becomes a lead line plus bullets.
- A table cell that runs long keeps a one-line summary. The rest goes into bullets under the table, each led by the row's name in bold.
- A procedure becomes a numbered list.
- A "needs X, Y and Z" sentence becomes a table or bullets: one requirement each.
- A manual checklist becomes plain bullets (not `- [ ]`), one check each.

---

### Task 1: Intro and Keybinds

**Files:**
- Create: `target/readme-shape.py` (git-ignored, not committed)
- Modify: `README.md` old lines 1-22 (`# niri-tasks` intro and `## Keybinds`)
- Modify: `README.md` `## Task panel`. Add one sentence (a fact moved out of the keybind table).

**Interfaces:**
- Produces: `target/readme-shape.py`, run as `python3 target/readme-shape.py [README.md]`. It prints `README shape OK` and exits 0, or prints one `<line>: <problem>` per offence and exits 1.

- [ ] **Step 1: Create the shape checker**

```bash
mkdir -p target
cat > target/readme-shape.py <<'EOF'
#!/usr/bin/env python3
"""Flag README prose blocks over ~3 lines and table cells over ~2 (80 cols a line)."""
import re, sys

WIDTH, PROSE_MAX, CELL_MAX, CODE_MAX = 80, 3, 2, 110
lines = open(sys.argv[1] if len(sys.argv) > 1 else "README.md").read().split("\n")
bad, block, start, fence = [], [], 0, False

def flush():
    if block:
        text = " ".join(l.strip() for l in block)
        n = -(-len(text) // WIDTH)
        if n > PROSE_MAX:
            bad.append(f"{start}: prose ~{n} lines: {text[:60]}…")

for i, line in enumerate(lines, 1):
    s = line.strip()
    if s.startswith("```"):
        flush(); block = []; fence = not fence; continue
    if fence:
        if len(line) > CODE_MAX:
            bad.append(f"{i}: code line {len(line)} chars")
        continue
    if s.startswith("|"):
        flush(); block = []
        for cell in s.strip("|").split("|"):
            n = -(-len(cell.strip()) // WIDTH)
            if n > CELL_MAX:
                bad.append(f"{i}: table cell ~{n} lines: {cell.strip()[:60]}…")
        continue
    item = re.match(r"(>\s*)?([-*]|\d+\.)\s", s)
    if not s or s.startswith("#") or item or line.startswith("    ") and not block:
        flush(); block = []
        if item:
            block, start = [s], i
        continue
    if not block:
        start = i
    block.append(s.lstrip("> "))
flush()
print("\n".join(bad) or "README shape OK")
sys.exit(1 if bad else 0)
EOF
python3 target/readme-shape.py | head -5
```

Expected: the first offences are `5: prose ~4 lines`, `10: prose ~5 lines`, `20: table cell ~3 lines`, `21: table cell ~16 lines`, `22: table cell ~6 lines`. The command exits 1.

- [ ] **Step 2: Rewrite the intro (old lines 3-13)**

Replace old lines 5-13 with:

```markdown
The niri workspace you are looking at *is* the task filter:

- Name a workspace `website`, and every task you add from it is tagged `+website`.
- The list shortcut shows that workspace's tasks and nothing else.
- Switch workspace, and the same keys show a different set.

Which tasks you see goes purely off the workspace name:

- `niritasks project open` sets it to the folder name.
- That is the only way a workspace gets a name, bar workspace 1, which starts as `general`.
- `~/Projects` is only ever read to offer folders to pick from: opening one, or moving a task to another workspace.
```

- [ ] **Step 3: Rewrite Keybinds (old lines 15-22)**

Replace the table with this table and the bullets that follow it:

```markdown
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
- The focused card has buttons: Go to… (refine it into a plan with Claude or be grilled about it, its report, plan and spec, its Claude sessions, its worktree, VS Code, read it aloud), start working in its own worktree, edit, complete, move to another project's workspace, stop, park as waiting, remove.
- Ctrl+Enter refines the focused task, or starts it once it is planned, and keeps the list up. On a task being worked with a Claude on it, it goes to that session.
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
```

Keep each bullet to one sentence. The checker allows up to ~3 lines (240 characters).

- [ ] **Step 4: Move the one fact Task panel lacks**

Old line 21's "With no tasks at all, finished ones included, or no name, a notification says so instead" appears nowhere else. Add it to `## Task panel` as its own paragraph, straight after the paragraph that starts "`Mod+Alt+Ctrl+T` hands the panel the keyboard" (old 51-54) and before the buttons table. Task 2 folds it into that subsection:

```markdown
With no tasks at all, finished ones included, or no workspace name, a notification says so instead.
```

- [ ] **Step 5: Check shape, facts and tests**

Run: `python3 target/readme-shape.py | awk -F: '$1<=60'`
Expected: no line numbers inside the intro or Keybinds section are flagged.

Fact audit: `git show cca971a:README.md | sed -n '1,22p'`. Every sentence must map to a new line.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`
Expected: every `test result: ok`, no `FAILED`.

- [ ] **Step 6: Commit**

```bash
git add README.md
git commit -m "docs(readme): break the intro and keybinds into short lists"
```

---

### Task 2: Task panel, part 1: cards, peek, buttons, keys

**Files:**
- Modify: `README.md` `## Task panel`, old lines 24-81 and 124-137

**Interfaces:**
- Consumes: `target/readme-shape.py` (Task 1)

- [ ] **Step 1: Add subheadings and rewrite the cards paragraph (old 26-35)**

Under `## Task panel`, add `### Cards`. Turn old 26-35 into:

```markdown
Each monitor's task panel holds the pending tasks of the workspace it is displaying, as a stack of cards.

Order:

1. The active tasks (there can be several, one per worktree being worked): ▶ and text in green.
2. Any up next.
3. The rest by priority (H, M, L, none), newest first.
4. The blocked ones last.

Icons:

- ○ for a plain task.
- A dimmed lock for one blocked on another task.
- ● for one already worked up into a plan with Claude, from a card's Go to list's **Refine** or **Grill me**.
- A report icon for a planned one whose notes link a refine report, which its Go to list's **Open report** opens.

Up to eight cards show, then a "+N more" card. Each card ends in how long ago its task was added: 5m, 3h, 2d, 4w.
```

- [ ] **Step 2: Rewrite the peek (old 37-49)**

Add `### The peek on the right edge`. Turn it into bullets, one fact each:

- the panel shows one card at most: the task of the herdr pane you have selected, on that workspace's monitor (keep the link to [Peeking at the focused pane's task](#peeking-at-the-focused-panes-task));
- only 30px shows;
- the pointer on it slides it out to full width, whole: description, id and notes, buttons; the buttons work by click while herdr keeps the keyboard; moving away slides it back;
- Go to… and Move to workspace open lists the keyboard drives, so a click on either first hands the panel the keyboard on that card;
- the space around the card is click-through, so the panel never gets in the way of the windows beneath; a fullscreen window covers it;
- with no task's pane selected, nothing shows;
- the rest of the workspace's tasks are on the keyboard's panel (below), whose "+N more" card shows any past the first eight.

- [ ] **Step 3: Rewrite the keyboard intro and buttons table (old 51-68)**

Add `### With the keyboard (Mod+Alt+Ctrl+T)`. Old 51-54 become three bullets: it moves to the middle of the screen, leaving no peek on the right edge; every card opens to its whole description, wrapped; the focused card shows a row of buttons. Then add Task 1's "no tasks… notification" line.

Cut each Does cell to ~2 lines. Move the overflow into bullets under the table, one `**Button**` lead per item:

| Old cell | Keep in cell | Move under the table |
|---|---|---|
| Go to… (58) | "A list of what else can be done with the task and where it can be gone to. First on every card" | drop the row list: the Go to… table further down (old 97-109) already names every row (Refine/Redo refine, Grill me, Get started/Back to worktree, Generate/Regenerate report, Explain branch, Open report, Branch report, Planning doc, Design spec, Go to refine/work session, Open worktree folder, Open in VS Code, Read aloud/Stop reading aloud) |
| Start working (59) | "The task's own worktree, herdr tab and Claude. Not on an active task" | Claude plans the task first and links the plan it saves as a `Plan: docs/superpowers/plans/<file>.md` note; an active task is already being worked |
| Up next (61) | "Mark it as the one to do next. Not on a waiting or finished task" | card turns yellow and moves up under the active tasks; on a task already up next it reads **Not up next** and clears it |
| Back to list (66) | "Off waiting, or reopened from finished, back on the list" | only on a waiting or finished task; a waiting task gets just this, Go to…, Edit, Complete, Move to workspace and Remove; a finished one just this, Go to…, Edit and Remove |
| Remove (67) | "Turns into **Confirm remove**; a second press deletes it" | moving away puts it back; `Ctrl+Delete` deletes the focused task at once, whichever button is focused, and moves to the next card |
| Clear all (68) | "On the Waiting tab: deletes every waiting task, after **Confirm clear all**" | on a strip under the tabs, not a card; arming takes the focus off the cards; a second press or `Enter` deletes every waiting task on the workspace and goes back to All; `Escape` or moving away puts it back |

- [ ] **Step 4: Rewrite the keys paragraph (old 70-81) as a key table**

Add `### Keys`. Use a `| Key | Does |` table plus bullets. Every fact in old 70-81 must land:

- Up/Down: move the darker fill and the buttons between cards; Down onto "+N more" shows the rest in its place and carries on to the first of them.
- Left, Right, Tab: move along the focused card's buttons. Enter: presses the focused one.
- The hint: beside the buttons, in smaller dimmed text, the focused card names its own keys. That is the focused button's letter and name (`o: Go to…`, `Del: Remove`), or on a card's description `Space: view notes` / `Space: hide notes`, and what Ctrl+Enter would do to this task.
- The line under the list names the keys that act the same on every card: Enter or Space, and Ctrl+Delete. It is not on Ideas, where the notepad takes those keys as typing.
- Notes: Enter or a click on the card's description shows, dimmed under it, the id and uuid (`#48 · <uuid>`, the uuid alone on a finished task), then its notes, one per note. They scroll past about twelve lines. A second press hides them. Opening another card's notes hides the first's, and giving the keyboard back hides them all.
- Every button that opens something gives the keyboard back as it runs. Go to…, Up next, Complete, Move to workspace, Waiting, Back to list and Remove keep the list up.
- Escape puts the panel back on the right edge, tucked away to the peeked card if there is one.
- A list taller than the screen scrolls, keeping the focused card in view.
- Hovering shows the buttons only on the peeked card; on the others only the keyboard does.

- [ ] **Step 5: Rewrite Ctrl+Enter / Ctrl+Delete (old 124-137)**

Keep it in its original position, after the Go to… table (Task 3 handles 83-122). Add `### Ctrl+Enter and Ctrl+Delete`, then bullets:

- both act on the focused card, whichever of its buttons is focused;
- Ctrl+Enter, by task state: unplanned → refine; planned → start working. Either way the panel keeps the keyboard with the same card focused, so you can carry on down the list. On a task being worked with a live Claude, planned or not → go to that session, as the Go to list's session rows do, and give the keyboard back. Worked task with no Claude → refine if it has no plan, nothing if it has one. Waiting or finished → nothing. A small `| Task | Ctrl+Enter |` table fits this best.
- Ctrl+Delete deletes at once, with no Confirm remove, and moves the focus to the next card (the one above, from the last) with the list kept up. Delete alone still asks first.
- Ctrl still held from the `Mod+Alt+Ctrl+T` chord counts. Let go of Ctrl before Enter if you only want the focused button, or before Delete if you want Remove to ask.
- Every other Ctrl chord passes through untouched, apart from Ctrl+Shift+Delete (the Waiting tab's Clear all).

- [ ] **Step 6: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. No offence may fall inside the parts this task rewrote. Use the new line numbers of `### Cards` … the end of `### Keys`, and of `### Ctrl+Enter and Ctrl+Delete`.

Fact audit: `git show cca971a:README.md | sed -n '24,81p;124,137p'`. Every sentence must map to a new line.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`. Expected: all ok.

- [ ] **Step 7: Commit**

```bash
git add README.md
git commit -m "docs(readme): split the task panel's cards, peek and keys"
```

---

### Task 3: Task panel, part 2: Move to workspace, Go to…, tabs, Ideas, looks

**Files:**
- Modify: `README.md` `## Task panel`, old lines 83-122 and 139-181

**Interfaces:**
- Consumes: `target/readme-shape.py` (Task 1). Task 2's buttons table no longer lists the Go to rows, so this task's Go to table must name every row.

- [ ] **Step 1: Move to workspace (old 83-91)**

Add `### Move to workspace`. Write it as numbered steps:

1. The cards swap for a list of your `~/Projects` folders, drawn like cards, under a line naming the task. Every folder but this workspace's own is listed.
2. Type to narrow it. The field above ranks folders as fzf does, best match first, and the highlight goes to the top match. Without fzf it keeps the folders whose names hold what you typed.
3. Up and Down move the highlight.
4. Enter or a click moves the task there. It is retagged to that folder's workspace, and the panel goes back to the cards with the next one focused.
5. Escape clears what you typed. On an empty field, it goes back to the cards, on the same one, without moving the task.

- [ ] **Step 2: Go to… (old 93-122)**

Add `### Go to…`. Keep the lead line ("swaps the cards for a list under a line naming the task…, each row only when it applies") and the `| Row | Shows when |` table. Cut the long cells to ~2 lines and move the overflow into bullets under the table:

- **Open report** (102): cell keeps "its notes link a report (`Report: <path>`)". Bullets carry the rest: the path is absolute, `~/...`, or a name in `~/.local/share/niri-tasks/reviews`; an HTML report opens in a new browser window on this workspace; a Markdown one opens in a VS Code of its own, as the plan and spec are.
- **Explain branch** (103): cell keeps "the task has a worktree". Numbered sub-steps carry the rest:
  1. Its tests run first in a `Tests:` tab, outside Claude's sandbox, when it has a test command (`git config niri-tasks.test '<command>'`, else this repo's `tests/all.sh` headless).
  2. Claude, in a tab of its own in the worktree, explains what the branch has changed since it left the default branch.
  3. Claude reads that test run into its report (or, with no test command, runs what tests it can itself).
  4. The deck is linked as `Branch report: <path>`.
- **Planning doc, Design spec** (105): cell keeps "its notes link a plan (`Plan: <path>`) or a spec (`Spec: <path>`)". Bullet: opened in a VS Code of its own, apart from your VS Code windows, sharing your extensions, settings and keybindings.

Old 111-115 (colours) becomes a `| Colour | Rows |` table: mauve for a Claude working on the plan (Refine, Grill me, Generate report, Explain branch); green for Get started; blue for going to a running Claude; yellow for the documents (Open report, Branch report, Planning doc, Design spec); sapphire for the worktree and VS Code; pink for Read aloud.

Old 117-122 becomes bullets: there is no field to type in; Up/Down move the highlight; Enter or a click runs the row and gives the keyboard back (Read aloud keeps it, back on the cards); Escape goes back to the cards on the same one; the plan and spec are linked by the Claude Start working starts, once it has saved its plan; their paths are the repository's, read in the task's worktree while it has one.

- [ ] **Step 3: Tabs (old 139-161)**

Add `### Tabs`. A table of the six filter tabs, `| Tab | Key | Holds |`:

| Tab | Key | Holds |
|---|---|---|
| **All** | `1` | everything but waiting and finished tasks |
| **Active** | `2` | started tasks |
| **Planned** | `3` | refined or grilled into a plan, not started yet |
| **To refine** | `4` | neither planned nor started yet |
| **Waiting** | `5` | parked tasks |
| **Finished** | `6` | the last 12 completed, the most recently finished first, each aged from when it was finished |

Then bullets for the rest of old 139-152:

- tabs narrow the cards while the panel has the keyboard;
- a tab shows only while it has a task under it, All apart;
- `1`–`6` each always pick the same tab, and do nothing while it is hidden;
- each label ends in its key, as **To refine (4)**;
- `[` and `]` step to the shown tab either side;
- a click picks one too;
- the arrows and Tab still move only between cards and buttons;
- a started task is under Active alone, not Planned or To refine;
- waiting tasks are on the Waiting tab only: not on All, and never the peek;
- finished tasks are on the Finished tab only, the same way, and Back to list on one reopens it, back on All;
- the panel opens on All every time, and goes back to All when the tab it is on runs out of tasks.

Then `#### Clear all` (old 152-161), as numbered steps:

1. It shows only on the Waiting tab, on a strip of its own under the tabs. `Ctrl+Shift+Delete` presses it; `Ctrl+Delete` and `Delete` are still the focused card's.
2. First press: it arms as **Confirm clear all** and takes the focus off the cards. No card's buttons show, and a card's own keys, `Ctrl+Delete` among them, do nothing.
3. Second press, or `Enter`, deletes every waiting task on this workspace, one `task status <uuid> deleted --yes` after another. The panel goes back to All, keeping the keyboard.
4. `Escape`, an arrow, Tab, or switching tab disarms it instead. The focus returns to the card it was on, and the panel stays up.

- [ ] **Step 4: Ideas (old 163-172)**

Add `### Ideas`. Use bullets:

- the last tab; not a filter: in place of the cards, one large text area, a notepad for this workspace's ideas that are not tasks yet;
- always there, after Finished; `7` picks it, as `]` does from the last tab before it;
- while it is picked every key is typing, the letters, digits and brackets included;
- `Escape` saves and goes back to the task list, on the tab you were on before Ideas, with the panel still up; a second `Escape` gives the keyboard back;
- `Ctrl+[` and `Ctrl+]` switch tab;
- one notepad per workspace tag, plain text in `$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md` (under `~/.local/share` without it);
- saved a second after typing stops, and again on leaving Ideas or when the panel gives the keyboard back.

- [ ] **Step 5: Looks (old 174-181)**

Add `### How it looks`. Use bullets: cards shaped like mako notifications, taking a terminal window's font and colours; a dark tint over blurred wallpaper with niri's faint focus-ring outline (`src/panel/style.rs` names the configs it copies); the blur is asked for by the panel itself, over `ext-background-effect-v1`, in the exact shape of the cards; a compositor without that protocol shows them unblurred; the lock icon comes from a Nerd Font (JetBrainsMono Nerd Font here); see `docs/adr/0001-task-panel-in-gtk-not-quickshell.md` for why it is drawn the way it is.

- [ ] **Step 6: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. Nothing may be flagged anywhere between `## Task panel` and `## Install`.

Fact audit: `git show cca971a:README.md | sed -n '83,122p;139,181p'`. Also check that every Go to row named in old line 58 is in the Go to table.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`. Expected: all ok.

- [ ] **Step 7: Commit**

```bash
git add README.md
git commit -m "docs(readme): split Go to, tabs and Ideas into short parts"
```

---

### Task 4: Install, Requirements, Privacy

**Files:**
- Modify: `README.md` old lines 183-299

- [ ] **Step 1: Install (old 183-204) as numbered steps**

```markdown
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
```

Then `install.sh` does, as bullets:

- builds `niritasks`;
- symlinks the niri include;
- symlinks the Claude skills (`workspace-tasks`, `refine-task`, `report-task`, `finish-worktree`, `reject-worktree`, `private-github-repo`) into `~/.claude/skills`;
- symlinks the refine mod (`claude/refine-mod`, which Refine's session writes the task with) into `~/.local/share/niri-tasks/refine-mod`;
- symlinks a Claude rule into `~/.claude/rules` (use `finish-worktree` to land a worktree branch);
- restarts the daemon onto the new binary;
- reloads niri.

Then add the line: "The symlinks make edits in this repo live at once. There is no copy to keep in sync."

- [ ] **Step 2: Requirements (old 206-291) as tables**

Under `### Requirements`, add a `| Needs | For | Without it |` table with one row per requirement, from old 208-228 and 245-257:

- `niri`, `taskwarrior`, a Rust toolchain: required.
- `ghostty` 1.2 or later: required, for `+new-window`.
- `notify-send`: feedback. Without it, feedback degrades to stderr.
- `fzf`: ranks what you type on the project list. Without it, plain-text matching, and it says so once.
- `tar`, `sqlite3`: for `niritasks export`/`import` (`sudo apt install sqlite3`).
- [herdr](https://herdr.dev): optional. `project open` runs the workspace's herdr session in the project terminal. Without it, a plain shell in the project folder.
- Claude Code (`claude`, 2.1.287 or later for Refine's mod) + herdr: for Refine, Grill me, Generate report, Explain branch. Add & all needs Claude Code alone. These open Claude inside that session.
- `bubblewrap`, `socat` (`sudo apt install bubblewrap socat`): the Bash sandbox. It keeps Claude to reading anything but secrets, reaching only a fixed list of source and docs hosts, and writing only the task and its own temp folder. Without them, Claude refuses to start rather than run unfenced.
- [worktrunk](https://worktrunk.dev) (`wt`) + herdr + Claude Code, with the `~/Projects` folder a git repo: for **Start working**. worktrunk makes the worktree and runs the repo's `.config/wt.toml` hooks, asking in a herdr tab the first time a repo's hooks need approving.
- herdr 0.9.1+, `python3`, `fzf`, plannotator-tui from herdr-annotate Full (`herdr plugin install plannotator/herdr-annotate`; Lite has no plannotator-tui): for **Task docs**, the herdr plugin `install.sh` links. Without it the key says so.
- `claude`, `curl`, `ffplay` (from ffmpeg), a [Kokoro](https://github.com/remsky/Kokoro-FastAPI) server on `127.0.0.1:8880`: for **Read aloud** (in a card's Go to list).
- `code`: VS Code, optional (old 284-287).

Split any cell that runs past ~2 lines into bullets under the table, led by the requirement in bold. That applies at least to Read aloud's Kokoro start-up: if Kokoro is not answering, Read aloud starts it, from the `kokoro-tts` container if there is one, otherwise a new one from `ghcr.io/remsky/kokoro-fastapi-gpu`. That needs `docker`, an NVIDIA GPU and the NVIDIA Container Toolkit, and the first start downloads the image. The container is the one [herdr-speak](https://github.com/pauldaywork/herdr-speak) uses, so the two share one server. It also applies to VS Code's lookup: `project open` starts VS Code when `code` is on `$PATH`, else only the terminal. The lookup is there because niri answers a spawn of a missing binary with a desktop notification, on every project opened.

Keep "Tested against niri 26.04 and Taskwarrior 3.5.0…" (old 289-291) as two bullets: tested versions; still runs on Taskwarrior 2.6.2 (`pending.data`/`completed.data` rather than 3.x's `taskchampion.sqlite3`; the panel watches both).

Old 230-243 (pane labels) and 259-282 (hand-started Claude, finish/reject/private-github-repo) are not requirements. They still stay under `### Requirements`, in the same order, as short bold-led bullet groups:

- **Pane labels:** border `Refine #61 · <description>` / `Work #61 · <description>`; status line `#61 · <description> · <state> · <branch>`, read afresh on each refresh; it replaces your own status line in those two Claudes only; herdr draws borders and labels only around split panes unless you add the `[ui] pane_borders = "always"` snippet (keep the indented block) and run `herdr server reload-config`; niri-tasks never edits that file; tokens `task_id`, `task`, `task_uuid` for an Agent row in `[ui.sidebar.agents]` as `$task_id`, `$task`, `$task_uuid`; the default sidebar shows none.
- **A Claude started by hand:** linked to a task when it marks it active (`niritasks task status <uuid> active`); agent named `work-<uuid8>`, the name Start working gives its own; pane labelled `Work #<id> · <description>`, so Go to offers Go to work session; an agent already named by Refine or Start working keeps its name.
- **`/finish-worktree`:** numbered steps: syncs `main` with origin → rebases the branch, tests the result → fast-forwards `main`, pushes → marks the task completed with `niritasks task status` → links any plan or spec doc the branch added that the task doesn't carry as a `Plan:`/`Spec:` note → removes the worktree. Then a bullet: it stops to ask only for a conflict with no obvious resolution, red tests, or `main` holding commits origin doesn't have.
- **`/reject-worktree`:** removes the worktree and its branch, locally and on origin; puts the task back on the list, stopped, to be started fresh; a reason gives a `Rejected:` note; asks first only when commits, uncommitted changes or a pushed branch would be lost.
- **`/private-github-repo`:** a private repo on your account, named after the folder, set as `origin` and pushed; runs `git init` if it has to; asks before a first commit; stops rather than push a secret, overwrite an existing `origin`, or reuse a name already taken on your account.

- [ ] **Step 3: Privacy (old 293-299)**

Use bullets: Read aloud sends the description and notes to Anthropic; `claude -p` rewrites them for listening, on Claude Haiku, with no tools and no saved session; the speech is made locally by Kokoro; nothing is saved, neither rewrite nor audio; Refine and Start working send the task to Claude too, since a Claude session works on it.

- [ ] **Step 4: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. Nothing may be flagged between `## Install` and `### Annotating a task's documents in herdr`.

Fact audit: `git show cca971a:README.md | sed -n '183,299p'`.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`. Expected: all ok.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs(readme): turn install and requirements into steps and tables"
```

---

### Task 5: The herdr plugin sections

**Files:**
- Modify: `README.md` old lines 301-374 (`### Annotating…`, `### Opening the task panel…`, `### Peeking…`). Keep all three headings verbatim.

- [ ] **Step 1: Annotating (old 301-338)**

Write numbered setup steps:

1. Bind the action in `~/.config/herdr/config.toml` (keep the toml block).
2. Run `herdr config check`.
3. Run `herdr server reload-config`.

`install.sh` links the plugin, `herdr/task-docs`, as the lead line. Then "Use it" as bullets:

- press it in the pane of a task's Claude, or any pane whose folder is on the task's `task/<slug>-<uuid8>` branch;
- a list opens over the pane (keep the existing four-item list; it is already bullets; split the last item's plan-timing sentence into its own sub-bullet);
- Enter opens the pick in plannotator-tui, to the right of the task's Claude;
- comment there; **Send** makes the comments that Claude's next message;
- a report made before reports had a Markdown twin opens in the browser instead, as a card's Open report does;
- a pane with no task, or a task with no Claude running: a notification, nothing opens;
- the list is `niritasks task docs <uuid>`, so a script can read it too.

Old 335-338 (Markdown twin) becomes bullets: each refine report is written first as `<report>.md`, its Markdown twin, diagrams kept as Mermaid blocks; its HTML slides are built from what that twin reads back as, so the two never disagree; plannotator-tui opens the Markdown.

- [ ] **Step 2: Opening the panel on a pane's task (old 340-358)**

Lead line: same plugin, second action `task-docs.panel`. Bind it on a key of its own, not `prefix+shift+t` (herdr's rename tab). Keep the toml block. Then bullets: press it in a task's pane, found as for Task docs; the panel slides out with the keyboard, on All, focus on that task's card, notes hidden; a task with no card on the focused workspace's panel (on another workspace, or not pending: waiting or finished, say) opens on its first card, with a notification saying why; a pane with no task: notification, nothing opens; it runs `niritasks task panel <uuid8>`.

- [ ] **Step 3: Peeking (old 360-374)**

Use bullets:

- the plugin hooks herdr's `pane.focused`, `tab.focused` and `workspace.focused` events; no key to bind;
- selecting a pane, tab or workspace finds the now-focused pane's task, as Task docs does; that card becomes the peek on its workspace's panel, the one card the edge shows;
- going to a niri workspace does the same without touching herdr: the daemon asks that workspace's herdr session for its focused pane each time a monitor switches to it, and when the daemon starts, so the peek is right after a restart too;
- the edge empties for a pane with no task, a session that is not running, or the task leaving the list (completed, waiting or removed);
- selecting the same pane again changes nothing;
- the hooks run `niritasks task peek --focused` and need herdr 0.9.3;
- they send no notification; if the peek does not follow the focus inside herdr, `herdr plugin log list --plugin task-docs` says why.

- [ ] **Step 4: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. Nothing may be flagged between `### Annotating` and `## Commands`.

Fact audit: `git show cca971a:README.md | sed -n '301,374p'`.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`. Expected: all ok.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs(readme): list the herdr plugin's steps and behaviour"
```

---

### Task 6: Commands, the two rules, the task box

**Files:**
- Modify: `README.md` old lines 376-528

- [ ] **Step 1: Wrap the Commands block (old 382-457)**

Keep `## Commands` immediately followed (after the llms.txt lead paragraph) by the one fenced block. The test reads the first block after that heading. Keep every `niritasks …` command line, flags included, on one line. Wrap only the `#` comments onto `#   …` continuation lines, aligned at column 36 as the block already does, so no line exceeds 110 characters. Old line 425 (`task report <uuid> --branch`) must become:

```
niritasks task report <uuid> --branch  # open the branch report linked from its notes; without one,
                                   #   Claude explains what its worktree branch has changed, opens the
                                   #   report and links it; first, a Tests: tab runs the worktree's tests
                                   #   outside Claude's sandbox (git config niri-tasks.test, else
                                   #   NIRITASKS_HEADLESS=1 bash tests/all.sh) and Claude reports on that run
```

Do the same for old 393, 412, 418, 420, 441, 446, 450, and any other line the checker flags.

Old 459-462 becomes bullets: every `<uuid>` is the full uuid or its first 8 characters, never a task's number; numbers are renumbered as tasks complete, so a stale one points at another task; `niritasks <command> --help` says the same as each line, at more length.

- [ ] **Step 2: Two rules (old 464-477) as a table**

```markdown
| Command | Splits words? | So `due:friday`… |
|---|---|---|
| `niritasks task add` | **Yes**: taskwarrior parses its own attribute syntax | sets a due date (`niritasks task add ship it due:friday`) |
| `niritasks task edit`, `niritasks task note` | **No**: the text goes through as one argument | stays literal text |
| The task box, adding | Yes | is a due date |
| The task box, editing an existing task | No | stays literal |
```

Then one line: both are pinned by tests in `tests/write_path.rs`, against a sandboxed task database, because getting them backwards fails silently.

- [ ] **Step 3: The task box (old 479-528)**

Split it under `### The task box` into bold-led groups:

- **What it is:** one window adds a task and edits one. Edit opens it on the task's description and every note. `niritasks task note <uuid>` opens it the same way, with the cursor in a new empty row at the end.
- **Notes:** each note is a row; it wraps; you edit it in place; its date sits small at its right end; × deletes it; "+ Add note" appends a row.
- **Add & refine:** adds the task and hands it straight to Claude's refine-task skill, in a new tab of the workspace's herdr session, as a card's Refine does.
- **Add & all**, as numbered steps of what `niritasks task refine <uuid> --unattended` does: runs Claude headless in the project folder, behind the same fence as a refine tab → writes the plan asking nothing; each question it would have asked is decided with its recommended answer and recorded as a `Decided:` note → builds the report and links it as `Report: <path>`, without opening it. Then bullets: the card is dimmed with an hourglass meanwhile and offers nothing; the notification says `Planned and reported`, or which step failed and where its log is (`$XDG_DATA_HOME/niri-tasks/unattended/`, by default `~/.local/share/niri-tasks/unattended/`); the log holds each run's final answer and every action Claude was refused; the notification counts the refusals and gives the log's path; Add & all has no key and needs Claude Code but not herdr; if a refine fails, either way the task stays added and a notification says why.
- **One box only:** Mod+Alt+T, or Edit or Note on a card, while a box is open brings that box forward and opens nothing, so nothing typed is lost. If you asked for a different box, a notification says one is already open. Save or discard it, then ask again.
- **Where the keyboard goes after:** a box opened by a card's Edit hands the keyboard back to the panel when it closes, however it closes. It returns on the tab Edit was pressed on, with that card focused (or the tab's first card, if that one has gone). A box opened any other way (Mod+Alt+T, `niritasks task edit`) leaves the panel tucked away. So does taking or giving back the panel's keyboard while the box is open.

Keep the key table (old 515-522) as is. Its cells are already short.

Old 524-528 (saving) becomes bullets: one `task import` writes the description and whole note list; untouched and edited notes keep their first date; new ones are dated when saved; a save that changed nothing writes nothing; Taskwarrior keeps notes in date order, so a note typed between two old ones moves to the end once saved; empty rows are dropped.

- [ ] **Step 4: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. Nothing may be flagged between `## Commands` and `## Backing up before a reinstall`.

Fact audit: `git show cca971a:README.md | sed -n '376,528p'`. For the Commands block, `diff <(git show cca971a:README.md | sed -n '382,457p' | grep -o 'niritasks [^#]*' | sed 's/ *$//') <(sed -n '/^## Commands$/,/^### Two rules/p' README.md | grep -o 'niritasks [^#]*' | sed 's/ *$//')` must print nothing.

Run: `cargo test --quiet readme_commands 2>&1 | grep -E 'test result: ok. [1-9]'` → one passing line. Then `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'` → all ok.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs(readme): wrap the commands block and split the task box"
```

---

### Task 7: Backing up and Notes

**Files:**
- Modify: `README.md` old lines 530-595

- [ ] **Step 1: Backup steps (old 532-584)**

These are already numbered. Keep the numbers and bold actions. Move each step's explanation into indented sub-bullets of one fact each:

1. **Push every task worktree branch.**
   - A task's `Plan:` and `Spec:` notes name files in its repo's `docs/superpowers/`; until the branch lands they live only on that branch.
   - In each repo, `git branch --list 'task/*'` lists them; `git push origin <branch>` each one.
   - `export` does not do this for you.
2. **Run `niritasks export`.** Sub-bullets: writes every task, whatever its status, to `~/niri-tasks-backup-<YYYYMMDD-HHMM>.tar.gz` or the path you give, and refuses one already there; prints the path and the task count; archive contents as a nested list (`tasks.json`, the store as `task export` writes it; `taskchampion.sqlite3`, a copy taken through SQLite's own backup; `reviews/`, the refine reports folder, when it exists); then lists each `Report:` or `Report (earlier draft):` note whose file is already gone, or is outside the reviews folder and so not in the archive, with uuid8, status and path; those notes will dangle after the restore whatever you do; the backup goes ahead without them.
3. **Copy the archive off the machine.**

The restore steps go the same way. Step 1 is the rc file: a fresh install has no `~/.taskrc`, which is not backed up; `task` refuses to run without one ("Cannot proceed without rc file."); restore yours, or run `task` once interactively to create one; `data.location` must point where you want the store. Step 2 is `niritasks import`: imports `tasks.json` with hooks off; copies `reviews/` back into `~/.local/share/niri-tasks/reviews` (or under `$XDG_DATA_HOME`) without overwriting files already there; prints the count; refuses a store that already holds tasks (any status, deleted included), so an old backup cannot undo newer edits; `--force` imports anyway, and a task in both takes the backup's copy; if it stops after the tasks are in (say, copying reports), rerunning with `--force` from the same backup is safe.

Old 569-573 becomes a "**If `tasks.json` will not import**" lead plus bullets: restore the SQLite copy by hand, once `task` has an rc file, into a store with nothing in it and no `task` running; a reinstalled `task` may already have made an empty `taskchampion.sqlite3` with `-wal` and `-shm` files beside it, which the `rm` below removes. Keep the bash block unchanged.

Old 583-584 becomes: "**Not backed up:** `~/.taskrc`, and the rest of `~/.local/share/niri-tasks` (unattended logs, ideas, the VS Code profile)."

- [ ] **Step 2: Notes (old 586-595)**

Use bullets: `~/.taskrc` is not managed here; the only requirement is `data.location` pointing somewhere `task` can read and write. Then a **Blur rule** lead with bullets: it is in `niri-tasks.kdl` and matches the panel's namespace `niri-tasks-panel`; it sets how the blur looks but never `blur true`; a rule's blur would fill the panel's whole surface, which is wider than the cards so they have room to slide; the panel asks for blur behind the cards alone instead, and niri takes noise and saturation from the rule.

- [ ] **Step 3: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. Nothing may be flagged between `## Backing up` and `## Development`.

Fact audit: `git show cca971a:README.md | sed -n '530,595p'`.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`. Expected: all ok.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs(readme): give the backup steps one fact a line"
```

---

### Task 8: Running the tests

**Files:**
- Modify: `README.md` old lines 597-702 (`### Running the tests`, up to the `[!IMPORTANT]` block)

- [ ] **Step 1: Intro (old 601-624)**

Keep the command block (old 601-611) as is. Old 613-624 becomes bullets: each exits non-zero on failure, so any can go in a loop or a hook; `tests/all.sh` runs everything in one command, cheapest-and-quietest first; it checks each suite's prerequisites and reports one it cannot run as a **skip, with the reason** (no niri, no Pillow, no Wayland display); a skip is not a failure; it exits non-zero only when a suite that ran said no, so it is safe on a machine that can only run half; no e2e suite takes the machine over: the panel and box tests run in a nested niri parked on a spare workspace, `e2e-tag.sh` at most names a spare workspace for a moment, and `all.sh` tells you which one to keep off; `NIRITASKS_HEADLESS=1 bash tests/all.sh` skips the three e2e suites and runs the rest, including `claude plugin test claude/refine-mod`.

- [ ] **Step 2: Needs/Touches table (old 626-635)**

Cut each cell to ~2 lines. Move the overflow into bullets under the table, led by the suite in bold:

- `tests/container.sh` Needs: "Docker, the user in the `docker` group". Bullet: the first run builds the image (several minutes, and again when one of its inputs changes) and needs the network; later runs do not. Touches: "Nothing of yours". Bullet: Docker's own images and volumes (`niritasks-tests*`, pruned as they go stale), and its logs in `target/container-tests/`.
- `tests/e2e-tag.sh` Touches: "At most the spare workspace's name, taken off again". Bullet: no herdr session (a fake `herdr` stands in); only a scratch task database of its own.
- `tests/e2e-panel.sh` Needs: "niri, Wayland, `python3-pil`, taskwarrior; `wtype` for its keyboard checks". Bullet: those keyboard checks are skipped without `wtype`. Touches: "A nested niri window, parked on the spare workspace". Bullets: it is parked at the end of your monitor, so keep off that workspace until it finishes; it never touches your clipboard, your screenshots or the `niri-tasks` daemon.
- `tests/e2e-box.sh` Touches: "A nested niri window, parked like the panel test's". Bullets: keep off that workspace until it finishes; its keys go only to that nested niri; never your keyboard or the `niri-tasks` daemon.

Old 637-638: keep as one line with the four `cargo test` narrowing examples.

- [ ] **Step 3: e2e-panel (old 640-670) into bullets and checklists**

Add `#### tests/e2e-panel.sh`. Bullets for old 640-651: a screen of its own, a nested niri with its own runtime directory and a flat 1600x1000 output, and its own daemon against a sandboxed `TASKDATA`; your wallpaper, windows, window rules, notifications and pointer cannot reach its frames; your `niri-tasks` daemon keeps running; the window opens over yours for a moment, then is parked floating and unfocused on the last workspace of that monitor; keep off that workspace, because going there focuses the window and the run fails, saying so, rather than measuring frames your typing could have reached; run it with `NIRITASKS=$PWD/target/debug/niritasks`, or it runs the installed, maybe older, `niritasks`; `NIRITASKS_E2E_KEEP=1` leaves the frames on disk.

Old 653-670 becomes "**Check by hand after changing `src/panel/surface.rs`**" (it cannot move the pointer). Lead: set up with a task peeked (select its herdr pane, or `niritasks task peek <uuid8>` from a pane of its workspace's session). Then two plain bullet checklists, one check per bullet, and every check from old 653-670 present.

*Pointer:* the peek slides out when the pointer reaches it; it slides back about 0.4s after it leaves; the slide is smooth; its notes and buttons show; a click just left of the peek lands on the window beneath; a click on its description does nothing; Complete completes it and empties the edge without taking the keyboard from herdr; Remove asks once, and the pointer leaving puts it back; Go to… slides the panel to the middle with the keyboard, on that card's Go to list.

*Keyboard, past what the script measures:* `Mod+Alt+Ctrl+T` slides the panel from the right edge to the middle, smoothly, with every card wrapped and the first darkened; the pointer passing over does not move it; active tasks show Stop and no Start working; Up/Down move the darkening between cards; Left, Right and Tab move along the buttons; `s` starts working; `e` opens the box; `t` stops; Delete arms Remove and only a second Delete deletes, while moving away disarms it; Enter on a description shows its notes dimmed under it, with the panel and its blur grown to fit, and a second Enter hides them; a card with more notes than fit stops at about twelve lines and scrolls the rest by wheel and by dragging the scrollbar, neither of which hides them, while a click on the notes does; `c` completes; `m` shows the project list; Escape comes back from it, while Enter on a folder moves the task there; `project open` on a workspace with no tasks shows the project list, and Escape closes it; Down reaching "+N more" shows the rest, focused on the first card it hid; a list taller than the screen scrolls with the focus; a click on a filter tab switches the cards and does not take the focus (the focus still falls back to the first card when the focused one is not under the new tab); a started planned task shows under Active but not Planned; Back to list on a waiting task brings it back to All; a click on Clear all arms it, darkening no card; a second click deletes the waiting tasks; Up or Down in between puts it back, with the focus on the card it was on; arming Clear all puts an armed Remove back, and arming Remove puts Clear all back; Escape tucks it away, to the peeked card if there is one.

- [ ] **Step 4: e2e-box (old 672-688)**

Add `#### tests/e2e-box.sh`. Bullets: runs in the same kind of nested niri (`tests/lib/nested-niri.sh` starts both); keys are pressed with `wtype` pointed at that nested niri, so they never reach your windows and you can keep working; keep off its parked workspace; the box is served by a daemon started inside the nested niri; once that daemon is stopped, it checks the CLI refuses with "daemon is not running" rather than drawing a box; your own daemon is never stopped. Then a **From a herdr pane** sub-list: both nested tests drop every `HERDR_*` variable before starting the nested niri, so they behave as from a plain terminal and never touch your herdr session or pane; either way the daemon and commands see a `$PATH` with every directory holding a `herdr` removed, and the run stops if `herdr` can still be found. Then "**Check by hand after changing `src/taskbox.rs` or `src/taskbox/`**" (it cannot click): × deletes its row; "+ Add note" appends an empty row with the cursor in it; a long note list scrolls, keeping a row made by Enter at the bottom in view.

- [ ] **Step 5: The IMPORTANT callout (old 690-702)**

Keep the `> [!IMPORTANT]` block. Make its prose short lines: all three scripts run `niritasks` **from `$PATH`**, the installed binary, not the one you just built; a green run after an edit you have not installed is testing the old code; point them at a build with `NIRITASKS=` (keep the code block); to try a change by hand use `bash try.sh` (below), not `install.sh` or `cargo install`, because both replace the `niritasks` your daemon and every other worktree's test runs use.

- [ ] **Step 6: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. Nothing may be flagged between `### Running the tests` and `### Running the tests in a container`.

Fact audit: `git show cca971a:README.md | sed -n '597,702p'`. Count the hand checks in old 653-670 and 685-688, and make sure the new checklists have at least as many bullets.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`. Expected: all ok.

- [ ] **Step 7: Commit**

```bash
git add README.md
git commit -m "docs(readme): turn the test notes into tables and checklists"
```

---

### Task 9: Container, try.sh, and why the scripts are not cargo tests

**Files:**
- Modify: `README.md` old lines 704-854

- [ ] **Step 1: Container (old 704-762)**

Under `### Running the tests in a container`:

- Lead: `tests/container.sh` runs `tests/all.sh` in a Docker container, against this checkout as it stands, uncommitted edits included.
- **Isolation** bullets: no mount, socket or display of yours, no network, only a copy of the files git would see; so your niri, workspaces, task database and installed `niritasks` are never touched, and there is no workspace to keep off; run it from any worktree, beside other worktrees' runs, foreground or background.
- Keep the usage code block.
- **The image** bullets (`tests/container/Dockerfile`): it reproduces what the suites use here, at this machine's versions and in the same folders. Contents as a sub-list: niri and ghostty from the same PPA; Taskwarrior 3.5.0 compiled from its tarball; rustc 1.95.0; herdr, worktrunk and Claude Code in `~/.local/bin`; Iosevka Term in `~/.local/share/fonts`, as here, because the panel test's own `XDG_DATA_HOME` hides that folder and its results depend on the font; the GTK theme and font settings; `en_US.UTF-8`; your timezone; a uid-1000 user. Then: each run makes the links `install.sh` makes, from the copy; VS Code and gh are left out; Claude Code is never logged in; the `docker run … check-image.sh` command (keep it verbatim, in a code block, since it is too long for prose) lists what the image holds.
- **Inside a run**, numbered: `tests/container/inside.sh` starts a headless sway → an outer niri in it with two named workspaces → writes a GTK `settings.ini` from the image's gsettings (GTK on Wayland reads no GSettings without a desktop portal) → builds the copy → runs `tests/all.sh` with `NIRITASKS` set to that build; the e2e suites nest their niri in the outer one, as they would in yours.
- **Rebuilds** bullets: built on the first run, and again only when one of its four inputs changes: the Dockerfile, its GTK settings keyfile (`tests/container/dconf/00-machine`), `Cargo.toml`, `Cargo.lock` (`tests/container/image.sh`); it holds the crate's dependencies already compiled; each checkout's build directory is a Docker volume filled from them, so a checkout's first run compiles only niri-tasks and later runs only what changed.
- **Parity** bullet: each run starts by saying whether the image still matches this machine (package versions, programs, font, GTK settings; `tests/container/parity.sh`); when it does not, rebuild with `NIRITASKS_CONTAINER_REBUILD=1 bash tests/container.sh`.
- **Runs and logs** bullets: a second run of the same checkout while one is going refuses, since the two would share a build directory; every run is logged in `target/container-tests/`, `latest.log` the newest; a failed run's logs and the e2e suites' frames are kept beside it in `<stamp>-artifacts/`.
- **Exits non-zero when**, as a sub-list: a suite that ran fails (as `tests/all.sh` does); there is no Docker; the image build fails; the run is refused; the container's desktop or build fails.
- **Clean-up** bullets: each run removes the build volumes nothing will use again (those of worktrees since removed, and of images since replaced); old images go only when a new one is built, which keeps the three newest; to reclaim everything (keep the code block).

- [ ] **Step 2: try.sh (old 764-796)**

Keep the code block. Then:

- **What it does** bullets: run from any checkout or worktree; it builds it and opens it in a nested niri, the same sandbox the e2e tests use (`tests/lib/nested-niri.sh`), but as an ordinary window, focused, on your workspace; its one workspace is named after the project; the panel shows the project's tasks from a copy of your task database, taken with an SQLite online backup; nothing is written back; notifications are printed in the terminal rather than shown on your desktop.
- **Keys** table: `niri/niri-tasks.kdl` is included with Mod on plain Alt, since your own niri takes Super combinations first. Rows: Alt+T adds a task; Alt+Shift+T adds and refines; Alt+Ctrl+T shows the panel; Alt+W opens a project; Alt+Return opens a terminal (stubbed in a try window).
- **The try shell** bullets: its `niritasks`, `task` and `niri msg` reach the nested niri; `reload` rebuilds and restarts the nested daemon, keeping the window and the copy; `exit` closes everything.
- **Kept apart** bullets: the installed binary, the `niri-tasks` service and `install.sh`'s links are never touched, so tries in several worktrees run beside each other and beside `tests/all.sh`; herdr is kept out, so Start working and the Go to list's sessions fail there, with the reason printed in the try shell; terminals, editors and clones are not opened either: `ghostty`, `code` and `gh` are stubs that print what would have run (`[notify] would run: …`), because ghostty and VS Code would hand off to your running instances and `gh` would clone into your real `~/Projects`; the `gh` stub fails, so a clone says it could not and the project list keeps the GitHub repos it had, from a copy of your `~/.cache/niritasks` that the try build writes instead of yours; Alt+W can still make a folder under `~/Projects`, for a new name typed there.
- Last line: installing a build for daily use is still `bash install.sh`, on main.

- [ ] **Step 3: Why the three scripts are not cargo tests (old 798-854)**

Use one bold lead per script, with bullets under each.

**`tests/e2e-box.sh`:**

- it needs a running niri, a Wayland display, and `wtype` to press the keys;
- it opens the box, types into it, presses Ctrl+Enter, and checks a task was written, against a sandboxed `TASKDATA`, so your real database is untouched;
- it runs inside a nested niri of its own, served by a daemon started there, because the daemon is the one process that draws;
- it exists because two bugs reached daily use that no unit test could have caught (sub-bullets):
  - the box opened carrying the daemon's `app_id` instead of its own, so every check matching on app-id looked straight past it;
  - Ctrl+Enter did nothing: the key mapping was correct and tested, but the event controller was in the wrong propagation phase, so the focused text view swallowed Return before the window saw it;
- both only exist once a compositor is involved.

**`tests/e2e-tag.sh`:**

- it needs niri to ask for workspaces;
- it pins the property `--session` exists for, which no unit test can see: the tag follows the terminal rather than the focus;
- it hands the binary a session the way herdr hands one to a pane (`HERDR_SESSION`, with `HERDR_SOCKET_PATH` as the backup) rather than starting herdr, so none of your sessions is attached to or created;
- the folder cases run under a throwaway `$HOME`;
- it checks that `niritasks task add` files under the session's workspace from a herdr pane, and under the focused one outside herdr, into a scratch task database it deletes afterwards; your own is never touched;
- it checks that `task start`, `task refine` and `task session` open in the same workspace's herdr session, against a fake `herdr` that logs which session it was asked for and opens nothing.

**`tests/e2e-panel.sh`, the least obvious:**

- the task panel is a layer-shell surface, so its behaviour is what the compositor puts on screen;
- the window can only report the size it *asked* for, and a surface that was never mapped reports nothing wrong while showing nothing;
- the overlay the panel replaced was invisible for whole sessions that way, past every unit test;
- so it screenshots a nested niri of its own and measures the panel against a frame taken with no tasks.

**What it measures**, one bullet each: nothing on the edge until a task is peeked; the peek's width and its notes; an empty edge for another tag's task; a task leaving pending; `--clear`; a restarted daemon and a workspace switch peeking at a stub herdr's focused pane; `--focused`; the keyboard's cards in the middle of the screen and the focused one's buttons; a filter tab's fill; a waiting task on the Waiting tab and off the tucked panel; Clear all taking the focus off the cards on its first Ctrl+Shift+Delete, and Enter then deleting both waiting tasks and nothing else; Escape cancelling Clear all; Ctrl+Enter's refine reaching `task refine`; "+N more" opening on Down; Escape putting them back; nothing for another tag's task; nothing once they are done; a daemon cold-started with nothing to show.

Then one line: which tab, card and button a key leads to, and what it arms, are `src/panel/state.rs`'s unit tests, which need no display.

**Why its numbers are exact**, one bullet each:

- the nested screen is a flat colour at a fixed size;
- with a task peeked, the peek starts at column 1566 of 1600;
- the keyboard's cards span 420-1180;
- a frame that should not have changed (another tag's task, Escape, an empty panel) is compared pixel for pixel;
- a nested niri parked out of sight draws only when asked, so each frame is shot until two in a row agree;
- it counts lines rather than pixels: the cards' shadows fade into the background over many pixels, and a column or row counts only once at least 20 of its pixels changed, which puts the panel's edge at the same column every run.

- [ ] **Step 4: Check shape, facts and tests**

Run: `python3 target/readme-shape.py`. Nothing may be flagged from `### Running the tests in a container` to the end.

Fact audit: `git show cca971a:README.md | sed -n '704,854p'`.

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`. Expected: all ok.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs(readme): list what the container, try.sh and e2e scripts do"
```

---

### Task 10: Whole-file check

**Files:**
- Modify: `README.md` only if a check below fails

- [ ] **Step 1: Shape over the whole file**

Run: `python3 target/readme-shape.py`
Expected: `README shape OK`, exit 0. Fix any offence the per-section checks missed.

- [ ] **Step 2: Headings and anchors held**

Run:

```bash
diff <(git show cca971a:README.md | grep '^#') <(grep '^#' README.md | grep -Fxf <(git show cca971a:README.md | grep '^#'))
grep -c '](#peeking-at-the-focused-panes-task)' README.md
```

Expected: the `diff` prints nothing (every old heading is still there, in order). The `grep -c` prints at least 1.

- [ ] **Step 3: Whole-file fact audit**

Run: `git diff cca971a -- README.md`. Read every removed (`-`) line. For each, find its fact on a `+` line or an unchanged line. Pay extra attention to numbers, paths, versions, flags and key names. To list those mechanically:

```bash
grep -oE '`[^`]+`|[0-9]+(\.[0-9]+)+|[0-9]+(px|s|m|h|d|w)\b' <(git show cca971a:README.md) | sort -u > target/old-tokens
grep -oE '`[^`]+`|[0-9]+(\.[0-9]+)+|[0-9]+(px|s|m|h|d|w)\b' README.md | sort -u > target/new-tokens
comm -23 target/old-tokens target/new-tokens
```

Expected: no output. Any token that shows up must be put back, or shown to still be present in other words (for example, a code span that was split into a table cell).

- [ ] **Step 4: llms.txt line still true**

Run: `sed -n 74p llms.txt`. The README must still cover install, keybinds, the task panel and task box, requirements, and how to run the tests. Check with `grep -nE '^## (Keybinds|Task panel|Install)|^### (Requirements|The task box|Running the tests)$' README.md`. Expected: 6 lines.

- [ ] **Step 5: Tests**

Run: `cargo test --quiet 2>&1 | grep -E 'test result|FAILED'`
Expected: every `test result: ok`, no `FAILED`.

- [ ] **Step 6: Commit (only if Steps 1-4 needed fixes)**

```bash
git add README.md
git commit -m "docs(readme): restore facts the section rewrites dropped"
```
