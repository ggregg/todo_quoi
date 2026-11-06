//! Horloge « hybride » : l'heure système, mais qui **ne recule jamais**.
//!
//! Pourquoi : la règle « dernier qui écrit gagne » (ADR-0004) compare les dates de modification.
//! Si l'horloge système recule (réglage manuel, synchronisation NTP) ou si deux modifications
//! tombent dans la même milliseconde, deux versions pourraient avoir la même date, ou une
//! version plus récente une date plus ancienne. Ici, chaque lecture renvoie au moins
//! « dernière date + 1 ms ».

use std::sync::Mutex;
use tq_application::ports::Clock;
use tq_domain::Timestamp;

/// Horloge monotone : `max(heure système, dernière date fournie + 1 ms)`.
#[derive(Default)]
pub struct HybridClock {
    last: Mutex<i64>,
}

impl HybridClock {
    /// Nouvelle horloge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Prend en compte une date venue d'un autre appareil (lecture d'un fichier synchronisé) :
    /// les dates suivantes seront postérieures, même si l'horloge de l'autre appareil avance.
    pub fn observe(&self, remote: Timestamp) {
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *last = (*last).max(remote.as_millis());
    }

    /// Cœur de l'algorithme, séparé de l'heure système pour pouvoir le tester.
    fn next_after(&self, wall_clock_millis: i64) -> Timestamp {
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let next = wall_clock_millis.max(*last + 1);
        *last = next;
        Timestamp::from_millis(next)
    }
}

impl Clock for HybridClock {
    fn now(&self) -> Timestamp {
        self.next_after(crate::system_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_wall_clock_when_it_moves_forward() {
        let clock = HybridClock::new();
        assert_eq!(clock.next_after(1_000).as_millis(), 1_000);
        assert_eq!(clock.next_after(5_000).as_millis(), 5_000);
    }

    #[test]
    fn never_goes_backwards_nor_repeats() {
        let clock = HybridClock::new();
        clock.next_after(5_000);
        assert_eq!(clock.next_after(5_000).as_millis(), 5_001); // même milliseconde
        assert_eq!(clock.next_after(3_000).as_millis(), 5_002); // horloge système reculée
    }

    #[test]
    fn a_date_from_another_device_pushes_the_clock_forward() {
        let clock = HybridClock::new();
        clock.observe(Timestamp::from_millis(9_000));
        assert_eq!(clock.next_after(1_000).as_millis(), 9_001);
    }

    #[test]
    fn real_clock_is_strictly_increasing() {
        let clock = HybridClock::new();
        let dates: Vec<Timestamp> = (0..1_000).map(|_| clock.now()).collect();
        assert!(dates.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
