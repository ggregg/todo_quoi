import { describe, expect, it } from 'vitest'
import { treeActionFor, type KeyPress } from './keyboard'

const press = (key: string, modifiers: Partial<KeyPress> = {}): KeyPress => ({
  key,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...modifiers,
})

describe('raccourcis clavier (spec §5.5)', () => {
  it('navigation et actions simples', () => {
    expect(treeActionFor(press('ArrowDown'), true)).toBe('selectNext')
    expect(treeActionFor(press(' '), true)).toBe('toggleDone')
    expect(treeActionFor(press('F2'), false)).toBe('rename')
    expect(treeActionFor(press('Tab'), false)).toBe('indent')
    expect(treeActionFor(press('Tab', { shiftKey: true }), false)).toBe('outdent')
  })

  it('Mod = Cmd sur Mac, Ctrl ailleurs', () => {
    expect(treeActionFor(press('n', { metaKey: true }), true)).toBe('newSibling')
    expect(treeActionFor(press('N', { metaKey: true, shiftKey: true }), true)).toBe('newChild')
    expect(treeActionFor(press('n', { ctrlKey: true }), true)).toBeNull()
    expect(treeActionFor(press('n', { ctrlKey: true }), false)).toBe('newSibling')
    expect(treeActionFor(press('Enter', { ctrlKey: true }), false)).toBe('zoomIn')
  })

  it('Alt+flèches déplace, Alt+← remonte du zoom', () => {
    expect(treeActionFor(press('ArrowUp', { altKey: true }), false)).toBe('moveUp')
    expect(treeActionFor(press('ArrowLeft', { altKey: true }), false)).toBe('zoomOut')
  })

  it('Retour arrière supprime sur Mac seulement', () => {
    expect(treeActionFor(press('Backspace'), true)).toBe('delete')
    expect(treeActionFor(press('Backspace'), false)).toBeNull()
    expect(treeActionFor(press('Delete'), false)).toBe('delete')
  })

  it('panneau de détail et fenêtres de tâche (spec §5.4bis)', () => {
    expect(treeActionFor(press('Enter'), false)).toBe('open')
    expect(treeActionFor(press('Enter', { shiftKey: true }), false)).toBe('openWindow')
    expect(treeActionFor(press('D', { metaKey: true, shiftKey: true }), true)).toBe('openWindow')
    expect(treeActionFor(press('Escape'), false)).toBe('closePanel')
  })

  it('les touches sans raccourci ne font rien', () => {
    expect(treeActionFor(press('a'), false)).toBeNull()
  })
})
