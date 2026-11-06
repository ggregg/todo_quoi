//! Identifiants : d'une tâche ([`TaskId`]) et d'un appareil ([`DeviceId`]).
//!
//! Ce sont des *newtypes* : une `String` enveloppée dans un type à part. Impossible de passer
//! un identifiant d'appareil là où un identifiant de tâche est attendu — le compilateur refuse.
//! Équivalent TS : les « branded types » (`type TaskId = string & { __brand: 'TaskId' }`).

use crate::DomainError;
use std::fmt;

/// Identifiant d'une tâche : un UUID (format `8-4-4-4-12` hexadécimal, en minuscules).
///
/// Le domaine **valide** les identifiants mais ne les **génère** pas : générer demande du
/// hasard et l'heure, fournis par un adaptateur (`IdGenerator`, UUID v7).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(String);

impl TaskId {
    /// Valide un identifiant ; les majuscules sont acceptées et converties en minuscules.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidTaskId`] si le texte n'est pas un UUID.
    ///
    /// ```
    /// use tq_domain::TaskId;
    /// assert!(TaskId::parse("0195F3C2-8E4A-7C1D-9B2F-3A6E5D4C7B1E").is_ok());
    /// assert!(TaskId::parse("pas-un-uuid").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let normalized = text.trim().to_ascii_lowercase();
        let is_uuid = normalized.len() == 36
            && normalized.char_indices().all(|(index, c)| match index {
                8 | 13 | 18 | 23 => c == '-',
                _ => c.is_ascii_hexdigit(),
            });
        if is_uuid {
            Ok(Self(normalized))
        } else {
            Err(DomainError::InvalidTaskId(text.to_string()))
        }
    }

    /// Le texte de l'identifiant.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// [rust] Implémenter `Display` permet `format!("{id}")` et `id.to_string()` (≈ `toString()`).
impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Identifiant d'un appareil (ex. `mac-greg`), utilisé pour savoir qui a modifié quoi.
///
/// Caractères autorisés : lettres minuscules ASCII, chiffres et `-` ; 1 à 40 caractères.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    const MAX_LEN: usize = 40;

    /// Valide un identifiant d'appareil déjà normalisé.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidDeviceId`] si un caractère n'est pas autorisé ou si la longueur
    /// n'est pas comprise entre 1 et 40.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let is_valid = (1..=Self::MAX_LEN).contains(&text.len())
            && text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if is_valid {
            Ok(Self(text.to_string()))
        } else {
            Err(DomainError::InvalidDeviceId(text.to_string()))
        }
    }

    /// Construit un identifiant à partir du nom de la machine (spec : « repris du nom de la
    /// machine »), en le normalisant : `MacBook Pro de Greg.local` → `macbook-pro-de-greg-local`.
    /// Un nom inutilisable donne `appareil`.
    pub fn from_machine_name(name: &str) -> Self {
        let mut normalized = String::new();
        for c in name.trim().chars() {
            if c.is_ascii_alphanumeric() {
                normalized.push(c.to_ascii_lowercase());
            } else if !normalized.ends_with('-') {
                normalized.push('-');
            }
        }
        let trimmed: String = normalized
            .trim_matches('-')
            .chars()
            .take(Self::MAX_LEN)
            .collect();
        let trimmed = trimmed.trim_end_matches('-');
        if trimmed.is_empty() {
            Self("appareil".to_string())
        } else {
            Self(trimmed.to_string())
        }
    }

    /// Le texte de l'identifiant.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_is_normalized_to_lowercase() {
        let id = TaskId::parse("0195F3C2-8E4A-7C1D-9B2F-3A6E5D4C7B1E").unwrap();
        assert_eq!(id.as_str(), "0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e");
    }

    #[test]
    fn malformed_task_ids_are_rejected() {
        for text in [
            "",
            "0195f3c2",
            "0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1",
            "0195f3c2_8e4a-7c1d-9b2f-3a6e5d4c7b1e",
            "zzzzzzzz-8e4a-7c1d-9b2f-3a6e5d4c7b1e",
        ] {
            assert!(TaskId::parse(text).is_err(), "devrait être refusé : {text}");
        }
    }

    #[test]
    fn device_id_is_derived_from_machine_name() {
        assert_eq!(
            DeviceId::from_machine_name("MacBook Pro de Greg.local").as_str(),
            "macbook-pro-de-greg-local"
        );
        assert_eq!(
            DeviceId::from_machine_name("PC-BUREAU").as_str(),
            "pc-bureau"
        );
        assert_eq!(DeviceId::from_machine_name("  ").as_str(), "appareil");
        assert_eq!(
            DeviceId::from_machine_name("Élodie's PC").as_str(),
            "lodie-s-pc"
        );
        assert!(DeviceId::from_machine_name(&"x".repeat(80)).as_str().len() <= 40);
    }

    #[test]
    fn device_id_validation() {
        assert!(DeviceId::parse("mac-greg").is_ok());
        assert!(DeviceId::parse("Mac Greg").is_err());
        assert!(DeviceId::parse("").is_err());
    }
}
