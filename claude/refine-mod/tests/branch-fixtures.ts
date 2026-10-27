import { withGit } from '../hooks/branch'
import type { Git } from '../hooks/branch'
import { parseReport } from '../hooks/report'
import type { Report } from '../hooks/report'

// The smallest branch report show_task_report takes, as the model gives it:
// its commits named by sha, which the git fixture (Task 2) knows.
export const BRANCH_MIN = {
  title: 'feat: New words',
  summary: 'The branch adds new words.',
  changes: ['Words: the description changes.'],
  sections: [{ kind: 'code', heading: 'The words in code', points: ['The words.'] }],
  files: [{ path: 'src/words.rs', change: 'change', why: 'Holds the words.' }],
  hot_spots: [{ spot: 'The words', why: 'They are new.', files: ['src/words.rs'] }],
  story: [{ step: 'Wrote the words', commits: ['abc1234', 'abd5678'], points: ['The words went in.'] }],
  map: [{ path: 'src/words.rs', change: 'change', items: [{ name: 'words()', does: 'Answers the words.' }] }],
  suite: { runs: [{ suite: 'cargo test', command: 'cargo test', passed: 3, failed: 0 }] },
  new_tests: [{ test: 'words_are_new', file: 'src/words.rs', layer: 'unit', pins: 'The words are new.', result: 'pass' }],
}

export const SHA_A = `abc1234${'0'.repeat(33)}`
export const SHA_B = `abd5678${'1'.repeat(33)}`

// What git says of BRANCH_MIN's branch: its two commits, the files it
// changes, and one file left uncommitted.
export const GIT: Git = {
  branch: 'task/feat-new-words-0b8f6a52',
  base: 'main',
  merge_base: `f00dfee${'2'.repeat(33)}`,
  log: [
    { sha: SHA_A, subject: 'feat: add the words' },
    { sha: SHA_B, subject: 'test: pin the words' },
  ],
  paths: ['src/words.rs', 'README.md'],
  added: 12,
  removed: 3,
  uncommitted: 1,
}

// A branch report as the tool holds it once git has named its commits,
// failing the test where it is refused.
export const branched = (input: Record<string, unknown> = BRANCH_MIN, git: Git = GIT): Report => {
  const parsed = parseReport(input, 'branch')
  if (typeof parsed === 'string') throw new Error(parsed)
  if (parsed.branch === undefined) throw new Error('no branch fields')
  const branch = withGit(parsed.branch, git)
  if (typeof branch === 'string') throw new Error(branch)
  return { ...parsed.report, branch }
}
