//! Verrouillage **automatique** des coffres après un délai sans activité (spec §9).
//!
//! L'interface signale l'activité de l'utilisateur ([`AutoLockVaults::record_activity`]) ; un
//! minuteur de `src-tauri` appelle [`AutoLockVaults::execute`] régulièrement. Un coffre gardé
//! dans le trousseau (« Se souvenir sur cet appareil ») ne se verrouille pas tout seul : il se
//! rouvrirait seul au lancement suivant, l'utilisateur a choisi de faire confiance à l'appareil.

use crate::ports::Clock;
use crate::vault::{AutoLockDelay, OpenVaults};
use std::sync::{Arc, Mutex, MutexGuard};
use tq_domain::{BaseId, Timestamp};

/// Ce que le cas d'usage retient entre deux appels.
#[derive(Debug, Clone, Copy)]
struct AutoLockState {
    last_activity: Timestamp,
    delay: AutoLockDelay,
}

/// Cas d'usage « verrouiller les coffres inactifs ». Garde la dernière activité et le délai.
pub struct AutoLockVaults {
    vaults: Arc<OpenVaults>,
    clock: Arc<dyn Clock>,
    state: Mutex<AutoLockState>,
}

impl AutoLockVaults {
    /// Assemble le cas d'usage ; l'activité part de maintenant.
    pub fn new(vaults: Arc<OpenVaults>, clock: Arc<dyn Clock>, delay: AutoLockDelay) -> Self {
        let now = clock.now();
        Self {
            vaults,
            clock,
            state: Mutex::new(AutoLockState {
                last_activity: now,
                delay,
            }),
        }
    }

    /// L'utilisateur vient d'agir (frappe, clic) : le délai repart de zéro.
    pub fn record_activity(&self) {
        self.state().last_activity = self.clock.now();
    }

    /// Change le délai (réglage de l'appareil).
    pub fn set_delay(&self, delay: AutoLockDelay) {
        self.state().delay = delay;
    }

    /// Délai en vigueur.
    pub fn delay(&self) -> AutoLockDelay {
        self.state().delay
    }

    /// Verrouille les coffres si le délai est dépassé ; renvoie les bases verrouillées.
    pub fn execute(&self) -> Vec<BaseId> {
        // [rust] `*self.state()` : copie de l'état (il est `Copy`) à travers le verrou, qui est
        //        relâché tout de suite (≈ lire une valeur puis libérer le verrou).
        let state = *self.state();
        if !state.delay.is_due(state.last_activity, self.clock.now()) {
            return Vec::new();
        }
        self.vaults.close_unremembered()
    }

    fn state(&self) -> MutexGuard<'_, AutoLockState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
