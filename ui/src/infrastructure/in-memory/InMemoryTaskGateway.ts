/**
 * Faux `TaskGateway` en mémoire, pour les tests et pour travailler l'interface dans un simple
 * navigateur (`pnpm ui:dev`), sans compiler le Rust.
 *
 * **Ce n'est pas une seconde implémentation des règles métier** : il stocke et déplace des
 * tâches de façon simple et ne fait **aucune proposition** R1/R3/R12. Les règles de référence
 * sont en Rust (`crates/domain`) ; les tests de l'interface qui ont besoin d'une proposition
 * utilisent un faux qui la renvoie explicitement.
 */
import type { TaskGateway } from '../../ports/TaskGateway'
import {
  compareSiblings,
  type CloseImpact,
  type CloseOutcome,
  type CloseScope,
  type DeleteOutcome,
  type MoveOutcome,
  type Movement,
  type Position,
  type ReopenOutcome,
  type RestoreOutcome,
  type Task,
  type TaskId,
  type UiError,
} from '../../domain/task'

export class InMemoryTaskGateway implements TaskGateway {
  private readonly tasks = new Map<TaskId, Task>()
  private readonly listeners = new Set<(ids: TaskId[]) => void>()
  private counter = 0

  constructor(initial: Task[] = []) {
    for (const task of initial) this.tasks.set(task.id, { ...task })
    this.counter = initial.length
  }

  async listTasks(): Promise<Task[]> {
    return [...this.tasks.values()].filter((t) => !t.deletedAt).map((t) => ({ ...t }))
  }
  async listTrash(): Promise<Task[]> {
    return [...this.tasks.values()].filter((t) => t.deletedAt).map((t) => ({ ...t }))
  }

  async createTask(title: string, position: Position): Promise<TaskId> {
    if (!title.trim()) throw ruleError('le titre ne peut pas être vide')
    const id = `00000000-0000-7000-8000-${(++this.counter).toString(16).padStart(12, '0')}`
    const parentId = position.kind === 'atEnd' ? position.parentId : this.get(position.siblingId).parentId
    const now = new Date().toISOString()
    const task: Task = {
      id, parentId, order: '', title: title.trim(), done: false, doneAt: null, tags: [], hasNote: false,
      createdAt: now, updatedAt: now, updatedBy: 'navigateur', deletedAt: null,
    }
    this.tasks.set(id, task)
    this.place(id, parentId, position.kind === 'after' ? position.siblingId : this.lastChild(parentId, id))
    return id
  }

  async renameTask(taskId: TaskId, title: string): Promise<void> {
    if (!title.trim()) throw ruleError('le titre ne peut pas être vide')
    this.update(taskId, { title: title.trim() })
  }
  async setTags(taskId: TaskId, tags: string[]): Promise<void> {
    this.update(taskId, { tags: [...new Set(tags.map((t) => t.trim().replace(/^#/, '').toLowerCase()))].sort() })
  }
  async getCloseImpact(taskId: TaskId): Promise<CloseImpact> {
    return { openDescendants: this.descendants(taskId).filter((t) => !t.done).length }
  }
  async closeTask(taskId: TaskId, scope: CloseScope): Promise<CloseOutcome> {
    const ids = [taskId, ...(scope === 'withOpenDescendants' ? this.descendants(taskId).filter((t) => !t.done).map((t) => t.id) : [])]
    for (const id of ids) this.update(id, { done: true, doneAt: new Date().toISOString() })
    return { closed: ids, proposeClosingParent: null }
  }
  async reopenTask(taskId: TaskId): Promise<ReopenOutcome> {
    this.update(taskId, { done: false, doneAt: null })
    return { proposeReopening: [] }
  }

  async moveTask(taskId: TaskId, movement: Movement): Promise<MoveOutcome> {
    const task = this.get(taskId)
    const siblings = this.children(task.parentId)
    const index = siblings.findIndex((t) => t.id === taskId)
    const previous = siblings[index - 1]
    if (movement.kind === 'indent' && previous) this.place(taskId, previous.id, this.lastChild(previous.id, taskId))
    if (movement.kind === 'outdent' && task.parentId) {
      this.place(taskId, this.get(task.parentId).parentId, task.parentId)
    }
    if (movement.kind === 'up' && previous) this.place(taskId, task.parentId, siblings[index - 2]?.id ?? null)
    if (movement.kind === 'down' && siblings[index + 1]) this.place(taskId, task.parentId, siblings[index + 1]!.id)
    if (movement.kind === 'to') this.place(taskId, movement.parentId, movement.afterId)
    return { proposeClosing: null, proposeReopening: [] }
  }

  async deleteTask(taskId: TaskId): Promise<DeleteOutcome> {
    const at = new Date().toISOString()
    const ids = [taskId, ...this.descendants(taskId).map((t) => t.id)]
    for (const id of ids) this.update(id, { deletedAt: at })
    return { deleted: ids }
  }
  async restoreTask(taskId: TaskId): Promise<RestoreOutcome> {
    const deletedAt = this.get(taskId).deletedAt
    const ids = [...this.tasks.values()].filter((t) => t.deletedAt === deletedAt).map((t) => t.id)
    for (const id of ids) this.update(id, { deletedAt: null })
    return { restored: ids, movedToRoot: false }
  }

  async onTasksChanged(listener: (ids: TaskId[]) => void): Promise<() => void> {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  // ----- Outils internes -------------------------------------------------------------------

  private get(id: TaskId): Task {
    const task = this.tasks.get(id)
    if (!task) throw ruleError(`tâche introuvable : ${id}`)
    return task
  }
  private update(id: TaskId, changes: Partial<Task>): void {
    Object.assign(this.get(id), changes, { updatedAt: new Date().toISOString() })
    for (const listener of this.listeners) listener([id])
  }
  private children(parentId: TaskId | null): Task[] {
    return [...this.tasks.values()].filter((t) => !t.deletedAt && t.parentId === parentId).sort(compareSiblings)
  }
  private lastChild(parentId: TaskId | null, except: TaskId): TaskId | null {
    return this.children(parentId).filter((t) => t.id !== except).at(-1)?.id ?? null
  }
  private descendants(id: TaskId): Task[] {
    return this.children(id).flatMap((child) => [child, ...this.descendants(child.id)])
  }
  /** Place `id` sous `parentId`, juste après `afterId` (`null` = en tête), puis renumérote les sœurs. */
  private place(id: TaskId, parentId: TaskId | null, afterId: TaskId | null): void {
    const siblings = this.children(parentId).filter((t) => t.id !== id)
    const insertAt = afterId === null ? 0 : siblings.findIndex((t) => t.id === afterId) + 1
    const ordered = [...siblings.slice(0, insertAt), this.get(id), ...siblings.slice(insertAt)]
    ordered.forEach((task, index) => {
      task.order = `k${String(index).padStart(6, '0')}`
    })
    this.update(id, { parentId })
  }
}

function ruleError(message: string): UiError {
  return { code: 'rule', message }
}
