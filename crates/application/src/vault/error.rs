//! **Couche application** : erreurs du coffre (traduites pour l'interface par `src-tauri`).

/// Échec d'une opération du coffre.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VaultError {
    /// La base n'a pas encore de coffre : il faut en créer un (premier chiffrement).
    #[error("cette base n'a pas encore de coffre")]
    NoVault,
    /// La base a déjà un coffre.
    #[error("cette base a déjà un coffre")]
    AlreadyExists,
    /// Le coffre est verrouillé.
    #[error("le coffre de cette base est verrouillé")]
    Locked,
    /// Une tâche chiffrée ne peut aller dans une base que si le coffre de celle-ci est ouvert :
    /// sa note y sera rechiffrée avec sa clé.
    #[error(
        "le coffre de la base de destination est verrouillé ou n'existe pas : \
         ouvrez-le (ou créez-le) pour y déplacer une tâche chiffrée"
    )]
    TargetLocked {
        /// Base de destination.
        base: tq_domain::BaseId,
    },
    /// Un média chiffré n'a pas de copie lisible sur le disque (rien à montrer dans le Finder).
    #[error("ce média est chiffré : il n'existe pas en clair sur le disque")]
    EncryptedMedia,
    /// Mot de passe incorrect.
    #[error("mot de passe incorrect")]
    WrongPassword,
    /// Mot de passe trop court.
    #[error("le mot de passe doit contenir au moins {min} caractères")]
    PasswordTooShort {
        /// Longueur minimale.
        min: usize,
    },
    /// Échec du chiffrement (données abîmées, aléa indisponible…).
    #[error("chiffrement : {0}")]
    Crypto(String),
    /// Le trousseau de l'OS a refusé.
    #[error("trousseau du système : {0}")]
    Keyring(String),
}
