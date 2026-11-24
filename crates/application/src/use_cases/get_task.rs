//! Lire une tâche complète, note comprise (panneau de détail, fenêtre de tâche).

use crate::AppError;
use crate::ports::TaskRepository;
use crate::session::tree_containing;
use std::sync::Arc;
use tq_domain::{Task, TaskId};

/// Cas d'usage « lire une tâche » (lecture seule).
pub struct GetTask {
    tasks: Arc<dyn TaskRepository>,
}

impl GetTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(tasks: Arc<dyn TaskRepository>) -> Self {
        Self { tasks }
    }

    /// La tâche `task_id`, y compris si elle est dans la corbeille ou archivée (lecture seule,
    /// R15).
    ///
    /// # Erreurs
    /// Tâche introuvable, stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<Task, AppError> {
        let tree = tree_containing(self.tasks.as_ref(), task_id)?;
        Ok(tree.get(task_id)?.clone())
    }
}
