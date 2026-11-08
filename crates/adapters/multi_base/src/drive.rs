//! Synchronisation de plusieurs bases : un passage = un passage sur chaque base.

use std::sync::Arc;
use tq_application::ports::{ConflictCopy, DriveSync, RepositoryError, ScanReport};
use tq_domain::{BaseId, TaskId, Timestamp};

/// `DriveSync` de chaque base, présentés comme un seul (rapports additionnés).
pub struct MultiBaseDriveSync {
    bases: Vec<(BaseId, Arc<dyn DriveSync>)>,
}

impl MultiBaseDriveSync {
    /// Assemble les scans des bases ouvertes.
    pub fn new(bases: Vec<(BaseId, Arc<dyn DriveSync>)>) -> Self {
        Self { bases }
    }
}

impl DriveSync for MultiBaseDriveSync {
    /// Chaque base est scannée, même si une autre est inaccessible ; l'erreur (la première)
    /// est rendue **après** : l'indicateur passe au rouge en nommant la base.
    fn scan(&self) -> Result<ScanReport, RepositoryError> {
        let mut total = ScanReport::default();
        let mut failure = None;
        for (base, sync) in &self.bases {
            match sync.scan() {
                Ok(report) => {
                    total.changed.extend(report.changed);
                    total.pending_downloads += report.pending_downloads;
                    total.resolved_conflicts += report.resolved_conflicts;
                    total.unreadable_files += report.unreadable_files;
                }
                Err(error) => {
                    failure.get_or_insert(RepositoryError::Unavailable(format!(
                        "base « {base} » : {error}"
                    )));
                }
            }
        }
        failure.map_or(Ok(total), Err)
    }

    fn conflict_copies(&self) -> Result<Vec<ConflictCopy>, RepositoryError> {
        let mut copies = Vec::new();
        for (_, sync) in &self.bases {
            copies.extend(sync.conflict_copies()?);
        }
        copies.sort_by_key(|copy| std::cmp::Reverse(copy.set_aside_at));
        Ok(copies)
    }

    /// Seule la base qui contient la tâche a un fichier à mettre de côté ; les autres n'ont rien.
    fn set_aside_current(&self, id: &TaskId) -> Result<(), RepositoryError> {
        for (_, sync) in &self.bases {
            sync.set_aside_current(id)?;
        }
        Ok(())
    }

    fn purge_conflict_copies(&self, before: Timestamp) -> Result<usize, RepositoryError> {
        let mut total = 0;
        for (_, sync) in &self.bases {
            total += sync.purge_conflict_copies(before)?;
        }
        Ok(total)
    }
}
