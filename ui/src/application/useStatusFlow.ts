/**
 * Clôture / réouverture avec les dialogues des règles R1 à R3 (et R12 après un déplacement).
 * Partagé par la liste (`useTaskTree`) et le panneau de détail (`useTaskDetail`).
 *
 * Les **décisions** viennent du cœur Rust (`proposeClosingParent`, `proposeReopening`…) :
 * ici on ne fait que poser la question et transmettre la réponse.
 */
import { t } from '../i18n'
import type { CloseScope, TaskId } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'
import type { Confirm } from './useConfirm'

export interface StatusFlowHooks {
  /** Titre d'une tâche, pour les messages. */
  titleOf: (id: TaskId) => string
  /** Appelé après chaque modification (recharger l'affichage). */
  afterChange: () => Promise<void>
}

export function useStatusFlow(gateway: TaskGateway, confirm: Confirm, hooks: StatusFlowHooks) {
  /** Clôture `id` (en demandant quoi faire des sous-tâches ouvertes, R2), puis propose R1. */
  async function close(id: TaskId): Promise<void> {
    const impact = await gateway.getCloseImpact(id)
    let scope: CloseScope = 'taskOnly'
    if (impact.openDescendants > 0) {
      const choice = await confirm.ask({
        title: t('close.withChildren.title'),
        message: t('close.withChildren.message', { title: hooks.titleOf(id), count: impact.openDescendants }),
        choices: [
          { id: 'cancel', label: t('common.cancel') },
          { id: 'taskOnly', label: t('close.withChildren.taskOnly') },
          { id: 'withOpenDescendants', label: t('close.withChildren.all'), primary: true },
        ],
      })
      if (choice === 'cancel') return
      scope = choice
    }
    const outcome = await gateway.closeTask(id, scope)
    await hooks.afterChange()
    await offerClosingParent(outcome.proposeClosingParent)
  }

  /** R1 — propose de clôturer le parent, puis le grand-parent… un niveau à la fois. */
  async function offerClosingParent(parentId: TaskId | null): Promise<void> {
    let proposal = parentId
    while (proposal !== null) {
      const title = hooks.titleOf(proposal)
      const choice = await confirm.ask({
        title: t('close.parent.title'),
        message: t('close.parent.message', { title }),
        choices: [
          { id: 'later', label: t('common.later') },
          { id: 'close', label: t('close.parent.confirm'), primary: true },
        ],
      })
      if (choice !== 'close') return
      proposal = (await gateway.closeTask(proposal, 'taskOnly')).proposeClosingParent
      await hooks.afterChange()
    }
  }

  /** Rouvre `id`, puis propose de rouvrir les ancêtres terminés (R3). */
  async function reopen(id: TaskId): Promise<void> {
    const outcome = await gateway.reopenTask(id)
    await hooks.afterChange()
    await offerReopening(outcome.proposeReopening)
  }

  /** R3 / R12 — propose de rouvrir des ancêtres terminés. */
  async function offerReopening(ids: TaskId[]): Promise<void> {
    if (ids.length === 0) return
    const titles = ids.map((id) => `« ${hooks.titleOf(id)} »`).join(', ')
    const choice = await confirm.ask({
      title: t('reopen.ancestors.title'),
      message: t('reopen.ancestors.message', { titles }),
      choices: [
        { id: 'no', label: t('common.no') },
        { id: 'reopen', label: t('reopen.ancestors.confirm'), primary: true },
      ],
    })
    if (choice !== 'reopen') return
    for (const id of ids) await gateway.reopenTask(id)
    await hooks.afterChange()
  }

  /** `Espace` / case à cocher : clôture une tâche ouverte, rouvre une tâche terminée. */
  function toggle(id: TaskId, isDone: boolean): Promise<void> {
    return isDone ? reopen(id) : close(id)
  }

  return { toggle, close, reopen, offerClosingParent, offerReopening }
}

export type StatusFlow = ReturnType<typeof useStatusFlow>
