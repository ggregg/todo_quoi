/**
 * Port des médias (images, audio, pièces jointes) — séparé de `TaskGateway` (ségrégation des
 * interfaces) : l'éditeur de note n'a besoin que de ceci.
 *
 * Un média est désigné par son **lien de note** (`../media/<sha256>.<ext>`), tel qu'il est écrit
 * dans le Markdown. Implémentations : `TauriMediaGateway` (protocole `tq-media://`) et
 * `InMemoryMediaGateway` (URL `blob:` dans un navigateur).
 */
import type { InjectionKey } from 'vue'

export interface MediaGateway {
  /** Stocke des octets déjà préparés (image compressée, audio encodé) ; renvoie le lien de note. */
  store(bytes: Uint8Array, extension: string): Promise<string>
  /** URL affichable par la webview pour un lien de note (synchrone : utilisée au rendu). */
  url(noteLink: string): string
  /** Ouvre le média avec l'application par défaut de l'OS. */
  open(noteLink: string): Promise<void>
  /** Montre le média dans le Finder / l'Explorateur. */
  reveal(noteLink: string): Promise<void>
}

export const mediaGatewayKey: InjectionKey<MediaGateway> = Symbol('MediaGateway')

/** `../media/abc.jpg` → `abc.jpg` (ou `null` si ce n'est pas un lien de média local). */
export function mediaName(noteLink: string): string | null {
  return noteLink.startsWith('../media/') ? noteLink.slice('../media/'.length) : null
}
