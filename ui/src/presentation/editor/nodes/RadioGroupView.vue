<script setup lang="ts">
/**
 * Groupe radio : intitulé modifiable + options (un seul choix). Les options elles-mêmes sont
 * du texte éditable ; le clic sur un rond sélectionne l'option (voir `radioSelection` dans
 * extensions.ts) ; `Entrée` dans une option en ajoute une nouvelle.
 */
import { NodeViewContent, NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'

defineProps(nodeViewProps)
</script>

<template>
  <NodeViewWrapper as="fieldset" class="radio-group">
    <input
      class="label"
      contenteditable="false"
      :value="node.attrs.label ?? ''"
      placeholder="Intitulé du choix"
      @input="updateAttributes({ label: ($event.target as HTMLInputElement).value })"
      @keydown.stop
    />
    <NodeViewContent class="options" />
  </NodeViewWrapper>
</template>

<style scoped>
.radio-group {
  margin: 6px 0;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
}
.label {
  display: block;
  width: 100%;
  margin-bottom: 4px;
  font: 12px var(--font);
  color: var(--muted);
  background: transparent;
  border: none;
  border-bottom: 1px dashed transparent;
}
.label:hover,
.label:focus {
  border-bottom-color: var(--border);
  outline: none;
}
</style>
