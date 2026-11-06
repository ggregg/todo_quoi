//! `manifest.json` : décrit l'espace de données (version du format), à la racine du dossier.
//!
//! Il permet de refuser proprement des données écrites par une version **plus récente** de
//! l'application (spécification §8 : passer en lecture seule et inviter à mettre à jour).

use crate::format::SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tq_application::ports::RepositoryError;

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    schema_version: u32,
}

/// Crée le manifeste s'il n'existe pas, sinon vérifie que sa version est gérée.
pub(crate) fn ensure(root: &Path) -> Result<(), RepositoryError> {
    let path = root.join("manifest.json");
    if !path.exists() {
        let text = serde_json::to_string_pretty(&Manifest {
            schema_version: SCHEMA_VERSION,
        })
        .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        return fs::write(&path, text + "\n").map_err(|error| unavailable(&path, &error));
    }
    let text = fs::read_to_string(&path).map_err(|error| unavailable(&path, &error))?;
    let manifest: Manifest = serde_json::from_str(&text).map_err(|error| {
        RepositoryError::Unavailable(format!("{} illisible : {error}", path.display()))
    })?;
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(RepositoryError::NewerSchema {
            found: manifest.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    Ok(())
}

pub(crate) fn unavailable(path: &Path, error: &std::io::Error) -> RepositoryError {
    RepositoryError::Unavailable(format!("{} : {error}", path.display()))
}
