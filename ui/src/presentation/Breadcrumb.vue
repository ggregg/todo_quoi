<script setup lang="ts">
/** Fil d'Ariane du zoom : « Accueil › Projet › Sous-projet ». Chaque élément est cliquable. */
import type { Task, TaskId } from '../domain/task'
import { t } from '../i18n'

defineProps<{ path: Task[] }>()
const emit = defineEmits<{ navigate: [id: TaskId | null] }>()
</script>

<template>
  <nav class="breadcrumb" aria-label="Fil d'Ariane">
    <button class="link" :class="{ current: path.length === 0 }" @click="emit('navigate', null)">
      {{ t('breadcrumb.home') }}
    </button>
    <template v-for="(task, index) in path" :key="task.id">
      <span class="separator">›</span>
      <button class="link" :class="{ current: index === path.length - 1 }" @click="emit('navigate', task.id)">
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
</style>
