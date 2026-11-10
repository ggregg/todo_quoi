//! Réglages **propres à cet appareil** (non synchronisés) : `settings.json` dans le dossier de
//! configuration de l'application (`app_config_dir()` de Tauri, selon l'OS).
//!
//! Contenu : les **bases** ouvertes (ADR-0009) — nom, couleur, dossier — et les préférences de
//! l'appareil (spec §5.7) : nom de l'appareil, durée de la corbeille, compression des images,
//! délai du verrouillage automatique des coffres (spec §9). Les dossiers des
//! drives diffèrent d'un appareil à l'autre (`/Users/greg/Library/…` sur le Mac,
//! `C:\Users\greg\OneDrive` sur le PC) : ils ne peuvent pas être enregistrés dans un drive.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tq_domain::rules::trash::TrashRetention;
use tq_domain::{BaseId, DeviceId};

const FILE_NAME: &str = "settings.json";
/// Couleur proposée pour une base sans couleur choisie.
pub const DEFAULT_COLOR: &str = "#4f7cff";

/// Une base ouverte sur cet appareil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseSetting {
    /// Identifiant (`perso`, `pro`…), stable : sert à mémoriser l'affichage.
    pub id: String,
    /// Nom affiché dans le bandeau.
    pub name: String,
    /// Couleur `#rrggbb`.
    pub color: String,
    /// Dossier de la base (`…/TodoQuoi`).
    pub path: PathBuf,
}

impl BaseSetting {
    /// Identifiant du domaine ; un identifiant illisible (fichier modifié à la main) est refait
    /// à partir du nom.
    pub fn base_id(&self) -> BaseId {
        BaseId::parse(&self.id).unwrap_or_else(|_| BaseId::from_name(&self.name))
    }
}

/// Réglages locaux.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSettings {
    /// Ancien réglage (une seule base, ADR-0008) : repris comme base « Mes tâches ».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
    /// Bases ouvertes, dans l'ordre du bandeau.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bases: Vec<BaseSetting>,
    /// Verrouillage automatique des coffres après ce nombre de minutes sans activité (`0` :
    /// jamais) ; absent : le délai par défaut (15 min).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_auto_lock_minutes: Option<u32>,
    /// Nom de l'appareil (spec §5.7) : celui de la machine au premier lancement, modifiable.
    /// Il signe les modifications et les fichiers liés (`updated_by`, `::filelink`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    /// Durée de conservation dans la corbeille, en jours (30 par défaut).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trash_retention_days: Option<u32>,
    /// Compression des images collées ou glissées dans les notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<ImageSettings>,
}

/// Compression des images (spec §5.7) : appliquée par l'interface avant l'envoi au cœur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSettings {
    /// Qualité JPEG, en pourcentage (50 à 100).
    pub quality: u8,
    /// Plus grand côté, en pixels.
    pub max_side: u32,
    /// `compress` (réduire) ou `original` (garder l'image telle quelle).
    pub handling: ImageHandling,
}

/// Que faire d'une image ajoutée à une note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageHandling {
    /// Réduire et recompresser (par défaut).
    Compress,
    /// Garder l'original.
    Original,
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            quality: 85,
            max_side: 2560,
            handling: ImageHandling::Compress,
        }
    }
}

impl LocalSettings {
    /// Lit les réglages ; fichier absent ou illisible = réglages par défaut.
    pub fn load(config_dir: &Path) -> Self {
        fs::read_to_string(config_dir.join(FILE_NAME))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Enregistre les réglages.
    ///
    /// # Erreurs
    /// Dossier de configuration impossible à créer ou à écrire.
    pub fn save(&self, config_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(config_dir)?;
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(config_dir.join(FILE_NAME), text + "\n")
    }

    /// Identifiant de l'appareil : celui du nom réglé, ou de `machine` (nom de la machine).
    pub fn device_id(&self, machine: impl FnOnce() -> DeviceId) -> DeviceId {
        match self.device_name.as_deref() {
            Some(name) if !name.trim().is_empty() => DeviceId::from_machine_name(name),
            _ => machine(),
        }
    }

    /// Durée de la corbeille réglée (30 jours par défaut).
    pub fn trash_retention(&self) -> TrashRetention {
        self.trash_retention_days
            .map_or(TrashRetention::DEFAULT, TrashRetention::from_days)
    }

    /// Bases à ouvrir. Sans base enregistrée : l'ancien dossier unique, sinon `device_folder`
    /// (dossier local à l'appareil), sous le nom « Mes tâches » et l'identifiant principal.
    pub fn bases_or(&self, device_folder: PathBuf) -> Vec<BaseSetting> {
        if !self.bases.is_empty() {
            return self.bases.clone();
        }
        vec![BaseSetting {
            id: BaseId::main().to_string(),
            name: "Mes tâches".to_string(),
            color: DEFAULT_COLOR.to_string(),
            path: self.data_dir.clone().unwrap_or(device_folder),
        }]
    }
}

/// Vrai si `color` est une couleur `#rrggbb`.
pub fn is_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_saved_and_read_back() {
        let dir = std::env::temp_dir().join(format!("tq-settings-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(LocalSettings::load(&dir), LocalSettings::default());
        let settings = LocalSettings {
            data_dir: None,
            bases: vec![BaseSetting {
                id: "pro".to_string(),
                name: "Pro".to_string(),
                color: "#ff8800".to_string(),
                path: PathBuf::from("/drive/TodoQuoi"),
            }],
            vault_auto_lock_minutes: Some(5),
            device_name: Some("Mac de Greg".to_string()),
            trash_retention_days: Some(7),
            images: Some(ImageSettings::default()),
        };
        settings.save(&dir).expect("écriture");
        assert_eq!(LocalSettings::load(&dir), settings);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_former_single_folder_becomes_the_main_base() {
        let old = LocalSettings {
            data_dir: Some(PathBuf::from("/drive/TodoQuoi")),
            ..LocalSettings::default()
        };
        let bases = old.bases_or(PathBuf::from("/local"));
        assert_eq!(bases.len(), 1);
        assert_eq!(
            (bases[0].base_id(), bases[0].path.clone()),
            (BaseId::main(), PathBuf::from("/drive/TodoQuoi"))
        );
        let fresh = LocalSettings::default().bases_or(PathBuf::from("/local"));
        assert_eq!(fresh[0].path, PathBuf::from("/local"));
    }

    #[test]
    fn device_and_retention_have_defaults() {
        let machine = || DeviceId::from_machine_name("machine");
        let fresh = LocalSettings::default();
        assert_eq!(fresh.device_id(machine).as_str(), "machine");
        assert_eq!(fresh.trash_retention(), TrashRetention::DEFAULT);
        let set = LocalSettings {
            device_name: Some("Mac de Greg".to_string()),
            trash_retention_days: Some(0),
            ..LocalSettings::default()
        };
        assert_eq!(set.device_id(machine).as_str(), "mac-de-greg");
        assert_eq!(set.trash_retention().days(), 1, "ramenée dans les bornes");
    }

    #[test]
    fn colors_are_checked() {
        assert!(is_color("#4f7cff"));
        assert!(!is_color("bleu"));
        assert!(!is_color("#12345"));
    }
}
