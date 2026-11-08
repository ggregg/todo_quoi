/** `SettingsGateway` réel : commandes `get_data_location`, `add_base`, `update_base`, `remove_base`. */
import { invoke } from '@tauri-apps/api/core'
import type { DataLocation, SettingsGateway } from '../../ports/SettingsGateway'

export class TauriSettingsGateway implements SettingsGateway {
  getDataLocation(): Promise<DataLocation> {
    return invoke('get_data_location')
  }
  addBase(name: string, color: string, path: string): Promise<void> {
    return invoke('add_base', { name, color, path })
  }
  updateBase(id: string, name: string, color: string): Promise<void> {
    return invoke('update_base', { id, name, color })
  }
  removeBase(id: string): Promise<void> {
    return invoke('remove_base', { id })
  }
}
