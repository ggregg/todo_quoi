/**
 * État et actions de la liste de tâches : sélection, dépliage, zoom, renommage, tâche ouverte
 * dans le panneau de détail, et dialogue avec le cœur (via le port `TaskGateway`).
 *
 * Les **décisions** (faut-il proposer de clôturer le parent ?) viennent du Rust dans les
 * réponses des commandes ; les dialogues correspondants sont dans `useStatusFlow`.
 */
import { computed, ref, shallowRef, watch } from 'vue'
import { t } from '../i18n'
import { isUiError, type Movement, type Position, type TaskId } from '../domain/task'
import type { ListView, Listing } from '../domain/view'
import type { TaskGateway } from '../ports/TaskGateway'
import type { TreeAction } from './keyboard'
import type { Confirm } from './useConfirm'
import { useStatusFlow } from './useStatusFlow'
import type { Toasts } from './useToast'
import { TaskIndex, displayRows, type Row } from './treeView'
import { dropAtEnd, dropOnTask, type DropZone } from './treeDrop'

const EXPANDED_STORAGE_KEY = 'todoquoi.expanded'
const DISPLAYED_BASE_KEY = 'todoquoi.displayedBase'
const COLLAPSED_BASES_KEY = 'todoquoi.collapsedBases'

/** Bases affichées : une seule (son identifiant), ou toutes en sections. */
export type BaseDisplay = string | 'all'
export const PAGE_SIZE = 10

export interface TaskTreeOptions {
  /** Stockage local à l'appareil pour l'état déplié (absent en test). */
  storage?: Pick<Storage, 'getItem' | 'setItem'>
}

