<script setup lang="ts">
/**
 * Fenêtre principale : fil d'Ariane, liste arborescente à gauche, panneau de détail à droite
 * (redimensionnable, fermé par Échap). `F6` passe de la liste au panneau et inversement.
 */
import { inject, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { taskGatewayKey } from '../ports/TaskGateway'
import { useConfirm } from '../application/useConfirm'
import { useTaskDetail } from '../application/useTaskDetail'
import { useTaskTree } from '../application/useTaskTree'
import { useToast } from '../application/useToast'
import { t } from '../i18n'
import TaskBreadcrumb from './TaskBreadcrumb.vue'
import ConfirmDialog from './ConfirmDialog.vue'
import DetailPanel from './DetailPanel.vue'
import TaskTree from './TaskTree.vue'
import ToastHost from './ToastHost.vue'

const gateway = inject(taskGatewayKey)
if (!gateway) throw new Error('TaskGateway non fourni (voir main.ts)')

const confirm = useConfirm()
const toasts = useToast()
const tree = useTaskTree(gateway, confirm, toasts, { storage: window.localStorage })
const detail = useTaskDetail(gateway, confirm, toasts)

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
  cleanups.push(await gateway.onTasksChanged((ids) => void detail.onExternalChange(ids)))
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

/** F6 : alterner le focus entre la liste et le panneau. */
function onGlobalKey(event: KeyboardEvent): void {
  if (event.key !== 'F6' || tree.openedId.value === null) return
  event.preventDefault()
  const inPanel = panelArea.value?.contains(document.activeElement) ?? false
  const target = inPanel
    ? treeArea.value?.querySelector<HTMLElement>('[role="tree"]')
    : panelArea.value?.querySelector<HTMLElement>('.title')
  target?.focus()
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
      <span class="brand">✓ {{ t('app.title') }}</span>
      <TaskBreadcrumb :path="tree.breadcrumb.value" @navigate="tree.zoomTo" />
    </header>
    <main>
      <div ref="treeArea" class="tree-area">
        <TaskTree :tree="tree" />
        <button class="add" @click="tree.addAtEnd()">＋ {{ t('tree.addTask') }}</button>
      </div>
      <template v-if="tree.openedId.value !== null">
        <div class="splitter" @mousedown.prevent="startResize" />
        <div ref="panelArea" class="panel-area" :style="{ width: `${panelWidth}px` }">
          <DetailPanel :state="detail" mode="panel" @close="closePanel" @detach="detach" />
        </div>
      </template>
    </main>
    <footer>
      <span v-if="tree.loaded.value">{{ t('status.count', tree.counts.value) }}</span>
      <span v-else>{{ t('status.loading') }}</span>
    </footer>
    <ConfirmDialog :confirm="confirm" />
    <ToastHost :toasts="toasts" />
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
.add {
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
</style>
