// The pure half of a branch report's facts: the git commands the mod runs
// on the task's worktree, outside the sandbox, what it keeps of their
// answers, and how the model's commit story is checked against them. The
// model names commits by sha alone; their subjects and the branch's numbers
// come from git, so the deck cannot misstate them. No `$`, so the unit
// tests reach it directly.

import type { Branch, BranchInput, Commit, Facts } from './report'

// The worktree a branch report reads and the branch it is measured
// against, from the session's settings.
export type Target = { worktree: string; base: string }

// The target, or undefined unless the worktree is absolute and the base a
// plain branch name: never an option, so it cannot change what git does.
export const branchTarget = (worktree: unknown, base: unknown): Target | undefined =>
  typeof worktree === 'string' &&
  worktree.startsWith('/') &&
  typeof base === 'string' &&
  /^[^-\s][^\s]*$/.test(base) &&
  !/[\u0000-\u001F\u007F]/.test(base)
    ? { worktree, base }
    : undefined

// Read only, with nothing in the repository's config run for it: no
// fsmonitor hook, and no optional lock taken. Diffs ask for no external
// diff and no textconv (numstatArgv).
export const gitArgv = (worktree: string, ...args: string[]): string[] => [
  'git', '--no-optional-locks', '-C', worktree, '-c', 'core.fsmonitor=false', ...args,
]

// The words gitArgv puts before the command's own.
const PREFIX = 6

export const branchArgv = (worktree: string): string[] => gitArgv(worktree, 'rev-parse', '--abbrev-ref', 'HEAD')
export const mergeBaseArgv = (worktree: string, base: string): string[] => gitArgv(worktree, 'merge-base', 'HEAD', base)
// Oldest first, as the story tells them: the full sha, a tab, the subject.
export const logArgv = (worktree: string, mergeBase: string): string[] =>
  gitArgv(worktree, 'log', '--reverse', '--format=%H%x09%s', `${mergeBase}..HEAD`)
// What is committed on the branch since the merge base; not the worktree's
// uncommitted changes, which statusArgv counts.
export const numstatArgv = (worktree: string, mergeBase: string): string[] =>
  gitArgv(worktree, 'diff', '--numstat', '--no-renames', '--no-ext-diff', '--no-textconv', mergeBase, 'HEAD')
export const statusArgv = (worktree: string): string[] =>
  gitArgv(worktree, 'status', '--porcelain', '--untracked-files=normal')

// A command as a refusal names it: `git` and its own words.
export const gitCommand = (argv: readonly string[]): string => ['git', ...argv.slice(PREFIX)].join(' ')

export type Logged = { sha: string; subject: string }

// Everything the mod reads from git for a branch report.
export type Git = {
  branch: string
  base: string
  merge_base: string
  log: Logged[]
  paths: string[]
  added: number
  removed: number
  uncommitted: number
}

const nonEmpty = (stdout: string): string[] => stdout.split('\n').filter(line => line !== '')

export const parseLog = (stdout: string): Logged[] =>
  nonEmpty(stdout).map(line => {
    const tab = line.indexOf('\t')
    return tab < 0 ? { sha: line, subject: '' } : { sha: line.slice(0, tab), subject: line.slice(tab + 1) }
  })

// `git diff --numstat`: added, removed, path, a tab between; a binary file
// says `-` for both, and counts as no lines.
export const parseNumstat = (stdout: string): { paths: string[]; added: number; removed: number } => {
  const rows = nonEmpty(stdout).map(line => line.split('\t'))
  const lines = (n: string | undefined): number => (n === undefined || n === '-' ? 0 : Number(n))
  return {
    paths: rows.map(([, , ...path]) => path.join('\t')),
    added: rows.reduce((sum, [a]) => sum + lines(a), 0),
    removed: rows.reduce((sum, [, r]) => sum + lines(r), 0),
  }
}

export const countStatus = (stdout: string): number => nonEmpty(stdout).length

// A sha as the deck shows it, and the least of one the model may give.
const SHORT = 7

export const factsOf = (git: Git): Facts => ({
  branch: git.branch,
  base: git.base,
  merge_base: git.merge_base.slice(0, SHORT),
  commits: git.log.length,
  files: git.paths.length,
  added: git.added,
  removed: git.removed,
  uncommitted: git.uncommitted,
})

// The model's branch fields with each commit named by git, and the
// branch's facts; or every problem with them, joined. Every commit on the
// branch goes in exactly one step of the story, and every file the map
// names is one the branch changes.
export const withGit = (input: BranchInput, git: Git): Branch | string => {
  if (git.log.length === 0) return `the branch has no commits since ${git.base}: there is nothing to report`
  const errors: string[] = []
  // Each commit told, by its full sha, and where.
  const told = new Map<string, string>()
  const story = input.story.map((step, i) => ({
    ...step,
    commits: step.commits.map((given, j): Commit => {
      const at = `story[${i}].commits[${j}]`
      const sha = given.trim().toLowerCase()
      if (sha.length < SHORT) {
        errors.push(`${at}: ${given} is too short; give at least ${SHORT} characters`)
        return { sha: given, subject: '' }
      }
      const found = git.log.filter(c => c.sha.startsWith(sha))
      const [commit] = found
      if (found.length !== 1 || commit === undefined) {
        errors.push(
          found.length === 0
            ? `${at}: ${given} is not a commit on the branch`
            : `${at}: ${given} names more than one commit; give more of it`,
        )
        return { sha: given, subject: '' }
      }
      const before = told.get(commit.sha)
      if (before !== undefined) errors.push(`${at}: ${given} is told already, at ${before}`)
      else told.set(commit.sha, at)
      return { sha: commit.sha.slice(0, SHORT), subject: commit.subject }
    }),
  }))
  const left = git.log.filter(c => !told.has(c.sha))
  if (left.length > 0) {
    const named = left.slice(0, 10).map(c => c.sha.slice(0, SHORT)).join(', ')
    errors.push(
      `the story leaves out ${left.length} commit(s): ${named}${left.length > 10 ? ', …' : ''}; every commit goes in one step`,
    )
  }
  input.map.forEach((m, i) => {
    if (!git.paths.includes(m.path)) errors.push(`map[${i}].path: ${m.path} is not a file the branch changes`)
  })
  if (errors.length > 0) return errors.join('; ')
  return { ...input, story, facts: factsOf(git) }
}
