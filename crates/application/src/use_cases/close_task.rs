//! Clôturer une tâche (`Espace` sur une tâche ouverte) — règles R1 et R2.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;
use tq_domain::rules::closure::{open_descendants, parent_to_propose_closing};

/// Que faire des sous-tâches encore ouvertes (R2) ? Réponse de l'utilisateur.
// Une énumération plutôt qu'un booléen : `CloseScope::WithOpenDescendants` se lit tout seul,
// `close(id, true)` non (Clean Code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseScope {
    /// Clôturer seulement la tâche.
    TaskOnly,
    /// Clôturer aussi toutes ses sous-tâches ouvertes.
    WithOpenDescendants,
}

/// Commande de clôture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseTaskCommand {
    /// Tâche à clôturer.
    pub task_id: TaskId,
    /// Sort des sous-tâches ouvertes.
    pub scope: CloseScope,
}

/// Résultat d'une clôture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseOutcome {
    /// Tâches effectivement clôturées (la tâche demandée en premier).
    pub closed: Vec<TaskId>,
    /// R1 : parent à proposer à la clôture (toutes ses sous-tâches sont désormais terminées).
    pub propose_closing_parent: Option<TaskId>,
}

/// Cas d'usage « clôturer ».
pub struct CloseTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl CloseTask {
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

    /// Clôture la tâche (et ses sous-tâches ouvertes si demandé), puis calcule la proposition R1.
    ///
    /// # Erreurs
    /// Tâche introuvable, déjà terminée ou dans la corbeille, stockage.
    pub fn execute(&self, command: CloseTaskCommand) -> Result<CloseOutcome, AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let stamp = self.stamper.stamp();

        let mut closed = vec![command.task_id.clone()];
        if command.scope == CloseScope::WithOpenDescendants {
            closed.extend(open_descendants(session.tree(), &command.task_id));
        }
        for id in &closed {
            session.tree_mut().get_mut(id)?.close(&stamp)?;
        }
        let propose_closing_parent = parent_to_propose_closing(session.tree(), &command.task_id)?;

        session.mark_changed(closed.iter().cloned());
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(CloseOutcome {
            closed,
            propose_closing_parent,
        })
    }
}
