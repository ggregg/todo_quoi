/**
 * POC I0-2 — ce que la webview sait faire seule, sans dépendance :
 *   A. Enregistrement audio (MediaRecorder) : formats disponibles, enregistrement, relecture.
 *   B. Lecture croisée : la webview lit-elle le format de l'autre OS ?
 *   C. Images : JPEG / WebP via canvas, orientation EXIF, suppression des métadonnées,
 *      détection de transparence, redimensionnement d'une grande image.
 *   D. Micro (bouton) : autorisation, enregistrement réel, relecture.
 *
 * Chaque test ajoute une ligne au tableau ; le rapport complet est envoyé au Rust
 * (`invoke('report')`) qui l'écrit dans un fichier lisible hors de l'app.
 */
import { invoke } from '@tauri-apps/api/core'

type Outcome = { section: string; name: string; ok: boolean | null; detail: string }
const outcomes: Outcome[] = []
const os = /Mac/.test(navigator.userAgent) ? 'macos' : /Windows/.test(navigator.userAgent) ? 'windows' : 'autre'

function record(section: string, name: string, ok: boolean | null, detail: unknown = '') {
  const text = typeof detail === 'string' ? detail : JSON.stringify(detail)
  outcomes.push({ section, name, ok, detail: text })
  const row = document.createElement('tr')
  const symbol = ok === null ? 'ℹ︎' : ok ? '✔' : '✘'
  row.innerHTML = `<td class="s">${symbol}</td><td class="n">${section} · ${name}</td><td class="d"></td>`
  ;(row.lastElementChild as HTMLElement).textContent = text
  document.querySelector('#results')!.append(row)
}

async function attempt(section: string, name: string, test: () => Promise<{ ok: boolean | null; detail?: unknown }>) {
  try {
    const { ok, detail } = await test()
    record(section, name, ok, detail ?? '')
  } catch (error) {
    record(section, name, false, `exception : ${error instanceof Error ? `${error.name}: ${error.message}` : String(error)}`)
  }
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
const extensionFor = (mime: string) => (mime.includes('webm') ? 'webm' : mime.includes('mp4') ? 'm4a' : mime.includes('ogg') ? 'ogg' : 'bin')

async function report() {
  const json = JSON.stringify({ os, userAgent: navigator.userAgent, date: new Date().toISOString(), outcomes }, null, 2)
  try {
    const path = await invoke<string>('report', { json })
    document.querySelector('#status')!.textContent = `rapport : ${path}`
  } catch {
    document.querySelector('#status')!.textContent = 'rapport non envoyé (hors Tauri ?)'
  }
}

async function saveSample(name: string, blob: Blob) {
  const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()))
  return invoke<string>('save_sample', { name, bytes })
}

// ----- Outils audio -------------------------------------------------------------------

/** Enregistre `ms` millisecondes d'un flux, avec le format par défaut de la webview. */
async function recordStream(stream: MediaStream, ms: number, mimeType?: string): Promise<Blob> {
  const recorder = new MediaRecorder(stream, mimeType ? { mimeType } : undefined)
  const chunks: Blob[] = []
  recorder.ondataavailable = (event) => event.data.size > 0 && chunks.push(event.data)
  const stopped = new Promise<void>((resolve) => (recorder.onstop = () => resolve()))
  recorder.start()
  await sleep(ms)
  recorder.stop()
  await stopped
  return new Blob(chunks, { type: recorder.mimeType })
}

/** Décode un fichier audio : preuve que la webview sait le lire. Renvoie la durée en s. */
async function decode(blob: Blob): Promise<number> {
  const context = new AudioContext()
  try {
    const buffer = await context.decodeAudioData(await blob.arrayBuffer())
    return Math.round(buffer.duration * 100) / 100
  } finally {
    await context.close()
  }
}

/** Charge le fichier dans un élément <audio> (ce que fera le lecteur de l'app). */
function loadInAudioElement(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const audio = new Audio()
    const url = URL.createObjectURL(blob)
    const timer = setTimeout(() => reject(new Error('délai dépassé (5 s)')), 5000)
    audio.oncanplay = () => {
      clearTimeout(timer)
      resolve(`canplay, duration=${audio.duration}`)
      URL.revokeObjectURL(url)
    }
    audio.onerror = () => {
      clearTimeout(timer)
      reject(new Error(`MediaError code ${audio.error?.code}`))
    }
    audio.src = url
  })
}

// ----- A. Enregistrement (sans micro : tonalité synthétique) ---------------------------

