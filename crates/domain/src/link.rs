//! Lien externe d'une note (site web, adresse e-mail), ouvert dans l'application du système.
//!
//! Règle de sécurité : seuls `http`, `https` et `mailto` sont ouverts. Un lien `file:`,
//! `javascript:` ou vers une application (`ms-settings:`…) écrit dans une note — qui peut venir
//! d'un autre appareil — ne doit jamais rien lancer d'autre qu'un navigateur ou une messagerie.

use crate::DomainError;
use std::fmt;

/// Un lien externe vérifié.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalLink(String);

impl ExternalLink {
    const MAX_CHARS: usize = 2048;
    const SCHEMES: [&'static str; 3] = ["http://", "https://", "mailto:"];

    /// Vérifie un lien. `www.exemple.fr` devient `https://www.exemple.fr`.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidLink`] : schéma non autorisé, espace ou caractère de contrôle,
    /// lien vide ou trop long.
    ///
    /// ```
    /// use tq_domain::ExternalLink;
    /// assert_eq!(ExternalLink::parse("www.rust-lang.org").unwrap().as_str(), "https://www.rust-lang.org");
    /// assert!(ExternalLink::parse("file:///etc/passwd").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let trimmed = text.trim();
        let candidate = if trimmed.to_lowercase().starts_with("www.") {
            format!("https://{trimmed}")
        } else {
            trimmed.to_string()
        };
        let lower = candidate.to_lowercase();
        let has_scheme = Self::SCHEMES
            .iter()
            .any(|scheme| lower.starts_with(scheme) && lower.len() > scheme.len());
        let is_clean = !candidate
            .chars()
            .any(|c| c.is_whitespace() || c.is_control());
        if has_scheme && is_clean && candidate.chars().count() <= Self::MAX_CHARS {
            Ok(Self(candidate))
        } else {
            Err(DomainError::InvalidLink(text.chars().take(80).collect()))
        }
    }

    /// Le lien, prêt à être ouvert.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ExternalLink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ExternalLink;

    #[test]
    fn web_and_mail_links_are_accepted() {
        for link in [
            "https://exemple.fr/a?b=1&c=2#d",
            "HTTP://EXEMPLE.FR",
            "mailto:greg@exemple.fr",
        ] {
            assert_eq!(ExternalLink::parse(link).expect(link).as_str(), link);
        }
        assert_eq!(
            ExternalLink::parse("  www.exemple.fr ")
                .expect("www")
                .as_str(),
            "https://www.exemple.fr"
        );
    }

    #[test]
    fn other_schemes_and_odd_text_are_refused() {
        let refused = [
            "",
            "https://",
            "file:///C:/Windows",
            "javascript:alert(1)",
            "ms-settings:privacy",
            "exemple.fr",
            "https://exemple.fr/un espace",
            "https://exemple.fr/\u{7}",
        ];
        for link in refused {
            assert!(
                ExternalLink::parse(link).is_err(),
                "devrait être refusé : {link:?}"
            );
        }
        assert!(ExternalLink::parse(&format!("https://{}", "a".repeat(2100))).is_err());
    }
}
