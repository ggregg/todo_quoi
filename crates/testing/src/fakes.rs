//! Faux ports, en mémoire.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use tq_application::ports::{
    AppEvent, Clock, EventPublisher, IdGenerator, MediaStore, NoteDigester, RepositoryError,
    SearchIndex, TaskRepository,
};
use tq_domain::search::{NoteDigest, SearchDocument, SearchHit, SearchQuery, search_documents};
use tq_domain::{MediaRef, NoteBody, Task, TaskId, Timestamp};

/// Verrouille un `Mutex` même si un test précédent a paniqué en le tenant.
// [rust] Un `Mutex` « empoisonné » signale qu'un fil a paniqué pendant qu'il le tenait.
//        Pour un faux de test, on récupère simplement la donnée (`into_inner`).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Dépôt de tâches en mémoire.
// [rust] `Mutex` : les ports sont partagés entre fils (`Send + Sync`) ; le `Mutex` garantit
//        qu'un seul fil modifie la table à la fois (≈ un verrou, inutile en JS mono-fil).
#[derive(Default)]
pub struct InMemoryTaskRepository {
    tasks: Mutex<BTreeMap<TaskId, Task>>,
    unavailable: Mutex<bool>,
}

impl InMemoryTaskRepository {
    /// Dépôt contenant déjà ces tâches.
    pub fn with_tasks(tasks: impl IntoIterator<Item = Task>) -> Self {
        let repository = Self::default();
        lock(&repository.tasks).extend(tasks.into_iter().map(|task| (task.id().clone(), task)));
        repository
    }

    /// Copie de toutes les tâches stockées.
    pub fn snapshot(&self) -> Vec<Task> {
        lock(&self.tasks).values().cloned().collect()
    }

    /// Lit une tâche stockée, si elle existe.
    pub fn get(&self, id: &TaskId) -> Option<Task> {
        lock(&self.tasks).get(id).cloned()
    }

    /// Simule un drive inaccessible : toutes les opérations échouent.
    pub fn make_unavailable(&self) {
        *lock(&self.unavailable) = true;
    }

