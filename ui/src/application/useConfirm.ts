/**
 * Dialogue de confirmation « à la promesse » : `const choice = await confirm.ask({...})`.
 * Le composant `ConfirmDialog.vue` affiche la question courante et appelle `answer()`.
 * Échap ou clic hors du dialogue = choix `cancel`.
 */
import { shallowRef, type InjectionKey } from 'vue'

export interface Choice<Id extends string = string> {
  id: Id
  label: string
  primary?: boolean
}

export interface Question<Id extends string = string> {
  title: string
  message: string
  choices: Choice<Id>[]
}

interface Pending {
  question: Question
  resolve: (choice: string) => void
}

export function useConfirm() {
  const current = shallowRef<Pending | null>(null)

  function ask<Id extends string>(question: Question<Id>): Promise<Id | 'cancel'> {
    current.value?.resolve('cancel')
    return new Promise((resolve) => {
      current.value = { question, resolve: resolve as (choice: string) => void }
    })
  }

  function answer(choice: string): void {
    const pending = current.value
    current.value = null
    pending?.resolve(choice)
  }

  return { current, ask, answer }
}

export type Confirm = ReturnType<typeof useConfirm>

/** Confirmation de la fenêtre, pour les composants profonds (éditeur de note). */
export const confirmKey: InjectionKey<Confirm> = Symbol('Confirm')
