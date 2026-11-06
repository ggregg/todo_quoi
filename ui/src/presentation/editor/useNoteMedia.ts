/**
 * Médias dans l'éditeur de note : coller / glisser des fichiers, joindre un fichier, enregistrer
 * un message audio, insérer un groupe radio.
 *
 * Chaque fichier est **préparé** (image compressée dans un Worker, cf. `mediaRules`), **stocké**
 * par le cœur (`MediaGateway.store` → `../media/<sha256>.<ext>`), puis **inséré** dans la note.
 */
import type { ShallowRef } from 'vue'
import type { Editor, JSONContent } from '@tiptap/core'
import type { MediaGateway } from '../../ports/MediaGateway'
import { extensionFor, formatSize, handlingFor, WARN_MEDIA_BYTES } from '../../application/mediaRules'
import { useAudioRecorder } from '../../application/useAudioRecorder'
import { compressImage } from '../../infrastructure/media/compressImage'
import { t } from '../../i18n'

export type Notify = (kind: 'info' | 'error', message: string) => void

export function useNoteMedia(editor: ShallowRef<Editor | undefined>, media: MediaGateway, notify: Notify) {
  const recorder = useAudioRecorder()

  /** Prépare et stocke un fichier ; renvoie le bloc à insérer (ou `null` s'il est refusé). */
  async function toNode(file: File): Promise<JSONContent | null> {
    const handling = handlingFor(file)
    if (handling === 'too-large') {
      notify('error', t('media.tooLarge', { name: file.name, size: formatSize(file.size) }))
      return null
    }
    if (handling === 'attachment' && file.size > WARN_MEDIA_BYTES) {
      notify('info', t('media.large', { name: file.name, size: formatSize(file.size) }))
    }
    if (handling === 'compress-image') {
      const image = await compressImage(file)
      const src = await media.store(image.bytes, image.extension)
      return { type: 'image', attrs: { src, alt: file.name } }
    }
    const src = await media.store(new Uint8Array(await file.arrayBuffer()), extensionFor(file))
    return handling === 'keep-image'
      ? { type: 'image', attrs: { src, alt: file.name } }
      : { type: 'attachment', attrs: { src, name: file.name, size: file.size } }
  }

  /** Insère des fichiers à `position` (glisser-déposer) ou à l'emplacement du curseur. */
  async function insertFiles(files: File[], position?: number): Promise<void> {
    for (const file of files) {
      try {
        const node = await toNode(file)
        if (!node || !editor.value) continue
        const at = position ?? editor.value.state.selection.to
        editor.value.chain().focus().insertContentAt(at, node).run()
      } catch (error) {
        notify('error', t('error.prefix', { message: errorMessage(error) }))
      }
    }
  }

  /** 🎙 : démarre l'enregistrement, ou l'arrête et insère le message audio. */
  async function toggleRecording(): Promise<void> {
    try {
      if (recorder.state.value === 'idle') return await recorder.start()
      const recording = await recorder.stop()
      if (!recording || !editor.value) return
      const src = await media.store(recording.bytes, recording.extension)
      editor.value
        .chain()
        .focus()
        .insertContent({
          type: 'audio',
          attrs: { src, duration: recording.duration, mime: recording.mime },
        })
        .run()
    } catch (error) {
      recorder.cancel()
      notify('error', t('media.micError', { message: errorMessage(error) }))
    }
  }

  /** ⦿ : insère un groupe radio de deux options, la première cochée. */
  function insertRadioGroup(): void {
    editor.value
      ?.chain()
      .focus()
      .insertContent({
        type: 'radioGroup',
        attrs: { label: t('editor.radioLabel') },
        content: [
          {
            type: 'radioOption',
            attrs: { checked: true },
            content: [{ type: 'text', text: t('editor.radioOption', { n: 1 }) }],
          },
          {
            type: 'radioOption',
            attrs: { checked: false },
            content: [{ type: 'text', text: t('editor.radioOption', { n: 2 }) }],
          },
        ],
      })
      .run()
  }

  return { recorder, insertFiles, toggleRecording, cancelRecording: recorder.cancel, insertRadioGroup }
}

function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error && 'message' in error) return String((error as { message: unknown }).message)
  return String(error)
}
