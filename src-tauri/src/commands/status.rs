//! Commandes de clôture et de réouverture (R1–R3).

use super::run;
use crate::composition::AppState;
use crate::dto::{CloseOutcomeDto, CloseScopeDto, ReopenOutcomeDto, parse_task_id};
use crate::error::UiError;
use tauri::State;
use tq_application::use_cases::close_task::CloseTaskCommand;

/// Clôture une tâche. `invoke('close_task', { taskId, scope: 'taskOnly' })`.
#[tauri::command]
pub async fn close_task(
    state: State<'_, AppState>,
    task_id: String,
    scope: CloseScopeDto,
) -> Result<CloseOutcomeDto, UiError> {
    let command = CloseTaskCommand {
        task_id: parse_task_id(&task_id)?,
        scope: scope.into(),
    };
    let use_case = state.close_task.clone();
    run(move || Ok(use_case.execute(command)?.into())).await
}

/// Rouvre une tâche. `invoke('reopen_task', { taskId })`.
#[tauri::command]
pub async fn reopen_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<ReopenOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.reopen_task.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}
