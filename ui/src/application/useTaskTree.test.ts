import { describe, expect, it, vi } from 'vitest'
import { InMemoryTaskGateway } from '../infrastructure/in-memory/InMemoryTaskGateway'
import { useConfirm } from './useConfirm'
import { useTaskTree } from './useTaskTree'
import { useToast } from './useToast'
import { id, makeTask } from './testSupport'

function setup(
  gateway = new InMemoryTaskGateway([makeTask(1, 'Release'), makeTask(2, 'Recette', 1), makeTask(3, 'Courses')]),
) {
  const confirm = useConfirm()
  const toasts = useToast()
  const tree = useTaskTree(gateway, confirm, toasts)
  return { gateway, confirm, toasts, tree }
}

/** Laisse les promesses en attente avancer jusqu'à la prochaine question ou la fin. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0))

describe('liste de tâches', () => {
  it('charge et sélectionne la première tâche', async () => {
    const { tree } = setup()
    await tree.start()
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Release', 'Courses'])
    expect(tree.selectedId.value).toBe(id(1))
  })

  it('aller sur une sous-tâche la montre dans l’arbre et l’ouvre dans le panneau', async () => {
    const { tree } = setup()
    await tree.start()
    tree.openPanel(id(1))
    tree.navigateTo(id(2))
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Release', 'Recette', 'Courses'])
    expect(tree.selectedId.value).toBe(id(2))
    expect(tree.openedId.value).toBe(id(2))
  })

  it('navigue, déplie et va au premier enfant au clavier', async () => {
    const { tree } = setup()
    await tree.start()
    await tree.handle('expandOrFirstChild')
    await tree.handle('expandOrFirstChild')
    expect(tree.selectedId.value).toBe(id(2))
    await tree.handle('collapseOrParent')
    expect(tree.selectedId.value).toBe(id(1))
  })

  it('crée une sœur après la sélection et passe en renommage', async () => {
    const { tree } = setup()
    await tree.start()
    await tree.handle('newSibling')
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Release', 'Nouvelle tâche', 'Courses'])
    expect(tree.renamingId.value).toBe(tree.selectedId.value)
    await tree.commitRename(tree.selectedId.value!, 'Ménage')
    expect(tree.rows.value[1]?.task.title).toBe('Ménage')
    expect(tree.renamingId.value).toBeNull()
  })

  it('zoome dans une tâche puis remonte', async () => {
    const { tree } = setup()
    await tree.start()
    await tree.handle('zoomIn')
    expect(tree.breadcrumb.value.map((task) => task.title)).toEqual(['Release'])
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Recette'])
    await tree.handle('zoomOut')
    expect(tree.zoomId.value).toBeNull()
    expect(tree.selectedId.value).toBe(id(1))
  })

  it('suppression avec « Annuler » (R5, R7)', async () => {
    const { tree, toasts } = setup()
    await tree.start()
    await tree.handle('delete')
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Courses'])
    expect(toasts.current.value?.undo).toBeDefined()
    await tree.handle('undo')
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Release', 'Courses'])
  })

  it('une erreur du cœur est affichée, pas perdue', async () => {
    const { tree, toasts, gateway } = setup()
    await tree.start()
    vi.spyOn(gateway, 'renameTask').mockRejectedValueOnce({ code: 'storage', message: 'drive inaccessible' })
    await tree.commitRename(id(1), 'Nouveau titre')
    expect(toasts.current.value).toMatchObject({ kind: 'error', message: 'Erreur : drive inaccessible' })
  })
})

describe('dialogues de clôture (les décisions viennent du cœur)', () => {
  it('R2 : demande quoi faire des sous-tâches ouvertes', async () => {
    const { tree, confirm, gateway } = setup()
    await tree.start()
    const close = vi.spyOn(gateway, 'closeTask')
    const pending = tree.toggleDone(id(1))
    await settle()
    expect(confirm.current.value?.question.choices.map((c) => c.id)).toEqual([
      'cancel',
      'taskOnly',
      'withOpenDescendants',
    ])
    confirm.answer('withOpenDescendants')
    await pending
    expect(close).toHaveBeenCalledWith(id(1), 'withOpenDescendants')
  })

  it('R1 : propose le parent renvoyé par le cœur, un niveau à la fois', async () => {
    const { tree, confirm, gateway } = setup()
    await tree.start()
    vi.spyOn(gateway, 'closeTask')
      .mockResolvedValueOnce({ closed: [id(2)], proposeClosingParent: id(1) })
      .mockResolvedValueOnce({ closed: [id(1)], proposeClosingParent: null })
    const pending = tree.toggleDone(id(2))
    await settle()
    expect(confirm.current.value?.question.message).toContain('« Release »')
    confirm.answer('close')
    await pending
    expect(gateway.closeTask).toHaveBeenLastCalledWith(id(1), 'taskOnly')
    expect(confirm.current.value).toBeNull()
  })

  it('R1 : « Plus tard » ne clôture rien', async () => {
    const { tree, confirm, gateway } = setup()
    await tree.start()
    const close = vi.spyOn(gateway, 'closeTask').mockResolvedValueOnce({ closed: [id(2)], proposeClosingParent: id(1) })
    const pending = tree.toggleDone(id(2))
    await settle()
    confirm.answer('later')
    await pending
    expect(close).toHaveBeenCalledTimes(1)
  })

  it('R3 : propose de rouvrir les ancêtres renvoyés par le cœur', async () => {
    const gateway = new InMemoryTaskGateway([
      makeTask(1, 'Release', null, { done: true }),
      makeTask(2, 'Recette', 1, { done: true }),
    ])
    const { tree, confirm } = setup(gateway)
    await tree.start()
    vi.spyOn(gateway, 'reopenTask').mockResolvedValueOnce({ proposeReopening: [id(1)] })
    const pending = tree.toggleDone(id(2))
    await settle()
    confirm.answer('reopen')
    await pending
    expect(gateway.reopenTask).toHaveBeenLastCalledWith(id(1))
  })
})

describe('plusieurs bases (ADR-0009)', () => {
  function twoBases() {
    return setup(
      new InMemoryTaskGateway(
        [
          makeTask(1, 'Courses', null, { base: 'perso' }),
          makeTask(2, 'Client', null, { base: 'pro' }),
          makeTask(3, 'Lot', 2, { base: 'pro' }),
        ],
        ['perso', 'pro'],
      ),
    )
  }

  it('« Tout » shows one section per base, a base alone shows only its tasks', async () => {
    const { tree } = twoBases()
    await tree.start()
    const kinds = tree.displayed.value.map((row) => (row.kind === 'base' ? `[${row.base}]` : row.task.title))
    expect(kinds).toEqual(['[perso]', 'Courses', '[pro]', 'Client'])
    tree.showBases('pro')
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Client'])
    tree.showBases('all')
    tree.toggleBaseSection('perso')
    expect(tree.displayed.value.map((row) => (row.kind === 'base' ? `[${row.base}]` : row.task.title))).toEqual([
      '[perso]',
      '[pro]',
      'Client',
    ])
  })

  it('creates at the top of a base and moves a task to another base with its subtasks', async () => {
    const { gateway, tree } = twoBases()
    await tree.start()
    await tree.addAtEnd('perso')
    const created = (await gateway.listTasks()).tasks.find((task) => task.title === 'Nouvelle tâche')
    expect(created?.base).toBe('perso')

    tree.startDrag(id(2))
    await tree.dropOnBase('perso')
    const tasks = (await gateway.listTasks()).tasks
    expect(tasks.find((task) => task.id === id(2))?.base).toBe('perso')
    expect(tasks.find((task) => task.id === id(3))?.base).toBe('perso')
  })
})

describe('« Ajouter une tâche » selon la sélection', () => {
  it('after the selected task, at the same level; in a selected base section otherwise', async () => {
    const { gateway, tree } = setup(
      new InMemoryTaskGateway(
        [
          makeTask(1, 'Projet', null, { base: 'pro' }),
          makeTask(2, 'Lot', 1, { base: 'pro' }),
          makeTask(3, 'Courses', null, { base: 'perso' }),
        ],
        ['perso', 'pro'],
      ),
    )
    await tree.start()
    tree.toggleExpanded(id(1))
    tree.select(id(2))
    await tree.addFromSelection()
    let created = (await gateway.listTasks()).tasks.find((task) => task.title === 'Nouvelle tâche')
    expect(created).toMatchObject({ parentId: id(1), base: 'pro' })

    await tree.commitRename(created!.id, 'Lot 2')
    tree.selectBase('perso')
    await tree.addFromSelection()
    created = (await gateway.listTasks()).tasks.find((task) => task.title === 'Nouvelle tâche')
    expect(created).toMatchObject({ parentId: null, base: 'perso' })
  })
})
