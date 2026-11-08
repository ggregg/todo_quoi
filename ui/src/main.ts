/**
 * Démarrage de l'interface.
 * 1. Choisit l'implémentation des ports `TaskGateway`, `MediaGateway`, `LinkGateway`, `SettingsGateway`, `SyncGateway`
 *    et `VaultGateway` :
 *    - dans Tauri : la vraie passerelle (commandes Rust) ;
 *    - dans un simple navigateur (`pnpm ui:dev`) : une passerelle en mémoire, données fictives.
 * 2. Choisit l'écran : `#task=<id>` → fenêtre de tâche seule ; sinon la fenêtre principale.
 * 3. Recharge la fenêtre quand la liste des bases change (ajout, détachement : ADR-0009) — le
 *    cœur a déjà reconstruit son assemblage, sans redémarrer.
 */
import { createApp, h } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import App from './presentation/App.vue'
import TaskWindow from './presentation/TaskWindow.vue'
import { taskGatewayKey, type TaskGateway } from './ports/TaskGateway'
import { mediaGatewayKey, type MediaGateway } from './ports/MediaGateway'
import { linkGatewayKey, type LinkGateway } from './ports/LinkGateway'
import { settingsGatewayKey, type SettingsGateway } from './ports/SettingsGateway'
import { syncGatewayKey, type SyncGateway } from './ports/SyncGateway'
import { folderPickerKey, type FolderPicker } from './ports/FolderPicker'
import { vaultGatewayKey, type VaultGateway } from './ports/VaultGateway'
import { TauriVaultGateway } from './infrastructure/tauri/TauriVaultGateway'
import { InMemoryVaultGateway } from './infrastructure/in-memory/InMemoryVaultGateway'
import { TauriFolderPicker } from './infrastructure/tauri/TauriFolderPicker'
import { BrowserFolderPicker } from './infrastructure/in-memory/BrowserFolderPicker'
import { TauriTaskGateway } from './infrastructure/tauri/TauriTaskGateway'
import { TauriMediaGateway } from './infrastructure/tauri/TauriMediaGateway'
import { TauriLinkGateway } from './infrastructure/tauri/TauriLinkGateway'
import { BrowserLinkGateway } from './infrastructure/in-memory/BrowserLinkGateway'
import { TauriSettingsGateway } from './infrastructure/tauri/TauriSettingsGateway'
import { InMemorySettingsGateway } from './infrastructure/in-memory/InMemorySettingsGateway'
import { TauriSyncGateway } from './infrastructure/tauri/TauriSyncGateway'
import { InMemorySyncGateway } from './infrastructure/in-memory/InMemorySyncGateway'
import { InMemoryTaskGateway } from './infrastructure/in-memory/InMemoryTaskGateway'
import { InMemoryMediaGateway } from './infrastructure/in-memory/InMemoryMediaGateway'
import { sampleTasks } from './infrastructure/in-memory/sampleTasks'
import './presentation/styles.css'

const memoryTasks = isTauri() ? null : new InMemoryTaskGateway(sampleTasks)
const gateway: TaskGateway = memoryTasks ?? new TauriTaskGateway()
const vault: VaultGateway = memoryTasks
  ? new InMemoryVaultGateway(memoryTasks, async () => (await memoryTasks.listTasks()).bases)
  : new TauriVaultGateway()
const media: MediaGateway = isTauri() ? new TauriMediaGateway() : new InMemoryMediaGateway()
const links: LinkGateway = isTauri() ? new TauriLinkGateway() : new BrowserLinkGateway()
const settings: SettingsGateway = isTauri() ? new TauriSettingsGateway() : new InMemorySettingsGateway()
const sync: SyncGateway = isTauri() ? new TauriSyncGateway() : new InMemorySyncGateway()
const folders: FolderPicker = isTauri() ? new TauriFolderPicker() : new BrowserFolderPicker()
const taskId = new URLSearchParams(window.location.hash.slice(1)).get('task')

const root = taskId ? { render: () => h(TaskWindow, { taskId }) } : App
createApp(root)
  .provide(taskGatewayKey, gateway)
  .provide(mediaGatewayKey, media)
  .provide(linkGatewayKey, links)
  .provide(settingsGatewayKey, settings)
  .provide(syncGatewayKey, sync)
  .provide(folderPickerKey, folders)
  .provide(vaultGatewayKey, vault)
  .mount('#app')

if (isTauri()) void listen('bases-changed', () => window.location.reload())
