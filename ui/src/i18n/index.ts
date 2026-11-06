import { fr, type MessageKey } from './fr'

/**
 * Texte traduit, avec remplacement des paramètres : `t('status.count', { total: 3, open: 1 })`.
 */
export function t(key: MessageKey, params: Record<string, string | number> = {}): string {
  return fr[key].replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match))
}
