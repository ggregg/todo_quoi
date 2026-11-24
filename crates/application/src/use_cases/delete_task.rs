//! Mettre une ou plusieurs tâches à la corbeille (`Suppr`, sélection : spec §5.5) — règle R5 ;
//! une archive aussi (R17).

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;
use tq_domain::rules::trash::move_subtrees_to_trash;

/// Résultat d'une suppression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteOutcome {
    /// Tâches mises à la corbeille (chaque tâche choisie et ses sous-tâches).
    pub deleted: Vec<TaskId>,
}

/// Cas d'usage « mettre à la corbeille ». « Annuler » = [`super::restore_task`].
pub struct DeleteTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl DeleteTask {
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

    /// Met chaque tâche de `task_ids` et ses sous-tâches à la corbeille, à la même date. Une
    /// archive y part avec ses sous-tâches archivées et quitte les archives (R17). Tout ou rien :
    /// si une tâche ne peut pas y aller, rien n'est enregistré.
    ///
    /// # Erreurs
    /// Tâche introuvable ou déjà dans la corbeille, stockage.
    pub fn execute(&self, task_ids: &[TaskId]) -> Result<DeleteOutcome, AppError> {
        let mut session = TreeSession::load_containing(self.tasks.as_ref(), task_ids)?;
        let deleted = move_subtrees_to_trash(session.tree_mut(), task_ids, &self.stamper.stamp())?;
        session.mark_changed(deleted.iter().cloned());
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(DeleteOutcome { deleted })
    }
}
