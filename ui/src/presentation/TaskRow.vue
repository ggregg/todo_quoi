<script setup lang="ts">
/**
 * Une ligne de la liste : chevron, case à cocher, titre (ou champ de renommage), étiquettes,
 * progression des sous-tâches. Purement présentationnel : il émet des intentions.
 */
import { nextTick, ref, watch } from 'vue'
import type { Row } from '../application/treeView'
import { t } from '../i18n'

const props = defineProps<{ row: Row; selected: boolean; renaming: boolean }>()
const emit = defineEmits<{
  select: []
  toggleExpanded: []
  toggleDone: []
  commitRename: [title: string]
  cancelRename: []
}>()

const input = ref<HTMLInputElement | null>(null)

// La ligne peut exister AVANT de passer en renommage (création : la tâche apparaît, puis on
// demande à la renommer). On réagit donc au changement de `renaming`, pas seulement au montage.
watch(
  () => props.renaming,
  async (renaming) => {
    if (!renaming) return
    await nextTick()
    input.value?.focus()
    input.value?.select()
  },
  { immediate: true },
)

function onRenameKey(event: KeyboardEvent): void {
  // Les touches du champ ne doivent pas déclencher les raccourcis de la liste.
  event.stopPropagation()
  if (event.key === 'Enter') emit('commitRename', (event.target as HTMLInputElement).value)
  if (event.key === 'Escape') emit('cancelRename')
}
</script>

<template>
  <div
    class="row"
    role="treeitem"
    :aria-selected="selected"
    :aria-expanded="row.childCount > 0 ? row.expanded : undefined"
    :aria-level="row.depth + 1"
    :class="{ selected, done: row.task.done }"
    :style="{ paddingLeft: `${10 + row.depth * 20}px` }"
    @mousedown="emit('select')"
  >
    <span class="chevron" @mousedown.stop="emit('toggleExpanded')">
      {{ row.childCount > 0 ? (row.expanded ? '▼' : '▶') : '' }}
    </span>
    <span
      class="checkbox"
      :class="{ checked: row.task.done }"
      role="checkbox"
      :aria-checked="row.task.done"
      @mousedown.stop="emit('select'), emit('toggleDone')"
      >{{ row.task.done ? '✓' : '' }}</span
    >
    <input
      v-if="renaming"
      ref="input"
      class="rename"
      :value="row.task.title"
      @keydown="onRenameKey"
      @blur="emit('commitRename', ($event.target as HTMLInputElement).value)"
    />
    <span v-else class="title">{{ row.task.title }}</span>
    <span v-if="row.task.hasNote" class="icon" :title="t('task.hasNote')">📝</span>
    <span v-for="tag in row.task.tags" :key="tag" class="chip">#{{ tag }}</span>
    <span
      class="progress"
      :class="{ complete: row.childCount > 0 && row.doneChildCount === row.childCount }"
      :title="row.childCount > 0 ? t('task.progress', { done: row.doneChildCount, total: row.childCount }) : undefined"
    >
      {{ row.childCount > 0 ? `${row.doneChildCount}/${row.childCount}` : '' }}
    </span>
  </div>
</template>

<style scoped>
.row {
  display: flex;
  align-items: center;
  gap: 6px;
  height: var(--row-height);
  padding-right: 12px;
  border-left: 3px solid transparent;
  user-select: none;
  cursor: default;
}
.row:hover {
  background: var(--hover);
}
.row.selected {
  background: var(--selected);
  border-left-color: var(--accent);
}
.chevron {
  width: 16px;
  flex: none;
  text-align: center;
  font-size: 10px;
  color: var(--muted);
  cursor: pointer;
}
.checkbox {
  width: 16px;
  height: 16px;
  flex: none;
  display: grid;
  place-items: center;
  border: 1.5px solid var(--muted);
  border-radius: 4px;
  font-size: 11px;
  color: #fff;
  cursor: pointer;
}
.checkbox.checked {
  background: var(--ok);
  border-color: var(--ok);
}
.title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.done .title {
  color: var(--done);
  text-decoration: line-through;
}
.rename {
  flex: 1;
  height: 24px;
  font: inherit;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--accent);
  border-radius: 4px;
  padding: 0 6px;
}
.icon {
  font-size: 12px;
}
.progress {
  min-width: 34px;
  text-align: right;
  font-size: 11px;
  color: var(--muted);
}
.progress.complete {
  color: var(--ok);
}
</style>
