<script setup lang="ts">
/**
 * Fenêtre de tâche (spec §5.4bis) : la tâche seule, avec le même panneau de détail.
 * - ✕ / Échap : ferme la fenêtre (la note en attente est d'abord enregistrée) ;
 * - ⇤ : réancre la tâche dans le panneau de la fenêtre principale, puis ferme celle-ci ;
 * - clic sur une sous-tâche : la fenêtre passe sur elle ;
 * - 🔒 / 🔓 et « Déverrouiller pour la lire » : comme dans le panneau (coffre de sa base). Un
 *   coffre ouvert ou fermé ailleurs (`vaults-changed`) recharge la tâche.
 */
import { inject, onBeforeUnmount, onMounted, provide, ref } from 'vue'
import { emitTo } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { taskGatewayKey } from '../ports/TaskGateway'
import { vaultGatewayKey } from '../ports/VaultGateway'
import { settingsGatewayKey } from '../ports/SettingsGateway'
import { useVaults } from '../application/useVaults'
import { useDataLocation } from '../application/useDataLocation'
import VaultDialog from './VaultDialog.vue'
import type { TaskId } from '../domain/task'
import { confirmKey, useConfirm } from '../application/useConfirm'
import { useTaskDetail } from '../application/useTaskDetail'
import { useToast } from '../application/useToast'
import ConfirmDialog from './ConfirmDialog.vue'
import DetailPanel from './DetailPanel.vue'
import ToastHost from './ToastHost.vue'
import ImageViewer from './ImageViewer.vue'

const props = defineProps<{ taskId: TaskId }>()
const gateway = inject(taskGatewayKey)
if (!gateway) throw new Error('TaskGateway non fourni (voir main.ts)')

const confirm = useConfirm()
provide(confirmKey, confirm)
const toasts = useToast()
const detail = useTaskDetail(gateway, confirm, toasts)
/** Tâche affichée : celle de l'ouverture, puis la sous-tâche sur laquelle on est allé. */
const currentId = ref<TaskId>(props.taskId)
const vaultGateway = inject(vaultGatewayKey)
if (!vaultGateway) throw new Error('VaultGateway non fourni (voir main.ts)')
const settingsGateway = inject(settingsGatewayKey)
if (!settingsGateway) throw new Error('SettingsGateway non fourni (voir main.ts)')
/** Noms des bases (titre du dialogue de mot de passe). */
const dataLocation = useDataLocation(settingsGateway)
const vaults = useVaults(vaultGateway, confirm, () => detail.load(currentId.value))

const cleanups: (() => void)[] = []
onMounted(async () => {
  await detail.load(currentId.value)
  void dataLocation.load()
  void vaults.load()
  cleanups.push(await vaults.start())
  cleanups.push(await gateway.onTasksChanged((ids, elsewhere) => void detail.onExternalChange(ids, elsewhere)))
})
onBeforeUnmount(() => cleanups.forEach((cleanup) => cleanup()))

/** Clic sur une sous-tâche (la note en cours est déjà enregistrée par le panneau). */
async function openTask(id: TaskId): Promise<void> {
  currentId.value = id
  await detail.load(id)
}

async function close(): Promise<void> {
  await detail.flush()
  await getCurrentWindow().close()
}

/** Événement écouté par la fenêtre principale (`App.vue`) : ouvrir cette tâche dans son panneau. */
async function dock(): Promise<void> {
  await detail.flush()
  await emitTo('main', 'dock-task', { taskId: currentId.value })
  await getCurrentWindow().close()
}
</script>

<template>
  <div class="window">
    <DetailPanel :state="detail" :vaults="vaults" mode="window" @close="close" @dock="dock" @open-task="openTask" />
    <VaultDialog :vaults="vaults" :base-info="dataLocation.infoOf" />
    <ConfirmDialog :confirm="confirm" />
    <ToastHost :toasts="toasts" />
    <ImageViewer />
  </div>
</template>

<style scoped>
.window {
  height: 100%;
}
</style>
