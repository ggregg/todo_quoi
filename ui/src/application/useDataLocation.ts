/**
 * Bases ouvertes (spec §5.1bis, ADR-0009) : noms, couleurs, dossiers, état ; ajout, modification,
 * détachement. Le cœur vérifie chaque dossier ; ajouter ou détacher recharge l'assemblage à chaud
 * (les fenêtres se rechargent ensuite).
 *
 * Détacher passe par la boîte de confirmation de l'app (`Confirm`) : `window.confirm` n'affiche
 * rien dans la webview de Tauri sous macOS (il répond « non » sans demander).
 */
import { computed, ref, shallowRef } from 'vue'
import type { BaseInfo, DataLocation, SettingsGateway } from '../ports/SettingsGateway'
import type { Confirm } from './useConfirm'
import { t } from '../i18n'

/** Couleurs proposées pour une nouvelle base. */
export const BASE_COLORS = ['#4f7cff', '#e8833a', '#2fa36b', '#c94f9b', '#8a63d2', '#d4a72c'] as const

export function useDataLocation(settings: SettingsGateway, confirm?: Confirm) {
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

  /**
   * Peut-on détacher cette base ? Non si c'est la seule et qu'elle est le dossier local de
   * l'appareil : sans base, TodoQuoi y reviendrait aussitôt (il en faut toujours une).
   */
  function canDetach(id: string): boolean {
    const current = location.value
    if (!current || current.kind === 'environment') return false
    return !(current.deviceOnly && current.bases.some((base) => base.id === id))
  }

  /** « Détacher » : demande confirmation, puis détache. La dernière base renvoie au dossier local. */
  async function askToRemove(id: string): Promise<void> {
    if (!canDetach(id) || !confirm) return
    const name = infoOf(id).name
    const last = (location.value?.bases.length ?? 0) <= 1
    const choice = await confirm.ask({
      title: t('bases.removeTitle', { name }),
      message: t(last ? 'bases.removeLastMessage' : 'bases.removeMessage'),
      choices: [
        { id: 'cancel', label: t('common.cancel') },
        { id: 'detach', label: t('bases.detach'), primary: true },
      ],
    })
    if (choice === 'detach') await removeBase(id)
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

  return {
    location,
    dialogOpen,
    busy,
    error,
    infoOf,
    load,
    openDialog,
    closeDialog,
    addBase,
    updateBase,
    removeBase,
    canDetach,
    askToRemove,
  }
}

export type DataLocationState = ReturnType<typeof useDataLocation>
