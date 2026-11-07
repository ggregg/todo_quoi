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
  private readonly layouts: Map<TaskId | null, ListLayout>

  constructor(
    tasks: Task[],
    layouts: ListLayout[] = [],
    private readonly media: Record<TaskId, MediaKind[]> = {},
  ) {
    this.layouts = new Map(layouts.map((layout) => [layout.taskId, layout]))
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

  /** Sous-tâches **affichées** : filtrées et triées selon la vue de la liste. */
  shownChildren(parentId: TaskId | null): Task[] {
    const layout = this.layouts.get(parentId)
    if (!layout) return this.children(parentId)
    return layout.children.map((id) => this.get(id)).filter((task): task is Task => task !== undefined)
  }

  /** Mise en forme de la liste des sous-tâches de `parentId` (`null` = racine). */
  layout(parentId: TaskId | null): ListLayout | undefined {
    return this.layouts.get(parentId)
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

/** Lignes visibles sous `rootId` (`null` = tout), en ne descendant que dans les nœuds dépliés. */
export function visibleRows(index: TaskIndex, rootId: TaskId | null, expanded: ReadonlySet<TaskId>): Row[] {
  const rows: Row[] = []
  const visit = (parentId: TaskId | null, depth: number) => {
    for (const task of index.shownChildren(parentId)) {
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
  visit(rootId, 0)
  return rows
}
