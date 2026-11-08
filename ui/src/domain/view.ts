/**
 * Vues par liste (spec §5.3) : types miroirs de `src-tauri/src/commands/views.rs`.
 * Le cœur Rust décide de l'héritage, du tri et des filtres ; l'interface affiche et édite.
 */
import type { Task, TaskId } from './task'

export type Column = 'progress' | 'tags' | 'icons' | 'createdAt' | 'updatedAt' | 'doneAt'
export type SortField = 'manual' | 'title' | 'createdAt' | 'updatedAt' | 'doneAt' | 'progress'
export type SortDirection = 'asc' | 'desc'
export type StatusChoice = 'all' | 'open' | 'done'
export type MediaChoice = 'any' | 'withMedia'
export type MediaKind = 'image' | 'audio' | 'file'

/** Toutes les colonnes, dans l'ordre proposé par l'éditeur de vue. */
export const ALL_COLUMNS: readonly Column[] = ['progress', 'tags', 'icons', 'createdAt', 'updatedAt', 'doneAt']
/** Tous les critères de tri. */
export const ALL_SORT_FIELDS: readonly SortField[] = ['manual', 'title', 'createdAt', 'updatedAt', 'doneAt', 'progress']

export interface SortKey {
  field: SortField
  direction: SortDirection
}

export interface ViewFilter {
  status: StatusChoice
  tagsIn: string[]
  tagsOut: string[]
  media: MediaChoice
}

export interface ListView {
  /** Colonnes affichées après le titre (toujours présent). */
  columns: Column[]
  sort: SortKey[]
  filter: ViewFilter
}

/** Origine de la vue appliquée à une liste. */
export type ViewSource = { kind: 'own' } | { kind: 'inherited'; fromId: TaskId } | { kind: 'root' }

/** Mise en forme d'une liste (`taskId: null` = liste racine). */
export interface ListLayout {
  /** Tâche dont c'est la liste ; `null` = premier niveau de `base`. */
  taskId: TaskId | null
  /** Base, pour une liste de premier niveau (`null` pour une liste de sous-tâches). */
  base: string | null
  view: ListView
  source: ViewSource
  /** Sous-tâches affichées (filtrées), dans l'ordre de la vue. */
  children: TaskId[]
}

/** Réponse de `list_tasks`. */
export interface Listing {
  tasks: Task[]
  /** Bases ouvertes, dans l'ordre du bandeau. */
  bases: string[]
  lists: ListLayout[]
  /** Médias de chaque tâche qui en a. */
  media: Record<TaskId, MediaKind[]>
}

/** Vue d'origine (miroir de `ListView::default()` en Rust). */
export function defaultView(): ListView {
  return {
    columns: ['progress', 'tags', 'icons'],
    sort: [{ field: 'manual', direction: 'asc' }],
    filter: { status: 'all', tagsIn: [], tagsOut: [], media: 'any' },
  }
}
