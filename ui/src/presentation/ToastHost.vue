<script setup lang="ts">
/** Notification courante (`useToast`), avec bouton « Annuler » si l'action est annulable. */
import type { Toasts } from '../application/useToast'
import { t } from '../i18n'

defineProps<{ toasts: Toasts }>()
</script>

<template>
  <div v-if="toasts.current.value" class="toast" :class="toasts.current.value.kind" role="status">
    <span>{{ toasts.current.value.message }}</span>
    <button v-if="toasts.current.value.undo" @click="toasts.undo()">{{ t('common.undo') }}</button>
  </div>
</template>

<style scoped>
.toast {
  position: fixed;
  left: 50%;
  bottom: 40px;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 14px;
  max-width: 90vw;
  padding: 8px 14px;
  border-radius: 8px;
  background: var(--text);
  color: var(--bg);
  box-shadow: var(--shadow);
  z-index: 90;
}
.toast.error {
  background: var(--danger);
  color: var(--on-accent);
}
button {
  background: transparent;
  color: inherit;
  border-color: currentColor;
}
</style>
