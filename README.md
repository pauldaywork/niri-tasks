# niri-tasks

Workspace-scoped Taskwarrior for [niri](https://github.com/YaLTeR/niri).

The niri workspace you are looking at *is* the task filter. Name a workspace
`website` and every task you add from it is tagged `+website`; the list
shortcut shows that workspace's tasks and nothing else. Switch workspace and the
same keys show a different set.

Which tasks you see goes purely off the workspace name, which `niritasks project open`
sets to the folder name. `~/Projects` is only ever read to offer folders to pick
from — opening one, or moving a task to another workspace.

## Keybinds

| Key | Does |
|---|---|
| `Mod+Alt+T` | Add a task to this workspace, with notes — Enter starts each one |
| `Mod+Alt+Shift+T` | The same, with Add & refine as the default: once added, the task opens in a "Refine: …" tab of the workspace's herdr session, where Claude works it up into a plan |
| `Mod+Alt+Ctrl+T` | Hand the task panel the keyboard, in the middle of the screen: every card shows its whole description, and the focused one has buttons to start working on it in its own worktree, refine it into a plan with Claude, edit it, stop it, park it as waiting, or remove it. Enter opens the full menu (note, grill me, update status, move to another workspace). Ctrl+Enter refines the focused task, or starts working on it once it is planned, and keeps the list up. Tabs above the cards, picked with 1–5 or [ and ], narrow them to All, Active, Planned, To refine or Waiting, and the Waiting tab's Clear all (Shift+Delete) deletes every waiting task. With no tasks, or no daemon, the fuzzel list instead |
| `Mod+Alt+W` | Pick a folder from `~/Projects` (or type a new name to make one), put it on its own named workspace, open a terminal and an editor in it |
| `Mod+Alt+Ctrl+W` | Rename this workspace (and with it, which tag its tasks carry) |

## Task panel

Each monitor shows the pending tasks of the workspace it is displaying, as a
stack of cards on the right edge — the active tasks first (there can be
several, one per worktree being worked), each ▶ and text in green, then the rest most urgent first: ○, a dimmed lock for one blocked on
another task, or ● for one already worked up into a plan with
Claude from the menu's **Refine** or **Grill me**. Up to eight, then a "+N
more" card.

The panel sits tucked away with only a 30px peek of each card showing. Move the
pointer onto it and the cards slide out to full width; move away and they slide
back. The space around the cards is click-through, so the panel never gets in
the way of the windows beneath, and a fullscreen window covers it. An unnamed
workspace, or one with nothing pending, shows no panel at all.

Click a card to open that task's actions in fuzzel — the same menu the picker
shows once you pick a task — on the monitor you clicked. The "+N more" card
shows the rest of the tasks in the panel.

`Mod+Alt+Ctrl+T` hands the panel the keyboard and moves it to the middle of the
screen, leaving no peek on the right edge while it is there. Every card opens up
to its whole description, wrapped, and the focused card shows a row of buttons
under it:

| Button | Key | Does |
|---|---|---|
| **Go to session** (blue) | `g` | The menu's Go to session: back to the Claude working on the task — only while one is, checked as the panel slides out |
| **Start working** (green) | `s` | The menu's Start working: the task's own worktree, herdr tab and Claude — not on an active task, which is already being worked |
| **Refine** (mauve) | `r` | Work it up into a plan with Claude |
| **Edit** (yellow) | `e` | Open it in the task box |
| **Stop** (peach) | `t` | Stop it — only on an active task |
| **Waiting** (teal) | | Park it as waiting: off the panel, and on the Waiting tab instead |
| **Back to list** (lavender) | `b` | Off waiting and back on the list — only on a waiting task, which gets just this, Edit and Remove |
| **Remove** (red) | `Delete` | Turns into **Confirm remove**; a second press deletes the task, moving away puts it back |
| **Clear all** (red) | `Shift+Delete` | On the Waiting tab's bar, not a card: turns into **Confirm clear all** and takes the focus off the cards; a second press, or `Enter`, deletes every waiting task on the workspace and goes back to All, while `Escape` or moving away puts it back |

Up and Down move a darker fill and the buttons between cards; Left, Right and
Tab move along the focused card's buttons, whose name shows beside them while
one is focused, and Enter presses the focused one. Enter on the card itself
opens its full menu, which also has Note, Grill me, Update status and Move to
workspace — led by Go to session while a Claude is working on the task. Every
button gives the keyboard back as it runs, and Escape puts the panel back on the
right edge, tucked away with the cards folded back to one line. A list taller
than the screen scrolls, keeping the focused card in view. Hovering never shows
the buttons — only the keyboard does.

Ctrl+Enter is the one key that does not give the keyboard back. On the focused
card it refines the task, or starts working on it once it is planned, and the
panel stays up with the same card focused, so you can carry on down the list.
It does nothing on a task that is already being worked, or on a waiting task.
Ctrl still held from the `Mod+Alt+Ctrl+T` chord counts, so let go of Ctrl before
pressing Enter if you only want the card's menu. Every other Ctrl chord passes
through untouched.

Above the cards, tabs narrow them while the panel has the keyboard: **All**,
**Active** (started), **Planned** (refined or grilled into a plan, and not
started yet), **To refine** (not planned yet) and **Waiting** (parked). A tab
shows only while it has a task under it, All apart. `1` to `5` pick one, each
always the same tab and doing nothing while it is hidden; `[` and `]` step to
the shown tab either side; a click picks one too. The arrows and Tab still
move only between cards and buttons. A started planned task is under Active,
not Planned. Waiting tasks are on the Waiting tab only: not on All, the hover
or the peek. The panel opens on All every time, and goes back to All when the
tab it is on runs out of tasks. The Waiting tab alone ends in **Clear all**,
which `Shift+Delete` presses (`Delete` alone is still the focused card's
Remove). Like Remove it asks first, as **Confirm clear all**, and its second
press deletes every waiting task on this workspace, one `task status <uuid>
deleted --yes` after another, putting the panel back on All with the keyboard
kept. Armed, it takes the focus off the cards, so no card's buttons show and
`Enter` confirms as a second `Shift+Delete` does; a card's own keys do nothing
until it is put back. `Escape`, an arrow or Tab puts it back with the focus on
the card it was on, and the panel stays up; switching tab puts it back too.

The cards are shaped like mako notifications and take a terminal window's font
and colours: a dark tint over blurred wallpaper, with niri's faint focus-ring outline
(`src/panel/style.rs` names the configs it copies). The blur is asked for by the
panel itself, over `ext-background-effect-v1`, in the exact shape of the cards;
a compositor without that protocol shows them unblurred. The lock icon comes
from a Nerd Font (JetBrainsMono Nerd Font here). See
`docs/adr/0001-task-panel-in-gtk-not-quickshell.md` for why it is drawn the way
it is.

## Install

```bash
git clone https://github.com/pauldaywork/niri-tasks ~/Projects/niri-tasks
cd ~/Projects/niri-tasks
bash install.sh
```

Then add the include to `~/.config/niri/config.kdl`:

```kdl
include "niri-tasks.kdl"
```

`install.sh` builds `niritasks`, symlinks the niri include, the fuzzel picker theme and
the Claude skills (`workspace-tasks`, `refine-task`, `finish-worktree`) into `~/.claude/skills`, a Claude rule into `~/.claude/rules` (use `finish-worktree` to land a worktree branch), restarts the daemon
onto the new binary, and reloads niri. Updating is `git pull && bash install.sh`.

The symlinks mean editing a file in this repo is immediately live — there is no
copy to keep in sync.

### Requirements

`niri`, `taskwarrior`, `fuzzel`, `ghostty` 1.2 or later (for `+new-window`),
and a Rust toolchain to build with. `notify-send` is used for feedback and
degrades to stderr without it.

[herdr](https://herdr.dev) is optional. With it on `$PATH`, `niritasks project
open` runs the workspace's herdr session in the project terminal; without it the
terminal is a plain shell in the project folder. The menu's **Refine** and
**Grill me** need both herdr and Claude Code (`claude`) on `$PATH` — they open
Claude inside that session — plus `bubblewrap` and `socat` for the Bash
sandbox that keeps that Claude to reading the project and writing only the
task (`sudo apt install bubblewrap socat`). Without them Claude refuses to
start rather than run unfenced. **Start working** needs herdr, Claude Code and
[worktrunk](https://worktrunk.dev) (`wt`), and the workspace's `~/Projects`
folder to be a git repository: worktrunk makes the task's worktree and runs the
repo's `.config/wt.toml` hooks, asking in a herdr tab the first time a repo's
hooks need approving.

A Claude you start by hand in a herdr pane is linked to a task when it marks
that task active (`niritasks task status <uuid> active`): its agent is named
`work-<uuid8>`, the name Start working gives its own, so the task's menu can
find it. An agent already named by Refine or Start working keeps its name.

When the work is done, `/finish-worktree` in that worktree's Claude lands it:
it syncs `main` with origin, rebases the branch and tests the result, then
fast-forwards `main`, pushes, marks the task completed with `niritasks task status`,
and removes the worktree. It stops to ask only for a conflict with no obvious
resolution, red tests, or `main` holding commits origin doesn't have.

VS Code is optional. `niritasks project open` starts one on the project folder when
`code` is on `$PATH`, and starts only the terminal when it is not — the lookup
is there because niri answers a spawn of a missing binary with a desktop
notification, which would otherwise fire on every project you opened.

Tested against niri 26.04 and taskwarrior 2.6.2.

## Commands

`niritasks` is usable directly, not just from keybinds. [`llms.txt`](llms.txt)
is the same reference written for agents, with the rules they most often get
wrong.

```
niritasks tag                      # the focused workspace's tag
niritasks tag --session            # the tag of the workspace this terminal was opened on
                                   #   (its herdr session, else its ~/Projects folder)
niritasks task active              # each active task's description, one per line, or nothing
niritasks task list                # the picker
niritasks task list --dry-run      # the picker's rows (marker, description, ¶ for notes, then the Add row), for testing
niritasks task panel               # hand the task panel the keyboard (Mod+Alt+Ctrl+T)
niritasks task menu <uuid>         # one task's actions, as a task card click opens them
niritasks task status <uuid> <state>  # the menu's Update status: active|stopped|waiting|completed|deleted
                                   #   (stopped also brings back a waiting task)
niritasks task status <uuid> deleted --yes  # deleted needs --yes, the menu's confirmation
niritasks task add <text>          # honours taskwarrior attributes: due:friday, priority:H
niritasks task add                 #   with no text, the task box (Mod+Alt+T)
niritasks task add --refine        # the add box, with Add & refine as its default (Mod+Alt+Shift+T)
                                   #   (with text, adds it and refines it straight away)
niritasks task get-text <uuid>     # print its description
niritasks task edit <uuid> <text>  # replaces the description; attributes stay literal
niritasks task edit <uuid>         #   with no text, the task box on its description and notes
niritasks task get-notes <uuid>    # print its notes, one per line: date, two spaces, text
niritasks task note <uuid> <text>  # attaches an annotation, literal too
niritasks task note <uuid>         #   with no text, the task box with the cursor in a new note
niritasks task refine <uuid>       # work it up into a plan with Claude, in the workspace's herdr session
niritasks task refine <uuid> --grill  #   the same, interviewing you first
niritasks task start <uuid>        # its own worktree (task/<slug>-<uuid8>), opened in the herdr session,
                                   #   with Claude planning it; picked again, back to both
niritasks task start <uuid> --here --workspace <name>
                                   # internal: the setup step, run inside the tab `task start` opens
niritasks task session <uuid>      # back to the Claude working on it (work-/task-<uuid8>) in the herdr session
niritasks workspace new            # create a workspace and name it
niritasks workspace rename         # rename the focused workspace, and with it its tag (Mod+Alt+Ctrl+W)
niritasks workspace default        # name workspace 1 "general" if it is unnamed (niri runs it at startup)
niritasks project open             # pick a ~/Projects folder onto its own named workspace (Mod+Alt+W)
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
and every note, and the menu's Note opens it the same way with the cursor in a
new empty row at the end. Each note is a row: it wraps, you edit it in place,
its date sits small at its right end, and × deletes it. "+ Add note" appends a
row. Adding has a second button, **Add & refine**: it adds the task and hands it
straight to Claude's refine-task skill, in a new tab of the workspace's herdr
session, as the menu's Refine does. If the refine fails, the task stays added
and a notification says why.

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

## Notes

`~/.taskrc` is not managed here. The only requirement is that
`data.location` points somewhere `task` can read and write.

The pickers use a stripped-down fuzzel theme (`fuzzel/picker.ini`) passed with
`--config=`, so your own `fuzzel.ini` is untouched. Its half-transparent
background expects a compositor blur behind it; on niri that is a layer rule
matching the `launcher` namespace. That one is yours to add — `launcher` is
fuzzel's namespace, not this tool's, so shipping a rule for it would be
reaching into someone else's surface:

```kdl
layer-rule {
    match namespace="^launcher$"
    background-effect {
        blur true
        xray true
        noise 0.05
        saturation 1.4
    }
}
```

Without it the picker still works, it is just flatter.

The task panel's rule, also in `niri-tasks.kdl`, works differently: it matches
the panel's own namespace, `niri-tasks-panel`, and sets how the blur looks but
never `blur true`. A rule's blur would fill the panel's whole surface, which is
wider than the cards so they have room to slide. The panel asks for blur behind
the cards alone instead, and niri takes noise and saturation from the rule.

## Development

### Running the tests

```bash
bash tests/all.sh           # everything this machine can run       ~100s

cargo test                  # unit, differential, write-path        ~2s
bash tests/e2e-tag.sh       # `niritasks tag --session`, against real niri ~1s
bash tests/e2e-panel.sh     # the task panel, in a nested niri       ~30s
bash tests/e2e-box.sh       # the task box, in a nested niri         ~65s
```

Each exits non-zero on failure, so any of them can go in a loop or a hook.

`tests/all.sh` is the four in one command, run cheapest-and-quietest first. It
checks each suite's prerequisites itself and reports one it cannot run as a
**skip, with the reason** — no niri, no Pillow, no Wayland display — rather than
letting it fail. A skip is not a failure: it exits non-zero only when a suite
that actually ran said no, which is what makes it safe on a machine that can
only run half of it. Neither e2e suite takes the machine over: both run in a
nested niri parked on a spare workspace, and it tells you which one to keep off.

| | Needs | Touches |
|---|---|---|
| `tests/all.sh` | Nothing of its own — whatever is missing is skipped and named | Whatever the suites it ends up running touch |
| `cargo test` | `taskwarrior` on `$PATH`, and `bash` for the differential suite | Nothing. The write-path suite points `TASKDATA` at a scratch directory |
| `tests/e2e-tag.sh` | niri running with **two named workspaces** — one focused, one not (it borrows the spare empty one if not) | At most the name of that spare workspace, taken off again. No herdr session and no task database at all |
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
`NIRITASKS_E2E_KEEP=1` leaves the frames on disk when you need to see what a
failure actually looked like.

It cannot move the pointer, so the hover is checked by hand after a change to
`src/panel/surface.rs`: the peek slides out when the pointer reaches it and back
about 0.4s after it leaves, the slide is smooth, and a click just left of the
peek, or between two cards, lands on the window beneath, while a click on a
card opens its actions on that monitor. And the keyboard, past what the
script measures: `Mod+Alt+Ctrl+T` slides the panel from the right edge to the
middle of the screen, smoothly, with every card wrapped and the first
darkened. The pointer passing over it does not move it. Check also active tasks with Stop and no
Start working; Up and Down move the darkening between cards, Left, Right and
Tab along the buttons; `s` starts working exactly as the menu does, `r`
refines, `e` opens the box, `t` stops; Delete arms Remove and only a second
Delete deletes, while moving away disarms it; Enter on a card opens the menu,
Enter on "+N more" shows the rest, and a list taller than the screen scrolls
with the focus; a click on a filter tab switches the cards and does not take the focus (the focus still falls back to the first card when the focused one is not under the new tab), a started planned task shows under Active but not Planned, and Back to list on a waiting task brings it back to All; a click on Clear all arms it, darkening no card, a second click deletes the waiting tasks, and Up or Down in between puts it back with the focus on the card it was on; arming Clear all puts an armed Remove back, and arming Remove puts Clear all back; Escape tucks it away to the one-line peek.

`e2e-box.sh` runs in the same kind of nested niri (`tests/lib/nested-niri.sh`
starts it for both). Its keys are pressed with `wtype` pointed at that nested
niri, so they never reach your windows, and you can keep working while it
runs — keep off the workspace it is parked on, as for the panel test. It
proves both ways the box opens: served by a daemon running inside the nested
niri, then with none running there, when the CLI builds the box itself. Your
own `niri-tasks` daemon is never stopped.

It cannot click, so after a change to `src/taskbox.rs` check the pointer half by
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
> And to try a change by hand rather than under test, either `bash install.sh`,
> which restarts the daemon for you, or `cargo install --path .` followed by
> `systemctl --user restart niri-tasks` — `cargo install` alone leaves the
> running daemon on the previous binary, so panel changes will not show up.

### Why the three scripts are not cargo tests

`tests/e2e-box.sh` is not a cargo test and cannot be: it needs a running niri, a
Wayland display, and `wtype` to press the keys. It opens the box, types into it,
presses Ctrl+Enter, and checks a task was actually written — against a sandboxed
`TASKDATA`, so your real database is untouched. It runs the whole thing twice,
inside a nested niri of its own, once served by a daemon there and once with
none, because the fallback is the reason this tool does not depend on a daemon.

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
`$HOME`. It never touches the task database.

`tests/e2e-panel.sh` is the third, and the least obvious. The task panel is a
layer-shell surface, so its behaviour is what the compositor puts on screen: the
window can only report the size it *asked* for, and a surface that was never
mapped reports nothing wrong while showing nothing — the overlay the panel
replaced was invisible for whole sessions that way, past every unit test. So it
screenshots a nested niri of its own and measures the panel against a frame taken
with no tasks: the peek's width, the stack's height as tasks are added, the
keyboard's cards in the middle of the screen, its filter tabs (a tab hidden while
empty, the keys skipping hidden tabs and stopping at the ends, reopening on
All, a waiting task on the Waiting tab and off the tucked panel, Clear all arming on its first Shift+Delete and taking the focus off the cards, ignoring a card's key while armed, cancelling on Escape and on a tab switch, and Enter then deleting both waiting tasks and nothing else), and Escape
putting them back,
nothing for another tag's task, nothing once they are done, and a daemon
cold-started with nothing to show.

The nested screen is a flat colour at a fixed size, so its numbers are exact:
the peek starts at column 1566 of 1600, the keyboard's cards span 420-1180, and
a frame that should not have changed — another tag's task, Escape, an empty
panel — is compared pixel for pixel. A nested niri parked out of sight draws
only when asked, so each frame is shot until two in a row agree. It counts
lines rather than pixels: the cards' shadows fade into the background over
many pixels, and a column or row counts only once at least 20 of its pixels
changed, which is what puts the panel's edge at the same column every run.

`tests/differential.rs` runs the original shell pipelines this was ported from
and compares them against the Rust functions over a corpus of awkward workspace
names. It is there because the tag-folding and project-name rules are subtly
different from each other, and the shell versions were the specification.
