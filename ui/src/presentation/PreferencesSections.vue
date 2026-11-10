<script setup lang="ts">
/**
 * Sections de la fenêtre Réglages (spec §5.7) après les bases : apparence (thème, langue),
 * coffres (verrouillage automatique), corbeille, images, appareil. Chaque valeur s'enregistre
 * dès qu'elle change ; le cœur la vérifie (bornes, nom d'appareil valide).
 */
import { inject, onMounted, ref, watch } from 'vue'
import { THEMES, themeKey, type Theme } from '../application/useTheme'
import { preferencesKey } from '../application/usePreferences'
import type { VaultsState } from '../application/useVaults'
import { AUTO_LOCK_CHOICES } from '../ports/VaultGateway'
import { LOCALES, t } from '../i18n'

defineProps<{ vaults: VaultsState }>()

const theme = inject(themeKey) ?? missing('Thème')
const preferences = inject(preferencesKey) ?? missing('Préférences')

function missing(what: string): never {
  throw new Error(`${what} non fourni (voir main.ts)`)
}

const THEME_LABELS: Record<Theme, string> = {
  system: t('theme.system'),
  light: t('theme.light'),
  dark: t('theme.dark'),
}
const RETENTION_CHOICES = [7, 14, 30, 60, 90, 180, 365]
const SIDE_CHOICES = [1280, 1920, 2560, 4096]

/** Saisie du nom de l'appareil (enregistré quand on quitte le champ). */
const deviceName = ref('')
watch(
  () => preferences.preferences.value?.deviceName,
  (name) => (deviceName.value = name ?? ''),
  { immediate: true },
)
onMounted(() => void preferences.load())

function autoLockLabel(minutes: number): string {
  return minutes === 0 ? t('vault.autoLockNever') : t('vault.autoLockAfter', { minutes })
}

function saveDeviceName(): void {
  const current = preferences.preferences.value?.deviceName
  const name = deviceName.value.trim()
  if (!name || name === current) return
  if (window.confirm(t('settings.deviceRenameConfirm', { name }))) void preferences.update({ deviceName: name })
  else deviceName.value = current ?? ''
}

function updateImages(change: Partial<NonNullable<typeof preferences.preferences.value>['images']>): void {
  const images = preferences.preferences.value?.images
  if (images) void preferences.update({ images: { ...images, ...change } })
}
</script>

<template>
  <h3>{{ t('settings.appearance') }}</h3>
  <div class="field" role="radiogroup" :aria-label="t('settings.theme')">
    <span>{{ t('settings.theme') }}</span>
    <label v-for="value in THEMES" :key="value" class="choice">
      <input type="radio" name="theme" :checked="theme.theme.value === value" @change="theme.choose(value)" />
      {{ THEME_LABELS[value] }}
    </label>
  </div>
  <label class="field">
    <span>{{ t('settings.language') }}</span>
    <select disabled :title="t('settings.languageHint')">
      <option v-for="locale in LOCALES" :key="locale.code" :value="locale.code">{{ locale.name }}</option>
    </select>
  </label>

  <h3>{{ t('vault.autoLockTitle') }}</h3>
  <label class="field">
    <span>{{ t('vault.autoLockLabel') }}</span>
    <select
      :value="vaults.autoLockMinutes.value ?? ''"
      @change="vaults.setAutoLock(Number(($event.target as HTMLSelectElement).value))"
    >
      <option v-for="minutes in AUTO_LOCK_CHOICES" :key="minutes" :value="minutes">
        {{ autoLockLabel(minutes) }}
      </option>
    </select>
  </label>
  <p class="hint">{{ t('vault.autoLockHint') }}</p>

  <template v-if="preferences.preferences.value">
    <h3>{{ t('settings.trash') }}</h3>
    <label class="field">
      <span>{{ t('settings.trashRetention') }}</span>
      <select
        :value="preferences.preferences.value.trashRetentionDays"
        @change="preferences.update({ trashRetentionDays: Number(($event.target as HTMLSelectElement).value) })"
      >
        <option v-for="days in RETENTION_CHOICES" :key="days" :value="days">
          {{ t('settings.days', { days }) }}
        </option>
      </select>
    </label>

    <h3>{{ t('settings.images') }}</h3>
    <div class="field" role="radiogroup" :aria-label="t('settings.images')">
      <label class="choice">
        <input
          type="radio"
          name="images"
          :checked="preferences.preferences.value.images.handling === 'compress'"
          @change="updateImages({ handling: 'compress' })"
        />
        {{ t('settings.imagesCompress') }}
      </label>
      <label class="choice">
        <input
          type="radio"
          name="images"
          :checked="preferences.preferences.value.images.handling === 'original'"
          @change="updateImages({ handling: 'original' })"
        />
        {{ t('settings.imagesOriginal') }}
      </label>
    </div>
    <template v-if="preferences.preferences.value.images.handling === 'compress'">
      <label class="field">
        <span>{{ t('settings.imagesMaxSide') }}</span>
        <select
          :value="preferences.preferences.value.images.maxSide"
          @change="updateImages({ maxSide: Number(($event.target as HTMLSelectElement).value) })"
        >
          <option v-for="side in SIDE_CHOICES" :key="side" :value="side">{{ side }} px</option>
        </select>
      </label>
      <label class="field">
        <span>{{ t('settings.imagesQuality', { quality: preferences.preferences.value.images.quality }) }}</span>
        <input
          type="range"
          min="50"
          max="100"
          step="5"
          :value="preferences.preferences.value.images.quality"
          @change="updateImages({ quality: Number(($event.target as HTMLInputElement).value) })"
        />
      </label>
    </template>
    <p class="hint">{{ t('settings.imagesHint') }}</p>

    <h3>{{ t('settings.device') }}</h3>
    <label class="field">
      <span>{{ t('settings.deviceName') }}</span>
      <input v-model="deviceName" maxlength="40" @change="saveDeviceName" @keydown.enter.prevent="saveDeviceName" />
    </label>
    <p class="hint">
      {{ t('settings.deviceHint', { id: preferences.preferences.value.deviceId }) }}
    </p>
  </template>
  <p v-if="preferences.error.value" class="error" role="alert">{{ preferences.error.value }}</p>
</template>

<style scoped>
h3 {
  margin: 16px 0 6px;
  font-size: 11px;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: var(--muted);
}
.field {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 12px;
  margin: 4px 0;
  font-size: 13px;
}
.field > span:first-child {
  min-width: 190px;
}
.choice {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.field input:not([type='radio']),
.field select {
  font: inherit;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 3px 6px;
}
.hint {
  margin: 2px 0 0;
  color: var(--muted);
  font-size: 12px;
}
.error {
  color: var(--danger);
  font-size: 12px;
}
</style>
