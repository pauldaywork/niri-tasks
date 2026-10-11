# niri-tasks

## Finishing a worktree branch

To land a finished worktree branch, use the `finish-worktree` skill instead of
`superpowers:finishing-a-development-branch` and its menu, including at the end
of `superpowers:subagent-driven-development` or `executing-plans`.

## Filing a task

Start every Taskwarrior task description you write — `niritasks task add`,
`task add`, an edit, a refine — with a Conventional Commits type, lowercase,
then a colon and a space: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`,
`build`, `ci`, `chore`, `style` or `revert`. Then the description, imperative:
`fix: Keep the daemon to one task box`. A bug is `fix:`; there is no `bug:`.
The type counts toward the ~50 characters a task card shows. Commit messages
follow the commit-message rule in your global instructions, which usually
gives a task's commits the task's type.

## Linking a task's documents

When you save a design spec or an implementation plan for a Taskwarrior
task, link it from the task with a note, its path relative to the
repository: `niritasks task note <uuid> Plan: docs/superpowers/plans/<file>.md`
once the plan is saved, and `Spec: docs/superpowers/specs/<file>.md` for a
design spec. The task card's Go to… lists and opens them, reading the path
in the task's worktree while it has one. When you rewrite a task's notes,
keep its `Spec:`, `Plan:` and `Report:` notes.