    fn check_available(&self) -> Result<(), RepositoryError> {
        if *lock(&self.unavailable) {
            Err(RepositoryError::Unavailable(
                "drive simulé hors ligne".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

impl TaskRepository for InMemoryTaskRepository {
    fn load_all(&self) -> Result<Vec<Task>, RepositoryError> {
        self.check_available()?;
        Ok(self.snapshot())
    }

    fn save(&self, task: &Task) -> Result<(), RepositoryError> {
        self.check_available()?;
        lock(&self.tasks).insert(task.id().clone(), task.clone());
        Ok(())
    }
}

/// Horloge de test : part d'un instant donné et avance d'1 ms à chaque lecture, pour que deux
/// modifications successives aient des dates différentes (comme l'horloge hybride réelle).
pub struct SteppingClock {
    next: AtomicI64,
}

impl SteppingClock {
    /// Horloge qui commence à `start`.
    pub fn starting_at(start: Timestamp) -> Self {
        Self {
            next: AtomicI64::new(start.as_millis()),
        }
    }
}

impl Default for SteppingClock {
    fn default() -> Self {
        // 2026-11-06T00:00:00.000Z
        Self::starting_at(Timestamp::from_millis(1_793_923_200_000))
    }
}

impl Clock for SteppingClock {
    fn now(&self) -> Timestamp {
        // [rust] Opération atomique « lire puis ajouter 1 » : sûre même entre plusieurs fils.
        Timestamp::from_millis(self.next.fetch_add(1, Ordering::SeqCst))
    }
}

/// Identifiants prévisibles : `…-000000000001`, `…-000000000002`…
#[derive(Default)]
pub struct SequentialIdGenerator {
    counter: AtomicU64,
}

impl IdGenerator for SequentialIdGenerator {
    fn new_task_id(&self) -> TaskId {
        let number = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        crate::task_id(number)
    }
}

/// Enregistre les événements publiés, pour les vérifier dans les tests.
#[derive(Default)]
pub struct RecordingEventPublisher {
    events: Mutex<Vec<AppEvent>>,
}

impl RecordingEventPublisher {
    /// Événements publiés depuis le début (ou le dernier [`Self::clear`]).
    pub fn events(&self) -> Vec<AppEvent> {
        lock(&self.events).clone()
    }

    /// Oublie les événements déjà publiés.
    pub fn clear(&self) {
        lock(&self.events).clear();
    }
}

impl EventPublisher for RecordingEventPublisher {
    fn publish(&self, event: AppEvent) {
        lock(&self.events).push(event);
    }
}

/// Stockage de médias en mémoire. L'« empreinte » est un hachage simple répété sur 64 chiffres
/// hexadécimaux : suffisant pour des tests (le vrai adaptateur utilise SHA-256).
#[derive(Default)]
pub struct InMemoryMediaStore {
    media: Mutex<HashMap<MediaRef, Vec<u8>>>,
}

impl InMemoryMediaStore {
    /// Nombre de médias stockés.
    pub fn len(&self) -> usize {
        lock(&self.media).len()
    }

    /// Vrai si aucun média n'est stocké.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl MediaStore for InMemoryMediaStore {
    fn store(&self, bytes: &[u8], extension: &str) -> Result<MediaRef, RepositoryError> {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let fake_hash = format!("{:016x}", hasher.finish()).repeat(4);
        let media = MediaRef::from_parts(&fake_hash, extension)
            .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        lock(&self.media).insert(media.clone(), bytes.to_vec());
        Ok(media)
    }

    fn read(&self, media: &MediaRef) -> Result<Vec<u8>, RepositoryError> {
        lock(&self.media)
            .get(media)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound(media.to_string()))
    }

    fn local_path(&self, _media: &MediaRef) -> Option<PathBuf> {
        None
    }
}

/// Index de recherche en mémoire (même principe que l'adaptateur `memory_index`, testé par le
/// même contrat). Compte les fiches refaites, pour vérifier qu'une tâche inchangée n'est pas
/// relue.
#[derive(Default)]
pub struct InMemorySearchIndex {
    documents: Mutex<HashMap<TaskId, SearchDocument>>,
    upserts: AtomicU64,
}

impl InMemorySearchIndex {
    /// Nombre de fiches.
    pub fn len(&self) -> usize {
        lock(&self.documents).len()
    }

    /// Vrai si l'index est vide.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Nombre total de fiches ajoutées ou refaites depuis la création.
    pub fn upsert_count(&self) -> u64 {
        self.upserts.load(Ordering::SeqCst)
    }
}

impl SearchIndex for InMemorySearchIndex {
    fn indexed_version(&self, id: &TaskId) -> Option<Timestamp> {
        lock(&self.documents)
            .get(id)
            .map(|document| document.version)
    }

    fn upsert(&self, document: SearchDocument) {
        self.upserts.fetch_add(1, Ordering::SeqCst);
        lock(&self.documents).insert(document.id.clone(), document);
    }

    fn remove(&self, id: &TaskId) {
        lock(&self.documents).remove(id);
    }

    fn retain_only(&self, ids: &[TaskId]) {
        lock(&self.documents).retain(|id, _| ids.contains(id));
    }

    fn search(&self, query: &SearchQuery, limit: usize) -> Vec<SearchHit> {
        search_documents(&lock(&self.documents), query, limit)
    }
}

/// Extraction de note simplifiée : le Markdown brut sert de texte ; `![`, `::audio`, `::file`
/// signalent les médias. Suffisant pour tester l'application (le vrai : `note_markdown`).
#[derive(Default)]
pub struct PlainNoteDigester;

impl NoteDigester for PlainNoteDigester {
    fn digest(&self, note: &NoteBody) -> NoteDigest {
        use tq_domain::search::MediaKind;
        let markdown = note.as_markdown();
        let media = [
            ("![", MediaKind::Image),
            ("::audio", MediaKind::Audio),
            ("::file", MediaKind::File),
        ]
        .into_iter()
        .filter(|(marker, _)| markdown.contains(marker))
        .map(|(_, kind)| kind)
        .collect();
        NoteDigest {
            text: markdown.to_string(),
            media,
        }
    }
}
