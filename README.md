# niri-tasks

Workspace-scoped Taskwarrior for [niri](https://github.com/YaLTeR/niri).

The niri workspace you are looking at *is* the task filter. Name a workspace
`website` and every task you add from it is tagged `+website`; the list
shortcut shows that workspace's tasks and nothing else. Switch workspace and the
same keys show a different set.

Which tasks you see goes purely off the workspace name, which `wt project open`
sets to the folder name. `~/Projects` is only ever read to offer folders to pick
from — opening one, or moving a task to another workspace.

## Keybinds

| Key | Does |
|---|---|
| `Mod+Alt+T` | Add a task to this workspace, with notes — one annotation per line |
| `Mod+Alt+L` | List this workspace's tasks — edit, note, delete, complete, set active, move to another workspace |
| `Mod+Alt+P` | Pick a folder from `~/Projects`, put it on its own named workspace, open a terminal and an editor in it |
| `Mod+Alt+W` | Create a new workspace and name it |
| `Mod+Shift+Alt+W` | Rename this workspace (and with it, which tag its tasks carry) |

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

`install.sh` builds `wt`, symlinks the niri include and the fuzzel picker theme,
and reloads niri. Updating is `git pull && bash install.sh`.

The symlinks mean editing a file in this repo is immediately live — there is no
copy to keep in sync.

### Requirements

`niri`, `taskwarrior`, `fuzzel`, `tmux`, and a Rust toolchain to build with.
`notify-send` is used for feedback and degrades to stderr without it.

VS Code is optional. `wt project open` starts one on the project folder when
`code` is on `$PATH`, and starts only the terminal when it is not — the lookup
is there because niri answers a spawn of a missing binary with a desktop
notification, which would otherwise fire on every project you opened.

Tested against niri 26.04 and taskwarrior 2.6.2.

## Commands

`wt` is usable directly, not just from keybinds:

```
wt tag                      # the focused workspace's tag
wt task active              # the active task's description, or nothing
wt task list                # the picker
wt task list --dry-run      # the rows it would show, for scripting and testing
wt task add <text>          # honours taskwarrior attributes: due:friday, priority:H
wt task edit <uuid> <text>  # replaces the description; attributes stay literal
wt task note <uuid> <text>  # attaches an annotation
wt workspace new|rename|default
wt project open
wt tmux-session             # ghostty's `command =`
```

### Two rules that look alike and are not

`wt task add` **word-splits** the description, so taskwarrior parses its own
attribute syntax — `wt task add ship it due:friday` sets a due date.

`wt task edit` and `wt task note` **do not** split. The text goes through as one
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

```bash
cargo test              # unit, differential, and write-path suites
bash tests/e2e-box.sh   # the task box, driven by real keypresses
```

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

`tests/differential.rs` runs the original shell pipelines this was ported from
and compares them against the Rust functions over a corpus of awkward workspace
names. It is there because the tag-folding, tmux-sanitising and project-name
rules are subtly different from each other, and the shell versions were the
specification.
