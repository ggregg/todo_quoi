//! Lire un média (affichage d'une image, lecture d'un audio, ouverture d'une pièce jointe).
//! Un média chiffré est rendu en clair si un coffre ouvert le déchiffre.

use crate::AppError;
use crate::ports::MediaStore;
use crate::vault::{SealedMedia, VaultError};
use std::path::PathBuf;
use std::sync::Arc;
use tq_domain::MediaRef;

/// Cas d'usage « lire un média » (lecture seule).
pub struct ReadMedia {
    media: Arc<dyn MediaStore>,
    sealed: Arc<SealedMedia>,
}

impl ReadMedia {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(media: Arc<dyn MediaStore>, sealed: Arc<SealedMedia>) -> Self {
        Self { media, sealed }
    }

    /// Contenu du média `name` (nom tel qu'écrit dans la note, sans `../media/`).
    ///
    /// # Erreurs
    /// Nom invalide, média introuvable, coffre verrouillé (média chiffré), stockage.
    pub fn execute(&self, name: &str) -> Result<(MediaRef, Vec<u8>), AppError> {
        let media = MediaRef::parse(name)?;
        let bytes = self.sealed.read(&media)?;
        Ok((media, bytes))
    }

    /// Emplacement local du média, pour l'ouvrir avec l'application par défaut.
    ///
    /// # Erreurs
    /// Nom invalide ; [`VaultError::EncryptedMedia`] pour un média chiffré (il n'existe pas en
    /// clair : l'ouvrir passe par une copie déchiffrée, voir `execute`) ;
    /// [`crate::ports::RepositoryError::NotFound`] si absent du disque.
    pub fn local_path(&self, name: &str) -> Result<PathBuf, AppError> {
        let media = MediaRef::parse(name)?;
        if media.is_sealed() {
            return Err(VaultError::EncryptedMedia.into());
        }
        self.media
            .local_path(&media)
            .filter(|path| path.exists())
            .ok_or_else(|| crate::ports::RepositoryError::NotFound(media.to_string()).into())
    }
}
