/**
 * Faux `FileAttachments` (navigateur, tests) : pas de fenêtre native (`pickFiles` → `null`, le
 * bouton « Joindre » retombe sur le champ fichier du navigateur, en copie). Les fichiers
 * « présents » et l'appareil courant sont réglables par les tests.
 */
import {
  fileNameOf,
  type AttachMode,
  type AttachedFile,
  type FileAttachments,
  type LinkState,
  type LinkedFile,
} from '../../ports/FileAttachments'

type LinkTarget = Pick<LinkedFile, 'path' | 'device'>

export class InMemoryFileAttachments implements FileAttachments {
  /** Chemins présents sur « cette machine ». */
  readonly present = new Set<string>()
  /** Choix simulé de la fenêtre native (`null` : pas de fenêtre native). */
  picked: string[] | null = null
  readonly moved: string[] = []
  readonly opened: string[] = []

  constructor(readonly device = 'navigateur') {}

  async pickFiles(): Promise<string[] | null> {
    return this.picked
  }
  async attach(_taskId: string, path: string, mode: AttachMode): Promise<AttachedFile> {
    if (!this.present.has(path)) throw { code: 'storage', message: `introuvable : ${path}` }
    if (mode === 'move') {
      this.present.delete(path)
      this.moved.push(path)
    }
    return { src: `../media/${'0'.repeat(64)}.bin`, name: fileNameOf(path), size: 1 }
  }
  async link(path: string): Promise<LinkedFile> {
    if (!this.present.has(path)) throw { code: 'storage', message: `introuvable : ${path}` }
    return { path, name: fileNameOf(path), device: this.device }
  }
  async linkState(link: LinkTarget): Promise<LinkState> {
    if (link.device !== this.device) return 'otherDevice'
    return this.present.has(link.path) ? 'available' : 'missing'
  }
  async openLinked(link: LinkTarget): Promise<void> {
    if ((await this.linkState(link)) !== 'available') throw { code: 'invalid-input', message: 'lien inactif' }
    this.opened.push(link.path)
  }
  async revealLinked(link: LinkTarget): Promise<void> {
    await this.openLinked(link)
  }
}
