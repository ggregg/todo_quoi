//! Actions groupées sur une **sélection** de tâches (spec §5.5 : `Maj+↑/↓`, `Mod+clic`).
//!
//! Chaque tâche choisie est traitée comme si elle était seule, avec ses sous-tâches. Deux
//! précautions rendent le groupe sûr :
//! - les **ancêtres d'abord** : un parent restauré redevient actif avant ses sous-tâches choisies,
//!   qui reviennent donc à leur place et non au premier niveau ;
//! - **une seule fois** : une tâche déjà emportée par une autre de la sélection (sous-tâche d'une
//!   tâche choisie, ou choisie deux fois) est sautée au lieu de provoquer une erreur.

use crate::{DomainError, TaskId, TaskTree};

/// Applique `action` à chaque tâche de `ids`, des ancêtres vers les descendants, sauf à celles
/// qu'une action précédente a déjà touchées. `action` renvoie les tâches qu'elle a touchées ;
/// le résultat les réunit, dans l'ordre.
///
/// # Erreurs
/// La première erreur de `action` (rien n'est enregistré : l'application ne valide l'arbre
/// qu'en cas de succès).
// [rust] `impl FnMut(…)` : une fonction passée en paramètre qui peut modifier ce qu'elle capture
//        (≈ un callback TS qui écrit dans une variable de la fonction appelante).
pub(crate) fn each_once(
    tree: &mut TaskTree,
    ids: &[TaskId],
    mut action: impl FnMut(&mut TaskTree, &TaskId) -> Result<Vec<TaskId>, DomainError>,
) -> Result<Vec<TaskId>, DomainError> {
    let mut touched: Vec<TaskId> = Vec::new();
    for id in ancestors_first(tree, ids) {
        if touched.contains(&id) {
            continue;
        }
        touched.extend(action(tree, &id)?);
    }
    Ok(touched)
}

/// `ids` sans doublon, triés par profondeur croissante (l'ordre donné est gardé à profondeur
/// égale). Une tâche introuvable compte pour la profondeur 0 : l'action dira qu'elle manque.
fn ancestors_first(tree: &TaskTree, ids: &[TaskId]) -> Vec<TaskId> {
    let mut unique: Vec<TaskId> = Vec::new();
    for id in ids {
        if !unique.contains(id) {
            unique.push(id.clone());
        }
    }
    // [rust] `sort_by_key` est stable : à clé égale, l'ordre d'origine est conservé.
    unique.sort_by_key(|id| tree.ancestors(id).map_or(0, |ancestors| ancestors.len()));
    unique
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::{id, tree};

    #[test]
    fn ancestors_come_first_and_each_task_once() {
        let tree = tree(&[("1", None), ("2", Some("1")), ("3", Some("2")), ("4", None)]);
        let ids = [id("3"), id("4"), id("1"), id("3")];
        assert_eq!(ancestors_first(&tree, &ids), [id("4"), id("1"), id("3")]);
    }

    #[test]
    fn a_task_already_touched_is_skipped() {
        let mut tree = tree(&[("1", None), ("2", Some("1")), ("3", None)]);
        let mut calls = Vec::new();
        let touched = each_once(&mut tree, &[id("2"), id("1"), id("3")], |_, task| {
            calls.push(task.clone());
            // « 1 » emporte « 2 », comme une sous-tâche.
            Ok(if *task == id("1") {
                vec![id("1"), id("2")]
            } else {
                vec![task.clone()]
            })
        })
        .expect("aucune erreur");
        assert_eq!(calls, [id("1"), id("3")]);
        assert_eq!(touched, [id("1"), id("2"), id("3")]);
    }
}
