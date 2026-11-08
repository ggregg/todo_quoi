/** Données fictives pour travailler l'interface dans un navigateur (`pnpm ui:dev`) : deux bases, « perso » et « pro ». */
import type { Task } from '../../domain/task'

const at = '2026-11-06T09:00:00.000Z'

function task(n: number, title: string, parent: number | null, extra: Partial<Task> = {}): Task {
  const id = (k: number) => `00000000-0000-7000-8000-${k.toString(16).padStart(12, '0')}`
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
    updatedBy: 'navigateur',
    deletedAt: null,
    base: 'perso',
    confidentiality: 'plain',
    ...extra,
  }
}

export const sampleTasks: Task[] = [
  task(1, 'Release 1.2 de l’extranet', null, { tags: ['release', 'client-x'], hasNote: true, base: 'pro' }),
  task(2, 'Changelog', 1, { done: true, doneAt: at, base: 'pro' }),
  task(3, 'Recette fonctionnelle', 1, { tags: ['recette'], base: 'pro' }),
  task(4, 'Scénarios connexion SSO', 3, { done: true, doneAt: at, base: 'pro' }),
  task(5, 'Scénarios export PDF', 3, { base: 'pro' }),
  task(6, 'Déploiement production', 1, { tags: ['ops'], base: 'pro' }),
  task(7, 'Courses week-end', null, { tags: ['perso'] }),
  task(8, 'Pain', 7, { done: true, doneAt: at }),
  task(9, 'Fromage', 7),
  task(10, 'Apprendre Tauri 2', null, { tags: ['veille'] }),
]
