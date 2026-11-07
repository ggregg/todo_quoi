<script setup lang="ts">
/**
 * Fil d'Ariane du zoom : « Accueil › Projet › Sous-projet ». Chaque élément est cliquable, et
 * accepte une tâche glissée depuis la liste (elle va à la fin de cette liste : « Accueil » =
 * premier niveau, ce qui détache une sous-tâche).
 */
import { ref } from 'vue'
import type { Task, TaskId } from '../domain/task'
import { t } from '../i18n'
import { TASK_DRAG_TYPE } from './dragTypes'

defineProps<{ path: Task[] }>()
const emit = defineEmits<{ navigate: [id: TaskId | null]; dropInto: [id: TaskId | null] }>()

/** Élément survolé pendant un glisser (`undefined` = aucun ; `null` = Accueil). */
const hovered = ref<TaskId | null | undefined>(undefined)

function onDragOver(event: DragEvent, id: TaskId | null): void {
  if (!event.dataTransfer?.types.includes(TASK_DRAG_TYPE)) return
  event.preventDefault()
  hovered.value = id
}

function onDrop(event: DragEvent, id: TaskId | null): void {
  event.preventDefault()
  hovered.value = undefined
  emit('dropInto', id)
}
</script>

<template>
  <nav class="breadcrumb" aria-label="Fil d'Ariane">
    <button
      class="link"
      :class="{ current: path.length === 0, 'drop-target': hovered === null }"
      @click="emit('navigate', null)"
      @dragover="onDragOver($event, null)"
      @dragleave="hovered = undefined"
      @drop="onDrop($event, null)"
    >
      {{ t('breadcrumb.home') }}
    </button>
    <template v-for="(task, index) in path" :key="task.id">
      <span class="separator">›</span>
      <button
        class="link"
        :class="{ current: index === path.length - 1, 'drop-target': hovered === task.id }"
        @click="emit('navigate', task.id)"
        @dragover="onDragOver($event, task.id)"
        @dragleave="hovered = undefined"
        @drop="onDrop($event, task.id)"
      >
        {{ task.title }}
      </button>
    </template>
  </nav>
</template>

<style scoped>
.breadcrumb {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
}
.link {
  background: none;
  border: none;
  padding: 2px 6px;
  color: var(--muted);
}
.link:hover {
  background: var(--hover);
  color: var(--text);
}
.link.current {
  color: var(--text);
  font-weight: 600;
}
.separator {
  color: var(--muted);
}
.drop-target {
  outline: 1px dashed var(--accent);
  background: var(--accent-soft);
}
</style>
