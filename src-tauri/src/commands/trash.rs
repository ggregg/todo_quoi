//! Commandes de corbeille (R5, R7, R8).

use super::run;
use crate::composition::LiveState;
use crate::dto::{DeleteOutcomeDto, RestoreOutcomeDto, parse_task_id};
use crate::error::UiError;
use tauri::State;

/// Met une tâche et ses sous-tâches à la corbeille. `invoke('delete_task', { taskId })`.
#[tauri::command]
pub async fn delete_task(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<DeleteOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().delete_task.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}

/// Restaure une tâche (ou annule une suppression). `invoke('restore_task', { taskId })`.
#[tauri::command]
pub async fn restore_task(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<RestoreOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().restore_task.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}

/// Supprime définitivement une tâche de la corbeille, avec ses sous-tâches à la corbeille ;
/// renvoie les tâches effacées. `invoke('purge_task', { taskId })`.
#[tauri::command]
pub async fn purge_task(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<Vec<String>, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().trash.purge.clone();
    run(move || {
        Ok(use_case
            .execute(&id)?
            .iter()
            .map(ToString::to_string)
            .collect())
    })
    .await
}
