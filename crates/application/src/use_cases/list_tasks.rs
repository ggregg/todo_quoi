//! Lister les tâches actives (hors corbeille) et la mise en forme de chaque liste, pour afficher
//! l'arbre — spec §5.2 et §5.3.

use crate::AppError;
use crate::ports::{BaseDirectory, RootViewStore, SearchIndex, TaskRepository};
use std::collections::HashMap;
use std::sync::Arc;
use tq_domain::search::MediaKind;
use tq_domain::view::{ListView, ViewSource, arrange_children, effective_view};
use tq_domain::{BaseId, Task, TaskId, TaskTree};

/// Une liste de l'arbre : le premier niveau d'une base, ou les sous-tâches d'une tâche.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ListTarget {
    /// Premier niveau d'une base (ADR-0009 ; une seule base : la base principale).
    Base(BaseId),
    /// Sous-tâches de cette tâche.
    Task(TaskId),
}

impl ListTarget {
    /// La tâche dont c'est la liste (`None` pour le premier niveau d'une base).
    pub fn task(&self) -> Option<&TaskId> {
        match self {
            ListTarget::Base(_) => None,
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
    /// Bases ouvertes, dans l'ordre du bandeau.
    pub bases: Vec<BaseId>,
    /// Une mise en forme par liste : le premier niveau de chaque base, puis chaque tâche active.
    pub lists: Vec<ListLayout>,
    /// Médias présents dans la note de chaque tâche qui en a (icônes de la liste).
    pub media: HashMap<TaskId, Vec<MediaKind>>,
}

/// Cas d'usage « lister les tâches » (lecture seule).
pub struct ListTasks {
    tasks: Arc<dyn TaskRepository>,
    root_view: Arc<dyn RootViewStore>,
    index: Arc<dyn SearchIndex>,
    bases: Arc<dyn BaseDirectory>,
}

impl ListTasks {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        root_view: Arc<dyn RootViewStore>,
        index: Arc<dyn SearchIndex>,
        bases: Arc<dyn BaseDirectory>,
    ) -> Self {
        Self {
            tasks,
            root_view,
            index,
            bases,
        }
    }

    /// Vue racine de chaque base (vue d'origine si aucune n'est enregistrée).
    fn root_views(&self, bases: &[BaseId]) -> Result<HashMap<BaseId, ListView>, AppError> {
        let mut views = HashMap::new();
        for base in bases {
            views.insert(
                base.clone(),
                self.root_view.load_root_view(base)?.unwrap_or_default(),
            );
        }
        Ok(views)
    }

    /// Tâches actives et mise en forme de chaque liste.
    ///
    /// # Erreurs
    /// Stockage, ou deux fichiers portant le même identifiant.
    pub fn execute(&self) -> Result<TaskListing, AppError> {
        // Charger d'abord : c'est ce qui met l'index (donc les médias) à jour.
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        let bases = self.bases.bases();
        let root_views = self.root_views(&bases)?;
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
        let targets = bases
            .iter()
            .map(|base| (ListTarget::Base(base.clone()), base.clone()))
            .chain(
                tasks
                    .iter()
                    .map(|task| (ListTarget::Task(task.id().clone()), task.base().clone())),
            );
        let lists = targets
            .map(|(list, base)| {
                let root_view = root_views.get(&base).cloned().unwrap_or_default();
                layout(&tree, list, &root_view, &media_of)
            })
            .collect();
        Ok(TaskListing {
            tasks,
            bases,
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
    let mut children = arrange_children(tree, list.task(), &view, media_of);
    // Premier niveau d'une base : seulement les tâches de cette base.
    if let ListTarget::Base(base) = &list {
        children.retain(|id| tree.get(id).is_ok_and(|task| task.base() == base));
    }
    ListLayout {
        list,
        view,
        source,
        children,
    }
}
