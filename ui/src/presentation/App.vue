<script setup lang="ts">
/**
 * Écran principal (I1) : fil d'Ariane, liste arborescente, barre d'état.
 * Le panneau de détail et l'éditeur de notes arrivent en I2.
 */
import { inject, onBeforeUnmount, onMounted } from 'vue'
import { taskGatewayKey } from '../ports/TaskGateway'
import { useConfirm } from '../application/useConfirm'
import { useTaskTree } from '../application/useTaskTree'
import { useToast } from '../application/useToast'
import { t } from '../i18n'
import Breadcrumb from './Breadcrumb.vue'
import ConfirmDialog from './ConfirmDialog.vue'
import TaskTree from './TaskTree.vue'
import ToastHost from './ToastHost.vue'

const gateway = inject(taskGatewayKey)
if (!gateway) throw new Error('TaskGateway non fourni (voir main.ts)')

const confirm = useConfirm()
const toasts = useToast()
const tree = useTaskTree(gateway, confirm, toasts, { storage: window.localStorage })

let stop: (() => void) | undefined
onMounted(async () => {
  stop = await tree.start()
})
onBeforeUnmount(() => stop?.())
</script>

<template>
  <div class="app">
    <header>
      <span class="brand">✓ {{ t('app.title') }}</span>
      <Breadcrumb :path="tree.breadcrumb.value" @navigate="tree.zoomTo" />
    </header>
    <main>
      <TaskTree :tree="tree" />
      <button class="add" @click="tree.addAtEnd()">＋ {{ t('tree.addTask') }}</button>
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
  flex-direction: column;
  background: var(--surface);
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
</style>
