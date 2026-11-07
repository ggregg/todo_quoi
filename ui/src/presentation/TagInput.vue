<script setup lang="ts">
/**
 * Saisie d'étiquette avec suggestions (étiquettes déjà utilisées, les plus fréquentes d'abord).
 * `↑` `↓` pour choisir, `Entrée` ou `Tab` pour valider, `Échap` pour fermer la liste.
 * `Entrée` sans suggestion choisie ajoute le texte tapé (nouvelle étiquette).
 */
import { computed, ref } from 'vue'
import { suggestTags } from '../application/tagSuggestions'
import { t } from '../i18n'

const props = defineProps<{ usage: ReadonlyMap<string, number>; exclude: readonly string[] }>()
const emit = defineEmits<{ add: [tag: string] }>()

const text = ref('')
const open = ref(false)
const active = ref(-1)
const suggestions = computed(() => (open.value ? suggestTags(props.usage, text.value, props.exclude) : []))

function choose(tag: string): void {
  if (tag.trim()) emit('add', tag)
  text.value = ''
  active.value = -1
}

function onKeydown(event: KeyboardEvent): void {
  const count = suggestions.value.length
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    open.value = true
    if (count > 0)
      active.value = (active.value + (event.key === 'ArrowDown' ? 1 : count - 1 + (active.value < 0 ? 1 : 0))) % count
  } else if (event.key === 'Enter' || (event.key === 'Tab' && active.value >= 0)) {
    event.preventDefault()
    choose(suggestions.value[active.value] ?? text.value)
  } else if (event.key === 'Escape' && open.value) {
    event.stopPropagation()
    open.value = false
  }
}

function onInput(): void {
  open.value = true
  active.value = -1
}
</script>

<template>
  <span class="tag-input">
    <input
      v-model="text"
      role="combobox"
      aria-autocomplete="list"
      :aria-expanded="suggestions.length > 0"
      :placeholder="t('detail.addTag')"
      @focus="open = true"
      @blur="open = false"
      @input="onInput"
      @keydown="onKeydown"
    />
    <ul v-if="suggestions.length > 0" role="listbox">
      <li
        v-for="(tag, index) in suggestions"
        :key="tag"
        role="option"
        :aria-selected="index === active"
        :class="{ active: index === active }"
        @mousedown.prevent="choose(tag)"
      >
        <span>#{{ tag }}</span>
        <span class="count">{{ usage.get(tag) }}</span>
      </li>
    </ul>
  </span>
</template>

<style scoped>
.tag-input {
  position: relative;
}
input {
  font: inherit;
  font-size: 12px;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 2px 8px;
  width: 140px;
}
ul {
  position: absolute;
  left: 0;
  top: calc(100% + 3px);
  z-index: 20;
  min-width: 180px;
  margin: 0;
  padding: 4px;
  list-style: none;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.16);
}
li {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 3px 8px;
  border-radius: 5px;
  font-size: 12px;
  cursor: pointer;
}
li.active,
li:hover {
  background: var(--accent-soft);
  color: var(--accent);
}
.count {
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}
</style>
