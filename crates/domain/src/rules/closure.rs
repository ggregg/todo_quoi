//! Clôture et réouverture en cascade (spécification §4.1).
//!
//! - **R1** : quand la dernière sous-tâche ouverte d'un parent est clôturée, proposer de
//!   clôturer ce parent — **un niveau à la fois** (décision du 2026-11-06) : si l'utilisateur
//!   accepte, la clôture du parent produit à son tour la proposition pour le grand-parent.
//! - **R2** : clôturer une tâche qui a encore des sous-tâches ouvertes → demander s'il faut
//!   aussi les clôturer ([`open_descendants`] donne leur liste).
//! - **R3** : rouvrir une sous-tâche dont un ancêtre est terminé → proposer de rouvrir les
//!   ancêtres terminés ([`closed_ancestors`]).
//! - **R4** (« Plus tard » ne repropose pas tant que rien ne change) relève de l'interface.
//!
//! Ces fonctions **proposent** ; c'est l'utilisateur qui décide, puis l'application exécute.

use crate::{DomainError, TaskId, TaskTree};

/// R1 — Parent à proposer à la clôture après la clôture de `closed`, s'il y en a un :
/// le parent est ouvert et **toutes** ses sous-tâches actives sont terminées.
///
/// # Erreurs
/// [`DomainError::TaskNotFound`] si `closed` n'existe pas.
pub fn parent_to_propose_closing(
    tree: &TaskTree,
    closed: &TaskId,
) -> Result<Option<TaskId>, DomainError> {
    let task = tree.get(closed)?;
    let Some(parent_id) = tree.effective_parent(task) else {
        return Ok(None);
    };
    Ok(is_ready_to_close(tree, &parent_id).then_some(parent_id))
}

/// Vrai si `id` est ouverte, hors corbeille, et a au moins une sous-tâche active, toutes
/// terminées. Sert à R1 et à R12 (une tâche qu'on retire d'un parent peut le rendre « prêt »).
pub fn is_ready_to_close(tree: &TaskTree, id: &TaskId) -> bool {
    let Ok(task) = tree.get(id) else {
        return false;
    };
    let children = tree.children(Some(id));
    !task.is_done()
        && !task.is_deleted()
        && !children.is_empty()
        && children.iter().all(|child| child.is_done())
}

/// R2 — Sous-tâches (à toute profondeur) encore ouvertes sous `id`.
pub fn open_descendants(tree: &TaskTree, id: &TaskId) -> Vec<TaskId> {
    tree.descendants(id)
        .into_iter()
        .filter(|task| !task.is_done())
        .map(|task| task.id().clone())
        .collect()
}

/// R3 — Ancêtres terminés de `id`, du plus proche au plus lointain.
///
/// # Erreurs
/// [`DomainError::TaskNotFound`] si `id` n'existe pas.
pub fn closed_ancestors(tree: &TaskTree, id: &TaskId) -> Result<Vec<TaskId>, DomainError> {
    Ok(tree
        .ancestors(id)?
        .into_iter()
        .filter(|task| task.is_done())
        .map(|task| task.id().clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::{close, id, tree};

    fn sample() -> TaskTree {
        tree(&[
            ("1", None),
            ("11", Some("1")),
            ("12", Some("1")),
            ("121", Some("12")),
            ("122", Some("12")),
        ])
    }

    #[test]
    fn r1_no_proposal_while_a_sibling_is_open() {
        let mut tree = sample();
        close(&mut tree, "121");
        assert_eq!(parent_to_propose_closing(&tree, &id("121")), Ok(None));
    }

    #[test]
    fn r1_proposes_the_parent_when_the_last_child_is_closed() {
        let mut tree = sample();
        close(&mut tree, "121");
        close(&mut tree, "122");
        assert_eq!(
            parent_to_propose_closing(&tree, &id("122")),
            Ok(Some(id("12")))
        );
    }

    #[test]
    fn r1_proposes_one_level_at_a_time() {
        let mut tree = sample();
        close(&mut tree, "11");
        close(&mut tree, "121");
        close(&mut tree, "122");
        // Le grand-parent « 1 » n'est pas proposé tant que « 12 » est ouvert…
        assert_eq!(
            parent_to_propose_closing(&tree, &id("122")),
            Ok(Some(id("12")))
        );
        // …puis il l'est quand l'utilisateur accepte de clôturer « 12 ».
        close(&mut tree, "12");
        assert_eq!(
            parent_to_propose_closing(&tree, &id("12")),
            Ok(Some(id("1")))
        );
    }

    #[test]
    fn r1_ignores_children_in_the_trash() {
        let mut tree = sample();
        close(&mut tree, "121");
        tree.get_mut(&id("122"))
            .unwrap()
            .move_to_trash(&crate::rules::test_support::stamp(3))
            .unwrap();
        assert_eq!(
            parent_to_propose_closing(&tree, &id("121")),
            Ok(Some(id("12")))
        );
    }

    #[test]
    fn r1_no_proposal_for_a_root_task() {
        let mut tree = sample();
        close(&mut tree, "1");
        assert_eq!(parent_to_propose_closing(&tree, &id("1")), Ok(None));
    }

    #[test]
    fn r2_lists_open_descendants_at_any_depth() {
        let mut tree = sample();
        close(&mut tree, "121");
        let mut open = open_descendants(&tree, &id("1"));
        open.sort();
        assert_eq!(open, vec![id("11"), id("12"), id("122")]);
    }

    #[test]
    fn r3_lists_closed_ancestors_nearest_first() {
        let mut tree = sample();
        close(&mut tree, "12");
        close(&mut tree, "1");
        assert_eq!(
            closed_ancestors(&tree, &id("121")),
            Ok(vec![id("12"), id("1")])
        );
    }
}
