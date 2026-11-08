//! Hexadécimal ↔ octets, pour écrire du binaire dans des fichiers texte (manifeste, tâches).
//!
//! Écrit à la main (≈ 20 lignes, testé) plutôt qu'une dépendance : ce n'est pas de la crypto,
//! seulement un codage. Le texte chiffré est deux fois plus long qu'en binaire, ce qui reste
//! négligeable pour des notes (quelques Ko).

use tq_application::vault::VaultError;

const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Octets → texte hexadécimal minuscule.
pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// Texte hexadécimal → octets. Les blancs (retours à la ligne d'un fichier) sont ignorés.
///
/// # Erreurs
/// [`VaultError::Crypto`] si un caractère n'est pas hexadécimal ou si la longueur est impaire.
pub(crate) fn decode(text: &str) -> Result<Vec<u8>, VaultError> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if !digits.len().is_multiple_of(2) {
        return Err(VaultError::Crypto("texte hexadécimal tronqué".into()));
    }
    digits
        .chunks(2)
        .map(|pair| Ok(value(pair[0])? << 4 | value(pair[1])?))
        .collect()
}

fn value(digit: u8) -> Result<u8, VaultError> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'a'..=b'f' => Ok(digit - b'a' + 10),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err(VaultError::Crypto("caractère non hexadécimal".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_of_every_byte() {
        let bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&bytes)).unwrap(), bytes);
    }

    #[test]
    fn known_values_and_tolerances() {
        assert_eq!(encode(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
        assert_eq!(decode("00 0F\na5ff").unwrap(), vec![0x00, 0x0f, 0xa5, 0xff]);
        assert!(decode("abc").is_err());
        assert!(decode("zz").is_err());
        assert_eq!(decode("").unwrap(), Vec::<u8>::new());
    }
}
