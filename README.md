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
| `Mod+Alt+Ctrl+T` | Hand the task panel the keyboard: every card shows its whole description, with buttons to start working on it in its own worktree, refine it into a plan with Claude, edit it, stop it, or remove it. Enter opens the full menu (note, grill me, update status, move to another workspace). With no tasks, or no daemon, the fuzzel list instead |
| `Mod+Alt+P` | Pick a folder from `~/Projects`, put it on its own named workspace, open a terminal and an editor in it |
| `Mod+Alt+W` | Create a new workspace and name it |
| `Mod+Alt+Ctrl+W` | Rename this workspace (and with it, which tag its tasks carry) |

## Task panel

Each monitor shows the pending tasks of the workspace it is displaying, as a
stack of cards on the right edge — the active tasks first (there can be
several, one per worktree being worked), each ▶ and text in green, then the rest most urgent first: ○, a dimmed lock for one blocked on
another task, or a clipboard-check for one already worked up into a plan with
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

`Mod+Alt+Ctrl+T` slides the panel out and hands it the keyboard. Every card
opens up to its whole description, wrapped, above a row of buttons:

| Button | Key | Does |
|---|---|---|
| **Start working** (green) | `s` | The menu's Start working: the task's own worktree, herdr tab and Claude — not on an active task, which is already being worked |
| **Refine** (mauve) | `r` | Work it up into a plan with Claude |
| **Edit** (yellow) | `e` | Open it in the task box |
| **Stop** (peach) | `t` | Stop it — only on an active task |
| **Remove** (red) | `Delete` | Turns into **Confirm remove**; a second press deletes the task, moving away puts it back |

Up and Down move a darker fill between cards; Left, Right and Tab move
along the focused card's buttons, whose name shows beside them while one is
focused, and Enter presses the focused one. Enter on
the card itself opens its full menu, which also has Note, Grill me, Update
status and Move to workspace — led by Go to session while a Claude is working
on the task. Every button gives the keyboard back as it runs,
and Escape tucks the panel away, folding the cards back to one line. A list
taller than the screen scrolls, keeping the focused card in view. Hovering
never shows the buttons — only the keyboard does.

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

`niritasks` is usable directly, not just from keybinds:

```
niritasks tag                      # the focused workspace's tag
niritasks tag --session            # the tag of the workspace this terminal was opened on
                                   #   (its herdr session, else its ~/Projects folder)
niritasks task active              # each active task's description, one per line, or nothing
niritasks task list                # the picker
niritasks task panel               # hand the task panel the keyboard (Mod+Alt+Ctrl+T)
niritasks task list --dry-run      # the rows it would show, for scripting and testing
niritasks task menu <uuid>         # one task's actions, as a task card click opens them
niritasks task status <uuid> <state>  # the menu's Update status: active|stopped|waiting|completed|deleted
                                   #   (stopped also brings back a waiting task)
                                   #   (uuid or its first 8 chars; deleted needs --yes)
niritasks task add <text>          # honours taskwarrior attributes: due:friday, priority:H
niritasks task edit <uuid> <text>  # replaces the description; attributes stay literal
niritasks task note <uuid> <text>  # attaches an annotation
niritasks task refine <uuid>       # work it up into a plan with Claude, in the workspace's herdr session
niritasks task refine <uuid> --grill  #   the same, interviewing you first
niritasks task start <uuid>        # its own worktree (task/<slug>-<uuid8>), opened in the herdr session,
                                   #   with Claude planning it; picked again, back to both
niritasks task session <uuid>      # back to the Claude working on it (work-/task-<uuid8>) in the herdr session
niritasks workspace new|rename|default
niritasks project open
niritasks terminal                 # a terminal in the focused workspace's ~/Projects folder (Mod+Return)
```

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
row.

| Key | Does |
|---|---|
| Enter in the description | Moves to the first note (making one if there are none) |
| Enter in a note | Adds a row below and moves to it |
| Backspace in an empty row | Deletes it and moves up |
| Ctrl+Enter | Saves, from anywhere |
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
bash tests/all.sh           # everything this machine can run       ~105s

