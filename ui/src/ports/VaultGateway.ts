/**
 * Port : coffre de chaque base (spec §9, ADR-0010). Un coffre par base, créé au premier
 * chiffrement par un mot de passe ; chaque coffre se déverrouille et se verrouille seul.
 *
 * Erreurs (`UiError.code`) : `vault-none` (créer le coffre), `vault-locked` (le déverrouiller),
 * `vault-target-locked` (coffre de la base de destination fermé), `vault-wrong-password`,
 * `vault-password-too-short`.
 *
 * Verrouillage automatique : après `autoLockMinutes` sans activité signalée par `recordActivity`
 * (le minuteur est en Rust) ; un coffre gardé dans le trousseau ne se verrouille pas seul.
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
  /** Ré-enveloppe la clé du coffre ; rien n'est rechiffré. */
  changePassword(base: BaseId, current: string, next: string): Promise<void>
  /** L'utilisateur agit : le délai du verrouillage automatique repart de zéro. */
  recordActivity(): Promise<void>
  /** Délai du verrouillage automatique, en minutes ; `0` : jamais. */
  getAutoLockMinutes(): Promise<number>
  setAutoLockMinutes(minutes: number): Promise<void>
  /** Un coffre s'est ouvert ou fermé (dans n'importe quelle fenêtre, ou automatiquement). */
  onVaultsChanged(listener: () => void): Promise<() => void>
  /** Renvoie les tâches chiffrées. */
  encryptTask(taskId: TaskId, scope: EncryptScope): Promise<TaskId[]>
  decryptTask(taskId: TaskId): Promise<void>
}

/** Délais proposés pour le verrouillage automatique (minutes ; `0` : jamais). */
export const AUTO_LOCK_CHOICES = [5, 15, 30, 60, 0] as const

/** Longueur minimale d'un mot de passe (miroir de `MIN_PASSWORD_CHARS`, le Rust vérifie). */
export const MIN_PASSWORD_CHARS = 8

export const vaultGatewayKey: InjectionKey<VaultGateway> = Symbol('VaultGateway')
