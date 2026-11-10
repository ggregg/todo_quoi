//! Les coffres déverrouillés pendant cette session, **en mémoire seulement** : quitter
//! l'application (ou verrouiller) les referme.

use crate::vault::{ContentKey, Remember};
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

/// Un coffre ouvert : sa clé, et si l'utilisateur l'a gardée dans le trousseau.
#[derive(Clone)]
struct Opened {
    key_id: KeyId,
    key: ContentKey,
    remember: Remember,
}

/// Clés des coffres ouverts, par base. Partagé (`Arc`) entre les cas d'usage du coffre et le
/// stockage de chaque base ([`crate::vault::BaseSealer`]).
#[derive(Default)]
pub struct OpenVaults {
    opened: Mutex<HashMap<BaseId, Opened>>,
}

impl OpenVaults {
    /// Aucun coffre ouvert.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ouvre le coffre `key_id` de `base` avec sa clé. `remember` : la clé est-elle gardée
    /// dans le trousseau de cet appareil ? (Un tel coffre ne se verrouille pas tout seul.)
    pub fn open(&self, base: BaseId, key_id: KeyId, key: ContentKey, remember: Remember) {
        let opened = Opened {
            key_id,
            key,
            remember,
        };
        self.lock_map().insert(base, opened);
    }

    /// Referme le coffre de `base` (la clé est effacée de la mémoire).
    pub fn close(&self, base: &BaseId) {
        self.lock_map().remove(base);
    }

    /// Referme les coffres dont la clé n'est pas gardée dans le trousseau (verrouillage
    /// automatique) ; renvoie leurs bases.
    pub fn close_unremembered(&self) -> Vec<BaseId> {
        let mut map = self.lock_map();
        let closing: Vec<BaseId> = map
            .iter()
            .filter(|(_, opened)| opened.remember == Remember::NotOnThisDevice)
            .map(|(base, _)| base.clone())
            .collect();
        for base in &closing {
            map.remove(base);
        }
        closing
    }

    /// Clé du coffre ouvert de `base`, avec son identifiant.
    pub fn key_of(&self, base: &BaseId) -> Option<(KeyId, ContentKey)> {
        self.lock_map()
            .get(base)
            .map(|opened| (opened.key_id.clone(), opened.key.clone()))
    }

    /// Clés de tous les coffres ouverts (lire un média chiffré : on ne sait pas de quelle base
    /// il vient, seule la bonne clé l'ouvre).
    pub fn open_keys(&self) -> Vec<ContentKey> {
        self.lock_map()
            .values()
            .map(|opened| opened.key.clone())
            .collect()
    }

    /// Vrai si le coffre de `base` est ouvert.
    pub fn is_open(&self, base: &BaseId) -> bool {
        self.lock_map().contains_key(base)
    }

    // [rust] Un `Mutex` « empoisonné » (un fil a paniqué en le tenant) est repris tel quel :
    //        la table reste cohérente, chaque opération y est atomique.
    fn lock_map(&self) -> MutexGuard<'_, HashMap<BaseId, Opened>> {
        self.opened
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
