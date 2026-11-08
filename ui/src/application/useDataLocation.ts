/**
 * Bases ouvertes (spec §5.1bis, ADR-0009) : noms, couleurs, dossiers, état ; ajout, modification,
 * détachement. Le cœur vérifie chaque dossier ; ajouter ou détacher recharge l'assemblage à chaud
 * (les fenêtres se rechargent ensuite).
 */
import { computed, ref, shallowRef } from 'vue'
import type { BaseInfo, DataLocation, SettingsGateway } from '../ports/SettingsGateway'

/** Couleurs proposées pour une nouvelle base. */
export const BASE_COLORS = ['#4f7cff', '#e8833a', '#2fa36b', '#c94f9b', '#8a63d2', '#d4a72c'] as const

export function useDataLocation(settings: SettingsGateway) {
  const location = shallowRef<DataLocation | null>(null)
  const dialogOpen = ref(false)
  const busy = ref(false)
  const error = ref<string | null>(null)

  /** Informations d'une base par identifiant (nom et couleur pour le bandeau et les sections). */
  const byId = computed(() => new Map((location.value?.bases ?? []).map((base) => [base.id, base])))
  function infoOf(id: string): BaseInfo {
    return byId.value.get(id) ?? { id, name: id, color: BASE_COLORS[0], path: '', storage: 'local', problem: null }
  }

  async function load(): Promise<void> {
    location.value = await settings.getDataLocation()
  }

  async function openDialog(): Promise<void> {
    error.value = null
    dialogOpen.value = true
    await load()
  }

  function closeDialog(): void {
    if (!busy.value) dialogOpen.value = false
  }

  function addBase(name: string, color: string, path: string): Promise<void> {
    if (!name.trim() || !path.trim()) return Promise.resolve()
    return attempt(() => settings.addBase(name.trim(), color, path.trim()))
  }

  function updateBase(id: string, name: string, color: string): Promise<void> {
    return attempt(() => settings.updateBase(id, name.trim(), color))
  }

  function removeBase(id: string): Promise<void> {
    return attempt(() => settings.removeBase(id))
  }

  async function attempt(action: () => Promise<void>): Promise<void> {
    busy.value = true
    error.value = null
    try {
      await action()
      await load()
    } catch (failure) {
      error.value =
        typeof failure === 'object' && failure && 'message' in failure ? String(failure.message) : String(failure)
    } finally {
      busy.value = false
    }
  }

  return { location, dialogOpen, busy, error, infoOf, load, openDialog, closeDialog, addBase, updateBase, removeBase }
}

export type DataLocationState = ReturnType<typeof useDataLocation>
