//! Commandes de lecture (la liste mise en forme est dans [`super::views`]).

use super::run;
use crate::composition::LiveState;
use crate::dto::{TaskDto, parse_task_id};
use crate::error::UiError;
use serde::Serialize;
use tauri::State;

/// Corbeille (racines supprimées, les plus récentes d'abord). `invoke('list_trash')`.
// [rust] `State<'_, LiveState>` : Tauri injecte l'état enregistré au démarrage (`manage`).
#[tauri::command]
pub async fn list_trash(state: State<'_, LiveState>) -> Result<Vec<TaskDto>, UiError> {
    let use_case = state.current().list_trash.clone();
    run(move || Ok(use_case.execute()?.iter().map(TaskDto::from).collect())).await
}

/// Impact d'une clôture (R2).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseImpactDto {
    /// Sous-tâches encore ouvertes.
    pub open_descendants: usize,
}

/// Combien de sous-tâches seraient concernées par la clôture ? `invoke('get_close_impact', { taskId })`.
#[tauri::command]
pub async fn get_close_impact(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<CloseImpactDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().get_close_impact.clone();
    run(move || {
        Ok(CloseImpactDto {
            open_descendants: use_case.execute(&id)?.open_descendants,
        })
    })
    .await
}
