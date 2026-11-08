<script setup lang="ts">
/**
 * Écran de la corbeille, à la place de l'arbre : tâches supprimées (les plus récentes d'abord)
 * et bouton « Restaurer » (avec leurs sous-tâches). `Échap` ou « Retour » ramène à la liste.
 */
import { computed, onMounted, ref } from 'vue'
import type { BaseInfo } from '../ports/SettingsGateway'
import type { Trash } from '../application/useTrash'
import { t } from '../i18n'
import AppIcon from './AppIcon.vue'

/** `bases` : bases affichées (la corbeille s'y limite) ; `baseInfo` : nom et couleur. */
const props = defineProps<{
  trash: Trash
  pathTo: (parentId: string | null) => string[]
  bases: string[]
  baseInfo: (id: string) => BaseInfo
}>()
const items = computed(() => props.trash.items.value.filter((task) => props.bases.includes(task.base)))
const emit = defineEmits<{ back: [] }>()
const root = ref<HTMLElement | null>(null)

onMounted(() => root.value?.focus())

function deletedOn(iso: string | null): string {
  return iso ? new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : ''
}
</script>

<template>
  <section ref="root" class="trash" tabindex="-1" :aria-label="t('trash.title')" @keydown.esc.prevent="emit('back')">
    <header>
      <button type="button" class="back" @click="emit('back')">
        <AppIcon name="chevronRight" class="flip" /> {{ t('trash.back') }}
      </button>
      <h2><AppIcon name="trash" /> {{ t('trash.title') }}</h2>
    </header>
    <p v-if="trash.loading.value && items.length === 0" class="hint">{{ t('status.loading') }}</p>
    <p v-else-if="items.length === 0" class="hint">{{ t('trash.empty') }}</p>
    <ul v-else>
      <li v-for="task in items" :key="task.id" :style="{ '--base-color': baseInfo(task.base).color }">
        <div class="text">
          <span class="title" :class="{ done: task.done }">
            <span v-if="bases.length > 1" class="base-dot" :title="baseInfo(task.base).name" />{{ task.title }}
          </span>
          <span class="date">
            {{ t('trash.deletedOn', { date: deletedOn(task.deletedAt) }) }}
            <template v-if="pathTo(task.parentId).length > 0">
              · {{ t('trash.from', { path: pathTo(task.parentId).join(' › ') }) }}
            </template>
          </span>
        </div>
        <button type="button" @click="trash.restore(task.id)">
          <AppIcon name="undo" :size="14" /> {{ t('trash.restore') }}
        </button>
      </li>
    </ul>
    <p class="hint footer">{{ t('trash.note') }}</p>
  </section>
</template>

<style scoped>
.trash {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  outline: none;
}
header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
}
h2 {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-size: 14px;
}
.back {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  font-size: 12px;
}
.flip {
  transform: scaleX(-1);
}
ul {
  flex: 1;
  overflow-y: auto;
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border);
}
li:hover {
  background: var(--hover);
}
.text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.title.done {
  text-decoration: line-through;
  color: var(--done);
}
.date,
.hint {
  font-size: 12px;
  color: var(--muted);
}
.hint {
  margin: 0;
  padding: 12px;
}
.footer {
  border-top: 1px solid var(--border);
}
li button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  font-size: 12px;
}
.base-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  margin-right: 6px;
  border-radius: 50%;
  background: var(--base-color);
}
</style>
