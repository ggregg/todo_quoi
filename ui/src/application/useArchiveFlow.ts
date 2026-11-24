/**
 * Archiver avec la question de R13 (« Archiver aussi les N tâches ouvertes ? »). Partagé par la
 * liste (`useTaskTree`) et la fenêtre de tâche (`useTaskDetail`).
 *
 * C'est le cœur Rust qui compte les tâches concernées (`getArchiveImpact`) et qui archive : ici
 * on ne fait que poser la question et transmettre la réponse.
 */
import { t } from '../i18n'
import type { TaskId } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'
import type { Confirm } from './useConfirm'

export function useArchiveFlow(gateway: TaskGateway, confirm: Confirm) {
  /** Archive `id` (titre `title`, pour la question), après confirmation s'il reste des tâches ouvertes. Vrai si c'est fait. */
  async function archive(id: TaskId, title: string): Promise<boolean> {
    const impact = await gateway.getArchiveImpact(id)
    if (impact.openTasks > 0) {
      const message =
        impact.tasks === 1
          ? t('archive.open.single', { title })
          : t('archive.open.many', { title, count: impact.openTasks })
      const choice = await confirm.ask({
        title: t('archive.open.title'),
        message,
        choices: [
          { id: 'cancel', label: t('common.cancel') },
          { id: 'archive', label: t('archive.open.confirm'), primary: true },
        ],
      })
      if (choice !== 'archive') return false
    }
    await gateway.archiveTask(id)
    return true
  }

  return { archive }
}
