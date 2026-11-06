/**
 * Port : ce que l'interface attend du cœur de l'application.
 *
 * Les composants et composables n'appellent **jamais** `invoke()` directement : ils passent par
 * cette interface, injectée au démarrage (`main.ts`). Deux implémentations :
 * - `TauriTaskGateway` : la vraie, via les commandes Tauri ;
 * - `InMemoryTaskGateway` : en mémoire, pour les tests et pour `pnpm ui:dev` dans un navigateur.
 */
import type { InjectionKey } from 'vue'
import type {
  CloseImpact,
  CloseOutcome,
  CloseScope,
  DeleteOutcome,
  MoveOutcome,
  Movement,
  Position,
  ReopenOutcome,
  RestoreOutcome,
  Task,
  TaskId,
} from '../domain/task'

export interface TaskGateway {
  listTasks(): Promise<Task[]>
  listTrash(): Promise<Task[]>
  createTask(title: string, position: Position): Promise<TaskId>
  renameTask(taskId: TaskId, title: string): Promise<void>
  setTags(taskId: TaskId, tags: string[]): Promise<void>
  getCloseImpact(taskId: TaskId): Promise<CloseImpact>
  closeTask(taskId: TaskId, scope: CloseScope): Promise<CloseOutcome>
  reopenTask(taskId: TaskId): Promise<ReopenOutcome>
  moveTask(taskId: TaskId, movement: Movement): Promise<MoveOutcome>
  deleteTask(taskId: TaskId): Promise<DeleteOutcome>
  restoreTask(taskId: TaskId): Promise<RestoreOutcome>
  /** S'abonne aux changements faits ailleurs (autre fenêtre, synchronisation). Renvoie le désabonnement. */
  onTasksChanged(listener: (ids: TaskId[]) => void): Promise<() => void>
}

/** Clé d'injection Vue (`provide` dans main.ts, `inject` dans les composants). */
export const taskGatewayKey: InjectionKey<TaskGateway> = Symbol('TaskGateway')
