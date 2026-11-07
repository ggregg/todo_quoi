/**
 * Suggestions d'étiquettes à la saisie : favoriser la **réutilisation** des étiquettes existantes
 * (une seule orthographe pour « client-x »), les plus utilisées d'abord.
 */
import type { Task } from '../domain/task'

/** Nombre de tâches qui portent chaque étiquette. */
export function tagUsage(tasks: readonly Task[]): Map<string, number> {
  const usage = new Map<string, number>()
  for (const task of tasks) for (const tag of task.tags) usage.set(tag, (usage.get(tag) ?? 0) + 1)
  return usage
}

/**
 * Étiquettes proposées pour la saisie `typed` : celles qui **commencent** par le texte, puis
 * celles qui le **contiennent** ; dans chaque groupe, les plus utilisées d'abord. Les étiquettes
 * déjà posées sont exclues. Saisie vide : les plus utilisées.
 */
export function suggestTags(
  usage: ReadonlyMap<string, number>,
  typed: string,
  exclude: readonly string[],
  limit = 8,
): string[] {
  const query = typed.trim().replace(/^#/, '').toLowerCase()
  const rank = (tag: string) => (query === '' || tag.startsWith(query) ? 0 : 1)
  return [...usage.entries()]
    .filter(([tag]) => !exclude.includes(tag) && tag.includes(query))
    .sort(([a, countA], [b, countB]) => rank(a) - rank(b) || countB - countA || a.localeCompare(b))
    .slice(0, limit)
    .map(([tag]) => tag)
}
