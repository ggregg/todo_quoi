/**
 * Indicateur de synchronisation (spec §7) : ● à jour · ⟳ téléchargement en cours · ⚠ à regarder
 * (fichiers illisibles, conflit réglé) · ✕ dossier inaccessible. Journal des conflits.
 * Le retour de la fenêtre au premier plan déclenche un passage immédiat.
 */
import { computed, ref, shallowRef } from 'vue'
import type { ConflictCopy, SyncGateway, SyncStatus } from '../ports/SyncGateway'

/** Ce qui signale le retour au premier plan (la fenêtre ; absent dans les tests). */
type FocusSource = Pick<Window, 'addEventListener' | 'removeEventListener'>

export function useSyncStatus(sync: SyncGateway, focusSource: FocusSource | null = globalThis.window ?? null) {
  const status = shallowRef<SyncStatus>({ kind: 'starting', pending: 0, unreadable: 0, conflicts: 0, message: null })
  const conflicts = shallowRef<ConflictCopy[]>([])
  const journalOpen = ref(false)

  /** Abonnement à l'état ; renvoie le désabonnement. */
  async function start(): Promise<() => void> {
    status.value = await sync.getStatus()
    const stop = await sync.onStatus((next) => (status.value = next))
    const onFocus = () => void sync.syncNow()
    focusSource?.addEventListener('focus', onFocus)
    return () => {
      stop()
      focusSource?.removeEventListener('focus', onFocus)
    }
  }

  /** Ouvre le journal (ce qui éteint le signalement des conflits). */
  async function openJournal(): Promise<void> {
    journalOpen.value = true
    conflicts.value = await sync.listConflicts()
  }

  function closeJournal(): void {
    journalOpen.value = false
  }

  /** Clé du libellé à afficher. */
  const labelKey = computed(() => `sync.${status.value.kind}` as const)

  return { status, conflicts, journalOpen, labelKey, start, openJournal, closeJournal, syncNow: () => sync.syncNow() }
}

export type SyncState = ReturnType<typeof useSyncStatus>
