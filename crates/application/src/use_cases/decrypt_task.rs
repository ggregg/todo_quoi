//! Remettre une tâche en clair (bouton 🔓) — spec §9. Ses sous-tâches restent chiffrées ; les
//! médias de sa note repassent en clair.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use crate::vault::SealedMedia;
use std::sync::Arc;
use tq_domain::TaskId;

/// Cas d'usage « déchiffrer ».
pub struct DecryptTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
    media: Arc<SealedMedia>,
}

impl DecryptTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        stamper: Stamper,
        events: Arc<dyn EventPublisher>,
        media: Arc<SealedMedia>,
    ) -> Self {
        Self {
            tasks,
            stamper,
            events,
            media,
        }
    }

    /// Remet la tâche en clair.
    ///
    /// # Erreurs
    /// Tâche introuvable, pas chiffrée, scellée (coffre verrouillé) ou dans la corbeille ;
    /// stockage des médias.
    pub fn execute(&self, id: &TaskId) -> Result<(), AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let stamp = self.stamper.stamp();
        session.tree_mut().get_mut(id)?.decrypt(&stamp)?;
        let ids = [id.clone()];
        let replaced = self.media.rename_in_tasks(
            session.tree_mut(),
            &ids,
            &stamp,
            SealedMedia::unseal_note,
        )?;
        let tree = session.tree().clone();
        session.mark_changed(ids);
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        self.media.erase_uncited(&tree, &replaced)
    }
}
