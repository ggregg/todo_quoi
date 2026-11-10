<script setup lang="ts">
/**
 * Mot de passe du coffre d'une base (spec §9, ADR-0010) : **création** au premier chiffrement
 * (deux saisies, avertissement « perte = irrécupérable »), **déverrouillage**, ou **changement**
 * (mot de passe actuel, puis le nouveau deux fois). Case « Se souvenir sur cet appareil »
 * (trousseau de l'OS), décochée par défaut. Échap = annuler.
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
const currentPassword = ref('')
const remember = ref(false)
const form = ref<HTMLFormElement | null>(null)
/** Le premier champ : mot de passe (actuel, pour un changement). */
const focusFirstField = () => form.value?.querySelector('input')?.focus()

const prompt = computed(() => props.vaults.prompt.value)
const creating = computed(() => prompt.value?.kind === 'create')
const changing = computed(() => prompt.value?.kind === 'change')
const TITLES = { create: 'vault.createTitle', unlock: 'vault.unlockTitle', change: 'vault.changeTitle' } as const
const SUBMITS = { create: 'vault.create', unlock: 'vault.unlock', change: 'vault.change' } as const
const title = computed(() => {
  if (!prompt.value) return ''
  return t(TITLES[prompt.value.kind], { base: props.baseInfo(prompt.value.base).name })
})

// Nouveau dialogue : champs vides (un mot de passe ne reste jamais affiché d'une fois à l'autre).
watch(prompt, async (current) => {
  password.value = ''
  confirmation.value = ''
  currentPassword.value = ''
  remember.value = false
  if (!current) return
  await nextTick()
  focusFirstField()
})

// Refusé (mot de passe incorrect…) : on vide la saisie et on y revient, prête à retaper.
watch(
  () => props.vaults.error.value,
  async (error) => {
    if (error === null || creating.value) return
    if (changing.value) currentPassword.value = ''
    else password.value = ''
    await nextTick()
    focusFirstField()
  },
)

function submit(): void {
  void props.vaults.submit({
    password: password.value,
    confirmation: confirmation.value,
    remember: remember.value,
    currentPassword: currentPassword.value,
  })
}
</script>

<template>
  <div v-if="prompt" class="overlay" @mousedown.self="vaults.cancel()" @keydown.esc.stop="vaults.cancel()">
    <form ref="form" class="dialog" role="dialog" aria-modal="true" :aria-label="title" @submit.prevent="submit">
      <h2><AppIcon name="lock" :size="16" /> {{ title }}</h2>
      <template v-if="creating">
        <p>{{ t('vault.createMessage') }}</p>
        <p class="warning" role="note">{{ t('vault.warning') }}</p>
      </template>

      <label v-if="changing">
        {{ t('vault.currentPassword') }}
        <input v-model="currentPassword" type="password" autocomplete="current-password" />
      </label>
      <label>
        {{ changing ? t('vault.newPassword') : t('vault.password') }}
        <input v-model="password" type="password" autocomplete="new-password" />
      </label>
      <p v-if="changing" class="hint">{{ t('vault.changeMessage') }}</p>
      <label v-if="creating || changing">
        {{ t('vault.confirmation') }}
        <input v-model="confirmation" type="password" autocomplete="new-password" />
        <small>{{ t('vault.minLength', { min: MIN_PASSWORD_CHARS }) }}</small>
      </label>
      <label v-if="!changing" class="remember">
        <input v-model="remember" type="checkbox" />
        {{ t('vault.remember') }}
      </label>

      <p v-if="vaults.error.value" class="error" role="alert">{{ vaults.error.value }}</p>

      <div class="choices">
        <button type="button" @click="vaults.cancel()">{{ t('vault.cancel') }}</button>
        <button type="submit" class="primary" :disabled="vaults.busy.value || password.length === 0">
          {{ prompt ? t(SUBMITS[prompt.kind]) : '' }}
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
small,
.hint {
  color: var(--muted);
  font-size: 12px;
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
