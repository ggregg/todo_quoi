/** `FolderPicker` pour le navigateur (`pnpm ui:dev`) : pas d'accès aux dossiers, on demande un chemin. */
import type { FolderPicker } from '../../ports/FolderPicker'

export class BrowserFolderPicker implements FolderPicker {
  async pickFolder(title: string): Promise<string | null> {
    return window.prompt(title, '/Users/demo/Documents/Nouvelle base')
  }
}
