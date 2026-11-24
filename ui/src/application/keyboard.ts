/**
 * Raccourcis clavier de la liste (spécification §5.5). Fonction **pure** : une touche → une
 * action, sans toucher au DOM ni à l'état — donc testable en une ligne.
 *
 * `Mod` = `Cmd` sous macOS, `Ctrl` ailleurs.
 */
export type TreeAction =
  | 'selectPrevious'
  | 'selectNext'
  | 'selectFirst'
  | 'selectLast'
  | 'pageUp'
  | 'pageDown'
  | 'expandOrFirstChild'
  | 'collapseOrParent'
  | 'open'
  | 'openWindow'
  | 'closePanel'
  | 'zoomIn'
  | 'zoomOut'
  | 'toggleDone'
  | 'newSibling'
  | 'newChild'
  | 'rename'
  | 'indent'
  | 'outdent'
  | 'moveUp'
  | 'moveDown'
  | 'delete'
  | 'undo'
  /** `Maj+↑` / `Maj+↓` : étendre la sélection (spec §5.5). */
  | 'extendPrevious'
  | 'extendNext'
  /** `Mod+Maj+A` : archiver la sélection, ou la restaurer si elle n'est faite que d'archives. */
  | 'archive'

/** Ce qu'on lit d'un `KeyboardEvent` (sous-ensemble, pour pouvoir tester sans navigateur). */
export interface KeyPress {
  key: string
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
  shiftKey: boolean
}

export function isMacPlatform(userAgent: string): boolean {
  return /Mac|iPhone|iPad/.test(userAgent)
}

export function treeActionFor(press: KeyPress, isMac: boolean): TreeAction | null {
  const mod = isMac ? press.metaKey : press.ctrlKey
  const { key, altKey: alt, shiftKey: shift } = press

  if (mod && key.toLowerCase() === 'n') return shift ? 'newChild' : 'newSibling'
  if (mod && shift && key.toLowerCase() === 'd') return 'openWindow'
  if (mod && shift && key.toLowerCase() === 'a') return 'archive'
  if (mod && key.toLowerCase() === 'z' && !shift) return 'undo'
  if (mod && key === 'Enter') return 'zoomIn'
  if (mod && key === 'Backspace') return 'zoomOut'
  if (mod) return null

  if (alt && key === 'ArrowUp') return 'moveUp'
  if (alt && key === 'ArrowDown') return 'moveDown'
  if (alt && key === 'ArrowLeft') return 'zoomOut'
  if (alt) return null

  switch (key) {
    case 'ArrowUp':
      return shift ? 'extendPrevious' : 'selectPrevious'
    case 'ArrowDown':
      return shift ? 'extendNext' : 'selectNext'
    case 'Home':
      return 'selectFirst'
    case 'End':
      return 'selectLast'
    case 'PageUp':
      return 'pageUp'
    case 'PageDown':
      return 'pageDown'
    case 'ArrowRight':
      return 'expandOrFirstChild'
    case 'ArrowLeft':
      return 'collapseOrParent'
    case 'Enter':
      return shift ? 'openWindow' : 'open'
    case 'Escape':
      return 'closePanel'
    case ' ':
      return 'toggleDone'
    case 'F2':
      return 'rename'
    case 'Tab':
      return shift ? 'outdent' : 'indent'
    case 'Delete':
      return 'delete'
    case 'Backspace':
      // Sur Mac, la touche « effacer » est la plus naturelle pour supprimer.
      return isMac ? 'delete' : null
    default:
      return null
  }
}

/** Clic sur une ligne : sélection simple, ajout ou retrait (`Mod+clic`), plage (`Maj+clic`). */
export type SelectGesture = 'single' | 'toggle' | 'range'

export function selectGestureFor(
  press: Pick<KeyPress, 'ctrlKey' | 'metaKey' | 'shiftKey'>,
  isMac: boolean,
): SelectGesture {
  if (isMac ? press.metaKey : press.ctrlKey) return 'toggle'
  return press.shiftKey ? 'range' : 'single'
}
