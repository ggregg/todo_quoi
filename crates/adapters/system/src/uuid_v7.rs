//! Identifiants UUID version 7 (RFC 9562), écrits à la main (~40 lignes, ADR-0006).
//!
//! Disposition des 128 bits :
//! ```text
//!  48 bits  horodatage (ms depuis 1970)   → les identifiants se trient par date de création
//!   4 bits  version = 7
//!  12 bits  aléa « a »
//!   2 bits  variante = 0b10
//!  62 bits  aléa « b »
//! ```
//!
//! Le hasard vient de `RandomState` de la bibliothèque standard : il sert normalement à
//! protéger les `HashMap`, et chaque instance reçoit des clés aléatoires fournies par l'OS.
//! Ce n'est pas un générateur cryptographique, mais un identifiant n'a besoin que d'être
//! **unique**, pas secret : 74 bits d'aléa + l'horodatage + un compteur suffisent largement.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use tq_application::ports::IdGenerator;
use tq_domain::TaskId;

/// Générateur d'identifiants UUID v7.
#[derive(Default)]
pub struct UuidV7Generator {
    counter: AtomicU64,
}

impl UuidV7Generator {
    /// Nouveau générateur.
    pub fn new() -> Self {
        Self::default()
    }

    fn random_u64(&self) -> u64 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(self.counter.fetch_add(1, Ordering::Relaxed));
        hasher.write_i64(crate::system_millis());
        hasher.finish()
    }
}

impl IdGenerator for UuidV7Generator {
    fn new_task_id(&self) -> TaskId {
        let millis = u64::try_from(crate::system_millis()).unwrap_or(0);
        uuid_v7(millis, self.random_u64(), self.random_u64())
    }
}

/// Assemble un UUID v7 à partir de l'horodatage et de deux nombres aléatoires.
fn uuid_v7(millis: u64, random_a: u64, random_b: u64) -> TaskId {
    let mut bytes = [0_u8; 16];
    // [rust] `to_be_bytes` : l'entier en octets, poids fort d'abord (« big-endian »).
    bytes[..6].copy_from_slice(&millis.to_be_bytes()[2..8]); // 48 bits de poids faible
    bytes[6] = 0x70 | (random_a >> 8 & 0x0F) as u8; // version 7 + 4 bits d'aléa
    bytes[7] = random_a as u8; // 8 bits d'aléa (troncature voulue)
    bytes[8] = 0x80 | (random_b >> 56 & 0x3F) as u8; // variante 0b10 + 6 bits d'aléa
    bytes[9..].copy_from_slice(&random_b.to_be_bytes()[1..8]); // 56 bits d'aléa
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let text = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    match TaskId::parse(&text) {
        Ok(id) => id,
        // Impossible : 32 chiffres hexadécimaux et 4 tirets aux bonnes places.
        Err(error) => unreachable!("UUID v7 mal formé ({error}) : {text}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn layout_follows_rfc_9562() {
        // 0x0195F3C28E4A ms, aléas connus : on vérifie version, variante et horodatage.
        let id = uuid_v7(0x0195_F3C2_8E4A, 0xFFFF, u64::MAX);
        assert_eq!(id.as_str(), "0195f3c2-8e4a-7fff-bfff-ffffffffffff");
        let id = uuid_v7(1, 0, 0);
        assert_eq!(id.as_str(), "00000000-0001-7000-8000-000000000000");
    }

    #[test]
    fn generated_ids_are_unique_version_7_and_time_ordered_by_prefix() {
        let generator = UuidV7Generator::new();
        let ids: Vec<TaskId> = (0..10_000).map(|_| generator.new_task_id()).collect();
        let unique: HashSet<&TaskId> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
        for id in &ids {
            let text = id.as_str();
            assert_eq!(&text[14..15], "7", "version : {text}");
            assert!("89ab".contains(&text[19..20]), "variante : {text}");
        }
        // Les 48 premiers bits (horodatage) ne décroissent jamais.
        assert!(
            ids.windows(2)
                .all(|pair| pair[0].as_str()[..13] <= pair[1].as_str()[..13])
        );
    }
}
