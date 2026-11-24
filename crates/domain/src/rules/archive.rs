//! Archives (spécification §4.4, ADR-0015).
//!
//! - **R13** : archiver = archiver la tâche **et ses sous-tâches actives**, à la même date
//!   → [`archive_subtree`] ; avant, l'interface demande confirmation s'il reste des tâches
//!   ouvertes → [`archive_impact`].
//! - **R14** : une tâche archivée est en lecture seule (voir `Task::ensure_active`) et absente
//!   des listes (`TaskTree::children` ne rend que des tâches actives).
//! - **R16** : restaurer remet le sous-arbre archivé en même temps à sa place, ou au premier
//!   niveau si le parent n'est plus actif → [`restore_archived_subtree`].
//! - **R18** : une même tâche à la fois active et archivée (synchronisation) → la version la plus
//!   récente l'emporte → [`with_archives`], [`same_apart_from_archiving`].
//!
//! Le rangement des fichiers dans `archive/` est l'affaire du stockage (adaptateur).

use crate::rules::conflict::is_newer;
use crate::rules::trash::RestoreOutcome;
use crate::{ChangeStamp, DomainError, Task, TaskId, TaskTree};
use std::collections::HashMap;

/// Ce qu'emporterait l'archivage d'une tâche (question posée avant d'archiver, R13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveImpact {
    /// Tâches archivées : la tâche et ses sous-tâches actives.
    pub tasks: usize,
    /// Parmi elles, les tâches encore ouvertes (la tâche comprise) : s'il y en a, l'interface
    /// demande « Archiver aussi les N tâches ouvertes ? ».
    pub open_tasks: usize,
}

/// R13 — Tâches qu'emporterait l'archivage de `id` : elle et ses sous-tâches actives.
///
/// # Erreurs
/// [`DomainError::TaskNotFound`], [`DomainError::TaskInTrash`] ou [`DomainError::TaskArchived`].
fn archivable_subtree(tree: &TaskTree, id: &TaskId) -> Result<Vec<TaskId>, DomainError> {
    let task = tree.get(id)?;
    if task.is_deleted() {
        return Err(DomainError::TaskInTrash(id.clone()));
    }
    if task.is_archived() {
        return Err(DomainError::TaskArchived(id.clone()));
    }
    let mut ids = vec![id.clone()];
    ids.extend(
        tree.descendants(id)
            .into_iter()
            .map(|task| task.id().clone()),
    );
    Ok(ids)
}

/// R13 — Combien de tâches l'archivage de `id` emporterait, et combien sont ouvertes.
///
/// # Erreurs
/// Comme [`archive_subtree`].
pub fn archive_impact(tree: &TaskTree, id: &TaskId) -> Result<ArchiveImpact, DomainError> {
    let ids = archivable_subtree(tree, id)?;
    let mut open_tasks = 0;
    for task_id in &ids {
        if !tree.get(task_id)?.is_done() {
            open_tasks += 1;
        }
    }
    Ok(ArchiveImpact {
        tasks: ids.len(),
        open_tasks,
    })
}

/// R13 — Archive `id` et ses sous-tâches actives, toutes à la même date (c'est ce qui permet
/// de les restaurer ensemble, R16). Les sous-tâches à la corbeille y restent. Renvoie les tâches
/// archivées, la tâche demandée en premier (utile pour « Annuler »).
///
/// # Erreurs
/// [`DomainError::TaskNotFound`], [`DomainError::TaskInTrash`] ou [`DomainError::TaskArchived`].
pub fn archive_subtree(
    tree: &mut TaskTree,
    id: &TaskId,
    stamp: &ChangeStamp,
) -> Result<Vec<TaskId>, DomainError> {
    let ids = archivable_subtree(tree, id)?;
    for task_id in &ids {
        tree.get_mut(task_id)?.archive(stamp)?;
    }
    Ok(ids)
}

