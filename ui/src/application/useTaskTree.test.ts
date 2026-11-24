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

describe('filtre rapide « Toutes / Ouvertes / Terminées »', () => {
  it('une liste qui hérite reçoit sa propre vue, avec ce statut', async () => {
    const { tree, gateway } = setup()
    await tree.start()
    await tree.handle('zoomIn')
    expect(tree.currentLayout.value?.source.kind).toBe('root')
    const customize = vi.spyOn(gateway, 'customizeListView')
    await tree.setStatusFilter('open')
    expect(customize).toHaveBeenCalledTimes(1)
    expect(customize.mock.calls[0]?.slice(0, 2)).toEqual([id(1), null])
    expect(tree.currentLayout.value?.source.kind).toBe('own')
    expect(tree.statusFilter.value).toBe('open')
    // Rien ne change si le statut est déjà celui-là.
    await tree.setStatusFilter('open')
    expect(customize).toHaveBeenCalledTimes(1)
  })

  it('en mode « Tout », chaque base affichée change sa vue racine', async () => {
    const gateway = new InMemoryTaskGateway(
      [makeTask(1, 'Courses', null, { base: 'perso' }), makeTask(2, 'Client', null, { base: 'pro' })],
      ['perso', 'pro'],
    )
    const { tree } = setup(gateway)
    await tree.start()
    expect(tree.statusFilter.value).toBe('all')
    const customize = vi.spyOn(gateway, 'customizeListView')
    await tree.setStatusFilter('done')
    expect(customize.mock.calls.map((call) => call.slice(0, 2))).toEqual([
      [null, 'perso'],
      [null, 'pro'],
    ])
    expect(tree.statusFilter.value).toBe('done')

    // Une seule base change : en « Tout », aucun statut commun n'est affiché.
    tree.showBases('pro')
    await tree.setStatusFilter('open')
    tree.showBases('all')
    expect(tree.statusFilter.value).toBeNull()
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

describe('archives (R13, R16)', () => {
  it('archiver une tâche ouverte demande confirmation, puis « Annuler » la restaure', async () => {
    const { tree, confirm, toasts } = setup()
    await tree.start()
    const done = tree.archive(id(1))
    await settle()
    expect(confirm.current.value?.question.message).toBe(
      'Archiver aussi les 2 tâche(s) encore ouverte(s) de « Release » ?',
    )
    confirm.answer('archive')
    await done
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Courses'])
    expect(toasts.current.value?.message).toBe('« Release » archivée')
    await toasts.current.value?.undo?.()
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Release', 'Courses'])
  })

  it('refuser la confirmation n’archive rien', async () => {
    const { tree, confirm } = setup()
    await tree.start()
    const done = tree.archive(id(3))
    await settle()
    expect(confirm.current.value?.question.message).toBe('« Courses » n’est pas terminée. L’archiver quand même ?')
    confirm.answer('cancel')
    await done
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Release', 'Courses'])
  })

  it('une tâche terminée s’archive sans question', async () => {
    const gateway = new InMemoryTaskGateway([makeTask(1, 'Livrée', null, { done: true }), makeTask(2, 'Courses')])
    const { tree, confirm } = setup(gateway)
    await tree.start()
    await tree.archive(id(1))
    expect(confirm.current.value).toBeNull()
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Courses'])
  })
})

describe('archives affichées (R15, R17)', () => {
  /** « Release » (et sa sous-tâche) archivée ; stockage local simulé. */
  async function withArchive() {
    const saved = new Map<string, string>()
    const storage = {
      getItem: (key: string) => saved.get(key) ?? null,
      setItem: (key: string, value: string) => void saved.set(key, value),
    }
    const gateway = new InMemoryTaskGateway([
      makeTask(1, 'Release', null, { done: true }),
      makeTask(2, 'Recette', 1, { done: true }),
      makeTask(3, 'Courses'),
    ])
    const toasts = useToast()
    const tree = useTaskTree(gateway, useConfirm(), toasts, { storage })
    await tree.start()
    await tree.archive(id(1))
    return { gateway, toasts, tree, saved, storage }
  }

  it('le bouton « Archives » les montre à leur place, sans les compter, et le choix est retenu', async () => {
    const { tree, gateway, saved, storage } = await withArchive()
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Courses'])
    await tree.toggleArchives()
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Release', 'Courses'])
    expect(tree.counts.value).toEqual({ total: 1, open: 1 })
    expect(saved.get('todoquoi.archives')).toBe('shown')

    const again = useTaskTree(gateway, useConfirm(), useToast(), { storage })
    await again.start()
    expect(again.archiveVisibility.value).toBe('shown')
    expect(again.rows.value).toHaveLength(2)
  })

  it('une archive est en lecture seule : un message au lieu de la modifier', async () => {
    const { tree, toasts, gateway } = await withArchive()
    await tree.toggleArchives()
    const reopen = vi.spyOn(gateway, 'reopenTask')
    await tree.toggleDone(id(1))
    expect(reopen).not.toHaveBeenCalled()
    expect(toasts.current.value?.message).toBe('Archive en lecture seule : restaurez-la pour la modifier')
    tree.select(id(1))
    await tree.handle('rename')
    expect(tree.renamingId.value).toBeNull()
    tree.startDrag(id(1))
    expect(tree.draggingId.value).toBeNull()
  })

  it('« Ajouter une tâche » avec une archive sélectionnée ajoute à la fin de la liste', async () => {
    const { tree, gateway } = await withArchive()
    await tree.toggleArchives()
    tree.select(id(1))
    await tree.addFromSelection()
    const created = (await gateway.listTasks()).tasks.find((task) => task.title === 'Nouvelle tâche')
    expect(created).toMatchObject({ parentId: null, archivedAt: null })
  })

  it('mettre une archive à la corbeille la retire des archives (R17)', async () => {
    const { tree, gateway } = await withArchive()
    await tree.toggleArchives()
    await tree.remove(id(1))
    expect(tree.rows.value.map((row) => row.task.title)).toEqual(['Courses'])
    expect((await gateway.listTrash()).map((task) => task.title)).toEqual(['Release'])
  })

  it('« Restaurer » la remet dans les listes', async () => {
    const { tree, toasts } = await withArchive()
    await tree.toggleArchives()
    await tree.unarchive(id(1))
    expect(tree.rows.value.map((row) => row.task.archivedAt)).toEqual([null, null])
    expect(toasts.current.value?.message).toBe('Archive restaurée')
  })
})

describe('sélection multiple (spec §5.5)', () => {
  /** Quatre tâches au premier niveau ; « Courses » est la seule ouverte. */
  async function withFour() {
    const gateway = new InMemoryTaskGateway([
      makeTask(1, 'Release', null, { done: true }),
      makeTask(2, 'Recette', 1, { done: true }),
      makeTask(3, 'Courses'),
      makeTask(4, 'Ménage', null, { done: true }),
    ])
    const { tree, confirm, toasts } = setup(gateway)
    await tree.start()
    return { gateway, tree, confirm, toasts }
  }
  const titles = (tree: ReturnType<typeof useTaskTree>) => tree.rows.value.map((row) => row.task.title)

  it('Maj+↓ puis Suppr met la sélection à la corbeille, et un seul « Annuler » la remet', async () => {
    const { tree, toasts } = await withFour()
    await tree.handle('extendNext')
    expect(tree.selectedIds.value).toEqual([id(1), id(3)])
    await tree.handle('delete')
    expect(titles(tree)).toEqual(['Ménage'])
    expect(toasts.current.value?.message).toBe('2 tâches mises à la corbeille')
    await tree.handle('undo')
    expect(titles(tree)).toEqual(['Release', 'Courses', 'Ménage'])
  })

  it('Mod+Maj+A archive la sélection sans question si tout est terminé, puis restaure s’il n’y a que des archives', async () => {
    const { tree, toasts } = await withFour()
    tree.selectWith(id(4), 'toggle')
    expect(tree.selectedIds.value).toEqual([id(1), id(4)])
    await tree.handle('archive')
    expect(titles(tree)).toEqual(['Courses'])
    expect(toasts.current.value?.message).toBe('2 tâches archivées')

    await tree.toggleArchives()
    tree.selectWith(id(1), 'single')
    tree.selectWith(id(4), 'toggle')
    await tree.handle('archive')
    expect(tree.rows.value.map((row) => row.task.archivedAt)).toEqual([null, null, null])
    expect(toasts.current.value?.message).toBe('2 archives restaurées')
  })

  it('une sélection avec des tâches ouvertes demande une seule confirmation', async () => {
    const { tree, confirm } = await withFour()
    tree.selectWith(id(4), 'range')
    const done = tree.archiveSelection()
    await settle()
    expect(confirm.current.value?.question.message).toBe(
      '1 tâche(s) encore ouverte(s) parmi les 3 sélectionnées et leurs sous-tâches. Les archiver quand même ?',
    )
    confirm.answer('archive')
    await done
    expect(titles(tree)).toEqual([])
  })

  it('les autres actions ne portent que sur la tâche courante ; Échap vide la sélection avant de fermer le panneau', async () => {
    const { tree } = await withFour()
    tree.openPanel(id(1))
    await tree.handle('extendNext')
    await tree.handle('closePanel')
    expect(tree.multipleSelected.value).toBe(false)
    expect(tree.openedId.value).not.toBeNull()
    await tree.handle('closePanel')
    expect(tree.openedId.value).toBeNull()

    await tree.handle('extendNext') // Courses + Ménage, Ménage courante
    await tree.handle('toggleDone')
    expect(tree.multipleSelected.value).toBe(false)
    // Seule « Ménage » est rouverte ; « Courses » reste ouverte, pas cochée.
    expect(tree.rows.value.map((row) => row.task.done)).toEqual([true, false, false])
  })
})
