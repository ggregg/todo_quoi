/**
 * Worker de compression d'image : tourne **hors du fil principal**, pour ne jamais figer
 * l'interface (POC I0-2 : jusqu'à 3 s sur le fil principal pour une photo de 24 Mpx).
 *
 * Reçoit une image, applique l'orientation EXIF, réduit le côté long (2560 px par défaut), puis
 * encode en JPEG (qualité 85 par défaut) — ou en PNG si l'image a de la transparence. Le réencodage efface les métadonnées
 * (GPS…), vérifié au POC.
 */
import { scaledSize } from '../../application/mediaRules'

export interface CompressRequest {
  id: number
  blob: Blob
  /** Qualité JPEG, de 0 à 1. */
  quality: number
  /** Plus grand côté, en pixels. */
  maxSide: number
}

export type CompressResponse =
  | { id: number; ok: true; bytes: Uint8Array; extension: 'jpg' | 'png'; width: number; height: number }
  | { id: number; ok: false; message: string }

// Dans un Worker, `self` est le contexte du Worker (pas `window`) : on le type explicitement.
const scope = self as unknown as {
  onmessage: ((event: MessageEvent<CompressRequest>) => void) | null
  postMessage(message: CompressResponse, transfer?: Transferable[]): void
}

scope.onmessage = async ({ data }) => {
  try {
    const bitmap = await createImageBitmap(data.blob, { imageOrientation: 'from-image' })
    const { width, height } = scaledSize(bitmap.width, bitmap.height, data.maxSide)
    const canvas = new OffscreenCanvas(width, height)
    const context = canvas.getContext('2d')
    if (!context) throw new Error('contexte 2D indisponible')
    context.drawImage(bitmap, 0, 0, width, height)
    bitmap.close()
    const transparent = hasTransparency(context.getImageData(0, 0, width, height).data)
    const blob = await canvas.convertToBlob(
      transparent ? { type: 'image/png' } : { type: 'image/jpeg', quality: data.quality },
    )
    const bytes = new Uint8Array(await blob.arrayBuffer())
    scope.postMessage({ id: data.id, ok: true, bytes, extension: transparent ? 'png' : 'jpg', width, height }, [
      bytes.buffer,
    ])
  } catch (error) {
    scope.postMessage({ id: data.id, ok: false, message: error instanceof Error ? error.message : String(error) })
  }
}

function hasTransparency(pixels: Uint8ClampedArray): boolean {
  for (let alpha = 3; alpha < pixels.length; alpha += 4) if (pixels[alpha]! < 255) return true
  return false
}
