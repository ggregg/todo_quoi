//! `manifest.json` : décrit l'espace de données, à la racine du dossier.
//!
//! - `schema_version` : permet de refuser proprement des données écrites par une version **plus
//!   récente** de l'application (spécification §8 : inviter à mettre à jour) ;
//! - `root_view` (facultatif) : vue de la liste racine (format : [`crate::view_format`]),
//!   implémente le port [`RootViewStore`] via [`FsRootViewStore`].
//!
//! Les champs inconnus sont conservés à la réécriture.

use crate::format::SCHEMA_VERSION;
use crate::storage::write_atomically;
use crate::view_format::ViewFile;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tq_application::ports::{RepositoryError, RootViewStore};
use tq_domain::view::ListView;

const FILE_NAME: &str = "manifest.json";

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    root_view: Option<Value>,
    #[serde(flatten)]
    unknown: Map<String, Value>,
}

/// Crée le manifeste s'il n'existe pas, sinon vérifie que sa version est gérée.
pub(crate) fn ensure(root: &Path) -> Result<(), RepositoryError> {
    let path = root.join(FILE_NAME);
    if !path.exists() {
        let manifest = Manifest {
            schema_version: SCHEMA_VERSION,
            root_view: None,
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
    /// Une vue illisible (modifiée à la main, version plus récente) est ignorée : vue d'origine.
    fn load_root_view(&self) -> Result<Option<ListView>, RepositoryError> {
        let manifest = read(&self.path)?;
        Ok(manifest
            .root_view
            .and_then(|value| serde_json::from_value::<ViewFile>(value).ok())
            .and_then(|file| file.to_view()))
    }

    fn save_root_view(&self, view: &ListView) -> Result<(), RepositoryError> {
        let mut manifest = read(&self.path)?;
        let value = serde_json::to_value(ViewFile::from_view(view))
            .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        manifest.root_view = Some(value);
        write(&self.path, &manifest)
    }
}
