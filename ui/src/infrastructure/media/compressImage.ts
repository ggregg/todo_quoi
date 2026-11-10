/**
 * Façade du Worker de compression : `await compressImage(fichier)`.
 * Un seul Worker pour toute l'application, créé à la première utilisation.
 */
import type { CompressRequest, CompressResponse } from './imageWorker'

export interface CompressedImage {
  bytes: Uint8Array
  extension: 'jpg' | 'png'
  width: number
}

let worker: Worker | null = null
let nextId = 0
const pending = new Map<number, (response: CompressResponse) => void>()

function getWorker(): Worker {
  if (worker) return worker
  // Syntaxe reconnue par Vite : le Worker est compilé et servi comme un module séparé.
  worker = new Worker(new URL('./imageWorker.ts', import.meta.url), { type: 'module' })
  worker.onmessage = ({ data }: MessageEvent<CompressResponse>) => {
    pending.get(data.id)?.(data)
    pending.delete(data.id)
  }
  return worker
}

/** Compresse `blob` : côté long ≤ `maxSide` px, qualité JPEG `quality` % (réglage « Images »). */
export function compressImage(
  blob: Blob,
  settings: { quality: number; maxSide: number } = { quality: 85, maxSide: 2560 },
): Promise<CompressedImage> {
  const id = ++nextId
  return new Promise((resolve, reject) => {
    pending.set(id, (response) => {
      if (response.ok) resolve({ bytes: response.bytes, extension: response.extension, width: response.width })
      else reject(new Error(`compression de l'image impossible : ${response.message}`))
    })
    const request: CompressRequest = { id, blob, quality: settings.quality / 100, maxSide: settings.maxSide }
    getWorker().postMessage(request)
  })
}
