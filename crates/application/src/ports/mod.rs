//! Ports **secondaires** : ce dont l'application a besoin, exprimé sous forme de traits.
//!
//! Un trait Rust ≈ une interface TypeScript. Chaque port est **petit** (une responsabilité,
//! principe de ségrégation des interfaces) et marqué `Send + Sync` : utilisable depuis
//! plusieurs fils d'exécution (la couche Tauri exécute les cas d'usage hors du fil de l'UI).
//!
//! Implémentations :
//! - réelles : crates `adapters/*` (fichiers du drive, horloge système…) ;
//! - de test : crate `testing` (en mémoire, horloge contrôlée…).

use std::path::PathBuf;
use tq_domain::search::{NoteDigest, SearchDocument, SearchHit, SearchQuery};
use tq_domain::{MediaRef, NoteBody, Task, TaskId, Timestamp};

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

/// Index de recherche : conserve une **fiche** par tâche active et cherche dedans (spec §5.6).
///
/// Il est tenu à jour par [`crate::IndexingTaskRepository`] à chaque lecture et écriture de
/// tâches : les cas d'usage n'ont pas à s'en soucier. Implémentation : `memory_index` (en
/// mémoire) ; un jour, peut-être, SQLite FTS sans toucher au reste (archi §6).
pub trait SearchIndex: Send + Sync {
    /// Version de la fiche connue pour cette tâche (`None` si absente) : évite de refaire une
    /// fiche quand la tâche n'a pas changé.
    fn indexed_version(&self, id: &TaskId) -> Option<Timestamp>;

    /// Ajoute ou remplace la fiche d'une tâche.
    fn upsert(&self, document: SearchDocument);

    /// Retire la fiche d'une tâche (mise à la corbeille).
    fn remove(&self, id: &TaskId);

    /// Ne garde que les fiches de ces tâches (les autres ont disparu du stockage).
    fn retain_only(&self, ids: &[TaskId]);

    /// Cherche ; au plus `limit` résultats, dans l'ordre de pertinence.
    fn search(&self, query: &SearchQuery, limit: usize) -> Vec<SearchHit>;
}

/// Extraction du contenu cherchable d'une note (texte à plat, médias présents).
///
/// Le domaine ne lit pas le Markdown (ADR-0007) : c'est l'adaptateur `note_markdown` qui sait
/// le faire.
pub trait NoteDigester: Send + Sync {
    /// Texte et médias de la note.
    fn digest(&self, note: &NoteBody) -> NoteDigest;
}

/// Stockage des médias (images, audio, pièces jointes), nommés par l'empreinte de leur contenu.
pub trait MediaStore: Send + Sync {
    /// Stocke un contenu (ou retrouve celui qui existe déjà à l'identique) et renvoie sa référence.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'écriture échoue ou si l'extension est invalide.
    fn store(&self, bytes: &[u8], extension: &str) -> Result<MediaRef, RepositoryError>;

    /// Lit un média.
    ///
    /// # Erreurs
    /// [`RepositoryError::NotFound`] s'il n'existe pas (pas encore synchronisé…).
    fn read(&self, media: &MediaRef) -> Result<Vec<u8>, RepositoryError>;

    /// Emplacement du média sur le disque, pour l'ouvrir avec l'application par défaut de l'OS.
    /// `None` si le stockage n'est pas un dossier local.
    fn local_path(&self, media: &MediaRef) -> Option<PathBuf>;
}

/// Problème de stockage.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// Lecture ou écriture impossible (dossier absent, droits, drive qui ne répond pas…).
    #[error("stockage inaccessible : {0}")]
    Unavailable(String),
    /// L'élément demandé n'existe pas (ou pas encore : synchronisation en cours).
    #[error("introuvable : {0}")]
    NotFound(String),
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
