import { describe, expect, it } from 'vitest'
import { makeTask } from './testSupport'
import { suggestTags, tagUsage } from './tagSuggestions'

const usage = tagUsage([
  makeTask(1, 'a', null, { tags: ['client-x', 'urgent'] }),
  makeTask(2, 'b', null, { tags: ['client-x', 'perso'] }),
  makeTask(3, 'c', null, { tags: ['client-y', 'client-x'] }),
  makeTask(4, 'd', null, { tags: ['maclient'] }),
])

describe('suggestTags', () => {
  it('prefix first, then most used, then alphabetical', () => {
    expect(suggestTags(usage, 'cli', [])).toEqual(['client-x', 'client-y', 'maclient'])
  })

  it('ignores # and case, excludes tags already set', () => {
    expect(suggestTags(usage, '#CLI', ['client-x'])).toEqual(['client-y', 'maclient'])
  })

  it('empty input proposes the most used tags', () => {
    expect(suggestTags(usage, '', [], 2)).toEqual(['client-x', 'client-y'])
  })
})
