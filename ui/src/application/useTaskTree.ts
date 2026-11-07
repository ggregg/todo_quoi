/**
 * État et actions de la liste de tâches : sélection, dépliage, zoom, renommage, tâche ouverte
 * dans le panneau de détail, et dialogue avec le cœur (via le port `TaskGateway`).
 *
 * Les **décisions** (faut-il proposer de clôturer le parent ?) viennent du Rust dans les
 * réponses des commandes ; les dialogues correspondants sont dans `useStatusFlow`.
 */
import { computed, ref, shallowRef, watch } from 'vue'
import { t } from '../i18n'
import { isUiError, type Movement, type Position, type Task, type TaskId } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'
import type { TreeAction } from './keyboard'
import type { Confirm } from './useConfirm'
import { useStatusFlow } from './useStatusFlow'
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
  /** Tâche affichée dans le panneau de détail (`null` = panneau fermé). */
  const openedId = ref<TaskId | null>(null)

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

  const status = useStatusFlow(gateway, confirm, {
    titleOf: (id) => index.value.get(id)?.title ?? '',
    afterChange: refresh,
  })

  // Panneau ouvert : il suit la sélection (comme dans la maquette).
  watch(selectedId, (id) => {
    if (openedId.value !== null && id !== null) openedId.value = id
  })

  // ----- Chargement ----------------------------------------------------------------------------

  async function refresh(): Promise<void> {
    tasks.value = await gateway.listTasks()
    loaded.value = true
    if (zoomId.value !== null && !index.value.get(zoomId.value)) zoomId.value = null
    if (openedId.value !== null && !index.value.get(openedId.value)) openedId.value = null
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

  /** Montre la tâche dans l'arbre (résultat de recherche) : sort du zoom s'il la cache, déplie ses ancêtres, la sélectionne. */
  function reveal(id: TaskId): void {
    const ancestors = index.value.ancestors(id)
    if (zoomId.value !== null && !ancestors.some((ancestor) => ancestor.id === zoomId.value)) zoomId.value = null
    const next = new Set(expanded.value)
    for (const ancestor of ancestors) next.add(ancestor.id)
    expanded.value = next
    writeExpanded(options.storage, next)
    selectedId.value = id
  }

  // ----- Panneau de détail et fenêtres de tâche --------------------------------------------------

  function openPanel(id: TaskId | null = selectedId.value): void {
    if (id === null) return
    selectedId.value = id
    openedId.value = id
  }

  function closePanel(): void {
    openedId.value = null
  }

  /** Ouvre la tâche dans sa propre fenêtre ; si elle était dans le panneau, elle le quitte. */
  async function openWindow(id: TaskId | null = selectedId.value): Promise<void> {
    if (id === null) return
    await gateway.openTaskWindow(id)
    if (openedId.value === id) openedId.value = null
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
    return create(
      selected === null ? { kind: 'atEnd', parentId: zoomId.value } : { kind: 'after', siblingId: selected },
    )
  }

  function newChild(): Promise<void> {
    return create({ kind: 'atEnd', parentId: selectedId.value ?? zoomId.value })
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
    await status.offerClosingParent(outcome.proposeClosing)
    await status.offerReopening(outcome.proposeReopening)
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

  async function toggleDone(id: TaskId): Promise<void> {
    const task = index.value.get(id)
    if (task) await status.toggle(id, task.done)
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
      open: () => openPanel(),
      openWindow: () => openWindow(),
      closePanel,
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
    tasks,
    loaded,
    rows,
    breadcrumb,
    counts,
    selectedId,
    zoomId,
    renamingId,
    openedId,
    // actions
    start,
    refresh,
    select,
    toggleExpanded,
    zoomTo,
    reveal,
    handle,
    guarded,
    openPanel,
    closePanel,
    openWindow: (id?: TaskId) => guarded(() => openWindow(id)),
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
