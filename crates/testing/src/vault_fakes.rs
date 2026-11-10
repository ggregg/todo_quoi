//! Faux ports du coffre (ADR-0010) : chiffrement **simulé**, trousseau et manifeste en mémoire.
//!
//! [`FakeCipher`] ne protège rien (le texte reste lisible en hexadécimal) : il vérifie seulement
//! qu'on déchiffre avec **la même clé** et **le même contexte**, ce qui suffit à tester les cas
//! d'usage et le stockage. Le vrai chiffrement est testé dans `adapters/crypto`.

use std::collections::HashMap;
use std::sync::Mutex;
use tq_application::ports::{
    Cipher, KdfParams, KeyringStore, RepositoryError, VaultKeys, VaultRecord, VaultStore,
};
use tq_application::vault::{ContentKey, Password, VaultError};
use tq_domain::{BaseId, KeyId};

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Création / ouverture de coffre simulées : la « clé enveloppée » est le mot de passe en
/// hexadécimal suivi de la clé ; chaque coffre reçoit une clé et un identifiant différents.
#[derive(Default)]
pub struct FakeVaultKeys {
    created: Mutex<u8>,
}

impl VaultKeys for FakeVaultKeys {
    fn create(&self, password: &Password) -> Result<(VaultRecord, ContentKey), VaultError> {
        let mut created = lock(&self.created);
        *created += 1;
        let number = *created;
        let key = ContentKey::from_bytes(vec![number; ContentKey::LENGTH])?;
        let key_id = KeyId::parse(&format!("{number:032x}"))
            .map_err(|error| VaultError::Crypto(error.to_string()))?;
        let record = VaultRecord {
            key_id,
            kdf: KdfParams {
                memory_kib: 8,
                iterations: 1,
                parallelism: 1,
            },
            salt: "00".into(),
            wrapped_key: format!(
                "{}:{}",
                hex(password.expose().as_bytes()),
                hex(key.expose())
            ),
        };
        Ok((record, key))
    }

    fn unlock(&self, record: &VaultRecord, password: &Password) -> Result<ContentKey, VaultError> {
        let (expected, key) = record
            .wrapped_key
            .split_once(':')
            .ok_or_else(|| VaultError::Crypto("enveloppe".into()))?;
        if expected != hex(password.expose().as_bytes()) {
            return Err(VaultError::WrongPassword);
        }
        ContentKey::from_bytes(unhex(key).ok_or_else(|| VaultError::Crypto("clé".into()))?)
    }

    fn rewrap(
        &self,
        record: &VaultRecord,
        key: &ContentKey,
        password: &Password,
    ) -> Result<VaultRecord, VaultError> {
        Ok(VaultRecord {
            wrapped_key: format!(
                "{}:{}",
                hex(password.expose().as_bytes()),
                hex(key.expose())
            ),
            ..record.clone()
        })
    }
}

/// Chiffrement simulé : `fake:<1er octet de la clé>:<contexte>:<texte>` en hexadécimal.
#[derive(Default)]
pub struct FakeCipher;

impl Cipher for FakeCipher {
    fn seal(
        &self,
        key: &ContentKey,
        plaintext: &[u8],
        associated: &[u8],
    ) -> Result<String, VaultError> {
        Ok(format!(
            "fake:{:02x}:{}:{}",
            key.expose()[0],
            hex(associated),
            hex(plaintext)
        ))
    }

    fn open(
        &self,
        key: &ContentKey,
        sealed: &str,
        associated: &[u8],
    ) -> Result<Vec<u8>, VaultError> {
        let refused = || VaultError::Crypto("déchiffrement refusé".into());
        let parts: Vec<&str> = sealed.split(':').collect();
        let [prefix, key_tag, context, text] = parts[..] else {
            return Err(refused());
        };
        let same_key = key_tag == format!("{:02x}", key.expose()[0]);
        if prefix != "fake" || !same_key || context != hex(associated) {
            return Err(refused());
        }
        unhex(text).ok_or_else(refused)
    }

    fn seal_bytes(
        &self,
        key: &ContentKey,
        plaintext: &[u8],
        associated: &[u8],
    ) -> Result<Vec<u8>, VaultError> {
        Ok(self.seal(key, plaintext, associated)?.into_bytes())
    }

    fn open_bytes(
        &self,
        key: &ContentKey,
        sealed: &[u8],
        associated: &[u8],
    ) -> Result<Vec<u8>, VaultError> {
        let text = String::from_utf8(sealed.to_vec())
            .map_err(|_| VaultError::Crypto("déchiffrement refusé".into()))?;
        self.open(key, &text, associated)
    }

    /// Empreinte simulée : dépend de la clé et du contenu (pas de sécurité).
    fn fingerprint(&self, key: &ContentKey, content: &[u8]) -> Result<String, VaultError> {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.expose().hash(&mut hasher);
        content.hash(&mut hasher);
        Ok(format!("{:016x}", hasher.finish()).repeat(4))
    }
}

/// Coffres des bases, en mémoire.
#[derive(Default)]
pub struct InMemoryVaultStore {
    vaults: Mutex<HashMap<BaseId, VaultRecord>>,
}

impl VaultStore for InMemoryVaultStore {
    fn load_vault(&self, base: &BaseId) -> Result<Option<VaultRecord>, RepositoryError> {
        Ok(lock(&self.vaults).get(base).cloned())
    }

    fn save_vault(&self, base: &BaseId, vault: &VaultRecord) -> Result<(), RepositoryError> {
        lock(&self.vaults).insert(base.clone(), vault.clone());
        Ok(())
    }
}

/// Trousseau en mémoire.
#[derive(Default)]
pub struct InMemoryKeyringStore {
    keys: Mutex<HashMap<KeyId, ContentKey>>,
}

impl InMemoryKeyringStore {
    /// Vrai si une clé est gardée pour ce coffre.
    pub fn remembers(&self, key_id: &KeyId) -> bool {
        lock(&self.keys).contains_key(key_id)
    }
}

impl KeyringStore for InMemoryKeyringStore {
    fn remember(&self, key_id: &KeyId, key: &ContentKey) -> Result<(), VaultError> {
        lock(&self.keys).insert(key_id.clone(), key.clone());
        Ok(())
    }

    fn recall(&self, key_id: &KeyId) -> Result<Option<ContentKey>, VaultError> {
        Ok(lock(&self.keys).get(key_id).cloned())
    }

    fn forget(&self, key_id: &KeyId) -> Result<(), VaultError> {
        lock(&self.keys).remove(key_id);
        Ok(())
    }
}
