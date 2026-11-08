//! Corbeille (spécification §4.2).
//!
//! - **R5** : supprimer = mettre la tâche **et ses sous-tâches actives** à la corbeille.
//! - **R6** : la corbeille liste les « racines » supprimées → [`trash_roots`].
//! - **R7** : restaurer remet le sous-arbre à sa place ; si le parent d'origine n'existe plus
//!   ou est lui-même dans la corbeille, la tâche revient au premier niveau.
//! - **R8** (purge définitive) touche aux fichiers : c'est l'affaire de l'application (I5).

use crate::{ChangeStamp, DomainError, Task, TaskId, TaskTree, Timestamp};

/// R5 — Met `id` et ses sous-tâches actives à la corbeille. Renvoie les tâches modifiées
/// (utile pour « Annuler »).
///
/// # Erreurs
/// [`DomainError::TaskNotFound`] ou [`DomainError::TaskInTrash`] (déjà dans la corbeille).
pub fn move_subtree_to_trash(
    tree: &mut TaskTree,
    id: &TaskId,
    stamp: &ChangeStamp,
) -> Result<Vec<TaskId>, DomainError> {
    if tree.get(id)?.is_deleted() {
        return Err(DomainError::TaskInTrash(id.clone()));
    }
    let mut ids = vec![id.clone()];
    ids.extend(
        tree.descendants(id)
            .into_iter()
            .map(|task| task.id().clone()),
    );
    for task_id in &ids {
        tree.get_mut(task_id)?.move_to_trash(stamp)?;
    }
    Ok(ids)
}

/// Résultat d'une restauration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreOutcome {
    /// Tâches sorties de la corbeille (la tâche demandée en premier).
    pub restored: Vec<TaskId>,
    /// Vrai si la tâche a été rattachée au premier niveau faute de parent (R7).
    pub moved_to_root: bool,
}

/// R7 — Restaure `id` et les sous-tâches supprimées **en même temps** qu'elle (même date de
/// mise à la corbeille) : une sous-tâche supprimée plus tôt, séparément, y reste.
///
/// # Erreurs
/// [`DomainError::TaskNotFound`] ou [`DomainError::TaskNotInTrash`].
pub fn restore_subtree(
    tree: &mut TaskTree,
    id: &TaskId,
    stamp: &ChangeStamp,
) -> Result<RestoreOutcome, DomainError> {
    let task = tree.get(id)?;
    let Some(deleted_at) = task.deleted_at() else {
        return Err(DomainError::TaskNotInTrash(id.clone()));
    };
    let moved_to_root = match task.parent() {
        None => false,
        Some(parent) => tree.get(parent).map_or(true, Task::is_deleted),
    };
    let deleted_together: Vec<TaskId> = tree
        .iter()
        .filter(|other| other.id() != id && other.deleted_at() == Some(deleted_at))
        .filter(|other| tree.is_in_subtree(other.id(), id))
        .map(|other| other.id().clone())
        .collect();

    let root_order = tree.order_at_end(None);
    let restored_task = tree.get_mut(id)?;
    restored_task.restore_from_trash(stamp)?;
    if moved_to_root {
        restored_task.detach_to_root(root_order, stamp);
    }
    for descendant in &deleted_together {
        tree.get_mut(descendant)?.restore_from_trash(stamp)?;
    }
    let mut restored = vec![id.clone()];
    restored.extend(deleted_together);
    Ok(RestoreOutcome {
        restored,
        moved_to_root,
    })
}

/// R6 — Tâches à afficher dans la corbeille : celles qui y sont **sans** que leur parent y soit
/// aussi (une sous-tâche supprimée avec son parent est montrée sous lui, repliée).
/// Triées de la plus récemment supprimée à la plus ancienne.
pub fn trash_roots(tree: &TaskTree) -> Vec<&Task> {
    let mut roots: Vec<&Task> = tree
        .iter()
        .filter(|task| task.is_deleted())
        .filter(|task| {
            tree.effective_parent(task)
                .and_then(|parent| tree.get(&parent).ok())
                .is_none_or(|parent| !parent.is_deleted())
        })
        .collect();
    roots.sort_by(|a, b| {
        b.deleted_at()
            .cmp(&a.deleted_at())
            .then_with(|| a.id().cmp(b.id()))
    });
    roots
}

