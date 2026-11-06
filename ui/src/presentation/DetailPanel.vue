<script setup lang="ts">
/**
 * Panneau de détail (spec §5.4) : statut, titre, étiquettes, note, sous-tâches, méta-données.
 * Le même composant sert dans le panneau latéral (`mode="panel"`) et dans une fenêtre de tâche
 * (`mode="window"`, spec §5.4bis).
 */
import { ref, watch } from 'vue'
import type { TaskDetailState } from '../application/useTaskDetail'
import { t } from '../i18n'
import NoteEditor from './editor/NoteEditor.vue'

const props = defineProps<{ state: TaskDetailState; mode: 'panel' | 'window' }>()
const emit = defineEmits<{ close: []; detach: []; dock: [] }>()

const title = ref('')
const newTag = ref('')
const newSubtask = ref('')

watch(
  () => props.state.task.value?.title,
  (value) => (title.value = value ?? ''),
  { immediate: true },
)

const saveLabels = {
  saved: t('detail.saved'),
  pending: t('detail.unsaved'),
  saving: t('detail.saving'),
  error: t('detail.saveFailed'),
} as const

function commitTitle(): void {
  void props.state.rename(title.value)
}

function addTag(): void {
  void props.state.addTag(newTag.value)
  newTag.value = ''
}

function addSubtask(): void {
  void props.state.addSubtask(newSubtask.value)
  newSubtask.value = ''
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short' })
}
</script>

<template>
  <section class="detail" :class="mode" @keydown.esc.stop="emit('close')">
    <p v-if="state.missing.value" class="missing">{{ t('detail.notFound') }}</p>

    <template v-else-if="state.task.value">
      <header>
        <span
          class="checkbox"
          :class="{ checked: state.task.value.done }"
          role="checkbox"
          :aria-checked="state.task.value.done"
          tabindex="0"
          @click="state.toggleDone()"
          @keydown.space.prevent="state.toggleDone()"
          >{{ state.task.value.done ? '✓' : '' }}</span
        >
        <input
          v-model="title"
          class="title"
          :aria-label="t('detail.titleLabel')"
          @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
          @blur="commitTitle"
        />
        <span class="save-state" :class="state.saveState.value">{{ saveLabels[state.saveState.value] }}</span>
        <button v-if="mode === 'panel'" class="icon" :title="t('detail.detach')" @click="emit('detach')">⧉</button>
        <button v-else class="icon" :title="t('detail.dock')" @click="emit('dock')">⇤</button>
        <button class="icon" :title="t('detail.close')" @click="emit('close')">✕</button>
      </header>

      <div class="body">
        <div class="tags" :aria-label="t('detail.tags')">
          <span v-for="tag in state.task.value.tags" :key="tag" class="chip">
            #{{ tag }}
            <button class="remove" :aria-label="t('detail.removeTag', { tag })" @click="state.removeTag(tag)">✕</button>
          </span>
          <input v-model="newTag" class="tag-input" :placeholder="t('detail.addTag')" @keydown.enter.prevent="addTag" />
        </div>

        <h3>{{ t('detail.note') }}</h3>
        <NoteEditor
          :key="state.task.value.id"
          :content="state.detail.value!.note"
          :version="state.noteVersion.value"
          @update="state.noteChanged"
          @blur="state.flush"
        />

        <h3>
          {{ t('detail.subtasks') }}
          <span v-if="state.progress.value.total > 0">({{ state.progress.value.done }}/{{ state.progress.value.total }})</span>
        </h3>
        <ul class="subtasks">
          <li v-for="child in state.children.value" :key="child.id" :class="{ done: child.done }">
            <span
              class="checkbox"
              :class="{ checked: child.done }"
              role="checkbox"
              :aria-checked="child.done"
              @click="state.toggleDone(child.id)"
              >{{ child.done ? '✓' : '' }}</span
            >
            <span>{{ child.title }}</span>
          </li>
        </ul>
        <input v-model="newSubtask" class="subtask-input" :placeholder="t('detail.addSubtask')" @keydown.enter.prevent="addSubtask" />

        <p class="meta">
          {{
            t('detail.meta', {
              created: formatDate(state.task.value.createdAt),
              updated: formatDate(state.task.value.updatedAt),
              device: state.task.value.updatedBy,
            })
          }}
        </p>
      </div>
    </template>
  </section>
</template>

<style scoped>
.detail {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  background: var(--surface);
}
header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
}
.title {
  flex: 1;
  min-width: 0;
  font: 600 17px var(--font);
  color: inherit;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 6px;
  padding: 2px 6px;
}
.title:hover,
.title:focus {
  border-color: var(--border);
}
.save-state {
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
}
.save-state.error {
  color: var(--danger);
}
.icon {
  background: transparent;
  border-color: transparent;
  padding: 2px 8px;
}
.icon:hover {
  background: var(--hover);
}
.body {
  flex: 1;
  overflow: auto;
  padding: 10px 18px 24px;
}
h3 {
  margin: 18px 0 6px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: var(--muted);
}
.tags {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}
.remove {
  background: none;
  border: none;
  padding: 0 0 0 4px;
  color: inherit;
  opacity: 0.7;
  font-size: 10px;
}
.tag-input,
.subtask-input {
  font: inherit;
  font-size: 12px;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 2px 8px;
}
.subtask-input {
  width: 100%;
  margin-top: 6px;
  font-size: 13px;
  padding: 4px 8px;
}
.subtasks {
  list-style: none;
  margin: 0;
  padding: 0;
}
.subtasks li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 0;
}
.subtasks li.done span:last-child {
  color: var(--done);
  text-decoration: line-through;
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
.meta {
  margin-top: 18px;
  font-size: 12px;
  color: var(--muted);
}
.missing {
  padding: 30px;
  text-align: center;
  color: var(--muted);
}
</style>
