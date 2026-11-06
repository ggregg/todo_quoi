/**
 * Règles de **préparation** des médias côté interface (validées au POC I0-2) — fonctions pures :
 * - images : côté long ≤ 2560 px, JPEG 85, PNG si transparence ; GIF animés et SVG gardés tels
 *   quels (WebP écarté : non encodable par WKWebView) ;
 * - pièces jointes : 50 Mo maximum, avertissement au-delà de 10 Mo (spec §6.3) ;
 * - audio : WebM/Opus 32 kbit/s, format unique sur les deux OS.
 *
 * La limite de 50 Mo est **aussi** vérifiée par le cœur Rust (`check_media_size`) : celle-ci ne
 * sert qu'à prévenir l'utilisateur avant un envoi inutile.
 */

export const MAX_IMAGE_SIDE = 2560
export const JPEG_QUALITY = 0.85
export const MAX_MEDIA_BYTES = 50 * 1024 * 1024
export const WARN_MEDIA_BYTES = 10 * 1024 * 1024
export const AUDIO_MIME = 'audio/webm;codecs=opus'
export const AUDIO_BITS_PER_SECOND = 32_000

export type FileHandling = 'compress-image' | 'keep-image' | 'attachment' | 'too-large'

/** Que faire d'un fichier collé ou glissé dans la note ? */
export function handlingFor(file: { type: string; size: number }): FileHandling {
  if (file.size > MAX_MEDIA_BYTES) return 'too-large'
  if (file.type === 'image/gif' || file.type === 'image/svg+xml') return 'keep-image'
  if (file.type.startsWith('image/')) return 'compress-image'
  return 'attachment'
}

/** Dimensions après réduction : le côté le plus long ne dépasse pas `max`, proportions gardées. */
export function scaledSize(width: number, height: number, max = MAX_IMAGE_SIDE): { width: number; height: number } {
  const scale = Math.min(1, max / Math.max(width, height))
  return { width: Math.round(width * scale), height: Math.round(height * scale) }
}

/** Extension de stockage d'un fichier : d'après son nom, sinon d'après son type MIME. */
export function extensionFor(file: { name?: string; type: string }): string {
  const fromName = file.name?.match(/\.([a-z0-9]{1,8})$/i)?.[1]
  if (fromName) return fromName.toLowerCase()
  const byType: Record<string, string> = {
    'image/jpeg': 'jpg',
    'image/png': 'png',
    'image/gif': 'gif',
    'image/svg+xml': 'svg',
    'image/webp': 'webp',
    'audio/webm': 'webm',
    'audio/mp4': 'm4a',
    'application/pdf': 'pdf',
    'text/plain': 'txt',
  }
  return byType[file.type.split(';')[0] ?? ''] ?? 'bin'
}

/** `12.4` → `0:12`, `75` → `1:15`. */
export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.round(seconds))
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`
}

/** `318000` → `311 Ko`, `5_242_880` → `5,0 Mo`. */
export function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} Ko`
  return `${(bytes / (1024 * 1024)).toFixed(1).replace('.', ',')} Mo`
}
