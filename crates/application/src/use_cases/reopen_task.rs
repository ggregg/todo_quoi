//! Rouvrir une tâche (`Espace` sur une tâche terminée) — règle R3.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;
use tq_domain::rules::closure::closed_ancestors;

/// Résultat d'une réouverture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReopenOutcome {
    /// R3 : ancêtres terminés à proposer à la réouverture, du plus proche au plus lointain.
    /// Si l'utilisateur accepte, l'interface appelle ce même cas d'usage pour chacun.
    pub propose_reopening: Vec<TaskId>,
}

/// Cas d'usage « rouvrir ».
pub struct ReopenTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl ReopenTask {
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

    /// Rouvre `task_id` et calcule la proposition R3.
    ///
    /// # Erreurs
    /// Tâche introuvable, déjà ouverte ou dans la corbeille, stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<ReopenOutcome, AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        session
            .tree_mut()
            .get_mut(task_id)?
            .reopen(&self.stamper.stamp())?;
        let propose_reopening = closed_ancestors(session.tree(), task_id)?;
        session.mark_changed([task_id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(ReopenOutcome { propose_reopening })
    }
}
