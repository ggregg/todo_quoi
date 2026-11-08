//! Le scan du dossier du drive (architecture §5.2–5.4) : implémentation du port
//! [`DriveSync`](tq_application::ports::DriveSync).
//!
//! À chaque passage (toutes les 3 s, appelé par `src-tauri`) :
//! 1. **copies de conflit** du drive (`<id> (1).md`, `<id> (conflicted copy …).md`,
//!    `<id>-DESKTOP-ABC.md`…) : la version la plus récente devient `<id>.md`, l'autre est rangée
//!    dans `.conflicts/` (règle du domaine `rules::conflict::is_newer`) ;
//! 2. **empreintes** (taille, date) de chaque fichier, comparées au passage précédent : ce qui a
//!    changé l'a été par un autre appareil (nos écritures mettent les empreintes à jour) ;
//! 3. **fichiers à la demande** : comptés et téléchargés en tâche de fond par une simple lecture
//!    (ADR-0008) ; ils seront vus au passage où ils seront arrivés.
//!
//! Aucune lecture de contenu pour un fichier inchangé : un passage coûte quelques `stat()`.

use crate::conflicts;
use crate::file_state::{FileStamp, KnownFiles, KnownFilesState, is_dataless};
use crate::format::parse_task_file;
use crate::storage::{task_files, write_atomically};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tq_application::ports::{ConflictCopy, DriveSync, RepositoryError, ScanReport};
use tq_domain::rules::conflict::is_newer;
use tq_domain::{TaskId, Timestamp};

/// Scan du dossier `tasks/` d'un espace. Obtenu par `FsMarkdownTaskRepository::drive_sync`.
pub struct FsDriveSync {
    root: PathBuf,
    tasks_dir: PathBuf,
    known: KnownFiles,
    /// Fichiers en cours de téléchargement (pour ne pas lancer deux lectures du même).
    downloading: Arc<Mutex<HashSet<PathBuf>>>,
}

impl FsDriveSync {
    pub(crate) fn new(root: PathBuf, tasks_dir: PathBuf, known: KnownFiles) -> Self {
        Self {
            root,
            tasks_dir,
            known,
            downloading: Arc::default(),
        }
    }

    /// Empreintes des fichiers présents localement ; les autres sont comptés et demandés.
    fn current_stamps(
        &self,
        report: &mut ScanReport,
    ) -> Result<HashMap<PathBuf, FileStamp>, RepositoryError> {
        let mut stamps = HashMap::new();
        for path in task_files(&self.tasks_dir)? {
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            if is_dataless(&metadata) {
                report.pending_downloads += 1;
                self.download_in_background(path);
            } else {
                stamps.insert(path, FileStamp::of(&metadata));
            }
        }
        Ok(stamps)
    }

    /// Une lecture complète fait télécharger le fichier par le client du drive. Elle peut
    /// bloquer longtemps : elle a son propre fil, et le scan n'attend pas.
    fn download_in_background(&self, path: PathBuf) {
        let downloading = self.downloading.clone();
        if !lock(&downloading).insert(path.clone()) {
            return; // déjà demandé
        }
        std::thread::spawn(move || {
            let _ = fs::read(&path);
            lock(&downloading).remove(&path);
        });
    }

    /// Règle les copies de conflit du drive ; renvoie les tâches concernées.
    fn resolve_conflict_copies(
        &self,
        state: &mut KnownFilesState,
    ) -> Result<Vec<TaskId>, RepositoryError> {
        let mut resolved = Vec::new();
        for path in task_files(&self.tasks_dir)? {
            if file_task_id(&path).is_some()
                || fs::metadata(&path).is_ok_and(|metadata| is_dataless(&metadata))
            {
                continue; // nom normal, ou pas encore téléchargé
            }
            match self.resolve_copy(&path) {
                Ok(id) => resolved.push(id),
                Err(()) => {
                    state.unreadable.insert(path);
                }
            }
        }
        Ok(resolved)
    }

