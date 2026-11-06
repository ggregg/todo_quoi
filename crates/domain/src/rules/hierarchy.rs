//! Déplacements dans l'arbre (spécification §4.3).
//!
//! - **R9** : indenter (la tâche devient la dernière sous-tâche de la tâche au-dessus) ;
//!   désindenter (elle devient la sœur suivante de son parent).
//! - **R10** : glisser-déposer avant / après / dans → [`placement_after`].
//! - **R11** : interdit de déplacer une tâche dans son propre sous-arbre → [`apply_move`].
//! - **R12** : les propositions de clôture / réouverture après un déplacement sont calculées
//!   par l'application avec [`crate::rules::closure`].
//!
//! Chaque fonction calcule d'abord une [`Placement`] (où aller), puis [`apply_move`] l'applique.
//! Séparer « calculer » et « appliquer » rend chaque étape simple à tester.

use crate::{ChangeStamp, DomainError, OrderKey, Task, TaskId, TaskTree};

/// Destination d'un déplacement : un parent (`None` = premier niveau) et une position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// Nouveau parent.
    pub parent: Option<TaskId>,
    /// Nouvelle position parmi les enfants de ce parent.
    pub order: OrderKey,
}

/// Position juste **après** `after` parmi les enfants de `parent` (`after = None` : en tête).
/// `moving` est la tâche déplacée : elle est ignorée parmi les sœurs.
///
/// # Erreurs
/// [`DomainError::TaskNotFound`] si `after` n'est pas un enfant de `parent`.
pub fn placement_after(
    tree: &TaskTree,
    moving: Option<&TaskId>,
    parent: Option<&TaskId>,
    after: Option<&TaskId>,
) -> Result<Placement, DomainError> {
    let siblings: Vec<&Task> = tree
        .children(parent)
        .into_iter()
        .filter(|task| Some(task.id()) != moving)
        .collect();
    let after_index = match after {
        None => None,
        Some(after_id) => Some(
            siblings
                .iter()
                .position(|task| task.id() == after_id)
                .ok_or_else(|| DomainError::TaskNotFound(after_id.clone()))?,
        ),
    };
    let before = after_index.map(|index| siblings[index].order());
    let next = match after_index {
        None => siblings.first(),
        Some(index) => siblings.get(index + 1),
    }
    .map(|task| task.order());
    // Deux sœurs peuvent avoir la même clé (créées en même temps sur deux appareils) :
    // il n'existe alors pas de clé strictement entre elles ; on se place juste après la première.
    let order = OrderKey::between(before, next).or_else(|_| OrderKey::between(before, None))?;
    Ok(Placement {
        parent: parent.cloned(),
        order,
    })
}

/// R9 — Indenter : devenir la dernière sous-tâche de la tâche située juste au-dessus.
///
/// # Erreurs
/// [`DomainError::NoPreviousSibling`] si la tâche est la première de sa liste.
pub fn indent(tree: &TaskTree, id: &TaskId) -> Result<Placement, DomainError> {
    let (siblings, index) = siblings_and_index(tree, id)?;
    let Some(previous) = index.checked_sub(1).map(|i| siblings[i].id().clone()) else {
        return Err(DomainError::NoPreviousSibling(id.clone()));
    };
    let last_child = tree
        .children(Some(&previous))
        .last()
        .map(|task| task.id().clone());
    placement_after(tree, Some(id), Some(&previous), last_child.as_ref())
}

/// R9 — Désindenter : devenir la sœur suivante de son parent.
///
/// # Erreurs
/// [`DomainError::AlreadyAtRoot`] si la tâche est au premier niveau.
pub fn outdent(tree: &TaskTree, id: &TaskId) -> Result<Placement, DomainError> {
    let task = tree.get(id)?;
    let Some(parent_id) = tree.effective_parent(task) else {
        return Err(DomainError::AlreadyAtRoot(id.clone()));
    };
    let grandparent = tree.effective_parent(tree.get(&parent_id)?);
    placement_after(tree, Some(id), grandparent.as_ref(), Some(&parent_id))
}

/// Monter d'un cran parmi ses sœurs.
///
/// # Erreurs
/// [`DomainError::CannotMoveFurther`] si la tâche est déjà la première.
pub fn move_up(tree: &TaskTree, id: &TaskId) -> Result<Placement, DomainError> {
    let (siblings, index) = siblings_and_index(tree, id)?;
    if index == 0 {
        return Err(DomainError::CannotMoveFurther(id.clone()));
    }
    let after = index.checked_sub(2).map(|i| siblings[i].id().clone());
    let parent = tree.effective_parent(tree.get(id)?);
    placement_after(tree, Some(id), parent.as_ref(), after.as_ref())
}

/// Descendre d'un cran parmi ses sœurs.
///
/// # Erreurs
/// [`DomainError::CannotMoveFurther`] si la tâche est déjà la dernière.
pub fn move_down(tree: &TaskTree, id: &TaskId) -> Result<Placement, DomainError> {
    let (siblings, index) = siblings_and_index(tree, id)?;
    let Some(next) = siblings.get(index + 1).map(|task| task.id().clone()) else {
        return Err(DomainError::CannotMoveFurther(id.clone()));
    };
    let parent = tree.effective_parent(tree.get(id)?);
    placement_after(tree, Some(id), parent.as_ref(), Some(&next))
}

