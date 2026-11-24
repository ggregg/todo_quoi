//! Commandes d'archivage (R13, R16, ADR-0015), pour une tâche ou une sélection (spec §5.5).

use super::run;
use crate::composition::LiveState;
use crate::dto::{ArchiveImpactDto, ArchiveOutcomeDto, RestoreOutcomeDto, parse_task_ids};
use crate::error::UiError;
use tauri::State;

/// Combien de tâches l'archivage emporterait, dont combien ouvertes ?
/// `invoke('get_archive_impact', { taskIds })`.
#[tauri::command]
pub async fn get_archive_impact(
    state: State<'_, LiveState>,
    task_ids: Vec<String>,
) -> Result<ArchiveImpactDto, UiError> {
    let ids = parse_task_ids(&task_ids)?;
    let use_case = state.current().archive.impact.clone();
    run(move || Ok(use_case.execute(&ids)?.into())).await
}

/// Archive des tâches et leurs sous-tâches actives. `invoke('archive_tasks', { taskIds })`.
#[tauri::command]
pub async fn archive_tasks(
    state: State<'_, LiveState>,
    task_ids: Vec<String>,
) -> Result<ArchiveOutcomeDto, UiError> {
    let ids = parse_task_ids(&task_ids)?;
    let use_case = state.current().archive.archive.clone();
    run(move || Ok(use_case.execute(&ids)?.into())).await
}

/// Restaure des archives (ou annule un archivage). `invoke('unarchive_tasks', { taskIds })`.
#[tauri::command]
pub async fn unarchive_tasks(
    state: State<'_, LiveState>,
    task_ids: Vec<String>,
) -> Result<RestoreOutcomeDto, UiError> {
    let ids = parse_task_ids(&task_ids)?;
    let use_case = state.current().archive.unarchive.clone();
    run(move || Ok(use_case.execute(&ids)?.into())).await
}
