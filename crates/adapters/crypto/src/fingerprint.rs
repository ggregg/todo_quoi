//! Empreinte **avec clé** (HMAC-SHA-256) : nom des médias chiffrés (ADR-0010).
//!
//! ```text
//! clé de contenu ──HMAC("todoquoi-media-name")──▶ clé de nommage ──HMAC(contenu)──▶ nom
//! ```
//! Une clé ne sert qu'à un usage : la clé de contenu chiffre, une sous-clé dérivée d'elle
//! nomme. Sans la clé, le nom ne permet pas de reconnaître un fichier (une empreinte SHA-256
//! le permettrait : il suffit de hacher le fichier qu'on soupçonne).

use crate::hex;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use tq_application::vault::{ContentKey, VaultError};
use zeroize::Zeroizing;

/// Étiquette de la sous-clé de nommage.
const NAMING_LABEL: &[u8] = b"todoquoi-media-name";

/// HMAC-SHA-256 de `content` avec la clé de nommage du coffre, en hexadécimal (64 caractères).
///
/// # Erreurs
/// [`VaultError::Crypto`] si la clé est refusée (n'arrive pas : HMAC accepte toute taille).
pub(crate) fn fingerprint(key: &ContentKey, content: &[u8]) -> Result<String, VaultError> {
    let naming_key = Zeroizing::new(hmac_sha256(key.expose(), NAMING_LABEL)?);
    Ok(hex::encode(&hmac_sha256(&naming_key[..], content)?))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Result<Vec<u8>, VaultError> {
    // [rust] `<Hmac<Sha256> as KeyInit>::new_from_slice` : on précise de quel trait vient la
    //        fonction (plusieurs traits en ont une du même nom).
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key)
        .map_err(|_| VaultError::Crypto("clé HMAC refusée".into()))?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha256_matches_rfc_4231_test_case_2() {
        // RFC 4231, cas 2 : clé « Jefe », message « what do ya want for nothing? ».
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?").unwrap();
        assert_eq!(
            hex::encode(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn the_name_depends_on_the_key_and_the_content() {
        let key = ContentKey::from_bytes(vec![1; 32]).unwrap();
        let other = ContentKey::from_bytes(vec![2; 32]).unwrap();
        let name = fingerprint(&key, b"photo").unwrap();
        assert_eq!(name.len(), 64);
        assert_eq!(name, fingerprint(&key, b"photo").unwrap(), "stable");
        assert_ne!(name, fingerprint(&other, b"photo").unwrap());
        assert_ne!(name, fingerprint(&key, b"autre").unwrap());
    }
}
