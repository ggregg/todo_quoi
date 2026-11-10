/**
 * Tableaux de la note : les commandes derrière les poignées, les boutons « + » et le menu du
 * clic droit (`TableControls.vue`).
 *
 * - Poignée de colonne / de ligne : sélectionne la colonne / la ligne, puis ouvre son menu
 *   (insérer avant / après, supprimer) ;
 * - « + » au bord droit / bas : ajoute une colonne / une ligne à la fin ;
 * - clic droit dans une cellule : toutes les commandes.
 *
 * Sélectionner plusieurs cellules : glisser d'une cellule à l'autre, ou Maj+clic. Une sélection
 * de cellules se vide avec Suppr. Tab / Maj+Tab passent d'une cellule à l'autre (Tab dans la
 * dernière ajoute une ligne).
 *
 * Pas de fusion de cellules : la note est enregistrée en tableau Markdown (GFM), qui ne sait pas
 * la représenter.
 */
import type { ShallowRef } from 'vue'
import type { Editor } from '@tiptap/core'
import { TextSelection } from '@tiptap/pm/state'
import { CellSelection, cellAround } from '@tiptap/pm/tables'
import type { ResolvedPos } from '@tiptap/pm/model'
import { t } from '../../i18n'

export type TableCommand =
  | 'rowAbove'
  | 'rowBelow'
  | 'rowDelete'
  | 'rowSelect'
  | 'columnLeft'
  | 'columnRight'
  | 'columnDelete'
  | 'columnSelect'
  | 'tableDelete'

/** Une entrée de menu ; `separated` : trait au-dessus. */
export interface TableMenuItem {
  command: TableCommand
  label: string
  danger?: boolean
  separated?: boolean
}

export const ROW_MENU: TableMenuItem[] = [
  { command: 'rowAbove', label: t('table.rowAbove') },
  { command: 'rowBelow', label: t('table.rowBelow') },
  { command: 'rowDelete', label: t('table.rowDelete'), danger: true, separated: true },
]

export const COLUMN_MENU: TableMenuItem[] = [
  { command: 'columnLeft', label: t('table.columnLeft') },
  { command: 'columnRight', label: t('table.columnRight') },
  { command: 'columnDelete', label: t('table.columnDelete'), danger: true, separated: true },
]

/** Menu du clic droit dans une cellule. */
export const CELL_MENU: TableMenuItem[] = [
  { command: 'rowAbove', label: t('table.rowAbove') },
  { command: 'rowBelow', label: t('table.rowBelow') },
  { command: 'columnLeft', label: t('table.columnLeft'), separated: true },
  { command: 'columnRight', label: t('table.columnRight') },
  { command: 'rowSelect', label: t('table.rowSelect'), separated: true },
  { command: 'columnSelect', label: t('table.columnSelect') },
  { command: 'rowDelete', label: t('table.rowDelete'), danger: true, separated: true },
  { command: 'columnDelete', label: t('table.columnDelete'), danger: true },
  { command: 'tableDelete', label: t('table.tableDelete'), danger: true },
]

export function useNoteTable(editor: ShallowRef<Editor | undefined>) {
  const chain = () => editor.value!.chain().focus()

  /** Position de la cellule `cell` (élément `td` / `th` de l'éditeur) dans le document. */
  function cellOf(cell: HTMLElement): ResolvedPos | null {
    const current = editor.value
    if (!current) return null
    const inside = current.view.posAtDOM(cell, 0)
    return cellAround(current.state.doc.resolve(inside))
  }

  /** Curseur dans `cell` — sauf si elle fait déjà partie d'une sélection de cellules, gardée. */
  function placeIn(cell: HTMLElement): void {
    const selection = editor.value?.state.selection
    if (selection instanceof CellSelection && cell.classList.contains('selectedCell')) return
    moveCursorInto(cell)
  }

  function moveCursorInto(cell: HTMLElement): void {
    const current = editor.value
    const $cell = cellOf(cell)
    if (!current || !$cell) return
    const { state, view } = current
    view.dispatch(state.tr.setSelection(TextSelection.near(state.doc.resolve($cell.pos + 1))))
  }

  /** Sélectionne la ligne ou la colonne entière de `cell`. */
  function selectWhole(kind: 'row' | 'column', cell: HTMLElement): void {
    const current = editor.value
    const $cell = cellOf(cell)
    if (!current || !$cell) return
    const whole = kind === 'row' ? CellSelection.rowSelection($cell) : CellSelection.colSelection($cell)
    current.view.dispatch(current.state.tr.setSelection(whole))
    current.view.focus()
  }

  /** Exécute `command` sur la sélection (cellule du curseur, ou cellules sélectionnées). */
  function run(command: TableCommand, cell: HTMLElement): void {
    const actions: Record<TableCommand, () => unknown> = {
      rowAbove: () => chain().addRowBefore().run(),
      rowBelow: () => chain().addRowAfter().run(),
      rowDelete: () => chain().deleteRow().run(),
      rowSelect: () => selectWhole('row', cell),
      columnLeft: () => chain().addColumnBefore().run(),
      columnRight: () => chain().addColumnAfter().run(),
      columnDelete: () => chain().deleteColumn().run(),
      columnSelect: () => selectWhole('column', cell),
      tableDelete: () => chain().deleteTable().run(),
    }
    actions[command]()
  }

  /** « + » du bord droit : une colonne après la dernière. */
  function appendColumn(table: HTMLTableElement): void {
    const lastCell = table.rows[0]?.cells[table.rows[0].cells.length - 1]
    if (!lastCell) return
    moveCursorInto(lastCell)
    chain().addColumnAfter().run()
  }

  /** « + » du bord bas : une ligne après la dernière. */
  function appendRow(table: HTMLTableElement): void {
    const lastCell = table.rows[table.rows.length - 1]?.cells[0]
    if (!lastCell) return
    moveCursorInto(lastCell)
    chain().addRowAfter().run()
  }

  return { placeIn, selectWhole, run, appendColumn, appendRow }
}

export type NoteTable = ReturnType<typeof useNoteTable>
