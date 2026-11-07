//! Lister les tâches actives (hors corbeille) et la mise en forme de chaque liste, pour afficher
//! l'arbre — spec §5.2 et §5.3.

use crate::AppError;
use crate::ports::{RootViewStore, SearchIndex, TaskRepository};
use std::collections::HashMap;
use std::sync::Arc;
use tq_domain::search::MediaKind;
use tq_domain::view::{ListView, ViewSource, arrange_children, effective_view};
use tq_domain::{Task, TaskId, TaskTree};

/// Une liste de l'arbre : la liste racine, ou les sous-tâches d'une tâche.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ListTarget {
    /// Premier niveau.
    Root,
    /// Sous-tâches de cette tâche.
    Task(TaskId),
}

impl ListTarget {
    /// La tâche dont c'est la liste (`None` pour la racine).
    pub fn task(&self) -> Option<&TaskId> {
        match self {
            ListTarget::Root => None,
            ListTarget::Task(id) => Some(id),
        }
    }
}

/// Mise en forme d'une liste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListLayout {
    /// La liste.
    pub list: ListTarget,
    /// Vue appliquée (propre ou héritée).
    pub view: ListView,
    /// Origine de la vue.
    pub source: ViewSource,
    /// Sous-tâches à afficher, filtrées et triées selon la vue.
    pub children: Vec<TaskId>,
}

/// Résultat de [`ListTasks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListing {
    /// Tâches actives, dans l'ordre manuel. Parent introuvable → `parent = None` à l'affichage.
    pub tasks: Vec<Task>,
    /// Une mise en forme par liste : la racine, puis chaque tâche active.
    pub lists: Vec<ListLayout>,
    /// Médias présents dans la note de chaque tâche qui en a (icônes de la liste).
    pub media: HashMap<TaskId, Vec<MediaKind>>,
}

/// Cas d'usage « lister les tâches » (lecture seule).
pub struct ListTasks {
    tasks: Arc<dyn TaskRepository>,
    root_view: Arc<dyn RootViewStore>,
    index: Arc<dyn SearchIndex>,
}

impl ListTasks {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        root_view: Arc<dyn RootViewStore>,
        index: Arc<dyn SearchIndex>,
    ) -> Self {
        Self {
            tasks,
            root_view,
            index,
        }
    }

    /// Tâches actives et mise en forme de chaque liste.
    ///
    /// # Erreurs
    /// Stockage, ou deux fichiers portant le même identifiant.
    pub fn execute(&self) -> Result<TaskListing, AppError> {
        // Charger d'abord : c'est ce qui met l'index (donc les médias) à jour.
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        let root_view = self.root_view.load_root_view()?.unwrap_or_default();
        let mut tasks: Vec<Task> = tree
            .iter()
            .filter(|task| !task.is_deleted())
            .cloned()
            .collect();
        tasks.sort_by(|a, b| a.order().cmp(b.order()).then_with(|| a.id().cmp(b.id())));

        let media: HashMap<TaskId, Vec<MediaKind>> = tasks
            .iter()
            .map(|task| (task.id().clone(), self.index.media_of(task.id())))
            .filter(|(_, kinds)| !kinds.is_empty())
            .collect();
        let media_of = |id: &TaskId| media.get(id).cloned().unwrap_or_default();
        let targets = std::iter::once(ListTarget::Root)
            .chain(tasks.iter().map(|task| ListTarget::Task(task.id().clone())));
        let lists = targets
            .map(|list| layout(&tree, list, &root_view, &media_of))
            .collect();
        Ok(TaskListing {
            tasks,
            lists,
            media,
        })
    }
}

fn layout(
    tree: &TaskTree,
    list: ListTarget,
    root_view: &ListView,
    media_of: &dyn Fn(&TaskId) -> Vec<MediaKind>,
) -> ListLayout {
    let (view, source) = effective_view(tree, list.task(), root_view);
    let children = arrange_children(tree, list.task(), &view, media_of);
    ListLayout {
        list,
        view,
        source,
        children,
    }
}
