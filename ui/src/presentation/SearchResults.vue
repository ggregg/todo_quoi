<script setup lang="ts">
/**
 * Résultats de recherche : liste plate avec chemin (fil d'Ariane) et extrait surligné (spec §5.6).
 * Affichée à la place de l'arbre tant qu'une recherche est en cours.
 */
import { nextTick, watch } from 'vue'
import type { SearchHit } from '../domain/task'
import { highlightParts, type Search } from '../application/useSearch'
import type { BaseInfo } from '../ports/SettingsGateway'
import { t } from '../i18n'

/** `showBase` : plusieurs bases affichées → chaque résultat montre la sienne. */
const props = defineProps<{ search: Search; baseInfo: (id: string) => BaseInfo; showBase: boolean }>()
const emit = defineEmits<{ open: [hit: SearchHit] }>()

// Le résultat actif reste visible quand on se déplace au clavier.
watch(
  () => props.search.activeIndex.value,
  async (index) => {
    await nextTick()
    document.getElementById(`search-hit-${index}`)?.scrollIntoView({ block: 'nearest' })
  },
)
</script>

<template>
  <div class="results">
    <p class="summary">
      <template v-if="search.searching.value && search.results.value.length === 0">{{
        t('search.searching')
      }}</template>
      <template v-else-if="search.results.value.length === 0">{{ t('search.empty') }}</template>
      <template v-else>{{ t('search.count', { count: search.results.value.length }) }}</template>
    </p>
    <ul id="search-results" role="listbox" :aria-label="t('search.label')">
      <li
        v-for="(hit, index) in search.results.value"
        :id="`search-hit-${index}`"
        :key="hit.id"
        role="option"
        :aria-selected="index === search.activeIndex.value"
        :class="{ active: index === search.activeIndex.value, done: hit.done }"
        @mousedown.prevent
        @mouseenter="search.setActive(index)"
        @click="emit('open', hit)"
      >
        <span v-if="hit.path.length || showBase" class="path">
          <span v-if="showBase" class="base" :style="{ '--base-color': baseInfo(hit.base).color }">{{
            baseInfo(hit.base).name
          }}</span>
          {{ hit.path.join(' › ') }}
        </span>
        <span class="title">
          <template v-for="(part, partIndex) in highlightParts(hit.title, hit.titleHighlights)" :key="partIndex">
            <mark v-if="part.hit">{{ part.text }}</mark>
            <template v-else>{{ part.text }}</template>
          </template>
        </span>
        <span v-if="hit.snippet" class="snippet">
          <template
            v-for="(part, partIndex) in highlightParts(hit.snippet.text, hit.snippet.highlights)"
            :key="partIndex"
          >
            <mark v-if="part.hit">{{ part.text }}</mark>
            <template v-else>{{ part.text }}</template>
          </template>
        </span>
      </li>
    </ul>
    <p class="hint">{{ t('search.hint') }}</p>
  </div>
</template>

<style scoped>
.results {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.summary,
.hint {
  margin: 0;
  padding: 6px 12px;
  font-size: 12px;
  color: var(--muted);
}
.hint {
  border-top: 1px solid var(--border);
}
ul {
  flex: 1;
  overflow-y: auto;
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 12px;
  cursor: pointer;
  border-left: 3px solid transparent;
}
li.active {
  background: var(--surface-2);
  border-left-color: var(--accent);
}
.path {
  font-size: 11px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.title {
  font-weight: 500;
}
li.done .title {
  text-decoration: line-through;
  color: var(--muted);
}
.snippet {
  font-size: 12px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
mark {
  background: color-mix(in srgb, var(--accent) 28%, transparent);
  color: inherit;
  border-radius: 2px;
}
.base {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-right: 6px;
  font-weight: 600;
  color: var(--base-color);
}
</style>
