<script setup lang="ts">
/**
 * Bouton « thème » de la barre haute (spec §5.1) : système → clair → sombre → système…
 * L'icône montre le thème en cours ; le titre dit lequel viendra au clic.
 */
import { computed, inject } from 'vue'
import { THEMES, themeKey, type Theme } from '../application/useTheme'
import { t } from '../i18n'
import AppIcon, { type IconName } from './AppIcon.vue'

const theme = inject(themeKey)
if (!theme) throw new Error('Thème non fourni (voir main.ts)')

const ICONS: Record<Theme, IconName> = { system: 'themeSystem', light: 'sun', dark: 'moon' }
const LABELS: Record<Theme, string> = {
  system: t('theme.system'),
  light: t('theme.light'),
  dark: t('theme.dark'),
}

const next = computed(() => THEMES[(THEMES.indexOf(theme.theme.value) + 1) % THEMES.length]!)
const title = computed(() => t('theme.button', { current: LABELS[theme.theme.value], next: LABELS[next.value] }))
</script>

<template>
  <button type="button" class="icon theme-button" :title="title" :aria-label="title" @click="theme.choose(next)">
    <AppIcon :name="ICONS[theme.theme.value]" />
  </button>
</template>
