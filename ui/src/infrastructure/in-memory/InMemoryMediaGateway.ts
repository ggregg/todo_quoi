/**
 * Médias en mémoire pour `pnpm ui:dev` et les tests : chaque média devient une URL `blob:`.
 * Le « nom » imite le format réel (64 chiffres hexadécimaux + extension) sans vrai SHA-256.
 */
import { mediaName, type MediaGateway } from '../../ports/MediaGateway'

export class InMemoryMediaGateway implements MediaGateway {
  private readonly urls = new Map<string, string>()
  private counter = 0
  /** Médias ouverts / révélés (pas d'OS dans un navigateur) : consultable par les tests. */
  readonly opened: string[] = []

  async store(_taskId: string, bytes: Uint8Array, extension: string): Promise<string> {
    const name = `${(++this.counter).toString(16).padStart(64, '0')}.${extension.toLowerCase()}`
    const url =
      typeof URL.createObjectURL === 'function' ? URL.createObjectURL(new Blob([bytes.slice()])) : `memoire:${name}`
    this.urls.set(name, url)
    return `../media/${name}`
  }

  url(noteLink: string): string {
    const name = mediaName(noteLink)
    return (name && this.urls.get(name)) ?? noteLink
  }

  async open(noteLink: string): Promise<void> {
    this.opened.push(noteLink)
  }

  async reveal(noteLink: string): Promise<void> {
    this.opened.push(noteLink)
  }
}
