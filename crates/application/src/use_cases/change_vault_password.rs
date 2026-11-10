//! Changer le mot de passe du coffre d'une base (spec §9, ADR-0010).
//!
//! Seule l'**enveloppe** de la clé change : les tâches et les médias ne sont pas rechiffrés, le
//! trousseau garde la même clé. Les autres appareils demanderont le nouveau mot de passe au
//! prochain déverrouillage (le manifeste voyage avec la base).

use crate::AppError;
use crate::ports::{VaultKeys, VaultStore};
use crate::vault::{Password, VaultError};
use std::sync::Arc;
use tq_domain::BaseId;

/// Cas d'usage « changer le mot de passe ».
pub struct ChangeVaultPassword {
    store: Arc<dyn VaultStore>,
    keys: Arc<dyn VaultKeys>,
}

impl ChangeVaultPassword {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(store: Arc<dyn VaultStore>, keys: Arc<dyn VaultKeys>) -> Self {
        Self { store, keys }
    }

    /// Remplace `current` par `new` pour le coffre de `base` (ouvert ou non).
    ///
    /// # Erreurs
    /// [`VaultError::NoVault`], [`VaultError::WrongPassword`] (mot de passe actuel), chiffrement,
    /// stockage.
    pub fn execute(
        &self,
        base: &BaseId,
        current: &Password,
        new: &Password,
    ) -> Result<(), AppError> {
        let record = self.store.load_vault(base)?.ok_or(VaultError::NoVault)?;
        let key = self.keys.unlock(&record, current)?;
        let rewrapped = self.keys.rewrap(&record, &key, new)?;
        self.store.save_vault(base, &rewrapped)?;
        Ok(())
    }
}
