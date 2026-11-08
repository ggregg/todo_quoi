//! Aléa cryptographique du système (via `getrandom`, tiré par RustCrypto).

use chacha20poly1305::aead::Generate;
use tq_application::vault::VaultError;

/// `N` octets aléatoires.
///
/// # Erreurs
/// [`VaultError::Crypto`] si le système ne fournit pas d'aléa (ne devrait pas arriver).
// [rust] `const N: usize` : un paramètre générique qui est une **valeur** (la taille du tableau),
//        connue à la compilation — pas d'équivalent direct en TS.
pub(crate) fn random_bytes<const N: usize>() -> Result<[u8; N], VaultError> {
    <[u8; N]>::try_generate().map_err(|error| VaultError::Crypto(format!("aléa : {error}")))
}
