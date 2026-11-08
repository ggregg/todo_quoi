/** `SyncGateway` pour le navigateur et les tests : état modifiable, aucun drive. */
import type { ConflictCopy, SyncGateway, SyncStatus } from '../../ports/SyncGateway'

export class InMemorySyncGateway implements SyncGateway {
  status: SyncStatus = { kind: 'upToDate', pending: 0, unreadable: 0, conflicts: 0, message: null }
  conflicts: ConflictCopy[] = []
  private readonly listeners = new Set<(status: SyncStatus) => void>()

  async getStatus(): Promise<SyncStatus> {
    return { ...this.status }
  }
  async syncNow(): Promise<void> {}
  async listConflicts(): Promise<ConflictCopy[]> {
    this.setStatus({ ...this.status, conflicts: 0, kind: this.status.unreadable > 0 ? 'attention' : 'upToDate' })
    return structuredClone(this.conflicts)
  }
  async onStatus(listener: (status: SyncStatus) => void): Promise<() => void> {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }
  /** Change l'état et prévient les abonnés (tests, démonstration). */
  setStatus(status: SyncStatus): void {
    this.status = status
    for (const listener of this.listeners) listener({ ...status })
  }
}
