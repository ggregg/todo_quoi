/**
 * Port : fichiers choisis par l'utilisateur dans le Finder / l'Explorateur — les **joindre** à
 * une note (copier ou déplacer dans la base) ou les **lier** (la note retient le chemin et
 * l'appareil ; le lien est inactif sur les autres appareils).
 *
 * Le glisser-déposer et le collage restent des copies (`MediaGateway`) : la webview ne donne pas
 * le chemin d'un fichier déposé.
 */
import type { InjectionKey } from 'vue'

/** Copier (l'original reste) ou déplacer (l'original est retiré de son dossier). */
export type AttachMode = 'copy' | 'move'

/** Fichier joint : bloc `attachment` de la note. */
export interface AttachedFile {
  src: string
  name: string
  size: number
}

/** Fichier lié : bloc `fileLink` de la note. */
export interface LinkedFile {
  path: string
  name: string
  device: string
}

/**
 * Ce que permet un lien ici : l'ouvrir ; le révéler seulement (dossier, programme) ; rien
 * (créé sur un autre appareil, ou fichier introuvable). Le cœur Rust décide.
 */
export type LinkState = 'available' | 'revealOnly' | 'otherDevice' | 'missing'

export interface FileAttachments {
  /** Fenêtre native de choix de fichiers ; `null` si elle n'existe pas (navigateur). */
  pickFiles(title: string): Promise<string[] | null>
  attach(taskId: string, path: string, mode: AttachMode): Promise<AttachedFile>
  link(path: string): Promise<LinkedFile>
  linkState(link: Pick<LinkedFile, 'path' | 'device'>): Promise<LinkState>
  openLinked(link: Pick<LinkedFile, 'path' | 'device'>): Promise<void>
  revealLinked(link: Pick<LinkedFile, 'path' | 'device'>): Promise<void>
}

export const fileAttachmentsKey: InjectionKey<FileAttachments> = Symbol('FileAttachments')

/** Nom du fichier d'un chemin macOS ou Windows (`C:\docs\a.pdf` → `a.pdf`). */
export function fileNameOf(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path
}
