/**
 * Corbeille (spec §5.2, R6–R7) : tâches supprimées, restauration.
 *
 * La liste vient du cœur (`listTrash` : les **racines** supprimées, les plus récentes d'abord ;
 * leurs sous-tâches suivent la restauration). La restauration elle-même est celle de l'arbre
 * (`TaskTree.restore`), qui sélectionne la tâche et signale un retour au premier niveau.
 */
import { ref, shallowRef } from 'vue'
import type { Task, TaskId } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'

export function useTrash(gateway: TaskGateway, restoreInTree: (id: TaskId) => Promise<void>) {
  const items = shallowRef<Task[]>([])
  const loading = ref(false)
  const open = ref(false)

  async function load(): Promise<void> {
    loading.value = true
    try {
      items.value = await gateway.listTrash()
    } finally {
      loading.value = false
    }
  }

  async function show(): Promise<void> {
    open.value = true
    await load()
  }

  function hide(): void {
    open.value = false
  }

  /** Restaure la tâche (et ses sous-tâches supprimées avec elle), puis met la liste à jour. */
  async function restore(id: TaskId): Promise<void> {
    await restoreInTree(id)
    await load()
  }

  return { items, loading, open, show, hide, load, restore }
}

export type Trash = ReturnType<typeof useTrash>
