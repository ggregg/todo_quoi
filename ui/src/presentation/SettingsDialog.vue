<script setup lang="ts">
/**
 * Fenêtre « Réglages » (spec §5.7, `Mod+,`) : d'abord les bases (spec §5.1bis, ADR-0009) —
 * bases ouvertes (renommer, couleur, retirer), ajout d'une base (drive détecté ou chemin collé,
 * nom, couleur) —, puis les préférences de l'appareil (`PreferencesSections`). Ajouter ou retirer une
 * base recharge l'assemblage à chaud ; le dossier détaché n'est ni effacé ni modifié.
 */
import { inject, ref, watch } from 'vue'
import { BASE_COLORS, type DataLocationState } from '../application/useDataLocation'
import type { DriveProvider, StorageKind } from '../ports/SettingsGateway'
import type { VaultsState } from '../application/useVaults'
import { t } from '../i18n'
import AppIcon from './AppIcon.vue'
import PreferencesSections from './PreferencesSections.vue'
import { folderPickerKey, suggestedBaseName } from '../ports/FolderPicker'

const props = defineProps<{ state: DataLocationState; vaults: VaultsState }>()

const dialog = ref<HTMLElement | null>(null)
const newName = ref('')
const newColor = ref<string>(BASE_COLORS[1])
const newPath = ref('')
const picker = inject(folderPickerKey)

/** Fenêtre native : choisir un dossier existant (base à rattacher) ou en créer un nouveau. */
async function pickFolder(): Promise<void> {
  const chosen = await picker?.pickFolder(t('bases.pickTitle'))
  if (!chosen) return
  newPath.value = chosen
  if (!newName.value.trim()) newName.value = suggestedBaseName(chosen)
}

watch(
  () => props.state.dialogOpen.value,
  (open) => {
    newName.value = ''
    newPath.value = ''
    if (open) setTimeout(() => dialog.value?.focus())
  },
)

const PROVIDER_NAMES: Record<DriveProvider, string> = {
  icloud: 'iCloud Drive',
  googleDrive: 'Google Drive',
  oneDrive: 'OneDrive',
  dropbox: 'Dropbox',
}

/** Libellé du stockage d'une base. */
function storageLabel(storage: StorageKind): string {
  if (storage === 'local') return t('bases.storage.local')
  if (storage === 'external') return t('bases.storage.external')
  if (storage === 'network') return t('bases.storage.network')
  return PROVIDER_NAMES[storage]
}

function rename(id: string, color: string, event: Event): void {
  const name = (event.target as HTMLInputElement).value
  if (name.trim()) void props.state.updateBase(id, name, color)
}

function remove(id: string, name: string): void {
  if (window.confirm(t('bases.removeConfirm', { name }))) void props.state.removeBase(id)
}
</script>

