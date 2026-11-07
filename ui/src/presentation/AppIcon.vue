<script setup lang="ts">
/**
 * Icônes de l'interface : un petit jeu **maison** de tracés SVG (24 × 24, trait de 1,8, bouts
 * arrondis), dessinés pour TodoQuoi. Pas de fonte d'icônes ni de bibliothèque (ADR-0006) : un
 * SVG en ligne prend la couleur du texte (`currentColor`), donc du thème, et reste net à toute
 * taille.
 *
 * Usage : `<AppIcon name="search" />` (décoratif, `aria-hidden`) ou `<AppIcon name="close" :label="…" />`.
 * Ajouter une icône : une entrée dans `ICONS` (contenu de la balise `<svg>`).
 */
import { computed } from 'vue'

const ICONS = {
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  chevronRight: '<path d="M9 6l6 6-6 6"/>',
  chevronDown: '<path d="M6 9l6 6 6-6"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  close: '<path d="M6 6l12 12M18 6L6 18"/>',
  search: '<circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/>',
  sliders: '<path d="M4 7h10M18 7h2M4 17h4M12 17h8"/><circle cx="16" cy="7" r="2"/><circle cx="10" cy="17" r="2"/>',
  note: '<rect x="5" y="3" width="14" height="18" rx="2"/><path d="M9 8h6M9 12h6M9 16h4"/>',
  image: '<rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="M21 16l-5-5-8 8"/>',
  mic: '<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21"/>',
  paperclip:
    '<path d="M20 11.5l-7.8 7.8a5 5 0 0 1-7.1-7.1l8.5-8.5a3.3 3.3 0 0 1 4.7 4.7L9.9 16.8a1.7 1.7 0 0 1-2.4-2.4l7.4-7.4"/>',
  file: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  detach: '<rect x="3" y="7" width="14" height="14" rx="2"/><path d="M14 3h7v7M21 3l-9 9"/>',
  dock: '<path d="M4 4v16M20 12H9M13 8l-4 4 4 4"/>',
  external: '<path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>',
  edit: '<path d="M4 20h4L19 9l-4-4L4 16z"/>',
  code: '<path d="M8 8l-4 4 4 4M16 8l4 4-4 4"/>',
  codeBlock: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M10 10l-2 2 2 2M14 10l2 2-2 2"/>',
  bulletList:
    '<path d="M9 7h11M9 12h11M9 17h11"/><circle cx="4.5" cy="7" r="1"/><circle cx="4.5" cy="12" r="1"/><circle cx="4.5" cy="17" r="1"/>',
  orderedList: '<path d="M10 7h10M10 12h10M10 17h10M4 5.5h1.5V9M3.5 14h3l-3 3.5h3"/>',
  taskList: '<rect x="3" y="4.5" width="5" height="5" rx="1"/><path d="M3.5 15.5l1.5 1.5 2.5-2.5M11 7h9M11 16h9"/>',
  quote: '<path d="M6 7h4v4c0 3-1.3 4.7-4 5.5M14 7h4v4c0 3-1.3 4.7-4 5.5"/>',
  table: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M3 15h18M10 4v16"/>',
  link: '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>',
  radio: '<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="3.2" fill="currentColor" stroke="none"/>',
  undo: '<path d="M9 14L4 9l5-5M4 9h10a6 6 0 0 1 0 12h-3"/>',
  redo: '<path d="M15 14l5-5-5-5M20 9H10a6 6 0 0 0 0 12h3"/>',
  stop: '<rect x="7" y="7" width="10" height="10" rx="1.5" fill="currentColor" stroke="none"/>',
  play: '<path d="M8 5.5v13l10.5-6.5z" fill="currentColor" stroke="none"/>',
  pause: '<path d="M8 5v14M16 5v14"/>',
  grip: '<circle cx="9" cy="7" r="1"/><circle cx="15" cy="7" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="9" cy="17" r="1"/><circle cx="15" cy="17" r="1"/>',
  expand: '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3"/>',
  logo: '<rect x="3" y="3" width="18" height="18" rx="5"/><path d="M7.5 12.5l3 3 6-6.5"/>',
} as const

export type IconName = keyof typeof ICONS

const props = withDefaults(defineProps<{ name: IconName; size?: number; label?: string }>(), { size: 16, label: '' })
const body = computed(() => ICONS[props.name])
</script>

<template>
  <!-- eslint-disable vue/no-v-html -- contenu constant défini ci-dessus, jamais de saisie -->
  <svg
    class="app-icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    :role="label ? 'img' : undefined"
    :aria-label="label || undefined"
    :aria-hidden="label ? undefined : 'true'"
    v-html="body"
  />
</template>

<style scoped>
.app-icon {
  flex: none;
  vertical-align: -0.15em;
}
</style>
