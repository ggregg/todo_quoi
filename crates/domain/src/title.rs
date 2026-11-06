//! Titre d'une tâche.

use crate::DomainError;
use std::fmt;

/// Titre d'une tâche : une seule ligne, non vide, 300 caractères au plus.
///
/// Un `Title` existant est **toujours valide** : on ne peut en obtenir un qu'en passant par
/// [`Title::parse`]. Le reste du code n'a donc jamais à revérifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Title(String);

impl Title {
    /// Longueur maximale, en caractères.
    pub const MAX_CHARS: usize = 300;

    /// Valide un titre ; les espaces de début et de fin sont retirés.
    ///
    /// # Erreurs
    /// [`DomainError::EmptyTitle`], [`DomainError::MultilineTitle`] ou
    /// [`DomainError::TitleTooLong`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        if trimmed.contains(['\n', '\r']) {
            return Err(DomainError::MultilineTitle);
        }
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(DomainError::TitleTooLong {
                max: Self::MAX_CHARS,
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Le texte du titre.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_trimmed() {
        assert_eq!(
            Title::parse("  Préparer la release  ").unwrap().as_str(),
            "Préparer la release"
        );
    }

    #[test]
    fn invalid_titles_are_rejected() {
        assert_eq!(Title::parse("   "), Err(DomainError::EmptyTitle));
        assert_eq!(Title::parse("a\nb"), Err(DomainError::MultilineTitle));
        assert_eq!(
            Title::parse(&"é".repeat(301)),
            Err(DomainError::TitleTooLong { max: 300 })
        );
        assert!(Title::parse(&"é".repeat(300)).is_ok());
    }
}
