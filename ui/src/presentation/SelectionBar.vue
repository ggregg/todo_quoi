<script setup lang="ts">
/**
 * Barre de la sélection multiple (spec §5.5) : nombre de tâches choisies et actions groupées
 * (archiver, restaurer, corbeille). Elle remplace les filtres de la barre de liste tant que
 * plusieurs tâches sont sélectionnées. Les mêmes actions existent au clavier (`Mod+Maj+A`, `Suppr`).
 */
import { computed } from 'vue'
import type { TaskTree } from '../application/useTaskTree'
import { isMacPlatform } from '../application/keyboard'
import { t } from '../i18n'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ tree: TaskTree }>()

const shortcut = isMacPlatform(navigator.userAgent) ? '⌘⇧A' : 'Ctrl+Maj+A'
const selected = computed(() =>
  props.tree.selectedIds.value.map((id) => props.tree.tasks.value.find((task) => task.id === id)),
)
const hasActive = computed(() => selected.value.some((task) => task && !task.archivedAt))
const hasArchived = computed(() => selected.value.some((task) => task?.archivedAt))
</script>

<template>
  <div class="selection-bar" role="toolbar" :aria-label="t('selection.count', { count: selected.length })">
    <span class="count">{{ t('selection.count', { count: selected.length }) }}</span>
    <button
      v-if="hasActive"
      type="button"
      :title="t('selection.archiveHint', { shortcut })"
      @click="tree.archiveSelection()"
    >
      <AppIcon name="archive" :size="14" /> {{ t('selection.archive') }}
    </button>
    <button v-if="hasArchived" type="button" @click="tree.unarchiveSelection()">
      <AppIcon name="archive" :size="14" /> {{ t('selection.unarchive') }}
    </button>
    <button type="button" :title="t('selection.deleteHint')" @click="tree.removeSelection()">
      <AppIcon name="trash" :size="14" /> {{ t('selection.delete') }}
    </button>
    <button
      type="button"
      class="clear"
      :title="t('selection.clear')"
      :aria-label="t('selection.clear')"
      @click="tree.clearSelection()"
    >
      <AppIcon name="close" :size="12" />
    </button>
  </div>
</template>

<style scoped>
.selection-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
}
.count {
  font-weight: 600;
  color: var(--accent);
  margin-right: 4px;
}
.selection-bar button {
  padding: 2px 8px;
  font-size: 12px;
}
.selection-bar button.clear {
  padding: 2px 4px;
}
</style>
