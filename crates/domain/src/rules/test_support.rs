//! Outils pour écrire les tests des règles de façon lisible :
//! `let tree = tree(&[("1", None), ("1.1", Some("1"))]);`

use crate::{ChangeStamp, DeviceId, NewTask, Task, TaskId, TaskTree, Timestamp, Title};

/// Identifiant de test lisible : `id("1.2")` → UUID dérivé du texte.
pub fn id(label: &str) -> TaskId {
    let digits: String = label.chars().filter(char::is_ascii_digit).collect();
    TaskId::parse(&format!("00000000-0000-7000-8000-{digits:0>12}"))
        .expect("libellé de test valide")
}

/// Horodatage de test.
pub fn stamp(millis: i64) -> ChangeStamp {
    ChangeStamp {
        at: Timestamp::from_millis(millis),
        by: DeviceId::parse("test").expect("valide"),
    }
}

/// Arbre de test : chaque tâche est ajoutée à la fin de son parent, dans l'ordre donné.
pub fn tree(spec: &[(&str, Option<&str>)]) -> TaskTree {
    let mut tree = TaskTree::default();
    for (label, parent) in spec {
        let parent = parent.map(id);
        let order = tree.order_at_end(parent.as_ref());
        let new = NewTask {
            id: id(label),
            parent,
            order,
            title: Title::parse(label).expect("titre"),
        };
        tree.insert(Task::create(new, &stamp(1)))
            .expect("pas de doublon");
    }
    tree
}

/// Titres des enfants de `parent`, dans l'ordre d'affichage.
pub fn titles(tree: &TaskTree, parent: Option<&str>) -> Vec<String> {
    tree.children(parent.map(id).as_ref())
        .iter()
        .map(|task| task.title().to_string())
        .collect()
}

/// Ferme une tâche de test.
pub fn close(tree: &mut TaskTree, label: &str) {
    tree.get_mut(&id(label))
        .expect("existe")
        .close(&stamp(2))
        .expect("ouverte");
}
