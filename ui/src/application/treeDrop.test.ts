import { describe, expect, it } from 'vitest'
import { TaskIndex } from './treeView'
import { dropAtEnd, dropOnTask, dropZoneAt } from './treeDrop'
import { id, makeTask } from './testSupport'

// 1 Projet ; 2 A (dans 1) ; 3 B (dans 1) ; 4 Courses ; 5 C (dans 3)
const index = new TaskIndex([
  makeTask(1, 'Projet'),
  makeTask(2, 'A', 1),
  makeTask(3, 'B', 1),
  makeTask(4, 'Courses'),
  makeTask(5, 'C', 3),
])

describe('treeDrop', () => {
  it('splits a row into before / inside / after', () => {
    expect([0.1, 0.5, 0.9].map(dropZoneAt)).toEqual(['before', 'inside', 'after'])
  })

  it('drops before, after and inside a task', () => {
    expect(dropOnTask(index, id(4), id(3), 'before')).toEqual({ kind: 'to', parentId: id(1), afterId: id(2) })
    expect(dropOnTask(index, id(4), id(2), 'before')).toEqual({ kind: 'to', parentId: id(1), afterId: null })
    expect(dropOnTask(index, id(4), id(2), 'after')).toEqual({ kind: 'to', parentId: id(1), afterId: id(2) })
    expect(dropOnTask(index, id(4), id(1), 'inside')).toEqual({ kind: 'to', parentId: id(1), afterId: id(3) })
  })

  it('refuses to drop a task into itself or its descendants', () => {
    expect(dropOnTask(index, id(1), id(1), 'inside')).toBeNull()
    expect(dropOnTask(index, id(1), id(5), 'after')).toBeNull()
    expect(dropAtEnd(index, id(3), id(5))).toBeNull()
  })

  it('detaches a task by dropping it at the end of the top-level list', () => {
    expect(dropAtEnd(index, id(5), null)).toEqual({ kind: 'to', parentId: null, afterId: id(4) })
  })

  it('moving a task just after its previous sibling ignores itself', () => {
    expect(dropOnTask(index, id(3), id(2), 'after')).toEqual({ kind: 'to', parentId: id(1), afterId: id(2) })
  })
})

describe('treeDrop et archives affichées (R15)', () => {
  // 1 Projet ; 2 A (dans 1, archivée) ; 3 B (dans 1) ; 4 Archivée (premier niveau) ; 5 Courses
  const archivedAt = '2026-11-20T09:00:00.000Z'
  const withArchives = new TaskIndex([
    makeTask(1, 'Projet'),
    makeTask(2, 'A', 1, { archivedAt }),
    makeTask(3, 'B', 1),
    makeTask(4, 'Archivée', null, { archivedAt }),
    makeTask(5, 'Courses'),
  ])

  it('on ne lâche ni sur une archive ni dedans', () => {
    expect(dropOnTask(withArchives, id(5), id(4), 'after')).toBeNull()
    expect(dropOnTask(withArchives, id(5), id(4), 'inside')).toBeNull()
    expect(dropAtEnd(withArchives, id(5), id(4))).toBeNull()
  })

  it('une archive ne sert pas de repère', () => {
    expect(dropOnTask(withArchives, id(5), id(3), 'before')).toEqual({ kind: 'to', parentId: id(1), afterId: null })
    expect(dropAtEnd(withArchives, id(3), null)).toEqual({ kind: 'to', parentId: null, afterId: id(5) })
  })
})
