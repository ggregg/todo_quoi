/**
 * `VaultGateway` réel : commandes `list_vaults`, `create_vault`, `unlock_vault`, `lock_vault`,
 * `change_vault_password`, `vault_activity`, `get/set_vault_auto_lock`, `encrypt_task`,
 * `decrypt_task` ; événement `vaults-changed`.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { BaseId, TaskId } from '../../domain/task'
import type { BaseVault, EncryptScope, VaultGateway } from '../../ports/VaultGateway'

export class TauriVaultGateway implements VaultGateway {
  listVaults(): Promise<BaseVault[]> {
    return invoke('list_vaults')
  }
  createVault(base: BaseId, password: string, remember: boolean): Promise<void> {
    return invoke('create_vault', { base, password, remember })
  }
  unlockVault(base: BaseId, password: string, remember: boolean): Promise<void> {
    return invoke('unlock_vault', { base, password, remember })
  }
  lockVault(base: BaseId): Promise<void> {
    return invoke('lock_vault', { base })
  }
  changePassword(base: BaseId, current: string, next: string): Promise<void> {
    return invoke('change_vault_password', { base, current, next })
  }
  recordActivity(): Promise<void> {
    return invoke('vault_activity')
  }
  getAutoLockMinutes(): Promise<number> {
    return invoke('get_vault_auto_lock')
  }
  setAutoLockMinutes(minutes: number): Promise<void> {
    return invoke('set_vault_auto_lock', { minutes })
  }
  onVaultsChanged(listener: () => void): Promise<() => void> {
    return listen('vaults-changed', () => listener())
  }
  encryptTask(taskId: TaskId, scope: EncryptScope): Promise<TaskId[]> {
    return invoke('encrypt_task', { taskId, scope })
  }
  decryptTask(taskId: TaskId): Promise<void> {
    return invoke('decrypt_task', { taskId })
  }
}
