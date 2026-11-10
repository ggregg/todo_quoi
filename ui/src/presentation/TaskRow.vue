<script setup lang="ts">
/**
 * Une ligne de la liste : chevron, case à cocher, titre (ou champ de renommage), puis les
 * colonnes choisies par la vue de la liste affichée (spec §5.3). Purement présentationnel : il
 * émet des intentions.
 */
import AppIcon, { type IconName } from './AppIcon.vue'
import { nextTick, ref, watch } from 'vue'
import type { Row } from '../application/treeView'
import type { Column, MediaKind } from '../domain/view'
import { dropZoneAt, type DropZone } from '../application/treeDrop'
import { TASK_DRAG_TYPE } from './dragTypes'
import { formatDay, t } from '../i18n'

const props = defineProps<{
  row: Row
  selected: boolean
  renaming: boolean
  columns: Column[]
  /** Une tâche peut-elle être lâchée ici, dans cette zone ? (fourni par l'arbre) */
  canDrop: (zone: DropZone) => boolean
}>()

const MEDIA_ICONS: Record<MediaKind, IconName> = { image: 'image', audio: 'mic', file: 'paperclip' }

/** Date courte, dans la langue du système (`12/11/2026`). */
function shortDate(iso: string | null): string {
  return iso ? formatDay(iso) : ''
}
const emit = defineEmits<{
  select: []
  toggleExpanded: []
  toggleDone: []
  commitRename: [title: string]
  cancelRename: []
  dragStart: []
  dragEnd: []
  dropHere: [zone: DropZone]
}>()

// ----- Glisser-déposer : repère avant / dedans / après -------------------------------------------
const dropZone = ref<DropZone | null>(null)

function zoneOf(event: DragEvent): DropZone {
  const box = (event.currentTarget as HTMLElement).getBoundingClientRect()
  return dropZoneAt((event.clientY - box.top) / box.height)
}

function onDragStart(event: DragEvent): void {
  event.dataTransfer?.setData(TASK_DRAG_TYPE, props.row.task.id)
  if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move'
  emit('dragStart')
}

function onDragOver(event: DragEvent): void {
  if (!event.dataTransfer?.types.includes(TASK_DRAG_TYPE)) return
  const zone = zoneOf(event)
  dropZone.value = props.canDrop(zone) ? zone : null
  if (dropZone.value) event.preventDefault()
}

function onDrop(event: DragEvent): void {
  const zone = dropZone.value
  dropZone.value = null
  if (!zone) return
  event.preventDefault()
  event.stopPropagation()
  emit('dropHere', zone)
}

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
    :class="{ selected, done: row.task.done, [`drop-${dropZone}`]: dropZone !== null }"
    :draggable="!renaming"
    :style="{ paddingLeft: `${10 + row.depth * 20}px` }"
    @dragstart="onDragStart"
    @dragend="emit('dragEnd')"
    @dragover="onDragOver"
    @dragleave="dropZone = null"
    @drop="onDrop"
    @mousedown="emit('select')"
  >
    <span class="chevron" @mousedown.stop="emit('toggleExpanded')">
      <AppIcon v-if="row.childCount > 0" :name="row.expanded ? 'chevronDown' : 'chevronRight'" :size="12" />
    </span>
    <span
      class="checkbox"
      :class="{ checked: row.task.done }"
      role="checkbox"
      :aria-checked="row.task.done"
      :aria-label="t('task.done')"
      @mousedown.stop="(emit('select'), emit('toggleDone'))"
      ><AppIcon v-if="row.task.done" name="check" :size="12"
    /></span>
    <input
      v-if="renaming"
      ref="input"
      class="rename"
      :value="row.task.title"
      @keydown="onRenameKey"
      @blur="emit('commitRename', ($event.target as HTMLInputElement).value)"
    />
    <span v-else class="title"
      >{{ row.task.title }}
      <AppIcon
        v-if="row.task.confidentiality !== 'plain'"
        class="encrypted"
        :class="{ sealed: row.task.confidentiality === 'sealed' }"
        name="lock"
        :size="11"
        :label="row.task.confidentiality === 'sealed' ? t('vault.sealedMessage') : t('vault.encrypted')"
    /></span>
    <template v-for="column in columns" :key="column">
      <span v-if="column === 'icons'" class="icons">
        <span v-if="row.task.hasNote" :title="t('task.hasNote')"><AppIcon name="note" :size="13" /></span>
        <span v-for="kind in row.media" :key="kind" :title="t(`task.media.${kind}`)"
          ><AppIcon :name="MEDIA_ICONS[kind]" :size="13"
        /></span>
      </span>
      <span v-else-if="column === 'tags'" class="tags">
        <span v-for="tag in row.task.tags" :key="tag" class="chip">#{{ tag }}</span>
      </span>
      <span
        v-else-if="column === 'progress'"
        class="progress"
        :class="{ complete: row.childCount > 0 && row.doneChildCount === row.childCount }"
        :title="
          row.childCount > 0 ? t('task.progress', { done: row.doneChildCount, total: row.childCount }) : undefined
        "
      >
        {{ row.childCount > 0 ? `${row.doneChildCount}/${row.childCount}` : '' }}
      </span>
      <span v-else class="date" :title="t(`view.column.${column}`)">
        {{
          shortDate(
            column === 'createdAt' ? row.task.createdAt : column === 'updatedAt' ? row.task.updatedAt : row.task.doneAt,
          )
        }}
      </span>
    </template>
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
/* Repères de dépôt : trait avant / après, cadre pour « dedans ». */
.row.drop-before {
  box-shadow: inset 0 2px 0 var(--accent);
}
.row.drop-after {
  box-shadow: inset 0 -2px 0 var(--accent);
}
.row.drop-inside {
  background: var(--accent-soft);
  outline: 1px dashed var(--accent);
  outline-offset: -2px;
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
  color: var(--on-accent);
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
.title .encrypted {
  margin-left: 4px;
  vertical-align: -1px;
  color: var(--muted);
}
.title .encrypted.sealed {
  color: var(--accent);
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
.icons {
  display: flex;
  gap: 2px;
  font-size: 12px;
}
.tags {
  display: flex;
  gap: 4px;
  overflow: hidden;
  max-width: 40%;
}
.date {
  min-width: 72px;
  text-align: right;
  font-size: 11px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
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
