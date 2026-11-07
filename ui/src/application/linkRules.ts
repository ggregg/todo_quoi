/**
 * Aide à la saisie des liens (affichage seulement : le cœur Rust revérifie avant d'ouvrir).
 * `www.exemple.fr` → `https://www.exemple.fr` ; `nom@exemple.fr` → `mailto:nom@exemple.fr`.
 */
export function normalizeLinkInput(text: string): string {
  const value = text.trim()
  if (value === '') return ''
  if (/^(https?:|mailto:)/i.test(value)) return value
  if (/^[^\s@/]+@[^\s@/]+\.[^\s@/]+$/.test(value)) return `mailto:${value}`
  return `https://${value.replace(/^\/+/, '')}`
}

/** Le lien sera-t-il ouvert (http, https, mailto) ? */
export function isOpenableLink(href: string): boolean {
  return /^(https?:\/\/.+|mailto:.+)$/i.test(href.trim())
}
