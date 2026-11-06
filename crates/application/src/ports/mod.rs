//! Ports **secondaires** : ce dont l'application a besoin, exprimé sous forme de traits.
//!
//! Un trait Rust ≈ une interface TypeScript. Chaque port est **petit** (une responsabilité,
//! principe de ségrégation des interfaces) et marqué `Send + Sync` : utilisable depuis
//! plusieurs fils d'exécution (la couche Tauri exécute les cas d'usage hors du fil de l'UI).
//!
//! Implémentations :
//! - réelles : crates `adapters/*` (fichiers du drive, horloge système…) ;
//! - de test : crate `testing` (en mémoire, horloge contrôlée…).

use tq_domain::{Task, TaskId, Timestamp};

/// Stockage des tâches (corbeille comprise).
pub trait TaskRepository: Send + Sync {
    /// Charge toutes les tâches.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si le stockage est inaccessible.
    fn load_all(&self) -> Result<Vec<Task>, RepositoryError>;

    /// Crée ou remplace une tâche.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'écriture échoue.
    fn save(&self, task: &Task) -> Result<(), RepositoryError>;
}

/// Problème de stockage.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// Lecture ou écriture impossible (dossier absent, droits, drive qui ne répond pas…).
    #[error("stockage inaccessible : {0}")]
    Unavailable(String),
    /// Les données ont été écrites par une version plus récente de TodoQuoi.
    #[error(
        "les données utilisent un format plus récent (version {found}, cette application gère \
         la version {supported}) : mettez TodoQuoi à jour sur cet appareil"
    )]
    NewerSchema {
        /// Version trouvée.
        found: u32,
        /// Version la plus récente gérée par cette application.
        supported: u32,
    },
}

/// Source de l'heure. En production, une horloge « hybride » qui ne recule jamais ;
/// en test, une horloge entièrement contrôlée.
pub trait Clock: Send + Sync {
    /// L'instant présent.
    fn now(&self) -> Timestamp;
}

/// Générateur d'identifiants de tâche (UUID v7 en production).
pub trait IdGenerator: Send + Sync {
    /// Un identifiant jamais utilisé.
    fn new_task_id(&self) -> TaskId;
}

/// Canal vers l'extérieur (l'interface) pour signaler un changement.
pub trait EventPublisher: Send + Sync {
    /// Publie un événement. Ne doit pas échouer : un événement perdu n'empêche pas l'action.
    fn publish(&self, event: AppEvent);
}

/// Événement publié après une action réussie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// Ces tâches ont été créées ou modifiées.
    TasksChanged {
        /// Tâches concernées.
        ids: Vec<TaskId>,
    },
}
