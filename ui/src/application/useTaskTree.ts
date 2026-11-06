/**
 * État et actions de la liste de tâches : sélection, dépliage, zoom, renommage, et dialogue
 * avec le cœur (via le port `TaskGateway`).
 *
 * Les **décisions** (faut-il proposer de clôturer le parent ?) viennent du Rust dans les
 * réponses des commandes (`proposeClosingParent`, `proposeReopening`…). Ce composable ne fait
 * que poser la question à l'utilisateur (`confirm`) et transmettre sa réponse.
 */
import { computed, ref, shallowRef } from 'vue'
import { t } from '../i18n'
import { isUiError, type Movement, type Position, type Task, type TaskId } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'
import type { TreeAction } from './keyboard'
import type { Confirm } from './useConfirm'
import type { Toasts } from './useToast'
import { TaskIndex, visibleRows } from './treeView'

const EXPANDED_STORAGE_KEY = 'todoquoi.expanded'
export const PAGE_SIZE = 10

export interface TaskTreeOptions {
  /** Stockage local à l'appareil pour l'état déplié (absent en test). */
  storage?: Pick<Storage, 'getItem' | 'setItem'>
}

export function useTaskTree(gateway: TaskGateway, confirm: Confirm, toasts: Toasts, options: TaskTreeOptions = {}) {
  const tasks = shallowRef<Task[]>([])
  const loaded = ref(false)
  const expanded = ref(new Set<TaskId>(readExpanded(options.storage)))
  const selectedId = ref<TaskId | null>(null)
  const zoomId = ref<TaskId | null>(null)
  const renamingId = ref<TaskId | null>(null)

  const index = computed(() => new TaskIndex(tasks.value))
  const rows = computed(() => visibleRows(index.value, zoomId.value, expanded.value))
  const breadcrumb = computed(() => {
    const zoom = zoomId.value === null ? undefined : index.value.get(zoomId.value)
    return zoom ? [...index.value.ancestors(zoom.id).reverse(), zoom] : []
  })
  const counts = computed(() => ({
    total: tasks.value.length,
    open: tasks.value.filter((task) => !task.done).length,
  }))
  const selectedRowIndex = computed(() => rows.value.findIndex((row) => row.task.id === selectedId.value))

  // ----- Chargement ----------------------------------------------------------------------------

  async function refresh(): Promise<void> {
    tasks.value = await gateway.listTasks()
    loaded.value = true
    if (zoomId.value !== null && !index.value.get(zoomId.value)) zoomId.value = null
    if (selectedRowIndex.value === -1) selectedId.value = rows.value[0]?.task.id ?? null
  }

  let refreshTimer: ReturnType<typeof setTimeout> | undefined
  /** Démarre : premier chargement + abonnement aux changements faits ailleurs. */
  async function start(): Promise<() => void> {
    await guarded(refresh)
    return gateway.onTasksChanged(() => {
      clearTimeout(refreshTimer)
      refreshTimer = setTimeout(() => void guarded(refresh), 100)
    })
  }

  // ----- Sélection, dépliage, zoom ----------------------------------------------------------------

  function select(id: TaskId | null): void {
    selectedId.value = id
  }

  function selectAt(rowIndex: number): void {
    const row = rows.value[Math.max(0, Math.min(rows.value.length - 1, rowIndex))]
    if (row) selectedId.value = row.task.id
  }

  function setExpanded(id: TaskId, value: boolean): void {
    const next = new Set(expanded.value)
    if (value) next.add(id)
    else next.delete(id)
    expanded.value = next
    writeExpanded(options.storage, next)
  }

  function toggleExpanded(id: TaskId): void {
    setExpanded(id, !expanded.value.has(id))
  }

  function expandOrFirstChild(): void {
    const row = rows.value[selectedRowIndex.value]
    if (!row || row.childCount === 0) return
    if (!row.expanded) setExpanded(row.task.id, true)
    else selectAt(selectedRowIndex.value + 1)
  }

  function collapseOrParent(): void {
    const row = rows.value[selectedRowIndex.value]
    if (!row) return
    if (row.expanded) return setExpanded(row.task.id, false)
    const parent = index.value.parentOf(row.task)
    if (parent !== null && parent !== zoomId.value) selectedId.value = parent
  }

  function zoomTo(id: TaskId | null): void {
    const previousZoom = zoomId.value
    zoomId.value = id
    selectedId.value = rows.value[0]?.task.id ?? (id === null ? previousZoom : null)
  }

  function zoomIn(): void {
    if (selectedId.value !== null) zoomTo(selectedId.value)
  }

  function zoomOut(): void {
    const zoom = zoomId.value === null ? undefined : index.value.get(zoomId.value)
    if (!zoom) return
    zoomId.value = index.value.parentOf(zoom)
    selectedId.value = zoom.id
  }

  // ----- Création et renommage -------------------------------------------------------------------

  async function create(position: Position): Promise<void> {
    const id = await gateway.createTask(t('task.newTitle'), position)
    if (position.kind === 'atEnd' && position.parentId !== null) setExpanded(position.parentId, true)
    await refresh()
    selectedId.value = id
    renamingId.value = id
  }

  function newSibling(): Promise<void> {
    const selected = selectedId.value
    return create(selected === null ? { kind: 'atEnd', parentId: zoomId.value } : { kind: 'after', siblingId: selected })
  }

  function newChild(): Promise<void> {
    const selected = selectedId.value
    return create({ kind: 'atEnd', parentId: selected ?? zoomId.value })
  }

  function addAtEnd(): Promise<void> {
    return create({ kind: 'atEnd', parentId: zoomId.value })
  }

  function startRenaming(): void {
    if (selectedId.value !== null) renamingId.value = selectedId.value
  }

  async function commitRename(id: TaskId, title: string): Promise<void> {
    renamingId.value = null
    const task = index.value.get(id)
    if (!task || title.trim() === '' || title.trim() === task.title) return
    await gateway.renameTask(id, title)
    await refresh()
  }

  function cancelRename(): void {
    renamingId.value = null
  }

  // ----- Clôture / réouverture (R1–R3) -----------------------------------------------------------

  async function toggleDone(id: TaskId): Promise<void> {
    const task = index.value.get(id)
    if (!task) return
    if (task.done) return reopen(id)
    const impact = await gateway.getCloseImpact(id)
    let scope: 'taskOnly' | 'withOpenDescendants' = 'taskOnly'
    if (impact.openDescendants > 0) {
      const choice = await confirm.ask({
        title: t('close.withChildren.title'),
        message: t('close.withChildren.message', { title: task.title, count: impact.openDescendants }),
        choices: [
          { id: 'cancel', label: t('common.cancel') },
          { id: 'taskOnly', label: t('close.withChildren.taskOnly') },
          { id: 'withOpenDescendants', label: t('close.withChildren.all'), primary: true },
        ],
      })
      if (choice === 'cancel') return
      scope = choice
    }
    const outcome = await gateway.closeTask(id, scope)
    await refresh()
    await offerClosingParent(outcome.proposeClosingParent)
  }

  /** R1 — propose de clôturer le parent, puis le grand-parent… un niveau à la fois. */
  async function offerClosingParent(parentId: TaskId | null): Promise<void> {
    let proposal = parentId
    while (proposal !== null) {
      const title = index.value.get(proposal)?.title ?? ''
      const choice = await confirm.ask({
        title: t('close.parent.title'),
        message: t('close.parent.message', { title }),
        choices: [
          { id: 'later', label: t('common.later') },
          { id: 'close', label: t('close.parent.confirm'), primary: true },
        ],
      })
      if (choice !== 'close') return
      proposal = (await gateway.closeTask(proposal, 'taskOnly')).proposeClosingParent
      await refresh()
    }
  }

  async function reopen(id: TaskId): Promise<void> {
    const outcome = await gateway.reopenTask(id)
    await refresh()
    await offerReopening(outcome.proposeReopening)
  }

  /** R3 — propose de rouvrir les ancêtres terminés. */
  async function offerReopening(ids: TaskId[]): Promise<void> {
    if (ids.length === 0) return
    const titles = ids.map((id) => `« ${index.value.get(id)?.title ?? id} »`).join(', ')
    const choice = await confirm.ask({
      title: t('reopen.ancestors.title'),
      message: t('reopen.ancestors.message', { titles }),
      choices: [
        { id: 'no', label: t('common.no') },
        { id: 'reopen', label: t('reopen.ancestors.confirm'), primary: true },
      ],
    })
    if (choice !== 'reopen') return
    for (const id of ids) await gateway.reopenTask(id)
    await refresh()
  }

  // ----- Déplacement (R9–R12) et corbeille (R5, R7) ---------------------------------------------

  async function move(movement: Movement): Promise<void> {
    const id = selectedId.value
    const task = id === null ? undefined : index.value.get(id)
    if (id === null || !task) return
    // Indenter range la tâche sous sa sœur précédente : on déplie celle-ci pour la garder visible.
    const siblings = index.value.children(index.value.parentOf(task))
    const previousSibling = siblings[siblings.findIndex((sibling) => sibling.id === id) - 1]
    const outcome = await gateway.moveTask(id, movement)
    if (movement.kind === 'indent' && previousSibling) setExpanded(previousSibling.id, true)
    await refresh()
    selectedId.value = id
    await offerClosingParent(outcome.proposeClosing)
    await offerReopening(outcome.proposeReopening)
  }

  async function remove(id: TaskId): Promise<void> {
    const task = index.value.get(id)
    if (!task) return
    const position = selectedRowIndex.value
    await gateway.deleteTask(id)
    await refresh()
    selectAt(position)
    toasts.show({
      kind: 'info',
      message: t('delete.done', { title: task.title }),
      undo: () => guarded(() => restore(id)),
    })
  }

  async function restore(id: TaskId): Promise<void> {
    const outcome = await gateway.restoreTask(id)
    await refresh()
    selectedId.value = id
    if (outcome.movedToRoot) toasts.show({ kind: 'info', message: t('restore.toRoot') })
  }

  // ----- Clavier --------------------------------------------------------------------------------

  async function handle(action: TreeAction): Promise<void> {
    const id = selectedId.value
    const actions: Record<TreeAction, () => void | Promise<void>> = {
      selectPrevious: () => selectAt(selectedRowIndex.value - 1),
      selectNext: () => selectAt(selectedRowIndex.value + 1),
      selectFirst: () => selectAt(0),
      selectLast: () => selectAt(rows.value.length - 1),
      pageUp: () => selectAt(selectedRowIndex.value - PAGE_SIZE),
      pageDown: () => selectAt(selectedRowIndex.value + PAGE_SIZE),
      expandOrFirstChild,
      collapseOrParent,
      open: () => toasts.show({ kind: 'info', message: t('status.detailLater') }),
      zoomIn,
      zoomOut,
      toggleDone: () => (id === null ? undefined : toggleDone(id)),
      newSibling,
      newChild,
      rename: startRenaming,
      indent: () => move({ kind: 'indent' }),
      outdent: () => move({ kind: 'outdent' }),
      moveUp: () => move({ kind: 'up' }),
      moveDown: () => move({ kind: 'down' }),
      delete: () => (id === null ? undefined : remove(id)),
      undo: () => toasts.undo(),
    }
    await guarded(async () => actions[action]())
  }

  /** Exécute une action et affiche l'erreur éventuelle au lieu de la laisser se perdre. */
  async function guarded(action: () => void | Promise<void>): Promise<void> {
    try {
      await action()
    } catch (error) {
      const message = isUiError(error) ? error.message : String(error)
      toasts.show({ kind: 'error', message: t('error.prefix', { message }) })
    }
  }

  return {
    // état
    tasks, loaded, rows, breadcrumb, counts, selectedId, zoomId, renamingId,
    // actions
    start, refresh, select, toggleExpanded, zoomTo, handle, guarded,
    toggleDone: (id: TaskId) => guarded(() => toggleDone(id)),
    addAtEnd: () => guarded(addAtEnd),
    commitRename: (id: TaskId, title: string) => guarded(() => commitRename(id, title)),
    cancelRename,
  }
}

export type TaskTree = ReturnType<typeof useTaskTree>

function readExpanded(storage: TaskTreeOptions['storage']): TaskId[] {
  try {
    const value = JSON.parse(storage?.getItem(EXPANDED_STORAGE_KEY) ?? '[]') as unknown
    return Array.isArray(value) ? value.filter((id): id is string => typeof id === 'string') : []
  } catch {
    return []
  }
}

function writeExpanded(storage: TaskTreeOptions['storage'], expanded: Set<TaskId>): void {
  try {
    storage?.setItem(EXPANDED_STORAGE_KEY, JSON.stringify([...expanded]))
  } catch {
    // Stockage indisponible (navigation privée…) : l'état déplié ne sera simplement pas retenu.
  }
}
