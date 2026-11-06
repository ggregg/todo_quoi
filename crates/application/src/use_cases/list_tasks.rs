//! Lister les tâches actives (hors corbeille), pour afficher l'arbre.

use crate::AppError;
use crate::ports::TaskRepository;
use std::sync::Arc;
use tq_domain::Task;

/// Cas d'usage « lister les tâches » (lecture seule).
///
/// Renvoie une liste « à plat » : l'interface reconstitue l'arbre grâce à `parent` et `order`.
/// Les tâches dont le parent est introuvable sont rendues avec `parent = None` (premier niveau).
pub struct ListTasks {
    tasks: Arc<dyn TaskRepository>,
}

impl ListTasks {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(tasks: Arc<dyn TaskRepository>) -> Self {
        Self { tasks }
    }

    /// Tâches actives.
    ///
    /// # Erreurs
    /// Stockage, ou deux fichiers portant le même identifiant.
    pub fn execute(&self) -> Result<Vec<Task>, AppError> {
        let tasks = self.tasks.load_all()?;
        let tree = tq_domain::TaskTree::new(tasks)?;
        let mut active: Vec<Task> = tree
            .iter()
            .filter(|task| !task.is_deleted())
            .cloned()
            .collect();
        active.sort_by(|a, b| a.order().cmp(b.order()).then_with(|| a.id().cmp(b.id())));
        Ok(active)
    }
}
