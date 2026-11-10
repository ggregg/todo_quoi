//! **Couche application** : implémentation du port `ContentSealer` (pas d'E/S), pour **une** base : la clé vient des coffres ouverts
//! ([`OpenVaults`]), le chiffrement du port `Cipher`.

use crate::ports::{Cipher, ContentSealer};
use crate::vault::{OpenVaults, VaultError};
use std::sync::Arc;
use tq_domain::{BaseId, SealedContent};

/// Scelle et descelle le contenu des tâches de la base `base`.
pub struct BaseSealer {
    base: BaseId,
    vaults: Arc<OpenVaults>,
    cipher: Arc<dyn Cipher>,
}

impl BaseSealer {
    /// Scelleur de la base `base`.
    pub fn new(base: BaseId, vaults: Arc<OpenVaults>, cipher: Arc<dyn Cipher>) -> Self {
        Self {
            base,
            vaults,
            cipher,
        }
    }
}

impl ContentSealer for BaseSealer {
    fn seal(&self, plaintext: &[u8], associated: &[u8]) -> Result<SealedContent, VaultError> {
        let (key_id, key) = self.vaults.key_of(&self.base).ok_or(VaultError::Locked)?;
        let sealed = self.cipher.seal(&key, plaintext, associated)?;
        Ok(SealedContent { key_id, sealed })
    }

    fn open(
        &self,
        sealed: &SealedContent,
        associated: &[u8],
    ) -> Result<Option<Vec<u8>>, VaultError> {
        let Some((key_id, key)) = self.vaults.key_of(&self.base) else {
            return Ok(None); // verrouillé : la tâche reste scellée
        };
        if key_id != sealed.key_id {
            return Ok(None); // chiffrée par un autre coffre (voir ADR-0010)
        }
        self.cipher.open(&key, &sealed.sealed, associated).map(Some)
    }
}
