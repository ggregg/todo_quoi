/**
 * Calculs d'**affichage** de l'arbre (fonctions pures, sans Vue) : enfants, ancêtres, lignes
 * visibles selon les nœuds dépliés et le zoom.
 *
 * Ce ne sont pas des règles métier : on présente la liste « à plat » renvoyée par le cœur Rust.
 * L'ordre et le filtrage de chaque liste viennent de sa mise en forme (`ListLayout`, calculée
 * en Rust selon sa vue) ; sans mise en forme, ordre manuel (`order`). Comme en Rust, une tâche
 * dont le parent est introuvable est affichée au premier niveau (`TaskTree::effective_parent`).
 */
import { compareSiblings, type Task, type TaskId } from '../domain/task'
import type { ListLayout, MediaKind } from '../domain/view'

/** Une ligne de la liste affichée. */
export interface Row {
  task: Task
  /** Profondeur sous la racine affichée (0 = premier niveau, ou enfants directs du zoom). */
  depth: number
  /** Toutes les sous-tâches actives (la progression ignore les filtres). */
  childCount: number
  doneChildCount: number
  expanded: boolean
  /** Médias présents dans la note. */
  media: MediaKind[]
}

/** Index de lecture sur un instantané des tâches. */
export class TaskIndex {
  private readonly byIdMap: Map<TaskId, Task>
  private readonly childrenMap = new Map<TaskId | null, Task[]>()
  /** Mise en forme par liste : clé = id de la tâche, ou `base:<id>` pour un premier niveau. */
  private readonly layouts: Map<string, ListLayout>

  constructor(
    tasks: Task[],
    layouts: ListLayout[] = [],
    private readonly media: Record<TaskId, MediaKind[]> = {},
  ) {
    this.layouts = new Map(layouts.map((layout) => [listKey(layout.taskId, layout.base), layout]))
    this.byIdMap = new Map(tasks.map((task) => [task.id, task]))
    for (const task of tasks) {
      const parent = this.parentOf(task)
      const siblings = this.childrenMap.get(parent) ?? []
      siblings.push(task)
      this.childrenMap.set(parent, siblings)
    }
    for (const siblings of this.childrenMap.values()) siblings.sort(compareSiblings)
  }

  get(id: TaskId): Task | undefined {
    return this.byIdMap.get(id)
  }

  get size(): number {
    return this.byIdMap.size
  }

  all(): Task[] {
    return [...this.byIdMap.values()]
  }

  /** Parent effectif : le parent déclaré s'il est présent, sinon `null`. */
  parentOf(task: Task): TaskId | null {
    return task.parentId !== null && this.byIdMap.has(task.parentId) ? task.parentId : null
  }

  /** Toutes les sous-tâches actives, dans l'ordre manuel. */
  children(parentId: TaskId | null): Task[] {
    return this.childrenMap.get(parentId) ?? []
  }

  /**
   * Sous-tâches **affichées** de `parentId` (filtrées et triées selon la vue de la liste) ;
   * `parentId` `null` : premier niveau de `base` (sans base : toutes les bases, ordre manuel).
   */
  shownChildren(parentId: TaskId | null, base?: string): Task[] {
    const layout = this.layouts.get(listKey(parentId, base ?? null))
    if (!layout) {
      const all = this.children(parentId)
      return parentId === null && base !== undefined ? all.filter((task) => task.base === base) : all
    }
    return layout.children.map((id) => this.get(id)).filter((task): task is Task => task !== undefined)
  }

  /** Mise en forme de la liste des sous-tâches de `parentId` (`null` : premier niveau de `base`). */
  layout(parentId: TaskId | null, base?: string): ListLayout | undefined {
    return this.layouts.get(listKey(parentId, base ?? null))
  }

  mediaOf(id: TaskId): MediaKind[] {
    return this.media[id] ?? []
  }

  /** Ancêtres, du parent direct vers la racine (protégé contre les boucles). */
  ancestors(id: TaskId): Task[] {
    const result: Task[] = []
    const seen = new Set<TaskId>([id])
    let current = this.get(id)
    while (current) {
      const parentId = this.parentOf(current)
      if (parentId === null || seen.has(parentId)) break
      seen.add(parentId)
      current = this.get(parentId)
      if (current) result.push(current)
    }
    return result
  }
}

/** Clé d'une liste : la tâche parente, ou le premier niveau d'une base. */
function listKey(taskId: TaskId | null, base: string | null): string {
  return taskId ?? `base:${base ?? ''}`
}

/**
 * Lignes visibles sous `rootId` (`null` = premier niveau de `base`), en ne descendant que dans
 * les nœuds dépliés. `depth0` : profondeur de la première génération (1 sous une section).
 */
export function visibleRows(
  index: TaskIndex,
  rootId: TaskId | null,
  expanded: ReadonlySet<TaskId>,
  base?: string,
  depth0 = 0,
): Row[] {
  const rows: Row[] = []
  const visit = (parentId: TaskId | null, depth: number) => {
    for (const task of index.shownChildren(parentId, parentId === null ? base : undefined)) {
      const children = index.children(task.id)
      const isExpanded = expanded.has(task.id) && index.shownChildren(task.id).length > 0
      rows.push({
        task,
        depth,
        childCount: children.length,
        doneChildCount: children.filter((child) => child.done).length,
        expanded: isExpanded,
        media: index.mediaOf(task.id),
      })
      if (isExpanded) visit(task.id, depth + 1)
    }
  }
  visit(rootId, depth0)
  return rows
}

/** En-tête de section d'une base, en mode « Tout » (ADR-0009). */
export interface BaseRow {
  kind: 'base'
  base: string
  expanded: boolean
  /** Tâches de premier niveau affichées dans la section. */
  count: number
}

/** Une ligne affichée : une tâche, ou l'en-tête d'une base. */
export type DisplayRow = ({ kind: 'task' } & Row) | BaseRow

/**
 * Lignes à afficher. Zoom : le sous-arbre de la tâche. Une seule base affichée : son premier
 * niveau. Plusieurs : une **section** par base (repliable), ses tâches en dessous.
 */
export function displayRows(
  index: TaskIndex,
  zoomId: TaskId | null,
  expanded: ReadonlySet<TaskId>,
  bases: readonly string[],
  collapsedBases: ReadonlySet<string>,
): DisplayRow[] {
  const asTasks = (rows: Row[]) => rows.map((row) => ({ kind: 'task' as const, ...row }))
  if (zoomId !== null) return asTasks(visibleRows(index, zoomId, expanded))
  if (bases.length === 1) return asTasks(visibleRows(index, null, expanded, bases[0]))
  return bases.flatMap((base) => {
    const sectionExpanded = !collapsedBases.has(base)
    const header: BaseRow = {
      kind: 'base',
      base,
      expanded: sectionExpanded,
      count: index.shownChildren(null, base).length,
    }
    return sectionExpanded ? [header, ...asTasks(visibleRows(index, null, expanded, base, 1))] : [header]
  })
}
