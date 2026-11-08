//! Créer le coffre d'une base, au premier chiffrement (spec §9, ADR-0010).

use crate::AppError;
use crate::ports::{KeyVault, VaultKeys, VaultStore};
use crate::vault::{OpenVaults, Password, Remember, VaultError};
use std::sync::Arc;
use tq_domain::BaseId;

/// Cas d'usage « créer le coffre d'une base ». Le coffre créé est aussitôt ouvert.
pub struct CreateVault {
    store: Arc<dyn VaultStore>,
    keys: Arc<dyn VaultKeys>,
    keyring: Arc<dyn KeyVault>,
    vaults: Arc<OpenVaults>,
}

impl CreateVault {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        store: Arc<dyn VaultStore>,
        keys: Arc<dyn VaultKeys>,
        keyring: Arc<dyn KeyVault>,
        vaults: Arc<OpenVaults>,
    ) -> Self {
        Self {
            store,
            keys,
            keyring,
            vaults,
        }
    }

    /// Crée le coffre de `base` protégé par `password`, et l'ouvre.
    ///
    /// # Erreurs
    /// [`VaultError::AlreadyExists`] (on ne remplace jamais un coffre : ses tâches deviendraient
    /// illisibles), chiffrement, trousseau, stockage.
    pub fn execute(
        &self,
        base: &BaseId,
        password: &Password,
        remember: Remember,
    ) -> Result<(), AppError> {
        if self.store.load_vault(base)?.is_some() {
            return Err(VaultError::AlreadyExists.into());
        }
        let (record, key) = self.keys.create(password)?;
        self.store.save_vault(base, &record)?;
        if remember == Remember::OnThisDevice {
            self.keyring.remember(&record.key_id, &key)?;
        }
        self.vaults.open(base.clone(), record.key_id, key);
        Ok(())
    }
}
