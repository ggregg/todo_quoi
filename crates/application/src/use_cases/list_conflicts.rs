//! Journal des conflits (spec §7) : les versions écartées, conservées 30 jours dans `.conflicts/`.

use crate::AppError;
use crate::ports::{ConflictCopy, DriveSync};
use std::sync::Arc;

/// Cas d'usage « lister les conflits » (lecture seule).
pub struct ListConflicts {
    drive: Arc<dyn DriveSync>,
}

impl ListConflicts {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(drive: Arc<dyn DriveSync>) -> Self {
        Self { drive }
    }

    /// Versions écartées, les plus récentes d'abord.
    ///
    /// # Erreurs
    /// Dossier illisible.
    pub fn execute(&self) -> Result<Vec<ConflictCopy>, AppError> {
        Ok(self.drive.conflict_copies()?)
    }
}
