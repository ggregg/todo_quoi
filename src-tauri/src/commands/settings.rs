//! Bases ouvertes sur cet appareil (spec §5.1bis et §7, ADR-0008 et ADR-0009) : lesquelles, où,
//! dans quel état ; en ajouter, les renommer, en détacher. Ajouter ou détacher une base
//! **reconstruit** tout l'assemblage (`composition.rs`) sur la nouvelle liste et le remplace à
//! chaud (`LiveState`), sans état à moitié migré et sans redémarrer le processus.

use crate::composition::{self, DataLocationKind, LiveState, OpenedBase, data_location};
use crate::error::UiError;
use crate::io_timeout::{DRIVE_TIMEOUT, with_timeout};
use crate::settings::{BaseSetting, LocalSettings, is_color};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tq_adapter_fs_markdown::{FsMarkdownTaskRepository, workspace_folder};
use tq_adapter_system::{
    DriveFolder, DriveProvider, StorageKind, classify_folder, detect_drive_folders,
};
use tq_domain::BaseId;

/// Un dossier de drive détecté.
#[derive(Debug, Clone, Serialize)]
pub struct DriveFolderDto {
    /// `'icloud' | 'googleDrive' | 'oneDrive' | 'dropbox'`.
    pub provider: &'static str,
    /// Chemin du drive.
    pub path: String,
}

/// Une base et son état.
#[derive(Debug, Clone, Serialize)]
pub struct BaseDto {
    /// Identifiant (`perso`, `pro`…).
    pub id: String,
    /// Nom affiché.
    pub name: String,
    /// Couleur `#rrggbb`.
    pub color: String,
    /// Dossier de la base.
    pub path: String,
    /// Stockage détecté : `'local' | 'external' | 'network' | 'icloud' | 'googleDrive' |
    /// 'oneDrive' | 'dropbox'`.
    pub storage: &'static str,
    /// Raison si la base n'a pas pu être ouverte au démarrage (`null` : ouverte).
    pub problem: Option<String>,
}

/// Bases et drives disponibles.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataLocationDto {
    /// `'environment'` (TODOQUOI_DATA_DIR, une base imposée) ou `'settings'`.
    pub kind: &'static str,
    /// Bases, dans l'ordre du bandeau.
    pub bases: Vec<BaseDto>,
    /// Vrai tant que la seule base est le dossier local de l'appareil (non synchronisé).
    pub device_only: bool,
    /// Drives détectés sur l'appareil.
    pub drives: Vec<DriveFolderDto>,
}

/// Bases ouvertes et drives disponibles. `invoke('get_data_location')`.
#[tauri::command]
pub async fn get_data_location(
    app: AppHandle,
    state: State<'_, LiveState>,
) -> Result<DataLocationDto, UiError> {
    let location = data_location(&app).map_err(|error| UiError::internal(error.to_string()))?;
    // La détection ne fait que des `stat()`, mais un drive bloqué ne doit pas figer l'écran.
    let drives = with_timeout(DRIVE_TIMEOUT, detect_drive_folders).unwrap_or_default();
    // Disque système sous Windows (`C:`) ; ailleurs, la variable n'existe pas.
    let system_drive = std::env::var("SystemDrive").ok();
    let bases = base_dtos(
        &state.current().opened_bases,
        &location.bases,
        &drives,
        system_drive.as_deref(),
    );
    Ok(DataLocationDto {
        kind: match location.kind {
            DataLocationKind::EnvironmentVariable => "environment",
            DataLocationKind::Settings => "settings",
        },
        device_only: location.bases.len() == 1 && location.bases[0].path == location.device_folder,
        bases,
        drives: drives
            .into_iter()
            .map(|drive| DriveFolderDto {
                provider: provider_name(drive.provider),
                path: drive.path.display().to_string(),
            })
            .collect(),
    })
}

