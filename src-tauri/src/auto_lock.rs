//! **Coquille Tauri** (hors de l'hexagone) : minuteur du **verrouillage automatique** des
//! coffres (spec §9) : un fil qui, toutes les
//! 20 s, demande au cas d'usage `AutoLockVaults` si le délai sans activité est dépassé. Les
//! coffres verrouillés : copies déchiffrées effacées, fenêtres prévenues (`vaults-changed`).
//!
//! Le cas d'usage survit aux rechargements de l'assemblage (ajout d'une base), comme les
//! coffres ouverts : il est géré à part par Tauri ([`SharedAutoLock`]).

use crate::events::notify_vaults_changed;
use crate::plain_copies;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tq_application::use_cases::auto_lock_vaults::AutoLockVaults;

/// Intervalle entre deux vérifications.
const CHECK_INTERVAL: Duration = Duration::from_secs(20);

/// Le verrouillage automatique, partagé par le minuteur et les commandes (`State`).
pub struct SharedAutoLock(pub Arc<AutoLockVaults>);

/// Lance le minuteur pour toute la vie de l'application.
pub fn start(app: AppHandle, auto_lock: Arc<AutoLockVaults>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(CHECK_INTERVAL);
            if !auto_lock.execute().is_empty() {
                after_vaults_locked(&app);
            }
        }
    });
}

/// Après le verrouillage d'un coffre (à la main ou automatique) : copies déchiffrées effacées,
/// fenêtres prévenues.
pub fn after_vaults_locked(app: &AppHandle) {
    plain_copies::clear();
    notify_vaults_changed(app);
}
