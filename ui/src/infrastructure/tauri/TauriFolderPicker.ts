/** `FolderPicker` réel : fenêtre de choix de dossier du système (plugin officiel `dialog`). */
import { open } from '@tauri-apps/plugin-dialog'
import type { FolderPicker } from '../../ports/FolderPicker'

export class TauriFolderPicker implements FolderPicker {
  async pickFolder(title: string): Promise<string | null> {
    // `canCreateDirectories` : bouton « Nouveau dossier » (macOS ; toujours présent sous Windows).
    const chosen = await open({ directory: true, multiple: false, canCreateDirectories: true, title })
    return typeof chosen === 'string' ? chosen : null
  }
}
