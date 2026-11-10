//! **Couche application** : secrets manipulés par l'application : le mot de passe saisi et la clé d'un coffre ouvert.
//!
//! Tous deux sont enveloppés dans `zeroize::Zeroizing` : leur mémoire est remplie de zéros
//! quand ils sont libérés, pour qu'ils ne traînent pas en mémoire (spec §9). Leur `Debug`
//! n'affiche jamais le contenu (un secret ne doit pas finir dans un journal).

use crate::vault::VaultError;
use zeroize::Zeroizing;

/// Longueur minimale d'un mot de passe de coffre, en caractères.
pub const MIN_PASSWORD_CHARS: usize = 8;

/// Mot de passe d'un coffre.
// [rust] `Zeroizing<String>` se comporte comme un `String` (`&self.0` donne le texte, par
//        déréférencement automatique), mais
//        efface son contenu à la libération (`Drop`, ≈ un destructeur appelé automatiquement).
pub struct Password(Zeroizing<String>);

impl Password {
    /// Valide un mot de passe.
    ///
    /// # Erreurs
    /// [`VaultError::PasswordTooShort`] sous [`MIN_PASSWORD_CHARS`] caractères.
    pub fn new(text: String) -> Result<Self, VaultError> {
        let text = Zeroizing::new(text);
        if text.chars().count() < MIN_PASSWORD_CHARS {
            return Err(VaultError::PasswordTooShort {
                min: MIN_PASSWORD_CHARS,
            });
        }
        Ok(Self(text))
    }

    /// Le texte du mot de passe, pour la dérivation de clé.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password(…)")
    }
}

/// Clé de contenu d'un coffre (32 octets), déverrouillée.
#[derive(Clone)]
pub struct ContentKey(Zeroizing<Vec<u8>>);

impl ContentKey {
    /// Taille d'une clé, en octets.
    pub const LENGTH: usize = 32;

    /// Enveloppe des octets de clé.
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`] si la taille n'est pas [`Self::LENGTH`].
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, VaultError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.len() != Self::LENGTH {
            return Err(VaultError::Crypto(format!(
                "clé de {} octets au lieu de {}",
                bytes.len(),
                Self::LENGTH
            )));
        }
        Ok(Self(bytes))
    }

    /// Les octets de la clé (chiffrement, trousseau).
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for ContentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ContentKey(…)")
    }
}

/// Garder la clé dans le trousseau de l'OS après le déverrouillage ? (Une énumération plutôt
/// qu'un booléen : `Remember::OnThisDevice` se lit tout seul.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remember {
    /// Ranger la clé dans le trousseau : le coffre s'ouvrira seul au prochain lancement.
    OnThisDevice,
    /// Ne pas la garder (et oublier celle qui y serait déjà).
    NotOnThisDevice,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_need_eight_characters_counted_as_characters_not_bytes() {
        assert!(Password::new("1234567".into()).is_err());
        assert!(Password::new("12345678".into()).is_ok());
        assert!(Password::new("éééééééé".into()).is_ok());
        assert!(Password::new("ééé".into()).is_err());
    }

    #[test]
    fn secrets_never_show_in_debug_output() {
        let password = Password::new("très secret".into()).unwrap();
        let key = ContentKey::from_bytes(vec![7; 32]).unwrap();
        assert_eq!(format!("{password:?} {key:?}"), "Password(…) ContentKey(…)");
    }

    #[test]
    fn a_content_key_has_32_bytes() {
        assert!(ContentKey::from_bytes(vec![0; 31]).is_err());
        assert!(ContentKey::from_bytes(vec![0; 32]).is_ok());
    }
}
