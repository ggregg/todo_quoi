/**
 * Implémentation réelle du port `TaskGateway` : une méthode = une commande Tauri
 * (`src-tauri/src/commands/`). Les noms d'arguments sont en camelCase (Tauri les convertit).
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { TaskGateway } from '../../ports/TaskGateway'
import type {
  CloseImpact,
  CloseOutcome,
  CloseScope,
  DeleteOutcome,
  MoveOutcome,
  Movement,
  Position,
  ReopenOutcome,
  RestoreOutcome,
  Task,
  TaskId,
} from '../../domain/task'

export class TauriTaskGateway implements TaskGateway {
  listTasks(): Promise<Task[]> {
    return invoke('list_tasks')
  }
  listTrash(): Promise<Task[]> {
    return invoke('list_trash')
  }
  createTask(title: string, position: Position): Promise<TaskId> {
    return invoke('create_task', { title, position })
  }
  renameTask(taskId: TaskId, title: string): Promise<void> {
    return invoke('rename_task', { taskId, title })
  }
  setTags(taskId: TaskId, tags: string[]): Promise<void> {
    return invoke('set_tags', { taskId, tags })
  }
  getCloseImpact(taskId: TaskId): Promise<CloseImpact> {
    return invoke('get_close_impact', { taskId })
  }
  closeTask(taskId: TaskId, scope: CloseScope): Promise<CloseOutcome> {
    return invoke('close_task', { taskId, scope })
  }
  reopenTask(taskId: TaskId): Promise<ReopenOutcome> {
    return invoke('reopen_task', { taskId })
  }
  moveTask(taskId: TaskId, movement: Movement): Promise<MoveOutcome> {
    return invoke('move_task', { taskId, movement })
  }
  deleteTask(taskId: TaskId): Promise<DeleteOutcome> {
    return invoke('delete_task', { taskId })
  }
  restoreTask(taskId: TaskId): Promise<RestoreOutcome> {
    return invoke('restore_task', { taskId })
  }
  async onTasksChanged(listener: (ids: TaskId[]) => void): Promise<() => void> {
    return listen<{ ids: TaskId[] }>('tasks-changed', (event) => listener(event.payload.ids))
  }
}