cargo test                  # unit, differential, write-path        ~2s
bash tests/e2e-tag.sh       # `niritasks tag --session`, against real niri ~1s
bash tests/e2e-panel.sh     # the task panel, measured in pixels    ~35s
bash tests/e2e-box.sh       # the task box, driven by real keys     ~65s
```

Each exits non-zero on failure, so any of them can go in a loop or a hook.

`tests/all.sh` is the four in one command, run cheapest-and-quietest first. It
checks each suite's prerequisites itself and reports one it cannot run as a
**skip, with the reason** — no niri, no Pillow, no Wayland display — rather than
letting it fail. A skip is not a failure: it exits non-zero only when a suite
that actually ran said no, which is what makes it safe on a machine that can
only run half of it. It warns you before the two that take the machine over.

| | Needs | Touches |
|---|---|---|
| `tests/all.sh` | Nothing of its own — whatever is missing is skipped and named | Whatever the suites it ends up running touch |
| `cargo test` | `taskwarrior` on `$PATH`, and `bash` for the differential suite | Nothing. The write-path suite points `TASKDATA` at a scratch directory |
| `tests/e2e-tag.sh` | niri running with **two named workspaces** — one focused, one not (it borrows the spare empty one if not) | At most the name of that spare workspace, taken off again. No herdr session and no task database at all |
| `tests/e2e-panel.sh` | niri, `python3-pil`, taskwarrior; `wtype` for its Escape check, which is skipped without it | Screenshots, so **your clipboard**; and the `niri-tasks` daemon. It removes every screenshot it takes and leaves the rest of the directory alone |
| `tests/e2e-box.sh` | `wtype`, a Wayland session, niri | **Your keyboard**, and the `niri-tasks` daemon |

Narrowing `cargo test` works as usual — `cargo test --lib`, `cargo test --test
write_path`, `cargo test tag::` for one module, `-- --nocapture` to see output.

`e2e-panel.sh` needs the middle of the right edge to hold still — it works by
comparing frames — so it checks that first and tells you what to move rather
than reporting a flaky answer. Don't switch workspaces while it runs: the panel
follows the workspace, and so does the tag it files its tasks under. And not
over a fullscreen window, which covers the panel. `NIRITASKS_E2E_KEEP=1` leaves
the frames on disk when you need to see what a failure actually looked like.

It cannot move the pointer, so the hover is checked by hand after a change to
`src/panel/surface.rs`: the peek slides out when the pointer reaches it and back
about 0.4s after it leaves, the slide is smooth, and a click just left of the
peek, or between two cards, lands on the window beneath, while a click on a
card opens its actions on that monitor. And the keyboard, past what the script
measures: `Mod+Alt+Ctrl+T` slides the panel out with every card wrapped and the
first darkened, active tasks with Stop and no Start working; Up and Down move
the darkening between cards, Left, Right and Tab along the buttons; `s` starts
working exactly as the menu does, `r` refines, `e` opens the box, `t` stops;
Delete arms Remove and only a second Delete deletes, while moving away disarms
it; Enter on a card opens the menu, Enter on "+N more" shows the rest, and a
list taller than the screen scrolls with the focus; Escape tucks it away to the
one-line peek.

Two things about `e2e-box.sh` in particular. It **types into whatever has
focus**, so start it and leave the keyboard alone until it finishes; anything
you type lands in the box alongside it. And it stops `niri-tasks.service`, runs
its own daemon for the first half, then starts the service again if it was
running — that is deliberate, since the point is to prove both the daemon path
and the fallback, but it means the task panel blinks out for a minute.

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
once served by the daemon and once with the daemon stopped, because the fallback
is the reason this tool does not depend on a daemon.

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
screenshots the right edge and measures the panel against a baseline taken with
no tasks: the peek's width, the stack's height as tasks are added, nothing for
another tag's task, nothing once they are done, and a daemon cold-started with
nothing to show.

It counts lines rather than pixels. The cards are translucent, so much of them
differs from the wallpaper by only a few levels while a window repainting
elsewhere differs by a lot — but a card is a solid block, so every column and
row through it changes over most of its run and noise never does. The spans of
such columns and rows are the panel's width and height.

`tests/differential.rs` runs the original shell pipelines this was ported from
and compares them against the Rust functions over a corpus of awkward workspace
names. It is there because the tag-folding and project-name rules are subtly
different from each other, and the shell versions were the specification.
