<script setup lang="ts">
/**
 * Mot de passe du coffre d'une base (spec §9, ADR-0010) : **création** au premier chiffrement
 * (deux saisies, avertissement « perte = irrécupérable ») ou **déverrouillage**. Case « Se
 * souvenir sur cet appareil » (trousseau de l'OS), décochée par défaut. Échap = annuler.
 */
import { computed, nextTick, ref, watch } from 'vue'
import type { VaultsState } from '../application/useVaults'
import type { BaseInfo } from '../ports/SettingsGateway'
import { MIN_PASSWORD_CHARS } from '../ports/VaultGateway'
import { t } from '../i18n'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ vaults: VaultsState; baseInfo: (id: string) => BaseInfo }>()

const password = ref('')
const confirmation = ref('')
const remember = ref(false)
const field = ref<HTMLInputElement | null>(null)

const prompt = computed(() => props.vaults.prompt.value)
const creating = computed(() => prompt.value?.kind === 'create')
const title = computed(() => {
  if (!prompt.value) return ''
  const base = props.baseInfo(prompt.value.base).name
  return t(creating.value ? 'vault.createTitle' : 'vault.unlockTitle', { base })
})

// Nouveau dialogue : champs vides (un mot de passe ne reste jamais affiché d'une fois à l'autre).
watch(prompt, async (current) => {
  password.value = ''
  confirmation.value = ''
  remember.value = false
  if (!current) return
  await nextTick()
  field.value?.focus()
})

// Refusé (mot de passe incorrect…) : on vide la saisie et on y revient, prête à retaper.
watch(
  () => props.vaults.error.value,
  async (error) => {
    if (error === null || creating.value) return
    password.value = ''
    await nextTick()
    field.value?.focus()
  },
)

function submit(): void {
  void props.vaults.submit(password.value, confirmation.value, remember.value)
}
</script>

<template>
  <div v-if="prompt" class="overlay" @mousedown.self="vaults.cancel()" @keydown.esc.stop="vaults.cancel()">
    <form class="dialog" role="dialog" aria-modal="true" :aria-label="title" @submit.prevent="submit">
      <h2><AppIcon name="lock" :size="16" /> {{ title }}</h2>
      <template v-if="creating">
        <p>{{ t('vault.createMessage') }}</p>
        <p class="warning" role="note">{{ t('vault.warning') }}</p>
      </template>

      <label>
        {{ t('vault.password') }}
        <input ref="field" v-model="password" type="password" autocomplete="new-password" />
      </label>
      <label v-if="creating">
        {{ t('vault.confirmation') }}
        <input v-model="confirmation" type="password" autocomplete="new-password" />
        <small>{{ t('vault.minLength', { min: MIN_PASSWORD_CHARS }) }}</small>
      </label>
      <label class="remember">
        <input v-model="remember" type="checkbox" />
        {{ t('vault.remember') }}
      </label>

      <p v-if="vaults.error.value" class="error" role="alert">{{ vaults.error.value }}</p>

      <div class="choices">
        <button type="button" @click="vaults.cancel()">{{ t('vault.cancel') }}</button>
        <button type="submit" class="primary" :disabled="vaults.busy.value || password.length === 0">
          {{ creating ? t('vault.create') : t('vault.unlock') }}
        </button>
      </div>
    </form>
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
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: min(440px, 92vw);
  padding: 18px 20px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow);
}
h2 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 16px;
}
p {
  margin: 0;
}
.warning {
  padding: 8px 10px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--danger, #c0392b) 12%, var(--surface));
  color: var(--danger, #c0392b);
  font-weight: 600;
}
label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}
label.remember {
  flex-direction: row;
  align-items: center;
  gap: 8px;
}
small {
  color: var(--muted);
}
.error {
  color: var(--danger, #c0392b);
}
.choices {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 6px;
}
</style>
