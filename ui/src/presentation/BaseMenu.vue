<script setup lang="ts">
/**
 * Menu d'une pastille du bandeau (clic droit, ou bouton « ⋯ ») : renommer l'étiquette, changer
 * la couleur, changer le mot de passe du coffre (s'il existe), détacher la base de TodoQuoi (le dossier n'est ni effacé ni modifié ; l'app
 * se recharge à chaud). Les réglages sont ceux de l'appareil (ADR-0009).
 */
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { BASE_COLORS, type DataLocationState } from '../application/useDataLocation'
import type { VaultsState } from '../application/useVaults'
import { t } from '../i18n'

const props = defineProps<{ bases: DataLocationState; baseId: string; vaults: VaultsState }>()
const emit = defineEmits<{ close: [] }>()

const name = ref('')
const root = ref<HTMLElement | null>(null)

// Un clic ailleurs ferme le menu (le nom saisi est enregistré par la perte de focus).
function closeOnOutsideClick(event: MouseEvent): void {
  if (root.value && !root.value.contains(event.target as Node)) emit('close')
}
onMounted(() => setTimeout(() => window.addEventListener('mousedown', closeOnOutsideClick)))
onBeforeUnmount(() => window.removeEventListener('mousedown', closeOnOutsideClick))
const nameInput = ref<HTMLInputElement | null>(null)

watch(
  () => props.baseId,
  async () => {
    name.value = props.bases.infoOf(props.baseId).name
    await nextTick()
    nameInput.value?.select()
  },
  { immediate: true },
)

async function rename(): Promise<void> {
  const info = props.bases.infoOf(props.baseId)
  if (name.value.trim() && name.value.trim() !== info.name)
    await props.bases.updateBase(info.id, name.value, info.color)
  emit('close')
}

function recolor(color: string): void {
  const info = props.bases.infoOf(props.baseId)
  void props.bases.updateBase(info.id, info.name, color)
}

function changePassword(): void {
  props.vaults.ask('change', props.baseId)
  emit('close')
}

function detach(): void {
  const info = props.bases.infoOf(props.baseId)
  if (window.confirm(t('bases.removeConfirm', { name: info.name }))) void props.bases.removeBase(info.id)
  emit('close')
}
</script>

<template>
  <div ref="root" class="base-menu" role="dialog" :aria-label="t('bases.menu')" @keydown.esc.stop="emit('close')">
    <form @submit.prevent="rename">
      <label>
        {{ t('bases.label') }}
        <input ref="nameInput" v-model="name" maxlength="40" @blur="rename" />
      </label>
    </form>
    <div class="colors">
      <button
        v-for="color in BASE_COLORS"
        :key="color"
        type="button"
        class="swatch"
        :class="{ chosen: bases.infoOf(baseId).color === color }"
        :style="{ background: color }"
        :aria-label="color"
        @mousedown.prevent
        @click="recolor(color)"
      />
      <input
        type="color"
        :value="bases.infoOf(baseId).color"
        :aria-label="t('bases.color')"
        @change="recolor(($event.target as HTMLInputElement).value)"
      />
    </div>
    <button v-if="vaults.stateOf(baseId) !== 'none'" type="button" @mousedown.prevent @click="changePassword">
      {{ t('vault.changeMenu') }}
    </button>
    <button type="button" class="detach" @mousedown.prevent @click="detach">{{ t('bases.detach') }}</button>
    <p v-if="bases.error.value" class="error" role="alert">{{ bases.error.value }}</p>
  </div>
</template>

<style scoped>
.base-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  z-index: 40;
  width: 240px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  font-size: 12px;
}
label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  color: var(--muted);
}
input:not([type='color']) {
  font: inherit;
  font-size: 13px;
  padding: 3px 6px;
  color: var(--text, inherit);
}
.colors {
  display: flex;
  align-items: center;
  gap: 5px;
}
.swatch {
  width: 20px;
  height: 20px;
  padding: 0;
  border-radius: 50%;
  border: 2px solid transparent;
}
.swatch.chosen {
  border-color: var(--text, #000);
}
input[type='color'] {
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  background: none;
}
.detach {
  align-self: flex-start;
  color: var(--danger);
}
.error {
  margin: 0;
  color: var(--danger);
}
</style>
