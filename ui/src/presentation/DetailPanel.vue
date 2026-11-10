<script setup lang="ts">
/**
 * Panneau de détail (spec §5.4) : statut, titre, étiquettes, note, sous-tâches, méta-données.
 * Le même composant sert dans le panneau latéral (`mode="panel"`) et dans une fenêtre de tâche
 * (`mode="window"`, spec §5.4bis).
 *
 * Coffre (spec §9) : bouton 🔒 / 🔓 pour chiffrer ou remettre en clair (si `vaults` est fourni) ;
 * seule la **note** est chiffrée : coffre verrouillé, elle est remplacée par un bouton
 * « Déverrouiller » ; titre, étiquettes et sous-tâches restent affichés et modifiables.
 */
import AppIcon from './AppIcon.vue'
import TagInput from './TagInput.vue'
import { computed, ref, watch } from 'vue'
import type { TaskDetailState } from '../application/useTaskDetail'
import type { TaskId } from '../domain/task'
import type { VaultsState } from '../application/useVaults'
import { t } from '../i18n'
import NoteEditor from './editor/NoteEditor.vue'

const props = defineProps<{ state: TaskDetailState; mode: 'panel' | 'window'; vaults?: VaultsState }>()
/** `openTask` : clic sur le titre d'une sous-tâche — aller sur cette tâche. */
const emit = defineEmits<{ close: []; detach: []; dock: []; openTask: [id: TaskId] }>()

const title = ref('')
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

function addSubtask(): void {
  void props.state.addSubtask(newSubtask.value)
  newSubtask.value = ''
}

const sealed = computed(() => props.state.task.value?.confidentiality === 'sealed')
const encrypted = computed(() => props.state.task.value?.confidentiality === 'encrypted')

async function toggleEncryption(): Promise<void> {
  const task = props.state.task.value
  if (!task || !props.vaults) return
  await props.state.flush() // la note en cours de saisie part avec la tâche
  if (encrypted.value) await props.vaults.decrypt(task)
  else await props.vaults.encrypt(task, props.state.children.value.length)
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
          ><AppIcon v-if="state.task.value.done" name="check" :size="12"
        /></span>
        <input
          v-model="title"
          class="title"
          :aria-label="t('detail.titleLabel')"
          @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
          @blur="commitTitle"
        />
        <span class="save-state" :class="state.saveState.value">{{ saveLabels[state.saveState.value] }}</span>
        <AppIcon v-if="sealed" class="sealed-mark" name="lock" :size="14" :label="t('vault.sealedMessage')" />
        <button
          v-if="vaults && !sealed"
          class="icon"
          :class="{ active: encrypted }"
          :title="encrypted ? t('vault.decrypt') : t('vault.encrypt')"
          :aria-pressed="encrypted"
          @click="toggleEncryption"
        >
          <AppIcon :name="encrypted ? 'lock' : 'unlock'" />
        </button>
        <button v-if="mode === 'panel'" class="icon" :title="t('detail.detach')" @click="emit('detach')">
          <AppIcon name="detach" />
        </button>
        <button v-else class="icon" :title="t('detail.dock')" @click="emit('dock')"><AppIcon name="dock" /></button>
        <button class="icon" :title="t('detail.close')" @click="emit('close')"><AppIcon name="close" /></button>
      </header>

      <div v-if="state.conflict.value" class="conflict" role="alert">
        <span>{{ t('conflict.message', { device: state.conflict.value.device }) }}</span>
        <button type="button" @click="state.reloadTheirs()">{{ t('conflict.reload') }}</button>
        <button type="button" class="primary" @click="state.keepMine()">{{ t('conflict.keepMine') }}</button>
      </div>

      <div class="body">
        <div class="tags" :aria-label="t('detail.tags')">
          <span v-for="tag in state.task.value.tags" :key="tag" class="chip">
            #{{ tag }}
            <button class="remove" :aria-label="t('detail.removeTag', { tag })" @click="state.removeTag(tag)">
              <AppIcon name="close" :size="10" />
            </button>
          </span>
          <TagInput :usage="state.tagUsage.value" :exclude="state.task.value.tags" @add="state.addTag" />
        </div>

        <div v-if="sealed" class="note sealed-note">
          <AppIcon name="lock" :size="24" />
          <p>{{ t('vault.sealedMessage') }}</p>
          <button v-if="vaults" type="button" class="primary" @click="vaults.ask('unlock', state.task.value.base)">
            {{ t('vault.unlockToRead') }}
          </button>
        </div>
        <NoteEditor
          v-else
          :key="state.task.value.id"
          :task-id="state.task.value.id"
          class="note"
          :content="state.detail.value!.note"
          :version="state.noteVersion.value"
          @update="state.noteChanged"
          @blur="state.flush"
          @notify="state.notify"
        />

        <h3>
          {{ t('detail.subtasks') }}
          <span v-if="state.progress.value.total > 0"
            >({{ state.progress.value.done }}/{{ state.progress.value.total }})</span
          >
        </h3>
        <ul class="subtasks">
          <li v-for="child in state.children.value" :key="child.id" :class="{ done: child.done }">
            <span
              class="checkbox"
              :class="{ checked: child.done }"
              role="checkbox"
              :aria-checked="child.done"
              @click="state.toggleDone(child.id)"
              ><AppIcon v-if="child.done" name="check" :size="12"
            /></span>
            <button
              type="button"
              class="subtask-title"
              :title="t('detail.openSubtask')"
              @click="state.flush().then(() => emit('openTask', child.id))"
            >
              {{ child.title }}
            </button>
          </li>
        </ul>
        <input
          v-model="newSubtask"
          class="subtask-input"
          :placeholder="t('detail.addSubtask')"
          @keydown.enter.prevent="addSubtask"
        />

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
.sealed-mark {
  color: var(--accent);
}
.sealed-note {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--muted);
  text-align: center;
}
.icon.active {
  color: var(--accent);
}
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
.conflict {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  background: color-mix(in srgb, var(--danger) 14%, var(--surface));
  border-bottom: 1px solid var(--danger);
  font-size: 13px;
}
.conflict span {
  flex: 1 1 220px;
}
.body {
  flex: 1;
  overflow: auto;
  padding: 10px 18px 24px;
  display: flex;
  flex-direction: column;
}
/* La note est le cœur du panneau : elle prend toute la place disponible. */
.note {
  flex: 1 0 auto;
  margin-top: 12px;
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
.subtask-title {
  padding: 0;
  border: none;
  background: none;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.subtask-title:hover,
.subtask-title:focus-visible {
  color: var(--accent);
  text-decoration: underline;
}
.subtasks li.done .subtask-title {
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
