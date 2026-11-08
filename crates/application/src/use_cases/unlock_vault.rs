//! Déverrouiller le coffre d'une base avec son mot de passe (spec §9, ADR-0010).

use crate::AppError;
use crate::ports::{KeyVault, VaultKeys, VaultStore};
use crate::vault::{OpenVaults, Password, Remember, VaultError};
use std::sync::Arc;
use tq_domain::BaseId;

/// Cas d'usage « déverrouiller ». Les tâches scellées de la base redeviennent lisibles au
/// chargement suivant (l'interface recharge la liste).
pub struct UnlockVault {
    store: Arc<dyn VaultStore>,
    keys: Arc<dyn VaultKeys>,
    keyring: Arc<dyn KeyVault>,
    vaults: Arc<OpenVaults>,
}

impl UnlockVault {
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

    /// Ouvre le coffre de `base` ; `remember` décide s'il s'ouvrira seul au prochain lancement.
    ///
    /// # Erreurs
    /// [`VaultError::NoVault`], [`VaultError::WrongPassword`], trousseau, stockage.
    pub fn execute(
        &self,
        base: &BaseId,
        password: &Password,
        remember: Remember,
    ) -> Result<(), AppError> {
        let record = self.store.load_vault(base)?.ok_or(VaultError::NoVault)?;
        let key = self.keys.unlock(&record, password)?;
        match remember {
            Remember::OnThisDevice => self.keyring.remember(&record.key_id, &key)?,
            Remember::NotOnThisDevice => self.keyring.forget(&record.key_id)?,
        }
        self.vaults.open(base.clone(), record.key_id, key);
        Ok(())
    }
}
