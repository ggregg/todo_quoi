//! Supprimer **définitivement** une tâche de la corbeille (« Supprimer définitivement »), sans
//! attendre les 30 jours de la purge automatique (R8). Irréversible : l'interface confirme avant.

use crate::AppError;
use crate::ports::{AppEvent, EventPublisher, TaskRepository};
use std::sync::Arc;
use tq_domain::rules::trash::purgeable_subtree;
use tq_domain::{TaskId, TaskTree};

/// Cas d'usage « supprimer définitivement ».
pub struct PurgeTask {
    tasks: Arc<dyn TaskRepository>,
    events: Arc<dyn EventPublisher>,
}

impl PurgeTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(tasks: Arc<dyn TaskRepository>, events: Arc<dyn EventPublisher>) -> Self {
        Self { tasks, events }
    }

    /// Efface la tâche et ses sous-tâches encore à la corbeille ; renvoie les tâches effacées.
    ///
    /// # Erreurs
    /// Tâche introuvable, ou pas à la corbeille (une tâche active ne se purge jamais) ; stockage.
    pub fn execute(&self, id: &TaskId) -> Result<Vec<TaskId>, AppError> {
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        let purged = purgeable_subtree(&tree, id)?;
        for purged_id in &purged {
            self.tasks.remove(purged_id)?;
        }
        self.events.publish(AppEvent::TasksChanged {
            ids: purged.clone(),
        });
        Ok(purged)
    }
}
