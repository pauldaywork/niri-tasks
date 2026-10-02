# To Follow Up: Add & refine, and Ctrl+Enter on the panel

Smaller findings from the reviews of the work that landed in
`e954f19..4d8cc00` on 2026-10-02: Add & refine in the task box (plan:
`docs/superpowers/plans/2026-10-02-add-and-refine.md`) and Ctrl+Enter on the
keyboard panel. None blocked the merge. Each is here to be weighed later, and
either filed as a task or struck off.

The findings worth a task already have one, on `+niri_tasks`:

- `a5552796` Open a spawned refine in the task's own workspace, not whichever is focused
- `0e41d20e` Keep the daemon from opening two task boxes at once
- `0dc2be1f` Match the new task's uuid in the e2e Add & refine check, not just any refine

## 1. Nothing on screen marks the default button

In a box opened with `niritasks task add --refine` (Mod+Alt+Shift+T),
Ctrl+Enter presses Add & refine, but only the hint line says so. Both buttons
look the same.

- Where: `src/taskbox.rs`, the footer in `build_window`.
- Possible fix: a `suggested-action`-style class on whichever button
  Ctrl+Enter presses (`cfg.refine` decides which), with a rule in
  `src/taskbox/style.rs`.

## 2. A note that fails to attach skips the refine

If `task::add_with_notes` adds the task but fails attaching a note, it returns
`Err`. The caller notifies the error, the task stays added, and no refine is
spawned.

- Where: the `Request::Add` arm in `src/daemon.rs`, and `TaskCommand::Add` in
  `src/main.rs` (the `?`).
- Reasonable as it is: the error is reported and the task can be refined from
  the panel. Change it only if refining a task that lost a note turns out to be
  wanted.

## 3. An old daemon ignores `--refine`

A daemon started before this change decodes `add refine` as a plain `add` and
opens an ordinary box, so Ctrl+Enter there adds without refining.

- Only happens when the binary is replaced without restarting the daemon.
  `install.sh` always restarts it, so in practice this is a manual
  `cargo install` without the restart.
- No fix needed unless that becomes a common way to update.

## 4. Two quick Ctrl+Enters on the panel can start two refines

Ctrl+Enter on the keyboard panel spawns `niritasks task refine` (or
`task start`) and keeps the panel up, so the key can be pressed again at once.

- A second refine normally finds the first one's herdr agent and switches to
  its tab. Two presses close enough together can both get past that check
  before either has made the agent, and open two tabs.
- Where: `Panel::advance` in `src/panel/surface.rs`, and `refine::launch` in
  `src/refine.rs`.
- Possible fix: ignore a repeat Ctrl+Enter on the same card for a second or so,
  or have `refine::launch` hold a per-task lock.

## 5. The panel e2e's Start check changes state later checks see

`tests/e2e-panel.sh`'s Ctrl+Enter Start check marks a sandbox task `+planned`,
and the checks after it run against that changed database. They pass today,
but reordering or adding checks could break on it.

- Possible fix: give the Start check a task of its own, or undo the tag after
  the check.
