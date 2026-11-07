//! Héritage des vues (spec §5.3) : une liste sans vue propre utilise celle de l'ancêtre le plus
//! proche qui en a une, sinon la vue racine.

use super::list_view::ListView;
use crate::{TaskId, TaskTree};

/// D'où vient la vue appliquée à une liste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewSource {
    /// La liste a sa propre vue (« personnalisée »).
    Own,
    /// Héritée de cet ancêtre.
    InheritedFrom(TaskId),
    /// Héritée de la vue racine (celle de l'espace, dans `manifest.json`).
    Root,
}

/// Vue effective de la liste des sous-tâches de `list` (`None` = liste racine) et son origine.
///
/// La liste racine utilise toujours `root_view` (sa source est [`ViewSource::Own`]). Une tâche
/// introuvable se comporte comme une tâche sans vue (héritage de la racine).
pub fn effective_view(
    tree: &TaskTree,
    list: Option<&TaskId>,
    root_view: &ListView,
) -> (ListView, ViewSource) {
    let Some(list) = list else {
        return (root_view.clone(), ViewSource::Own);
    };
    let Ok(task) = tree.get(list) else {
        return (root_view.clone(), ViewSource::Root);
    };
    if let Some(view) = task.view() {
        return (view.clone(), ViewSource::Own);
    }
    // `ancestors` va du parent vers la racine : le premier qui a une vue est le plus proche.
    let ancestors = tree.ancestors(list).unwrap_or_default();
    match ancestors
        .into_iter()
        .find_map(|ancestor| Some((ancestor, ancestor.view()?)))
    {
        Some((ancestor, view)) => (
            view.clone(),
            ViewSource::InheritedFrom(ancestor.id().clone()),
        ),
        None => (root_view.clone(), ViewSource::Root),
    }
}
