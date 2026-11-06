<script setup lang="ts">
/**
 * Fenêtre de tâche (spec §5.4bis) : la tâche seule, avec le même panneau de détail.
 * - ✕ / Échap : ferme la fenêtre (la note en attente est d'abord enregistrée) ;
 * - ⇤ : réancre la tâche dans le panneau de la fenêtre principale, puis ferme celle-ci.
 */
import { inject, onBeforeUnmount, onMounted } from 'vue'
import { emitTo } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { taskGatewayKey } from '../ports/TaskGateway'
import type { TaskId } from '../domain/task'
import { useConfirm } from '../application/useConfirm'
import { useTaskDetail } from '../application/useTaskDetail'
import { useToast } from '../application/useToast'
import ConfirmDialog from './ConfirmDialog.vue'
import DetailPanel from './DetailPanel.vue'
import ToastHost from './ToastHost.vue'

const props = defineProps<{ taskId: TaskId }>()
const gateway = inject(taskGatewayKey)
if (!gateway) throw new Error('TaskGateway non fourni (voir main.ts)')

const confirm = useConfirm()
const toasts = useToast()
const detail = useTaskDetail(gateway, confirm, toasts)

let unsubscribe: (() => void) | undefined
onMounted(async () => {
  await detail.load(props.taskId)
  unsubscribe = await gateway.onTasksChanged((ids) => void detail.onExternalChange(ids))
})
onBeforeUnmount(() => unsubscribe?.())

async function close(): Promise<void> {
  await detail.flush()
  await getCurrentWindow().close()
}

/** Événement écouté par la fenêtre principale (`App.vue`) : ouvrir cette tâche dans son panneau. */
async function dock(): Promise<void> {
  await detail.flush()
  await emitTo('main', 'dock-task', { taskId: props.taskId })
  await getCurrentWindow().close()
}
</script>

<template>
  <div class="window">
    <DetailPanel :state="detail" mode="window" @close="close" @dock="dock" />
    <ConfirmDialog :confirm="confirm" />
    <ToastHost :toasts="toasts" />
  </div>
</template>

<style scoped>
.window {
  height: 100%;
}
</style>
