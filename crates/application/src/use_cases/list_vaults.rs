//! État du coffre de chaque base ouverte (pastilles du bandeau, bouton 🔒).

use crate::AppError;
use crate::ports::{BaseDirectory, VaultStore};
use crate::vault::{OpenVaults, VaultState};
use std::sync::Arc;
use tq_domain::BaseId;

/// Coffre d'une base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseVault {
    /// La base.
    pub base: BaseId,
    /// Aucun coffre, verrouillé ou ouvert.
    pub state: VaultState,
}

/// Cas d'usage « état des coffres » (lecture seule).
pub struct ListVaults {
    bases: Arc<dyn BaseDirectory>,
    store: Arc<dyn VaultStore>,
    vaults: Arc<OpenVaults>,
}

impl ListVaults {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        bases: Arc<dyn BaseDirectory>,
        store: Arc<dyn VaultStore>,
        vaults: Arc<OpenVaults>,
    ) -> Self {
        Self {
            bases,
            store,
            vaults,
        }
    }

    /// Un état par base ouverte, dans l'ordre du bandeau.
    ///
    /// # Erreurs
    /// Stockage (manifeste illisible).
    pub fn execute(&self) -> Result<Vec<BaseVault>, AppError> {
        self.bases
            .bases()
            .into_iter()
            .map(|base| {
                let state = self.state_of(&base)?;
                Ok(BaseVault { base, state })
            })
            .collect()
    }

    fn state_of(&self, base: &BaseId) -> Result<VaultState, AppError> {
        if self.vaults.is_open(base) {
            return Ok(VaultState::Unlocked);
        }
        Ok(match self.store.load_vault(base)? {
            Some(_) => VaultState::Locked,
            None => VaultState::None,
        })
    }
}
