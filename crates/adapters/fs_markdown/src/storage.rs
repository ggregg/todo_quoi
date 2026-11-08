//! Lecture et écriture des fichiers de tâche dans le dossier du drive.
//!
//! Deux précautions propres aux dossiers synchronisés :
//! - **écriture atomique** : on écrit dans un fichier temporaire `.~<id>.tmp`, on force
//!   l'écriture sur disque, puis on le renomme. Le client du drive ne voit jamais un fichier
//!   à moitié écrit (les noms en `.~` sont ignorés par OneDrive / Dropbox et par la lecture) ;
//! - **tolérance** : un fichier illisible ne bloque pas l'application ; il est écarté et
//!   signalé ([`FsMarkdownTaskRepository::rejected_files`]), jamais réécrit.

use crate::conflicts;
use crate::drive_sync::FsDriveSync;
use crate::file_state::{FileStamp, KnownFiles, is_dataless};
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
    root: PathBuf,
    tasks_dir: PathBuf,
    rejected: Mutex<Vec<RejectedFile>>,
    /// Empreintes partagées avec le scan ([`FsDriveSync`]).
    known: KnownFiles,
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
            root,
            tasks_dir,
            rejected: Mutex::new(Vec::new()),
            known: KnownFiles::default(),
        })
    }

    /// Le scan du dossier (port `DriveSync`), qui partage les empreintes de ce dépôt : il ne
    /// signale donc pas nos propres écritures comme des changements venus d'ailleurs.
    pub fn drive_sync(&self) -> FsDriveSync {
        FsDriveSync::new(
            self.root.clone(),
            self.tasks_dir.clone(),
            self.known.clone(),
        )
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

    /// Fichiers `.md` du dossier `tasks/` présents localement, hors fichiers temporaires. Les
    /// fichiers « à la demande » pas encore téléchargés sont laissés au scan (ADR-0008) : les
    /// lire ici bloquerait l'interface le temps du téléchargement.
    fn task_files(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        Ok(task_files(&self.tasks_dir)?
            .into_iter()
            .filter(|path| fs::metadata(path).is_ok_and(|metadata| !is_dataless(&metadata)))
            .collect())
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

    /// Écrit la tâche. Si le fichier a été modifié par un autre appareil depuis le dernier
    /// scan, sa version est d'abord rangée dans `.conflicts/` : la nôtre, plus récente, gagne
    /// (dernier qui écrit), l'autre n'est pas perdue.
    fn save(&self, task: &Task) -> Result<(), RepositoryError> {
        let path = self.task_path(task.id());
        let mut known = self.known.lock();
        self.set_aside_if_changed_elsewhere(&path, known.stamps.get(&path).copied())?;
        let unknown = Self::existing_unknown_fields(&path);
        let text = render_task_file(task, &unknown)
            .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        write_atomically(&path, &text).map_err(|error| unavailable(&path, &error))?;
        if let Ok(metadata) = fs::metadata(&path) {
            known.stamps.insert(path, FileStamp::of(&metadata));
        }
        Ok(())
    }

    fn remove(&self, id: &TaskId) -> Result<(), RepositoryError> {
        let path = self.task_path(id);
        let mut known = self.known.lock();
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(unavailable(&path, &error)),
        }
        known.stamps.remove(&path);
        Ok(())
    }
}

impl FsMarkdownTaskRepository {
    /// Range la version sur disque dans `.conflicts/` si son empreinte n'est plus celle connue.
    fn set_aside_if_changed_elsewhere(
        &self,
        path: &Path,
        known: Option<FileStamp>,
    ) -> Result<(), RepositoryError> {
        let (Some(known), Ok(metadata)) = (known, fs::metadata(path)) else {
            return Ok(()); // fichier nouveau, ou pas encore vu par le scan
        };
        if FileStamp::of(&metadata) == known {
            return Ok(());
        }
        let text = fs::read_to_string(path).map_err(|error| unavailable(path, &error))?;
        let device = parse_task_file(&text)
            .map(|parsed| parsed.task.updated_by().to_string())
            .unwrap_or_default();
        let id = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        conflicts::set_aside(&self.root, &id, &device, &text)
            .map_err(|error| unavailable(path, &error))?;
        Ok(())
    }
}

/// Fichiers `.md` d'un dossier `tasks/`, hors fichiers temporaires, triés.
pub(crate) fn task_files(tasks_dir: &Path) -> Result<Vec<PathBuf>, RepositoryError> {
    let entries = fs::read_dir(tasks_dir).map_err(|error| unavailable(tasks_dir, &error))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter(|path| !file_name(path).starts_with(TEMPORARY_PREFIX))
        .collect();
    files.sort();
    Ok(files)
}

/// Deux fichiers pour la même tâche (copie de conflit créée par le drive, `x (1).md`…) :
/// on garde la version la plus récente et on signale l'autre, en attendant que le scan
/// ([`FsDriveSync`]) règle le conflit et range la perdante dans `.conflicts/`.
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