    /// Range la version perdante dans `.conflicts/` et laisse la gagnante sous `<id>.md`.
    fn resolve_copy(&self, copy_path: &Path) -> Result<TaskId, ()> {
        let copy_text = fs::read_to_string(copy_path).map_err(|_| ())?;
        let copy = parse_task_file(&copy_text).map_err(|_| ())?.task;
        let id = copy.id().clone();
        let canonical = self.tasks_dir.join(format!("{id}.md"));
        let current_text = fs::read_to_string(&canonical).ok();
        let current = current_text
            .as_deref()
            .and_then(|text| parse_task_file(text).ok())
            .map(|parsed| parsed.task);
        let copy_wins = current
            .as_ref()
            .is_none_or(|current| is_newer(&copy, current));
        let set_aside = |text: &str, device: &str| {
            conflicts::set_aside(&self.root, &id.to_string(), device, text)
        };
        if copy_wins {
            if let Some(text) = &current_text {
                let device = current
                    .as_ref()
                    .map(|task| task.updated_by().to_string())
                    .unwrap_or_default();
                set_aside(text, &device).map_err(|_| ())?;
            }
            write_atomically(&canonical, &copy_text).map_err(|_| ())?;
        } else {
            set_aside(&copy_text, copy.updated_by().as_str()).map_err(|_| ())?;
        }
        fs::remove_file(copy_path).map_err(|_| ())?;
        Ok(id)
    }

    /// Note les fichiers modifiés illisibles (ou redevenus lisibles) ; renvoie les tâches changées.
    fn changed_tasks(
        state: &mut KnownFilesState,
        current: &HashMap<PathBuf, FileStamp>,
    ) -> Vec<TaskId> {
        let mut changed = Vec::new();
        for (path, stamp) in current {
            if state.initialized && state.stamps.get(path) == Some(stamp) {
                continue;
            }
            let readable =
                fs::read_to_string(path).is_ok_and(|text| parse_task_file(&text).is_ok());
            if readable {
                state.unreadable.remove(path);
            } else {
                state.unreadable.insert(path.clone());
            }
            if state.initialized {
                changed.extend(file_task_id(path));
            }
        }
        if state.initialized {
            let removed = state
                .stamps
                .keys()
                .filter(|path| !current.contains_key(*path));
            changed.extend(removed.filter_map(|path| file_task_id(path)));
        }
        state.unreadable.retain(|path| path.exists());
        changed
    }
}

impl DriveSync for FsDriveSync {
    fn scan(&self) -> Result<ScanReport, RepositoryError> {
        // Le verrou empêche toute écriture du dépôt pendant le passage.
        let mut state = self.known.lock();
        let mut report = ScanReport::default();
        let resolved = self.resolve_conflict_copies(&mut state)?;
        report.resolved_conflicts = resolved.len();
        let current = self.current_stamps(&mut report)?;
        let mut changed = Self::changed_tasks(&mut state, &current);
        changed.extend(resolved);
        changed.sort();
        changed.dedup();
        report.changed = changed;
        report.unreadable_files = state.unreadable.len();
        state.stamps = current;
        state.initialized = true;
        Ok(report)
    }

    fn conflict_copies(&self) -> Result<Vec<ConflictCopy>, RepositoryError> {
        conflicts::list(&self.root)
    }

    fn set_aside_current(&self, id: &TaskId) -> Result<(), RepositoryError> {
        let path = self.tasks_dir.join(format!("{id}.md"));
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(()); // rien sur disque
        };
        let device = parse_task_file(&text)
            .map(|parsed| parsed.task.updated_by().to_string())
            .unwrap_or_default();
        conflicts::set_aside(&self.root, &id.to_string(), &device, &text)
            .map(|_| ())
            .map_err(|error| RepositoryError::Unavailable(format!("{} : {error}", path.display())))
    }

    fn purge_conflict_copies(&self, before: Timestamp) -> Result<usize, RepositoryError> {
        conflicts::purge(&self.root, before)
    }
}

/// Identifiant porté par un nom de fichier normal (`<uuid>.md`), sinon `None` (copie de conflit).
fn file_task_id(path: &Path) -> Option<TaskId> {
    TaskId::parse(&path.file_stem()?.to_string_lossy()).ok()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
