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
| `Mod+Alt+T` | Add a task to this workspace, with notes — one annotation per line |
| `Mod+Alt+Ctrl+T` | List this workspace's tasks — edit, note, delete, complete, set active, move to another workspace |
| `Mod+Alt+P` | Pick a folder from `~/Projects`, put it on its own named workspace, open a terminal and an editor in it |
| `Mod+Alt+W` | Create a new workspace and name it |
| `Mod+Alt+Ctrl+W` | Rename this workspace (and with it, which tag its tasks carry) |

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
the `workspace-tasks` skill into `~/.claude/skills`, restarts the overlay daemon
onto the new binary, and reloads niri. Updating is `git pull && bash install.sh`.

The symlinks mean editing a file in this repo is immediately live — there is no
copy to keep in sync.

### Requirements

`niri`, `taskwarrior`, `fuzzel`, `ghostty` 1.2 or later (for `+new-window`),
and a Rust toolchain to build with. `notify-send` is used for feedback and
degrades to stderr without it.

[herdr](https://herdr.dev) is optional. With it on `$PATH`, `niritasks project
open` runs the workspace's herdr session in the project terminal; without it the
terminal is a plain shell in the project folder.

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
niritasks task active              # the active task's description, or nothing
niritasks task list                # the picker
niritasks task list --dry-run      # the rows it would show, for scripting and testing
niritasks task add <text>          # honours taskwarrior attributes: due:friday, priority:H
niritasks task edit <uuid> <text>  # replaces the description; attributes stay literal
niritasks task note <uuid> <text>  # attaches an annotation
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

There is a third rule, in the add box's notes area: **line breaks are kept**,
and each line becomes its own annotation. A description collapses its newlines
away because taskwarrior descriptions are one line; notes are a list, so the
breaks are what say where one note ends and the next begins. Blank lines are
dropped rather than filed as empty notes.

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

The active-task overlay makes the same trade and comes with its own layer rule
already written, in `niri-tasks.kdl`, matching the namespace `^niri-tasks$` —
same four settings as above, so the two surfaces read alike. The overlay sets
that namespace itself (`overlay.rs`, `NAMESPACE`) rather than taking the crate
default of `gtk4-layer-shell`, which would match every GTK layer-shell app on
the system. The rule blurs with `xray true`, so what is blurred is the
wallpaper rather than whichever window happens to be under the text, and the
readout looks the same wherever it is shown. Drop the rule and the overlay
still works, it is just flat.

## Development

### Running the tests

```bash
bash tests/all.sh           # everything this machine can run       ~105s

cargo test                  # unit, differential, write-path        ~2s
bash tests/e2e-tag.sh       # `niritasks tag --session`, against real niri ~1s
bash tests/e2e-overlay.sh   # the pill, measured in pixels          ~35s
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
| `tests/e2e-overlay.sh` | niri, `python3-pil`, taskwarrior | Screenshots, so **your clipboard**; and the `niri-tasks` daemon. It removes every screenshot it takes and leaves the rest of the directory alone |
| `tests/e2e-box.sh` | `wtype`, a Wayland session, niri | **Your keyboard**, and the `niri-tasks` daemon |

Narrowing `cargo test` works as usual — `cargo test --lib`, `cargo test --test
write_path`, `cargo test tag::` for one module, `-- --nocapture` to see output.

`e2e-overlay.sh` needs the bottom of the screen to hold still — it works by
comparing frames — so it checks that first and tells you what to move rather
than reporting a flaky answer. Don't switch workspaces while it runs: the
overlay follows the focused workspace, and so does the tag it files its tasks
under. `NIRITASKS_E2E_KEEP=1` leaves the frames on disk when you need to see what a
failure actually looked like.

It measures a strip of screen, so it follows `NIRITASKS_OVERLAY_MARGIN` rather than
assuming the default — set the same value you set in the unit file and it moves
the strip and the daemon it starts together:

```bash
NIRITASKS_OVERLAY_MARGIN=56 bash tests/e2e-overlay.sh
```

Left unset, both sit at the default 10. Note that a larger margin puts the strip
over whatever window is there, and the stillness check will refuse to run if
that window is redrawing.

Two things about `e2e-box.sh` in particular. It **types into whatever has
focus**, so start it and leave the keyboard alone until it finishes; anything
you type lands in the box alongside it. And it stops `niri-tasks.service`, runs
its own daemon for the first half, then starts the service again if it was
running — that is deliberate, since the point is to prove both the daemon path
and the fallback, but it means the overlay blinks out for a minute.

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
> running daemon on the previous binary, so overlay changes will not show up.

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

`tests/e2e-overlay.sh` is the third, and the least obvious. The overlay is a
layer-shell surface, so its behaviour is what the compositor puts on screen: the
window can only report the size it *asked* for, which is exactly what has been
wrong twice — once pinned to 80 characters whatever the task said, once keeping
the previous task's width so a long description ellipsised to "mak…" inside a
short pill. Both passed every unit test. So it screenshots the pill and measures
it, against a baseline taken with no task active.

It counts columns rather than pixels. The pill is translucent, so most of it
differs from the wallpaper by only a few levels while a window repainting
elsewhere differs by a lot — but the pill is a solid band ~33px tall, so every
column inside it changes down most of its height and noise never does. The
distance between the first and last such column is the pill's width. Both old
bugs were reintroduced on purpose to confirm the checks catch them.

`tests/differential.rs` runs the original shell pipelines this was ported from
and compares them against the Rust functions over a corpus of awkward workspace
names. It is there because the tag-folding and project-name rules are subtly
different from each other, and the shell versions were the specification.
