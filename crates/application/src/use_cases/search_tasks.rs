//! Rechercher des tâches (`Mod+F`) — spec §5.6.

use crate::ports::SearchIndex;
use std::sync::Arc;
use tq_domain::BaseId;
use tq_domain::search::{SearchHit, SearchQuery};
use tq_domain::view::ArchiveVisibility;

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

    /// Résultats pour la saisie `text` (vide = aucun résultat), dans les bases `bases` (vide =
    /// toutes : ce sont les bases affichées, ADR-0009), archives comprises si `archives` vaut
    /// `Shown` (bouton « Archives », R15).
    ///
    /// L'index est alimenté par [`crate::IndexingTaskRepository`] dès le premier chargement de
    /// la liste (et les archives dès qu'elles sont affichées) : il est donc prêt quand
    /// l'utilisateur ouvre la recherche.
    pub fn execute(
        &self,
        text: &str,
        bases: Vec<BaseId>,
        archives: ArchiveVisibility,
    ) -> Vec<SearchHit> {
        let query = SearchQuery::parse(text)
            .in_bases(bases)
            .with_archives(archives);
        if query.is_empty() {
            return Vec::new();
        }
        self.index.search(&query, MAX_RESULTS)
    }
}
