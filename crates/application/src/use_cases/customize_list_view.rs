//! « Personnaliser cette liste » : donner une vue propre à une liste (spec §5.3).

use super::list_tasks::ListTarget;
use crate::AppError;
use crate::Stamper;
use crate::ports::{AppEvent, EventPublisher, RootViewStore, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::view::ListView;

/// Cas d'usage « personnaliser une liste ».
pub struct CustomizeListView {
    tasks: Arc<dyn TaskRepository>,
    root_view: Arc<dyn RootViewStore>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl CustomizeListView {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        root_view: Arc<dyn RootViewStore>,
        stamper: Stamper,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            tasks,
            root_view,
            stamper,
            events,
        }
    }

    /// Enregistre `view` pour `list` : dans `manifest.json` pour la racine, dans le fichier de
    /// la tâche sinon. Les listes qui en héritent changent aussi.
    ///
    /// # Erreurs
    /// Tâche introuvable ou dans la corbeille, stockage.
    pub fn execute(&self, list: &ListTarget, view: ListView) -> Result<(), AppError> {
        let task_id = match list {
            ListTarget::Task(task_id) => task_id,
            ListTarget::Base(base) => {
                self.root_view.save_root_view(base, &view.normalized())?;
                self.events
                    .publish(AppEvent::TasksChanged { ids: Vec::new() });
                return Ok(());
            }
        };
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        session
            .tree_mut()
            .get_mut(task_id)?
            .customize_view(view, &self.stamper.stamp())?;
        session.mark_changed([task_id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())
    }
}
