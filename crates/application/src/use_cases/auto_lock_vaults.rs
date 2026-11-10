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

/// Cas d'usage « verrouiller les coffres inactifs ». Garde la dernière activité et le délai.
pub struct AutoLockVaults {
    vaults: Arc<OpenVaults>,
    clock: Arc<dyn Clock>,
    state: Mutex<(Timestamp, AutoLockDelay)>,
}

impl AutoLockVaults {
    /// Assemble le cas d'usage ; l'activité part de maintenant.
    pub fn new(vaults: Arc<OpenVaults>, clock: Arc<dyn Clock>, delay: AutoLockDelay) -> Self {
        let now = clock.now();
        Self {
            vaults,
            clock,
            state: Mutex::new((now, delay)),
        }
    }

    /// L'utilisateur vient d'agir (frappe, clic) : le délai repart de zéro.
    pub fn record_activity(&self) {
        self.state().0 = self.clock.now();
    }

    /// Change le délai (réglage de l'appareil).
    pub fn set_delay(&self, delay: AutoLockDelay) {
        self.state().1 = delay;
    }

    /// Délai en vigueur.
    pub fn delay(&self) -> AutoLockDelay {
        self.state().1
    }

    /// Verrouille les coffres si le délai est dépassé ; renvoie les bases verrouillées.
    pub fn execute(&self) -> Vec<BaseId> {
        let (last_activity, delay) = *self.state();
        if !delay.is_due(last_activity, self.clock.now()) {
            return Vec::new();
        }
        self.vaults.close_unremembered()
    }

    fn state(&self) -> MutexGuard<'_, (Timestamp, AutoLockDelay)> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
