//! Médias dans `<racine>/media/` : un fichier par contenu, nommé `<sha256>.<extension>`.
//!
//! - **Immuables** : un média n'est jamais réécrit ; stocker deux fois le même contenu ne crée
//!   qu'un fichier (déduplication gratuite, aucun conflit de synchronisation possible).
//! - **Écriture atomique**, comme pour les tâches (`.~<nom>.tmp` puis renommage).

use crate::manifest::unavailable;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tq_application::ports::{MediaStore, RepositoryError};
use tq_domain::{BaseId, MediaRef};

/// Stockage des médias dans un dossier.
pub struct FsMediaStore {
    media_dir: PathBuf,
}

impl FsMediaStore {
    /// Ouvre (et crée au besoin) `<racine>/media/`.
    ///
    /// # Erreurs
    /// [`RepositoryError::Unavailable`] si le dossier ne peut pas être créé.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RepositoryError> {
        let media_dir = root.into().join("media");
        fs::create_dir_all(&media_dir).map_err(|error| unavailable(&media_dir, &error))?;
        Ok(Self { media_dir })
    }

    fn path_of(&self, media: &MediaRef) -> PathBuf {
        // `MediaRef` est validé (ni `/` ni `..`) : le chemin reste dans le dossier des médias.
        self.media_dir.join(media.as_str())
    }
}

impl MediaStore for FsMediaStore {
    /// Un dossier = une base : `base` est ignoré (l'aiguillage entre bases est fait par la crate
    /// `multi_base`).
    fn store(
        &self,
        _base: &BaseId,
        bytes: &[u8],
        extension: &str,
    ) -> Result<MediaRef, RepositoryError> {
        let media = MediaRef::from_parts(&sha256_hex(bytes), extension)
            .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        let path = self.path_of(&media);
        if !path.exists() {
            write_atomically(&path, bytes).map_err(|error| unavailable(&path, &error))?;
        }
        Ok(media)
    }

    fn read(&self, media: &MediaRef) -> Result<Vec<u8>, RepositoryError> {
        let path = self.path_of(media);
        fs::read(&path).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => RepositoryError::NotFound(media.to_string()),
            _ => unavailable(&path, &error),
        })
    }

    /// `None` si le média n'est pas dans **cette** base : avec plusieurs bases, c'est ce qui
    /// permet de trouver celle qui l'a (un chemin inexistant ne s'ouvre pas).
    fn local_path(&self, media: &MediaRef) -> Option<PathBuf> {
        Some(self.path_of(media)).filter(|path| path.is_file())
    }

    /// Un dossier = une base : `base` est ignoré.
    fn remove(&self, _base: &BaseId, media: &MediaRef) -> Result<(), RepositoryError> {
        let path = self.path_of(media);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(unavailable(&path, &error)),
        }
    }
}

/// Empreinte SHA-256 en hexadécimal minuscule (64 caractères).
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = path.with_file_name(format!(".~{name}.tmp"));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_a_known_value() {
        // Valeur de référence publique : SHA-256("abc").
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
