//! Commandes de corbeille (R5, R7, R8) ; mise à la corbeille et restauration acceptent une
//! sélection (spec §5.5).

use super::run;
use crate::composition::LiveState;
use crate::dto::{DeleteOutcomeDto, RestoreOutcomeDto, parse_task_id, parse_task_ids};
use crate::error::UiError;
use tauri::State;

/// Met des tâches et leurs sous-tâches à la corbeille. `invoke('delete_tasks', { taskIds })`.
#[tauri::command]
pub async fn delete_tasks(
    state: State<'_, LiveState>,
    task_ids: Vec<String>,
) -> Result<DeleteOutcomeDto, UiError> {
    let ids = parse_task_ids(&task_ids)?;
    let use_case = state.current().delete_task.clone();
    run(move || Ok(use_case.execute(&ids)?.into())).await
}

/// Restaure des tâches (ou annule une suppression). `invoke('restore_tasks', { taskIds })`.
#[tauri::command]
pub async fn restore_tasks(
    state: State<'_, LiveState>,
    task_ids: Vec<String>,
) -> Result<RestoreOutcomeDto, UiError> {
    let ids = parse_task_ids(&task_ids)?;
    let use_case = state.current().restore_task.clone();
    run(move || Ok(use_case.execute(&ids)?.into())).await
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
