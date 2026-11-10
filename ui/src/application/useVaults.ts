/**
 * Coffres des bases (spec §9, ADR-0010) : état de chaque coffre (pastilles du bandeau), dialogue
 * de mot de passe (création au premier chiffrement, déverrouillage, changement), chiffrer /
 * déchiffrer, délai du verrouillage automatique.
 *
 * `start()` : écoute `vaults-changed` (un coffre ouvert ou fermé dans une autre fenêtre, ou
 * verrouillé automatiquement) et signale l'activité de l'utilisateur au cœur (au plus une fois
 * toutes les 30 s), qui décide quand verrouiller.
 *
 * Aucune règle ici : le Rust décide (coffre requis, mot de passe trop court…) ; on enchaîne
 * seulement les étapes que l'utilisateur doit franchir (« créez d'abord le coffre de cette base »).
 */
import { ref, shallowRef } from 'vue'
import { isUiError, type BaseId, type Task } from '../domain/task'
import type { BaseVault, EncryptScope, VaultGateway, VaultState } from '../ports/VaultGateway'
import type { Confirm } from './useConfirm'
import { t } from '../i18n'

/** Dialogue de mot de passe ouvert : créer le coffre d'une base, le déverrouiller, ou en changer le mot de passe. */
export interface VaultPrompt {
  kind: 'create' | 'unlock' | 'change'
  base: BaseId
}

/** Ce que l'utilisateur a saisi dans le dialogue de mot de passe. */
export interface VaultEntry {
  password: string
  /** Deuxième saisie (création, changement). */
  confirmation?: string
  /** Case « Se souvenir sur cet appareil » (création, déverrouillage). */
  remember?: boolean
  /** Mot de passe actuel (changement seulement). */
  currentPassword?: string
}

/** Intervalle minimal entre deux signalements d'activité. */
export const ACTIVITY_THROTTLE_MS = 30_000

/** Ce qui doit être rafraîchi quand un coffre s'ouvre ou se ferme (tâches scellées ou non). */
type OnVaultChange = () => Promise<void>

export function useVaults(gateway: VaultGateway, confirm: Confirm, onChange: OnVaultChange) {
  const vaults = shallowRef<BaseVault[]>([])
  const prompt = shallowRef<VaultPrompt | null>(null)
  const error = ref<string | null>(null)
  const busy = ref(false)
  const autoLockMinutes = ref<number | null>(null)
  /** Action à reprendre une fois le coffre créé ou déverrouillé (chiffrer la tâche demandée). */
  let resume: (() => Promise<void>) | null = null

  async function load(): Promise<void> {
    vaults.value = await gateway.listVaults()
  }

  function stateOf(base: BaseId): VaultState {
    return vaults.value.find((vault) => vault.base === base)?.state ?? 'none'
  }

  function ask(kind: VaultPrompt['kind'], base: BaseId, then: (() => Promise<void>) | null = null): void {
    error.value = null
    resume = then
    prompt.value = { kind, base }
  }

  function cancel(): void {
    prompt.value = null
    resume = null
  }

  /** Valide le dialogue. */
  async function submit({
    password,
    confirmation = '',
    remember = false,
    currentPassword = '',
  }: VaultEntry): Promise<void> {
    const current = prompt.value
    if (current === null || busy.value) return
    if (current.kind !== 'unlock' && password !== confirmation) {
      error.value = t('vault.mismatch')
      return
    }
    busy.value = true
    error.value = null
    try {
      if (current.kind === 'create') await gateway.createVault(current.base, password, remember)
      else if (current.kind === 'unlock') await gateway.unlockVault(current.base, password, remember)
      else await gateway.changePassword(current.base, currentPassword, password)
      const then = resume
      cancel()
      if (current.kind === 'change') return
      await afterChange()
      await then?.()
    } catch (failure) {
      error.value = messageOf(failure)
    } finally {
      busy.value = false
    }
  }

  async function lock(base: BaseId): Promise<void> {
    await gateway.lockVault(base)
    await afterChange()
  }

  /** Clic sur le cadenas d'une base : verrouiller si ouvert, demander le mot de passe sinon. */
  async function toggle(base: BaseId): Promise<void> {
    const state = stateOf(base)
    if (state === 'unlocked') await lock(base)
    else if (state === 'locked') ask('unlock', base)
  }

  /** Bouton 🔒 : chiffrer la tâche, après avoir créé ou ouvert le coffre de sa base au besoin. */
  async function encrypt(task: Task, subtasks: number): Promise<void> {
    const scope = await chooseScope(subtasks)
    if (scope === null) return
    const run = async () => {
      await gateway.encryptTask(task.id, scope)
    }
    const state = stateOf(task.base)
    if (state === 'none') ask('create', task.base, run)
    else if (state === 'locked') ask('unlock', task.base, run)
    else await run()
  }

  async function decrypt(task: Task): Promise<void> {
    await gateway.decryptTask(task.id)
  }

  /** « Chiffrer aussi les sous-tâches ? » — proposé par défaut (spec §9). */
  async function chooseScope(subtasks: number): Promise<EncryptScope | null> {
    if (subtasks === 0) return 'taskOnly'
    const choice = await confirm.ask({
      title: t('vault.scopeTitle'),
      message: t('vault.scopeMessage'),
      choices: [
        { id: 'withSubtasks', label: t('vault.withSubtasks'), primary: true },
        { id: 'taskOnly', label: t('vault.taskOnly') },
      ],
    })
    return choice === 'cancel' ? null : choice
  }

  async function afterChange(): Promise<void> {
    await load()
    await onChange()
  }

  async function loadAutoLock(): Promise<void> {
    autoLockMinutes.value = await gateway.getAutoLockMinutes()
  }

  async function setAutoLock(minutes: number): Promise<void> {
    await gateway.setAutoLockMinutes(minutes)
    autoLockMinutes.value = minutes
  }

  /** Écoute les changements de coffres et signale l'activité ; renvoie de quoi arrêter. */
  async function start(target: Pick<Window, 'addEventListener' | 'removeEventListener'> = window) {
    let lastSignal = 0
    const signal = () => {
      const now = Date.now()
      if (now - lastSignal < ACTIVITY_THROTTLE_MS) return
      lastSignal = now
      void gateway.recordActivity()
    }
    target.addEventListener('keydown', signal, true)
    target.addEventListener('pointerdown', signal, true)
    const unlisten = await gateway.onVaultsChanged(() => void afterChange())
    return () => {
      unlisten()
      target.removeEventListener('keydown', signal, true)
      target.removeEventListener('pointerdown', signal, true)
    }
  }

  return {
    vaults,
    prompt,
    error,
    busy,
    autoLockMinutes,
    load,
    loadAutoLock,
    setAutoLock,
    start,
    stateOf,
    ask,
    cancel,
    submit,
    lock,
    toggle,
    encrypt,
    decrypt,
  }
}

function messageOf(failure: unknown): string {
  if (!isUiError(failure)) return String(failure)
  if (failure.code === 'vault-wrong-password') return t('vault.wrongPassword')
  return failure.message
}

export type VaultsState = ReturnType<typeof useVaults>
