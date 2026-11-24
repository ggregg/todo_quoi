/**
 * Sélection multiple de la liste (spec §5.5) : `Maj+↑/↓`, `Maj+clic` (plage), `Mod+clic`
 * (ajouter ou retirer une tâche).
 *
 * La tâche **courante** reste `selectedId` (celle du clavier et du panneau de détail). Les tâches
 * **marquées** forment la sélection quand il y en a au moins deux ; sinon, la sélection est la
 * tâche courante seule. Changer la tâche courante autrement que par un geste de sélection
 * (flèche, clic simple, création…) revient à une sélection simple.
 *
 * Fonctions d'affichage seulement : ce que fait une action groupée est décidé par le cœur Rust.
 */
import { computed, ref, watch, type Ref } from 'vue'
import type { TaskId } from '../domain/task'

/** `rowIds` : les tâches affichées, dans l'ordre (une plage suit cet ordre). */
export function useTreeSelection(selectedId: Ref<TaskId | null>, rowIds: () => TaskId[]) {
  const marked = ref(new Set<TaskId>())
  /** Point de départ d'une plage : la dernière tâche choisie par un clic ou une flèche. */
  let anchor: TaskId | null = selectedId.value

  // `sync` : la sélection suit la tâche courante **tout de suite**, avant que l'appelant ne
  // continue (un geste marque d'abord, puis change la tâche courante).
  watch(
    selectedId,
    (id) => {
      if (id !== null && marked.value.has(id)) return
      marked.value = new Set()
      anchor = id
    },
    { flush: 'sync' },
  )

  const isMultiple = computed(() => marked.value.size > 1)
  /** Tâches sélectionnées, dans l'ordre d'affichage. */
  const ids = computed<TaskId[]>(() => {
    if (isMultiple.value) return rowIds().filter((id) => marked.value.has(id))
    return selectedId.value === null ? [] : [selectedId.value]
  })

  function has(id: TaskId): boolean {
    return isMultiple.value ? marked.value.has(id) : id === selectedId.value
  }

  /** Revient à la seule tâche courante. */
  function clear(): void {
    marked.value = new Set()
    anchor = selectedId.value
  }

  /** `Mod+clic` : ajoute `id` à la sélection, ou l'en retire (jamais la dernière). */
  function toggle(id: TaskId): void {
    const next = new Set(isMultiple.value ? marked.value : selectedId.value === null ? [] : [selectedId.value])
    if (next.has(id) && next.size > 1) next.delete(id)
    else next.add(id)
    const current = next.has(id) ? id : [...next].at(-1)
    mark(next, current ?? id)
    anchor = id
  }

  /** `Maj+clic`, `Maj+↑/↓` : de la tâche de départ jusqu'à `id`, dans l'ordre affiché. */
  function extendTo(id: TaskId): void {
    const all = rowIds()
    const from = all.indexOf(anchor ?? id)
    const to = all.indexOf(id)
    if (from < 0 || to < 0) {
      selectedId.value = id
      return
    }
    mark(new Set(all.slice(Math.min(from, to), Math.max(from, to) + 1)), id)
  }

  /** `Maj+↑` (`-1`) / `Maj+↓` (`+1`) depuis la tâche courante. */
  function extendBy(delta: number): void {
    const all = rowIds()
    const next = all[all.indexOf(selectedId.value ?? '') + delta]
    if (next !== undefined) extendTo(next)
  }

  /** Après un rechargement : oublie les tâches qui ne sont plus affichées. */
  function prune(): void {
    const shown = new Set(rowIds())
    const kept = new Set([...marked.value].filter((id) => shown.has(id)))
    if (kept.size !== marked.value.size) marked.value = kept.size > 1 ? kept : new Set()
  }

  /** Marque `next` puis fait de `current` la tâche courante (dans cet ordre : voir le `watch`). */
  function mark(next: Set<TaskId>, current: TaskId): void {
    marked.value = next.size > 1 ? next : new Set()
    selectedId.value = current
  }

  return { ids, isMultiple, has, clear, toggle, extendTo, extendBy, prune }
}

export type TreeSelection = ReturnType<typeof useTreeSelection>
