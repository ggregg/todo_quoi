//! Erreur renvoyée à l'interface : un code stable (pour le code TypeScript) et un message
//! en français (pour l'utilisateur).

use serde::Serialize;
use tq_application::AppError;
use tq_application::ports::RepositoryError;
use tq_application::vault::VaultError;

/// Erreur d'une commande, sérialisée en `{ "code": "...", "message": "..." }`.
///
/// Codes : `rule` (règle métier refusée), `storage` (drive / fichiers), `invalid-input`
/// (donnée mal formée envoyée par l'interface), `internal` (bug), et pour le coffre
/// (ADR-0010) : `vault-none` (pas encore de coffre : en créer un), `vault-locked`
/// (le déverrouiller), `vault-target-locked` (coffre de la base de destination fermé),
/// `vault-wrong-password`, `vault-password-too-short`, `vault` (autre).
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
        let code = match &error {
            AppError::Domain(_) => "rule",
            AppError::Vault(vault) | AppError::Repository(RepositoryError::Vault(vault)) => {
                vault_code(vault)
            }
            AppError::Repository(_) => "storage",
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}

/// Code stable d'une erreur de coffre : l'interface s'en sert pour proposer la bonne action.
fn vault_code(error: &VaultError) -> &'static str {
    match error {
        VaultError::NoVault => "vault-none",
        VaultError::Locked => "vault-locked",
        VaultError::TargetLocked { .. } => "vault-target-locked",
        VaultError::WrongPassword => "vault-wrong-password",
        VaultError::PasswordTooShort { .. } => "vault-password-too-short",
        VaultError::AlreadyExists
        | VaultError::EncryptedMedia
        | VaultError::Crypto(_)
        | VaultError::Keyring(_) => "vault",
    }
}
