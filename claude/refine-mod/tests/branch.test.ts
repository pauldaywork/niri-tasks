import { describe, expect, test } from 'claude-code/testing'
import {
  branchArgv,
  branchTarget,
  countStatus,
  factsOf,
  gitArgv,
  gitCommand,
  logArgv,
  mergeBaseArgv,
  numstatArgv,
  parseLog,
  parseNumstat,
  statusArgv,
  withGit,
} from '../hooks/branch'
import { parseReport } from '../hooks/report'
import type { BranchInput } from '../hooks/report'
import { BRANCH_MIN, GIT, SHA_A, SHA_B } from './branch-fixtures'

const input = (): BranchInput => {
  const parsed = parseReport(BRANCH_MIN, 'branch')
  if (typeof parsed === 'string' || parsed.branch === undefined) throw new Error(String(parsed))
  return parsed.branch
}

describe('the target', () => {
  test('takes an absolute worktree and a branch name', () => {
    expect(branchTarget('/w/t', 'main')).toEqual({ worktree: '/w/t', base: 'main' })
  })

  test('refuses a relative worktree, an empty base, an option and a space', () => {
    expect(branchTarget('w/t', 'main')).toBeUndefined()
    expect(branchTarget('/w/t', '')).toBeUndefined()
    expect(branchTarget('/w/t', '--output=/x')).toBeUndefined()
    expect(branchTarget('/w/t', 'ma in')).toBeUndefined()
    expect(branchTarget(undefined, 'main')).toBeUndefined()
  })
})

describe('the git commands', () => {
  test('read the worktree with nothing in its config run', () => {
    expect(gitArgv('/w', 'status')).toEqual(['git', '--no-optional-locks', '-C', '/w', '-c', 'core.fsmonitor=false', 'status'])
    expect(branchArgv('/w').slice(6)).toEqual(['rev-parse', '--abbrev-ref', 'HEAD'])
    expect(mergeBaseArgv('/w', 'main').slice(6)).toEqual(['merge-base', 'HEAD', 'main'])
    expect(logArgv('/w', 'mb').slice(6)).toEqual(['log', '--reverse', '--format=%H%x09%s', 'mb..HEAD'])
    expect(numstatArgv('/w', 'mb').slice(6)).toEqual([
      'diff', '--numstat', '--no-renames', '--no-ext-diff', '--no-textconv', 'mb', 'HEAD',
    ])
    expect(statusArgv('/w').slice(6)).toEqual(['status', '--porcelain', '--untracked-files=normal'])
  })

  test('a command is named by its own words in a refusal', () => {
    expect(gitCommand(mergeBaseArgv('/w', 'main'))).toBe('git merge-base HEAD main')
  })
})

describe('reading what git answers', () => {
  test('the log, oldest first, a subject keeping its tabs', () => {
    expect(parseLog(`${SHA_A}\tfeat: a\tb\n${SHA_B}\tfix: c\n`)).toEqual([
      { sha: SHA_A, subject: 'feat: a\tb' },
      { sha: SHA_B, subject: 'fix: c' },
    ])
    expect(parseLog('')).toEqual([])
  })

  test('numstat counts a binary file as no lines, and keeps a path with spaces', () => {
    expect(parseNumstat('3\t1\tsrc/a.rs\n-\t-\timg.png\n10\t0\tdocs/a b.md\n')).toEqual({
      paths: ['src/a.rs', 'img.png', 'docs/a b.md'],
      added: 13,
      removed: 1,
    })
  })

  test('status counts each file with changes not committed', () => {
    expect(countStatus(' M a\n?? b\n')).toBe(2)
    expect(countStatus('')).toBe(0)
  })

  test('the facts name the merge base short', () => {
    expect(factsOf(GIT)).toEqual({
      branch: 'task/feat-new-words-0b8f6a52',
      base: 'main',
      merge_base: 'f00dfee',
      commits: 2,
      files: 2,
      added: 12,
      removed: 3,
      uncommitted: 1,
    })
  })
})

describe('withGit', () => {
  test('gives each commit its short sha and subject, and the branch its facts', () => {
    const given = { ...input(), story: [{ step: 'Wrote the words', commits: ['abc1234', SHA_B.toUpperCase()], points: ['p'] }] }
    expect(withGit(given, GIT)).toEqual({
      ...given,
      story: [
        {
          step: 'Wrote the words',
          commits: [
            { sha: 'abc1234', subject: 'feat: add the words' },
            { sha: 'abd5678', subject: 'test: pin the words' },
          ],
          points: ['p'],
        },
      ],
      facts: factsOf(GIT),
    })
  })

  test('refuses a sha too short, one not on the branch, one told twice, and a commit left out', () => {
    const story = [{ step: 's', commits: ['abc1234', 'abc12', 'fff9999', SHA_A], points: ['p'] }]
    expect(withGit({ ...input(), story }, GIT)).toBe(
      'story[0].commits[1]: abc12 is too short; give at least 7 characters; ' +
        'story[0].commits[2]: fff9999 is not a commit on the branch; ' +
        `story[0].commits[3]: ${SHA_A} is told already, at story[0].commits[0]; ` +
        'the story leaves out 1 commit(s): abd5678; every commit goes in one step',
    )
  })

  test('refuses a prefix two commits share', () => {
    const twin = { sha: `abc1234${'9'.repeat(33)}`, subject: 'x' }
    const story = [{ step: 's', commits: ['abc1234', SHA_B], points: ['p'] }]
    expect(withGit({ ...input(), story }, { ...GIT, log: [...GIT.log, twin] })).toBe(
      'story[0].commits[0]: abc1234 names more than one commit; give more of it; ' +
        'the story leaves out 2 commit(s): abc1234, abc1234; every commit goes in one step',
    )
  })

  test('refuses a map file the branch does not change', () => {
    const map = [{ path: 'src/other.rs', change: 'change' as const, items: [{ name: 'f', does: 'd' }] }]
    expect(withGit({ ...input(), map }, GIT)).toBe('map[0].path: src/other.rs is not a file the branch changes')
  })

  test('refuses a branch with no commits', () => {
    expect(withGit(input(), { ...GIT, log: [] })).toBe('the branch has no commits since main: there is nothing to report')
  })
})
