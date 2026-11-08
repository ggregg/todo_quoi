//! Verrouiller le coffre d'une base (spec §9, ADR-0010).

use crate::vault::OpenVaults;
use std::sync::Arc;
use tq_domain::BaseId;

/// Cas d'usage « verrouiller » : la clé est effacée de la mémoire ; les tâches chiffrées de la
/// base redeviennent scellées. La clé éventuellement gardée dans le trousseau y reste (le coffre
/// se rouvrira au prochain lancement) : c'est le déverrouillage qui choisit de s'en souvenir.
pub struct LockVault {
    vaults: Arc<OpenVaults>,
}

impl LockVault {
    /// Assemble le cas d'usage.
    pub fn new(vaults: Arc<OpenVaults>) -> Self {
        Self { vaults }
    }

    /// Referme le coffre de `base` (déjà fermé : rien à faire).
    pub fn execute(&self, base: &BaseId) {
        self.vaults.close(base);
    }
}
