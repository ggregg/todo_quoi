/**
 * Médias via le cœur Rust : envoi en binaire brut (`store_media`), affichage par le protocole
 * `tq-media://` (`src-tauri/src/media_protocol.rs`), ouverture par l'OS.
 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { mediaName, type MediaGateway } from '../../ports/MediaGateway'

export class TauriMediaGateway implements MediaGateway {
  store(bytes: Uint8Array, extension: string): Promise<string> {
    // Corps binaire brut + extension dans un en-tête (voir commands/media.rs).
    return invoke('store_media', bytes, { headers: { 'x-media-extension': extension } })
  }

  url(noteLink: string): string {
    const name = mediaName(noteLink)
    // `convertFileSrc` produit `tq-media://localhost/<nom>` (macOS) ou
    // `http://tq-media.localhost/<nom>` (Windows) selon la plateforme.
    return name ? convertFileSrc(name, 'tq-media') : noteLink
  }

  open(noteLink: string): Promise<void> {
    return invoke('open_media', { name: mediaName(noteLink) ?? noteLink })
  }

  reveal(noteLink: string): Promise<void> {
    return invoke('reveal_media', { name: mediaName(noteLink) ?? noteLink })
  }
}