/// Durée de conservation dans la corbeille (spec §11 : 30 jours).
pub const TRASH_RETENTION_DAYS: i64 = 30;

const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// Tâches de la corbeille à effacer définitivement à `now` : supprimées depuis plus de
/// [`TRASH_RETENTION_DAYS`] jours (sous-tâches comprises : elles portent la même date).
pub fn expired_in_trash(tree: &TaskTree, now: Timestamp) -> Vec<TaskId> {
    let limit = now.plus_millis(-TRASH_RETENTION_DAYS * MILLIS_PER_DAY);
    tree.iter()
        .filter(|task| task.deleted_at().is_some_and(|deleted| deleted <= limit))
        .map(|task| task.id().clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::{id, stamp, titles, tree};

    fn sample() -> TaskTree {
        tree(&[
            ("1", None),
            ("2", None),
            ("21", Some("2")),
            ("211", Some("21")),
            ("22", Some("2")),
        ])
    }

    #[test]
    fn only_tasks_deleted_more_than_thirty_days_ago_expire() {
        let day = MILLIS_PER_DAY;
        let mut tree = tree(&[("1", None), ("2", None), ("3", None)]);
        move_subtree_to_trash(&mut tree, &id("1"), &stamp(0)).unwrap();
        move_subtree_to_trash(&mut tree, &id("2"), &stamp(10 * day)).unwrap();
        let expired = expired_in_trash(&tree, Timestamp::from_millis(30 * day));
        assert_eq!(expired, [id("1")]);
        assert!(expired_in_trash(&tree, Timestamp::from_millis(29 * day)).is_empty());
    }

    #[test]
    fn r5_deleting_a_task_deletes_its_subtree() {
        let mut tree = sample();
        let mut deleted = move_subtree_to_trash(&mut tree, &id("2"), &stamp(10)).unwrap();
        deleted.sort();
        assert_eq!(deleted, vec![id("2"), id("21"), id("22"), id("211")]);
        assert_eq!(titles(&tree, None), ["1"]);
    }

    #[test]
    fn r6_trash_shows_only_the_deleted_roots() {
        let mut tree = sample();
        move_subtree_to_trash(&mut tree, &id("22"), &stamp(10)).unwrap();
        move_subtree_to_trash(&mut tree, &id("2"), &stamp(20)).unwrap();
        let roots: Vec<String> = trash_roots(&tree)
            .iter()
            .map(|t| t.title().to_string())
            .collect();
        assert_eq!(roots, ["2"]); // « 22 » est sous « 2 », lui aussi dans la corbeille
    }

    #[test]
    fn r7_restore_brings_back_what_was_deleted_together() {
        let mut tree = sample();
        move_subtree_to_trash(&mut tree, &id("22"), &stamp(10)).unwrap();
        move_subtree_to_trash(&mut tree, &id("2"), &stamp(20)).unwrap();
        let outcome = restore_subtree(&mut tree, &id("2"), &stamp(30)).unwrap();
        assert!(!outcome.moved_to_root);
        assert_eq!(titles(&tree, Some("2")), ["21"]); // « 22 », supprimée avant, reste dans la corbeille
        assert_eq!(titles(&tree, Some("21")), ["211"]);
    }

    #[test]
    fn r7_restore_to_root_when_the_parent_is_in_the_trash() {
        let mut tree = sample();
        move_subtree_to_trash(&mut tree, &id("21"), &stamp(10)).unwrap();
        move_subtree_to_trash(&mut tree, &id("2"), &stamp(20)).unwrap();
        let outcome = restore_subtree(&mut tree, &id("21"), &stamp(30)).unwrap();
        assert!(outcome.moved_to_root);
        assert_eq!(titles(&tree, None), ["1", "21"]);
        assert_eq!(titles(&tree, Some("21")), ["211"]);
    }

    #[test]
    fn restoring_an_active_task_is_refused() {
        let mut tree = sample();
        assert_eq!(
            restore_subtree(&mut tree, &id("1"), &stamp(10)),
            Err(DomainError::TaskNotInTrash(id("1")))
        );
    }
}
