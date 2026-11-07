//! Mettre en forme une liste : garder les tâches qui passent les filtres, puis les trier.

use super::list_view::{ListView, MediaChoice, SortDirection, SortField, SortKey, StatusChoice};
use crate::search::MediaKind;
use crate::{Task, TaskId, TaskTree};
use std::cmp::Ordering;

/// Progression d'une tâche : sous-tâches terminées et total (sous-tâches actives directes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Terminées.
    pub done: usize,
    /// Total.
    pub total: usize,
}

impl Progress {
    /// Progression de `task` dans `tree`.
    pub fn of(tree: &TaskTree, task: &TaskId) -> Self {
        let children = tree.children(Some(task));
        Self {
            done: children.iter().filter(|child| child.is_done()).count(),
            total: children.len(),
        }
    }

    /// Compare deux progressions en proportion (`1/2` = `2/4`) ; sans sous-tâche = 0 %.
    fn cmp_ratio(self, other: Self) -> Ordering {
        // Produits en croix : pas de division, donc pas d'arrondi.
        let left = self.done * other.total.max(1);
        let right = other.done * self.total.max(1);
        left.cmp(&right)
    }
}

/// Sous-tâches de `parent` (`None` = premier niveau) à afficher, dans l'ordre de la vue.
///
/// `media_of` donne les médias présents dans la note d'une tâche (connus de l'index de
/// recherche) : le domaine ne lit pas les notes.
pub fn arrange_children(
    tree: &TaskTree,
    parent: Option<&TaskId>,
    view: &ListView,
    media_of: &dyn Fn(&TaskId) -> Vec<MediaKind>,
) -> Vec<TaskId> {
    let mut shown: Vec<&Task> = tree
        .children(parent)
        .into_iter()
        .filter(|task| passes_filter(view, task, media_of))
        .collect();
    // `children` est déjà dans l'ordre manuel : `sort_by` est stable, l'ordre manuel départage.
    shown.sort_by(|a, b| compare(tree, &view.sort, a, b));
    shown.into_iter().map(|task| task.id().clone()).collect()
}

fn passes_filter(
    view: &ListView,
    task: &Task,
    media_of: &dyn Fn(&TaskId) -> Vec<MediaKind>,
) -> bool {
    let filter = &view.filter;
    let status_ok = match filter.status {
        StatusChoice::All => true,
        StatusChoice::Open => !task.is_done(),
        StatusChoice::Done => task.is_done(),
    };
    status_ok
        && filter
            .tags_included
            .iter()
            .all(|tag| task.tags().contains(tag))
        && !filter
            .tags_excluded
            .iter()
            .any(|tag| task.tags().contains(tag))
        && (filter.media == MediaChoice::Any || !media_of(task.id()).is_empty())
}

fn compare(tree: &TaskTree, keys: &[SortKey], a: &Task, b: &Task) -> Ordering {
    keys.iter()
        .map(|key| {
            let ordering = compare_field(tree, key.field, a, b);
            match key.direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        })
        .find(|ordering| ordering.is_ne())
        .unwrap_or(Ordering::Equal)
}

fn compare_field(tree: &TaskTree, field: SortField, a: &Task, b: &Task) -> Ordering {
    match field {
        SortField::Manual => a.order().cmp(b.order()),
        SortField::Title => {
            let fold = |task: &Task| task.title().as_str().to_lowercase();
            fold(a).cmp(&fold(b))
        }
        SortField::CreatedAt => a.created_at().cmp(&b.created_at()),
        SortField::UpdatedAt => a.updated_at().cmp(&b.updated_at()),
        // `None` (ouverte) > `Some(date)` : les tâches ouvertes viennent après les terminées.
        SortField::DoneAt => match (a.done_at(), b.done_at()) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
        SortField::Progress => Progress::of(tree, a.id()).cmp_ratio(Progress::of(tree, b.id())),
    }
}
