/**
 * Préférences de l'appareil (spec §5.7) : nom de l'appareil, durée de la corbeille, compression
 * des images. Une instance pour toute la fenêtre (`main.ts`) : la fenêtre Réglages les modifie,
 * l'éditeur de notes lit la compression des images. Le cœur Rust vérifie et borne les valeurs.
 */
import { ref, type InjectionKey } from 'vue'
import { isUiError } from '../domain/task'
import {
  DEFAULT_IMAGE_SETTINGS,
  type ImageSettings,
  type Preferences,
  type SettingsGateway,
} from '../ports/SettingsGateway'

export function usePreferences(gateway: SettingsGateway) {
  const preferences = ref<Preferences | null>(null)
  const error = ref<string | null>(null)

  async function load(): Promise<void> {
    preferences.value = await gateway.getPreferences()
  }

  /** Modifie une partie des préférences et l'enregistre (relit ensuite ce que le cœur a gardé). */
  async function update(change: Partial<Omit<Preferences, 'deviceId'>>): Promise<void> {
    if (!preferences.value) return
    error.value = null
    try {
      await gateway.setPreferences({ ...preferences.value, ...change })
      await load()
    } catch (failure) {
      error.value = isUiError(failure) ? failure.message : String(failure)
    }
  }

  /** Compression des images (valeurs par défaut tant que les préférences ne sont pas lues). */
  function images(): ImageSettings {
    return preferences.value?.images ?? DEFAULT_IMAGE_SETTINGS
  }

  return { preferences, error, load, update, images }
}

export type PreferencesState = ReturnType<typeof usePreferences>

export const preferencesKey: InjectionKey<PreferencesState> = Symbol('Preferences')
