//! Commandes des médias : stocker (binaire brut), ouvrir avec l'application par défaut,
//! révéler dans le Finder / l'Explorateur.

use super::run;
use crate::composition::AppState;
use crate::error::UiError;
use std::path::Path;
use std::process::Command;
use tauri::State;
use tauri::ipc::{InvokeBody, Request};

/// En-tête qui transporte l'extension du fichier envoyé (`jpg`, `webm`, `pdf`…).
pub const EXTENSION_HEADER: &str = "x-media-extension";

/// Stocke un média et renvoie le lien à écrire dans la note (`../media/<nom>`).
///
/// Côté TypeScript : `invoke('store_media', octets, { headers: { 'x-media-extension': 'jpg' } })`.
/// Les octets voyagent **bruts** : un tableau JSON de 50 Mo serait beaucoup trop lent.
#[tauri::command]
pub async fn store_media(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<String, UiError> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(UiError::invalid_input(
            "le média doit être envoyé en binaire brut",
        ));
    };
    let extension = request
        .headers()
        .get(EXTENSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("bin")
        .to_string();
    let bytes = bytes.clone();
    let use_case = state.store_media.clone();
    run(move || Ok(use_case.execute(&bytes, &extension)?.note_link())).await
}

/// Ouvre le média avec l'application par défaut de l'OS. `invoke('open_media', { name })`.
#[tauri::command]
pub async fn open_media(state: State<'_, AppState>, name: String) -> Result<(), UiError> {
    let use_case = state.read_media.clone();
    let path = run(move || use_case.local_path(&name)).await?;
    launch(open_command(&path))
}

/// Montre le média dans le Finder / l'Explorateur. `invoke('reveal_media', { name })`.
#[tauri::command]
pub async fn reveal_media(state: State<'_, AppState>, name: String) -> Result<(), UiError> {
    let use_case = state.read_media.clone();
    let path = run(move || use_case.local_path(&name)).await?;
    launch(reveal_command(&path))
}

fn launch(mut command: Command) -> Result<(), UiError> {
    let mut child = command.spawn().map_err(|error| {
        UiError::internal(format!(
            "impossible de lancer l'application du système : {error}"
        ))
    })?;
    // On attend la fin de `open` / `explorer` dans un fil séparé : un processus jamais attendu
    // resterait « zombie » jusqu'à la fermeture de TodoQuoi.
    std::thread::spawn(move || child.wait());
    Ok(())
}

// [rust] `#[cfg(target_os = "…")]` : seule la version de l'OS en cours est compilée.
//        Aucune dépendance : on appelle les commandes standard de chaque système.

#[cfg(target_os = "macos")]
fn open_command(path: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg(path);
    command
}

#[cfg(target_os = "macos")]
fn reveal_command(path: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg("-R").arg(path);
    command
}

#[cfg(target_os = "windows")]
fn open_command(path: &Path) -> Command {
    let mut command = Command::new("explorer");
    command.arg(path);
    command
}

#[cfg(target_os = "windows")]
fn reveal_command(path: &Path) -> Command {
    let mut command = Command::new("explorer");
    // `/select,` et le chemin forment UN SEUL argument pour l'Explorateur.
    command.arg(format!("/select,{}", path.display()));
    command
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn open_command(path: &Path) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(path);
    command
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn reveal_command(path: &Path) -> Command {
    // Pas de commande standard « révéler » sous Linux : on ouvre le dossier.
    open_command(path.parent().unwrap_or(path))
}
