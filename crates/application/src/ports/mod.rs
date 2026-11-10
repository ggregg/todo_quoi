//! Ports **secondaires** : ce dont l'application a besoin, exprimé sous forme de traits.
//!
//! Un trait Rust ≈ une interface TypeScript. Chaque port est **petit** (une responsabilité,
//! principe de ségrégation des interfaces) et marqué `Send + Sync` : utilisable depuis
//! plusieurs fils d'exécution (la couche Tauri exécute les cas d'usage hors du fil de l'UI).
//!
//! Implémentations :
//! - réelles : crates `adapters/*` (fichiers du drive, horloge système…) ;
//! - de test : crate `testing` (en mémoire, horloge contrôlée…).

mod vault;

pub use vault::{Cipher, ContentSealer, KdfParams, KeyVault, VaultKeys, VaultRecord, VaultStore};

use std::path::Path;
use std::path::PathBuf;
use tq_domain::search::{MediaKind, NoteDigest, SearchDocument, SearchHit, SearchQuery};
use tq_domain::view::ListView;
use tq_domain::{BaseId, LocalFileKind, MediaRef, NoteBody, Task, TaskId, Timestamp};

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

    /// Efface **définitivement** une tâche (vidage de la corbeille après 30 jours).
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'effacement échoue. Une tâche déjà absente n'est pas une erreur.
    fn remove(&self, id: &TaskId) -> Result<(), RepositoryError>;
}

/// Résultat d'un passage sur le dossier du drive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanReport {
    /// Tâches modifiées, ajoutées ou effacées **par un autre appareil** depuis le passage
    /// précédent (nos propres écritures n'y figurent pas).
    pub changed: Vec<TaskId>,
    /// Fichiers pas encore téléchargés par le client du drive (« à la demande »).
    pub pending_downloads: usize,
    /// Copies de conflit réglées pendant ce passage (perdantes rangées dans `.conflicts/`).
    pub resolved_conflicts: usize,
    /// Fichiers illisibles, ignorés (jamais réécrits).
    pub unreadable_files: usize,
}

/// Une version perdante conservée dans `.conflicts/` (filet de sécurité, 30 jours).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictCopy {
    /// Tâche concernée, si le fichier est lisible.
    pub task_id: Option<TaskId>,
    /// Titre de cette version, si lisible.
    pub title: Option<String>,
    /// Nom du fichier dans `.conflicts/`.
    pub file_name: String,
    /// Date à laquelle la version a été écartée.
    pub set_aside_at: Timestamp,
}

/// Synchronisation par le dossier du drive (spec §7, architecture §5) : repérer les changements
/// faits ailleurs, régler les copies de conflit, faire télécharger les fichiers « à la demande ».
pub trait DriveSync: Send + Sync {
    /// Un passage (toutes les 3 s) : le premier sert de point de départ et ne signale rien.
    ///
    /// # Erreurs
    /// [`RepositoryError::Unavailable`] si le dossier est inaccessible.
    fn scan(&self) -> Result<ScanReport, RepositoryError>;

    /// Versions écartées, les plus récentes d'abord (journal des conflits).
    ///
    /// # Erreurs
    /// [`RepositoryError`] si le dossier est illisible.
    fn conflict_copies(&self) -> Result<Vec<ConflictCopy>, RepositoryError>;

    /// Range la version actuellement sur disque de la tâche dans `.conflicts/`, avant que
    /// l'utilisateur ne la remplace par la sienne (« Garder la mienne »).
    ///
    /// # Erreurs
    /// [`RepositoryError`] si la copie échoue. Fichier absent : rien à faire.
    fn set_aside_current(&self, id: &TaskId) -> Result<(), RepositoryError>;

    /// Efface les versions écartées avant `before` ; renvoie leur nombre.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'effacement échoue.
    fn purge_conflict_copies(&self, before: Timestamp) -> Result<usize, RepositoryError>;
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

    /// Médias présents dans la note d'une tâche (déjà extraits pour sa fiche) : sert au filtre
    /// « contient médias » et aux icônes de la liste, sans relire les notes.
    fn media_of(&self, id: &TaskId) -> Vec<MediaKind>;
}

/// Les bases ouvertes sur cet appareil, dans l'ordre du bandeau (ADR-0009).
pub trait BaseDirectory: Send + Sync {
    /// Bases ouvertes (au moins une).
    fn bases(&self) -> Vec<BaseId>;
}

/// Vue de la **liste racine** de chaque base (spec §5.3), commune à tous les appareils.
pub trait RootViewStore: Send + Sync {
    /// Vue racine enregistrée ; `None` si aucune (on utilise alors `ListView::default()`).
    ///
    /// # Erreurs
    /// [`RepositoryError`] si le stockage est inaccessible.
    fn load_root_view(&self, base: &BaseId) -> Result<Option<ListView>, RepositoryError>;

    /// Enregistre la vue racine.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'écriture échoue.
    fn save_root_view(&self, base: &BaseId, view: &ListView) -> Result<(), RepositoryError>;
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
    fn store(
        &self,
        base: &BaseId,
        bytes: &[u8],
        extension: &str,
    ) -> Result<MediaRef, RepositoryError>;

    /// Range `bytes` sous le nom `media`, choisi par l'application (média chiffré : nommé par
    /// une empreinte avec la clé du coffre). Un média qui existe déjà n'est pas réécrit.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'écriture échoue.
    fn put(&self, base: &BaseId, media: &MediaRef, bytes: &[u8]) -> Result<(), RepositoryError>;

    /// Lit un média (tel quel : chiffré s'il l'est).
    ///
    /// # Erreurs
    /// [`RepositoryError::NotFound`] s'il n'existe pas (pas encore synchronisé…).
    fn read(&self, media: &MediaRef) -> Result<Vec<u8>, RepositoryError>;

    /// Emplacement du média sur le disque, pour l'ouvrir avec l'application par défaut de l'OS.
    /// `None` si le stockage n'est pas un dossier local, ou si le média n'y est pas.
    fn local_path(&self, media: &MediaRef) -> Option<PathBuf>;

    /// Efface **définitivement** un média de la base `base` (purge d'une tâche, R8).
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'effacement échoue. Un média déjà absent n'est pas une erreur.
    fn remove(&self, base: &BaseId, media: &MediaRef) -> Result<(), RepositoryError>;
}

/// Fichiers de l'utilisateur **hors des bases** (joindre, déplacer ou lier un fichier choisi
/// dans le Finder / l'Explorateur).
pub trait LocalFiles: Send + Sync {
    /// Ce qui se trouve à `path` (rien, dossier, programme, document).
    fn kind(&self, path: &Path) -> LocalFileKind;
    /// Lit le fichier.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si la lecture échoue.
    fn read(&self, path: &Path) -> Result<Vec<u8>, RepositoryError>;
    /// Efface le fichier d'origine (« Déplacer », une fois la copie rangée dans la base).
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'effacement échoue.
    fn remove(&self, path: &Path) -> Result<(), RepositoryError>;
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
    /// Tâche chiffrée à écrire alors que le coffre de sa base est verrouillé (ou absent).
    #[error("{0}")]
    Vault(crate::vault::VaultError),
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
    /// Ces tâches ont été modifiées **par un autre appareil** (vu par le scan du drive) : une
    /// saisie locale en cours sur l'une d'elles est en conflit (bandeau « Recharger / Garder »).
    ChangedElsewhere {
        /// Tâches concernées.
        ids: Vec<TaskId>,
    },
}
