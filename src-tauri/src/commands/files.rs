//! Fichiers choisis par l'utilisateur dans le Finder / l'Explorateur : les joindre à une note
//! (copier ou déplacer dans la base), ou les **lier** (chemin + appareil), puis ouvrir ou
//! révéler un fichier lié — seulement là où le cœur le permet (`CheckLinkedFile`).

use super::run;
use crate::composition::LiveState;
use crate::dto::parse_task_id;
use crate::error::UiError;
use crate::system_open::{open_with_system, reveal_in_file_manager};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use tq_application::use_cases::attach_file::AttachMode;
use tq_domain::{DeviceId, LinkAvailability};

/// Copier ou déplacer. JSON : `"copy"` ou `"move"`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttachModeDto {
    /// Copier dans la base.
    Copy,
    /// Déplacer dans la base (l'original est effacé).
    Move,
}

/// Fichier joint, à insérer dans la note (`::file{src name size}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedFileDto {
    /// Lien de note (`../media/<nom>`).
    pub src: String,
    /// Nom d'origine.
    pub name: String,
    /// Taille en octets.
    pub size: usize,
}

/// Fichier lié, à insérer dans la note (`::filelink{path name device}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedFileDto {
    /// Chemin sur l'appareil d'origine.
    pub path: String,
    /// Nom du fichier.
    pub name: String,
    /// Appareil d'origine.
    pub device: String,
}

/// Ce que permet un lien ici : `available`, `revealOnly`, `otherDevice`, `missing`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkStatusDto {
    /// État.
    pub state: &'static str,
}

/// Joint un fichier choisi. `invoke('attach_file', { taskId, path, mode: 'copy' })`.
#[tauri::command]
pub async fn attach_file(
    state: State<'_, LiveState>,
    task_id: String,
    path: String,
    mode: AttachModeDto,
) -> Result<AttachedFileDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let mode = match mode {
        AttachModeDto::Copy => AttachMode::Copy,
        AttachModeDto::Move => AttachMode::Move,
    };
    let use_case = state.current().media.attach.clone();
    run(move || {
        let attached = use_case.execute(&id, &PathBuf::from(path), mode)?;
        Ok(AttachedFileDto {
            src: attached.media.note_link(),
            name: attached.name,
            size: attached.size,
        })
    })
    .await
}

/// Lie un fichier choisi. `invoke('link_file', { path })`.
#[tauri::command]
pub async fn link_file(
    state: State<'_, LiveState>,
    path: String,
) -> Result<LinkedFileDto, UiError> {
    let use_case = state.current().media.link.clone();
    run(move || {
        let link = use_case.execute(&PathBuf::from(path))?;
        Ok(LinkedFileDto {
            path: link.path,
            name: link.name,
            device: link.device.to_string(),
        })
    })
    .await
}

/// Ce que permet un fichier lié ici. `invoke('linked_file_status', { path, device })`.
#[tauri::command]
pub async fn linked_file_status(
    state: State<'_, LiveState>,
    path: String,
    device: String,
) -> Result<LinkStatusDto, UiError> {
    let availability = check(&state, path, device).await?;
    Ok(LinkStatusDto {
        state: match availability {
            LinkAvailability::Available => "available",
            LinkAvailability::RevealOnly => "revealOnly",
            LinkAvailability::OnOtherDevice(_) => "otherDevice",
            LinkAvailability::Missing => "missing",
        },
    })
}

/// Ouvre un fichier lié, s'il est ouvrable ici. `invoke('open_linked_file', { path, device })`.
#[tauri::command]
pub async fn open_linked_file(
    state: State<'_, LiveState>,
    path: String,
    device: String,
) -> Result<(), UiError> {
    match check(&state, path.clone(), device).await? {
        LinkAvailability::Available => open_with_system(PathBuf::from(path).as_os_str()),
        other => Err(refused(&other)),
    }
}

/// Montre un fichier lié dans le Finder / l'Explorateur. `invoke('reveal_linked_file', …)`.
#[tauri::command]
pub async fn reveal_linked_file(
    state: State<'_, LiveState>,
    path: String,
    device: String,
) -> Result<(), UiError> {
    match check(&state, path.clone(), device).await? {
        LinkAvailability::Available | LinkAvailability::RevealOnly => {
            reveal_in_file_manager(&PathBuf::from(path))
        }
        other => Err(refused(&other)),
    }
}

async fn check(
    state: &State<'_, LiveState>,
    path: String,
    device: String,
) -> Result<LinkAvailability, UiError> {
    let device =
        DeviceId::parse(&device).map_err(|error| UiError::invalid_input(error.to_string()))?;
    let use_case = state.current().media.check_link.clone();
    run(move || Ok(use_case.execute(&PathBuf::from(path), &device))).await
}

fn refused(availability: &LinkAvailability) -> UiError {
    UiError::invalid_input(match availability {
        LinkAvailability::OnOtherDevice(device) => {
            format!("ce lien a été créé sur « {device} » : il ne s'ouvre que là-bas")
        }
        LinkAvailability::Missing => "le fichier lié est introuvable (déplacé, renommé ou effacé ?)".to_string(),
        LinkAvailability::RevealOnly => {
            "par sécurité, un dossier ou un programme lié ne s'ouvre pas depuis une note : utilisez « Révéler »".to_string()
        }
        LinkAvailability::Available => String::new(),
    })
}