/// Ajoute une base, puis recharge l'assemblage. `invoke('add_base', { name, color, path })`.
///
/// N'importe quel dossier convient : disque local, disque externe, dossier d'un drive cloud —
/// le stockage est détecté, l'outil s'adapte (scan, fichiers à la demande, délais). Un dossier
/// contenant déjà un `manifest.json` est repris tel quel ; sinon la base y est créée
/// **directement** (sans sous-dossier). Voir `workspace_folder`.
#[tauri::command]
pub async fn add_base(
    app: AppHandle,
    name: String,
    color: String,
    path: String,
) -> Result<(), UiError> {
    let name = checked_name(&name)?;
    let color = checked_color(&color)?;
    let chosen = PathBuf::from(path.trim());
    if !chosen.is_absolute() {
        return Err(UiError::invalid_input(
            "indiquez un chemin complet (ex. /Users/… ou C:\\Users\\…)",
        ));
    }
    let workspace = tauri::async_runtime::spawn_blocking(move || prepare(chosen))
        .await
        .map_err(|error| UiError::internal(error.to_string()))??;
    let location = data_location(&app).map_err(|error| UiError::internal(error.to_string()))?;
    // Les bases existantes sont toujours gardées, y compris le dossier local « Mes tâches » :
    // ses tâches restent visibles (on peut le retirer ensuite si on le souhaite).
    let mut bases = location.bases;
    if bases.iter().any(|base| base.path == workspace) {
        return Err(UiError::invalid_input("ce dossier est déjà ouvert"));
    }
    let id = unique_id(&bases, &name);
    bases.push(BaseSetting {
        id,
        name,
        color,
        path: workspace,
    });
    save_bases(&app, bases)?;
    reload(app).await
}

/// Renomme une base ou change sa couleur (sans redémarrer). `invoke('update_base', { id, name, color })`.
#[tauri::command]
pub async fn update_base(
    app: AppHandle,
    id: String,
    name: String,
    color: String,
) -> Result<(), UiError> {
    let name = checked_name(&name)?;
    let color = checked_color(&color)?;
    let mut bases = data_location(&app)
        .map_err(|error| UiError::internal(error.to_string()))?
        .bases;
    let base = bases
        .iter_mut()
        .find(|base| base.base_id().as_str() == id)
        .ok_or_else(|| UiError::invalid_input("base inconnue"))?;
    base.name = name;
    base.color = color;
    save_bases(&app, bases)
}

/// Détache une base de l'application (le dossier n'est ni effacé ni modifié), puis recharge l'assemblage.
/// `invoke('remove_base', { id })`.
#[tauri::command]
pub async fn remove_base(app: AppHandle, id: String) -> Result<(), UiError> {
    let mut bases = data_location(&app)
        .map_err(|error| UiError::internal(error.to_string()))?
        .bases;
    bases.retain(|base| base.base_id().as_str() != id);
    save_bases(&app, bases)?; // aucune base : retour au dossier local de l'appareil
    reload(app).await
}

/// Reconstruit tout l'assemblage sur la nouvelle liste de bases et le met en service **à chaud**
/// (sans redémarrer le processus : en développement, un redémarrage arrêterait aussi le serveur
/// de l'interface). L'interface se recharge ensuite (`BASES_CHANGED`).
pub(crate) async fn reload(app: AppHandle) -> Result<(), UiError> {
    let builder = app.clone();
    let state = tauri::async_runtime::spawn_blocking(move || {
        composition::build(&builder).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| UiError::internal(error.to_string()))?
    .map_err(UiError::internal)?;
    app.state::<LiveState>().replace(state);
    let _ = app.emit(BASES_CHANGED, ());
    Ok(())
}

/// Événement envoyé à l'interface quand la liste des bases a changé.
pub const BASES_CHANGED: &str = "bases-changed";

/// Vérifie le dossier choisi et crée l'espace si besoin (avec délai : le drive peut bloquer).
fn prepare(chosen: PathBuf) -> Result<PathBuf, UiError> {
    if !chosen.is_dir() {
        return Err(UiError::invalid_input(format!(
            "dossier introuvable : {}",
            chosen.display()
        )));
    }
    let workspace = workspace_folder(&chosen);
    let target = workspace.clone();
    let opened = with_timeout(DRIVE_TIMEOUT, move || {
        FsMarkdownTaskRepository::open(target)
    })
    .map_err(|_| {
        UiError::invalid_input(
            "le dossier ne répond pas (disque débranché, drive hors ligne ?) : réessayez",
        )
    })?;
    opened.map_err(|error| UiError::invalid_input(error.to_string()))?;
    Ok(workspace)
}

fn save_bases(app: &AppHandle, bases: Vec<BaseSetting>) -> Result<(), UiError> {
    if std::env::var_os(crate::composition::DATA_DIR_VARIABLE).is_some() {
        return Err(UiError::invalid_input(
            "TODOQUOI_DATA_DIR est défini : les bases sont imposées par cette variable",
        ));
    }
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| UiError::internal(error.to_string()))?;
    // Les autres réglages (verrouillage automatique…) sont relus, pour être conservés.
    LocalSettings {
        data_dir: None,
        bases,
        ..LocalSettings::load(&config_dir)
    }
    .save(&config_dir)
    .map_err(|error| UiError::internal(format!("réglages non enregistrés : {error}")))
}

