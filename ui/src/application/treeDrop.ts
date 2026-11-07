/**
 * Glisser-déposer dans l'arbre : traduire « lâché ici » en déplacement (`Movement` `to`).
 *
 * Trois zones sur une ligne : le quart du haut = **avant** la tâche, le quart du bas = **après**,
 * le milieu = **dedans** (à la fin de ses sous-tâches). Lâcher sur un fond de liste ou sur un
 * élément du fil d'Ariane = à la fin de cette liste : c'est ainsi qu'on **détache** une tâche.
 *
 * Seule l'interface est concernée ici ; le cœur Rust revérifie tout (R11 : pas de cycle) et
 * applique les propositions R12.
 */
import type { Movement, TaskId } from '../domain/task'
import type { TaskIndex } from './treeView'

export type DropZone = 'before' | 'inside' | 'after'

/** Zone visée selon la position verticale du pointeur dans la ligne (0 = haut, 1 = bas). */
export function dropZoneAt(ratio: number): DropZone {
  if (ratio < 0.25) return 'before'
  if (ratio > 0.75) return 'after'
  return 'inside'
}

/** Vrai si `candidate` est `ancestor` ou l'un de ses descendants (déposer là créerait un cycle). */
export function isWithin(index: TaskIndex, candidate: TaskId, ancestor: TaskId): boolean {
  return candidate === ancestor || index.ancestors(candidate).some((task) => task.id === ancestor)
}

/** Déplacement pour `dragged` lâché sur la ligne `target`, ou `null` si c'est impossible. */
export function dropOnTask(index: TaskIndex, dragged: TaskId, target: TaskId, zone: DropZone): Movement | null {
  const targetTask = index.get(target)
  if (!targetTask || isWithin(index, target, dragged)) return null
  if (zone === 'inside') return dropAtEnd(index, dragged, target)
  const parentId = index.parentOf(targetTask)
  const siblings = index.children(parentId).filter((task) => task.id !== dragged)
  const position = siblings.findIndex((task) => task.id === target)
  const afterId = zone === 'after' ? target : (siblings[position - 1]?.id ?? null)
  return { kind: 'to', parentId, afterId }
}

/** Déplacement pour `dragged` lâché à la fin de la liste `parentId` (`null` = premier niveau). */
export function dropAtEnd(index: TaskIndex, dragged: TaskId, parentId: TaskId | null): Movement | null {
  if (parentId !== null && isWithin(index, parentId, dragged)) return null
  const last = index
    .children(parentId)
    .filter((task) => task.id !== dragged)
    .at(-1)
  return { kind: 'to', parentId, afterId: last?.id ?? null }
}
