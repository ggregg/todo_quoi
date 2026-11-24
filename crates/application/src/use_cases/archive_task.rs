//! Archiver une ou plusieurs tâches (sélection : spec §5.5) — règle R13 (ADR-0015).

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;
use tq_domain::rules::archive::archive_subtrees;

/// Résultat d'un archivage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveOutcome {
    /// Tâches archivées (chaque tâche choisie, puis ses sous-tâches actives).
    pub archived: Vec<TaskId>,
}

/// Cas d'usage « archiver ». « Annuler » = [`super::unarchive_task`]. Le stockage range les
/// tâches archivées dans le dossier `archive/` de leur base.
pub struct ArchiveTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl ArchiveTask {
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

    /// Archive chaque tâche de `task_ids` et ses sous-tâches actives, à la même date. La
    /// confirmation (tâches ouvertes) est demandée avant par l'interface
    /// ([`super::get_archive_impact`]). Tout ou rien.
    ///
    /// # Erreurs
    /// Tâche introuvable, dans la corbeille ou déjà archivée ; stockage.
    pub fn execute(&self, task_ids: &[TaskId]) -> Result<ArchiveOutcome, AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let archived = archive_subtrees(session.tree_mut(), task_ids, &self.stamper.stamp())?;
        session.mark_changed(archived.iter().cloned());
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(ArchiveOutcome { archived })
    }
}
