//! Port `VaultKeys` : créer un coffre, retrouver sa clé à partir du mot de passe.
//!
//! ```text
//! mot de passe ──Argon2id(sel)──▶ clé d'enveloppe ──XChaCha20-Poly1305──▶ clé de contenu
//! ```
//! - La **clé de contenu** (aléatoire, 256 bits) chiffre les tâches ; elle ne change jamais.
//! - Seule son **enveloppe** dépend du mot de passe : changer de mot de passe = ré-envelopper,
//!   sans rechiffrer les tâches.
//! - Un mauvais mot de passe donne une autre clé d'enveloppe, dont l'authentification échoue :
//!   on le sait sans stocker de condensat du mot de passe.

use crate::hex;
use crate::random::random_bytes;
use crate::xchacha::{open_with, seal_with};
use argon2::{Algorithm, Argon2, Params, Version};
use tq_application::ports::{KdfParams, VaultKeys, VaultRecord};
use tq_application::vault::{ContentKey, Password, VaultError};
use tq_domain::KeyId;
use zeroize::Zeroizing;

/// Paramètres des nouveaux coffres (architecture §7) : 64 Mio, 3 passes, 1 fil.
pub const DEFAULT_KDF: KdfParams = KdfParams {
    memory_kib: 64 * 1024,
    iterations: 3,
    parallelism: 1,
};

const SALT_LENGTH: usize = 16;
/// Identifiant d'un coffre : 16 octets aléatoires (32 chiffres hexadécimaux).
const KEY_ID_LENGTH: usize = 16;
/// Clé d'enveloppe dérivée du mot de passe : 32 octets (XChaCha20-Poly1305).
const WRAPPING_KEY_LENGTH: usize = 32;

/// Dérivation Argon2id et enveloppe de la clé de contenu.
#[derive(Debug, Clone, Copy)]
pub struct Argon2VaultKeys {
    kdf: KdfParams,
}

impl Argon2VaultKeys {
    /// Avec les paramètres de production ([`DEFAULT_KDF`]).
    pub fn new() -> Self {
        Self { kdf: DEFAULT_KDF }
    }

    /// Avec d'autres paramètres pour les **nouveaux** coffres (tests : dérivation rapide). Un
    /// coffre existant s'ouvre toujours avec les paramètres écrits dans son manifeste.
    pub fn with_params(kdf: KdfParams) -> Self {
        Self { kdf }
    }
}

impl Default for Argon2VaultKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultKeys for Argon2VaultKeys {
    fn create(&self, password: &Password) -> Result<(VaultRecord, ContentKey), VaultError> {
        // [rust] `random_bytes::<16>()` : on précise le paramètre générique à l'appel (« turbofish »,
        //        ≈ `randomBytes<16>()` en TS) ; une expression (`ContentKey::LENGTH`) va entre `{ }`.
        let key_id = KeyId::parse(&hex::encode(&random_bytes::<KEY_ID_LENGTH>()?))
            .map_err(|error| VaultError::Crypto(error.to_string()))?;
        let key = ContentKey::from_bytes(random_bytes::<{ ContentKey::LENGTH }>()?.to_vec())?;
        let record = self.wrap(key_id, &key, password)?;
        Ok((record, key))
    }

    fn unlock(&self, record: &VaultRecord, password: &Password) -> Result<ContentKey, VaultError> {
        let salt = hex::decode(&record.salt)?;
        let wrapping = derive(password, &salt, record.kdf)?;
        // [rust] `&wrapping[..]` : une tranche (`&[u8]`) sur tout le tableau, ce qu'attend
        //        `open_with` (≈ passer un `Uint8Array` entier).
        let key = open_with(
            &wrapping[..],
            &record.wrapped_key,
            &wrap_context(&record.key_id),
        )
        .map_err(|_| VaultError::WrongPassword)?;
        ContentKey::from_bytes(key)
    }

    /// Nouveau sel, paramètres actuels de l'application (un coffre ancien profite des
    /// paramètres plus forts au changement de mot de passe), même identifiant.
    fn rewrap(
        &self,
        record: &VaultRecord,
        key: &ContentKey,
        password: &Password,
    ) -> Result<VaultRecord, VaultError> {
        self.wrap(record.key_id.clone(), key, password)
    }
}

impl Argon2VaultKeys {
    /// Enveloppe `key` avec `password` : nouveau sel, paramètres de l'application.
    fn wrap(
        &self,
        key_id: KeyId,
        key: &ContentKey,
        password: &Password,
    ) -> Result<VaultRecord, VaultError> {
        let salt = random_bytes::<SALT_LENGTH>()?;
        let wrapping = derive(password, &salt, self.kdf)?;
        let wrapped_key = seal_with(&wrapping[..], key.expose(), &wrap_context(&key_id))?;
        Ok(VaultRecord {
            key_id,
            kdf: self.kdf,
            salt: hex::encode(&salt),
            wrapped_key,
        })
    }
}

/// Clé d'enveloppe dérivée du mot de passe (effacée de la mémoire après usage).
fn derive(
    password: &Password,
    salt: &[u8],
    kdf: KdfParams,
) -> Result<Zeroizing<[u8; WRAPPING_KEY_LENGTH]>, VaultError> {
    let length = Some(WRAPPING_KEY_LENGTH);
    let params = Params::new(kdf.memory_kib, kdf.iterations, kdf.parallelism, length)
        .map_err(|error| VaultError::Crypto(format!("paramètres Argon2 : {error}")))?;
    let mut output = Zeroizing::new([0u8; WRAPPING_KEY_LENGTH]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.expose().as_bytes(), salt, &mut output[..])
        .map_err(|error| VaultError::Crypto(format!("Argon2 : {error}")))?;
    Ok(output)
}

/// Contexte authentifié de l'enveloppe : une enveloppe copiée dans un autre coffre ne s'ouvre pas.
fn wrap_context(key_id: &KeyId) -> Vec<u8> {
    format!("todoquoi-vault:{key_id}").into_bytes()
}
