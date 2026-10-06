import { describe, expect, test } from 'claude-code/testing'
import {
  CHANGE,
  MAX_LINE,
  WRITE,
  isSandboxed,
  isUuid,
  merge,
  notApproved,
  notAsked,
  overlong,
  parseExport,
  parseInput,
  planLines,
  refusal,
  taskDate,
} from '../hooks/plan'
import type { Task } from '../hooks/plan'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'

const task = (over: Partial<Task> = {}): Task => ({
  uuid: UUID,
  status: 'pending',
  description: 'feat: Old words',
  entry: '20260101T000000Z',
  annotations: [{ entry: '20260101T000001Z', description: 'first note' }],
  tags: ['zeta', 'alpha'],
  ...over,
})

const EXPECTED = { description: 'feat: Old words', notes: ['first note'] }

describe('isUuid', () => {
  test('takes a uuid, any case', () => {
    expect(isUuid(UUID)).toBe(true)
    expect(isUuid(UUID.toUpperCase())).toBe(true)
  })
  test('refuses the rest', () => {
    for (const value of ['', 'abc', `${UUID}x`, undefined, 7]) expect(isUuid(value)).toBe(false)
  })
})

describe('isSandboxed', () => {
  test('needs both flags true', () => {
    expect(isSandboxed({ sandbox: { enabled: true, failIfUnavailable: true } })).toBe(true)
    expect(isSandboxed({ sandbox: { enabled: true } })).toBe(false)
    expect(isSandboxed({ sandbox: { enabled: 'true', failIfUnavailable: true } })).toBe(false)
    expect(isSandboxed({})).toBe(false)
  })
})

describe('parseInput', () => {
  test('takes a well-formed input', () => {
    const input = { expected: EXPECTED, description: 'feat: New', notes: ['a', 'b'] }
    expect(parseInput(input)).toEqual(input)
  })
  test('says what is wrong', () => {
    expect(parseInput({ description: 'x', notes: [] })).toBe('expected must be an object')
    expect(parseInput({ expected: EXPECTED, description: 'x', notes: [1] })).toBe(
      'input.notes must be an array of strings',
    )
    expect(parseInput({ expected: EXPECTED, description: ' ', notes: [] })).toBe(
      'description must not be empty',
    )
    expect(parseInput({ expected: EXPECTED, description: 'a\nb', notes: [] })).toBe(
      'description must be one line',
    )
    expect(parseInput({ expected: EXPECTED, description: 'x', notes: ['a\nb'] })).toBe(
      'each note must be one line',
    )
  })
})

describe('parseExport', () => {
  test('answers the one task, or undefined', () => {
    expect(parseExport(JSON.stringify([task()]))?.uuid).toBe(UUID)
    expect(parseExport('[]')).toBeUndefined()
  })
})

describe('refusal', () => {
  test('none while the task matches', () => {
    expect(refusal(task(), EXPECTED)).toBeUndefined()
  })
  test('a missing or finished task', () => {
    expect(refusal(undefined, EXPECTED)).toBe('the task no longer exists')
    expect(refusal(task({ status: 'completed' }), EXPECTED)).toBe('the task is completed, not pending')
  })
  test('says what changed', () => {
    const why = refusal(task({ description: 'feat: Edited' }), EXPECTED)
    expect(why).toContain('its description is now "feat: Edited", not "feat: Old words"')
    expect(refusal(task({ annotations: [] }), EXPECTED)).toContain(
      'its notes are now [], not ["first note"]',
    )
  })
})

describe('merge', () => {
  test('sets description and notes, adds planned, keeps the rest', () => {
    const now = Date.UTC(2026, 9, 6, 1, 2, 3, 456)
    const merged = merge(task(), { expected: EXPECTED, description: 'feat: New', notes: ['a', 'b'] }, now)
    expect(taskDate(now)).toBe('20261006T010203Z')
    expect(merged).toEqual({
      ...task(),
      description: 'feat: New',
      annotations: [
        { entry: '20261006T010203Z', description: 'a' },
        { entry: '20261006T010204Z', description: 'b' },
      ],
      tags: ['alpha', 'planned', 'zeta'],
    })
  })
  test('does not repeat planned', () => {
    const merged = merge(task({ tags: ['planned'] }), { expected: EXPECTED, description: 'x', notes: [] }, 0)
    expect(merged.tags).toEqual(['planned'])
    expect(merged.annotations).toEqual([])
  })
})

describe('planLines', () => {
  test('one line for the description, one per note, in order', () => {
    expect(planLines({ description: 'feat: New', notes: ['a', 'b'] })).toEqual([
      'Description: feat: New',
      'Note 1: a',
      'Note 2: b',
    ])
  })
  test('says when the task will keep no notes', () => {
    expect(planLines({ description: 'feat: New', notes: [] })).toEqual([
      'Description: feat: New',
      'Notes: none (the task keeps no notes)',
    ])
  })
})

describe('overlong', () => {
  test('none while every line fits', () => {
    expect(overlong(planLines({ description: 'd', notes: ['x'.repeat(MAX_LINE - 'Note 1: '.length)] }))).toBeUndefined()
  })
  test('names the first line past the most the terminal draws', () => {
    const lines = planLines({ description: 'd', notes: ['ok', 'x'.repeat(MAX_LINE - 'Note 2: '.length + 1)] })
    expect(overlong(lines)).toBe(
      'note 2 would show as a line of 2001 characters, over the 2000 the terminal draws. ' +
        'Shorten it or split it into notes, then call again.',
    )
    expect(overlong([`Description: ${'y'.repeat(MAX_LINE)}`])).toStartWith('the description would show')
  })
})

describe('notApproved', () => {
  test('carries the choice, or the typed words', () => {
    expect(notApproved(CHANGE)).toContain(`the person chose "${CHANGE}"`)
    expect(notApproved('fewer notes')).toContain('the person answered "fewer notes" instead of approving')
    expect(notApproved(`${WRITE}, ${CHANGE}`)).toContain('instead of approving')
  })
})

describe('notAsked', () => {
  test('says nothing was written and why', () => {
    expect(notAsked('no one to ask')).toBe(
      'Nothing written: the person could not be asked to approve the plan (no one to ask). ' +
        'write_task_plan writes only after they choose "Write it to the task", ' +
        'which needs someone at an interactive session.',
    )
  })
})
