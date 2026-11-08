/**
 * Faux `TaskGateway` en mémoire, pour les tests et pour travailler l'interface dans un simple
 * navigateur (`pnpm ui:dev`), sans compiler le Rust.
 *
 * **Ce n'est pas une seconde implémentation des règles métier** : il stocke et déplace des
 * tâches de façon simple et ne fait **aucune proposition** R1/R3/R12. Les règles de référence
 * sont en Rust (`crates/domain`) ; les tests de l'interface qui ont besoin d'une proposition
 * utilisent un faux qui la renvoie explicitement.
 *
 * Les vues par liste sont enregistrées et héritées, mais **ni triées ni filtrées** ici (ordre
 * manuel) : le tri et les filtres de référence sont en Rust (`tq_domain::view`).
 *
 * La recherche y est **approximative** (mots dans le titre ou la note, sans filtres `tag:`…,
 * sans extrait) : de quoi essayer l'interface ; la vraie est en Rust (`tq_domain::search`).
 */
import type { TaskGateway } from '../../ports/TaskGateway'
import { defaultView, type ListLayout, type ListView, type Listing, type ViewSource } from '../../domain/view'
import { MAIN_BASE, type BaseId } from '../../domain/task'
import {
  compareSiblings,
  emptyNote,
  type NoteDocument,
  type TaskDetail,
  type CloseImpact,
  type CloseOutcome,
  type CloseScope,
  type DeleteOutcome,
  type MoveOutcome,
  type Movement,
  type Position,
  type ReopenOutcome,
  type RestoreOutcome,
  type SearchHit,
  type Task,
  type TaskId,
  type UiError,
} from '../../domain/task'

export class InMemoryTaskGateway implements TaskGateway {
  private readonly tasks = new Map<TaskId, Task>()
  private readonly listeners = new Set<(ids: TaskId[], elsewhere: boolean) => void>()
  /** Tâches pour lesquelles « Garder la mienne » a été demandé (consultable par les tests). */
  readonly keptLocal: TaskId[] = []
  private readonly notes = new Map<TaskId, NoteDocument>()
  private readonly views = new Map<TaskId, ListView>()
  private readonly rootViews = new Map<BaseId, ListView>()
  /** Bases ouvertes, dans l'ordre du bandeau. */
  private readonly baseList: BaseId[]
  /** Fenêtres demandées (pas de vraie fenêtre dans un navigateur) : consultable par les tests. */
  readonly openedWindows: TaskId[] = []
  private counter = 0

  constructor(initial: Task[] = [], bases?: BaseId[]) {
    for (const task of initial) this.tasks.set(task.id, { ...task })
    this.counter = initial.length
    const found = [...new Set(initial.map((task) => task.base))]
    this.baseList = bases ?? (found.length > 0 ? found : [MAIN_BASE])
  }

  async listTasks(): Promise<Listing> {
    const tasks = [...this.tasks.values()].filter((t) => !t.deletedAt).map((t) => ({ ...t }))
    tasks.sort(compareSiblings)
    const lists = [
      ...this.baseList.map((base) => this.layout(null, base, tasks)),
      ...tasks.map((task) => this.layout(task.id, task.base, tasks)),
    ]
    return { tasks, bases: [...this.baseList], lists, media: {} }
  }
  async customizeListView(taskId: TaskId | null, base: BaseId | null, view: ListView): Promise<void> {
    if (taskId === null) this.rootViews.set(base ?? MAIN_BASE, structuredClone(view))
    else this.views.set(this.get(taskId).id, structuredClone(view))
    this.notify([])
  }
  async inheritListView(taskId: TaskId): Promise<void> {
    this.views.delete(this.get(taskId).id)
    this.notify([])
  }
  /** Racines supprimées (pas leurs sous-tâches supprimées en même temps), les plus récentes d'abord. */
  async listTrash(): Promise<Task[]> {
    const deleted = [...this.tasks.values()].filter((t) => t.deletedAt)
    const isRoot = (t: Task) => {
      const parent = t.parentId === null ? undefined : this.tasks.get(t.parentId)
      return !parent || parent.deletedAt !== t.deletedAt
    }
    return deleted
      .filter(isRoot)
      .sort((a, b) => (b.deletedAt ?? '').localeCompare(a.deletedAt ?? ''))
      .map((t) => ({ ...t }))
  }

