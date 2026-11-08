/**
 * Corbeille (spec §5.2, R6–R8) : tâches supprimées, restauration, suppression définitive
 * (après confirmation : irréversible).
 *
 * La liste vient du cœur (`listTrash` : les **racines** supprimées, les plus récentes d'abord ;
 * leurs sous-tâches suivent la restauration). La restauration elle-même est celle de l'arbre
 * (`TaskTree.restore`), qui sélectionne la tâche et signale un retour au premier niveau.
 */
import { ref, shallowRef } from 'vue'
import type { Task, TaskId } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'
import type { Confirm } from './useConfirm'
import { t } from '../i18n'

export function useTrash(gateway: TaskGateway, restoreInTree: (id: TaskId) => Promise<void>, confirm: Confirm) {
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

  /** Supprime définitivement, après confirmation (irréversible). Renvoie vrai si c'est fait. */
  async function purge(task: Task): Promise<boolean> {
    const choice = await confirm.ask({
      title: t('trash.purgeTitle'),
      message: t('trash.purgeMessage', { title: task.title }),
      choices: [
        { id: 'cancel', label: t('vault.cancel') },
        { id: 'purge', label: t('trash.purge'), primary: true },
      ],
    })
    if (choice !== 'purge') return false
    await gateway.purgeTask(task.id)
    await load()
    return true
  }

  return { items, loading, open, show, hide, load, restore, purge }
}

export type Trash = ReturnType<typeof useTrash>
