/**
 * Enregistrement d'un message audio avec `MediaRecorder` (validé au POC I0-2) :
 * - format **WebM/Opus 32 kbit/s** imposé sur les deux OS (repli sur le format par défaut si
 *   une webview ne le proposait pas) ;
 * - l'indicateur « enregistrement » et le chronomètre ne démarrent qu'à l'événement `start`
 *   (au POC, les 0,7 premières secondes étaient perdues pendant l'ouverture du micro) ;
 * - la durée est **mesurée** pendant l'enregistrement : un WebM tout juste produit ne l'annonce
 *   pas toujours (`NaN` / `Infinity` selon la webview) ;
 * - **retour visuel** : niveau du micro (`level`, 0 – 1, mesuré par un `AnalyserNode` Web Audio),
 *   nom du micro utilisé (`microphone`) et alerte si aucun son n'est capté (`silent`).
 */
import { ref } from 'vue'
import { SilenceWatch, levelOf } from './audioLevel'
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
  /** Niveau du micro, 0 – 1. */
  const level = ref(0)
  /** Nom du micro utilisé (tel que donné par le système). */
  const microphone = ref('')
  /** Aucun son capté depuis plusieurs secondes : micro débranché, coupé ou mauvais micro ? */
  const silent = ref(false)

  let recorder: MediaRecorder | null = null
  let stream: MediaStream | null = null
  let chunks: Blob[] = []
  let startedAt = 0
  let ticker: ReturnType<typeof setInterval> | undefined
  let meter: { context: AudioContext; frame: number } | null = null

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
      microphone.value = stream.getAudioTracks()[0]?.label ?? ''
      startMeter(stream)
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

  /** Mesure le niveau ~60 fois par seconde, sans rien enregistrer de plus. */
  function startMeter(source: MediaStream): void {
    const context = new AudioContext()
    const analyser = context.createAnalyser()
    analyser.fftSize = 512
    context.createMediaStreamSource(source).connect(analyser)
    const samples = new Uint8Array(analyser.fftSize)
    const watch = new SilenceWatch(performance.now())
    const measure = () => {
      analyser.getByteTimeDomainData(samples)
      level.value = levelOf(samples)
      silent.value = state.value === 'recording' && watch.update(level.value, performance.now())
      if (meter) meter.frame = requestAnimationFrame(measure)
    }
    meter = { context, frame: requestAnimationFrame(measure) }
  }

  function stopMeter(): void {
    if (!meter) return
    cancelAnimationFrame(meter.frame)
    void meter.context.close()
    meter = null
    level.value = 0
    silent.value = false
  }

  /** Abandonne l'enregistrement en cours. */
  function cancel(): void {
    if (recorder && recorder.state !== 'inactive') recorder.stop()
    release()
  }

  function release(): void {
    clearInterval(ticker)
    stopMeter()
    stream?.getTracks().forEach((track) => track.stop())
    stream = null
    recorder = null
    state.value = 'idle'
    elapsed.value = 0
  }

  return { state, elapsed, level, microphone, silent, start, stop, cancel }
}
