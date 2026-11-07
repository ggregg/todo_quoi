<script setup lang="ts">
/**
 * Image en plein écran, au-dessus de toute la fenêtre.
 * `Échap` ou clic sur le fond : fermer · `←` `→` : image précédente / suivante de la note ·
 * clic sur l'image : basculer entre « ajustée à l'écran » et « taille réelle » (défilement).
 */
import { nextTick, ref, watch } from 'vue'
import { useImageViewer } from '../application/useImageViewer'
import { t } from '../i18n'
import AppIcon from './AppIcon.vue'

const viewer = useImageViewer()
const actualSize = ref(false)
const dialog = ref<HTMLElement | null>(null)
let previousFocus: HTMLElement | null = null

watch(viewer.current, async (image, before) => {
  actualSize.value = false
  if (image && !before) {
    previousFocus = document.activeElement as HTMLElement | null
    await nextTick()
    dialog.value?.focus()
  }
  if (!image && before) previousFocus?.focus()
})

function onKeydown(event: KeyboardEvent): void {
  const actions: Record<string, () => void> = {
    Escape: viewer.close,
    ArrowRight: () => viewer.go(1),
    ArrowLeft: () => viewer.go(-1),
  }
  const action = actions[event.key]
  if (!action) return
  event.preventDefault()
  event.stopPropagation()
  action()
}
</script>

<template>
  <div
    v-if="viewer.current.value"
    ref="dialog"
    class="viewer"
    role="dialog"
    aria-modal="true"
    :aria-label="t('viewer.label')"
    tabindex="-1"
    @keydown="onKeydown"
    @click.self="viewer.close()"
  >
    <div class="stage" :class="{ actual: actualSize }" @click.self="viewer.close()">
      <img
        :src="viewer.current.value.url"
        :alt="viewer.current.value.alt"
        :title="t(actualSize ? 'viewer.fit' : 'viewer.actualSize')"
        @click="actualSize = !actualSize"
      />
    </div>
    <div class="bar">
      <span class="caption">{{ viewer.current.value.alt }}</span>
      <span v-if="viewer.count.value > 1" class="position">
        <button type="button" :aria-label="t('viewer.previous')" @click="viewer.go(-1)">
          <AppIcon name="chevronRight" class="flip" />
        </button>
        {{ viewer.index.value + 1 }} / {{ viewer.count.value }}
        <button type="button" :aria-label="t('viewer.next')" @click="viewer.go(1)">
          <AppIcon name="chevronRight" />
        </button>
      </span>
      <button type="button" :aria-label="t('viewer.close')" @click="viewer.close()">
        <AppIcon name="close" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.viewer {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  flex-direction: column;
  background: rgba(10, 10, 12, 0.92);
  color: #eee;
  outline: none;
}
.stage {
  flex: 1;
  min-height: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  overflow: hidden;
}
.stage img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  cursor: zoom-in;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
}
.stage.actual {
  overflow: auto;
  place-items: start center;
}
.stage.actual img {
  max-width: none;
  max-height: none;
  cursor: zoom-out;
}
.bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  font-size: 13px;
}
.caption {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  opacity: 0.8;
}
.position {
  display: flex;
  align-items: center;
  gap: 6px;
  font-variant-numeric: tabular-nums;
}
.bar button {
  display: inline-grid;
  place-items: center;
  padding: 4px 6px;
  color: inherit;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 6px;
}
.bar button:hover {
  background: rgba(255, 255, 255, 0.18);
}
.flip {
  transform: scaleX(-1);
}
</style>
