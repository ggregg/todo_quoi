//! Remplacer les étiquettes d'une tâche.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::{Tag, TaskId};

/// Cas d'usage « remplacer les étiquettes ».
pub struct SetTags {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl SetTags {
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

    /// Remplace les étiquettes de `task_id` par `tags` (saisie libre, normalisée).
    ///
    /// # Erreurs
    /// Étiquette invalide (aucune n'est alors appliquée), tâche introuvable, stockage.
    pub fn execute(&self, task_id: &TaskId, tags: &[String]) -> Result<(), AppError> {
        // [rust] `collect::<Result<Vec<_>, _>>()` : s'arrête à la première erreur et la renvoie,
        //        sinon fournit le Vec complet (≈ `Promise.all` qui échoue au premier rejet).
        let tags = tags
            .iter()
            .map(|tag| Tag::parse(tag))
            .collect::<Result<Vec<_>, _>>()?;
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        session
            .tree_mut()
            .get_mut(task_id)?
            .set_tags(tags, &self.stamper.stamp())?;
        session.mark_changed([task_id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())
    }
}
