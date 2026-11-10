<script setup lang="ts">
/**
 * Commandes des tableaux, posées **sur** la grille (voir `useNoteTable`) :
 * - au survol d'une cellule, une poignée en haut de sa colonne et à gauche de sa ligne : clic =
 *   sélectionner la colonne / la ligne et ouvrir son menu ;
 * - « + » le long du bord droit (colonne) et du bord bas (ligne) ;
 * - clic droit (ou touche Menu) dans une cellule : toutes les commandes.
 *
 * Le composant écoute la zone de note `container` ; les positions y sont calculées à partir de
 * celles des cellules (rien n'est ajouté au document).
 */
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import type { Editor } from '@tiptap/core'
import { CELL_MENU, COLUMN_MENU, ROW_MENU, type NoteTable, type TableMenuItem } from './useNoteTable'
import { t } from '../../i18n'

const props = defineProps<{ editor: Editor; table: NoteTable; container: HTMLElement }>()

interface Box {
  left: number
  top: number
  width?: number
  height?: number
}

/** Poignées et « + » de la cellule survolée. */
interface Hover {
  cell: HTMLElement
  table: HTMLTableElement
  columnGrip: Box
  rowGrip: Box
  plusRight: Box
  plusBottom: Box
}

const hover = shallowRef<Hover | null>(null)
const menu = shallowRef<{ items: TableMenuItem[]; cell: HTMLElement; at: Box } | null>(null)
const menuElement = ref<HTMLElement | null>(null)
/** Quitter la cellule vers une poignée traverse un petit vide : on attend avant de masquer. */
let hideTimer: ReturnType<typeof setTimeout> | undefined

function relative(rect: DOMRect): DOMRect {
  const origin = props.container.getBoundingClientRect()
  return new DOMRect(rect.left - origin.left, rect.top - origin.top, rect.width, rect.height)
}

function hoverOf(cell: HTMLElement): Hover | null {
  const table = cell.closest('table')
  const row = cell.closest('tr')
  if (!table || !row) return null
  const [box, area, line] = [cell, table, row].map((element) => relative(element.getBoundingClientRect())) as [
    DOMRect,
    DOMRect,
    DOMRect,
  ]
  return {
    cell,
    table,
    columnGrip: { left: box.left + box.width / 2 - 12, top: area.top - 16 },
    rowGrip: { left: area.left - 16, top: line.top + line.height / 2 - 12 },
    plusRight: { left: area.right + 3, top: area.top, height: area.height },
    plusBottom: { left: area.left, top: area.bottom + 3, width: area.width },
  }
}

function onMouseMove(event: MouseEvent): void {
  const target = event.target as HTMLElement
  if (target.closest('.table-controls')) return clearTimeout(hideTimer)
  const cell = target.closest<HTMLElement>('.ProseMirror td, .ProseMirror th')
  if (cell) {
    clearTimeout(hideTimer)
    if (hover.value?.cell !== cell) hover.value = hoverOf(cell)
    return
  }
  clearTimeout(hideTimer)
  hideTimer = setTimeout(() => (hover.value = null), 350)
}

function onMouseLeave(): void {
  clearTimeout(hideTimer)
  hideTimer = setTimeout(() => (hover.value = null), 350)
}

/** Cellule visée par un clic droit, ou celle du curseur (touche Menu du clavier). */
function cellForContextMenu(event: MouseEvent): HTMLElement | null {
  const fromPointer = (event.target as HTMLElement).closest<HTMLElement>('.ProseMirror td, .ProseMirror th')
  if (fromPointer) return fromPointer
  if (!props.editor.isActive('table')) return null
  const { node } = props.editor.view.domAtPos(props.editor.state.selection.from)
  const element = node instanceof HTMLElement ? node : node.parentElement
  return element?.closest<HTMLElement>('td, th') ?? null
}

function onContextMenu(event: MouseEvent): void {
  const cell = cellForContextMenu(event)
  if (!cell) return
  event.preventDefault()
  props.table.placeIn(cell)
  const keyboard = event.clientX === 0 && event.clientY === 0
  const origin = props.container.getBoundingClientRect()
  const at = keyboard
    ? { left: relative(cell.getBoundingClientRect()).left, top: relative(cell.getBoundingClientRect()).bottom }
    : { left: event.clientX - origin.left, top: event.clientY - origin.top }
  void openMenu(CELL_MENU, cell, at)
}

function grip(kind: 'row' | 'column'): void {
  const current = hover.value
  if (!current) return
  props.table.selectWhole(kind, current.cell)
  const box = kind === 'row' ? current.rowGrip : current.columnGrip
  void openMenu(kind === 'row' ? ROW_MENU : COLUMN_MENU, current.cell, { left: box.left, top: box.top + 26 })
}

async function openMenu(items: TableMenuItem[], cell: HTMLElement, at: Box): Promise<void> {
  menu.value = { items, cell, at }
  await nextTick()
  menuElement.value?.querySelector<HTMLButtonElement>('button')?.focus()
}

