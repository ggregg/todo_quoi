//! `manifest.json` : décrit l'espace de données, à la racine du dossier.
//!
//! - `schema_version` : permet de refuser proprement des données écrites par une version **plus
//!   récente** de l'application (spécification §8 : inviter à mettre à jour) ;
//! - `root_view` (facultatif) : vue de la liste racine (format : [`crate::view_format`]),
//!   implémente le port [`RootViewStore`] via [`FsRootViewStore`] ;
//! - `vault` (facultatif) : le coffre de la base (ADR-0010) — identifiant, paramètres Argon2id,
//!   sel, clé de contenu **enveloppée** ; rien de secret. Port [`VaultStore`] via [`FsVaultStore`].
//!
//! Les champs inconnus sont conservés à la réécriture.

use crate::format::{PLAIN_SCHEMA_VERSION, SCHEMA_VERSION};
use crate::storage::write_atomically;
use crate::view_format::ViewFile;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tq_application::ports::{KdfParams, RepositoryError, RootViewStore, VaultRecord, VaultStore};
use tq_application::vault::VaultError;
use tq_domain::view::ListView;
use tq_domain::{BaseId, KeyId};

const FILE_NAME: &str = "manifest.json";

/// Sous-dossier que créaient les versions d'avant le 2026-11-09 dans le dossier choisi.
pub const LEGACY_WORKSPACE_FOLDER: &str = "TodoQuoi";

