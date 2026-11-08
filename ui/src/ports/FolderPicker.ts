/**
 * Port : choisir — ou créer — un dossier avec la fenêtre **native** du système (Finder,
 * Explorateur), pour attacher une base existante ou en créer une nouvelle (ADR-0008 révisée).
 */
import type { InjectionKey } from 'vue'

export interface FolderPicker {
  /** Dossier choisi (chemin complet), ou `null` si l'utilisateur annule. */
  pickFolder(title: string): Promise<string | null>
}

export const folderPickerKey: InjectionKey<FolderPicker> = Symbol('FolderPicker')

/** Nom proposé pour une base à partir de son dossier : `…/Pro/TodoQuoi` → « Pro ». */
export function suggestedBaseName(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean)
  const last = parts.at(-1) ?? ''
  const name = last.toLowerCase() === 'todoquoi' ? (parts.at(-2) ?? last) : last
  return name.replace(/^[A-Za-z]:$/, '')
}
