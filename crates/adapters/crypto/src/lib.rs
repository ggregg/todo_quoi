//! # Chiffrement (spec §9, ADR-0010)
//!
//! **Place dans l'hexagone** : adaptateur secondaire. Implémente deux ports de l'application :
//! - `VaultKeys` ([`Argon2VaultKeys`]) : créer un coffre et retrouver sa clé à partir du mot de
//!   passe — dérivation **Argon2id** (64 Mio, 3 passes), clé de contenu aléatoire de 256 bits
//!   enveloppée par la clé dérivée ;
//! - `Cipher` ([`XChaChaCipher`]) : chiffrement authentifié **XChaCha20-Poly1305** (nonce
//!   aléatoire de 192 bits, sans risque de répétition), et nom des médias chiffrés
//!   (**HMAC-SHA-256** avec une sous-clé du coffre, [`fingerprint`]).
//!
//! **On n'écrit pas de crypto** (architecture §16) : tout le calcul vient des crates RustCrypto.
//! Ce module ne fait qu'assembler, et coder les résultats en texte ([`hex`]) pour les fichiers.

mod argon2_keys;
mod fingerprint;
mod hex;
mod random;
mod xchacha;

pub use argon2_keys::{Argon2VaultKeys, DEFAULT_KDF};
pub use xchacha::XChaChaCipher;
