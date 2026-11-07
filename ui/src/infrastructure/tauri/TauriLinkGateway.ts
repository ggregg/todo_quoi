/** `LinkGateway` réel : commande Rust `open_link` (vérification du lien, puis système). */
import { invoke } from '@tauri-apps/api/core'
import type { LinkGateway } from '../../ports/LinkGateway'

export class TauriLinkGateway implements LinkGateway {
  open(url: string): Promise<void> {
    return invoke('open_link', { url })
  }
}
