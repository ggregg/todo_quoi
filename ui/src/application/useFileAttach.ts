/**
 * « Joindre un fichier » (bouton de l'éditeur) : choisir des fichiers dans la fenêtre native,
 * puis **copier**, **déplacer** ou **lier** — un seul choix pour tous les fichiers choisis.
 * Renvoie les blocs à insérer dans la note ; aucune règle ici (le cœur Rust vérifie tout).
 */
import type { JSONContent } from '@tiptap/core'
import { fileNameOf, type FileAttachments } from '../ports/FileAttachments'
import type { Confirm } from './useConfirm'
import { isUiError } from '../domain/task'
import { t } from '../i18n'

export type AttachChoice = 'copy' | 'move' | 'link'

/** Résultat : blocs à insérer, ou `null` si la fenêtre native n'existe pas (navigateur). */
export interface AttachResult {
  nodes: JSONContent[]
  failures: string[]
}

export function useFileAttach(files: FileAttachments, confirm: Confirm, taskId: () => string) {
  async function chooseMode(paths: string[]): Promise<AttachChoice | null> {
    const subject = paths.length === 1 ? `« ${fileNameOf(paths[0]!)} »` : t('attach.several', { count: paths.length })
    const choice = await confirm.ask<AttachChoice>({
      title: t('attach.title', { subject }),
      message: t('attach.message'),
      choices: [
        { id: 'link', label: t('attach.link') },
        { id: 'move', label: t('attach.move') },
        { id: 'copy', label: t('attach.copy'), primary: true },
      ],
    })
    return choice === 'cancel' ? null : choice
  }

  async function toNode(path: string, mode: AttachChoice): Promise<JSONContent> {
    if (mode === 'link') {
      const link = await files.link(path)
      return { type: 'fileLink', attrs: { ...link } }
    }
    const attached = await files.attach(taskId(), path, mode)
    return { type: 'attachment', attrs: { ...attached } }
  }

  /** Fenêtre native → choix → blocs. `null` : pas de fenêtre native, utiliser le champ fichier. */
  async function pickAndAttach(): Promise<AttachResult | null> {
    const paths = await files.pickFiles(t('editor.attach'))
    if (paths === null) return null
    if (paths.length === 0) return { nodes: [], failures: [] }
    const mode = await chooseMode(paths)
    if (mode === null) return { nodes: [], failures: [] }
    const result: AttachResult = { nodes: [], failures: [] }
    for (const path of paths) {
      try {
        result.nodes.push(await toNode(path, mode))
      } catch (error) {
        result.failures.push(`${fileNameOf(path)} : ${isUiError(error) ? error.message : String(error)}`)
      }
    }
    return result
  }

  return { pickAndAttach }
}
