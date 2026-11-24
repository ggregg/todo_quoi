//! Restaurer une tâche archivée (ou annuler un archivage) — règle R16 (ADR-0015).

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;
use tq_domain::rules::archive::restore_archived_subtree;
use tq_domain::rules::trash::RestoreOutcome;

/// Cas d'usage « restaurer une archive ». Le stockage remet les tâches restaurées dans le
/// dossier `tasks/` de leur base.
pub struct UnarchiveTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl UnarchiveTask {
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

    /// Restaure `task_id` et ce qui a été archivé avec elle. `moved_to_root` indique à
    /// l'interface d'afficher « restaurée au premier niveau ».
    ///
    /// # Erreurs
    /// Tâche introuvable ou pas archivée ; stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<RestoreOutcome, AppError> {
        // Archives comprises : la tâche y est, et son parent peut-être aussi (R16).
        let mut session = TreeSession::load_with_archives(self.tasks.as_ref())?;
        let outcome = restore_archived_subtree(session.tree_mut(), task_id, &self.stamper.stamp())?;
        session.mark_changed(outcome.restored.iter().cloned());
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(outcome)
    }
}
