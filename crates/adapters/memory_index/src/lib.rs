//! # Index de recherche en mémoire
//!
//! **Place dans l'hexagone** : adaptateur secondaire, implémente le port
//! [`SearchIndex`](tq_application::ports::SearchIndex).
//!
//! Une table `TaskId → fiche` protégée par un verrou. L'algorithme de recherche (correspondance,
//! ordre, extraits) est une règle métier : il est dans `tq_domain::search` ; cet adaptateur ne
//! fait que **conserver** les fiches. Cible : < 10 ms pour 10 000 tâches (archi §6).
//!
//! Plus tard (I5) : instantané `cache.json` pour un démarrage rapide.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tq_application::ports::SearchIndex;
use tq_domain::search::{MediaKind, SearchDocument, SearchHit, SearchQuery, search_documents};
use tq_domain::{TaskId, Timestamp};

/// Index de recherche en mémoire, partagé entre les fils d'exécution.
// [rust] `RwLock` : verrou « plusieurs lecteurs OU un seul écrivain » — les recherches
//        (lectures) peuvent avoir lieu en même temps ; une mise à jour attend son tour.
#[derive(Default)]
pub struct MemorySearchIndex {
    documents: RwLock<HashMap<TaskId, SearchDocument>>,
}

impl MemorySearchIndex {
    /// Index vide (il se remplit au premier chargement des tâches).
    pub fn new() -> Self {
        Self::default()
    }

    /// Nombre de fiches.
    pub fn len(&self) -> usize {
        self.read().len()
    }

    /// Vrai si l'index est vide.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Un verrou « empoisonné » (un fil a paniqué en écrivant) : l'index n'est qu'un cache,
    // reconstruit au prochain chargement — on continue avec son contenu.
    fn read(&self) -> RwLockReadGuard<'_, HashMap<TaskId, SearchDocument>> {
        self.documents
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, HashMap<TaskId, SearchDocument>> {
        self.documents
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl SearchIndex for MemorySearchIndex {
    fn indexed_version(&self, id: &TaskId) -> Option<Timestamp> {
        self.read().get(id).map(|document| document.version)
    }

    fn upsert(&self, document: SearchDocument) {
        self.write().insert(document.id.clone(), document);
    }

    fn remove(&self, id: &TaskId) {
        self.write().remove(id);
    }

    fn retain_only(&self, ids: &[TaskId]) {
        let keep: HashSet<&TaskId> = ids.iter().collect();
        self.write().retain(|id, _| keep.contains(id));
    }

    fn search(&self, query: &SearchQuery, limit: usize) -> Vec<SearchHit> {
        search_documents(&self.read(), query, limit)
    }

    fn media_of(&self, id: &TaskId) -> Vec<MediaKind> {
        self.read()
            .get(id)
            .map(|document| document.media.clone())
            .unwrap_or_default()
    }
}
