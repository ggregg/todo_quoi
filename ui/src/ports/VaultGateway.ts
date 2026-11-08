/**
 * Port : coffre de chaque base (spec §9, ADR-0010). Un coffre par base, créé au premier
 * chiffrement par un mot de passe ; chaque coffre se déverrouille et se verrouille seul.
 *
 * Erreurs (`UiError.code`) : `vault-none` (créer le coffre), `vault-locked` (le déverrouiller),
 * `vault-wrong-password`, `vault-password-too-short`.
 */
import type { InjectionKey } from 'vue'
import type { BaseId, TaskId } from '../domain/task'

/** Aucun coffre, verrouillé, ouvert. */
export type VaultState = 'none' | 'locked' | 'unlocked'

export interface BaseVault {
  base: BaseId
  state: VaultState
}

/** Chiffrer aussi les sous-tâches (proposé par défaut) ? */
export type EncryptScope = 'taskOnly' | 'withSubtasks'

export interface VaultGateway {
  listVaults(): Promise<BaseVault[]>
  /** `remember` : garder la clé dans le trousseau de l'OS (« Se souvenir sur cet appareil »). */
  createVault(base: BaseId, password: string, remember: boolean): Promise<void>
  unlockVault(base: BaseId, password: string, remember: boolean): Promise<void>
  lockVault(base: BaseId): Promise<void>
  /** Renvoie les tâches chiffrées. */
  encryptTask(taskId: TaskId, scope: EncryptScope): Promise<TaskId[]>
  decryptTask(taskId: TaskId): Promise<void>
}

/** Longueur minimale d'un mot de passe (miroir de `MIN_PASSWORD_CHARS`, le Rust vérifie). */
export const MIN_PASSWORD_CHARS = 8

export const vaultGatewayKey: InjectionKey<VaultGateway> = Symbol('VaultGateway')
