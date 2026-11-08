/** `VaultGateway` réel : commandes `list_vaults`, `create_vault`, `unlock_vault`, `lock_vault`, `encrypt_task`, `decrypt_task`. */
import { invoke } from '@tauri-apps/api/core'
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
  encryptTask(taskId: TaskId, scope: EncryptScope): Promise<TaskId[]> {
    return invoke('encrypt_task', { taskId, scope })
  }
  decryptTask(taskId: TaskId): Promise<void> {
    return invoke('decrypt_task', { taskId })
  }
}
