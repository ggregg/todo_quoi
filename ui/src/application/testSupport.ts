/** Outils de test partagés : fabriquer des tâches lisiblement. */
import { MAIN_BASE, type Task } from '../domain/task'

export const id = (n: number) => `00000000-0000-7000-8000-${n.toString(16).padStart(12, '0')}`

export function makeTask(n: number, title: string, parent: number | null = null, extra: Partial<Task> = {}): Task {
  const at = '2026-11-06T09:00:00.000Z'
  return {
    id: id(n),
    parentId: parent === null ? null : id(parent),
    order: `k${String(n).padStart(6, '0')}`,
    title,
    done: false,
    doneAt: null,
    tags: [],
    hasNote: false,
    createdAt: at,
    updatedAt: at,
    updatedBy: 'test',
    deletedAt: null,
    base: MAIN_BASE,
    ...extra,
  }
}
