//! Corbeille (spécification §4.2).
//!
//! - **R5** : supprimer = mettre la tâche **et ses sous-tâches actives** à la corbeille.
//! - **R6** : la corbeille liste les « racines » supprimées → [`trash_roots`].
//! - **R7** : restaurer remet le sous-arbre à sa place ; si le parent d'origine n'existe plus
//!   ou est lui-même dans la corbeille, la tâche revient au premier niveau.
//! - **R8** (purge définitive) touche aux fichiers : c'est l'affaire de l'application (I5).

use crate::{
    BaseId, ChangeStamp, DomainError, MediaRef, Task, TaskId, TaskTree, Timestamp, media_refs_in,
};
use std::collections::HashSet;

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

const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// Durée de conservation dans la corbeille, en jours (réglage de l'appareil, spec §5.7 ; 30 par
/// défaut, spec §11). Toujours entre 1 et 365 : une valeur hors bornes est ramenée dedans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrashRetention(u32);

impl TrashRetention {
    /// 30 jours.
    pub const DEFAULT: Self = Self(30);
    /// Bornes acceptées.
    pub const MIN_DAYS: u32 = 1;
    /// Bornes acceptées.
    pub const MAX_DAYS: u32 = 365;

    /// Durée de `days` jours, ramenée entre [`Self::MIN_DAYS`] et [`Self::MAX_DAYS`].
    pub fn from_days(days: u32) -> Self {
        Self(days.clamp(Self::MIN_DAYS, Self::MAX_DAYS))
    }

    /// Nombre de jours.
    pub fn days(self) -> u32 {
        self.0
    }

    /// Instant avant lequel une suppression a expiré, vu de `now`.
    pub fn limit(self, now: Timestamp) -> Timestamp {
        now.plus_millis(-i64::from(self.0) * MILLIS_PER_DAY)
    }
}

/// Durée de conservation des versions écartées par les conflits (spec §11 : 30 jours).
pub const CONFLICT_RETENTION: TrashRetention = TrashRetention::DEFAULT;

/// Tâches de la corbeille à effacer définitivement à `now` : supprimées depuis plus de
/// `retention` (sous-tâches comprises : elles portent la même date).
pub fn expired_in_trash(tree: &TaskTree, now: Timestamp, retention: TrashRetention) -> Vec<TaskId> {
    let limit = retention.limit(now);
    tree.iter()
        .filter(|task| task.deleted_at().is_some_and(|deleted| deleted <= limit))
        .map(|task| task.id().clone())
        .collect()
}

/// Tâches à effacer **définitivement** quand l'utilisateur purge `id` depuis la corbeille : la
/// tâche et ses sous-tâches encore à la corbeille (une sous-tâche restaurée entre-temps n'en
/// fait plus partie : elle a été rattachée ailleurs).
///
/// # Erreurs
/// [`DomainError::TaskNotFound`] ; [`DomainError::TaskNotInTrash`] : on ne purge jamais une
/// tâche active (il faut d'abord la mettre à la corbeille).
pub fn purgeable_subtree(tree: &TaskTree, id: &TaskId) -> Result<Vec<TaskId>, DomainError> {
    let task = tree.get(id)?;
    if !task.is_deleted() {
        return Err(DomainError::TaskNotInTrash(id.clone()));
    }
    let mut ids = vec![id.clone()];
    ids.extend(
        tree.iter()
            .filter(|other| other.id() != id && other.is_deleted())
            .filter(|other| tree.is_in_subtree(other.id(), id))
            .map(|other| other.id().clone()),
    );
    Ok(ids)
}

/// Médias à effacer avec les tâches `purged` (R8) : cités par leurs notes et par **aucune**
/// autre tâche de la même base (corbeille comprise) — un même fichier collé dans deux notes
/// n'est stocké qu'une fois, nommé par son contenu.
///
/// Prudence : rien n'est effacé dans une base où une note est **scellée** (coffre verrouillé) :
/// on ne peut pas savoir quels médias elle cite. Ils le seront à une purge ultérieure.
pub fn orphaned_media(tree: &TaskTree, purged: &[TaskId]) -> Vec<(BaseId, MediaRef)> {
    let purged_set: HashSet<&TaskId> = purged.iter().collect();
    let mut orphans: Vec<(BaseId, MediaRef)> = Vec::new();
    for task in tree.iter().filter(|task| purged_set.contains(task.id())) {
        if base_has_sealed_note(tree, task.base()) {
            continue;
        }
        for media in media_refs_in(task.note()) {
            let candidate = (task.base().clone(), media);
            let still_cited = tree.iter().any(|other| {
                !purged_set.contains(other.id())
                    && other.base() == task.base()
                    && media_refs_in(other.note()).contains(&candidate.1)
            });
            if !still_cited && !orphans.contains(&candidate) {
                orphans.push(candidate);
            }
        }
    }
    orphans
}

/// Parmi `candidates` (médias que la note d'une tâche de `base` vient de cesser de citer : ses
/// médias ont été chiffrés, ou remis en clair), ceux qu'**aucune** tâche de la base ne cite
/// plus : on peut les effacer. Un média en clair laissé derrière une note chiffrée trahirait
/// son contenu.
///
/// Même prudence que [`orphaned_media`] : rien si une note de la base est scellée.
pub fn uncited_media(tree: &TaskTree, base: &BaseId, candidates: &[MediaRef]) -> Vec<MediaRef> {
    if base_has_sealed_note(tree, base) {
        return Vec::new();
    }
    candidates
        .iter()
        .filter(|media| {
            !tree
                .iter()
                .any(|task| task.base() == base && media_refs_in(task.note()).contains(media))
        })
        .cloned()
        .collect()
}

