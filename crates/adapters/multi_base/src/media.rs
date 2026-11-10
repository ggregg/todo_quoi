//! Médias de plusieurs bases.

use crate::find;
use std::path::PathBuf;
use std::sync::Arc;
use tq_application::ports::{MediaStore, RepositoryError};
use tq_domain::{BaseId, MediaRef};

/// `MediaStore` de chaque base, présentés comme un seul.
pub struct MultiBaseMediaStore {
    bases: Vec<(BaseId, Arc<dyn MediaStore>)>,
}

impl MultiBaseMediaStore {
    /// Assemble les stockages de médias des bases ouvertes.
    pub fn new(bases: Vec<(BaseId, Arc<dyn MediaStore>)>) -> Self {
        Self { bases }
    }

    /// Copie `media` de la base `from` vers la base `to` (déplacement d'une tâche). Un média
    /// absent de `from` est ignoré : la note le citait sans qu'il existe. Un média chiffré est
    /// recopié tel quel, sous son nom (l'application rechiffre ceux des tâches chiffrées avant
    /// de les déplacer : ce cas ne se présente que pour un média cité ailleurs).
    ///
    /// # Erreurs
    /// Base inconnue, écriture impossible.
    pub fn copy(
        &self,
        media: &MediaRef,
        from: &BaseId,
        to: &BaseId,
    ) -> Result<(), RepositoryError> {
        let Ok(bytes) = find(&self.bases, from)?.read(media) else {
            return Ok(());
        };
        let target = find(&self.bases, to)?;
        if media.is_sealed() {
            return target.put(to, media, &bytes);
        }
        target.store(to, &bytes, media.extension())?;
        Ok(())
    }
}

impl MediaStore for MultiBaseMediaStore {
    fn store(
        &self,
        base: &BaseId,
        bytes: &[u8],
        extension: &str,
    ) -> Result<MediaRef, RepositoryError> {
        find(&self.bases, base)?.store(base, bytes, extension)
    }

    fn put(&self, base: &BaseId, media: &MediaRef, bytes: &[u8]) -> Result<(), RepositoryError> {
        find(&self.bases, base)?.put(base, media, bytes)
    }

    /// Cherche dans chaque base : même nom = même contenu (empreinte).
    fn read(&self, media: &MediaRef) -> Result<Vec<u8>, RepositoryError> {
        self.bases
            .iter()
            .find_map(|(_, store)| store.read(media).ok())
            .ok_or_else(|| RepositoryError::NotFound(media.to_string()))
    }

    fn local_path(&self, media: &MediaRef) -> Option<PathBuf> {
        self.bases
            .iter()
            .find_map(|(_, store)| store.local_path(media))
    }

    /// Efface dans **cette** base seulement : les autres bases gardent leur copie.
    fn remove(&self, base: &BaseId, media: &MediaRef) -> Result<(), RepositoryError> {
        find(&self.bases, base)?.remove(base, media)
    }
}
