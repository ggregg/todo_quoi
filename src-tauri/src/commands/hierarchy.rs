//! Commande de déplacement (R9–R12).

use super::run;
use crate::composition::LiveState;
use crate::dto::{MoveOutcomeDto, MovementDto, parse_task_id};
use crate::error::UiError;
use tauri::State;

/// Déplace une tâche. `invoke('move_task', { taskId, movement: { kind: 'indent' } })`.
#[tauri::command]
pub async fn move_task(
    state: State<'_, LiveState>,
    task_id: String,
    movement: MovementDto,
) -> Result<MoveOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let movement = movement.into_movement()?;
    let use_case = state.current().move_task.clone();
    run(move || Ok(use_case.execute(&id, movement)?.into())).await
}
