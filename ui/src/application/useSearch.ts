/**
 * Recherche (`Mod+F`, spec §5.6) : saisie, résultats, résultat actif, navigation au clavier.
 *
 * La recherche elle-même (syntaxe, accents, ordre, extraits) est faite par le cœur Rust ; ce
 * composable ne fait qu'envoyer la saisie (après une courte pause de frappe) et afficher.
 * Une réponse arrivée après une saisie plus récente est ignorée.
 */
import { computed, ref, shallowRef } from 'vue'
import type { Highlight, SearchHit } from '../domain/task'
import type { TaskGateway } from '../ports/TaskGateway'

/** Pause de frappe avant d'interroger le cœur (ms). */
export const SEARCH_DELAY_MS = 120

export interface SearchHandlers {
  /** `Entrée` ou clic : ouvrir la tâche dans le panneau. */
  open(hit: SearchHit): void
  /** `Mod+Entrée` : révéler la tâche dans l'arbre (et quitter la recherche). */
  reveal(hit: SearchHit): void
}

export function useSearch(gateway: TaskGateway, handlers: SearchHandlers, delay = SEARCH_DELAY_MS) {
  const query = ref('')
  const results = shallowRef<SearchHit[]>([])
  const activeIndex = ref(0)
  const searching = ref(false)
  const active = computed(() => query.value.trim() !== '')
  let timer: ReturnType<typeof setTimeout> | undefined
  let generation = 0

  function setQuery(text: string): void {
    query.value = text
    clearTimeout(timer)
    if (!active.value) return clear()
    timer = setTimeout(() => void run(text), delay)
  }

  async function run(text: string): Promise<void> {
    const current = ++generation
    searching.value = true
    const hits = await gateway.searchTasks(text).catch(() => [])
    if (current !== generation) return
    results.value = hits
    activeIndex.value = 0
    searching.value = false
  }

  function clear(): void {
    generation++
    query.value = ''
    results.value = []
    activeIndex.value = 0
    searching.value = false
  }

  function moveActive(step: number): void {
    const count = results.value.length
    if (count > 0) activeIndex.value = (activeIndex.value + step + count) % count
  }

  function setActive(index: number): void {
    if (index >= 0 && index < results.value.length) activeIndex.value = index
  }

  /** Clavier dans le champ de recherche. Renvoie `true` si la touche a été traitée. */
  function handleKey(event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey'>): boolean {
    const hit = results.value[activeIndex.value]
    switch (event.key) {
      case 'ArrowDown':
        moveActive(1)
        return true
      case 'ArrowUp':
        moveActive(-1)
        return true
      case 'Escape':
        clear()
        return true
      case 'Enter':
        if (!hit) return true
        if (event.metaKey || event.ctrlKey) {
          handlers.reveal(hit)
          clear()
        } else handlers.open(hit)
        return true
      default:
        return false
    }
  }

  return { query, results, activeIndex, searching, active, setQuery, setActive, clear, handleKey }
}

export type Search = ReturnType<typeof useSearch>

/** Découpe `text` en morceaux surlignés ou non (positions **en caractères**, pas en unités UTF-16). */
export function highlightParts(text: string, highlights: Highlight[]): { text: string; hit: boolean }[] {
  const characters = Array.from(text)
  const parts: { text: string; hit: boolean }[] = []
  let position = 0
  for (const { start, end } of highlights) {
    if (start < position || end > characters.length) continue
    if (start > position) parts.push({ text: characters.slice(position, start).join(''), hit: false })
    parts.push({ text: characters.slice(start, end).join(''), hit: true })
    position = end
  }
  if (position < characters.length) parts.push({ text: characters.slice(position).join(''), hit: false })
  return parts
}
