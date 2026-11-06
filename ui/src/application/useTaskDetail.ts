/**
 * Panneau de détail d'une tâche (spécification §5.4) : titre, statut, étiquettes, note,
 * sous-tâches. Utilisé dans le panneau latéral et dans les fenêtres de tâche.
 *
 * Sauvegarde automatique de la note (spec : « débounce 1 s, flush à la perte de focus /
 * fermeture ») : chaque frappe repousse l'enregistrement d'une seconde ; changer de tâche ou
 * fermer enregistre immédiatement ce qui est en attente.
 */
import { computed, ref, shallowRef } from 'vue'
import { t } from '../i18n'
import { compareSiblings, isUiError, type NoteDocument, type Task, type TaskDetail, type TaskId } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'
import type { Confirm } from './useConfirm'
import { useStatusFlow } from './useStatusFlow'
import type { Toasts } from './useToast'

export type SaveState = 'saved' | 'pending' | 'saving' | 'error'

export interface TaskDetailOptions {
  /** Délai d'inactivité avant d'enregistrer la note (ms). */
  saveDelayMs?: number
}

export function useTaskDetail(gateway: TaskGateway, confirm: Confirm, toasts: Toasts, options: TaskDetailOptions = {}) {
  const saveDelayMs = options.saveDelayMs ?? 1000
  const detail = shallowRef<TaskDetail | null>(null)
  const children = shallowRef<Task[]>([])
  const missing = ref(false)
  const saveState = ref<SaveState>('saved')
  /** Incrémenté quand la note est rechargée depuis le cœur : l'éditeur remplace alors son contenu. */
  const noteVersion = ref(0)

  let pendingNote: { taskId: TaskId; note: NoteDocument } | null = null
  let saveTimer: ReturnType<typeof setTimeout> | undefined

  const task = computed(() => detail.value?.task ?? null)
  const progress = computed(() => ({
    done: children.value.filter((child) => child.done).length,
    total: children.value.length,
  }))

  const status = useStatusFlow(gateway, confirm, {
    titleOf: (id) =>
      (id === task.value?.id ? task.value.title : children.value.find((c) => c.id === id)?.title) ?? '',
    afterChange: reloadTask,
  })

  // ----- Chargement ------------------------------------------------------------------------

  /** Affiche la tâche `id` (après avoir enregistré la note en attente de la précédente). */
  async function load(id: TaskId): Promise<void> {
    await flush()
    await guarded(async () => {
      try {
        detail.value = await gateway.getTaskDetail(id)
        missing.value = false
      } catch (error) {
        detail.value = null
        missing.value = true
        if (!isUiError(error) || error.code !== 'rule') throw error
        return
      }
      noteVersion.value++
      await reloadChildren()
    })
  }

  /** Recharge titre, statut, étiquettes et sous-tâches, **sans** toucher à la note en cours d'édition. */
  async function reloadTask(): Promise<void> {
    const current = detail.value
    if (!current) return
    const fresh = await gateway.getTaskDetail(current.task.id)
    detail.value = { task: fresh.task, note: current.note }
    await reloadChildren()
  }

  async function reloadChildren(): Promise<void> {
    const id = detail.value?.task.id
    const all = id ? await gateway.listTasks() : []
    children.value = all.filter((candidate) => candidate.parentId === id).sort(compareSiblings)
  }

  /**
   * Des tâches ont changé ailleurs (autre fenêtre, synchronisation). Si c'est celle affichée et
   * qu'aucune modification locale n'attend, on recharge aussi la note ; sinon la saisie locale
   * est conservée (la gestion des conflits arrive en I5).
   */
  async function onExternalChange(ids: TaskId[]): Promise<void> {
    const id = detail.value?.task.id
    if (!id) return
    const touchesThisTask = ids.includes(id)
    if (touchesThisTask && pendingNote === null && saveState.value !== 'saving') {
      await guarded(async () => {
        detail.value = await gateway.getTaskDetail(id)
        noteVersion.value++
        await reloadChildren()
      })
    } else {
      await guarded(reloadTask)
    }
  }

  // ----- Note : sauvegarde automatique ----------------------------------------------------------

  /** L'éditeur signale une modification de la note. */
  function noteChanged(note: NoteDocument): void {
    const id = detail.value?.task.id
    if (!id) return
    pendingNote = { taskId: id, note }
    saveState.value = 'pending'
    clearTimeout(saveTimer)
    saveTimer = setTimeout(() => void flush(), saveDelayMs)
  }

  /** Enregistre immédiatement la note en attente, s'il y en a une. */
  async function flush(): Promise<void> {
    clearTimeout(saveTimer)
    const pending = pendingNote
    if (!pending) return
    pendingNote = null
    saveState.value = 'saving'
    try {
      await gateway.updateNote(pending.taskId, pending.note)
      if (detail.value?.task.id === pending.taskId) detail.value = { ...detail.value, note: pending.note }
      saveState.value = pendingNote ? 'pending' : 'saved'
    } catch (error) {
      pendingNote ??= pending // on garde la saisie pour réessayer au prochain enregistrement
      saveState.value = 'error'
      toasts.show({ kind: 'error', message: t('error.prefix', { message: isUiError(error) ? error.message : String(error) }) })
    }
  }

  // ----- Titre, statut, étiquettes, sous-tâches ------------------------------------------------

  async function rename(title: string): Promise<void> {
    const current = task.value
    if (!current || title.trim() === '' || title.trim() === current.title) return
    await guarded(async () => {
      await gateway.renameTask(current.id, title)
      await reloadTask()
    })
  }

  async function setTags(tags: string[]): Promise<void> {
    const current = task.value
    if (!current) return
    await guarded(async () => {
      await gateway.setTags(current.id, tags)
      await reloadTask()
    })
  }

  function addTag(tag: string): Promise<void> {
    const value = tag.trim()
    if (!value || !task.value) return Promise.resolve()
    return setTags([...task.value.tags, value])
  }

  function removeTag(tag: string): Promise<void> {
    return setTags((task.value?.tags ?? []).filter((existing) => existing !== tag))
  }

  function toggleDone(id: TaskId = task.value?.id ?? ''): Promise<void> {
    const target = id === task.value?.id ? task.value : children.value.find((child) => child.id === id)
    if (!target) return Promise.resolve()
    return guarded(() => status.toggle(target.id, target.done))
  }

  async function addSubtask(title: string): Promise<void> {
    const current = task.value
    if (!current || !title.trim()) return
    await guarded(async () => {
      await gateway.createTask(title, { kind: 'atEnd', parentId: current.id })
      await reloadTask()
    })
  }

  async function guarded(action: () => Promise<void>): Promise<void> {
    try {
      await action()
    } catch (error) {
      const message = isUiError(error) ? error.message : String(error)
      toasts.show({ kind: 'error', message: t('error.prefix', { message }) })
    }
  }

  return {
    // état
    detail, task, children, progress, missing, saveState, noteVersion,
    // actions
    load, flush, noteChanged, onExternalChange, rename, addTag, removeTag, toggleDone, addSubtask,
  }
}

export type TaskDetailState = ReturnType<typeof useTaskDetail>
