//! Commandes de corbeille (R5, R7).

use super::run;
use crate::composition::AppState;
use crate::dto::{DeleteOutcomeDto, RestoreOutcomeDto, parse_task_id};
use crate::error::UiError;
use tauri::State;

/// Met une tâche et ses sous-tâches à la corbeille. `invoke('delete_task', { taskId })`.
#[tauri::command]
pub async fn delete_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<DeleteOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.delete_task.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}

/// Restaure une tâche (ou annule une suppression). `invoke('restore_task', { taskId })`.
#[tauri::command]
pub async fn restore_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<RestoreOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.restore_task.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}
