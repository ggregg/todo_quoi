import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import type { TaskId } from '../domain/task'
import { useTreeSelection } from './treeSelection'

/** Cinq lignes affichées `a`…`e`, la tâche courante sur `start`. */
function setup(start: TaskId = 'b') {
  const shown = ref<TaskId[]>(['a', 'b', 'c', 'd', 'e'])
  const current = ref<TaskId | null>(start)
  const selection = useTreeSelection(current, () => shown.value)
  return { shown, current, selection }
}

describe('sélection multiple de la liste (spec §5.5)', () => {
  it('sans geste, la sélection est la tâche courante seule', () => {
    const { selection } = setup()
    expect(selection.ids.value).toEqual(['b'])
    expect(selection.isMultiple.value).toBe(false)
    expect(selection.has('b')).toBe(true)
  })

  it('Maj+↓ / Maj+↑ étendent puis réduisent la plage depuis la tâche de départ', () => {
    const { current, selection } = setup()
    selection.extendBy(1)
    selection.extendBy(1)
    expect(selection.ids.value).toEqual(['b', 'c', 'd'])
    expect(current.value).toBe('d')
    selection.extendBy(-1)
    selection.extendBy(-1)
    selection.extendBy(-1)
    expect(selection.ids.value).toEqual(['a', 'b'])
  })

  it('Maj+clic : de la tâche de départ jusqu’à la ligne cliquée, dans l’ordre affiché', () => {
    const { selection } = setup('d')
    selection.extendTo('a')
    expect(selection.ids.value).toEqual(['a', 'b', 'c', 'd'])
  })

  it('Mod+clic ajoute ou retire une tâche, jamais la dernière', () => {
    const { current, selection } = setup()
    selection.toggle('d')
    expect(selection.ids.value).toEqual(['b', 'd'])
    expect(current.value).toBe('d')
    selection.toggle('d')
    expect(selection.ids.value).toEqual(['b'])
    selection.toggle('b')
    expect(selection.ids.value).toEqual(['b'])
  })

  it('Maj+clic après Mod+clic part de la dernière tâche cliquée', () => {
    const { selection } = setup('a')
    selection.toggle('c')
    selection.extendTo('e')
    expect(selection.ids.value).toEqual(['c', 'd', 'e'])
  })

  it('changer la tâche courante autrement (flèche, clic simple) revient à une sélection simple', () => {
    const { current, selection } = setup()
    selection.extendBy(1)
    current.value = 'e'
    expect(selection.isMultiple.value).toBe(false)
    expect(selection.ids.value).toEqual(['e'])
  })

  it('après un rechargement, oublie les tâches qui ne sont plus affichées', () => {
    const { shown, selection } = setup()
    selection.extendTo('d')
    shown.value = ['a', 'b', 'd', 'e']
    selection.prune()
    expect(selection.ids.value).toEqual(['b', 'd'])
    shown.value = ['a', 'd', 'e']
    selection.prune()
    expect(selection.isMultiple.value).toBe(false)
  })
})
