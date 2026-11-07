//! Erreurs métier : tout ce qu'une règle du domaine peut refuser.
//!
//! Chaque variante porte un message en français destiné à être affiché tel quel.

use crate::TaskId;

/// Une règle métier a été violée.
// [rust] `thiserror::Error` génère l'implémentation du trait `std::error::Error` ;
//        `#[error("…")]` donne le message affiché (`{0}` = premier champ, `{max}` = champ nommé).
//        Équivalent TS : une classe `DomainError extends Error` par cas, en beaucoup plus court.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Titre vide ou composé uniquement d'espaces.
    #[error("le titre ne peut pas être vide")]
    EmptyTitle,
    /// Titre trop long.
    #[error("le titre dépasse {max} caractères")]
    TitleTooLong {
        /// Longueur maximale autorisée.
        max: usize,
    },
    /// Titre contenant un retour à la ligne.
    #[error("le titre doit tenir sur une seule ligne")]
    MultilineTitle,
    /// Étiquette mal formée.
    #[error("étiquette invalide : « {0} »")]
    InvalidTag(String),
    /// Lien externe refusé (seuls http, https et mailto sont ouverts).
    #[error("lien non pris en charge : « {0} » (http, https ou mailto attendu)")]
    InvalidLink(String),
    /// Identifiant de tâche mal formé (UUID attendu).
    #[error("identifiant de tâche invalide : « {0} »")]
    InvalidTaskId(String),
    /// Identifiant d'appareil mal formé.
    #[error("identifiant d'appareil invalide : « {0} »")]
    InvalidDeviceId(String),
    /// Date au format RFC 3339 invalide.
    #[error("date invalide : « {0} »")]
    InvalidTimestamp(String),
    /// Clé d'ordre mal formée.
    #[error("clé d'ordre invalide : « {0} »")]
    InvalidOrderKey(String),
    /// Demande de clé entre deux bornes qui ne sont pas dans l'ordre.
    #[error("les clés d'ordre encadrantes doivent être croissantes")]
    OrderKeysNotIncreasing,
    /// Aucune tâche avec cet identifiant.
    #[error("tâche introuvable : {0}")]
    TaskNotFound(TaskId),
    /// Deux tâches portent le même identifiant.
    #[error("tâche en double : {0}")]
    DuplicateTask(TaskId),
    /// Clôture d'une tâche déjà terminée.
    #[error("la tâche {0} est déjà terminée")]
    AlreadyClosed(TaskId),
    /// Réouverture d'une tâche déjà ouverte.
    #[error("la tâche {0} est déjà ouverte")]
    AlreadyOpen(TaskId),
    /// Modification d'une tâche qui est dans la corbeille.
    #[error("la tâche {0} est dans la corbeille")]
    TaskInTrash(TaskId),
    /// Restauration d'une tâche qui n'est pas dans la corbeille.
    #[error("la tâche {0} n'est pas dans la corbeille")]
    TaskNotInTrash(TaskId),
    /// Déplacement d'une tâche sous elle-même ou sous une de ses sous-tâches (R11).
    #[error("impossible de déplacer la tâche {task} dans son propre sous-arbre")]
    MoveIntoOwnSubtree {
        /// La tâche qu'on voulait déplacer.
        task: TaskId,
    },
    /// Indentation impossible : pas de tâche au-dessus au même niveau (R9).
    #[error("aucune tâche au-dessus de {0} sous laquelle l'indenter")]
    NoPreviousSibling(TaskId),
    /// Désindentation impossible : la tâche est déjà au premier niveau (R9).
    #[error("la tâche {0} est déjà au premier niveau")]
    AlreadyAtRoot(TaskId),
    /// Nom de média mal formé (ou qui ressemble à un chemin).
    #[error("référence de média invalide : « {0} »")]
    InvalidMediaRef(String),
    /// Média trop volumineux.
    #[error("le fichier dépasse la taille maximale ({max_megabytes} Mo)")]
    MediaTooLarge {
        /// Taille maximale, en mégaoctets.
        max_megabytes: usize,
    },
    /// Monter / descendre impossible : la tâche est déjà en haut ou en bas.
    #[error("la tâche {0} ne peut pas aller plus loin dans cette direction")]
    CannotMoveFurther(TaskId),
}
