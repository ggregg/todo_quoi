//! Commandes des médias : stocker (binaire brut), ouvrir avec l'application par défaut,
//! révéler dans le Finder / l'Explorateur (via [`crate::system_open`]). Un média **chiffré**
//! s'ouvre par une copie déchiffrée temporaire ([`crate::plain_copies`]) ; il ne se révèle pas.

use super::run;
use crate::composition::LiveState;
use crate::dto::parse_task_id;
use crate::error::UiError;
use crate::plain_copies;
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
    let media = tq_domain::MediaRef::parse(&name)
        .map_err(|error| UiError::from(tq_application::AppError::from(error)))?;
    if !media.is_sealed() {
        let path = run(move || use_case.local_path(&name)).await?;
        return open_with_system(path.as_os_str());
    }
    // Chiffré : déchiffré dans une copie temporaire `<empreinte>.<ext>`, effacée au verrouillage.
    let (_, bytes) = run(move || use_case.execute(&name)).await?;
    let path = plain_copies::write(media.plain_file_name(), &bytes)?;
    open_with_system(path.as_os_str())
}

/// Montre le média dans le Finder / l'Explorateur. `invoke('reveal_media', { name })`.
#[tauri::command]
pub async fn reveal_media(state: State<'_, LiveState>, name: String) -> Result<(), UiError> {
    let use_case = state.current().media.read.clone();
    let path = run(move || use_case.local_path(&name)).await?;
    reveal_in_file_manager(&path)
}

#[cfg(test)]
mod tests {
    /// Sans `connect-src ipc: http://ipc.localhost`, la CSP de l'app **construite** bloque l'IPC
    /// de Tauri : il se rabat sur un transport JSON et `store_media` ne reçoit plus d'octets bruts
    /// (« le média doit être envoyé en binaire brut »). `pnpm dev` n'applique pas la CSP : seule
    /// l'app installée échouait.
    #[test]
    fn the_csp_lets_raw_media_reach_the_core() {
        // [rust] `include_str!` lit le fichier à la compilation et l'inclut comme `&str`
        // (≈ `import config from './tauri.conf.json?raw'` de Vite).
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).expect("tauri.conf.json");
        let csp = config["app"]["security"]["csp"].as_str().expect("CSP");
        let connect = csp
            .split(';')
            .map(str::trim)
            .find(|directive| directive.starts_with("connect-src"))
            .expect("directive connect-src");
        for source in ["ipc:", "http://ipc.localhost"] {
            assert!(
                connect.split_whitespace().any(|allowed| allowed == source),
                "{source} manque dans « {connect} »"
            );
        }
    }
}