/// Identifiant tiré du nom, rendu unique (`pro`, `pro-2`…).
fn unique_id(bases: &[BaseSetting], name: &str) -> String {
    let wanted = BaseId::from_name(name).to_string();
    let taken = |candidate: &str| {
        bases
            .iter()
            .any(|base| base.base_id().as_str() == candidate)
    };
    (1..)
        .map(|n| {
            if n == 1 {
                wanted.clone()
            } else {
                format!("{wanted}-{n}")
            }
        })
        .find(|candidate| !taken(candidate))
        .unwrap_or(wanted)
}

fn checked_name(name: &str) -> Result<String, UiError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err(UiError::invalid_input("nom de base : 1 à 40 caractères"));
    }
    Ok(name.to_string())
}

fn checked_color(color: &str) -> Result<String, UiError> {
    if is_color(color) {
        Ok(color.to_lowercase())
    } else {
        Err(UiError::invalid_input("couleur attendue : #rrggbb"))
    }
}

/// Bases ouvertes au démarrage (un ajout ou un retrait redémarre l'app), avec le nom et la
/// couleur des réglages **actuels** : un renommage ou un changement de couleur se voit tout de
/// suite, sans redémarrer.
fn base_dtos(
    opened: &[OpenedBase],
    current: &[BaseSetting],
    drives: &[DriveFolder],
    system_drive: Option<&str>,
) -> Vec<BaseDto> {
    opened
        .iter()
        .map(|opened| {
            let id = opened.setting.base_id();
            let latest = current
                .iter()
                .find(|setting| setting.base_id() == id)
                .unwrap_or(&opened.setting);
            BaseDto {
                id: id.to_string(),
                name: latest.name.clone(),
                color: latest.color.clone(),
                path: opened.setting.path.display().to_string(),
                storage: storage_name(classify_folder(&opened.setting.path, drives, system_drive)),
                problem: opened.problem.clone(),
            }
        })
        .collect()
}

fn storage_name(kind: StorageKind) -> &'static str {
    match kind {
        StorageKind::Cloud(provider) => provider_name(provider),
        StorageKind::ExternalDisk => "external",
        StorageKind::NetworkShare => "network",
        StorageKind::LocalDisk => "local",
    }
}

fn provider_name(provider: DriveProvider) -> &'static str {
    match provider {
        DriveProvider::ICloud => "icloud",
        DriveProvider::GoogleDrive => "googleDrive",
        DriveProvider::OneDrive => "oneDrive",
        DriveProvider::Dropbox => "dropbox",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(name: &str, color: &str) -> BaseSetting {
        BaseSetting {
            id: "pro".to_string(),
            name: name.to_string(),
            color: color.to_string(),
            path: PathBuf::from("/Users/greg/Documents/TodoQuoi"),
        }
    }

    #[test]
    fn a_renamed_or_recoloured_base_shows_its_current_settings_without_restart() {
        let opened = [OpenedBase {
            setting: setting("Pro", "#4f7cff"),
            problem: None,
        }];
        let current = [setting("Travail", "#e8833a")];
        let bases = base_dtos(&opened, &current, &[], None);
        assert_eq!(
            (bases[0].name.as_str(), bases[0].color.as_str()),
            ("Travail", "#e8833a")
        );
        assert_eq!(bases[0].storage, "local");
    }

    #[test]
    fn a_base_missing_from_settings_keeps_its_startup_values() {
        let opened = [OpenedBase {
            setting: setting("Pro", "#4f7cff"),
            problem: Some("hors ligne".to_string()),
        }];
        let bases = base_dtos(&opened, &[], &[], None);
        assert_eq!(
            (bases[0].color.as_str(), bases[0].problem.as_deref()),
            ("#4f7cff", Some("hors ligne"))
        );
    }
}
