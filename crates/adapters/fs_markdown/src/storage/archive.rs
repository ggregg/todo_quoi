//! Le dossier `archive/` d'une base (ADR-0015) : la partie du dépôt
//! [`FsMarkdownTaskRepository`] qui range les tâches archivées **à part**.
//!
//! - Une tâche archivée s'écrit dans `archive/<id>.md`, une tâche active dans `tasks/<id>.md` ;
//!   `save` déplace le fichier quand l'état change (archiver, restaurer).
//! - `archive/` n'est lu qu'à la demande ([`TaskRepository::load_archived`]) : ni le scan toutes
//!   les 3 s, ni les listes n'y touchent. Ses fichiers « à la demande » sont lus quand même : on
//!   attend leur téléchargement plutôt que d'ignorer les médias qu'ils citent.
//! - **Doublons** (R18) : un même identifiant dans les deux dossiers (archivée sur un appareil,
//!   modifiée sur l'autre) est réglé à la lecture des tâches actives : la version la plus récente
//!   reste, l'autre est mise de côté dans `.conflicts/` puis effacée.
//!
//! [`TaskRepository::load_archived`]: tq_application::ports::TaskRepository::load_archived

// [rust] Ce fichier est un **sous-module** de `storage` (fichier `storage/archive.rs`) : il voit
//        les champs privés de `FsMarkdownTaskRepository` et ajoute des méthodes avec son propre
//        bloc `impl` (≈ une classe TS répartie sur deux fichiers, impossible en TS sans mixin).

use super::{FsMarkdownTaskRepository, keep_most_recent, task_files};
use crate::manifest::{self, unavailable};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tq_application::ports::RepositoryError;
use tq_domain::rules::archive::same_apart_from_archiving;
use tq_domain::rules::conflict::is_newer;
use tq_domain::{Task, TaskId};

/// Nom du dossier des archives, à la racine de la base.
pub(crate) const ARCHIVE_DIR: &str = "archive";

impl FsMarkdownTaskRepository {
    pub(super) fn archive_path(&self, id: &TaskId) -> PathBuf {
        self.archive_dir.join(format!("{id}.md"))
    }

    /// Avant le premier archivage : version du manifeste relevée, dossier créé.
    pub(super) fn prepare_archives(&self) -> Result<(), RepositoryError> {
        manifest::require_archives(&self.root)?;
        fs::create_dir_all(&self.archive_dir)
            .map_err(|error| unavailable(&self.archive_dir, &error))
    }

    /// Identifiants présents dans `archive/`, d'après les **noms** de fichiers (rien n'est lu).
    pub(super) fn archived_ids(&self) -> HashSet<TaskId> {
        let Ok(entries) = fs::read_dir(&self.archive_dir) else {
            return HashSet::new(); // pas encore d'archive
        };
        entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let stem = path.file_stem()?.to_string_lossy().into_owned();
                TaskId::parse(&stem).ok()
            })
            .collect()
    }

    /// Toutes les archives. Un fichier illisible est ignoré (jamais réécrit) ; deux fichiers pour
    /// la même tâche (copie de conflit du drive) : le plus récent.
    pub(super) fn read_archives(&self) -> Result<Vec<Task>, RepositoryError> {
        if !self.archive_dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut by_id: HashMap<TaskId, (Task, PathBuf)> = HashMap::new();
        let mut ignored = Vec::new();
        for path in task_files(&self.archive_dir)? {
            if let Ok(task) = self.read_task(&path) {
                keep_most_recent(&mut by_id, as_archived(task), path, &mut ignored);
            }
        }
        Ok(by_id.into_values().map(|(task, _)| task).collect())
    }

    /// R18 — `active`, lue dans `tasks/`, a aussi un fichier dans `archive/`. La version la plus
    /// récente reste ; l'autre est mise de côté (sauf si elle ne diffère que par l'archivage :
    /// c'est la copie d'avant, que le drive n'a pas encore effacée) puis effacée.
    /// Renvoie `active` si elle l'emporte, `None` si c'est l'archive.
    pub(super) fn settle_duplicate(
        &self,
        active: Task,
        active_path: &Path,
    ) -> Result<Option<Task>, RepositoryError> {
        let archive_path = self.archive_path(active.id());
        let Ok(archived) = self.read_task(&archive_path).map(as_archived) else {
            return Ok(Some(active)); // archive illisible : on n'y touche pas
        };
        let archive_wins = is_newer(&archived, &active);
        let (loser_path, loser, winner) = if archive_wins {
            (active_path.to_path_buf(), &active, &archived)
        } else {
            (archive_path, &archived, &active)
        };
        if !same_apart_from_archiving(loser, winner) {
            self.set_aside_file(&loser_path)?;
        }
        let mut known = self.known.lock();
        remove_if_present(&loser_path)?;
        known.stamps.remove(&loser_path);
        Ok(if archive_wins { None } else { Some(active) })
    }
}

/// Une tâche lue dans `archive/` est archivée, même si son fichier ne porte pas la date
/// (fichier rangé à la main) : on prend alors sa date de modification. Le dossier fait foi.
pub(super) fn as_archived(task: Task) -> Task {
    if task.is_archived() {
        return task;
    }
    let mut record = task.to_record();
    record.archived_at = Some(record.updated_at);
    Task::from_record(record)
}

/// Une tâche lue dans `tasks/` est active, même si son fichier porte une date d'archivage
/// (fichier ressorti à la main d'`archive/`). Le dossier fait foi.
pub(super) fn as_active(task: Task) -> Task {
    if !task.is_archived() {
        return task;
    }
    let mut record = task.to_record();
    record.archived_at = None;
    Task::from_record(record)
}

/// Efface `path` ; un fichier déjà absent n'est pas une erreur.
pub(super) fn remove_if_present(path: &Path) -> Result<(), RepositoryError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(unavailable(path, &error)),
    }
}