/// R16 — Restaure `id` et les sous-tâches archivées **en même temps** qu'elle (même date
/// d'archivage) : une sous-tâche archivée plus tôt, à part, reste archivée. Si le parent n'est
/// plus actif (corbeille, archivé, absent), la tâche revient au premier niveau.
///
/// `tree` doit contenir les tâches actives **et** les archives (sinon le parent paraîtrait absent).
///
/// # Erreurs
/// [`DomainError::TaskNotFound`] ou [`DomainError::TaskNotArchived`].
pub fn restore_archived_subtree(
    tree: &mut TaskTree,
    id: &TaskId,
    stamp: &ChangeStamp,
) -> Result<RestoreOutcome, DomainError> {
    let task = tree.get(id)?;
    let Some(archived_at) = task.archived_at() else {
        return Err(DomainError::TaskNotArchived(id.clone()));
    };
    let moved_to_root = match task.parent() {
        None => false,
        Some(parent) => tree.get(parent).map_or(true, |parent| !parent.is_active()),
    };
    let archived_together: Vec<TaskId> = tree
        .iter()
        .filter(|other| other.id() != id && other.archived_at() == Some(archived_at))
        .filter(|other| tree.is_in_subtree(other.id(), id))
        .map(|other| other.id().clone())
        .collect();

    let root_order = tree.order_at_end(None);
    let restored_task = tree.get_mut(id)?;
    restored_task.restore_from_archive(stamp)?;
    if moved_to_root {
        restored_task.detach_to_root(root_order, stamp);
    }
    for descendant in &archived_together {
        tree.get_mut(descendant)?.restore_from_archive(stamp)?;
    }
    let mut restored = vec![id.clone()];
    restored.extend(archived_together);
    Ok(RestoreOutcome {
        restored,
        moved_to_root,
    })
}

/// R18 — Tâches actives et archives réunies, pour un arbre complet (restaurer, purger). Une
/// tâche présente des deux côtés (archivée sur un appareil, modifiée sur l'autre) n'est gardée
/// qu'une fois : sa version la plus récente.
pub fn with_archives(active: Vec<Task>, archived: Vec<Task>) -> Vec<Task> {
    let mut by_id: HashMap<TaskId, Task> = HashMap::new();
    for task in active.into_iter().chain(archived) {
        let keep = by_id
            .get(task.id())
            .is_none_or(|known| is_newer(&task, known));
        if keep {
            by_id.insert(task.id().clone(), task);
        }
    }
    by_id.into_values().collect()
}

