//! Confidentialité d'une tâche : en clair, chiffrée, ou **scellée** (chiffrée et coffre
//! verrouillé) — spécification §9, ADR-0010.
//!
//! **Seule la note est chiffrée** (choix du mainteneur, ADR-0010) : titre, étiquettes, vue,
//! statut et place restent en clair, visibles et modifiables même coffre verrouillé.
//!
//! Le domaine ne chiffre rien (il n'a ni clé ni algorithme) : il dit seulement **ce qui est
//! permis** dans chaque état. La note d'une tâche scellée est illisible et ne peut pas être
//! remplacée — sinon on écraserait la vraie note par une note vide.

use crate::DomainError;

/// Identifiant du coffre (donc de la clé) qui a chiffré une tâche : 32 caractères
/// hexadécimaux minuscules, tirés au hasard à la création du coffre.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(String);

impl KeyId {
    /// Longueur en caractères hexadécimaux (128 bits).
    pub const LENGTH: usize = 32;

    /// Valide un identifiant de coffre.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidKeyId`] si ce n'est pas 32 caractères `0-9a-f`.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let valid = text.len() == Self::LENGTH
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
        if valid {
            Ok(Self(text.to_string()))
        } else {
            Err(DomainError::InvalidKeyId(text.to_string()))
        }
    }

    /// Texte de l'identifiant.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Note chiffrée d'une tâche, gardée **telle quelle** tant que le coffre est verrouillé : il est
/// réécrit à l'identique si l'on clôt ou déplace la tâche sans la déverrouiller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedContent {
    /// Coffre qui l'a chiffré.
    pub key_id: KeyId,
    /// Texte chiffré (format choisi par l'adaptateur de chiffrement), opaque pour le domaine.
    pub sealed: String,
}

/// Confidentialité d'une tâche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confidentiality {
    /// En clair.
    Plain,
    /// Note chiffrée sur le disque ; lisible ici (coffre de sa base déverrouillé).
    Encrypted,
    /// Chiffrée, coffre verrouillé : note illisible.
    Sealed(SealedContent),
}

impl Confidentiality {
    /// Vrai si la tâche est écrite chiffrée (déverrouillée ou non).
    pub fn is_encrypted(&self) -> bool {
        !matches!(self, Confidentiality::Plain)
    }

    /// Vrai si sa note est illisible (coffre verrouillé).
    pub fn is_sealed(&self) -> bool {
        matches!(self, Confidentiality::Sealed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_ids_are_32_lowercase_hex_characters() {
        let cases = [
            ("0123456789abcdef0123456789abcdef", true),
            ("0123456789ABCDEF0123456789ABCDEF", false),
            ("0123456789abcdef", false),
            ("0123456789abcdef0123456789abcdeg", false),
            ("", false),
        ];
        for (text, valid) in cases {
            assert_eq!(KeyId::parse(text).is_ok(), valid, "{text}");
        }
    }

    #[test]
    fn only_sealed_content_is_unreadable() {
        let sealed = Confidentiality::Sealed(SealedContent {
            key_id: KeyId::parse("0123456789abcdef0123456789abcdef").unwrap(),
            sealed: "…".to_string(),
        });
        assert!(sealed.is_sealed() && sealed.is_encrypted());
        assert!(Confidentiality::Encrypted.is_encrypted());
        assert!(!Confidentiality::Encrypted.is_sealed());
        assert!(!Confidentiality::Plain.is_encrypted());
    }
}
