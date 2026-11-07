<script setup lang="ts">
/**
 * Image redimensionnable : poignée dans le coin inférieur droit.
 * - glisser : change la largeur (proportions conservées, jamais plus large que la note) ;
 * - double-clic sur la poignée : revient à la taille d'origine ;
 * - double-clic sur l'image, ou bouton au survol : plein écran (toutes les images de la note,
 *   `←` `→` pour passer de l'une à l'autre).
 * La largeur est enregistrée dans l'attribut `width`, écrit `{width=480}` dans le Markdown.
 */
import { computed, ref } from 'vue'
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'
import type { MediaGateway } from '../../../ports/MediaGateway'
import { openImageViewer } from '../../../application/useImageViewer'
import { t } from '../../../i18n'
import AppIcon from '../../AppIcon.vue'

const MIN_WIDTH = 48

const props = defineProps(nodeViewProps)
const media = computed(() => props.extension.options.media as MediaGateway)
const src = computed(() => media.value.url(String(props.node.attrs.src ?? '')))

const image = ref<HTMLImageElement | null>(null)
/** Largeur pendant le glissement (avant enregistrement), sinon `null`. */
const draggingWidth = ref<number | null>(null)
const width = computed(() => draggingWidth.value ?? (props.node.attrs.width as number | null))

/** Plein écran : toutes les images de la note, en commençant par celle-ci. */
function showFullscreen(): void {
  const images: { url: string; alt: string }[] = []
  let start = 0
  const position = typeof props.getPos === 'function' ? props.getPos() : undefined
  props.editor.state.doc.descendants((node, nodePosition) => {
    if (node.type.name !== 'image') return
    if (nodePosition === position) start = images.length
    images.push({ url: media.value.url(String(node.attrs.src ?? '')), alt: String(node.attrs.alt ?? '') })
  })
  openImageViewer(images, start)
}

function startResize(event: PointerEvent): void {
  const element = image.value
  if (!element) return
  event.preventDefault()
  const handle = event.currentTarget as HTMLElement
  handle.setPointerCapture(event.pointerId)
  const startX = event.clientX
  const startWidth = element.getBoundingClientRect().width
  const maxWidth = element.closest('.ProseMirror')?.clientWidth ?? Number.POSITIVE_INFINITY

  const move = (moveEvent: PointerEvent) => {
    draggingWidth.value = Math.round(Math.min(maxWidth, Math.max(MIN_WIDTH, startWidth + moveEvent.clientX - startX)))
  }
  const stop = () => {
    handle.removeEventListener('pointermove', move)
    handle.removeEventListener('pointerup', stop)
    if (draggingWidth.value !== null) props.updateAttributes({ width: draggingWidth.value, height: null })
    draggingWidth.value = null
  }
  handle.addEventListener('pointermove', move)
  handle.addEventListener('pointerup', stop)
}

function resetSize(): void {
  props.updateAttributes({ width: null, height: null })
}
</script>

<template>
  <NodeViewWrapper class="image-block" :class="{ selected }" data-drag-handle>
    <span class="frame">
      <img
        ref="image"
        :src="src"
        :alt="node.attrs.alt ?? ''"
        :title="node.attrs.title ?? undefined"
        :style="{ width: width ? `${width}px` : undefined }"
        draggable="false"
        @dblclick="showFullscreen"
      />
      <button
        type="button"
        class="fullscreen"
        :title="t('editor.imageFullscreen')"
        :aria-label="t('editor.imageFullscreen')"
        @mousedown.prevent
        @click="showFullscreen"
      >
        <AppIcon name="expand" :size="14" />
      </button>
      <span
        class="handle"
        title="Glisser pour redimensionner · double-clic : taille d’origine"
        @pointerdown="startResize"
        @dblclick="resetSize"
      />
      <span v-if="draggingWidth !== null" class="size">{{ draggingWidth }} px</span>
    </span>
  </NodeViewWrapper>
</template>

<style scoped>
.image-block {
  margin: 6px 0;
  line-height: 0;
}
.frame {
  position: relative;
  display: inline-block;
  max-width: 100%;
}
img {
  display: block;
  max-width: 100%;
  height: auto;
  border-radius: 6px;
}
.selected img {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.handle {
  position: absolute;
  right: -5px;
  bottom: -5px;
  width: 14px;
  height: 14px;
  border: 2px solid var(--surface);
  border-radius: 3px;
  background: var(--accent);
  cursor: nwse-resize;
  opacity: 0;
  transition: opacity 0.15s;
}
.frame:hover .handle,
.selected .handle,
.frame:hover .fullscreen,
.selected .fullscreen {
  opacity: 1;
}
.fullscreen {
  position: absolute;
  top: 6px;
  right: 6px;
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s;
}
.size {
  position: absolute;
  right: 6px;
  top: 6px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.65);
  color: #fff;
  font:
    11px/1.4 ui-monospace,
    Menlo,
    Consolas,
    monospace;
}
</style>