function closeMenu(): void {
  menu.value = null
  props.editor.view.focus()
}

function choose(item: TableMenuItem): void {
  const current = menu.value
  if (!current) return
  menu.value = null
  hover.value = null
  props.table.run(item.command, current.cell)
}

/** Flèches haut / bas dans le menu, Échap pour le fermer. */
function onMenuKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') return closeMenu()
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
  event.preventDefault()
  const buttons = [...(menuElement.value?.querySelectorAll<HTMLButtonElement>('button') ?? [])]
  const index = buttons.indexOf(document.activeElement as HTMLButtonElement)
  const next = (index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length
  buttons[next]?.focus()
}

function onWindowMouseDown(event: MouseEvent): void {
  if (menu.value && !menuElement.value?.contains(event.target as Node)) menu.value = null
}

function appendColumn(): void {
  if (hover.value) props.table.appendColumn(hover.value.table)
  hover.value = null
}

function appendRow(): void {
  if (hover.value) props.table.appendRow(hover.value.table)
  hover.value = null
}

const px = (box: Box) => ({
  left: `${box.left}px`,
  top: `${box.top}px`,
  ...(box.width === undefined ? {} : { width: `${box.width}px` }),
  ...(box.height === undefined ? {} : { height: `${box.height}px` }),
})

onMounted(() => {
  props.container.addEventListener('mousemove', onMouseMove)
  props.container.addEventListener('mouseleave', onMouseLeave)
  props.container.addEventListener('contextmenu', onContextMenu)
  window.addEventListener('mousedown', onWindowMouseDown)
})
onBeforeUnmount(() => {
  clearTimeout(hideTimer)
  props.container.removeEventListener('mousemove', onMouseMove)
  props.container.removeEventListener('mouseleave', onMouseLeave)
  props.container.removeEventListener('contextmenu', onContextMenu)
  window.removeEventListener('mousedown', onWindowMouseDown)
})
</script>

<template>
  <aside class="table-controls" @mousedown.stop>
    <template v-if="hover && !menu">
      <button
        type="button"
        class="grip column"
        :style="px(hover.columnGrip)"
        :title="t('table.columnMenu')"
        :aria-label="t('table.columnMenu')"
        @mousedown.prevent
        @click="grip('column')"
      >
        ⋯
      </button>
      <button
        type="button"
        class="grip row"
        :style="px(hover.rowGrip)"
        :title="t('table.rowMenu')"
        :aria-label="t('table.rowMenu')"
        @mousedown.prevent
        @click="grip('row')"
      >
        ⋮
      </button>
      <button
        type="button"
        class="plus right"
        :style="px(hover.plusRight)"
        :title="t('table.appendColumn')"
        :aria-label="t('table.appendColumn')"
        @mousedown.prevent
        @click="appendColumn"
      >
        +
      </button>
      <button
        type="button"
        class="plus bottom"
        :style="px(hover.plusBottom)"
        :title="t('table.appendRow')"
        :aria-label="t('table.appendRow')"
        @mousedown.prevent
        @click="appendRow"
      >
        +
      </button>
    </template>
    <div
      v-if="menu"
      ref="menuElement"
      class="menu"
      role="menu"
      :aria-label="t('table.menu')"
      :style="px(menu.at)"
      @keydown="onMenuKeydown"
    >
      <template v-for="item in menu.items" :key="item.command">
        <hr v-if="item.separated" />
        <button type="button" role="menuitem" :class="{ danger: item.danger }" @click="choose(item)">
          {{ item.label }}
        </button>
      </template>
    </div>
  </aside>
</template>

<style scoped>
.table-controls {
  position: absolute;
  inset: 0 auto auto 0;
  width: 0;
  height: 0;
  z-index: 5;
}
button {
  position: absolute;
  font: inherit;
  color: var(--muted);
  background: var(--surface);
  border: 1px solid var(--border);
  cursor: pointer;
  padding: 0;
  line-height: 1;
}
button:hover,
button:focus-visible {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}
.grip {
  border-radius: 4px;
  font-size: 12px;
}
.grip.column {
  width: 24px;
  height: 13px;
}
.grip.row {
  width: 13px;
  height: 24px;
}
.plus {
  border-radius: 4px;
  font-size: 13px;
  opacity: 0.55;
}
.plus:hover {
  opacity: 1;
}
.plus.right {
  width: 14px;
}
.plus.bottom {
  height: 14px;
}
.menu {
  position: absolute;
  min-width: 210px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: var(--shadow);
  font-size: 13px;
}
.menu button {
  position: static;
  text-align: left;
  padding: 5px 10px;
  border: none;
  border-radius: 5px;
  color: var(--text);
  background: none;
}
.menu button:hover,
.menu button:focus-visible {
  background: var(--hover);
  color: var(--text);
  outline: none;
}
.menu button.danger {
  color: var(--danger);
}
.menu hr {
  border: none;
  border-top: 1px solid var(--border);
  margin: 4px 2px;
}
</style>
