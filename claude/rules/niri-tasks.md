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
