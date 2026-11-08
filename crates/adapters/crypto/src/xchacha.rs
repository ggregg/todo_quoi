//! Port `Cipher` : XChaCha20-Poly1305.
//!
//! Format d'un texte chiffré : `xc20p1:` suivi, en hexadécimal, du nonce (24 octets) puis du
//! chiffré et de son étiquette d'authentification (16 octets). Le préfixe permettra de changer
//! d'algorithme un jour sans ambiguïté.

use crate::hex;
use crate::random::random_bytes;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use tq_application::ports::Cipher;
use tq_application::vault::{ContentKey, VaultError};

const PREFIX: &str = "xc20p1:";
const NONCE_LENGTH: usize = 24;

/// Chiffrement authentifié XChaCha20-Poly1305.
#[derive(Debug, Default, Clone, Copy)]
pub struct XChaChaCipher;

impl XChaChaCipher {
    /// Le chiffrement (sans état : la clé est passée à chaque appel).
    pub fn new() -> Self {
        Self
    }
}

impl Cipher for XChaChaCipher {
    fn seal(
        &self,
        key: &ContentKey,
        plaintext: &[u8],
        associated: &[u8],
    ) -> Result<String, VaultError> {
        seal_with(key.expose(), plaintext, associated)
    }

    fn open(
        &self,
        key: &ContentKey,
        sealed: &str,
        associated: &[u8],
    ) -> Result<Vec<u8>, VaultError> {
        open_with(key.expose(), sealed, associated)
    }
}

/// Chiffre avec une clé brute de 32 octets (sert aussi à envelopper la clé d'un coffre).
pub(crate) fn seal_with(
    key: &[u8],
    plaintext: &[u8],
    associated: &[u8],
) -> Result<String, VaultError> {
    let cipher = cipher_for(key)?;
    // Un nonce aléatoire de 192 bits par chiffrement : la probabilité d'en tirer deux fois le
    // même est négligeable (c'est l'intérêt de la variante « X »).
    let nonce = XNonce::from(random_bytes::<NONCE_LENGTH>()?);
    let payload = Payload {
        msg: plaintext,
        aad: associated,
    };
    let ciphertext = cipher
        .encrypt(&nonce, payload)
        .map_err(|_| VaultError::Crypto("chiffrement impossible".into()))?;
    let mut bytes = nonce.to_vec();
    bytes.extend(ciphertext);
    Ok(format!("{PREFIX}{}", hex::encode(&bytes)))
}

/// Déchiffre avec une clé brute de 32 octets.
pub(crate) fn open_with(
    key: &[u8],
    sealed: &str,
    associated: &[u8],
) -> Result<Vec<u8>, VaultError> {
    let body = sealed
        .trim()
        .strip_prefix(PREFIX)
        .ok_or_else(|| VaultError::Crypto("format de chiffrement inconnu".into()))?;
    let bytes = hex::decode(body)?;
    if bytes.len() < NONCE_LENGTH {
        return Err(VaultError::Crypto("texte chiffré tronqué".into()));
    }
    let (nonce, ciphertext) = bytes.split_at(NONCE_LENGTH);
    let nonce = XNonce::try_from(nonce).map_err(|_| VaultError::Crypto("nonce invalide".into()))?;
    let payload = Payload {
        msg: ciphertext,
        aad: associated,
    };
    // [rust] L'erreur d'AEAD ne dit volontairement rien (mauvaise clé ? texte modifié ?) :
    //        donner la différence aiderait un attaquant.
    cipher_for(key)?
        .decrypt(&nonce, payload)
        .map_err(|_| VaultError::Crypto("déchiffrement refusé (clé ou données)".into()))
}

fn cipher_for(key: &[u8]) -> Result<XChaCha20Poly1305, VaultError> {
    XChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| VaultError::Crypto("clé de taille invalide".into()))
}
