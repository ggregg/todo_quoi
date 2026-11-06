//! Étiquette libre attachée à une tâche (`#release`, `#client-x`).

use crate::DomainError;
use std::fmt;

/// Une étiquette : en minuscules, sans espace ni virgule, 1 à 40 caractères.
///
/// La saisie est tolérante : `  #Client-X ` devient `client-x`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag(String);

impl Tag {
    const MAX_CHARS: usize = 40;

    /// Normalise et valide une étiquette saisie.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidTag`] si l'étiquette est vide, trop longue, ou contient un espace
    /// ou une virgule.
    ///
    /// ```
    /// use tq_domain::Tag;
    /// assert_eq!(Tag::parse(" #Client-X ").unwrap().as_str(), "client-x");
    /// assert!(Tag::parse("deux mots").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let normalized = text.trim().trim_start_matches('#').to_lowercase();
        let length = normalized.chars().count();
        let is_valid = (1..=Self::MAX_CHARS).contains(&length)
            && !normalized.chars().any(|c| c.is_whitespace() || c == ',');
        if is_valid {
            Ok(Self(normalized))
        } else {
            Err(DomainError::InvalidTag(text.to_string()))
        }
    }

    /// Le texte de l'étiquette (sans `#`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_are_kept_and_lowercased() {
        assert_eq!(Tag::parse("Équipe").unwrap().as_str(), "équipe");
    }

    #[test]
    fn invalid_tags_are_rejected() {
        for text in ["", "#", "a,b", "a b", &"x".repeat(41)] {
            assert!(Tag::parse(text).is_err(), "devrait être refusé : {text:?}");
        }
    }
}
