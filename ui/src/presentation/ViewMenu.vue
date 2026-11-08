<script setup lang="ts">
/**
 * Éditeur de la vue de la liste affichée (spec §5.3) : colonnes, tri (deux critères), filtres.
 * Une vue héritée est en lecture seule tant qu'on ne choisit pas « Personnaliser cette liste » ;
 * une vue propre est enregistrée à chaque modification.
 */
import AppIcon from './AppIcon.vue'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import type { TaskTree } from '../application/useTaskTree'
import { setSortKey, setStatus, setTags, toggleColumn, toggleWithMedia } from '../application/viewEditing'
import { ALL_COLUMNS, ALL_SORT_FIELDS, defaultView, type ListView, type SortField } from '../domain/view'
import { t } from '../i18n'

const props = defineProps<{ tree: TaskTree }>()
const open = ref(false)
const root = ref<HTMLElement | null>(null)

// Un clic hors du menu (et hors de son bouton) le ferme.
function closeOnOutsideClick(event: MouseEvent): void {
  if (root.value && !root.value.contains(event.target as Node)) open.value = false
}
watch(open, (isOpen) => {
  if (isOpen) window.addEventListener('mousedown', closeOnOutsideClick)
  else window.removeEventListener('mousedown', closeOnOutsideClick)
})
onBeforeUnmount(() => window.removeEventListener('mousedown', closeOnOutsideClick))

const layout = computed(() => props.tree.currentLayout.value)
const view = computed(() => layout.value?.view ?? defaultView())
const isRoot = computed(() => props.tree.zoomId.value === null)
/** Mode « Tout » (plusieurs bases, sans zoom) : la vue racine dépend de la base — on en choisit une. */
const allMode = computed(() => props.tree.zoomId.value === null && props.tree.singleBase.value === null)
const editable = computed(() => !allMode.value && layout.value?.source.kind === 'own')
const sourceText = computed(() => {
  const source = layout.value?.source
  if (!source || source.kind === 'own') return t(isRoot.value ? 'view.source.rootOwn' : 'view.source.own')
  if (source.kind === 'root') return t('view.source.root')
  return t('view.source.inherited', { title: props.tree.titleOf(source.fromId) })
})

function save(next: ListView): void {
  void props.tree.customizeView(next)
}

function onSortField(position: number, field: string): void {
  const direction = view.value.sort[position]?.direction ?? 'asc'
  save(setSortKey(view.value, position, field === '' ? null : { field: field as SortField, direction }))
}

function onSortDirection(position: number): void {
  const key = view.value.sort[position]
  if (key) save(setSortKey(view.value, position, { ...key, direction: key.direction === 'asc' ? 'desc' : 'asc' }))
}
</script>

<template>
  <div ref="root" class="view-menu">
    <button type="button" class="toggle" :aria-expanded="open" @click="open = !open">
      <AppIcon name="sliders" /> {{ t('view.button') }}
    </button>
    <div v-if="open" class="panel" role="dialog" :aria-label="t('view.title')" @keydown.esc.stop="open = false">
      <p v-if="allMode" class="source">{{ t('bases.allModeView') }}</p>
      <p v-else class="source">{{ sourceText }}</p>
      <div v-if="!allMode" class="actions">
        <button v-if="!editable" type="button" class="primary" @click="save(view)">{{ t('view.customize') }}</button>
        <button v-if="editable && !isRoot" type="button" @click="tree.inheritView()">{{ t('view.inherit') }}</button>
        <button v-if="editable && isRoot" type="button" @click="save(defaultView())">{{ t('view.reset') }}</button>
      </div>
      <fieldset :disabled="!editable">
        <legend>{{ t('view.columns') }}</legend>
        <label v-for="column in ALL_COLUMNS" :key="column" class="check">
          <input type="checkbox" :checked="view.columns.includes(column)" @change="save(toggleColumn(view, column))" />
          {{ t(`view.column.${column}`) }}
        </label>
      </fieldset>
      <fieldset :disabled="!editable">
        <legend>{{ t('view.sort') }}</legend>
        <div v-for="position in [0, 1]" :key="position" class="sort">
          <span v-if="position === 1" class="then">{{ t('view.sortThen') }}</span>
          <select
            :value="view.sort[position]?.field ?? ''"
            :disabled="position === 1 && view.sort.length === 0"
            @change="onSortField(position, ($event.target as HTMLSelectElement).value)"
          >
            <option v-if="position === 1" value="">{{ t('view.sortNone') }}</option>
            <option v-for="field in ALL_SORT_FIELDS" :key="field" :value="field">{{ t(`view.field.${field}`) }}</option>
          </select>
          <button v-if="view.sort[position]" type="button" class="direction" @click="onSortDirection(position)">
            {{ view.sort[position]?.direction === 'desc' ? '↓' : '↑' }}
            {{ t(`view.direction.${view.sort[position]?.direction ?? 'asc'}`) }}
          </button>
        </div>
      </fieldset>
      <fieldset :disabled="!editable">
        <legend>{{ t('view.filters') }}</legend>
        <div class="status">
          <label v-for="status in ['all', 'open', 'done'] as const" :key="status" class="check">
            <input
              type="radio"
              name="view-status"
              :checked="view.filter.status === status"
              @change="save(setStatus(view, status))"
            />
            {{ t(`view.status.${status}`) }}
          </label>
        </div>
        <label class="field">
          {{ t('view.tagsIn') }}
          <input
            type="text"
            :placeholder="t('view.tagsHint')"
            :value="view.filter.tagsIn.join(' ')"
            @change="save(setTags(view, 'tagsIn', ($event.target as HTMLInputElement).value))"
          />
        </label>
        <label class="field">
          {{ t('view.tagsOut') }}
          <input
            type="text"
            :placeholder="t('view.tagsHint')"
            :value="view.filter.tagsOut.join(' ')"
            @change="save(setTags(view, 'tagsOut', ($event.target as HTMLInputElement).value))"
          />
        </label>
        <label class="check">
          <input type="checkbox" :checked="view.filter.media === 'withMedia'" @change="save(toggleWithMedia(view))" />
          {{ t('view.withMedia') }}
        </label>
      </fieldset>
    </div>
  </div>
</template>

<style scoped>
.view-menu {
  position: relative;
}
.toggle {
  padding: 2px 8px;
  font-size: 12px;
}
.panel {
  position: absolute;
  right: 0;
  top: calc(100% + 4px);
  z-index: 30;
  width: 300px;
  max-height: 70vh;
  overflow-y: auto;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  font-size: 13px;
}
.source {
  margin: 0;
  color: var(--muted);
}
.actions {
  display: flex;
  gap: 6px;
}
fieldset {
  margin: 0;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: 6px;
  display: flex;
  flex-wrap: wrap;
  gap: 4px 10px;
}
fieldset:disabled {
  opacity: 0.55;
}
legend {
  padding: 0 4px;
  font-size: 11px;
  color: var(--muted);
}
.check {
  display: flex;
  align-items: center;
  gap: 4px;
}
.sort,
.status {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 6px;
}
.then {
  color: var(--muted);
}
.direction {
  padding: 1px 6px;
  font-size: 12px;
}
.field {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.field input {
  font: inherit;
}
</style>
