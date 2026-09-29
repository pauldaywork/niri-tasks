# niri-tasks

Workspace-scoped Taskwarrior for niri: a workspace's name decides which tasks
belong to it. The vocabulary below exists because "the workspace", "the tag"
and "what's on screen" are easy to blur, and each surface this tool draws has
been called several things.

## Language

### Scoping

**Workspace tag**:
The Taskwarrior tag derived from a niri workspace's name — lowercased, with
every run of other characters collapsed to `_`. An unnamed workspace has none.
_Avoid_: project, label, context (Taskwarrior has its own contexts)

**Focused workspace**:
The one workspace, across all monitors, that has keyboard focus. Keybinds file
tasks under its workspace tag.
_Avoid_: current workspace (ambiguous with active)

**Active workspace**:
The workspace a given monitor is showing. Every monitor has one; only one of
them is also the focused workspace.
_Avoid_: visible workspace, current workspace

**Active task**:
The started pending task on a workspace tag. There is at most one per tag.
_Avoid_: current task, in-progress task

**Planned task**:
A pending task the `refine-task` skill has worked up into a plan — a sharper
description and a consolidated set of notes — marked with the `+planned` tag
and a clipboard-check icon on its task card and picker row.
_Avoid_: ready (Taskwarrior's `+READY` means something else), refined, groomed

### On screen

**Task panel**:
The list of task cards on a monitor's right edge, showing the pending tasks of
that monitor's active workspace. It is tucked away to a peek until hovered,
or until Mod+Alt+Ctrl+T slides it out and hands it the keyboard.
_Avoid_: pill, overlay, widget, sidebar, drawer

**Task card**:
One task in the task panel, drawn like a notification: a status icon and a
one-line description.
_Avoid_: row (that is the picker's word), item, block

**Peek**:
The strip of each task card left showing when the task panel is tucked away.
_Avoid_: sliver, handle, tab

**Picker**:
The fuzzel list of a workspace's tasks, opened from a keybind, for acting on
one. Picking a task, or clicking its task card, opens its action menu.
_Avoid_: menu, launcher

**Task box**:
The GTK window for adding a task or editing one — its description and its
note rows. Edit and Note open the same box.
_Avoid_: dialog, prompt

**Note row**:
One note in the task box: its text, wrapping and editable in place, its date,
and an × to delete it.
_Avoid_: line, annotation (Taskwarrior's word for what a note row is saved as)
