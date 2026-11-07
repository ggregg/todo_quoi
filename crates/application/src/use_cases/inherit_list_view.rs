//! « Revenir à l'héritage » : retirer la vue propre d'une liste (spec §5.3).

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;

/// Cas d'usage « revenir à l'héritage ». La liste racine n'hérite de rien : elle n'est pas
/// concernée (on la remet à zéro en la personnalisant avec la vue d'origine).
pub struct InheritListView {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl InheritListView {
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

    /// La liste des sous-tâches de `task_id` reprend la vue de son ancêtre (ou la vue racine).
    ///
    /// # Erreurs
    /// Tâche introuvable ou dans la corbeille, stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<(), AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        session
            .tree_mut()
            .get_mut(task_id)?
            .inherit_view(&self.stamper.stamp())?;
        session.mark_changed([task_id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())
    }
}
