//! Tenir l'index de recherche à jour **sans** toucher aux cas d'usage.
//!
//! [`IndexingTaskRepository`] est un *décorateur* : il implémente [`TaskRepository`] en
//! déléguant au vrai dépôt, et met à jour l'index au passage (principe ouvert/fermé : on
//! ajoute un comportement sans modifier le code existant). La composition root
//! (`src-tauri/src/composition.rs`) le place entre les cas d'usage et le stockage.
//!
//! - `load_all` : refait les fiches des tâches qui ont changé (nouvelle version) et oublie celles
//!   qui ont disparu ; c'est aussi ce qui prend en compte les modifications venues d'un autre
//!   appareil ;
//! - `save` : refait la fiche de la tâche (ou la retire si elle part à la corbeille).

use crate::ports::{NoteDigester, RepositoryError, SearchIndex, TaskRepository};
use std::sync::Arc;
use tq_domain::search::SearchDocument;
use tq_domain::{Task, TaskId};

/// Dépôt de tâches qui alimente l'index de recherche.
pub struct IndexingTaskRepository {
    inner: Arc<dyn TaskRepository>,
    index: Arc<dyn SearchIndex>,
    digester: Arc<dyn NoteDigester>,
}

impl IndexingTaskRepository {
    /// Enveloppe `inner` ; l'index est alimenté avec les notes extraites par `digester`.
    pub fn new(
        inner: Arc<dyn TaskRepository>,
        index: Arc<dyn SearchIndex>,
        digester: Arc<dyn NoteDigester>,
    ) -> Self {
        Self {
            inner,
            index,
            digester,
        }
    }

    fn index_task(&self, task: &Task) {
        if task.is_deleted() {
            self.index.remove(task.id());
            return;
        }
        if self.index.indexed_version(task.id()) == Some(task.updated_at()) {
            return; // fiche déjà à jour : on évite de relire la note
        }
        let digest = self.digester.digest(task.note());
        self.index.upsert(SearchDocument::new(task, digest));
    }
}

impl TaskRepository for IndexingTaskRepository {
    fn load_all(&self) -> Result<Vec<Task>, RepositoryError> {
        let tasks = self.inner.load_all()?;
        for task in &tasks {
            self.index_task(task);
        }
        let active: Vec<TaskId> = tasks
            .iter()
            .filter(|task| !task.is_deleted())
            .map(|task| task.id().clone())
            .collect();
        self.index.retain_only(&active);
        Ok(tasks)
    }

    fn save(&self, task: &Task) -> Result<(), RepositoryError> {
        self.inner.save(task)?;
        self.index_task(task);
        Ok(())
    }
}
