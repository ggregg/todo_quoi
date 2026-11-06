//! Renommer une tâche (`F2`, ou saisie dans le panneau de détail).

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::{TaskId, Title};

/// Cas d'usage « renommer ».
pub struct RenameTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl RenameTask {
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

    /// Remplace le titre de `task_id`.
    ///
    /// # Erreurs
    /// Titre invalide, tâche introuvable ou dans la corbeille, stockage.
    pub fn execute(&self, task_id: &TaskId, title: &str) -> Result<(), AppError> {
        let title = Title::parse(title)?;
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        session
            .tree_mut()
            .get_mut(task_id)?
            .rename(title, &self.stamper.stamp())?;
        session.mark_changed([task_id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())
    }
}
