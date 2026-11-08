//! # Adaptateur « système »
//!
//! **Place dans l'hexagone** : adaptateur secondaire. Implémente les ports qui dépendent de la
//! machine :
//! - [`HybridClock`] → port `Clock` : l'heure, sans jamais reculer ;
//! - [`UuidV7Generator`] → port `IdGenerator` : identifiants uniques triables par date ;
//! - [`machine_device_id`] : l'identité de cet appareil, tirée du nom de la machine ;
//! - [`detect_drive_folders`] : dossiers de drive présents (choix du dossier de données, ADR-0008) ;
//! - [`classify_folder`] : type de stockage d'un dossier de base (local, externe, réseau, cloud).

mod clock;
mod device;
mod drive_folders;
mod storage_kind;
mod uuid_v7;

pub use clock::HybridClock;
pub use device::machine_device_id;
pub use drive_folders::{DriveFolder, DriveProvider, detect_drive_folders, detect_in};
pub use storage_kind::{StorageKind, classify_folder};
pub use uuid_v7::UuidV7Generator;

/// Millisecondes écoulées depuis l'époque Unix selon l'horloge du système.
fn system_millis() -> i64 {
    // [rust] `duration_since` échoue si l'horloge système est avant 1970 (machine mal réglée) :
    //        on se rabat alors sur 0 plutôt que de planter.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}
