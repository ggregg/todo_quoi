<script setup lang="ts">
/**
 * Bouton « Archives » de la barre de liste (R15) : enfoncé, les archives apparaissent à leur
 * place, grisées et en lecture seule. Le choix est mémorisé sur l'appareil (`useTaskTree`).
 */
import { computed } from 'vue'
import type { TaskTree } from '../application/useTaskTree'
import { t } from '../i18n'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ tree: TaskTree }>()

const shown = computed(() => props.tree.archiveVisibility.value === 'shown')
</script>

<template>
  <button
    type="button"
    class="archives-toggle"
    :class="{ chosen: shown }"
    :aria-pressed="shown"
    :title="shown ? t('archive.toggleHide') : t('archive.toggleShow')"
    @click="tree.toggleArchives()"
  >
    <AppIcon name="archive" /> {{ t('archive.toggle') }}
  </button>
</template>

<style scoped>
.archives-toggle {
  padding: 2px 8px;
  font-size: 12px;
}
.archives-toggle.chosen {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}
</style>