export function useTaskTree(gateway: TaskGateway, confirm: Confirm, toasts: Toasts, options: TaskTreeOptions = {}) {
  const listing = shallowRef<Listing>({ tasks: [], bases: [], lists: [], media: {} })
  const tasks = computed(() => listing.value.tasks)
  const loaded = ref(false)
  const expanded = ref(new Set<TaskId>(readExpanded(options.storage)))
  const selectedId = ref<TaskId | null>(null)
  const zoomId = ref<TaskId | null>(null)
  /** En-tête de section sélectionné (mode « Tout ») ; `null` si c'est une tâche qui l'est. */
  const selectedBase = ref<string | null>(null)
  const renamingId = ref<TaskId | null>(null)
  /** Tâche affichée dans le panneau de détail (`null` = panneau fermé). */
  const openedId = ref<TaskId | null>(null)

  const index = computed(() => new TaskIndex(listing.value.tasks, listing.value.lists, listing.value.media))

  // ----- Bases affichées (bandeau, ADR-0009) — mémorisé sur l'appareil -------------------------
  const display = ref<BaseDisplay>(readText(options.storage, DISPLAYED_BASE_KEY) ?? 'all')
  const collapsedBases = ref(new Set<string>(readList(options.storage, COLLAPSED_BASES_KEY)))
  /** Bases réellement affichées (une base inconnue, fermée entre-temps, revient à « Tout »). */
  const visibleBases = computed(() => {
    const bases = listing.value.bases
    return display.value !== 'all' && bases.includes(display.value) ? [display.value] : bases
  })
  /** Compteur du bas : tâches des bases affichées. */
  const counts = computed(() => {
    const shown = tasks.value.filter((task) => visibleBases.value.includes(task.base))
    return { total: shown.length, open: shown.filter((task) => !task.done).length }
  })
  /** Base de la liste affichée quand il n'y en a qu'une (sinon `null` : sections). */
  const singleBase = computed(() => (visibleBases.value.length === 1 ? (visibleBases.value[0] ?? null) : null))

  /** Mise en forme de la liste affichée (tâche zoomée, ou premier niveau de la base affichée). */
  const currentLayout = computed(() =>
    zoomId.value !== null
      ? index.value.layout(zoomId.value)
      : index.value.layout(null, singleBase.value ?? visibleBases.value[0]),
  )
  /** Lignes affichées : tâches et, en mode « Tout », en-têtes de section. */
  const displayed = computed(() =>
    displayRows(index.value, zoomId.value, expanded.value, visibleBases.value, collapsedBases.value),
  )
  /** Tâches affichées, dans l'ordre : c'est ce que parcourt le clavier. */
  const rows = computed(() => displayed.value.filter((row): row is Row & { kind: 'task' } => row.kind === 'task'))
  const breadcrumb = computed(() => {
    const zoom = zoomId.value === null ? undefined : index.value.get(zoomId.value)
    return zoom ? [...index.value.ancestors(zoom.id).reverse(), zoom] : []
  })

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
    listing.value = await gateway.listTasks()
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
    selectedBase.value = null
  }

  /** Sélectionne l'en-tête d'une section (mode « Tout ») : « Ajouter » créera dans cette base. */
  function selectBase(base: string): void {
    selectedBase.value = base
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

  // ----- Glisser-déposer (voir treeDrop.ts) ------------------------------------------------------

  /** Tâche en cours de déplacement à la souris. */
  const draggingId = ref<TaskId | null>(null)

  /** Peut-on lâcher la tâche en cours de déplacement sur `target` ? (affichage du repère) */
  function canDropOn(target: TaskId, zone: DropZone): boolean {
    return draggingId.value !== null && dropOnTask(index.value, draggingId.value, target, zone) !== null
  }

  async function dropOn(target: TaskId, zone: DropZone): Promise<void> {
    const dragged = draggingId.value
    draggingId.value = null
    const movement = dragged === null ? null : dropOnTask(index.value, dragged, target, zone)
    if (dragged !== null && movement) await moveTask(dragged, movement)
  }

  /** Lâcher à la fin d'une liste (fond de la liste, fil d'Ariane) : `null` = premier niveau. */
  async function dropAtListEnd(parentId: TaskId | null): Promise<void> {
    if (parentId === null && singleBase.value !== null) return dropOnBase(singleBase.value)
    const dragged = draggingId.value
    draggingId.value = null
    const movement = dragged === null ? null : dropAtEnd(index.value, dragged, parentId)
    if (dragged !== null && movement) await moveTask(dragged, movement)
  }

  /** Lâcher sur une base (en-tête de section, pastille du bandeau) : à la fin de son premier niveau. */
  async function dropOnBase(base: string): Promise<void> {
    const dragged = draggingId.value
    draggingId.value = null
    if (dragged !== null) await moveTask(dragged, { kind: 'toBase', base })
  }

  // ----- Vues par liste (spec §5.3) ------------------------------------------------------------

  /** Enregistre la vue de la liste affichée (la personnalise si elle était héritée). */
  async function customizeView(view: ListView): Promise<void> {
    await gateway.customizeListView(zoomId.value, zoomId.value === null ? singleBase.value : null, view)
    await refresh()
  }

  /** La liste affichée reprend la vue dont elle hérite (sans effet sur la racine). */
  async function inheritView(): Promise<void> {
    if (zoomId.value === null) return
    await gateway.inheritListView(zoomId.value)
    await refresh()
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

  /** « Ajouter » : dans la tâche zoomée, sinon au premier niveau de `base` (ou de la base affichée). */
  function addAtEnd(base?: string): Promise<void> {
    if (zoomId.value !== null) return create({ kind: 'atEnd', parentId: zoomId.value })
    const target = base ?? singleBase.value ?? (selectedTaskBase() || visibleBases.value[0])
    return create(target ? { kind: 'atTopOf', base: target } : { kind: 'atEnd', parentId: null })
  }

  /**
   * « Ajouter une tâche » selon la sélection : une section de base sélectionnée → au premier
   * niveau de cette base ; une tâche sélectionnée (et affichée) → juste après elle, au même
   * niveau ; sinon → à la fin de la liste affichée.
   */
  function addFromSelection(): Promise<void> {
    if (selectedBase.value !== null) return addAtEnd(selectedBase.value)
    const selected = selectedId.value
    if (selected !== null && rows.value.some((row) => row.task.id === selected)) {
      return create({ kind: 'after', siblingId: selected })
    }
    return addAtEnd()
  }

  function selectedTaskBase(): string | undefined {
    return selectedId.value === null ? undefined : index.value.get(selectedId.value)?.base
  }

  // ----- Bandeau des bases ------------------------------------------------------------------------

  /** Affiche une seule base, ou toutes (`'all'`, même s'il n'y en a qu'une). Quitte le zoom s'il sort des bases affichées. */
  function showBases(choice: BaseDisplay): void {
    display.value = choice
    writeText(options.storage, DISPLAYED_BASE_KEY, choice)
    const zoom = zoomId.value === null ? undefined : index.value.get(zoomId.value)
    if (zoom && !visibleBases.value.includes(zoom.base)) zoomId.value = null
    if (selectedRowIndex.value === -1) selectedId.value = rows.value[0]?.task.id ?? null
  }

  function toggleBaseSection(base: string): void {
    const next = new Set(collapsedBases.value)
    if (next.has(base)) next.delete(base)
    else next.add(base)
    collapsedBases.value = next
    writeText(options.storage, COLLAPSED_BASES_KEY, JSON.stringify([...next]))
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
    if (selectedId.value !== null) await moveTask(selectedId.value, movement)
  }

  async function moveTask(id: TaskId, movement: Movement): Promise<void> {
    const task = index.value.get(id)
    if (!task) return
    // Indenter range la tâche sous sa sœur précédente : on déplie celle-ci pour la garder visible.
    const siblings = index.value.children(index.value.parentOf(task))
    const previousSibling = siblings[siblings.findIndex((sibling) => sibling.id === id) - 1]
    const outcome = await gateway.moveTask(id, movement)
    if (movement.kind === 'indent' && previousSibling) setExpanded(previousSibling.id, true)
    if (movement.kind === 'to' && movement.parentId !== null) setExpanded(movement.parentId, true)
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
    // bases (ADR-0009)
    display,
    /** Toutes les bases ouvertes, dans l'ordre du bandeau. */
    allBases: computed(() => listing.value.bases),
    visibleBases,
    singleBase,
    displayed,
    showBases,
    toggleBaseSection,
    dropOnBase: (base: string) => guarded(() => dropOnBase(base)),
    // état
    tasks,
    loaded,
    rows,
    breadcrumb,
    counts,
    currentLayout,
    /** Titre d'une tâche (texte d'aide de l'éditeur de vue). */
    titleOf: (id: TaskId) => index.value.get(id)?.title ?? '',
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
    restore: (id: TaskId) => guarded(() => restore(id)),
    /** Titres du chemin jusqu'à `parentId` inclus (vide si premier niveau ou parent disparu). */
    pathTo: (parentId: TaskId | null): string[] => {
      const parent = parentId === null ? undefined : index.value.get(parentId)
      return parent ? [...index.value.ancestors(parent.id).reverse(), parent].map((task) => task.title) : []
    },
    draggingId,
    startDrag: (id: TaskId) => (draggingId.value = id),
    endDrag: () => (draggingId.value = null),
    canDropOn,
    dropOn: (target: TaskId, zone: DropZone) => guarded(() => dropOn(target, zone)),
    dropAtListEnd: (parentId: TaskId | null) => guarded(() => dropAtListEnd(parentId)),
    customizeView: (view: ListView) => guarded(() => customizeView(view)),
    inheritView: () => guarded(inheritView),
    handle,
    guarded,
    openPanel,
    closePanel,
    openWindow: (id?: TaskId) => guarded(() => openWindow(id)),
    toggleDone: (id: TaskId) => guarded(() => toggleDone(id)),
    addAtEnd: (base?: string) => guarded(() => addAtEnd(base)),
    addFromSelection: () => guarded(addFromSelection),
    selectedBase,
    selectBase,
    commitRename: (id: TaskId, title: string) => guarded(() => commitRename(id, title)),
    cancelRename,
  }
}

export type TaskTree = ReturnType<typeof useTaskTree>

function readText(storage: TaskTreeOptions['storage'], key: string): string | null {
  try {
    return storage?.getItem(key) ?? null
  } catch {
    return null
  }
}

function writeText(storage: TaskTreeOptions['storage'], key: string, value: string): void {
  try {
    storage?.setItem(key, value)
  } catch {
    // Stockage indisponible : le choix ne sera simplement pas retenu.
  }
}

function readList(storage: TaskTreeOptions['storage'], key: string): string[] {
  try {
    const value = JSON.parse(readText(storage, key) ?? '[]') as unknown
    return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : []
  } catch {
    return []
  }
}

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
