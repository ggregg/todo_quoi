/**
 * Thème de l'interface (spec §2, §5.7) : **système** (celui de l'OS, par défaut), clair ou
 * sombre. Réglage de l'appareil, gardé dans le stockage local de la webview (commun à toutes
 * les fenêtres de l'app) ; une fenêtre suit le choix fait dans une autre (événement `storage`).
 *
 * Le thème se pose sur `<html data-theme="…">` (rien pour « système ») ; les couleurs sont dans
 * `styles.css` (`light-dark()`). `native` aligne aussi la barre de titre de la fenêtre.
 */
import { ref, type InjectionKey } from 'vue'

export type Theme = 'system' | 'light' | 'dark'
export const THEMES: readonly Theme[] = ['system', 'light', 'dark']
export const THEME_STORAGE_KEY = 'todoquoi.theme'

type ThemeStorage = Pick<Storage, 'getItem' | 'setItem'>

export interface ThemeOptions {
  storage?: ThemeStorage
  root?: HTMLElement
  /** Barre de titre de la fenêtre (Tauri) ; `null` = celle de l'OS. */
  native?: (theme: 'light' | 'dark' | null) => void
}

/** Thème enregistré ; inconnu ou absent : « système ». */
export function readTheme(storage?: ThemeStorage): Theme {
  try {
    const saved = storage?.getItem(THEME_STORAGE_KEY)
    return THEMES.find((theme) => theme === saved) ?? 'system'
  } catch {
    return 'system'
  }
}

/** Pose le thème sur la page (appelé aussi avant le montage, pour éviter un éclair de l'autre thème). */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  if (theme === 'system') delete root.dataset.theme
  else root.dataset.theme = theme
}

export function useTheme(options: ThemeOptions = {}) {
  const theme = ref<Theme>(readTheme(options.storage))

  function show(next: Theme): void {
    theme.value = next
    applyTheme(next, options.root)
    options.native?.(next === 'system' ? null : next)
  }

  /** Choisit le thème et l'enregistre pour l'appareil. */
  function choose(next: Theme): void {
    try {
      options.storage?.setItem(THEME_STORAGE_KEY, next)
    } catch {
      // Stockage indisponible : le choix vaut pour cette session seulement.
    }
    show(next)
  }

  /** Suit les changements faits dans une autre fenêtre ; renvoie de quoi arrêter. */
  function start(target: Pick<Window, 'addEventListener' | 'removeEventListener'> = window): () => void {
    show(theme.value)
    const onStorage = (event: StorageEvent) => {
      if (event.key === THEME_STORAGE_KEY) show(readTheme(options.storage))
    }
    target.addEventListener('storage', onStorage)
    return () => target.removeEventListener('storage', onStorage)
  }

  return { theme, choose, start }
}

export type ThemeState = ReturnType<typeof useTheme>

export const themeKey: InjectionKey<ThemeState> = Symbol('Theme')
