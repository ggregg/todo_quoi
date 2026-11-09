<script setup lang="ts">
/**
 * Bloc « fichier lié » (`::filelink{path name device}`) : le fichier reste où il est, la note
 * retient son chemin et l'appareil qui l'a lié. Ouvrir / révéler seulement là où le cœur le
 * permet ; ailleurs le bloc reste visible, grisé, avec la raison (autre appareil, introuvable).
 */
import AppIcon from '../../AppIcon.vue'
import { computed, onMounted, ref } from 'vue'
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'
import type { FileAttachments, LinkState } from '../../../ports/FileAttachments'
import { isUiError } from '../../../domain/task'
import { t } from '../../../i18n'

const props = defineProps(nodeViewProps)
const files = computed(() => props.extension.options.files as FileAttachments)
const link = computed(() => ({
  path: String(props.node.attrs.path ?? ''),
  device: String(props.node.attrs.device ?? ''),
}))
const name = computed(() => String(props.node.attrs.name ?? '') || t('link.label'))
const isMac = /Mac/.test(navigator.userAgent)
const state = ref<LinkState | null>(null)
const problem = ref<string | null>(null)

/** Explication quand le lien est inactif (ou seulement révélable). */
const reason = computed(() => {
  if (state.value === 'otherDevice') return t('link.otherDevice', { device: link.value.device })
  if (state.value === 'missing') return t('link.missing')
  if (state.value === 'revealOnly') return t('link.revealOnly')
  return null
})

async function refresh(): Promise<void> {
  try {
    state.value = await files.value.linkState(link.value)
  } catch {
    state.value = 'missing'
  }
}

async function attempt(action: () => Promise<void>): Promise<void> {
  problem.value = null
  try {
    await action()
  } catch (error) {
    problem.value = isUiError(error) ? error.message : String(error)
    await refresh() // le fichier a peut-être disparu depuis l'affichage
  }
}

onMounted(refresh)
</script>

<template>
  <NodeViewWrapper
    class="file-link"
    :class="{ selected, inactive: state === 'otherDevice' || state === 'missing' }"
    contenteditable="false"
    data-drag-handle
    :title="link.path"
  >
    <span class="icon"><AppIcon name="link" :size="18" /></span>
    <span class="name">{{ name }}</span>
    <span class="device">{{ link.device }}</span>
    <template v-if="state === 'available' || state === 'revealOnly'">
      <button v-if="state === 'available'" type="button" @click="attempt(() => files.openLinked(link))">
        {{ t('link.open') }}
      </button>
      <button type="button" @click="attempt(() => files.revealLinked(link))">
        {{ isMac ? 'Révéler dans le Finder' : 'Afficher dans l’Explorateur' }}
      </button>
    </template>
    <span v-if="reason" class="reason" :class="{ warning: state === 'missing' }">{{ reason }}</span>
    <span v-if="problem" class="reason warning" role="alert">{{ problem }}</span>
  </NodeViewWrapper>
</template>

<style scoped>
.file-link {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 8px;
  max-width: 100%;
  box-sizing: border-box;
  margin: 6px 0;
  padding: 6px 12px;
  border: 1px dashed var(--border);
  border-radius: 8px;
  background: var(--surface-2);
}
.file-link.selected {
  outline: 2px solid var(--accent);
}
.file-link.inactive .icon,
.file-link.inactive .name {
  opacity: 0.55;
}
.name {
  flex: 1 1 120px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}
.device {
  font-size: 11px;
  padding: 0 6px;
  border-radius: 999px;
  background: var(--surface);
  color: var(--muted);
}
.reason {
  flex-basis: 100%;
  font-size: 12px;
  color: var(--muted);
}
.reason.warning {
  color: var(--danger);
}
button {
  padding: 1px 8px;
}
</style>
