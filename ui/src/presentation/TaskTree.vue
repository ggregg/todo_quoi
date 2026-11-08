<script setup lang="ts">
/**
 * Liste arborescente **virtualisée** : seules les lignes visibles (plus une marge) sont créées
 * dans le DOM, ce qui garde la liste fluide avec des milliers de tâches. Toutes les lignes ont
 * la même hauteur, ce qui rend le calcul trivial (pas de bibliothèque, ADR-0006).
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { DisplayRow } from '../application/treeView'
import type { BaseInfo } from '../ports/SettingsGateway'
import AppIcon from './AppIcon.vue'
import { isMacPlatform, treeActionFor } from '../application/keyboard'
import type { TaskTree } from '../application/useTaskTree'
import { t } from '../i18n'
import TaskRow from './TaskRow.vue'
import { TASK_DRAG_TYPE } from './dragTypes'
import { defaultView } from '../domain/view'

const ROW_HEIGHT = 30
const OVERSCAN = 8

/** `baseInfo` : nom et couleur d'une base (en-têtes de section). */
const props = defineProps<{ tree: TaskTree; baseInfo: (id: string) => BaseInfo }>()

function acceptTaskOnBase(event: DragEvent): void {
  if (props.tree.draggingId.value !== null && event.dataTransfer?.types.includes(TASK_DRAG_TYPE)) event.preventDefault()
}

const container = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const viewportHeight = ref(600)
const isMac = isMacPlatform(navigator.userAgent)

/** Lignes affichées : tâches et, en mode « Tout », en-têtes de section de base. */
const rows = computed(() => props.tree.displayed.value)
const firstIndex = computed(() => Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - OVERSCAN))
const visible = computed<{ row: DisplayRow; index: number }[]>(() => {
  const count = Math.ceil(viewportHeight.value / ROW_HEIGHT) + OVERSCAN * 2
  return rows.value.slice(firstIndex.value, firstIndex.value + count).map((row, offset) => ({
    row,
    index: firstIndex.value + offset,
  }))
})

// Lâcher sur le fond de la liste : à la fin de la liste affichée (détacher une sous-tâche).
function onListDragOver(event: DragEvent): void {
  if (props.tree.draggingId.value !== null && event.dataTransfer?.types.includes(TASK_DRAG_TYPE)) event.preventDefault()
}

function onListDrop(event: DragEvent): void {
  if (props.tree.draggingId.value === null) return
  event.preventDefault()
  void props.tree.dropAtListEnd(props.tree.zoomId.value)
}

function onKeydown(event: KeyboardEvent): void {
  const action = treeActionFor(event, isMac)
  if (!action) return
  event.preventDefault()
  void props.tree.handle(action)
}

/** Fait défiler juste ce qu'il faut pour que la ligne sélectionnée soit visible. */
watch(
  () => props.tree.selectedId.value,
  async () => {
    await nextTick()
    const element = container.value
    const index = rows.value.findIndex((row) => row.kind === 'task' && row.task.id === props.tree.selectedId.value)
    if (!element || index < 0) return
    const top = index * ROW_HEIGHT
    if (top < element.scrollTop) element.scrollTop = top
    else if (top + ROW_HEIGHT > element.scrollTop + element.clientHeight) {
      element.scrollTop = top + ROW_HEIGHT - element.clientHeight
    }
  },
)

const resizeObserver = new ResizeObserver(([entry]) => {
  if (entry) viewportHeight.value = entry.contentRect.height
})
onMounted(() => {
  if (container.value) resizeObserver.observe(container.value)
  container.value?.focus()
})
onBeforeUnmount(() => resizeObserver.disconnect())

/** Après un renommage, le focus revient à la liste pour continuer au clavier. */
function refocus(): void {
  void nextTick(() => container.value?.focus())
}
</script>

<template>
  <div
    ref="container"
    class="tree"
    role="tree"
    tabindex="0"
    :aria-label="t('tree.label')"
    @keydown="onKeydown"
    @scroll="scrollTop = ($event.target as HTMLElement).scrollTop"
    @dragover="onListDragOver"
    @drop="onListDrop"
  >
    <div :style="{ height: `${rows.length * ROW_HEIGHT}px`, position: 'relative' }">
      <div :style="{ transform: `translateY(${firstIndex * ROW_HEIGHT}px)` }">
        <template v-for="{ row } in visible" :key="row.kind === 'task' ? row.task.id : `base:${row.base}`">
          <div
            v-if="row.kind === 'base'"
            class="base-row"
            :class="{ selected: tree.selectedBase.value === row.base }"
            :style="{ '--base-color': baseInfo(row.base).color }"
            @mousedown="tree.selectBase(row.base)"
            @dragover="acceptTaskOnBase"
            @drop.prevent.stop="tree.dropOnBase(row.base)"
          >
            <button
              type="button"
              class="chevron"
              :aria-expanded="row.expanded"
              @click="tree.toggleBaseSection(row.base)"
            >
              <AppIcon :name="row.expanded ? 'chevronDown' : 'chevronRight'" :size="12" />
            </button>
            <span class="dot" />
            <span class="base-name">{{ baseInfo(row.base).name }}</span>
            <span class="base-count">{{ row.count }}</span>
            <button type="button" class="base-add" :title="t('tree.addTask')" @click="tree.addAtEnd(row.base)">
              <AppIcon name="plus" :size="12" />
            </button>
          </div>
          <TaskRow
            v-else
            :row="row"
            :selected="row.task.id === tree.selectedId.value"
            :renaming="row.task.id === tree.renamingId.value"
            :columns="tree.currentLayout.value?.view.columns ?? defaultView().columns"
            :can-drop="(zone) => tree.canDropOn(row.task.id, zone)"
            @select="tree.select(row.task.id)"
            @toggle-expanded="tree.toggleExpanded(row.task.id)"
            @toggle-done="tree.toggleDone(row.task.id)"
            @dblclick="$event.shiftKey ? tree.openWindow(row.task.id) : tree.openPanel(row.task.id)"
            @commit-rename="(title) => (tree.commitRename(row.task.id, title), refocus())"
            @cancel-rename="(tree.cancelRename(), refocus())"
            @drag-start="tree.startDrag(row.task.id)"
            @drag-end="tree.endDrag()"
            @drop-here="(zone) => tree.dropOn(row.task.id, zone)"
          />
        </template>
      </div>
    </div>
    <p v-if="tree.loaded.value && rows.length === 0" class="empty">
      {{ t('tree.empty', { shortcut: isMac ? '⌘N' : 'Ctrl+N' }) }}
    </p>
  </div>
</template>

<style scoped>
.tree {
  flex: 1;
  overflow: auto;
  outline: none;
  padding: 4px 0;
}
.tree:not(:focus-within) :deep(.row.selected) {
  border-left-color: var(--muted);
}
.empty {
  padding: 30px;
  text-align: center;
  color: var(--muted);
}
.base-row {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 30px;
  padding: 0 12px 0 6px;
  font-weight: 600;
  background: color-mix(in srgb, var(--base-color) 10%, var(--surface));
  border-left: 3px solid var(--base-color);
}
.base-row.selected {
  outline: 2px solid var(--base-color);
  outline-offset: -2px;
}
.base-row .chevron,
.base-row .base-add {
  display: inline-grid;
  place-items: center;
  padding: 2px;
  background: none;
  border: none;
  color: var(--muted);
}
.base-row .dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--base-color);
}
.base-row .base-name {
  flex: 1;
}
.base-row .base-count {
  font-size: 11px;
  font-weight: 400;
  color: var(--muted);
}
</style>
