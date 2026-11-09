//! Joindre un fichier choisi dans le Finder / l'Explorateur : le **copier** dans la base de la
//! tâche, ou l'y **déplacer** (copie, puis effacement de l'original).

use crate::AppError;
use crate::ports::{LocalFiles, RepositoryError};
use crate::use_cases::store_media::StoreMedia;
use std::path::Path;
use std::sync::Arc;
use tq_domain::{LocalFileKind, MediaRef, TaskId};

/// Garder l'original, ou le retirer de son dossier ? (Énumération plutôt que booléen.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachMode {
    /// Copier : l'original reste où il est.
    Copy,
    /// Déplacer : l'original est effacé une fois la copie rangée dans la base.
    Move,
}

/// Fichier joint : le média rangé dans la base, son nom d'origine et sa taille.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedFile {
    /// Média (la note le cite par `../media/<référence>`).
    pub media: MediaRef,
    /// Nom d'origine (affiché dans la note).
    pub name: String,
    /// Taille en octets.
    pub size: usize,
}

/// Cas d'usage « joindre un fichier ».
pub struct AttachFile {
    store: Arc<StoreMedia>,
    files: Arc<dyn LocalFiles>,
}

impl AttachFile {
    /// Assemble le cas d'usage : le rangement est celui de « stocker un média » (taille
    /// maximale, base de la tâche).
    pub fn new(store: Arc<StoreMedia>, files: Arc<dyn LocalFiles>) -> Self {
        Self { store, files }
    }

    /// Joint le fichier `path` à la note de `task_id`.
    ///
    /// # Erreurs
    /// Fichier introuvable ou dossier ; plus de 50 Mo ; tâche introuvable ; stockage. En mode
    /// déplacer, l'original n'est effacé que si la copie a réussi.
    pub fn execute(
        &self,
        task_id: &TaskId,
        path: &Path,
        mode: AttachMode,
    ) -> Result<AttachedFile, AppError> {
        if matches!(
            self.files.kind(path),
            LocalFileKind::Missing | LocalFileKind::Directory
        ) {
            return Err(RepositoryError::NotFound(path.display().to_string()).into());
        }
        let bytes = self.files.read(path)?;
        let media = self.store.execute(task_id, &bytes, &extension_of(path))?;
        if mode == AttachMode::Move {
            self.files.remove(path)?;
        }
        Ok(AttachedFile {
            media,
            name: file_name(path),
            size: bytes.len(),
        })
    }
}

/// Nom du fichier (`devis.pdf`), ou le chemin entier s'il n'en a pas.
pub(crate) fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Extension en minuscules (`PDF` → `pdf`) ; `bin` sans extension.
fn extension_of(path: &Path) -> String {
    path.extension().map_or_else(
        || "bin".to_string(),
        |ext| ext.to_string_lossy().to_lowercase(),
    )
}
