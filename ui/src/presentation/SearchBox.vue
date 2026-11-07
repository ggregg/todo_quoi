<script setup lang="ts">
/** Champ de recherche de la barre haute. `Mod+F` y place le focus (géré par `App.vue`). */
import { ref } from 'vue'
import type { Search } from '../application/useSearch'
import { t } from '../i18n'

const props = defineProps<{ search: Search }>()
const input = ref<HTMLInputElement | null>(null)
const isMac = /Mac/.test(navigator.userAgent)

function onKeydown(event: KeyboardEvent): void {
  if (props.search.handleKey(event)) event.preventDefault()
  if (event.key === 'Escape') input.value?.blur()
}

defineExpose({ focus: () => input.value?.select() })
</script>

<template>
  <div class="search-box">
    <span class="icon" aria-hidden="true">🔍</span>
    <input
      ref="input"
      type="search"
      role="combobox"
      aria-controls="search-results"
      :aria-expanded="search.active.value"
      :aria-label="t('search.label')"
      :placeholder="t('search.placeholder', { shortcut: isMac ? '⌘F' : 'Ctrl+F' })"
      :value="search.query.value"
      spellcheck="false"
      @input="search.setQuery(($event.target as HTMLInputElement).value)"
      @keydown="onKeydown"
    />
  </div>
</template>

<style scoped>
.search-box {
  position: relative;
  margin-left: auto;
  width: min(360px, 40vw);
}
.icon {
  position: absolute;
  left: 8px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 12px;
  opacity: 0.6;
}
input {
  width: 100%;
  box-sizing: border-box;
  padding: 5px 8px 5px 28px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface-2);
  color: inherit;
  font: inherit;
}
input:focus {
  outline: 2px solid var(--accent);
  outline-offset: -1px;
}
</style>
