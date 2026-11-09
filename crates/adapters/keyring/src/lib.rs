//! # Trousseau de l'OS (spec §9, ADR-0010)
//!
//! **Place dans l'hexagone** : adaptateur secondaire. Implémente le port `KeyVault` : garder la
//! clé d'un coffre déverrouillé quand l'utilisateur coche « Se souvenir sur cet appareil » —
//! dans le **Trousseau** macOS (trousseau de session) ou le **Gestionnaire d'identification**
//! Windows. Une entrée par coffre : service `TodoQuoi`, compte `vault-<key_id>`.
//!
//! Le magasin est choisi à la compilation (`#[cfg]`). **Sans trousseau**, le port ne garde rien
//! et ne retrouve rien (« Se souvenir » est alors sans effet, jamais une erreur) : c'est le cas
//! sous Linux (jusqu'en I7) et **en développement** — un binaire recompilé a une autre signature,
//! et macOS redemanderait l'accès au trousseau à chaque relance. Pour l'essayer quand même en
//! développement : variable d'environnement [`DEV_KEYCHAIN_VARIABLE`] (`TODOQUOI_KEYCHAIN=1`).

use keyring_core::{CredentialStore, Entry, Error};
use std::sync::Arc;
use tq_application::ports::KeyVault;
use tq_application::vault::{ContentKey, VaultError};
use tq_domain::KeyId;

const SERVICE: &str = "TodoQuoi";

/// Variable d'environnement qui active le trousseau dans une build de développement.
pub const DEV_KEYCHAIN_VARIABLE: &str = "TODOQUOI_KEYCHAIN";

/// Port `KeyVault` sur le trousseau du système.
pub struct OsKeyVault {
    /// `None` : pas de trousseau pris en charge sur ce système.
    store: Option<Arc<CredentialStore>>,
}

impl OsKeyVault {
    /// Le trousseau de ce système (Trousseau macOS, Gestionnaire d'identification Windows) —
    /// sauf en développement, où il faut le demander (voir l'en-tête du module).
    pub fn system() -> Self {
        // [rust] `cfg!(debug_assertions)` : vrai dans une build de développement (`cargo run`,
        //        `pnpm dev`), faux en build de production (`--release`). Évalué à la compilation.
        let in_development = cfg!(debug_assertions);
        if in_development && std::env::var_os(DEV_KEYCHAIN_VARIABLE).is_none() {
            return Self { store: None };
        }
        Self {
            store: system_store(),
        }
    }

    /// Un magasin donné (tests : magasin en mémoire de `keyring_core::mock`).
    pub fn with_store(store: Arc<CredentialStore>) -> Self {
        Self { store: Some(store) }
    }

    /// Entrée du coffre `key_id` ; `None` sans trousseau.
    fn entry(&self, key_id: &KeyId) -> Result<Option<Entry>, VaultError> {
        let Some(store) = &self.store else {
            return Ok(None);
        };
        store
            .build(SERVICE, &format!("vault-{key_id}"), None)
            .map(Some)
            .map_err(keyring_error)
    }
}

impl KeyVault for OsKeyVault {
    /// Sans trousseau : rien n'est gardé (le coffre s'ouvre quand même).
    fn remember(&self, key_id: &KeyId, key: &ContentKey) -> Result<(), VaultError> {
        let Some(entry) = self.entry(key_id)? else {
            return Ok(());
        };
        entry.set_secret(key.expose()).map_err(keyring_error)
    }

    fn recall(&self, key_id: &KeyId) -> Result<Option<ContentKey>, VaultError> {
        let Some(entry) = self.entry(key_id)? else {
            return Ok(None);
        };
        match entry.get_secret() {
            Ok(bytes) => ContentKey::from_bytes(bytes).map(Some),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(keyring_error(error)),
        }
    }

    fn forget(&self, key_id: &KeyId) -> Result<(), VaultError> {
        let Some(entry) = self.entry(key_id)? else {
            return Ok(());
        };
        match entry.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(keyring_error(error)),
        }
    }
}

fn keyring_error(error: Error) -> VaultError {
    VaultError::Keyring(error.to_string())
}

#[cfg(target_os = "macos")]
fn system_store() -> Option<Arc<CredentialStore>> {
    // [rust] `as Arc<CredentialStore>` : on « oublie » le type précis du magasin pour ne garder
    //        que son interface (≈ affecter une classe à une variable typée par son interface).
    apple_native_keyring_store::keychain::Store::new()
        .ok()
        .map(|store| store as Arc<CredentialStore>)
}

#[cfg(target_os = "windows")]
fn system_store() -> Option<Arc<CredentialStore>> {
    windows_native_keyring_store::Store::new()
        .ok()
        .map(|store| store as Arc<CredentialStore>)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn system_store() -> Option<Arc<CredentialStore>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> OsKeyVault {
        OsKeyVault::with_store(keyring_core::mock::Store::new().unwrap())
    }

    fn key_id() -> KeyId {
        KeyId::parse("0123456789abcdef0123456789abcdef").unwrap()
    }

    #[test]
    fn remember_recall_forget() {
        let vault = vault();
        assert!(vault.recall(&key_id()).unwrap().is_none());
        let key = ContentKey::from_bytes(vec![9; 32]).unwrap();
        vault.remember(&key_id(), &key).unwrap();
        assert_eq!(
            vault.recall(&key_id()).unwrap().unwrap().expose(),
            key.expose()
        );
        vault.forget(&key_id()).unwrap();
        assert!(vault.recall(&key_id()).unwrap().is_none());
        vault.forget(&key_id()).unwrap(); // déjà oubliée : pas une erreur
    }

    #[test]
    fn without_a_keyring_nothing_is_remembered() {
        let vault = OsKeyVault { store: None };
        let key = ContentKey::from_bytes(vec![9; 32]).unwrap();
        assert!(
            vault.remember(&key_id(), &key).is_ok(),
            "sans effet, pas une erreur"
        );
        assert!(vault.recall(&key_id()).unwrap().is_none());
        assert!(vault.forget(&key_id()).is_ok());
    }
}
