//! Vues racines de plusieurs bases, et liste des bases ouvertes.

use crate::find;
use std::sync::Arc;
use tq_application::ports::{BaseDirectory, RepositoryError, RootViewStore};
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
