//! Au lancement : rouvrir les coffres dont la clé est gardée dans le trousseau de l'OS
//! (« Se souvenir sur cet appareil », ADR-0010).

use crate::AppError;
use crate::ports::{BaseDirectory, KeyVault, VaultStore};
use crate::vault::{OpenVaults, Remember};
use std::sync::Arc;
use tq_domain::BaseId;

/// Cas d'usage « rouvrir les coffres mémorisés ».
pub struct RestoreRememberedVaults {
    bases: Arc<dyn BaseDirectory>,
    store: Arc<dyn VaultStore>,
    keyring: Arc<dyn KeyVault>,
    vaults: Arc<OpenVaults>,
}

impl RestoreRememberedVaults {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        bases: Arc<dyn BaseDirectory>,
        store: Arc<dyn VaultStore>,
        keyring: Arc<dyn KeyVault>,
        vaults: Arc<OpenVaults>,
    ) -> Self {
        Self {
            bases,
            store,
            keyring,
            vaults,
        }
    }

    /// Ouvre chaque coffre mémorisé ; renvoie les bases rouvertes. Une base dont le coffre ne
    /// peut pas être lu est laissée verrouillée (l'utilisateur saisira son mot de passe).
    pub fn execute(&self) -> Vec<BaseId> {
        self.bases
            .bases()
            .into_iter()
            .filter(|base| !self.vaults.is_open(base))
            .filter(|base| self.restore(base).unwrap_or(false))
            .collect()
    }

    fn restore(&self, base: &BaseId) -> Result<bool, AppError> {
        let Some(record) = self.store.load_vault(base)? else {
            return Ok(false);
        };
        let Some(key) = self.keyring.recall(&record.key_id)? else {
            return Ok(false);
        };
        self.vaults
            .open(base.clone(), record.key_id, key, Remember::OnThisDevice);
        Ok(true)
    }
}
