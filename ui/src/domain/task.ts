/**
 * Types échangés avec le cœur Rust (DTO). Miroir de `src-tauri/src/dto.rs` : mêmes noms de
 * champs (camelCase). Toute modification doit être faite des deux côtés ; le test
 * `src-tauri/src/dto.rs::tests` vérifie la forme JSON côté Rust.
 *
 * Aucune règle métier ici : le Rust décide, l'interface affiche (CLAUDE.md).
 */

export type TaskId = string

export interface Task {
  id: TaskId
  /** `null` = premier niveau (ou parent introuvable). */
  parentId: TaskId | null
  /** Clé d'ordre : les sœurs se trient par comparaison de texte (voir `compareSiblings`). */
  order: string
  title: string
  done: boolean
  doneAt: string | null
  tags: string[]
  hasNote: boolean
  createdAt: string
  updatedAt: string
  updatedBy: string
  deletedAt: string | null
}

export type Position =
  | { kind: 'atEnd'; parentId: TaskId | null }
  | { kind: 'after'; siblingId: TaskId }

export type Movement =
  | { kind: 'indent' }
  | { kind: 'outdent' }
  | { kind: 'up' }
  | { kind: 'down' }
  | { kind: 'to'; parentId: TaskId | null; afterId: TaskId | null }

/** Réponse à « clôturer aussi les sous-tâches ouvertes ? » (R2). */
export type CloseScope = 'taskOnly' | 'withOpenDescendants'

export interface CloseImpact {
  openDescendants: number
}
export interface CloseOutcome {
  closed: TaskId[]
  /** R1 : parent à proposer à la clôture. */
  proposeClosingParent: TaskId | null
}
export interface ReopenOutcome {
  /** R3 : ancêtres terminés à proposer à la réouverture. */
  proposeReopening: TaskId[]
}
export interface MoveOutcome {
  /** R12 : ancien parent à proposer à la clôture. */
  proposeClosing: TaskId | null
  /** R12 : nouveaux ancêtres terminés à proposer à la réouverture. */
  proposeReopening: TaskId[]
}
export interface DeleteOutcome {
  deleted: TaskId[]
}
export interface RestoreOutcome {
  restored: TaskId[]
  movedToRoot: boolean
}

/** Erreur renvoyée par une commande (voir `src-tauri/src/error.rs`). */
export interface UiError {
  code: 'rule' | 'storage' | 'invalid-input' | 'internal'
  message: string
}

export function isUiError(value: unknown): value is UiError {
  return typeof value === 'object' && value !== null && 'code' in value && 'message' in value
}

/**
 * Ordre d'affichage de deux sœurs : clé d'ordre puis identifiant, **comparés octet par octet**
 * comme en Rust (pas `localeCompare`, qui suivrait les règles de la langue).
 */
export function compareSiblings(a: Task, b: Task): number {
  if (a.order !== b.order) return a.order < b.order ? -1 : 1
  if (a.id !== b.id) return a.id < b.id ? -1 : 1
  return 0
}