/// R18 — Vrai si `a` et `b` ne diffèrent que par l'archivage (date d'archivage, auteur et date
/// de la modification) : l'une est la copie d'avant l'archivage (ou la restauration) de l'autre,
/// que le drive n'a pas encore effacée. Rien n'est perdu en l'effaçant ; sinon, la version
/// perdante doit être mise de côté (`.conflicts/`).
pub fn same_apart_from_archiving(a: &Task, b: &Task) -> bool {
    // Les deux photos sont comparées sans date d'archivage, avec la date et l'auteur de `a`.
    let neutral = |task: &Task| {
        let mut record = task.to_record();
        record.archived_at = None;
        record.updated_at = a.updated_at();
        record.updated_by = a.updated_by().clone();
        record
    };
    neutral(a) == neutral(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::{close, id, stamp, titles, tree};
    use crate::rules::trash::move_subtree_to_trash;
    use crate::{Timestamp, Title};

    /// Préparation de l'arbre propre à un cas de test.
    // [rust] `fn(&mut TaskTree)` : type « pointeur de fonction » (une fonction, ou une closure
    //        qui ne capture rien) ; `type` lui donne un nom (≈ `type Prepare = (t: Tree) => void`).
    type Prepare = fn(&mut TaskTree);

    fn sample() -> TaskTree {
        tree(&[
            ("1", None),
            ("2", None),
            ("21", Some("2")),
            ("211", Some("21")),
            ("22", Some("2")),
        ])
    }

    fn archived_ids(tree: &TaskTree) -> Vec<TaskId> {
        let mut ids: Vec<TaskId> = tree
            .iter()
            .filter(|task| task.is_archived())
            .map(|task| task.id().clone())
            .collect();
        ids.sort();
        ids
    }

    #[test]
    fn r13_archiving_a_task_archives_its_active_subtree_at_the_same_date() {
        let mut tree = sample();
        let archived = archive_subtree(&mut tree, &id("2"), &stamp(10)).unwrap();
        assert_eq!(archived[0], id("2"));
        assert_eq!(
            archived_ids(&tree),
            [id("2"), id("21"), id("22"), id("211")]
        );
        assert!(
            tree.iter()
                .filter(|task| task.is_archived())
                .all(|task| task.archived_at() == Some(Timestamp::from_millis(10)))
        );
        assert_eq!(titles(&tree, None), ["1"]); // R14 : absentes des listes
    }

    #[test]
    fn r13_subtasks_in_the_trash_stay_there() {
        let mut tree = sample();
        move_subtree_to_trash(&mut tree, &id("22"), &stamp(5)).unwrap();
        archive_subtree(&mut tree, &id("2"), &stamp(10)).unwrap();
        let in_trash = tree.get(&id("22")).unwrap();
        assert!(in_trash.is_deleted() && !in_trash.is_archived());
    }

    #[test]
    fn r13_archiving_is_refused_in_the_trash_and_twice() {
        let cases: [(&str, Prepare, DomainError); 2] = [
            (
                "corbeille",
                |tree| {
                    move_subtree_to_trash(tree, &id("1"), &stamp(5)).unwrap();
                },
                DomainError::TaskInTrash(id("1")),
            ),
            (
                "déjà archivée",
                |tree| {
                    archive_subtree(tree, &id("1"), &stamp(5)).unwrap();
                },
                DomainError::TaskArchived(id("1")),
            ),
        ];
        for (case, prepare, expected) in cases {
            let mut tree = sample();
            prepare(&mut tree);
            assert_eq!(
                archive_subtree(&mut tree, &id("1"), &stamp(10)),
                Err(expected.clone()),
                "{case}"
            );
            assert_eq!(archive_impact(&tree, &id("1")), Err(expected), "{case}");
        }
    }

    #[test]
    fn r13_the_impact_counts_the_open_tasks_of_the_subtree() {
        let mut tree = sample();
        close(&mut tree, "211");
        close(&mut tree, "1");
        let cases = [("2", 4, 3), ("21", 2, 1), ("211", 1, 0), ("1", 1, 0)];
        for (label, tasks, open_tasks) in cases {
            assert_eq!(
                archive_impact(&tree, &id(label)),
                Ok(ArchiveImpact { tasks, open_tasks }),
                "tâche {label}"
            );
        }
    }

    #[test]
    fn r16_restore_brings_back_what_was_archived_together() {
        let mut tree = sample();
        archive_subtree(&mut tree, &id("22"), &stamp(10)).unwrap();
        archive_subtree(&mut tree, &id("2"), &stamp(20)).unwrap();
        let outcome = restore_archived_subtree(&mut tree, &id("2"), &stamp(30)).unwrap();
        assert!(!outcome.moved_to_root);
        assert_eq!(outcome.restored.len(), 3);
        assert_eq!(titles(&tree, Some("2")), ["21"]); // « 22 », archivée avant, le reste
        assert_eq!(titles(&tree, Some("21")), ["211"]);
        assert_eq!(archived_ids(&tree), [id("22")]);
    }

    #[test]
    fn r16_restore_to_root_when_the_parent_is_no_longer_active() {
        let cases: [(&str, Prepare); 2] = [
            ("parent archivé", |tree| {
                archive_subtree(tree, &id("2"), &stamp(20)).unwrap();
            }),
            ("parent à la corbeille", |tree| {
                move_subtree_to_trash(tree, &id("2"), &stamp(20)).unwrap();
            }),
        ];
        for (case, retire_parent) in cases {
            let mut tree = sample();
            archive_subtree(&mut tree, &id("21"), &stamp(10)).unwrap();
            retire_parent(&mut tree);
            let outcome = restore_archived_subtree(&mut tree, &id("21"), &stamp(30)).unwrap();
            assert!(outcome.moved_to_root, "{case}");
            assert_eq!(titles(&tree, None), ["1", "21"], "{case}");
            assert_eq!(titles(&tree, Some("21")), ["211"], "{case}");
        }
    }

    #[test]
    fn r16_restoring_an_active_task_is_refused() {
        let mut tree = sample();
        assert_eq!(
            restore_archived_subtree(&mut tree, &id("1"), &stamp(10)),
            Err(DomainError::TaskNotArchived(id("1")))
        );
    }

    #[test]
    fn r18_the_most_recent_version_wins_between_active_and_archived() {
        let base = sample();
        let active = base.get(&id("1")).unwrap().clone();
        let mut archived = active.clone();
        archived.archive(&stamp(10)).unwrap();
        let mut edited_later = active.clone();
        edited_later
            .rename(Title::parse("Modifiée").unwrap(), &stamp(20))
            .unwrap();
        let merged = with_archives(vec![active], vec![archived.clone()]);
        assert_eq!(merged, [archived.clone()]);
        let merged = with_archives(vec![edited_later.clone()], vec![archived]);
        assert_eq!(merged, [edited_later]);
    }

    #[test]
    fn r18_a_copy_left_behind_by_archiving_differs_only_by_archiving() {
        let base = sample();
        let before = base.get(&id("1")).unwrap().clone();
        let mut archived = before.clone();
        archived.archive(&stamp(10)).unwrap();
        assert!(same_apart_from_archiving(&before, &archived));
        assert!(same_apart_from_archiving(&archived, &before));
        let mut renamed = before.clone();
        renamed
            .rename(Title::parse("Modifiée").unwrap(), &stamp(5))
            .unwrap();
        assert!(!same_apart_from_archiving(&renamed, &archived));
    }
}
