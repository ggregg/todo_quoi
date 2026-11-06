<script setup lang="ts">
/**
 * Liste arborescente **virtualisée** : seules les lignes visibles (plus une marge) sont créées
 * dans le DOM, ce qui garde la liste fluide avec des milliers de tâches. Toutes les lignes ont
 * la même hauteur, ce qui rend le calcul trivial (pas de bibliothèque, ADR-0006).
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { Row } from '../application/treeView'
import { isMacPlatform, treeActionFor } from '../application/keyboard'
import type { TaskTree } from '../application/useTaskTree'
import { t } from '../i18n'
import TaskRow from './TaskRow.vue'

const ROW_HEIGHT = 30
const OVERSCAN = 8

const props = defineProps<{ tree: TaskTree }>()

const container = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const viewportHeight = ref(600)
const isMac = isMacPlatform(navigator.userAgent)

const rows = computed(() => props.tree.rows.value)
const firstIndex = computed(() => Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - OVERSCAN))
const visible = computed<{ row: Row; index: number }[]>(() => {
  const count = Math.ceil(viewportHeight.value / ROW_HEIGHT) + OVERSCAN * 2
  return rows.value.slice(firstIndex.value, firstIndex.value + count).map((row, offset) => ({
    row,
    index: firstIndex.value + offset,
  }))
})

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
    const index = rows.value.findIndex((row) => row.task.id === props.tree.selectedId.value)
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
  >
    <div :style="{ height: `${rows.length * ROW_HEIGHT}px`, position: 'relative' }">
      <div :style="{ transform: `translateY(${firstIndex * ROW_HEIGHT}px)` }">
        <TaskRow
          v-for="{ row } in visible"
          :key="row.task.id"
          :row="row"
          :selected="row.task.id === tree.selectedId.value"
          :renaming="row.task.id === tree.renamingId.value"
          @select="tree.select(row.task.id)"
          @toggle-expanded="tree.toggleExpanded(row.task.id)"
          @toggle-done="tree.toggleDone(row.task.id)"
          @dblclick="tree.handle('open')"
          @commit-rename="(title) => (tree.commitRename(row.task.id, title), refocus())"
          @cancel-rename="(tree.cancelRename(), refocus())"
        />
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
</style>