async function sectionRecording() {
  const S = 'A. Enregistrement'
  await attempt(S, 'MediaRecorder disponible', async () => ({ ok: typeof MediaRecorder !== 'undefined' }))
  const candidates = ['audio/webm;codecs=opus', 'audio/webm', 'audio/mp4;codecs=mp4a.40.2', 'audio/mp4', 'audio/ogg;codecs=opus']
  await attempt(S, 'formats acceptés (isTypeSupported)', async () => ({
    ok: null,
    detail: Object.fromEntries(candidates.map((type) => [type, MediaRecorder.isTypeSupported(type)])),
  }))

  let synth: Blob | undefined
  await attempt(S, 'enregistrer une tonalité 440 Hz (1,5 s)', async () => {
    const context = new AudioContext()
    await context.resume()
    const oscillator = context.createOscillator()
    const destination = context.createMediaStreamDestination()
    oscillator.frequency.value = 440
    oscillator.connect(destination)
    oscillator.start()
    synth = await recordStream(destination.stream, 1500)
    oscillator.stop()
    await context.close()
    return { ok: synth.size > 1000, detail: { mime: synth.type, octets: synth.size, contexte: context.state } }
  })
  if (!synth) return
  const name = `tonalite-${os}.${extensionFor(synth.type)}`
  await attempt(S, `relire son propre enregistrement (decodeAudioData)`, async () => ({ ok: true, detail: `durée ${await decode(synth!)} s` }))
  await attempt(S, `relire son propre enregistrement (<audio>)`, async () => ({ ok: true, detail: await loadInAudioElement(synth!) }))
  await attempt(S, `sauvegarder ${name} pour l'autre OS`, async () => ({ ok: true, detail: await saveSample(name, synth!) }))

  // Format unique envisagé : WebM/Opus imposé sur tous les OS.
  const webm = 'audio/webm;codecs=opus'
  if (!MediaRecorder.isTypeSupported(webm)) {
    record(S, 'enregistrement forcé en WebM/Opus', false, 'non supporté')
    return
  }
  let forced: Blob | undefined
  await attempt(S, 'enregistrement forcé en WebM/Opus (1,5 s)', async () => {
    const context = new AudioContext()
    await context.resume()
    const oscillator = context.createOscillator()
    const destination = context.createMediaStreamDestination()
    oscillator.connect(destination)
    oscillator.start()
    forced = await recordStream(destination.stream, 1500, webm)
    oscillator.stop()
    await context.close()
    return { ok: forced.size > 1000 && forced.type.includes('webm'), detail: { mime: forced.type, octets: forced.size } }
  })
  if (!forced) return
  await attempt(S, 'relire le WebM/Opus forcé (decodeAudioData)', async () => ({ ok: true, detail: `durée ${await decode(forced!)} s` }))
  await attempt(S, 'relire le WebM/Opus forcé (<audio>)', async () => ({ ok: true, detail: await loadInAudioElement(forced!) }))
  await attempt(S, `sauvegarder tonalite-${os}-force.webm`, async () => ({ ok: true, detail: await saveSample(`tonalite-${os}-force.webm`, forced!) }))
}

// ----- B. Lecture croisée ---------------------------------------------------------------

async function sectionCrossPlayback() {
  const S = 'B. Lecture croisée'
  const types = ['audio/webm; codecs="opus"', 'audio/mp4; codecs="mp4a.40.2"', 'audio/ogg; codecs="opus"']
  const probe = new Audio()
  await attempt(S, 'canPlayType', async () => ({ ok: null, detail: Object.fromEntries(types.map((t) => [t, probe.canPlayType(t) || '(non)'])) }))

  const samples = [
    'tonalite-chromium.webm', 'tonalite-windows.webm', 'micro-windows.webm',
    'tonalite-macos.m4a', 'tonalite-macos-force.webm', 'micro-macos.m4a', 'micro-macos.webm',
  ]
  for (const sample of samples) {
    const response = await fetch(`/samples/${sample}`)
    // Vite renvoie index.html (200) pour un fichier absent : on vérifie aussi le type.
    const isHtml = response.headers.get('content-type')?.includes('text/html') ?? false
    if (!response.ok || isHtml) {
      record(S, sample, null, 'échantillon absent (sera produit sur l’autre OS)')
      continue
    }
    const blob = await response.blob()
    await attempt(S, `${sample} — decodeAudioData`, async () => ({ ok: true, detail: `durée ${await decode(blob)} s, ${blob.size} o` }))
    await attempt(S, `${sample} — <audio>`, async () => ({ ok: true, detail: await loadInAudioElement(blob) }))
  }
}

// ----- C. Images ------------------------------------------------------------------------

const canvasToBlob = (canvas: HTMLCanvasElement, type: string, quality?: number) =>
  new Promise<Blob>((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('toBlob a renvoyé null'))), type, quality))

