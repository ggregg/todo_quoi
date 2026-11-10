/**
 * Boutons « Aligner » et « Couleur du texte » de l'éditeur (ADR-0012).
 *
 * Alignement « là où c'est pertinent » : paragraphes et titres ; dans un tableau, **toute la
 * colonne** (le tableau Markdown aligne par colonne) ; pas dans une liste (le Markdown ne peut
 * pas l'enregistrer : le bouton est alors désactivé).
 */
import type { ShallowRef } from 'vue'
import type { Editor } from '@tiptap/core'
import { CellSelection, cellAround } from '@tiptap/pm/tables'
import type { TextAlignment, TextColor } from './formatting'

export function useNoteFormat(editor: ShallowRef<Editor | undefined>) {
  const inList = () => !!editor.value && (editor.value.isActive('listItem') || editor.value.isActive('taskItem'))

  /** Vrai si le curseur est dans un bloc qu'on peut aligner. */
  function canAlign(): boolean {
    return !!editor.value && !inList()
  }

  /** Alignement du bloc (ou de la cellule) du curseur ; `null` : à gauche par défaut. */
  function currentAlignment(): TextAlignment | null {
    const current = editor.value
    if (!current) return null
    const type = ['tableHeader', 'tableCell', 'heading', 'paragraph'].find((name) => current.isActive(name))
    return type ? ((current.getAttributes(type).textAlign as TextAlignment | null) ?? null) : null
  }

  /** Aligne le bloc du curseur, ou la colonne du tableau ; `null` : revenir à gauche. */
  function align(value: TextAlignment | null): void {
    const current = editor.value
    if (!current || inList()) return
    if (current.isActive('table')) return alignColumn(current, value)
    current
      .chain()
      .focus()
      .updateAttributes('paragraph', { textAlign: value })
      .updateAttributes('heading', { textAlign: value })
      .run()
  }

  function alignColumn(current: Editor, value: TextAlignment | null): void {
    const { state, view } = current
    const selection = state.selection
    const anchor = selection instanceof CellSelection ? selection.$anchorCell : cellAround(selection.$from)
    const head = selection instanceof CellSelection ? selection.$headCell : anchor
    if (!anchor || !head) return
    const tr = state.tr
    CellSelection.colSelection(anchor, head).forEachCell((node, pos) => {
      tr.setNodeMarkup(pos, undefined, { ...node.attrs, textAlign: value })
    })
    view.dispatch(tr)
    view.focus()
  }

  /** Couleur du texte sélectionné (ou à la position du curseur) ; `null` : aucune. */
  function currentColor(): TextColor | null {
    return (editor.value?.getAttributes('textColor').color as TextColor | undefined) ?? null
  }

  /** Colore la sélection ; `null` : couleur par défaut. */
  function color(value: TextColor | null): void {
    const chain = editor.value?.chain().focus()
    if (!chain) return
    if (value === null) chain.unsetMark('textColor').run()
    else chain.setMark('textColor', { color: value }).run()
  }

  return { canAlign, currentAlignment, align, currentColor, color }
}
