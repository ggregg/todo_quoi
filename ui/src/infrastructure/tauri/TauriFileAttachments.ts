/** `FileAttachments` réel : fenêtre native (plugin `dialog`) et commandes `attach_file`, `link_file`… */
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { AttachMode, AttachedFile, FileAttachments, LinkState, LinkedFile } from '../../ports/FileAttachments'

type LinkTarget = Pick<LinkedFile, 'path' | 'device'>

export class TauriFileAttachments implements FileAttachments {
  async pickFiles(title: string): Promise<string[] | null> {
    const chosen = await open({ multiple: true, directory: false, title })
    if (chosen === null) return []
    return Array.isArray(chosen) ? chosen : [chosen]
  }
  attach(taskId: string, path: string, mode: AttachMode): Promise<AttachedFile> {
    return invoke('attach_file', { taskId, path, mode })
  }
  link(path: string): Promise<LinkedFile> {
    return invoke('link_file', { path })
  }
  async linkState(link: LinkTarget): Promise<LinkState> {
    const status = await invoke<{ state: LinkState }>('linked_file_status', { ...link })
    return status.state
  }
  openLinked(link: LinkTarget): Promise<void> {
    return invoke('open_linked_file', { ...link })
  }
  revealLinked(link: LinkTarget): Promise<void> {
    return invoke('reveal_linked_file', { ...link })
  }
}
