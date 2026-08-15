# niri-tasks

Workspace-scoped Taskwarrior for [niri](https://github.com/YaLTeR/niri).

The niri workspace you are looking at *is* the task filter. Name a workspace
`keystone` and every task you add from it is tagged `+keystone`; the list
shortcut shows that workspace's tasks and nothing else. Switch workspace and the
same keys show a different set.

Nothing here knows about `~/Projects` — it goes purely off the workspace name,
which `wt project open` sets to the folder name.

## Keybinds

| Key | Does |
|---|---|
| `Mod+Alt+T` | Add a task to this workspace |
| `Mod+Alt+L` | List this workspace's tasks — edit, note, delete, complete, set active |
| `Mod+Alt+P` | Pick a folder from `~/Projects`, put it on its own named workspace, open a terminal in it |
| `Mod+Alt+W` | Create a new workspace and name it |
| `Mod+Shift+Alt+W` | Rename this workspace (and with it, which tag its tasks carry) |

## Install

```bash
git clone https://github.com/<you>/niri-tasks ~/Projects/niri-tasks
cd ~/Projects/niri-tasks
bash install.sh
```

Then add the include to `~/.config/niri/config.kdl`:

```kdl
include "tasks.kdl"
```

`install.sh` builds `wt`, symlinks the niri include and the fuzzel picker theme,
and reloads niri. Updating is `git pull && bash install.sh`.

The symlinks mean editing a file in this repo is immediately live — there is no
copy to keep in sync.

### Requirements

`niri`, `taskwarrior`, `fuzzel`, `tmux`, and a Rust toolchain to build with.
`notify-send` is used for feedback and degrades to stderr without it.

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

## Notes

`~/.taskrc` is not managed here. The only requirement is that
`data.location` points somewhere `task` can read and write.

The pickers use a stripped-down fuzzel theme (`fuzzel/picker.ini`) passed with
`--config=`, so your own `fuzzel.ini` is untouched. Its half-transparent
background expects a compositor blur behind it; on niri that is a layer rule
matching the `launcher` namespace. Without one the picker still works, it is
just flatter.

## Development

```bash
cargo test          # unit, differential, and write-path suites
```

`tests/differential.rs` runs the original shell pipelines this was ported from
and compares them against the Rust functions over a corpus of awkward workspace
names. It is there because the tag-folding, tmux-sanitising and project-name
rules are subtly different from each other, and the shell versions were the
specification.
