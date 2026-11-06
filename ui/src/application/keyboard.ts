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
      return 'selectPrevious'
    case 'ArrowDown':
      return 'selectNext'
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
      return 'open'
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
