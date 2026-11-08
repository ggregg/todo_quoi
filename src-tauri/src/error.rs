//! Erreur renvoyée à l'interface : un code stable (pour le code TypeScript) et un message
//! en français (pour l'utilisateur).

use serde::Serialize;
use tq_application::AppError;

/// Erreur d'une commande, sérialisée en `{ "code": "...", "message": "..." }`.
///
/// Codes : `rule` (règle métier refusée), `storage` (drive / fichiers), `invalid-input`
/// (donnée mal formée envoyée par l'interface), `internal` (bug).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UiError {
    /// Catégorie stable, exploitable par l'interface.
    pub code: &'static str,
    /// Message lisible.
    pub message: String,
}

impl UiError {
    /// Donnée reçue de l'interface invalide (identifiant mal formé…).
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            code: "invalid-input",
            message: message.into(),
        }
    }

    /// Stockage indisponible (drive hors ligne, dossier qui ne répond pas).
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "storage",
            message: message.into(),
        }
    }

    /// Erreur inattendue (fil de travail interrompu…).
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal",
            message: message.into(),
        }
    }
}

// [rust] `From` : permet d'écrire `?` sur un `Result<_, AppError>` dans une fonction qui renvoie
//        `Result<_, UiError>` — la conversion est automatique.
impl From<AppError> for UiError {
    fn from(error: AppError) -> Self {
        let code = match error {
            AppError::Domain(_) => "rule",
            AppError::Repository(_) => "storage",
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}
