//! Lister la corbeille — règle R6.

use crate::AppError;
use crate::ports::TaskRepository;
use std::sync::Arc;
use tq_domain::rules::trash::trash_roots;
use tq_domain::{Task, TaskTree};

/// Cas d'usage « lister la corbeille » (lecture seule).
pub struct ListTrash {
    tasks: Arc<dyn TaskRepository>,
}

impl ListTrash {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(tasks: Arc<dyn TaskRepository>) -> Self {
        Self { tasks }
    }

    /// Tâches à afficher dans la corbeille (les plus récentes d'abord).
    ///
    /// # Erreurs
    /// Stockage.
    pub fn execute(&self) -> Result<Vec<Task>, AppError> {
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        Ok(trash_roots(&tree).into_iter().cloned().collect())
    }
}
