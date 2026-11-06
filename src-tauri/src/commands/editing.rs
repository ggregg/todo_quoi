//! Commandes de création et de modification du contenu d'une tâche.

use super::run;
use crate::composition::AppState;
use crate::dto::{PositionDto, parse_task_id};
use crate::error::UiError;
use tauri::State;
use tq_application::use_cases::create_task::CreateTaskCommand;

/// Crée une tâche et renvoie son identifiant.
/// `invoke('create_task', { title, position: { kind: 'atEnd', parentId: null } })`.
#[tauri::command]
pub async fn create_task(
    state: State<'_, AppState>,
    title: String,
    position: PositionDto,
) -> Result<String, UiError> {
    let command = CreateTaskCommand {
        title,
        position: position.into_position()?,
    };
    let use_case = state.create_task.clone();
    run(move || Ok(use_case.execute(command)?.to_string())).await
}

/// Renomme une tâche. `invoke('rename_task', { taskId, title })`.
#[tauri::command]
pub async fn rename_task(
    state: State<'_, AppState>,
    task_id: String,
    title: String,
) -> Result<(), UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.rename_task.clone();
    run(move || use_case.execute(&id, &title)).await
}

/// Remplace les étiquettes. `invoke('set_tags', { taskId, tags: ['release'] })`.
#[tauri::command]
pub async fn set_tags(
    state: State<'_, AppState>,
    task_id: String,
    tags: Vec<String>,
) -> Result<(), UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.set_tags.clone();
    run(move || use_case.execute(&id, &tags)).await
}
