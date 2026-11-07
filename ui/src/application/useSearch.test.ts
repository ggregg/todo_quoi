import { describe, expect, it, vi } from 'vitest'
import { InMemoryTaskGateway } from '../infrastructure/in-memory/InMemoryTaskGateway'
import { highlightParts, useSearch } from './useSearch'
import { makeTask } from './testSupport'
import type { SearchHit } from '../domain/task'

const tasks = [makeTask(1, 'Projet A'), makeTask(2, 'Préparer la release', 1), makeTask(3, 'Courses')]

function setup() {
  const opened: SearchHit[] = []
  const revealed: SearchHit[] = []
  const search = useSearch(
    new InMemoryTaskGateway(tasks),
    { open: (hit) => opened.push(hit), reveal: (hit) => revealed.push(hit) },
    0,
  )
  return { search, opened, revealed }
}

async function typeAndWait(search: ReturnType<typeof setup>['search'], text: string) {
  search.setQuery(text)
  await vi.waitFor(() => expect(search.searching.value).toBe(false))
  await new Promise((resolve) => setTimeout(resolve, 5))
}

describe('useSearch', () => {
  it('finds tasks ignoring case and accents, with their path', async () => {
    const { search } = setup()
    await typeAndWait(search, 'PREPARER')
    expect(search.results.value.map((hit) => hit.title)).toEqual(['Préparer la release'])
    expect(search.results.value[0]?.path).toEqual(['Projet A'])
  })

  it('Enter opens, Mod+Enter reveals and leaves the search, Escape clears', async () => {
    const { search, opened, revealed } = setup()
    await typeAndWait(search, 'r')
    expect(search.results.value.length).toBeGreaterThan(1)
    search.handleKey({ key: 'ArrowDown', metaKey: false, ctrlKey: false })
    expect(search.activeIndex.value).toBe(1)
    search.handleKey({ key: 'Enter', metaKey: false, ctrlKey: false })
    expect(opened).toHaveLength(1)
    search.handleKey({ key: 'Enter', metaKey: true, ctrlKey: false })
    expect(revealed).toHaveLength(1)
    expect(search.active.value).toBe(false)
    await typeAndWait(search, 'courses')
    search.handleKey({ key: 'Escape', metaKey: false, ctrlKey: false })
    expect(search.results.value).toEqual([])
  })

  it('a blank query shows nothing', async () => {
    const { search } = setup()
    await typeAndWait(search, '   ')
    expect(search.active.value).toBe(false)
    expect(search.results.value).toEqual([])
  })
})

describe('highlightParts', () => {
  it('splits by characters, emoji included', () => {
    expect(highlightParts('🎙 été ok', [{ start: 2, end: 5 }])).toEqual([
      { text: '🎙 ', hit: false },
      { text: 'été', hit: true },
      { text: ' ok', hit: false },
    ])
  })

  it('ignores out-of-range or overlapping positions', () => {
    expect(highlightParts('abc', [{ start: 1, end: 9 }])).toEqual([{ text: 'abc', hit: false }])
  })
})
