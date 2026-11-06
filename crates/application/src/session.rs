//! Déroulé commun à tous les cas d'usage qui modifient des tâches :
//! **charger** l'arbre → **modifier** (règles du domaine) → **enregistrer** ce qui a changé
//! → **prévenir** l'interface.

use crate::AppError;
use crate::ports::{AppEvent, EventPublisher, TaskRepository};
use tq_domain::{TaskId, TaskTree};

/// Une « séance de travail » sur l'arbre des tâches, qui retient ce qui a été modifié.
pub(crate) struct TreeSession {
    tree: TaskTree,
    changed: Vec<TaskId>,
}

impl TreeSession {
    /// Charge toutes les tâches.
    pub(crate) fn load(repository: &dyn TaskRepository) -> Result<Self, AppError> {
        Ok(Self {
            tree: TaskTree::new(repository.load_all()?)?,
            changed: Vec::new(),
        })
    }

    /// L'arbre, en lecture.
    pub(crate) fn tree(&self) -> &TaskTree {
        &self.tree
    }

    /// L'arbre, en modification. Penser à appeler [`Self::mark_changed`] ensuite.
    pub(crate) fn tree_mut(&mut self) -> &mut TaskTree {
        &mut self.tree
    }

    /// Note des tâches à enregistrer.
    // [rust] `impl IntoIterator<Item = TaskId>` : accepte un `Vec`, un tableau, un itérateur…
    //        (≈ un paramètre `Iterable<TaskId>` en TS).
    pub(crate) fn mark_changed(&mut self, ids: impl IntoIterator<Item = TaskId>) {
        self.changed.extend(ids);
    }

    /// Enregistre les tâches modifiées (une seule fois chacune) et publie l'événement.
    pub(crate) fn commit(
        mut self,
        repository: &dyn TaskRepository,
        events: &dyn EventPublisher,
    ) -> Result<(), AppError> {
        self.changed.sort();
        self.changed.dedup();
        for id in &self.changed {
            repository.save(self.tree.get(id)?)?;
        }
        if !self.changed.is_empty() {
            events.publish(AppEvent::TasksChanged { ids: self.changed });
        }
        Ok(())
    }
}
