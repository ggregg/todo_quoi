<script setup lang="ts">
/**
 * Filtre rapide de la barre de liste (spec §5.3) : « Toutes / Ouvertes / Terminées », sans ouvrir
 * l'éditeur « Vue ». C'est le filtre de statut de la vue (même réglage, enregistré et hérité) ;
 * le détail (liste qui reçoit sa propre vue, mode « Tout ») est dans `useTaskTree`.
 */
import { computed } from 'vue'
import type { TaskTree } from '../application/useTaskTree'
import type { StatusChoice } from '../domain/view'
import { t } from '../i18n'

const props = defineProps<{ tree: TaskTree }>()

const STATUSES: readonly StatusChoice[] = ['all', 'open', 'done']

/** Mode « Tout » (plusieurs bases, sans zoom) : le choix vaut pour chaque base affichée. */
const allBases = computed(() => props.tree.zoomId.value === null && props.tree.visibleBases.value.length > 1)
</script>

<template>
  <div
    class="status-filter"
    role="group"
    :aria-label="t('view.quickStatus')"
    :title="allBases ? t('view.quickStatusAllBases') : undefined"
  >
    <button
      v-for="status in STATUSES"
      :key="status"
      type="button"
      :aria-pressed="tree.statusFilter.value === status"
      :class="{ chosen: tree.statusFilter.value === status }"
      @click="tree.setStatusFilter(status)"
    >
      {{ t(`view.status.${status}`) }}
    </button>
  </div>
</template>

<style scoped>
.status-filter {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}
.status-filter button {
  padding: 2px 10px;
  font-size: 12px;
  color: inherit;
  background: transparent;
  border: none;
  border-radius: 0;
}
.status-filter button + button {
  border-left: 1px solid var(--border);
}
.status-filter button:hover:not(.chosen) {
  background: var(--hover);
}
.status-filter button.chosen {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}
</style>