/** Image de test 300×150 : moitié gauche rouge, droite bleue (permet de voir une rotation). */
function landscapeCanvas(alpha = false): HTMLCanvasElement {
  const canvas = document.createElement('canvas')
  canvas.width = 300
  canvas.height = 150
  const context = canvas.getContext('2d')!
  if (!alpha) {
    context.fillStyle = '#d33'
    context.fillRect(0, 0, 150, 150)
  }
  context.fillStyle = '#33d'
  context.fillRect(150, 0, 150, 150)
  return canvas
}

/** Insère un segment EXIF minimal (Orientation = 6, « tourner de 90° ») juste après le SOI. */
function withExifOrientation6(jpeg: Uint8Array): Uint8Array<ArrayBuffer> {
  const tiff = [0x4d, 0x4d, 0x00, 0x2a, 0, 0, 0, 8, 0x00, 0x01, 0x01, 0x12, 0x00, 0x03, 0, 0, 0, 1, 0x00, 0x06, 0x00, 0x00, 0, 0, 0, 0]
  const payload = [...new TextEncoder().encode('Exif'), 0, 0, ...tiff]
  const length = payload.length + 2
  const app1 = [0xff, 0xe1, length >> 8, length & 0xff, ...payload]
  return new Uint8Array([...jpeg.slice(0, 2), ...app1, ...jpeg.slice(2)])
}

const containsExif = (bytes: Uint8Array) => readExif(bytes) !== null

/** Lit le segment EXIF (APP1) d'un JPEG : taille, nombre d'étiquettes, orientation, GPS. */
function readExif(bytes: Uint8Array): { octets: number; etiquettes: number[]; orientation: number | null; gps: boolean } | null {
  let offset = 2
  while (offset + 4 < bytes.length && bytes[offset] === 0xff) {
    const marker = bytes[offset + 1]
    const length = (bytes[offset + 2] << 8) | bytes[offset + 3]
    const isExif = marker === 0xe1 && new TextDecoder('latin1').decode(bytes.slice(offset + 4, offset + 8)) === 'Exif'
    if (isExif) {
      const tiff = offset + 10
      const little = bytes[tiff] === 0x49
      const u16 = (at: number) => (little ? bytes[at] | (bytes[at + 1] << 8) : (bytes[at] << 8) | bytes[at + 1])
      const u32 = (at: number) => (little ? u16(at) + u16(at + 2) * 65536 : u16(at) * 65536 + u16(at + 2))
      const ifd = tiff + u32(tiff + 4)
      const tags: number[] = []
      let orientation: number | null = null
      for (let i = 0; i < u16(ifd); i++) {
        const entry = ifd + 2 + i * 12
        const tag = u16(entry)
        tags.push(tag)
        if (tag === 0x0112) orientation = u16(entry + 8)
      }
      return { octets: length, etiquettes: tags.map((t) => Number(`0x${t.toString(16)}`)), orientation, gps: tags.includes(0x8825) }
    }
    if (marker === 0xda) break // début des données d'image
    offset += 2 + length
  }
  return null
}

function hasTransparency(bitmap: ImageBitmap): boolean {
  const canvas = document.createElement('canvas')
  canvas.width = bitmap.width
  canvas.height = bitmap.height
  const context = canvas.getContext('2d')!
  context.drawImage(bitmap, 0, 0)
  const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data
  for (let i = 3; i < pixels.length; i += 4) if (pixels[i] < 255) return true
  return false
}

/** Compression telle que prévue dans l'app : côté long ≤ 2560 px, JPEG 0,85 (PNG si transparence). */
async function compress(source: Blob): Promise<{ blob: Blob; width: number; height: number; ms: number }> {
  const started = performance.now()
  const bitmap = await createImageBitmap(source, { imageOrientation: 'from-image' })
  const scale = Math.min(1, 2560 / Math.max(bitmap.width, bitmap.height))
  const canvas = document.createElement('canvas')
  canvas.width = Math.round(bitmap.width * scale)
  canvas.height = Math.round(bitmap.height * scale)
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0, canvas.width, canvas.height)
  const blob = hasTransparency(bitmap) ? await canvasToBlob(canvas, 'image/png') : await canvasToBlob(canvas, 'image/jpeg', 0.85)
  return { blob, width: canvas.width, height: canvas.height, ms: Math.round(performance.now() - started) }
}

