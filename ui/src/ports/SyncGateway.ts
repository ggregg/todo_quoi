/**
 * Port : état de la synchronisation par le dossier du drive (spec §7) et journal des conflits.
 */
import type { InjectionKey } from 'vue'

/** État de l'indicateur (miroir de `SyncStatusDto`, `src-tauri/src/scanner.rs`). */
export interface SyncStatus {
  kind: 'starting' | 'upToDate' | 'downloading' | 'attention' | 'offline'
  /** Fichiers en attente de téléchargement par le client du drive. */
  pending: number
  /** Fichiers illisibles (ignorés, jamais réécrits). */
  unreadable: number
  /** Conflits réglés depuis la dernière consultation du journal. */
  conflicts: number
  message: string | null
}

/** Une version écartée par un conflit, conservée 30 jours. */
export interface ConflictCopy {
  taskId: string | null
  title: string | null
  fileName: string
  setAsideAt: string
}

export interface SyncGateway {
  getStatus(): Promise<SyncStatus>
  /** Demande un passage immédiat (retour au premier plan). */
  syncNow(): Promise<void>
  /** Journal des conflits ; le consulter éteint leur signalement. */
  listConflicts(): Promise<ConflictCopy[]>
  onStatus(listener: (status: SyncStatus) => void): Promise<() => void>
}

export const syncGatewayKey: InjectionKey<SyncGateway> = Symbol('SyncGateway')