/// Applique un déplacement, après avoir vérifié R11 et que le nouveau parent est utilisable.
///
/// # Erreurs
/// [`DomainError::MoveIntoOwnSubtree`], [`DomainError::TaskNotFound`] ou
/// [`DomainError::TaskInTrash`] (parent dans la corbeille).
pub fn apply_move(
    tree: &mut TaskTree,
    id: &TaskId,
    placement: Placement,
    stamp: &ChangeStamp,
) -> Result<(), DomainError> {
    if let Some(parent) = &placement.parent {
        if tree.is_in_subtree(parent, id) {
            return Err(DomainError::MoveIntoOwnSubtree { task: id.clone() });
        }
        if tree.get(parent)?.is_deleted() {
            return Err(DomainError::TaskInTrash(parent.clone()));
        }
    }
    tree.get_mut(id)?
        .move_to(placement.parent, placement.order, stamp)
}

/// Sœurs d'une tâche (elle comprise) et sa position parmi elles.
fn siblings_and_index<'t>(
    tree: &'t TaskTree,
    id: &TaskId,
) -> Result<(Vec<&'t Task>, usize), DomainError> {
    // [rust] `'t` : « les références renvoyées vivent aussi longtemps que l'arbre emprunté ».
    //        Annotation exigée ici car la fonction reçoit deux références (`tree` et `id`).
    let parent = tree.effective_parent(tree.get(id)?);
    let siblings = tree.children(parent.as_ref());
    let index = siblings
        .iter()
        .position(|sibling| sibling.id() == id)
        .ok_or_else(|| DomainError::TaskInTrash(id.clone()))?;
    Ok((siblings, index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::{id, stamp, titles, tree};

    fn sample() -> TaskTree {
        tree(&[("1", None), ("2", None), ("3", None), ("21", Some("2"))])
    }

    fn apply(tree: &mut TaskTree, label: &str, placement: Placement) {
        apply_move(tree, &id(label), placement, &stamp(10)).unwrap();
    }

    #[test]
    fn r9_indent_becomes_last_child_of_previous_sibling() {
        let mut tree = sample();
        let placement = indent(&tree, &id("3")).unwrap();
        apply(&mut tree, "3", placement);
        assert_eq!(titles(&tree, None), ["1", "2"]);
        assert_eq!(titles(&tree, Some("2")), ["21", "3"]);
    }

    #[test]
    fn r9_cannot_indent_the_first_task() {
        assert_eq!(
            indent(&sample(), &id("1")),
            Err(DomainError::NoPreviousSibling(id("1")))
        );
    }

    #[test]
    fn r9_outdent_becomes_next_sibling_of_parent() {
        let mut tree = sample();
        let placement = outdent(&tree, &id("21")).unwrap();
        apply(&mut tree, "21", placement);
        assert_eq!(titles(&tree, None), ["1", "2", "21", "3"]);
        assert!(titles(&tree, Some("2")).is_empty());
    }

    #[test]
    fn r9_cannot_outdent_a_root_task() {
        assert_eq!(
            outdent(&sample(), &id("1")),
            Err(DomainError::AlreadyAtRoot(id("1")))
        );
    }

    #[test]
    fn move_up_and_down() {
        let mut tree = sample();
        let placement = move_up(&tree, &id("3")).unwrap();
        apply(&mut tree, "3", placement);
        assert_eq!(titles(&tree, None), ["1", "3", "2"]);
        let placement = move_up(&tree, &id("3")).unwrap();
        apply(&mut tree, "3", placement);
        assert_eq!(titles(&tree, None), ["3", "1", "2"]);
        assert_eq!(
            move_up(&tree, &id("3")),
            Err(DomainError::CannotMoveFurther(id("3")))
        );
        let placement = move_down(&tree, &id("3")).unwrap();
        apply(&mut tree, "3", placement);
        assert_eq!(titles(&tree, None), ["1", "3", "2"]);
    }

    #[test]
    fn r10_drop_after_a_given_sibling_or_first() {
        let mut tree = sample();
        let placement = placement_after(&tree, Some(&id("1")), None, Some(&id("2"))).unwrap();
        apply(&mut tree, "1", placement);
        assert_eq!(titles(&tree, None), ["2", "1", "3"]);
        let placement = placement_after(&tree, Some(&id("3")), Some(&id("2")), None).unwrap();
        apply(&mut tree, "3", placement);
        assert_eq!(titles(&tree, Some("2")), ["3", "21"]);
    }

    #[test]
    fn r11_cannot_move_a_task_into_its_own_subtree() {
        let mut tree = sample();
        let into_child = placement_after(&tree, Some(&id("2")), Some(&id("21")), None).unwrap();
        assert_eq!(
            apply_move(&mut tree, &id("2"), into_child, &stamp(10)),
            Err(DomainError::MoveIntoOwnSubtree { task: id("2") })
        );
        let into_itself = placement_after(&tree, Some(&id("2")), Some(&id("2")), None).unwrap();
        assert!(apply_move(&mut tree, &id("2"), into_itself, &stamp(10)).is_err());
    }
}