fn base_has_sealed_note(tree: &TaskTree, base: &BaseId) -> bool {
    tree.iter()
        .any(|task| task.base() == base && task.is_sealed())
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
    fn purging_takes_the_task_and_its_subtasks_still_in_the_trash() {
        let mut tree = sample();
        move_subtree_to_trash(&mut tree, &id("2"), &stamp(10)).unwrap();
        let mut purged = purgeable_subtree(&tree, &id("2")).unwrap();
        purged.sort();
        let mut expected = vec![id("2"), id("21"), id("211"), id("22")];
        expected.sort();
        assert_eq!(purged, expected);
    }

    fn note_citing(names: &[&str]) -> crate::NoteBody {
        let links: Vec<String> = names
            .iter()
            .map(|name| format!("![x](../media/{}.jpg)", name.repeat(64)))
            .collect();
        crate::NoteBody::from_markdown(links.join("\n"))
    }

    fn media(name: &str) -> MediaRef {
        MediaRef::parse(&format!("{}.jpg", name.repeat(64))).unwrap()
    }

    fn with_note(mut task: Task, names: &[&str]) -> Task {
        task.replace_note(note_citing(names), &stamp(1)).unwrap();
        task
    }

    #[test]
    fn purged_media_go_unless_another_task_of_the_base_still_cites_them() {
        let base = tree(&[("1", None), ("2", None), ("3", None)]);
        let pick = |label: &str| base.get(&id(label)).unwrap().clone();
        let (one, two, three) = (pick("1"), pick("2"), pick("3"));
        let tree = TaskTree::new(vec![
            with_note(one, &["a", "b"]),
            with_note(two, &["b"]),
            with_note(three.in_base(BaseId::parse("pro").unwrap()), &["a"]),
        ])
        .unwrap();
        // « a » n'est cité ailleurs que dans une AUTRE base : il part. « b » reste (tâche 2).
        assert_eq!(
            orphaned_media(&tree, &[id("1")]),
            [(BaseId::main(), media("a"))]
        );
    }

    #[test]
    fn nothing_is_erased_in_a_base_with_a_sealed_note() {
        let base = tree(&[("1", None), ("2", None)]);
        let one = base.get(&id("1")).unwrap().clone();
        let mut record = base.get(&id("2")).unwrap().to_record();
        record.confidentiality = crate::Confidentiality::Sealed(crate::SealedContent {
            key_id: crate::KeyId::parse("0123456789abcdef0123456789abcdef").unwrap(),
            sealed: "x".into(),
        });
        let tree = TaskTree::new(vec![with_note(one, &["a"]), Task::from_record(record)]).unwrap();
        assert!(orphaned_media(&tree, &[id("1")]).is_empty());
    }

    #[test]
    fn media_no_note_of_the_base_cites_any_more_can_go() {
        let base = tree(&[("1", None), ("2", None), ("3", None)]);
        let pick = |label: &str| base.get(&id(label)).unwrap().clone();
        let tree = TaskTree::new(vec![
            with_note(pick("1"), &["c"]),
            with_note(pick("2"), &["b"]),
            with_note(pick("3").in_base(BaseId::parse("pro").unwrap()), &["a"]),
        ])
        .unwrap();
        let candidates = [media("a"), media("b")];
        // « a » n'est plus cité que dans une autre base ; « b » l'est encore par la tâche 2.
        assert_eq!(
            uncited_media(&tree, &BaseId::main(), &candidates),
            [media("a")]
        );
    }

    #[test]
    fn an_active_task_is_never_purged() {
        let tree = sample();
        assert!(matches!(
            purgeable_subtree(&tree, &id("1")),
            Err(DomainError::TaskNotInTrash(_))
        ));
    }

    #[test]
    fn only_tasks_deleted_more_than_thirty_days_ago_expire() {
        let day = MILLIS_PER_DAY;
        let mut tree = tree(&[("1", None), ("2", None), ("3", None)]);
        move_subtree_to_trash(&mut tree, &id("1"), &stamp(0)).unwrap();
        move_subtree_to_trash(&mut tree, &id("2"), &stamp(10 * day)).unwrap();
        let thirty = TrashRetention::DEFAULT;
        let expired = expired_in_trash(&tree, Timestamp::from_millis(30 * day), thirty);
        assert_eq!(expired, [id("1")]);
        assert!(expired_in_trash(&tree, Timestamp::from_millis(29 * day), thirty).is_empty());
        // Réglée à 7 jours : la tâche supprimée au jour 10 expire au jour 17.
        let week = TrashRetention::from_days(7);
        let expired = expired_in_trash(&tree, Timestamp::from_millis(17 * day), week);
        assert_eq!(expired.len(), 2);
    }

    #[test]
    fn retention_stays_between_one_day_and_one_year() {
        assert_eq!(TrashRetention::from_days(0).days(), 1);
        assert_eq!(TrashRetention::from_days(90).days(), 90);
        assert_eq!(TrashRetention::from_days(10_000).days(), 365);
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
