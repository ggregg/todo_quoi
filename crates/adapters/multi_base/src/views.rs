//! Vues racines et coffres de plusieurs bases, et liste des bases ouvertes.

use crate::find;
use std::sync::Arc;
use tq_application::ports::{
    BaseDirectory, RepositoryError, RootViewStore, VaultRecord, VaultStore,
};
use tq_domain::BaseId;
use tq_domain::view::ListView;

/// `RootViewStore` de chaque base (son `manifest.json`), présentés comme un seul.
pub struct MultiBaseRootViews {
    bases: Vec<(BaseId, Arc<dyn RootViewStore>)>,
}

impl MultiBaseRootViews {
    /// Assemble les vues racines des bases ouvertes.
    pub fn new(bases: Vec<(BaseId, Arc<dyn RootViewStore>)>) -> Self {
        Self { bases }
    }
}

impl RootViewStore for MultiBaseRootViews {
    fn load_root_view(&self, base: &BaseId) -> Result<Option<ListView>, RepositoryError> {
        find(&self.bases, base)?.load_root_view(base)
    }

    fn save_root_view(&self, base: &BaseId, view: &ListView) -> Result<(), RepositoryError> {
        find(&self.bases, base)?.save_root_view(base, view)
    }
}

/// Les bases ouvertes, dans l'ordre choisi (bandeau).
pub struct OpenBases(Vec<BaseId>);

impl OpenBases {
    /// Bases ouvertes, dans l'ordre d'affichage.
    pub fn new(bases: Vec<BaseId>) -> Self {
        Self(bases)
    }
}

impl BaseDirectory for OpenBases {
    fn bases(&self) -> Vec<BaseId> {
        self.0.clone()
    }
}

/// `VaultStore` de chaque base (son `manifest.json`), présentés comme un seul : chaque base a
/// son propre coffre (ADR-0010).
pub struct MultiBaseVaults {
    bases: Vec<(BaseId, Arc<dyn VaultStore>)>,
}

impl MultiBaseVaults {
    /// Assemble les coffres des bases ouvertes.
    pub fn new(bases: Vec<(BaseId, Arc<dyn VaultStore>)>) -> Self {
        Self { bases }
    }
}

impl VaultStore for MultiBaseVaults {
    fn load_vault(&self, base: &BaseId) -> Result<Option<VaultRecord>, RepositoryError> {
        find(&self.bases, base)?.load_vault(base)
    }

    fn save_vault(&self, base: &BaseId, vault: &VaultRecord) -> Result<(), RepositoryError> {
        find(&self.bases, base)?.save_vault(base, vault)
    }
}
