/**
 * Traduction de l'interface (spec §2 : interface en français, **i18n prête**).
 *
 * - Un dictionnaire typé par langue (`fr.ts`) : une clé absente ou mal orthographiée est une
 *   erreur de compilation (ADR-0006). Ajouter une langue = un fichier `xx.ts` de type `Messages`
 *   (mêmes clés, vérifié par le compilateur), une entrée dans `LOCALES` et dans `DICTIONARIES`.
 * - Dates et nombres : `Intl`, avec la langue de l'interface (pas celle du système), pour qu'un
 *   texte français n'affiche pas une date anglaise.
 * - Messages d'erreur du cœur Rust : en français aussi ; le `code` de `UiError` est stable,
 *   c'est lui qu'une autre langue traduirait.
 */
import { fr, type MessageKey } from './fr'

/** Un dictionnaire complet : toutes les clés du français. */
export type Messages = Record<MessageKey, string>

/** Langues de l'interface. */
export const LOCALES = [{ code: 'fr', name: 'Français' }] as const
export type Locale = (typeof LOCALES)[number]['code']

const DICTIONARIES: Record<Locale, Messages> = { fr }

/** Langue en cours (une seule pour l'instant : le français). */
let locale: Locale = 'fr'

/** Langue de l'interface, pour `Intl` et les attributs `lang`. */
export function currentLocale(): Locale {
  return locale
}

/** Change la langue (prêt pour le jour où il y en aura plusieurs). */
export function setLocale(next: Locale): void {
  locale = next
}

/**
 * Texte traduit, avec remplacement des paramètres : `t('status.count', { total: 3, open: 1 })`.
 */
export function t(key: MessageKey, params: Record<string, string | number> = {}): string {
  const text = DICTIONARIES[locale][key]
  return text.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match))
}

/** Date et heure (`9 nov. 2026, 14:05` en français) ; `style` : précision de la date. */
export function formatDateTime(iso: string, style: 'short' | 'medium' = 'medium'): string {
  return new Date(iso).toLocaleString(locale, { dateStyle: style, timeStyle: 'short' })
}

/** Date seule (`09/11/2026` en français). */
export function formatDay(iso: string): string {
  return new Date(iso).toLocaleDateString(locale)
}
