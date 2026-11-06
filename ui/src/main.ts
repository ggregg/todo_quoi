/**
 * Démarrage de l'interface : choisit l'implémentation du port `TaskGateway` puis monte l'app.
 * - dans Tauri : la vraie passerelle (commandes Rust) ;
 * - dans un simple navigateur (`pnpm ui:dev`) : une passerelle en mémoire avec des données fictives.
 */
import { createApp } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import App from './presentation/App.vue'
import { taskGatewayKey, type TaskGateway } from './ports/TaskGateway'
import { TauriTaskGateway } from './infrastructure/tauri/TauriTaskGateway'
import { InMemoryTaskGateway } from './infrastructure/in-memory/InMemoryTaskGateway'
import { sampleTasks } from './infrastructure/in-memory/sampleTasks'
import './presentation/styles.css'

const gateway: TaskGateway = isTauri() ? new TauriTaskGateway() : new InMemoryTaskGateway(sampleTasks)

createApp(App).provide(taskGatewayKey, gateway).mount('#app')
