import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { InMemoryTaskGateway } from '../infrastructure/in-memory/InMemoryTaskGateway'
import type { NoteDocument } from '../domain/task'
import { useConfirm } from './useConfirm'
import { useTaskDetail } from './useTaskDetail'
import { useToast } from './useToast'
import { id, makeTask } from './testSupport'

const note = (text: string): NoteDocument => ({
  type: 'doc',
  content: [{ type: 'paragraph', content: [{ type: 'text', text }] }],
})

function setup() {
  const gateway = new InMemoryTaskGateway([
    makeTask(1, 'Release', null, { tags: ['release'] }),
    makeTask(2, 'Recette', 1),
    makeTask(3, 'Déploiement', 1, { done: true }),
    makeTask(4, 'Courses'),
  ])
  const toasts = useToast()
  const detail = useTaskDetail(gateway, useConfirm(), toasts, { saveDelayMs: 1000 })
  return { gateway, toasts, detail }
}

describe('panneau de détail', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('charge la tâche, ses sous-tâches et sa progression', async () => {
    const { detail } = setup()
    await detail.load(id(1))
    expect(detail.task.value?.title).toBe('Release')
    expect(detail.children.value.map((child) => child.title)).toEqual(['Recette', 'Déploiement'])
    expect(detail.progress.value).toEqual({ done: 1, total: 2 })
  })

  it('enregistre la note 1 s après la dernière frappe (débounce)', async () => {
    const { gateway, detail } = setup()
    const save = vi.spyOn(gateway, 'updateNote')
    await detail.load(id(1))
    detail.noteChanged(note('Bon'))
    await vi.advanceTimersByTimeAsync(600)
    detail.noteChanged(note('Bonjour'))
    expect(detail.saveState.value).toBe('pending')
    await vi.advanceTimersByTimeAsync(600)
    expect(save).not.toHaveBeenCalled() // la 2e frappe a repoussé l'enregistrement
    await vi.advanceTimersByTimeAsync(500)
    expect(save).toHaveBeenCalledTimes(1)
    expect(save).toHaveBeenCalledWith(id(1), note('Bonjour'))
    expect(detail.saveState.value).toBe('saved')
  })

  it('changer de tâche enregistre immédiatement la note en attente', async () => {
    const { gateway, detail } = setup()
    const save = vi.spyOn(gateway, 'updateNote')
    await detail.load(id(1))
    detail.noteChanged(note('Brouillon'))
    await detail.load(id(4))
    expect(save).toHaveBeenCalledWith(id(1), note('Brouillon'))
    expect((await gateway.getTaskDetail(id(1))).note).toEqual(note('Brouillon'))
  })

  it('un échec d’enregistrement est signalé et la saisie est conservée', async () => {
    const { gateway, detail, toasts } = setup()
    await detail.load(id(1))
    vi.spyOn(gateway, 'updateNote').mockRejectedValueOnce({ code: 'storage', message: 'drive inaccessible' })
    detail.noteChanged(note('Important'))
    await detail.flush()
    expect(detail.saveState.value).toBe('error')
    expect(toasts.current.value?.message).toContain('drive inaccessible')
    await detail.flush() // nouvel essai
    expect((await gateway.getTaskDetail(id(1))).note).toEqual(note('Important'))
  })

  it('un changement venu d’ailleurs recharge la note, sauf si une saisie locale attend', async () => {
    const { gateway, detail } = setup()
    await detail.load(id(1))
    const version = detail.noteVersion.value
    await gateway.updateNote(id(1), note('Modifiée sur un autre appareil'))
    await detail.onExternalChange([id(1)])
    expect(detail.noteVersion.value).toBe(version + 1)
    expect(detail.detail.value?.note).toEqual(note('Modifiée sur un autre appareil'))

    detail.noteChanged(note('Saisie locale'))
    await detail.onExternalChange([id(1)])
    expect(detail.noteVersion.value).toBe(version + 1) // la saisie locale n'est pas écrasée
  })

  it('titre, étiquettes et sous-tâches', async () => {
    const { detail } = setup()
    await detail.load(id(1))
    await detail.rename('Release 1.3')
    await detail.addTag('#Client-X')
    await detail.removeTag('release')
    await detail.addSubtask('Changelog')
    expect(detail.task.value).toMatchObject({ title: 'Release 1.3', tags: ['client-x'] })
    expect(detail.children.value.map((child) => child.title)).toContain('Changelog')
  })

  it('une tâche disparue affiche « n’existe plus »', async () => {
    const { detail } = setup()
    await detail.load('00000000-0000-7000-8000-0000000000ff')
    expect(detail.missing.value).toBe(true)
  })
})
