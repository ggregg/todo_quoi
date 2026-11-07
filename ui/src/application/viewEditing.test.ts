import { describe, expect, it } from 'vitest'
import { defaultView } from '../domain/view'
import { parseTags, setSortKey, setStatus, setTags, toggleColumn, toggleWithMedia } from './viewEditing'

describe('viewEditing', () => {
  it('toggles columns keeping the reference order', () => {
    const view = toggleColumn(toggleColumn(defaultView(), 'doneAt'), 'progress')
    expect(view.columns).toEqual(['tags', 'icons', 'doneAt'])
  })

  it('sets, replaces and removes sort keys', () => {
    let view = setSortKey(defaultView(), 0, { field: 'title', direction: 'asc' })
    view = setSortKey(view, 1, { field: 'updatedAt', direction: 'desc' })
    expect(view.sort.map((key) => key.field)).toEqual(['title', 'updatedAt'])
    view = setSortKey(view, 0, { field: 'updatedAt', direction: 'asc' })
    expect(view.sort).toEqual([{ field: 'updatedAt', direction: 'asc' }])
    expect(setSortKey(view, 0, null).sort).toEqual([{ field: 'manual', direction: 'asc' }])
  })

  it('edits filters', () => {
    let view = setStatus(defaultView(), 'open')
    view = setTags(view, 'tagsOut', ' #Perso, perso  travail ')
    view = toggleWithMedia(view)
    expect(view.filter).toEqual({ status: 'open', tagsIn: [], tagsOut: ['perso', 'travail'], media: 'withMedia' })
  })

  it('parses tags leniently', () => {
    expect(parseTags('')).toEqual([])
    expect(parseTags('#a,b  #A')).toEqual(['a', 'b'])
  })
})
