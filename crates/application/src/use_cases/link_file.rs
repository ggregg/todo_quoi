//! Lier un fichier depuis une note : la note retient son chemin et **cet appareil** ; le fichier
//! reste où il est (rien n'est copié dans la base).

use crate::AppError;
use crate::ports::{LocalFiles, RepositoryError};
use crate::use_cases::attach_file::file_name;
use std::path::Path;
use std::sync::Arc;
use tq_domain::{DeviceId, LinkedFile, LocalFileKind};

/// Cas d'usage « lier un fichier ».
pub struct LinkFile {
    files: Arc<dyn LocalFiles>,
    device: DeviceId,
}

impl LinkFile {
    /// Assemble le cas d'usage ; `device` : cet appareil (propriétaire des liens créés ici).
    pub fn new(files: Arc<dyn LocalFiles>, device: DeviceId) -> Self {
        Self { files, device }
    }

    /// Le lien à écrire dans la note.
    ///
    /// # Erreurs
    /// [`RepositoryError::NotFound`] si rien n'existe à `path`.
    pub fn execute(&self, path: &Path) -> Result<LinkedFile, AppError> {
        if self.files.kind(path) == LocalFileKind::Missing {
            return Err(RepositoryError::NotFound(path.display().to_string()).into());
        }
        Ok(LinkedFile {
            path: path.display().to_string(),
            name: file_name(path),
            device: self.device.clone(),
        })
    }
}
