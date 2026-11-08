//! Base de tâches (ADR-0009) : un dossier `TodoQuoi/` — « perso », « pro »… Une tâche appartient
//! à une base ; le fichier ne l'écrit pas, c'est le dossier qui la porte.

use crate::DomainError;
use std::fmt;

/// Identifiant d'une base, choisi sur l'appareil (ex. `perso`, `pro`).
///
/// Caractères autorisés : lettres minuscules ASCII, chiffres et `-` ; 1 à 40 caractères.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BaseId(String);

impl BaseId {
    const MAX_LEN: usize = 40;
    const MAIN: &'static str = "principale";

    /// Valide un identifiant de base.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidBaseId`] si un caractère n'est pas autorisé ou si la longueur
    /// n'est pas comprise entre 1 et 40.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let is_valid = (1..=Self::MAX_LEN).contains(&text.len())
            && text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if is_valid {
            Ok(Self(text.to_string()))
        } else {
            Err(DomainError::InvalidBaseId(text.to_string()))
        }
    }

    /// Base par défaut, quand une seule base est ouverte (comportement d'avant l'ADR-0009).
    pub fn main() -> Self {
        Self(Self::MAIN.to_string())
    }

    /// Identifiant tiré d'un nom saisi : `Pro – Client X` → `pro-client-x` (vide → `base`).
    pub fn from_name(name: &str) -> Self {
        let mut slug = String::new();
        for c in name.trim().chars() {
            if c.is_ascii_alphanumeric() {
                slug.push(c.to_ascii_lowercase());
            } else if !slug.ends_with('-') {
                slug.push('-');
            }
        }
        let slug: String = slug.trim_matches('-').chars().take(Self::MAX_LEN).collect();
        let slug = slug.trim_end_matches('-');
        Self(if slug.is_empty() {
            "base".to_string()
        } else {
            slug.to_string()
        })
    }

    /// Le texte de l'identifiant.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for BaseId {
    fn default() -> Self {
        Self::main()
    }
}

impl fmt::Display for BaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::BaseId;

    #[test]
    fn identifiers_are_slugs() {
        assert!(BaseId::parse("pro-2027").is_ok());
        assert!(BaseId::parse("Pro").is_err());
        assert!(BaseId::parse("").is_err());
        assert_eq!(
            BaseId::from_name("  Pro – Client X ").as_str(),
            "pro-client-x"
        );
        assert_eq!(BaseId::from_name("Été").as_str(), "t");
        assert_eq!(BaseId::from_name("!!").as_str(), "base");
    }
}
