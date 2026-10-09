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
In the code, `workspace::Workspace` is a named workspace whose name folds to a
usable tag, built once per command by the focused, session or caller policy;
`dirs::Dirs` is where its folder, and the home, data and cache folders, come
from.
_Avoid_: project, label, context (Taskwarrior has its own contexts)

**Focused workspace**:
The one workspace, across all monitors, that has keyboard focus. Keybinds file
tasks under its workspace tag.
_Avoid_: current workspace (ambiguous with active)

**Active workspace**:
The workspace a given monitor is showing. Every monitor has one; only one of
them is also the focused workspace.
_Avoid_: visible workspace, current workspace

**Session**:
The herdr session a workspace's project terminal attaches to: one per
workspace, where that workspace's agents and terminals run. Refine, Start
working and Go to session each open a task's tab or agent in it, and a pane
inside one knows which session it is in without asking what is focused.
In the code, `session::Session` opens a workspace's session and finds or
starts the Claude in it, and `Session::named` knows a session by name alone,
for the pane link and the panel's agent list. Nothing outside the `session`
module talks to herdr.
_Avoid_: project (the project list's word), terminal (the window showing it), herdr
workspace (a session has several of those)

**Active task**:
A started pending task on a workspace tag. A tag can have several at once,
one per agent working a task in its own worktree; making one active leaves the
others alone.
_Avoid_: current task, in-progress task

**Planned task**:
A pending task the `refine-task` skill has worked up into a plan — a sharper
description and a consolidated set of notes — marked with the `+planned` tag
and a clipboard-check icon on its task card.
_Avoid_: ready (Taskwarrior's `+READY` means something else), refined, groomed

**Up next task**:
A pending task marked as the one to do next, with Taskwarrior's own `+next`
tag, from Up next on its action row. Its task card is
yellow and sits directly under the active tasks, above any priority, unless
it is active itself (green) or waiting; the tag adds urgency, so `task next` lifts it too. Up next again takes the mark off.
_Avoid_: priority, starred, pinned

### On screen

**Task panel**:
The list of task cards on a monitor's right edge, showing the pending tasks of
that monitor's active workspace. It is tucked away to a peek until hovered,
or until Mod+Alt+Ctrl+T hands it the keyboard, which moves it to the middle of the
screen until the keyboard is given back.
_Avoid_: pill, overlay, widget, sidebar, drawer

**Task card**:
One task in the task panel, drawn like a notification: a status icon, a
one-line description and, at its right end, how long ago the task was added
(5m, 3h, 2d, 4w) — or, while the task panel has the keyboard, the whole
description wrapped, above its action row while it has focus. The cards run
active, then up next, then by Taskwarrior priority, newest first, with blocked
tasks under the rest. A finished task's card, on the Finished tab alone, is
dimmed, carries a check, and counts its age from when the task was finished.
While the task panel has the keyboard, Enter or a click on a card's body shows,
dimmed under its description, the task's id and uuid on one line (`#48 ·
<uuid>`, the uuid alone on a finished task, which has no id) and then its
notes, one per note, until a second press or the keyboard is given back.
_Avoid_: row (the action row is part of a card), item, block

**Task action**:
One thing that can be done to a task — Go to session, Back to list, Start
working, Refine, Grill me, Edit, Speak, Up next, Complete, Move to workspace,
Stop, Waiting or Remove — with its words, its icon and the `niritasks` command
it runs. Which ones a task gets goes by its state: a waiting task gets only
Back to list, Edit, Speak, Complete, Move to workspace and Remove, a finished
one only Back to list, Edit, Speak and Remove, and Back to list reopens a
finished one. The action row shows them.
_Avoid_: command (the CLI's word), button, verb

**Action row**:
The buttons along the focused task card's bottom edge while the task panel has
the keyboard; the other cards hide theirs — Go to session, Start working,
Refine, Grill me, Edit, Speak, Up next, Complete, Move to workspace, Stop,
Waiting and Remove, an active task getting Stop in place of Start working, only
a task with a live Claude getting Go to session, a waiting task getting just
Back to list, Edit, Speak, Complete, Move to workspace and Remove, and a
finished one just Back to list, Edit, Speak and Remove — each running its task
action's `niritasks` command. Enter or a click on the card's body shows or
hides the task's id, uuid and notes instead (see Task card).
After the buttons, dimmed, the hint names the keys that change from card to
card: the focused button's, or on the card's body, `Space: view
notes` or `Space: hide notes`; and what Ctrl+Enter does to the task: Refine
until it has a plan, then Start working, and Go to session once it is being
worked with a live Claude. The keys every card shares, Enter or Space and
Ctrl+Delete, are on a line under the list instead.
_Avoid_: toolbar, button bar, quick actions

**Filter tab**:
One of the tabs above the task cards while the task panel has the keyboard —
All, Active, Planned, To refine, Waiting and Finished — narrowing the cards to
the tasks it names, and shown only while it has any, All apart. A filter, not a
status: a started task is under Active and neither Planned nor To refine, To
refine is every unstarted task that is not a planned task, and a waiting task is
under Waiting alone. Finished lists the workspace's last 12 completed tasks, the
most recently finished first, and a finished task is under it alone. The Waiting
tab alone has Clear all (Ctrl+Shift+Delete), on a strip under the tab bar, which
deletes every task under it on a second press, as Remove does one. The Ideas tab
after them is not one.
_Avoid_: tab alone (Tab is also a key), category, view, status

**Ideas tab**:
The last tab above the task cards while the task panel has the keyboard,
after Finished and always shown: in place of the cards, one large text area,
the workspace's notepad for ideas that are not tasks yet, kept per workspace
tag in `$XDG_DATA_HOME/niri-tasks/ideas/<tag>.md`. Not a filter tab: it shows
no cards, and while it is picked every key is typing but Escape, which goes
back to the task list on the tab picked before it, keeping the keyboard, and
Ctrl+[ and Ctrl+], which switch tab. 7 picks it, as ] does from the last
filter tab. Saved a second after typing stops, and again on leaving it or
giving the keyboard back.
_Avoid_: notes (a note row is a task's), scratchpad, memo

**Peek**:
The strip of each task card left showing when the task panel is tucked away.
_Avoid_: sliver, handle, tab

**Project list**:
What the task panel shows in place of the task cards for picking a
`~/Projects` folder, drawn like task cards under a line saying what it is
for, with a text field above them that narrows them to fzf's matches as you
type. Up and Down move the highlight and Enter or a click picks. Move to
workspace lists every folder but the task's own workspace's, and moves the
task there; Escape clears the text, then goes back to the cards on the same
card. Mod+Alt+W lists every folder and then the GitHub repos not cloned
yet, and opens the one picked on its own named workspace, cloning a repo
first. When nothing matches, the line under the title says what Enter will
do with the text, opening, making or cloning, decided by the same rule
`project open` acts on.
It shows with no task and on an unnamed workspace, and Escape clears the
text, then closes the panel.
_Avoid_: picker, menu, folder list

**Mode**:
Which of three screens the task panel is showing while it has the keyboard:
Tasks (the task cards under their filter tabs), Ideas (the notepad) or
Projects (the project list). Without the keyboard it is always on Tasks. In
the code, `PanelState::mode`.
_Avoid_: view, screen (in code), state

**Choice**:
What `project open` does with a name, and so what Enter on the project list
will do with what is typed: open a folder that exists, make a new one, clone
a GitHub repo not cloned yet, refuse it, or nothing. One function decides it
(`Projects::choice`), for the panel's hint and the CLI alike; a name that is
one of your repos and not a folder clones.
_Avoid_: resolution, selection, pick (the act of choosing a row)

**Task box**:
The GTK window for adding a task or editing one — its description and its
note rows. Edit and Note open the same box. When adding, a second button,
Add & refine (Ctrl+Shift+Enter), adds the task and refines it straight away;
Mod+Alt+Shift+T opens the box with that as the default.
Only one is ever open: asking for another brings the open one forward.
_Avoid_: dialog, prompt

**Note row**:
One note in the task box: its text, wrapping and editable in place, its date,
and an × to delete it.
_Avoid_: line, annotation (Taskwarrior's word for what a note row is saved as)
