<script setup lang="ts">
/** Bloc « pièce jointe » : nom, taille, ouvrir avec l'application par défaut, révéler (spec §6.3). */
import AppIcon from '../../AppIcon.vue'
import { computed } from 'vue'
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'
import type { MediaGateway } from '../../../ports/MediaGateway'
import { formatSize } from '../../../application/mediaRules'

const props = defineProps(nodeViewProps)
const media = computed(() => props.extension.options.media as MediaGateway)
const src = computed(() => String(props.node.attrs.src ?? ''))
const size = computed(() => Number(props.node.attrs.size ?? 0))
const isMac = /Mac/.test(navigator.userAgent)
</script>

<template>
  <NodeViewWrapper class="attachment" :class="{ selected }" contenteditable="false" data-drag-handle>
    <span class="icon"><AppIcon name="file" :size="18" /></span>
    <span class="name" :title="node.attrs.name ?? ''">{{ node.attrs.name ?? 'Pièce jointe' }}</span>
    <span v-if="size > 0" class="size">{{ formatSize(size) }}</span>
    <button type="button" @click="media.open(src)">Ouvrir</button>
    <button type="button" @click="media.reveal(src)">
      {{ isMac ? 'Révéler dans le Finder' : 'Afficher dans l’Explorateur' }}
    </button>
  </NodeViewWrapper>
</template>

<style scoped>
.attachment {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 8px;
  max-width: 100%;
  box-sizing: border-box;
  margin: 6px 0;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-2);
}
.attachment.selected {
  outline: 2px solid var(--accent);
}
/* Nom tronqué par « … » (nom complet au survol) : un nom long ne déborde jamais de la note. */
.name {
  flex: 1 1 120px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}
.size {
  font-size: 12px;
  color: var(--muted);
}
button {
  padding: 1px 8px;
  font-size: 12px;
}
</style>
