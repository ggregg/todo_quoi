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
  NoteDocument,
  Position,
  ReopenOutcome,
  RestoreOutcome,
  SearchHit,
  Task,
  TaskDetail,
  TaskId,
} from '../domain/task'
import type { ListView, Listing } from '../domain/view'

export interface TaskGateway {
  /** Tâches actives et mise en forme de chaque liste (vues, tri, filtres : décidés par le cœur). */
  listTasks(): Promise<Listing>
  /** « Personnaliser cette liste » (`taskId` `null` = liste racine). */
  /** Personnalise les sous-tâches de `taskId`, ou le premier niveau de `base` si `taskId` est `null`. */
  customizeListView(taskId: TaskId | null, base: string | null, view: ListView): Promise<void>
  /** « Revenir à l'héritage ». */
  inheritListView(taskId: TaskId): Promise<void>
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
  /** Tâche et note (panneau de détail). */
  getTaskDetail(taskId: TaskId): Promise<TaskDetail>
  /** Remplace la note (sauvegarde automatique de l'éditeur). */
  updateNote(taskId: TaskId, note: NoteDocument): Promise<void>
  /** Recherche plein texte (syntaxe : `tag:`, `-tag:`, `is:`, `has:`, `in:"…"`, `"phrase"`). */
  /** Recherche dans `bases` (vide = toutes : ce sont les bases affichées, ADR-0009). */
  searchTasks(query: string, bases: string[]): Promise<SearchHit[]>
  /** Ouvre la tâche seule dans sa propre fenêtre (ou la ramène au premier plan). */
  openTaskWindow(taskId: TaskId): Promise<void>
  /**
   * S'abonne aux changements (autre fenêtre, synchronisation). `elsewhere` : le changement vient
   * d'un **autre appareil** (scan du drive). Renvoie le désabonnement.
   */
  onTasksChanged(listener: (ids: TaskId[], elsewhere: boolean) => void): Promise<() => void>
  /** « Garder la mienne » : met de côté (`.conflicts/`) la version venue d'un autre appareil. */
  keepLocalVersion(taskId: TaskId): Promise<void>
}

/** Clé d'injection Vue (`provide` dans main.ts, `inject` dans les composants). */
export const taskGatewayKey: InjectionKey<TaskGateway> = Symbol('TaskGateway')