async function sectionImages() {
  const S = 'C. Images'
  await attempt(S, 'encodage WebP (toBlob)', async () => {
    const blob = await canvasToBlob(landscapeCanvas(), 'image/webp', 0.85)
    return { ok: null, detail: `type obtenu : ${blob.type} (WebP ${blob.type === 'image/webp' ? 'supporté' : 'NON supporté'})` }
  })

  const plainJpeg = new Uint8Array(await (await canvasToBlob(landscapeCanvas(), 'image/jpeg', 0.9)).arrayBuffer())
  const rotatedJpeg = new Blob([withExifOrientation6(plainJpeg)], { type: 'image/jpeg' })
  await attempt(S, 'orientation EXIF appliquée (300×150 → 150×300)', async () => {
    const bitmap = await createImageBitmap(rotatedJpeg, { imageOrientation: 'from-image' })
    return { ok: bitmap.width === 150 && bitmap.height === 300, detail: `${bitmap.width}×${bitmap.height}` }
  })
  await attempt(S, 'EXIF source lisible (orientation 6 insérée)', async () => ({
    ok: readExif(new Uint8Array(await rotatedJpeg.arrayBuffer()))?.orientation === 6,
    detail: readExif(new Uint8Array(await rotatedJpeg.arrayBuffer())),
  }))
  await attempt(S, 'après compression : pas d’orientation résiduelle ni de GPS', async () => {
    const { blob, width, height } = await compress(rotatedJpeg)
    const exif = readExif(new Uint8Array(await blob.arrayBuffer()))
    // Une orientation résiduelle ≠ 1 ferait tourner l'image une 2e fois à l'affichage.
    const ok = blob.type === 'image/jpeg' && width === 150 && (exif === null || ((exif.orientation ?? 1) === 1 && !exif.gps))
    return { ok, detail: { type: blob.type, taille: `${width}×${height}`, exif_sortie: exif } }
  })
  await attempt(S, 'JPEG produit par le canvas seul (sans EXIF source)', async () => {
    const exif = readExif(plainJpeg)
    return { ok: null, detail: exif ?? 'aucun EXIF' }
  })
  await attempt(S, 'transparence détectée → PNG', async () => {
    const { blob } = await compress(await canvasToBlob(landscapeCanvas(true), 'image/png'))
    return { ok: blob.type === 'image/png', detail: blob.type }
  })
  await attempt(S, 'image opaque → JPEG', async () => {
    const { blob } = await compress(await canvasToBlob(landscapeCanvas(), 'image/png'))
    return { ok: blob.type === 'image/jpeg', detail: blob.type }
  })
  await attempt(S, 'grande image 6000×4000 → 2560×1707', async () => {
    const big = document.createElement('canvas')
    big.width = 6000
    big.height = 4000
    const context = big.getContext('2d')!
    const gradient = context.createLinearGradient(0, 0, 6000, 4000)
    gradient.addColorStop(0, '#6d9bff')
    gradient.addColorStop(1, '#9b6dff')
    context.fillStyle = gradient
    context.fillRect(0, 0, 6000, 4000)
    const source = await canvasToBlob(big, 'image/jpeg', 0.95)
    const { blob, width, height, ms } = await compress(source)
    return { ok: width === 2560 && height === 1707, detail: `${width}×${height}, ${Math.round(source.size / 1024)} Ko → ${Math.round(blob.size / 1024)} Ko en ${ms} ms` }
  })
}

// ----- D. Micro -------------------------------------------------------------------------

async function sectionMicrophone() {
  const S = 'D. Micro'
  const status = document.querySelector('#status')!
  status.textContent = 'enregistrement… parlez 3 s'
  let voice: Blob | undefined
  await attempt(S, 'autorisation + enregistrement 3 s', async () => {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
    voice = await recordStream(stream, 3000)
    stream.getTracks().forEach((track) => track.stop())
    return { ok: voice.size > 1000, detail: { mime: voice.type, octets: voice.size, ko_par_minute: Math.round((voice.size / 3) * 60 / 1024) } }
  })
  if (voice) {
    await attempt(S, 'relecture (decodeAudioData)', async () => ({ ok: true, detail: `durée ${await decode(voice!)} s` }))
    await attempt(S, 'relecture (<audio>)', async () => ({ ok: true, detail: await loadInAudioElement(voice!) }))
    const name = `micro-${os}.${extensionFor(voice.type)}`
    await attempt(S, `sauvegarder ${name}`, async () => ({ ok: true, detail: await saveSample(name, voice!) }))
  }
  status.textContent = ''
  await report()
}

// ----- Lancement ------------------------------------------------------------------------

document.querySelector('#env')!.textContent = `${os} · ${navigator.userAgent}`
document.querySelector('#mic')!.addEventListener('click', () => void sectionMicrophone())

await sectionRecording()
await sectionCrossPlayback()
await sectionImages()
await report()
