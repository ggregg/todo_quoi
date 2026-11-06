/**
 * Enregistrement d'un message audio avec `MediaRecorder` (validé au POC I0-2) :
 * - format **WebM/Opus 32 kbit/s** imposé sur les deux OS (repli sur le format par défaut si
 *   une webview ne le proposait pas) ;
 * - l'indicateur « enregistrement » et le chronomètre ne démarrent qu'à l'événement `start`
 *   (au POC, les 0,7 premières secondes étaient perdues pendant l'ouverture du micro) ;
 * - la durée est **mesurée** pendant l'enregistrement : un WebM tout juste produit ne l'annonce
 *   pas toujours (`NaN` / `Infinity` selon la webview).
 */
import { ref } from 'vue'
import { AUDIO_BITS_PER_SECOND, AUDIO_MIME, extensionFor } from './mediaRules'

export interface Recording {
  bytes: Uint8Array
  extension: string
  mime: string
  /** Durée en secondes (une décimale). */
  duration: number
}

export type RecorderState = 'idle' | 'starting' | 'recording'

export function useAudioRecorder() {
  const state = ref<RecorderState>('idle')
  const elapsed = ref(0)

  let recorder: MediaRecorder | null = null
  let stream: MediaStream | null = null
  let chunks: Blob[] = []
  let startedAt = 0
  let ticker: ReturnType<typeof setInterval> | undefined

  async function start(): Promise<void> {
    if (state.value !== 'idle') return
    state.value = 'starting'
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      const mimeType = MediaRecorder.isTypeSupported(AUDIO_MIME) ? AUDIO_MIME : undefined
      recorder = new MediaRecorder(stream, { mimeType, audioBitsPerSecond: AUDIO_BITS_PER_SECOND })
      chunks = []
      recorder.ondataavailable = (event) => {
        if (event.data.size > 0) chunks.push(event.data)
      }
      recorder.onstart = () => {
        startedAt = performance.now()
        state.value = 'recording'
        ticker = setInterval(() => (elapsed.value = (performance.now() - startedAt) / 1000), 250)
      }
      recorder.start()
    } catch (error) {
      release()
      throw error
    }
  }

  /** Arrête et renvoie l'enregistrement (`null` s'il n'y en avait pas). */
  async function stop(): Promise<Recording | null> {
    const active = recorder
    if (!active || state.value !== 'recording') {
      release()
      return null
    }
    const duration = Math.round(((performance.now() - startedAt) / 1000) * 10) / 10
    const stopped = new Promise<void>((resolve) => (active.onstop = () => resolve()))
    active.stop()
    await stopped
    const mime = active.mimeType || 'audio/webm'
    const blob = new Blob(chunks, { type: mime })
    release()
    return {
      bytes: new Uint8Array(await blob.arrayBuffer()),
      extension: extensionFor({ type: mime }),
      mime: mime.split(';')[0] ?? mime,
      duration,
    }
  }

  /** Abandonne l'enregistrement en cours. */
  function cancel(): void {
    if (recorder && recorder.state !== 'inactive') recorder.stop()
    release()
  }

  function release(): void {
    clearInterval(ticker)
    stream?.getTracks().forEach((track) => track.stop())
    stream = null
    recorder = null
    state.value = 'idle'
    elapsed.value = 0
  }

  return { state, elapsed, start, stop, cancel }
}
