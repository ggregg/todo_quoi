/**
 * Démarrage de l'interface.
 * 1. Choisit l'implémentation des ports `TaskGateway` et `MediaGateway` :
 *    - dans Tauri : la vraie passerelle (commandes Rust) ;
 *    - dans un simple navigateur (`pnpm ui:dev`) : une passerelle en mémoire, données fictives.
 * 2. Choisit l'écran : `#task=<id>` → fenêtre de tâche seule ; sinon la fenêtre principale.
 */
import { createApp, h } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import App from './presentation/App.vue'
import TaskWindow from './presentation/TaskWindow.vue'
import { taskGatewayKey, type TaskGateway } from './ports/TaskGateway'
import { mediaGatewayKey, type MediaGateway } from './ports/MediaGateway'
import { TauriTaskGateway } from './infrastructure/tauri/TauriTaskGateway'
import { TauriMediaGateway } from './infrastructure/tauri/TauriMediaGateway'
import { InMemoryTaskGateway } from './infrastructure/in-memory/InMemoryTaskGateway'
import { InMemoryMediaGateway } from './infrastructure/in-memory/InMemoryMediaGateway'
import { sampleTasks } from './infrastructure/in-memory/sampleTasks'
import './presentation/styles.css'

const gateway: TaskGateway = isTauri() ? new TauriTaskGateway() : new InMemoryTaskGateway(sampleTasks)
const media: MediaGateway = isTauri() ? new TauriMediaGateway() : new InMemoryMediaGateway()
const taskId = new URLSearchParams(window.location.hash.slice(1)).get('task')

const root = taskId ? { render: () => h(TaskWindow, { taskId }) } : App
createApp(root).provide(taskGatewayKey, gateway).provide(mediaGatewayKey, media).mount('#app')
