import { describe, expect, it } from 'vitest'
import { defaultView, type ListLayout } from '../domain/view'
import { TaskIndex, visibleRows } from './treeView'
import { id, makeTask } from './testSupport'

const tasks = [
  makeTask(1, 'Release'),
  makeTask(2, 'Recette', 1),
  makeTask(3, 'Scénarios', 2, { done: true }),
  makeTask(4, 'Déploiement', 1, { done: true }),
  makeTask(5, 'Courses'),
  makeTask(6, 'Orpheline', 99), // parent introuvable
]

const titles = (rows: { task: { title: string } }[]) => rows.map((row) => row.task.title)

describe('affichage de l’arbre', () => {
  it('ne montre que les enfants des nœuds dépliés', () => {
    const index = new TaskIndex(tasks)
    expect(titles(visibleRows(index, null, new Set()))).toEqual(['Release', 'Courses', 'Orpheline'])
    expect(titles(visibleRows(index, null, new Set([id(1), id(2)])))).toEqual([
      'Release',
      'Recette',
      'Scénarios',
      'Déploiement',
      'Courses',
      'Orpheline',
    ])
  })

  it('une tâche au parent introuvable apparaît au premier niveau (comme en Rust)', () => {
    expect(new TaskIndex(tasks).parentOf(tasks[5]!)).toBeNull()
  })

  it('calcule la progression des sous-tâches', () => {
    const [release] = visibleRows(new TaskIndex(tasks), null, new Set())
    expect(release).toMatchObject({ childCount: 2, doneChildCount: 1 })
  })

  it('le zoom affiche les enfants de la tâche zoomée', () => {
    expect(titles(visibleRows(new TaskIndex(tasks), id(1), new Set()))).toEqual(['Recette', 'Déploiement'])
  })

  it('les ancêtres vont du parent vers la racine', () => {
    expect(new TaskIndex(tasks).ancestors(id(3)).map((t) => t.title)).toEqual(['Recette', 'Release'])
  })
})

describe('treeView with list layouts', () => {
  it('follows the order and filtering decided by the core, progress stays complete', () => {
    const tasks = [makeTask(1, 'Projet'), makeTask(2, 'A', 1, { done: true }), makeTask(3, 'B', 1), makeTask(4, 'C', 1)]
    const layouts: ListLayout[] = [
      { taskId: null, base: 'principale', view: defaultView(), source: { kind: 'own' }, children: [id(1)] },
      { taskId: id(1), base: null, view: defaultView(), source: { kind: 'root' }, children: [id(4), id(3)] },
    ]
    const index = new TaskIndex(tasks, layouts, { [id(3)]: ['image'] })
    const rows = visibleRows(index, null, new Set([id(1)]), 'principale')
    expect(rows.map((row) => row.task.title)).toEqual(['Projet', 'C', 'B'])
    expect(rows[0]).toMatchObject({ childCount: 3, doneChildCount: 1 })
    expect(rows[2]?.media).toEqual(['image'])
  })
})
