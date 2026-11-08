/** `SyncGateway` réel : commandes `get_sync_status`, `sync_now`, `list_conflicts`, événement `sync-status`. */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { ConflictCopy, SyncGateway, SyncStatus } from '../../ports/SyncGateway'

export class TauriSyncGateway implements SyncGateway {
  getStatus(): Promise<SyncStatus> {
    return invoke('get_sync_status')
  }
  syncNow(): Promise<void> {
    return invoke('sync_now')
  }
  listConflicts(): Promise<ConflictCopy[]> {
    return invoke('list_conflicts')
  }
  async onStatus(listener: (status: SyncStatus) => void): Promise<() => void> {
    return listen<SyncStatus>('sync-status', (event) => listener(event.payload))
  }
}
