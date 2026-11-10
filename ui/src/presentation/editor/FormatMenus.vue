<script setup lang="ts">
/**
 * Deux boutons de la barre d'outils, chacun avec son petit menu (ADR-0012) :
 * - **Aligner** : à gauche, centré, à droite (paragraphe, titre, colonne de tableau ; désactivé
 *   dans une liste) ;
 * - **Couleur du texte** : palette fixe, et « Par défaut ».
 * Échap ou un clic ailleurs referme le menu.
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import type { useNoteFormat } from './useNoteFormat'
import { TEXT_ALIGNMENTS, TEXT_COLORS, type TextAlignment, type TextColor } from './formatting'
import AppIcon, { type IconName } from '../AppIcon.vue'
import { t } from '../../i18n'

const props = defineProps<{ format: ReturnType<typeof useNoteFormat> }>()

const open = ref<'align' | 'color' | null>(null)
const root = ref<HTMLElement | null>(null)

const ALIGN_ICONS: Record<TextAlignment, IconName> = {
  left: 'alignLeft',
  center: 'alignCenter',
  right: 'alignRight',
}
const ALIGN_LABELS: Record<TextAlignment, string> = {
  left: t('format.alignLeft'),
  center: t('format.alignCenter'),
  right: t('format.alignRight'),
}
const COLOR_LABELS: Record<TextColor, string> = {
  red: t('format.red'),
  orange: t('format.orange'),
  green: t('format.green'),
  blue: t('format.blue'),
  purple: t('format.purple'),
  gray: t('format.gray'),
}

const alignment = computed(() => props.format.currentAlignment() ?? 'left')
const color = computed(() => props.format.currentColor())

async function toggle(menu: 'align' | 'color'): Promise<void> {
  open.value = open.value === menu ? null : menu
  await nextTick()
  root.value?.querySelector<HTMLButtonElement>('.menu button')?.focus()
}

function chooseAlignment(value: TextAlignment): void {
  open.value = null
  props.format.align(value === 'left' ? null : value)
}

function chooseColor(value: TextColor | null): void {
  open.value = null
  props.format.color(value)
}

function closeOnOutsideClick(event: MouseEvent): void {
  if (open.value && !root.value?.contains(event.target as Node)) open.value = null
}
onMounted(() => window.addEventListener('mousedown', closeOnOutsideClick))
onBeforeUnmount(() => window.removeEventListener('mousedown', closeOnOutsideClick))
</script>

<template>
  <span ref="root" class="format-menus" @keydown.esc.stop="open = null">
    <span class="anchor">
      <button
        type="button"
        :title="format.canAlign() ? t('format.align') : t('format.alignInList')"
        :aria-label="t('format.align')"
        aria-haspopup="menu"
        :aria-expanded="open === 'align'"
        :disabled="!format.canAlign()"
        :class="{ active: alignment !== 'left' }"
        @mousedown.prevent
        @click="toggle('align')"
      >
        <AppIcon :name="ALIGN_ICONS[alignment]" />
      </button>
      <span v-if="open === 'align'" class="menu" role="menu" :aria-label="t('format.align')">
        <button
          v-for="value in TEXT_ALIGNMENTS"
          :key="value"
          type="button"
          role="menuitemradio"
          :aria-checked="alignment === value"
          :class="{ chosen: alignment === value }"
          @mousedown.prevent
          @click="chooseAlignment(value)"
        >
          <AppIcon :name="ALIGN_ICONS[value]" :size="16" /> {{ ALIGN_LABELS[value] }}
        </button>
      </span>
    </span>
    <span class="anchor">
      <button
        type="button"
        :title="t('format.color')"
        :aria-label="t('format.color')"
        aria-haspopup="menu"
        :aria-expanded="open === 'color'"
        :class="{ active: color !== null }"
        @mousedown.prevent
        @click="toggle('color')"
      >
        <span class="color-icon">
          <AppIcon name="textColor" />
          <span class="bar" :data-color="color ?? undefined" />
        </span>
      </button>
      <span v-if="open === 'color'" class="menu" role="menu" :aria-label="t('format.color')">
        <button
          v-for="value in TEXT_COLORS"
          :key="value"
          type="button"
          role="menuitemradio"
          :aria-checked="color === value"
          :class="{ chosen: color === value }"
          @mousedown.prevent
          @click="chooseColor(value)"
        >
          <span class="swatch" :data-color="value" /> {{ COLOR_LABELS[value] }}
        </button>
        <button
          type="button"
          role="menuitemradio"
          :aria-checked="color === null"
          :class="{ chosen: color === null }"
          @mousedown.prevent
          @click="chooseColor(null)"
        >
          <span class="swatch default" /> {{ t('format.defaultColor') }}
        </button>
      </span>
    </span>
  </span>
</template>

<style scoped>
.format-menus {
  display: contents;
}
.anchor {
  position: relative;
  display: inline-flex;
}
/* Même allure que les boutons de la barre d'outils (`NoteEditor.vue`). */
.anchor > button {
  display: inline-grid;
  place-items: center;
  height: 28px;
  min-width: 28px;
  padding: 2px 6px;
  font-size: 12px;
  color: inherit;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 6px;
  cursor: pointer;
}
.anchor > button:hover:not(:disabled) {
  background: var(--hover);
}
.anchor > button.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.anchor > button:disabled {
  opacity: 0.4;
  cursor: default;
}
.color-icon {
  position: relative;
  display: inline-grid;
  place-items: center;
}
.bar {
  position: absolute;
  left: 3px;
  right: 3px;
  bottom: -1px;
  height: 3px;
  border-radius: 2px;
  background: var(--text);
}
.menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  z-index: 20;
  min-width: 170px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: var(--shadow);
}
.menu button {
  display: flex;
  align-items: center;
  gap: 8px;
  height: auto;
  padding: 5px 10px;
  border: none;
  border-radius: 5px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
}
.menu button:hover,
.menu button:focus-visible {
  background: var(--hover);
  outline: none;
}
.menu button.chosen {
  font-weight: 600;
}
.swatch {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--text);
}
.swatch.default {
  background: none;
  border: 1.5px solid var(--muted);
}
[data-color='red'] {
  background: var(--text-red);
}
[data-color='orange'] {
  background: var(--text-orange);
}
[data-color='green'] {
  background: var(--text-green);
}
[data-color='blue'] {
  background: var(--text-blue);
}
[data-color='purple'] {
  background: var(--text-purple);
}
[data-color='gray'] {
  background: var(--text-gray);
}
</style>
