/**
 * Calculs d'**affichage** de l'arbre (fonctions pures, sans Vue) : enfants triés, ancêtres,
 * lignes visibles selon les nœuds dépliés et le zoom.
 *
 * Ce ne sont pas des règles métier : on se contente de présenter la liste « à plat » renvoyée
 * par le cœur Rust (`parentId` + `order`). Comme en Rust, une tâche dont le parent est
 * introuvable est affichée au premier niveau (`TaskTree::effective_parent`).
 */
import { compareSiblings, type Task, type TaskId } from '../domain/task'

/** Une ligne de la liste affichée. */
export interface Row {
  task: Task
  /** Profondeur sous la racine affichée (0 = premier niveau, ou enfants directs du zoom). */
  depth: number
  childCount: number
  doneChildCount: number
  expanded: boolean
}

/** Index de lecture sur un instantané des tâches. */
export class TaskIndex {
  private readonly byIdMap: Map<TaskId, Task>
  private readonly childrenMap = new Map<TaskId | null, Task[]>()

  constructor(tasks: Task[]) {
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

  children(parentId: TaskId | null): Task[] {
    return this.childrenMap.get(parentId) ?? []
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
    for (const task of index.children(parentId)) {
      const children = index.children(task.id)
      const isExpanded = expanded.has(task.id) && children.length > 0
      rows.push({
        task,
        depth,
        childCount: children.length,
        doneChildCount: children.filter((child) => child.done).length,
        expanded: isExpanded,
      })
      if (isExpanded) visit(task.id, depth + 1)
    }
  }
  visit(rootId, 0)
  return rows
}