  async createTask(title: string, position: Position): Promise<TaskId> {
    if (!title.trim()) throw ruleError('le titre ne peut pas être vide')
    const id = `00000000-0000-7000-8000-${(++this.counter).toString(16).padStart(12, '0')}`
    const parentId =
      position.kind === 'atEnd'
        ? position.parentId
        : position.kind === 'after'
          ? this.get(position.siblingId).parentId
          : null
    const base =
      position.kind === 'atTopOf'
        ? position.base
        : position.kind === 'after'
          ? this.get(position.siblingId).base
          : parentId
            ? this.get(parentId).base
            : MAIN_BASE
    const now = new Date().toISOString()
    const task: Task = {
      id,
      parentId,
      order: '',
      title: title.trim(),
      done: false,
      doneAt: null,
      tags: [],
      hasNote: false,
      createdAt: now,
      updatedAt: now,
      updatedBy: 'navigateur',
      deletedAt: null,
      base,
    }
    this.tasks.set(id, task)
    this.place(id, parentId, position.kind === 'after' ? position.siblingId : this.lastChild(parentId, id, base))
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
    const ids = [
      taskId,
      ...(scope === 'withOpenDescendants'
        ? this.descendants(taskId)
            .filter((t) => !t.done)
            .map((t) => t.id)
        : []),
    ]
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
    if (movement.kind === 'to') {
      const target = movement.parentId ?? movement.afterId
      if (target) this.rebase(taskId, this.get(target).base)
      this.place(taskId, movement.parentId, movement.afterId)
    }
    if (movement.kind === 'toBase') {
      this.rebase(taskId, movement.base)
      this.place(taskId, null, this.lastChild(null, taskId, movement.base))
    }
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

  async getTaskDetail(taskId: TaskId): Promise<TaskDetail> {
    return { task: { ...this.get(taskId) }, note: structuredClone(this.notes.get(taskId) ?? emptyNote()) }
  }
  async updateNote(taskId: TaskId, note: NoteDocument): Promise<void> {
    this.notes.set(taskId, structuredClone(note))
    this.update(taskId, { hasNote: JSON.stringify(note).includes('"text"') })
  }
  async searchTasks(query: string, bases: BaseId[] = []): Promise<SearchHit[]> {
    const terms = fold(query).split(/\s+/).filter(Boolean)
    if (terms.length === 0) return []
    return [...this.tasks.values()]
      .filter((task) => !task.deletedAt && (bases.length === 0 || bases.includes(task.base)))
      .filter((task) => {
        const text = fold(`${task.title} ${task.tags.join(' ')} ${JSON.stringify(this.notes.get(task.id) ?? '')}`)
        return terms.every((term) => text.includes(term))
      })
      .map((task) => ({
        id: task.id,
        base: task.base,
        title: task.title,
        path: this.pathOf(task),
        done: task.done,
        titleHighlights: [],
        snippet: null,
      }))
  }
  async openTaskWindow(taskId: TaskId): Promise<void> {
    this.get(taskId)
    this.openedWindows.push(taskId)
  }

  async onTasksChanged(listener: (ids: TaskId[], elsewhere: boolean) => void): Promise<() => void> {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }
  async keepLocalVersion(taskId: TaskId): Promise<void> {
    this.keptLocal.push(this.get(taskId).id)
  }

  /** Simule une modification faite sur un autre appareil (tests, démonstration). */
  simulateChangeElsewhere(taskId: TaskId, changes: Partial<Task>, note?: NoteDocument): void {
    Object.assign(this.get(taskId), changes, { updatedAt: new Date().toISOString() })
    if (note) this.notes.set(taskId, structuredClone(note))
    for (const listener of this.listeners) listener([taskId], true)
  }

  // ----- Outils internes -------------------------------------------------------------------

  private layout(taskId: TaskId | null, base: BaseId, tasks: Task[]): ListLayout {
    const children = tasks
      .filter((task) => task.parentId === taskId && (taskId !== null || task.base === base))
      .map((task) => task.id)
    const [view, source] = this.effectiveView(taskId, base)
    return { taskId, base: taskId === null ? base : null, view: structuredClone(view), source, children }
  }
  private effectiveView(taskId: TaskId | null, base: BaseId): [ListView, ViewSource] {
    const rootView = this.rootViews.get(base) ?? defaultView()
    if (taskId === null) return [rootView, { kind: 'own' }]
    const own = this.views.get(taskId)
    if (own) return [own, { kind: 'own' }]
    let parentId = this.tasks.get(taskId)?.parentId ?? null
    for (let depth = 0; parentId !== null && depth < this.tasks.size; depth++) {
      const view = this.views.get(parentId)
      if (view) return [view, { kind: 'inherited', fromId: parentId }]
      parentId = this.tasks.get(parentId)?.parentId ?? null
    }
    return [rootView, { kind: 'root' }]
  }
  /** La tâche et ses sous-tâches passent dans `base` (déplacement entre bases). */
  private rebase(taskId: TaskId, base: BaseId): void {
    for (const id of [taskId, ...this.descendants(taskId).map((task) => task.id)]) this.get(id).base = base
  }
  private pathOf(task: Task): string[] {
    const path: string[] = []
    let parent = task.parentId === null ? undefined : this.tasks.get(task.parentId)
    while (parent && path.length < this.tasks.size) {
      path.unshift(parent.title)
      parent = parent.parentId === null ? undefined : this.tasks.get(parent.parentId)
    }
    return path
  }
  private get(id: TaskId): Task {
    const task = this.tasks.get(id)
    if (!task) throw ruleError(`tâche introuvable : ${id}`)
    return task
  }
  private update(id: TaskId, changes: Partial<Task>): void {
    Object.assign(this.get(id), changes, { updatedAt: new Date().toISOString() })
    this.notify([id])
  }
  private notify(ids: TaskId[]): void {
    for (const listener of this.listeners) listener(ids, false)
  }
  private children(parentId: TaskId | null): Task[] {
    return [...this.tasks.values()].filter((t) => !t.deletedAt && t.parentId === parentId).sort(compareSiblings)
  }
  private lastChild(parentId: TaskId | null, except: TaskId, base?: BaseId): TaskId | null {
    return (
      this.children(parentId)
        .filter((t) => t.id !== except && (parentId !== null || base === undefined || t.base === base))
        .at(-1)?.id ?? null
    )
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

/** Minuscules sans accents (approximation de `fold_text` côté Rust). */
function fold(text: string): string {
  return text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
}

function ruleError(message: string): UiError {
  return { code: 'rule', message }
}
