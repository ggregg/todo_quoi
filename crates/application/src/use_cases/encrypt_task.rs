//! Chiffrer une tâche (bouton 🔒), et par défaut ses sous-tâches — spec §9.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use crate::vault::{OpenVaults, VaultError};
use std::sync::Arc;
use tq_domain::TaskId;

/// Chiffrer aussi les sous-tâches ? (Proposé par défaut dans l'interface.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptScope {
    /// Seulement la tâche.
    TaskOnly,
    /// La tâche et toutes ses sous-tâches (déjà chiffrées : laissées telles quelles).
    WithSubtasks,
}

/// Cas d'usage « chiffrer ».
pub struct EncryptTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
    vaults: Arc<OpenVaults>,
}

impl EncryptTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        stamper: Stamper,
        events: Arc<dyn EventPublisher>,
        vaults: Arc<OpenVaults>,
    ) -> Self {
        Self {
            tasks,
            stamper,
            events,
            vaults,
        }
    }

    /// Chiffre la tâche (et ses sous-tâches si demandé) ; renvoie les tâches chiffrées.
    ///
    /// # Erreurs
    /// [`VaultError::Locked`] si le coffre de sa base n'est pas ouvert (l'interface propose alors
    /// de le créer ou de le déverrouiller) ; tâche introuvable, déjà chiffrée, dans la corbeille.
    pub fn execute(&self, id: &TaskId, scope: EncryptScope) -> Result<Vec<TaskId>, AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let base = session.tree().get(id)?.base().clone();
        if !self.vaults.is_open(&base) {
            return Err(VaultError::Locked.into());
        }
        let stamp = self.stamper.stamp();
        session.tree_mut().get_mut(id)?.encrypt(&stamp)?;
        let mut encrypted = vec![id.clone()];
        if scope == EncryptScope::WithSubtasks {
            encrypted.extend(Self::plain_descendants(&session, id));
        }
        for descendant in &encrypted[1..] {
            session.tree_mut().get_mut(descendant)?.encrypt(&stamp)?;
        }
        session.mark_changed(encrypted.iter().cloned());
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(encrypted)
    }

    /// Sous-tâches actives encore en clair.
    fn plain_descendants(session: &TreeSession, id: &TaskId) -> Vec<TaskId> {
        session
            .tree()
            .descendants(id)
            .into_iter()
            .filter(|task| !task.is_deleted() && !task.confidentiality().is_encrypted())
            .map(|task| task.id().clone())
            .collect()
    }
}
