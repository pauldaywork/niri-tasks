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
