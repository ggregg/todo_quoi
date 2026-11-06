//! Restaurer une tâche depuis la corbeille (ou annuler une suppression) — règle R7.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;
use tq_domain::rules::trash::{RestoreOutcome, restore_subtree};

/// Cas d'usage « restaurer ».
pub struct RestoreTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl RestoreTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        stamper: Stamper,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            tasks,
            stamper,
            events,
        }
    }

    /// Restaure `task_id` et ce qui a été supprimé avec elle.
    /// `moved_to_root` indique à l'interface d'afficher « restaurée au premier niveau ».
    ///
    /// # Erreurs
    /// Tâche introuvable ou absente de la corbeille, stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<RestoreOutcome, AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let outcome = restore_subtree(session.tree_mut(), task_id, &self.stamper.stamp())?;
        session.mark_changed(outcome.restored.iter().cloned());
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(outcome)
    }
}
