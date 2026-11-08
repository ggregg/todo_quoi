//! Synchronisation (spec §7) : état de l'indicateur, passage immédiat, journal des conflits.

use super::run;
use crate::composition::LiveState;
use crate::dto::parse_task_id;
use crate::error::UiError;
use crate::scanner::{SyncMonitor, SyncStatusDto};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

/// Dernier état de la synchronisation. `invoke('get_sync_status')`.
#[tauri::command]
pub fn get_sync_status(monitor: State<'_, Arc<SyncMonitor>>) -> SyncStatusDto {
    monitor.status()
}

/// Demande un passage immédiat (retour au premier plan, clic sur l'indicateur).
/// `invoke('sync_now')`.
#[tauri::command]
pub fn sync_now(monitor: State<'_, Arc<SyncMonitor>>) {
    monitor.wake();
}

/// « Garder la mienne » : met de côté la version venue d'un autre appareil, avant que
/// l'interface n'enregistre la sienne. `invoke('keep_local_version', { taskId })`.
#[tauri::command]
pub async fn keep_local_version(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<(), UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().sync.keep_local_version.clone();
    run(move || use_case.execute(&id)).await
}

/// Une version écartée par un conflit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictCopyDto {
    /// Tâche concernée (si lisible).
    pub task_id: Option<String>,
    /// Titre de cette version (si lisible).
    pub title: Option<String>,
    /// Fichier dans `.conflicts/`.
    pub file_name: String,
    /// Date de mise à l'écart (RFC 3339).
    pub set_aside_at: String,
}

/// Journal des conflits ; le consulter éteint leur signalement. `invoke('list_conflicts')`.
#[tauri::command]
pub async fn list_conflicts(
    state: State<'_, LiveState>,
    monitor: State<'_, Arc<SyncMonitor>>,
) -> Result<Vec<ConflictCopyDto>, UiError> {
    monitor.acknowledge_conflicts();
    monitor.wake();
    let use_case = state.current().sync.list_conflicts.clone();
    run(move || {
        Ok(use_case
            .execute()?
            .into_iter()
            .map(|copy| ConflictCopyDto {
                task_id: copy.task_id.map(|id| id.to_string()),
                title: copy.title,
                file_name: copy.file_name,
                set_aside_at: copy.set_aside_at.to_rfc3339(),
            })
            .collect())
    })
    .await
}
