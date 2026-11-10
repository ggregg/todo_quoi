<script setup lang="ts">
/**
 * Bandeau des bases (spec §5.1bis, ADR-0009) : « Tout » (toujours présent) puis une pastille par
 * base, quel que soit son stockage (disque local, disque externe, drive cloud). Un clic sur
 * une base l'affiche seule ; « Tout » les affiche ensemble, en sections. On peut aussi lâcher une
 * tâche sur une pastille pour l'envoyer dans cette base. « + » : gérer / ajouter des bases.
 * Cadenas d'une base qui a un coffre (ADR-0010) : fermé = verrouillé (clic : déverrouiller),
 * ouvert = déverrouillé (clic : verrouiller).
 */
import { ref } from 'vue'
import type { DataLocationState } from '../application/useDataLocation'
import BaseMenu from './BaseMenu.vue'
import type { TaskTree } from '../application/useTaskTree'
import type { VaultsState } from '../application/useVaults'
import { t } from '../i18n'
import AppIcon from './AppIcon.vue'
import { TASK_DRAG_TYPE } from './dragTypes'

const props = defineProps<{ tree: TaskTree; bases: DataLocationState; vaults: VaultsState }>()
/** Base dont le menu (renommer, couleur, détacher) est ouvert. */
const menuFor = ref<string | null>(null)

function openMenu(id: string): void {
  menuFor.value = menuFor.value === id ? null : id
}

function acceptTask(event: DragEvent): void {
  if (props.tree.draggingId.value !== null && event.dataTransfer?.types.includes(TASK_DRAG_TYPE)) event.preventDefault()
}
</script>

<template>
  <nav class="base-bar" :aria-label="t('bases.bar')">
    <button
      type="button"
      class="chip all"
      :class="{ active: tree.display.value === 'all' }"
      @click="tree.showBases('all')"
    >
      {{ t('bases.all') }}
    </button>
    <span v-for="id in tree.allBases.value" :key="id" class="chip-holder">
      <button
        type="button"
        class="chip"
        :class="{
          active: tree.singleBase.value === id && tree.display.value !== 'all',
          faded: bases.infoOf(id).problem,
        }"
        :style="{ '--base-color': bases.infoOf(id).color }"
        :title="bases.infoOf(id).problem ?? bases.infoOf(id).path"
        @click="tree.showBases(id)"
        @dragover="acceptTask"
        @drop.prevent="tree.dropOnBase(id)"
        @contextmenu.prevent="openMenu(id)"
      >
        <span class="dot" />
        {{ bases.infoOf(id).name }}
        <span
          v-if="vaults.stateOf(id) !== 'none'"
          class="vault"
          :class="vaults.stateOf(id)"
          role="button"
          tabindex="0"
          :title="vaults.stateOf(id) === 'unlocked' ? t('vault.lock') : t('vault.locked')"
          :aria-label="vaults.stateOf(id) === 'unlocked' ? t('vault.lock') : t('vault.locked')"
          @click.stop="vaults.toggle(id)"
          @keydown.enter.stop.prevent="vaults.toggle(id)"
          ><AppIcon :name="vaults.stateOf(id) === 'unlocked' ? 'unlock' : 'lock'" :size="12"
        /></span>
        <span
          class="more"
          role="button"
          tabindex="0"
          :aria-label="t('bases.menu')"
          @click.stop="openMenu(id)"
          @keydown.enter.stop.prevent="openMenu(id)"
          >⋯</span
        >
      </button>
      <BaseMenu v-if="menuFor === id" :bases="bases" :base-id="id" :vaults="vaults" @close="menuFor = null" />
    </span>
    <button
      type="button"
      class="manage"
      :title="t('bases.manage')"
      :aria-label="t('bases.manage')"
      @click="bases.openDialog()"
    >
      <AppIcon name="plus" :size="14" />
    </button>
  </nav>
</template>

<style scoped>
.base-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 12px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  font-size: 12px;
  white-space: nowrap;
}
.chip.active {
  border-color: var(--base-color, var(--accent));
  background: color-mix(in srgb, var(--base-color, var(--accent)) 16%, var(--surface));
  font-weight: 600;
}
.vault {
  display: inline-grid;
  place-items: center;
  color: var(--muted);
}
.vault.locked {
  color: var(--base-color, var(--accent));
}
.chip.faded {
  opacity: 0.5;
}
.dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--base-color);
}
.chip-holder {
  position: relative;
}
/* « ⋯ » : ne prend aucune place au repos ; la pastille s'élargit au survol pour l'afficher. */
.more {
  display: inline-block;
  max-width: 0;
  margin-left: -6px;
  overflow: hidden;
  border-radius: 4px;
  color: var(--muted);
  opacity: 0;
  transition:
    max-width 0.15s ease,
    margin-left 0.15s ease,
    opacity 0.15s ease;
}
.chip:hover .more,
.more:focus {
  max-width: 1.4em;
  margin-left: 0;
  padding: 0 2px;
  opacity: 1;
}
/* Écran tactile (pas de survol) : toujours visible, sinon le menu serait inaccessible. */
@media (hover: none) {
  .more {
    max-width: 1.4em;
    margin-left: 0;
    padding: 0 2px;
    opacity: 1;
  }
}
.more:hover {
  background: var(--hover);
}
.base-bar {
  overflow: visible;
}
.manage {
  display: inline-grid;
  place-items: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border-radius: 50%;
}
</style>
