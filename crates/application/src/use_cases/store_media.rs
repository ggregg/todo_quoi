//! Stocker un média (image collée, fichier glissé, message audio enregistré).
//!
//! Le contenu arrive déjà préparé par l'interface (image compressée, audio encodé : POC I0-2) ;
//! ici on vérifie la taille, puis on le range **dans la base de la tâche** dont la note le cite
//! (ADR-0009). La note le citera par `../media/<référence>`.

use crate::AppError;
use crate::ports::{MediaStore, TaskRepository};
use std::sync::Arc;
use tq_domain::{MediaRef, TaskId, TaskTree, check_media_size};

/// Cas d'usage « stocker un média ».
pub struct StoreMedia {
    media: Arc<dyn MediaStore>,
    tasks: Arc<dyn TaskRepository>,
}

impl StoreMedia {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(media: Arc<dyn MediaStore>, tasks: Arc<dyn TaskRepository>) -> Self {
        Self { media, tasks }
    }

    /// Stocke `bytes` (extension `jpg`, `webm`, `pdf`…) pour la note de `task_id`.
    ///
    /// # Erreurs
    /// Fichier de plus de 50 Mo, extension invalide, tâche introuvable, stockage.
    pub fn execute(
        &self,
        task_id: &TaskId,
        bytes: &[u8],
        extension: &str,
    ) -> Result<MediaRef, AppError> {
        check_media_size(bytes.len())?;
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        let base = tree.get(task_id)?.base().clone();
        Ok(self.media.store(&base, bytes, extension)?)
    }
}
