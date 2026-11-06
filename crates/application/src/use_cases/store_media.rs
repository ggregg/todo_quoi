//! Stocker un média (image collée, fichier glissé, message audio enregistré).
//!
//! Le contenu arrive déjà préparé par l'interface (image compressée, audio encodé : POC I0-2) ;
//! ici on vérifie la taille, puis on le range. La note le citera par `../media/<référence>`.

use crate::AppError;
use crate::ports::MediaStore;
use std::sync::Arc;
use tq_domain::{MediaRef, check_media_size};

/// Cas d'usage « stocker un média ».
pub struct StoreMedia {
    media: Arc<dyn MediaStore>,
}

impl StoreMedia {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(media: Arc<dyn MediaStore>) -> Self {
        Self { media }
    }

    /// Stocke `bytes` avec l'extension `extension` (`jpg`, `webm`, `pdf`…).
    ///
    /// # Erreurs
    /// Fichier de plus de 50 Mo, extension invalide, stockage.
    pub fn execute(&self, bytes: &[u8], extension: &str) -> Result<MediaRef, AppError> {
        check_media_size(bytes.len())?;
        Ok(self.media.store(bytes, extension)?)
    }
}
