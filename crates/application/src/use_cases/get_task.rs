//! Lire une tâche complète, note comprise (panneau de détail, fenêtre de tâche).

use crate::AppError;
use crate::ports::TaskRepository;
use std::sync::Arc;
use tq_domain::{Task, TaskId, TaskTree};

/// Cas d'usage « lire une tâche » (lecture seule).
pub struct GetTask {
    tasks: Arc<dyn TaskRepository>,
}

impl GetTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(tasks: Arc<dyn TaskRepository>) -> Self {
        Self { tasks }
    }

    /// La tâche `task_id`, y compris si elle est dans la corbeille.
    ///
    /// # Erreurs
    /// Tâche introuvable, stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<Task, AppError> {
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        Ok(tree.get(task_id)?.clone())
    }
}
