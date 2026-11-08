//! Erreur d'un cas d'usage : soit une règle métier refusée, soit un problème de stockage.

use crate::ports::RepositoryError;
use crate::vault::VaultError;
use tq_domain::DomainError;

/// Échec d'un cas d'usage.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// Une règle du domaine a refusé l'action (titre vide, tâche introuvable…).
    // [rust] `#[from]` génère la conversion automatique : dans une fonction qui renvoie
    //        `Result<_, AppError>`, l'opérateur `?` transforme une `DomainError` en `AppError`.
    //        `transparent` : le message affiché est celui de l'erreur d'origine.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Le stockage n'a pas pu lire ou écrire.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// Le coffre a refusé (verrouillé, mot de passe incorrect…).
    #[error(transparent)]
    Vault(#[from] VaultError),
}
