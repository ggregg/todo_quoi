//! Préférences de l'appareil (spec §5.7, fenêtre « Réglages ») : nom de l'appareil, durée de la
//! corbeille, compression des images. Enregistrées dans `settings.json` (jamais dans une base).
//! Changer le nom ou la durée reconstruit l'assemblage (comme ajouter une base) ; la compression
//! ne concerne que l'interface.

use super::settings::reload;
use crate::error::UiError;
use crate::settings::{ImageSettings, LocalSettings};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tq_adapter_system::machine_device_id;
use tq_domain::DeviceId;
use tq_domain::rules::trash::TrashRetention;

/// Préférences, telles que l'interface les affiche et les renvoie.
/// JSON : `{ deviceName, deviceId, trashRetentionDays, images: { quality, maxSide, handling } }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesDto {
    /// Nom de l'appareil, tel que saisi.
    pub device_name: String,
    /// Identifiant qui en découle (`Mac de Greg` → `mac-de-greg`) ; ignoré à l'enregistrement.
    #[serde(default)]
    pub device_id: String,
    /// Durée de la corbeille, en jours (1 à 365).
    pub trash_retention_days: u32,
    /// Compression des images.
    pub images: ImageSettings,
}

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, UiError> {
    app.path()
        .app_config_dir()
        .map_err(|error| UiError::internal(error.to_string()))
}

/// Nom de l'appareil enregistré, ou celui de la machine.
fn device_name(settings: &LocalSettings) -> String {
    settings
        .device_name
        .clone()
        .unwrap_or_else(|| machine_device_id().as_str().to_string())
}

/// Préférences de l'appareil. `invoke('get_preferences')`.
#[tauri::command]
pub fn get_preferences(app: AppHandle) -> Result<PreferencesDto, UiError> {
    let settings = LocalSettings::load(&config_dir(&app)?);
    let name = device_name(&settings);
    Ok(PreferencesDto {
        device_id: DeviceId::from_machine_name(&name).as_str().to_string(),
        device_name: name,
        trash_retention_days: settings.trash_retention().days(),
        images: settings.images.unwrap_or_default(),
    })
}

/// Enregistre les préférences ; reconstruit l'assemblage si le nom ou la durée change.
/// `invoke('set_preferences', { preferences })`.
#[tauri::command]
pub async fn set_preferences(app: AppHandle, preferences: PreferencesDto) -> Result<(), UiError> {
    let name = preferences.device_name.trim().to_string();
    if DeviceId::from_machine_name(&name).as_str().is_empty() {
        return Err(UiError::invalid_input(
            "le nom de l'appareil doit contenir au moins une lettre ou un chiffre",
        ));
    }
    let dir = config_dir(&app)?;
    let before = LocalSettings::load(&dir);
    let after = LocalSettings {
        device_name: Some(name),
        trash_retention_days: Some(
            TrashRetention::from_days(preferences.trash_retention_days).days(),
        ),
        images: Some(checked_images(preferences.images)),
        ..before.clone()
    };
    after
        .save(&dir)
        .map_err(|error| UiError::internal(format!("réglages non enregistrés : {error}")))?;
    let rebuild = before.device_id(machine_device_id) != after.device_id(machine_device_id)
        || before.trash_retention() != after.trash_retention();
    if rebuild {
        reload(app).await?;
    }
    Ok(())
}

/// Qualité ramenée entre 50 et 100 %, côté entre 640 et 8192 pixels.
fn checked_images(images: ImageSettings) -> ImageSettings {
    ImageSettings {
        quality: images.quality.clamp(50, 100),
        max_side: images.max_side.clamp(640, 8192),
        handling: images.handling,
    }
}

/// Au premier lancement : le nom de la machine devient le nom de l'appareil (spec §5.7), pour
/// qu'un changement de nom de la machine ne change pas l'appareil qui signe les tâches.
pub fn remember_device_name(app: &AppHandle) {
    let Ok(dir) = config_dir(app) else {
        return;
    };
    let settings = LocalSettings::load(&dir);
    if settings.device_name.is_some() {
        return;
    }
    let named = LocalSettings {
        device_name: Some(machine_device_id().as_str().to_string()),
        ..settings
    };
    let _ = named.save(&dir); // sans réglages enregistrables, le nom de la machine sert à chaque lancement
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ImageHandling;

    #[test]
    fn image_settings_are_kept_within_bounds() {
        let checked = checked_images(ImageSettings {
            quality: 10,
            max_side: 100_000,
            handling: ImageHandling::Original,
        });
        assert_eq!((checked.quality, checked.max_side), (50, 8192));
        assert_eq!(checked.handling, ImageHandling::Original);
    }
}
