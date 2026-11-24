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
  ArchiveImpact,
  ArchiveOutcome,
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
import type { ArchiveVisibility, ListView, Listing } from '../domain/view'

export interface TaskGateway {
  /**
   * Tâches actives et mise en forme de chaque liste (vues, tri, filtres : décidés par le cœur) ;
   * avec `'shown'`, les archives aussi, à leur place (R15). Par défaut : cachées.
   */
  listTasks(archives?: ArchiveVisibility): Promise<Listing>
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
  // Les actions suivantes prennent une **sélection** (spec §5.5) : une tâche, ou plusieurs.
  /** Met les tâches et leurs sous-tâches à la corbeille (R5 ; une archive aussi, R17). */
  deleteTasks(taskIds: TaskId[]): Promise<DeleteOutcome>
  /** Restaure des tâches de la corbeille (R7) ; « Annuler » d'une suppression. */
  restoreTasks(taskIds: TaskId[]): Promise<RestoreOutcome>
  /** Combien de tâches (dont ouvertes) l'archivage de `taskIds` emporterait (R13). */
  getArchiveImpact(taskIds: TaskId[]): Promise<ArchiveImpact>
  /** Archive les tâches et leurs sous-tâches actives (dossier `archive/` de leur base, ADR-0015). */
  archiveTasks(taskIds: TaskId[]): Promise<ArchiveOutcome>
  /** Restaure des archives, et ce qui a été archivé avec elles (R16) ; « Annuler » d'un archivage. */
  unarchiveTasks(taskIds: TaskId[]): Promise<RestoreOutcome>
  /** Supprime définitivement une tâche de la corbeille (et ses sous-tâches à la corbeille) ; renvoie les tâches effacées. */
  purgeTask(taskId: TaskId): Promise<TaskId[]>
  /** Tâche et note (panneau de détail). */
  getTaskDetail(taskId: TaskId): Promise<TaskDetail>
  /** Remplace la note (sauvegarde automatique de l'éditeur). */
  updateNote(taskId: TaskId, note: NoteDocument): Promise<void>
  /** Recherche plein texte (syntaxe : `tag:`, `-tag:`, `is:`, `has:`, `in:"…"`, `"phrase"`). */
  /** Recherche dans `bases` (vide = toutes : ce sont les bases affichées, ADR-0009). */
  /** Recherche dans `bases` (vide = toutes) ; archives comprises avec `'shown'` (R15). */
  searchTasks(query: string, bases: string[], archives?: ArchiveVisibility): Promise<SearchHit[]>
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
