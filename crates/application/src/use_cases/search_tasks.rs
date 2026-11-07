//! Rechercher des tâches (`Mod+F`) — spec §5.6.

use crate::ports::SearchIndex;
use std::sync::Arc;
use tq_domain::search::{SearchHit, SearchQuery};

/// Nombre maximal de résultats renvoyés : au-delà, l'utilisateur affine sa recherche.
pub const MAX_RESULTS: usize = 200;

/// Cas d'usage « rechercher » (lecture seule, sur l'index : aucun accès au disque).
pub struct SearchTasks {
    index: Arc<dyn SearchIndex>,
}

impl SearchTasks {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(index: Arc<dyn SearchIndex>) -> Self {
        Self { index }
    }

    /// Résultats pour la saisie `text` (vide = aucun résultat).
    ///
    /// L'index est alimenté par [`crate::IndexingTaskRepository`] dès le premier chargement de
    /// la liste : il est donc prêt quand l'utilisateur ouvre la recherche.
    pub fn execute(&self, text: &str) -> Vec<SearchHit> {
        let query = SearchQuery::parse(text);
        if query.is_empty() {
            return Vec::new();
        }
        self.index.search(&query, MAX_RESULTS)
    }
}
