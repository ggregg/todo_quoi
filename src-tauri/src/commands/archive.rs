//! Commandes d'archivage (R13, R16, ADR-0015).

use super::run;
use crate::composition::LiveState;
use crate::dto::{ArchiveImpactDto, ArchiveOutcomeDto, RestoreOutcomeDto, parse_task_id};
use crate::error::UiError;
use tauri::State;

/// Combien de tâches l'archivage emporterait, dont combien ouvertes ?
/// `invoke('get_archive_impact', { taskId })`.
#[tauri::command]
pub async fn get_archive_impact(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<ArchiveImpactDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().archive.impact.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}

/// Archive une tâche et ses sous-tâches actives. `invoke('archive_task', { taskId })`.
#[tauri::command]
pub async fn archive_task(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<ArchiveOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().archive.archive.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}

/// Restaure une tâche archivée (ou annule un archivage). `invoke('unarchive_task', { taskId })`.
#[tauri::command]
pub async fn unarchive_task(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<RestoreOutcomeDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().archive.unarchive.clone();
    run(move || Ok(use_case.execute(&id)?.into())).await
}
