//! Les coffres déverrouillés pendant cette session, **en mémoire seulement** : quitter
//! l'application (ou verrouiller) les referme.

use crate::vault::ContentKey;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use tq_domain::{BaseId, KeyId};

/// État du coffre d'une base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultState {
    /// Pas encore de coffre (rien n'a été chiffré dans cette base).
    None,
    /// Coffre verrouillé.
    Locked,
    /// Coffre ouvert.
    Unlocked,
}

/// Clés des coffres ouverts, par base. Partagé (`Arc`) entre les cas d'usage du coffre et le
/// stockage de chaque base ([`crate::vault::BaseSealer`]).
#[derive(Default)]
pub struct OpenVaults {
    keys: Mutex<HashMap<BaseId, (KeyId, ContentKey)>>,
}

impl OpenVaults {
    /// Aucun coffre ouvert.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ouvre le coffre `key_id` de `base` avec sa clé.
    pub fn open(&self, base: BaseId, key_id: KeyId, key: ContentKey) {
        self.lock_map().insert(base, (key_id, key));
    }

    /// Referme le coffre de `base` (la clé est effacée de la mémoire).
    pub fn close(&self, base: &BaseId) {
        self.lock_map().remove(base);
    }

    /// Clé du coffre ouvert de `base`, avec son identifiant.
    pub fn key_of(&self, base: &BaseId) -> Option<(KeyId, ContentKey)> {
        self.lock_map().get(base).cloned()
    }

    /// Vrai si le coffre de `base` est ouvert.
    pub fn is_open(&self, base: &BaseId) -> bool {
        self.lock_map().contains_key(base)
    }

    // [rust] Un `Mutex` « empoisonné » (un fil a paniqué en le tenant) est repris tel quel :
    //        la table reste cohérente, chaque opération y est atomique.
    fn lock_map(&self) -> MutexGuard<'_, HashMap<BaseId, (KeyId, ContentKey)>> {
        self.keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
