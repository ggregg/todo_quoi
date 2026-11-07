//! Lecture et écriture des fichiers de tâche dans le dossier du drive.
//!
//! Deux précautions propres aux dossiers synchronisés :
//! - **écriture atomique** : on écrit dans un fichier temporaire `.~<id>.tmp`, on force
//!   l'écriture sur disque, puis on le renomme. Le client du drive ne voit jamais un fichier
//!   à moitié écrit (les noms en `.~` sont ignorés par OneDrive / Dropbox et par la lecture) ;
//! - **tolérance** : un fichier illisible ne bloque pas l'application ; il est écarté et
//!   signalé ([`FsMarkdownTaskRepository::rejected_files`]), jamais réécrit.

use crate::format::{parse_task_file, render_task_file};
use crate::manifest::{self, unavailable};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tq_application::ports::{RepositoryError, TaskRepository};
use tq_domain::{Task, TaskId};

const TEMPORARY_PREFIX: &str = ".~";

/// Fichier écarté à la lecture, et pourquoi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedFile {
    /// Chemin du fichier.
    pub path: PathBuf,
    /// Raison, lisible par l'utilisateur.
    pub reason: String,
}

/// Dépôt de tâches dans un dossier (`<racine>/tasks/*.md`).
pub struct FsMarkdownTaskRepository {
    tasks_dir: PathBuf,
    rejected: Mutex<Vec<RejectedFile>>,
}

impl FsMarkdownTaskRepository {
    /// Ouvre (ou initialise) l'espace de données situé dans `root`.
    ///
    /// # Erreurs
    /// [`RepositoryError::Unavailable`] si le dossier n'est pas utilisable ;
    /// [`RepositoryError::NewerSchema`] si les données viennent d'une version plus récente.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RepositoryError> {
        let root = root.into();
        let tasks_dir = root.join("tasks");
        fs::create_dir_all(&tasks_dir).map_err(|error| unavailable(&tasks_dir, &error))?;
        manifest::ensure(&root)?;
        Ok(Self {
            tasks_dir,
            rejected: Mutex::new(Vec::new()),
        })
    }

    /// Fichiers écartés lors du dernier chargement (illisibles, en double…).
    pub fn rejected_files(&self) -> Vec<RejectedFile> {
        self.rejected
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn task_path(&self, id: &TaskId) -> PathBuf {
        self.tasks_dir.join(format!("{id}.md"))
    }

    /// Fichiers `.md` du dossier `tasks/`, hors fichiers temporaires.
    fn task_files(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        let entries =
            fs::read_dir(&self.tasks_dir).map_err(|error| unavailable(&self.tasks_dir, &error))?;
        let mut files: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
            .filter(|path| !file_name(path).starts_with(TEMPORARY_PREFIX))
            .collect();
        files.sort();
        Ok(files)
    }

    /// Champs d'en-tête inconnus du fichier existant, à conserver lors de la réécriture.
    fn existing_unknown_fields(path: &Path) -> toml::Table {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| parse_task_file(&text).ok())
            .map(|parsed| parsed.unknown)
            .unwrap_or_default()
    }
}

impl TaskRepository for FsMarkdownTaskRepository {
    fn load_all(&self) -> Result<Vec<Task>, RepositoryError> {
        let mut by_id: HashMap<TaskId, (Task, PathBuf)> = HashMap::new();
        let mut rejected = Vec::new();
        for path in self.task_files()? {
            let parsed = fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|text| parse_task_file(&text).map_err(|error| error.to_string()));
            match parsed {
                Ok(parsed) => keep_most_recent(&mut by_id, parsed.task, path, &mut rejected),
                Err(reason) => rejected.push(RejectedFile { path, reason }),
            }
        }
        *self
            .rejected
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = rejected;
        Ok(by_id.into_values().map(|(task, _)| task).collect())
    }

    fn save(&self, task: &Task) -> Result<(), RepositoryError> {
        let path = self.task_path(task.id());
        let unknown = Self::existing_unknown_fields(&path);
        let text = render_task_file(task, &unknown)
            .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        write_atomically(&path, &text).map_err(|error| unavailable(&path, &error))
    }
}

/// Deux fichiers pour la même tâche (copie de conflit créée par le drive, `x (1).md`…) :
/// on garde la version la plus récente et on signale l'autre. Le traitement complet des
/// conflits (`.conflicts/`) arrive en I5.
fn keep_most_recent(
    by_id: &mut HashMap<TaskId, (Task, PathBuf)>,
    task: Task,
    path: PathBuf,
    rejected: &mut Vec<RejectedFile>,
) {
    let id = task.id().clone();
    let Some((existing, existing_path)) = by_id.remove(&id) else {
        by_id.insert(id, (task, path));
        return;
    };
    let (kept, dropped) = if task.updated_at() > existing.updated_at() {
        ((task, path), existing_path)
    } else {
        ((existing, existing_path), path)
    };
    rejected.push(RejectedFile {
        path: dropped,
        reason: format!("doublon de la tâche {id} (version plus ancienne)"),
    });
    by_id.insert(id, kept);
}

/// Écrit `text` dans `path` sans jamais laisser de fichier partiellement écrit.
pub(crate) fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    let temporary = path.with_file_name(format!("{TEMPORARY_PREFIX}{}.tmp", file_name(path)));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?; // force l'écriture physique avant le renommage
    drop(file);
    fs::rename(&temporary, path)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
