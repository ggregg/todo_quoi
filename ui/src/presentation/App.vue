<script setup lang="ts">
/**
 * Fenêtre principale : fil d'Ariane et recherche en haut, liste arborescente à gauche (remplacée
 * par les résultats pendant une recherche), panneau de détail à droite (redimensionnable, fermé
 * par Échap). `F6` passe de la liste au panneau ; `Mod+F` place le focus dans la recherche.
 */
import AppIcon from './AppIcon.vue'
import { inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { taskGatewayKey } from '../ports/TaskGateway'
import { useConfirm } from '../application/useConfirm'
import { useSearch } from '../application/useSearch'
import { useTrash } from '../application/useTrash'
import { useDataLocation } from '../application/useDataLocation'
import { settingsGatewayKey } from '../ports/SettingsGateway'
import { syncGatewayKey } from '../ports/SyncGateway'
import { useSyncStatus } from '../application/useSyncStatus'
import { useTaskDetail } from '../application/useTaskDetail'
import { useTaskTree } from '../application/useTaskTree'
import { useVaults } from '../application/useVaults'
import { vaultGatewayKey } from '../ports/VaultGateway'
import { useToast } from '../application/useToast'
import { t } from '../i18n'
import TaskBreadcrumb from './TaskBreadcrumb.vue'
import ConfirmDialog from './ConfirmDialog.vue'
import SearchBox from './SearchBox.vue'
import SearchResults from './SearchResults.vue'
import DetailPanel from './DetailPanel.vue'
import TaskTree from './TaskTree.vue'
import ViewMenu from './ViewMenu.vue'
import ToastHost from './ToastHost.vue'
import ImageViewer from './ImageViewer.vue'
import TrashList from './TrashList.vue'
import BasesDialog from './BasesDialog.vue'
import BaseBar from './BaseBar.vue'
import SyncIndicator from './SyncIndicator.vue'
import VaultDialog from './VaultDialog.vue'

const gateway = inject(taskGatewayKey)
if (!gateway) throw new Error('TaskGateway non fourni (voir main.ts)')

const confirm = useConfirm()
const toasts = useToast()
const tree = useTaskTree(gateway, confirm, toasts, { storage: window.localStorage })
const detail = useTaskDetail(gateway, confirm, toasts)
const search = useSearch(
  gateway,
  {
    open: (hit) => tree.openPanel(hit.id),
    reveal: (hit) => {
      tree.reveal(hit.id)
      void nextTick(() => treeArea.value?.querySelector<HTMLElement>('[role="tree"]')?.focus())
    },
  },
  undefined,
  () => tree.visibleBases.value,
)
const trash = useTrash(gateway, tree.restore, confirm)
const settingsGateway = inject(settingsGatewayKey)
if (!settingsGateway) throw new Error('SettingsGateway non fourni (voir main.ts)')
const dataLocation = useDataLocation(settingsGateway)
const syncGateway = inject(syncGatewayKey)
if (!syncGateway) throw new Error('SyncGateway non fourni (voir main.ts)')
const sync = useSyncStatus(syncGateway)
const vaultGateway = inject(vaultGatewayKey)
if (!vaultGateway) throw new Error('VaultGateway non fourni (voir main.ts)')
// Un coffre qui s'ouvre ou se ferme rend des tâches lisibles ou scellées : on relit la liste et
// le panneau ouvert.
const vaults = useVaults(vaultGateway, confirm, async () => {
  await tree.refresh()
  if (tree.openedId.value !== null) await detail.load(tree.openedId.value)
})
const searchBox = ref<InstanceType<typeof SearchBox> | null>(null)

const panelWidth = ref(460)
const treeArea = ref<HTMLElement | null>(null)
const panelArea = ref<HTMLElement | null>(null)

// Le panneau affiche la tâche ouverte (et enregistre la note de la précédente avant de changer).
watch(tree.openedId, (id) => {
  if (id === null) void detail.flush()
  else void detail.load(id)
})

const cleanups: (() => void)[] = []
onMounted(async () => {
  cleanups.push(await tree.start())
  void dataLocation.load()
  void vaults.load()
  cleanups.push(await sync.start())
  cleanups.push(
    await gateway.onTasksChanged((ids, elsewhere) => {
      void detail.onExternalChange(ids, elsewhere)
      if (trash.open.value) void trash.load()
    }),
  )
  if (isTauri()) {
    // Une fenêtre de tâche demande à être réancrée dans le panneau (bouton ⇤).
    cleanups.push(await listen<{ taskId: string }>('dock-task', (event) => tree.openPanel(event.payload.taskId)))
  }
  window.addEventListener('keydown', onGlobalKey)
})
onBeforeUnmount(() => {
  cleanups.forEach((cleanup) => cleanup())
  window.removeEventListener('keydown', onGlobalKey)
})

/** `Mod+F` : rechercher. `F6` : alterner le focus entre la liste et le panneau. */
function onGlobalKey(event: KeyboardEvent): void {
  if ((event.metaKey || event.ctrlKey) && event.key === ',') {
    event.preventDefault()
    void dataLocation.openDialog()
    return
  }
  if ((event.metaKey || event.ctrlKey) && !event.altKey && event.key.toLowerCase() === 'f') {
    event.preventDefault()
    searchBox.value?.focus()
    return
  }
  if (event.key !== 'F6' || tree.openedId.value === null) return
  event.preventDefault()
  const inPanel = panelArea.value?.contains(document.activeElement) ?? false
  const target = inPanel
    ? treeArea.value?.querySelector<HTMLElement>('[role="tree"]')
    : panelArea.value?.querySelector<HTMLElement>('.title')
  target?.focus()
}

function closeTrash(): void {
  trash.hide()
  void nextTick(() => treeArea.value?.querySelector<HTMLElement>('[role="tree"]')?.focus())
}

function closePanel(): void {
  tree.closePanel()
  treeArea.value?.querySelector<HTMLElement>('[role="tree"]')?.focus()
}

async function detach(): Promise<void> {
  await detail.flush()
  await tree.openWindow(tree.openedId.value ?? undefined)
}

function startResize(event: MouseEvent): void {
  const startX = event.clientX
  const startWidth = panelWidth.value
  const move = (moveEvent: MouseEvent) => {
    panelWidth.value = Math.min(Math.max(340, startWidth + startX - moveEvent.clientX), window.innerWidth - 320)
  }
  window.addEventListener('mousemove', move)
  window.addEventListener('mouseup', () => window.removeEventListener('mousemove', move), { once: true })
}
</script>

<template>
  <div class="app">
    <header>
      <span class="brand"><AppIcon name="logo" :size="18" /> {{ t('app.title') }}</span>
      <TaskBreadcrumb :path="tree.breadcrumb.value" @navigate="tree.zoomTo" @drop-into="tree.dropAtListEnd" />
      <SearchBox ref="searchBox" :search="search" />
      <SyncIndicator :sync="sync" />
    </header>
    <BaseBar :tree="tree" :bases="dataLocation" :vaults="vaults" />
    <main>
      <div ref="treeArea" class="tree-area">
        <SearchResults
          v-if="search.active.value"
          :search="search"
          :base-info="dataLocation.infoOf"
          :show-base="tree.visibleBases.value.length > 1"
          @open="(hit) => tree.openPanel(hit.id)"
        />
        <TrashList
          v-else-if="trash.open.value"
          :trash="trash"
          :path-to="tree.pathTo"
          :bases="tree.visibleBases.value"
          :base-info="dataLocation.infoOf"
          @back="closeTrash"
        />
        <template v-else>
          <div class="list-bar"><ViewMenu :tree="tree" /></div>
          <TaskTree :tree="tree" :base-info="dataLocation.infoOf" />
          <div class="list-footer">
            <button class="add" @click="tree.addFromSelection()">
              <AppIcon name="plus" /> {{ t('tree.addTask') }}
            </button>
            <button class="add" @click="trash.show()"><AppIcon name="trash" /> {{ t('trash.open') }}</button>
          </div>
        </template>
      </div>
      <template v-if="tree.openedId.value !== null">
        <div class="splitter" @mousedown.prevent="startResize" />
        <div ref="panelArea" class="panel-area" :style="{ width: `${panelWidth}px` }">
          <DetailPanel :state="detail" :vaults="vaults" mode="panel" @close="closePanel" @detach="detach" />
        </div>
      </template>
    </main>
    <footer>
      <span v-if="tree.loaded.value">{{ t('status.count', tree.counts.value) }}</span>
      <span v-else>{{ t('status.loading') }}</span>
      <button
        v-if="dataLocation.location.value?.deviceOnly"
        type="button"
        class="location deviceOnly"
        :title="t('bases.manage')"
        @click="dataLocation.openDialog()"
      >
        <AppIcon name="folder" :size="13" /> {{ t('bases.deviceOnlyBadge') }}
      </button>
    </footer>
    <BasesDialog :state="dataLocation" />
    <VaultDialog :vaults="vaults" :base-info="dataLocation.infoOf" />
    <ConfirmDialog :confirm="confirm" />
    <ToastHost :toasts="toasts" />
    <ImageViewer />
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100%;
}
header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.brand {
  font-weight: 700;
  white-space: nowrap;
}
main {
  flex: 1;
  min-height: 0;
  display: flex;
}
.tree-area {
  flex: 1;
  min-width: 300px;
  display: flex;
  flex-direction: column;
  background: var(--surface);
}
.list-bar {
  display: flex;
  justify-content: flex-end;
  padding: 4px 12px;
  border-bottom: 1px solid var(--border);
}
.splitter {
  width: 5px;
  cursor: col-resize;
  background: var(--border);
}
.splitter:hover {
  background: var(--accent);
}
.panel-area {
  min-width: 340px;
  min-height: 0;
}
.list-footer {
  display: flex;
  justify-content: space-between;
}
.add {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  align-self: flex-start;
  margin: 6px 12px;
  background: none;
  border: none;
  color: var(--muted);
}
.add:hover {
  color: var(--accent);
}
footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 4px 12px;
  font-size: 12px;
  color: var(--muted);
  background: var(--surface);
  border-top: 1px solid var(--border);
}
@media (max-width: 760px) {
  .panel-area {
    position: fixed;
    inset: 44px 0 26px 0;
    width: auto !important;
    z-index: 20;
  }
  .splitter {
    display: none;
  }
}
.location {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 60%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 0 6px;
  font-size: 12px;
  color: var(--muted);
  background: none;
  border: none;
}
.location.deviceOnly {
  color: var(--accent);
}
.location:hover {
  text-decoration: underline;
}
</style>