/// Dossier de la base pour un dossier choisi par l'utilisateur (spec §5.1bis) : **le dossier
/// lui-même** — les fichiers (`manifest.json`, `tasks/`, `media/`) y sont créés directement.
///
/// Seule exception, pour rattacher une base créée par une version précédente : un dossier sans
/// `manifest.json` mais avec un sous-dossier `TodoQuoi/` qui en a un → ce sous-dossier.
pub fn workspace_folder(chosen: &Path) -> PathBuf {
    let legacy = chosen.join(LEGACY_WORKSPACE_FOLDER);
    let is_legacy = !chosen.join(FILE_NAME).is_file() && legacy.join(FILE_NAME).is_file();
    if is_legacy {
        legacy
    } else {
        chosen.to_path_buf()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    root_view: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vault: Option<VaultFile>,
    #[serde(flatten)]
    unknown: Map<String, Value>,
}

/// Le coffre tel qu'il est écrit dans `manifest.json`.
#[derive(Debug, Serialize, Deserialize)]
struct VaultFile {
    key_id: String,
    kdf: KdfFile,
    salt: String,
    wrapped_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KdfFile {
    /// Toujours `argon2id` : écrit pour qu'un autre algorithme ne soit jamais pris pour celui-ci.
    algorithm: String,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
}

const KDF_ALGORITHM: &str = "argon2id";

impl VaultFile {
    fn from_record(record: &VaultRecord) -> Self {
        Self {
            key_id: record.key_id.to_string(),
            kdf: KdfFile {
                algorithm: KDF_ALGORITHM.to_string(),
                memory_kib: record.kdf.memory_kib,
                iterations: record.kdf.iterations,
                parallelism: record.kdf.parallelism,
            },
            salt: record.salt.clone(),
            wrapped_key: record.wrapped_key.clone(),
        }
    }

    fn to_record(&self) -> Result<VaultRecord, RepositoryError> {
        let invalid = |reason: String| {
            RepositoryError::Unavailable(format!("coffre illisible dans {FILE_NAME} : {reason}"))
        };
        if self.kdf.algorithm != KDF_ALGORITHM {
            return Err(invalid(format!(
                "algorithme « {} » inconnu",
                self.kdf.algorithm
            )));
        }
        Ok(VaultRecord {
            key_id: KeyId::parse(&self.key_id).map_err(|error| invalid(error.to_string()))?,
            kdf: KdfParams {
                memory_kib: self.kdf.memory_kib,
                iterations: self.kdf.iterations,
                parallelism: self.kdf.parallelism,
            },
            salt: self.salt.clone(),
            wrapped_key: self.wrapped_key.clone(),
        })
    }
}

/// Crée le manifeste s'il n'existe pas, sinon vérifie que sa version est gérée.
pub(crate) fn ensure(root: &Path) -> Result<(), RepositoryError> {
    let path = root.join(FILE_NAME);
    if !path.exists() {
        let manifest = Manifest {
            schema_version: PLAIN_SCHEMA_VERSION,
            root_view: None,
            vault: None,
            unknown: Map::new(),
        };
        return write(&path, &manifest);
    }
    read(&path).map(|_| ())
}

fn read(path: &Path) -> Result<Manifest, RepositoryError> {
    let text = fs::read_to_string(path).map_err(|error| unavailable(path, &error))?;
    let manifest: Manifest = serde_json::from_str(&text).map_err(|error| {
        RepositoryError::Unavailable(format!("{} illisible : {error}", path.display()))
    })?;
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(RepositoryError::NewerSchema {
            found: manifest.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    Ok(manifest)
}

fn write(path: &Path, manifest: &Manifest) -> Result<(), RepositoryError> {
    let text = serde_json::to_string_pretty(manifest)
        .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
    write_atomically(path, &(text + "\n")).map_err(|error| unavailable(path, &error))
}

pub(crate) fn unavailable(path: &Path, error: &std::io::Error) -> RepositoryError {
    RepositoryError::Unavailable(format!("{} : {error}", path.display()))
}

/// Vue racine enregistrée dans `manifest.json`.
pub struct FsRootViewStore {
    path: PathBuf,
}

impl FsRootViewStore {
    /// Ouvre (ou initialise) le manifeste de l'espace situé dans `root`.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si le manifeste est illisible ou d'une version plus récente.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RepositoryError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| unavailable(&root, &error))?;
        ensure(&root)?;
        Ok(Self {
            path: root.join(FILE_NAME),
        })
    }
}

impl RootViewStore for FsRootViewStore {
    /// Un dossier = une base : `base` est ignoré. Une vue illisible (modifiée à la main, version
    /// plus récente) est ignorée : vue d'origine.
    fn load_root_view(&self, _base: &BaseId) -> Result<Option<ListView>, RepositoryError> {
        let manifest = read(&self.path)?;
        Ok(manifest
            .root_view
            .and_then(|value| serde_json::from_value::<ViewFile>(value).ok())
            .and_then(|file| file.to_view()))
    }

    fn save_root_view(&self, _base: &BaseId, view: &ListView) -> Result<(), RepositoryError> {
        let mut manifest = read(&self.path)?;
        let value = serde_json::to_value(ViewFile::from_view(view))
            .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        manifest.root_view = Some(value);
        write(&self.path, &manifest)
    }
}

/// Coffre de la base, enregistré dans `manifest.json`.
pub struct FsVaultStore {
    path: PathBuf,
}

impl FsVaultStore {
    /// Ouvre (ou initialise) le manifeste de l'espace situé dans `root`.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si le manifeste est illisible ou d'une version plus récente.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RepositoryError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| unavailable(&root, &error))?;
        ensure(&root)?;
        Ok(Self {
            path: root.join(FILE_NAME),
        })
    }
}

impl VaultStore for FsVaultStore {
    /// Un dossier = une base : `base` est ignoré.
    fn load_vault(&self, _base: &BaseId) -> Result<Option<VaultRecord>, RepositoryError> {
        read(&self.path)?
            .vault
            .as_ref()
            .map(VaultFile::to_record)
            .transpose()
    }

    /// Refuse de remplacer un coffre par un **autre** (ses tâches deviendraient illisibles) ;
    /// réécrire le même coffre (nouveau mot de passe, un jour) est permis.
    fn save_vault(&self, _base: &BaseId, vault: &VaultRecord) -> Result<(), RepositoryError> {
        let mut manifest = read(&self.path)?;
        if let Some(existing) = &manifest.vault
            && existing.key_id != vault.key_id.as_str()
        {
            return Err(RepositoryError::Vault(VaultError::AlreadyExists));
        }
        manifest.vault = Some(VaultFile::from_record(vault));
        write(&self.path, &manifest)
    }
}
