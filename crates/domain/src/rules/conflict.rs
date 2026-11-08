//! Conflits entre deux versions d'une même tâche (spec §7, ADR-0004) : **le dernier qui écrit
//! gagne**. Deux appareils peuvent avoir modifié la tâche hors ligne ; le drive garde alors deux
//! fichiers (`<id>.md` et une copie de conflit) ou l'un écrase l'autre.

use crate::Task;

/// Vrai si `candidate` doit remplacer `current` : modifiée plus tard, ou — à la milliseconde
/// près — par un appareil dont le nom vient après (départage arbitraire mais **identique sur
/// tous les appareils**, pour qu'ils choisissent le même gagnant).
pub fn is_newer(candidate: &Task, current: &Task) -> bool {
    (candidate.updated_at(), candidate.updated_by().as_str())
        > (current.updated_at(), current.updated_by().as_str())
}

#[cfg(test)]
mod tests {
    use super::is_newer;
    use crate::rules::test_support::id;
    use crate::{ChangeStamp, DeviceId, NewTask, OrderKey, Task, Timestamp, Title};

    fn version(millis: i64, device: &str) -> Task {
        let stamp = ChangeStamp {
            at: Timestamp::from_millis(millis),
            by: DeviceId::parse(device).expect("appareil"),
        };
        let new = NewTask {
            id: id("1"),
            parent: None,
            order: OrderKey::first(),
            title: Title::parse("T").expect("titre"),
        };
        Task::create(new, &stamp)
    }

    #[test]
    fn the_latest_write_wins() {
        assert!(is_newer(&version(2, "pc"), &version(1, "mac")));
        assert!(!is_newer(&version(1, "pc"), &version(2, "mac")));
    }

    #[test]
    fn same_millisecond_is_settled_by_device_name_identically_everywhere() {
        assert!(is_newer(&version(5, "pc"), &version(5, "mac")));
        assert!(!is_newer(&version(5, "mac"), &version(5, "pc")));
        assert!(!is_newer(&version(5, "mac"), &version(5, "mac")));
    }
}
