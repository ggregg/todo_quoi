<script setup lang="ts">
/** Bloc « message audio » : lecteur natif + vitesse de lecture (spec §6.2). */
import { computed, ref } from 'vue'
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'
import type { MediaGateway } from '../../../ports/MediaGateway'
import { formatDuration } from '../../../application/mediaRules'

const props = defineProps(nodeViewProps)
const media = computed(() => props.extension.options.media as MediaGateway)
const src = computed(() => media.value.url(String(props.node.attrs.src ?? '')))
const duration = computed(() => Number(props.node.attrs.duration ?? 0))

const player = ref<HTMLAudioElement | null>(null)
const speeds = [1, 1.5, 2]
const speed = ref(1)

function nextSpeed(): void {
  speed.value = speeds[(speeds.indexOf(speed.value) + 1) % speeds.length] ?? 1
  if (player.value) player.value.playbackRate = speed.value
}
</script>

<template>
  <NodeViewWrapper class="audio-block" :class="{ selected }" contenteditable="false" data-drag-handle>
    <span class="icon">🎙</span>
    <audio ref="player" :src="src" controls preload="metadata" />
    <button type="button" class="speed" title="Vitesse de lecture" @click="nextSpeed">
      ×{{ String(speed).replace('.', ',') }}
    </button>
    <span v-if="duration > 0" class="duration">{{ formatDuration(duration) }}</span>
  </NodeViewWrapper>
</template>

<style scoped>
.audio-block {
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: 460px;
  margin: 6px 0;
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: 20px;
  background: var(--surface-2);
}
.audio-block.selected {
  outline: 2px solid var(--accent);
}
audio {
  height: 32px;
  flex: 1;
  min-width: 0;
}
.speed {
  padding: 0 6px;
  font-size: 11px;
}
.duration {
  font:
    11px ui-monospace,
    Menlo,
    Consolas,
    monospace;
  color: var(--muted);
}
</style>