<template>
  <div v-if="state.dialogOpen.value" class="backdrop" @mousedown.self="state.closeDialog()">
    <section
      ref="dialog"
      class="dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('settings.title')"
      tabindex="-1"
      @keydown.esc.stop="state.closeDialog()"
    >
      <header>
        <h2><AppIcon name="sliders" /> {{ t('settings.title') }}</h2>
        <button type="button" class="icon" :aria-label="t('detail.close')" @click="state.closeDialog()">
          <AppIcon name="close" />
        </button>
      </header>

      <p v-if="state.location.value?.kind === 'environment'" class="hint">{{ t('bases.envHint') }}</p>
      <p v-else-if="state.location.value?.deviceOnly" class="hint warning">{{ t('bases.deviceOnly') }}</p>

      <h3>{{ t('bases.title') }}</h3>
      <ul class="bases">
        <li v-for="base in state.location.value?.bases" :key="base.id">
          <input
            type="color"
            :value="base.color"
            :aria-label="t('bases.color')"
            @change="state.updateBase(base.id, base.name, ($event.target as HTMLInputElement).value)"
          />
          <div class="base">
            <input
              class="name"
              :value="base.name"
              :aria-label="t('bases.name')"
              @change="rename(base.id, base.color, $event)"
            />
            <span class="storage"
              >{{ storageLabel(base.storage) }} · <code>{{ base.path }}</code></span
            >
            <span v-if="base.problem" class="error">{{ t('bases.unavailable', { problem: base.problem }) }}</span>
          </div>
          <button type="button" :disabled="state.busy.value" @click="remove(base.id, base.name)">
            {{ t('bases.detach') }}
          </button>
        </li>
      </ul>

      <h3>{{ t('bases.add') }}</h3>
      <form class="add" @submit.prevent="state.addBase(newName, newColor, newPath)">
        <div class="row">
          <input v-model="newName" :placeholder="t('bases.namePlaceholder')" />
          <span class="swatches">
            <button
              v-for="color in BASE_COLORS"
              :key="color"
              type="button"
              class="swatch"
              :class="{ chosen: newColor === color }"
              :style="{ background: color }"
              :aria-label="color"
              @click="newColor = color"
            />
          </span>
        </div>
        <p v-if="(state.location.value?.drives.length ?? 0) > 0" class="hint">{{ t('bases.detected') }}</p>
        <ul v-if="(state.location.value?.drives.length ?? 0) > 0" class="drives">
          <li v-for="drive in state.location.value?.drives" :key="drive.path">
            <button type="button" class="link" @click="newPath = drive.path">
              <strong>{{ PROVIDER_NAMES[drive.provider] }}</strong> <code>{{ drive.path }}</code>
            </button>
          </li>
        </ul>
        <button type="button" class="primary pick" :disabled="state.busy.value" @click="pickFolder">
          <AppIcon name="folder" :size="14" /> {{ t('bases.pick') }}
        </button>
        <div class="row">
          <input v-model="newPath" class="path" :placeholder="t('bases.pathPlaceholder')" spellcheck="false" />
          <button type="submit" class="primary" :disabled="state.busy.value || !newName.trim() || !newPath.trim()">
            {{ t('bases.addButton') }}
          </button>
        </div>
      </form>
      <p class="hint">{{ t('bases.howItWorks') }}</p>

      <PreferencesSections :vaults="vaults" />
      <p v-if="state.error.value" class="error" role="alert">{{ state.error.value }}</p>
      <p v-if="state.busy.value" class="hint">{{ t('bases.checking') }}</p>
    </section>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.35);
}
.dialog {
  width: min(640px, calc(100vw - 32px));
  max-height: calc(100vh - 48px);
  overflow-y: auto;
  padding: 16px 20px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.25);
  outline: none;
  font-size: 13px;
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
h2 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 16px;
}
h3 {
  margin: 16px 0 6px;
  font-size: 11px;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: var(--muted);
}
.icon {
  display: grid;
  place-items: center;
  background: transparent;
  border-color: transparent;
}
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
.bases li {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
input[type='color'] {
  width: 28px;
  height: 28px;
  padding: 0;
  border: none;
  background: none;
}
.base {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.name {
  font: inherit;
  font-weight: 600;
  border: 1px solid transparent;
  border-radius: 4px;
  padding: 1px 4px;
  background: transparent;
  color: inherit;
}
.name:hover,
.name:focus {
  border-color: var(--border);
}
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
  word-break: break-all;
  color: var(--muted);
}
.add {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.row input {
  flex: 1;
  font: inherit;
  padding: 4px 8px;
}
.path {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace !important;
  font-size: 12px !important;
}
.swatches {
  display: flex;
  gap: 4px;
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
.drives .link {
  display: flex;
  gap: 8px;
  align-items: baseline;
  width: 100%;
  text-align: left;
  background: none;
  border: none;
  padding: 3px 0;
}
.hint {
  color: var(--muted);
  font-size: 12px;
}
.warning {
  color: var(--accent);
}
.error {
  color: var(--danger);
  font-size: 12px;
}
.storage {
  font-size: 11px;
  color: var(--muted);
}
.pick {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>
