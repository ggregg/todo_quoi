//! Se synchroniser avec le dossier du drive (spec §7) : appelé toutes les 3 s par le scanner de
//! `src-tauri`, et au retour au premier plan.

use crate::AppError;
use crate::ports::{AppEvent, DriveSync, EventPublisher, ScanReport};
use std::sync::Arc;

/// État affiché par l'indicateur de synchronisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// ● Tout est à jour.
    UpToDate,
    /// ⟳ Des fichiers attendent d'être téléchargés par le client du drive.
    Downloading {
        /// Nombre de fichiers.
        pending: usize,
    },
    /// ⚠ À regarder : fichiers illisibles, ou conflit réglé (version écartée dans `.conflicts/`).
    Attention {
        /// Fichiers illisibles.
        unreadable: usize,
        /// Conflits réglés pendant ce passage.
        conflicts: usize,
    },
}

impl SyncStatus {
    fn from_report(report: &ScanReport) -> Self {
        if report.unreadable_files > 0 || report.resolved_conflicts > 0 {
            return Self::Attention {
                unreadable: report.unreadable_files,
                conflicts: report.resolved_conflicts,
            };
        }
        if report.pending_downloads > 0 {
            return Self::Downloading {
                pending: report.pending_downloads,
            };
        }
        Self::UpToDate
    }
}

/// Cas d'usage « se synchroniser ».
pub struct SyncWithDrive {
    drive: Arc<dyn DriveSync>,
    events: Arc<dyn EventPublisher>,
}

impl SyncWithDrive {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(drive: Arc<dyn DriveSync>, events: Arc<dyn EventPublisher>) -> Self {
        Self { drive, events }
    }

    /// Un passage : annonce à l'interface les tâches changées ailleurs et renvoie l'état.
    ///
    /// # Erreurs
    /// Dossier du drive inaccessible.
    pub fn execute(&self) -> Result<SyncStatus, AppError> {
        let report = self.drive.scan()?;
        if !report.changed.is_empty() {
            self.events.publish(AppEvent::ChangedElsewhere {
                ids: report.changed.clone(),
            });
        }
        Ok(SyncStatus::from_report(&report))
    }
}
