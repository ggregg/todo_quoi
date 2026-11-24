//! Déroulé commun à tous les cas d'usage qui modifient des tâches :
//! **charger** l'arbre → **modifier** (règles du domaine) → **enregistrer** ce qui a changé
//! → **prévenir** l'interface.

use crate::AppError;
use crate::ports::{AppEvent, EventPublisher, TaskRepository};
use tq_domain::rules::archive::with_archives;
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

    /// Charge toutes les tâches, **archives comprises** : pour restaurer une archive, ou pour
    /// effacer des médias sans effacer ceux qu'une archive cite encore.
    pub(crate) fn load_with_archives(repository: &dyn TaskRepository) -> Result<Self, AppError> {
        Ok(Self {
            tree: tree_with_archives(repository)?,
            changed: Vec::new(),
        })
    }

    /// Charge les tâches actives ; si l'une de `ids` n'y est pas (c'est peut-être une archive,
    /// R17), les archives aussi.
    pub(crate) fn load_containing(
        repository: &dyn TaskRepository,
        ids: &[TaskId],
    ) -> Result<Self, AppError> {
        Ok(Self {
            tree: tree_containing(repository, ids)?,
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

/// L'arbre complet : tâches actives, corbeille et archives. Une tâche présente des deux côtés
/// (doublon laissé par la synchronisation, R18) n'y figure qu'une fois, en version la plus récente.
///
/// # Erreurs
/// Stockage.
pub(crate) fn tree_with_archives(repository: &dyn TaskRepository) -> Result<TaskTree, AppError> {
    // Les tâches actives d'abord : le stockage règle à cette lecture les doublons R18.
    let active = repository.load_all()?;
    let archived = repository.load_archived()?;
    Ok(TaskTree::new(with_archives(active, archived))?)
}

/// Les tâches actives et la corbeille ; si l'une de `ids` n'y est pas, l'arbre complet avec les archives
/// (les archives ne sont lues que pour une tâche qui pourrait en être une).
///
/// # Erreurs
/// Stockage.
pub(crate) fn tree_containing(
    repository: &dyn TaskRepository,
    ids: &[TaskId],
) -> Result<TaskTree, AppError> {
    let tree = TaskTree::new(repository.load_all()?)?;
    if ids.iter().all(|id| tree.get(id).is_ok()) {
        return Ok(tree);
    }
    tree_with_archives(repository)
}
