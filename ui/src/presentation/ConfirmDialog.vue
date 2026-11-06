<script setup lang="ts">
/** Affiche la question courante de `useConfirm` ; Échap ou clic à l'extérieur = « annuler ». */
import { nextTick, ref, watch } from 'vue'
import type { Confirm } from '../application/useConfirm'

const props = defineProps<{ confirm: Confirm }>()
const dialog = ref<HTMLElement | null>(null)
/** Élément qui avait le focus avant l'ouverture (la liste) : on le lui rend à la fermeture. */
let previouslyFocused: HTMLElement | null = null

watch(
  () => props.confirm.current.value,
  async (pending, before) => {
    if (!pending) {
      previouslyFocused?.focus()
      previouslyFocused = null
      return
    }
    if (!before) previouslyFocused = document.activeElement as HTMLElement | null
    await nextTick()
    // Focus sur le bouton principal (Entrée le valide), sinon sur le premier bouton.
    const buttons = dialog.value
    ;(buttons?.querySelector<HTMLButtonElement>('button.primary') ?? buttons?.querySelector('button'))?.focus()
  },
)
</script>

<template>
  <div
    v-if="confirm.current.value"
    class="overlay"
    @mousedown.self="confirm.answer('cancel')"
    @keydown.esc.stop="confirm.answer('cancel')"
  >
    <div ref="dialog" class="dialog" role="dialog" aria-modal="true" :aria-label="confirm.current.value.question.title">
      <h2>{{ confirm.current.value.question.title }}</h2>
      <p>{{ confirm.current.value.question.message }}</p>
      <div class="choices">
        <button
          v-for="choice in confirm.current.value.question.choices"
          :key="choice.id"
          :class="{ primary: choice.primary }"
          @click="confirm.answer(choice.id)"
        >
          {{ choice.label }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(10, 15, 25, 0.35);
  z-index: 100;
}
.dialog {
  width: min(460px, 92vw);
  padding: 18px 20px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow);
}
h2 {
  margin: 0 0 8px;
  font-size: 16px;
}
.choices {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
</style>
