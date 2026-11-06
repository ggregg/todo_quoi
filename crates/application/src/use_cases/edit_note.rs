//! Modifier la note d'une tâche (sauvegarde automatique de l'éditeur).
//!
//! L'application reçoit la note déjà convertie en Markdown : la conversion depuis l'éditeur est
//! faite à la frontière par l'adaptateur `note_markdown` (ADR-0007).

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::{NoteBody, TaskId};

/// Cas d'usage « modifier la note ».
pub struct EditNote {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl EditNote {
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

    /// Remplace la note de `task_id`. Si le texte n'a pas changé, rien n'est écrit : la
    /// sauvegarde automatique peut appeler ce cas d'usage sans risque de réécrire le fichier
    /// (et donc de le faire resynchroniser) pour rien.
    ///
    /// # Erreurs
    /// Tâche introuvable ou dans la corbeille, stockage.
    pub fn execute(&self, task_id: &TaskId, note: NoteBody) -> Result<(), AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let task = session.tree_mut().get_mut(task_id)?;
        if task.note() == &note {
            return Ok(());
        }
        task.replace_note(note, &self.stamper.stamp())?;
        session.mark_changed([task_id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())
    }
}
