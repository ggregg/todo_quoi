/**
 * Notifications brèves en bas de l'écran, avec action « Annuler » facultative (8 s, spec R5).
 * La dernière action annulable est aussi déclenchée par `Mod+Z`.
 */
import { shallowRef } from 'vue'

export interface Toast {
  message: string
  undo?: () => Promise<void>
  kind: 'info' | 'error'
}

const DURATION_MS = 8000

export function useToast() {
  const current = shallowRef<Toast | null>(null)
  let timer: ReturnType<typeof setTimeout> | undefined

  function show(toast: Toast): void {
    clearTimeout(timer)
    current.value = toast
    timer = setTimeout(() => (current.value = null), DURATION_MS)
  }

  function dismiss(): void {
    clearTimeout(timer)
    current.value = null
  }

  /** Exécute l'annulation proposée par la notification courante, s'il y en a une. */
  async function undo(): Promise<void> {
    const action = current.value?.undo
    dismiss()
    await action?.()
  }

  return { current, show, dismiss, undo }
}

export type Toasts = ReturnType<typeof useToast>
