//! Avant d'archiver : combien de tâches partiraient, dont combien encore ouvertes ? (R13)
//!
//! L'interface appelle ce cas d'usage pour savoir s'il faut demander
//! « Archiver aussi les N tâches ouvertes ? » avant d'appeler [`super::archive_task`]
//! (même séparation question / action que [`super::get_close_impact`]).

use crate::AppError;
use crate::ports::TaskRepository;
use std::sync::Arc;
use tq_domain::rules::archive::{ArchiveImpact, archive_impact};
use tq_domain::{TaskId, TaskTree};

/// Cas d'usage « mesurer l'impact d'un archivage » (lecture seule).
pub struct GetArchiveImpact {
    tasks: Arc<dyn TaskRepository>,
}

impl GetArchiveImpact {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(tasks: Arc<dyn TaskRepository>) -> Self {
        Self { tasks }
    }

    /// Calcule l'impact de l'archivage de `task_id`.
    ///
    /// # Erreurs
    /// Tâche introuvable, dans la corbeille ou déjà archivée ; stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<ArchiveImpact, AppError> {
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        Ok(archive_impact(&tree, task_id)?)
    }
}
