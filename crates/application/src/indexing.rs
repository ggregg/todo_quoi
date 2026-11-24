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
//! - `save` : refait la fiche de la tâche (rangée comme archive si elle est archivée), ou la
//!   retire si elle part à la corbeille ;
//! - `load_archived` : refait de même les fiches des archives qui ont changé et oublie celles
//!   qui ont disparu. La recherche ne les montre qu'avec le bouton « Archives » (R14, R15) ;
//!   les icônes de médias des archives viennent de ces fiches.
//!
//! Une archive **chiffrée** est indexée sans sa note (titre et étiquettes seulement) : `archive/`
//! n'est pas relu quand un coffre se verrouille, une note déchiffrée resterait en mémoire.

use crate::ports::{NoteDigester, RepositoryError, SearchIndex, TaskRepository};
use std::sync::Arc;
use tq_domain::search::{NoteDigest, SearchDocument, Shelf};
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
        // Une tâche chiffrée est toujours refaite : sa note devient lisible ou illisible quand le
        // coffre s'ouvre ou se ferme, sans que sa version change. Scellée, sa note est vide :
        // on la trouve par son titre et ses étiquettes, jamais par sa note (index en mémoire).
        let encrypted = task.confidentiality().is_encrypted();
        if self.index.is_up_to_date(task) && !encrypted {
            return; // fiche déjà à jour : on évite de relire la note
        }
        let digest = if encrypted && task.is_archived() {
            NoteDigest::default() // voir l'en-tête du module
        } else {
            self.digester.digest(task.note())
        };
        self.index.upsert(SearchDocument::new(task, digest));
    }

    /// Indexe `tasks`, toutes lues dans le rayon `shelf`, et oublie les fiches de ce rayon qui
    /// n'y sont plus.
    fn index_shelf(&self, shelf: Shelf, tasks: &[Task]) {
        for task in tasks {
            self.index_task(task);
        }
        let present: Vec<TaskId> = tasks
            .iter()
            .filter(|task| !task.is_deleted())
            .map(|task| task.id().clone())
            .collect();
        self.index.retain_only(shelf, &present);
    }
}

impl TaskRepository for IndexingTaskRepository {
    fn load_all(&self) -> Result<Vec<Task>, RepositoryError> {
        let tasks = self.inner.load_all()?;
        self.index_shelf(Shelf::Active, &tasks);
        Ok(tasks)
    }

    fn load_archived(&self) -> Result<Vec<Task>, RepositoryError> {
        let archives = self.inner.load_archived()?;
        self.index_shelf(Shelf::Archived, &archives);
        Ok(archives)
    }

    fn save(&self, task: &Task) -> Result<(), RepositoryError> {
        self.inner.save(task)?;
        self.index_task(task);
        Ok(())
    }

    fn remove(&self, id: &TaskId) -> Result<(), RepositoryError> {
        self.inner.remove(id)?;
        self.index.remove(id);
        Ok(())
    }
}
