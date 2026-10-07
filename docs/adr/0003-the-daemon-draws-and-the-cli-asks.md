# The daemon draws; the CLI asks

`niritasks daemon` is the one process that puts anything on screen: the task
panel on each monitor, the task box, and the project list. Every `niritasks`
command that needs a window of this tool's own, the box, the panel or the
project list, sends one line over `$XDG_RUNTIME_DIR/niri-tasks.sock` and, when
nothing answers, exits 1 with `ipc::NO_DAEMON`, which says the daemon is not
running and how to start it. The CLI never draws a window itself; the only
windows it starts are other programs', a terminal or an editor.

Until 2026-10-07 the box was the exception: the CLI built its own GTK window
when no daemon answered, on the argument that "a daemon you can do without is
a cache; one you cannot is a dependency". That argument stopped holding once
the panel, which cannot exist without the daemon, became the one GUI for
acting on a task, and the project list moved into it. Keeping the fallback
meant two copies of the submit logic and a slow path nobody used.

The same day fuzzel left: the task picker and the action menu had already
folded into the panel's action row, and the project picker became the panel's
project list, ranked by fzf when it is installed and by substring match when
it is not. Workspaces are named only by opening a project
(`niritasks project open`), never renamed; the daemon names workspace 1
`general` at start.

## Considered options

- **Keep the CLI-built box as a fallback.** Rejected: the daemon is a
  dependency anyway, and the fallback duplicated add-and-refine and edit.
- **Rebuild the project picker as a separate daemon popup window.** Not
  taken: the panel already had the list machinery (cards, focus, keys), so
  the list became a mode of the panel instead.
- **Keep fuzzel for the project list.** Rejected once the panel list worked:
  one GUI toolkit, and typed rows instead of a string echoed back.

## Consequences

The panel and the box are only ever drawn by one GTK process, so styles,
fonts and the blur are set once. A machine without the daemon running gets
the same refusal from every GUI command. e2e-box runs once, served by a
daemon inside the nested niri, and checks the refusal rather than a fallback.
