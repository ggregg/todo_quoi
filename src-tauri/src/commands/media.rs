//! Commandes des médias : stocker (binaire brut), ouvrir avec l'application par défaut,
//! révéler dans le Finder / l'Explorateur (via [`crate::system_open`]).

use super::run;
use crate::composition::LiveState;
use crate::dto::parse_task_id;
use crate::error::UiError;
use crate::system_open::{open_with_system, reveal_in_file_manager};
use tauri::State;
use tauri::ipc::{InvokeBody, Request};

/// En-tête qui transporte l'extension du fichier envoyé (`jpg`, `webm`, `pdf`…).
pub const EXTENSION_HEADER: &str = "x-media-extension";
/// En-tête qui dit pour quelle tâche (donc dans quelle base) ranger le média.
pub const TASK_HEADER: &str = "x-task-id";

/// Stocke un média et renvoie le lien à écrire dans la note (`../media/<nom>`).
///
/// Côté TypeScript : `invoke('store_media', octets, { headers: { 'x-media-extension': 'jpg' } })`.
/// Les octets voyagent **bruts** : un tableau JSON de 50 Mo serait beaucoup trop lent.
#[tauri::command]
pub async fn store_media(
    state: State<'_, LiveState>,
    request: Request<'_>,
) -> Result<String, UiError> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(UiError::invalid_input(
            "le média doit être envoyé en binaire brut",
        ));
    };
    let task_id = request
        .headers()
        .get(TASK_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| UiError::invalid_input("en-tête x-task-id manquant"))
        .and_then(parse_task_id)?;
    let extension = request
        .headers()
        .get(EXTENSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("bin")
        .to_string();
    let bytes = bytes.clone();
    let use_case = state.current().media.store.clone();
    run(move || Ok(use_case.execute(&task_id, &bytes, &extension)?.note_link())).await
}

/// Ouvre le média avec l'application par défaut de l'OS. `invoke('open_media', { name })`.
#[tauri::command]
pub async fn open_media(state: State<'_, LiveState>, name: String) -> Result<(), UiError> {
    let use_case = state.current().media.read.clone();
    let path = run(move || use_case.local_path(&name)).await?;
    open_with_system(path.as_os_str())
}

/// Montre le média dans le Finder / l'Explorateur. `invoke('reveal_media', { name })`.
#[tauri::command]
pub async fn reveal_media(state: State<'_, LiveState>, name: String) -> Result<(), UiError> {
    let use_case = state.current().media.read.clone();
    let path = run(move || use_case.local_path(&name)).await?;
    reveal_in_file_manager(&path)
}
