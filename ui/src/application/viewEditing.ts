/**
 * Modifications d'une vue dans l'éditeur (fonctions pures : une vue → une nouvelle vue).
 * Le cœur Rust revalide et normalise tout ce qui est enregistré (doublons, étiquettes).
 */
import { ALL_COLUMNS, type Column, type ListView, type SortKey, type StatusChoice } from '../domain/view'

/** Affiche ou masque une colonne ; l'ordre reste celui de `ALL_COLUMNS`. */
export function toggleColumn(view: ListView, column: Column): ListView {
  const shown = new Set(view.columns)
  if (shown.has(column)) shown.delete(column)
  else shown.add(column)
  return { ...view, columns: ALL_COLUMNS.filter((candidate) => shown.has(candidate)) }
}

/** Remplace le critère de tri n° `position` (0 ou 1) ; `null` le retire (et ceux d'après). */
export function setSortKey(view: ListView, position: number, key: SortKey | null): ListView {
  const sort = view.sort.slice(0, position)
  if (key) sort.push(key, ...view.sort.slice(position + 1).filter((other) => other.field !== key.field))
  return { ...view, sort: sort.length > 0 ? sort : [{ field: 'manual', direction: 'asc' }] }
}

export function setStatus(view: ListView, status: StatusChoice): ListView {
  return { ...view, filter: { ...view.filter, status } }
}

/** Étiquettes saisies séparées par des espaces ou des virgules, `#` facultatif. */
export function parseTags(text: string): string[] {
  const tags = text
    .split(/[\s,]+/)
    .map((tag) => tag.replace(/^#/, '').toLowerCase())
    .filter(Boolean)
  return [...new Set(tags)]
}

export function setTags(view: ListView, which: 'tagsIn' | 'tagsOut', text: string): ListView {
  return { ...view, filter: { ...view.filter, [which]: parseTags(text) } }
}

export function toggleWithMedia(view: ListView): ListView {
  const media = view.filter.media === 'any' ? 'withMedia' : 'any'
  return { ...view, filter: { ...view.filter, media } }
}
