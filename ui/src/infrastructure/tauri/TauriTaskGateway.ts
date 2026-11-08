/**
 * Implémentation réelle du port `TaskGateway` : une méthode = une commande Tauri
 * (`src-tauri/src/commands/`). Les noms d'arguments sont en camelCase (Tauri les convertit).
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { TaskGateway } from '../../ports/TaskGateway'
import type { ListView, Listing } from '../../domain/view'
import type {
  CloseImpact,
  CloseOutcome,
  CloseScope,
  DeleteOutcome,
  MoveOutcome,
  Movement,
  NoteDocument,
  Position,
  ReopenOutcome,
  RestoreOutcome,
  SearchHit,
  Task,
  TaskDetail,
  TaskId,
} from '../../domain/task'

export class TauriTaskGateway implements TaskGateway {
  listTasks(): Promise<Listing> {
    return invoke('list_tasks')
  }
  customizeListView(taskId: TaskId | null, base: string | null, view: ListView): Promise<void> {
    return invoke('customize_list_view', { taskId, base, view })
  }
  inheritListView(taskId: TaskId): Promise<void> {
    return invoke('inherit_list_view', { taskId })
  }
  listTrash(): Promise<Task[]> {
    return invoke('list_trash')
  }
  searchTasks(query: string, bases: string[]): Promise<SearchHit[]> {
    return invoke('search_tasks', { query, bases })
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
  getTaskDetail(taskId: TaskId): Promise<TaskDetail> {
    return invoke('get_task_detail', { taskId })
  }
  updateNote(taskId: TaskId, note: NoteDocument): Promise<void> {
    return invoke('update_note', { taskId, note })
  }
  openTaskWindow(taskId: TaskId): Promise<void> {
    return invoke('open_task_window', { taskId })
  }
  keepLocalVersion(taskId: TaskId): Promise<void> {
    return invoke('keep_local_version', { taskId })
  }
  async onTasksChanged(listener: (ids: TaskId[], elsewhere: boolean) => void): Promise<() => void> {
    return listen<{ ids: TaskId[]; elsewhere: boolean }>('tasks-changed', (event) =>
      listener(event.payload.ids, event.payload.elsewhere),
    )
  }
}
