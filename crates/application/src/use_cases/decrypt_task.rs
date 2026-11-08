//! Remettre une tâche en clair (bouton 🔓) — spec §9. Ses sous-tâches restent chiffrées.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;

/// Cas d'usage « déchiffrer ».
pub struct DecryptTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl DecryptTask {
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

    /// Remet la tâche en clair.
    ///
    /// # Erreurs
    /// Tâche introuvable, pas chiffrée, scellée (coffre verrouillé) ou dans la corbeille.
    pub fn execute(&self, id: &TaskId) -> Result<(), AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        session
            .tree_mut()
            .get_mut(id)?
            .decrypt(&self.stamper.stamp())?;
        session.mark_changed([id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())
    }
}
