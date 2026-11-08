//! « Garder la mienne » (spec §7) : la tâche ouverte a été modifiée sur un autre appareil pendant
//! la saisie ; l'utilisateur garde sa version. Celle de l'autre appareil n'est pas perdue : elle
//! est rangée dans `.conflicts/` avant que la saisie locale ne soit enregistrée.

use crate::AppError;
use crate::ports::DriveSync;
use std::sync::Arc;
use tq_domain::TaskId;

/// Cas d'usage « garder ma version ».
pub struct KeepLocalVersion {
    drive: Arc<dyn DriveSync>,
}

impl KeepLocalVersion {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(drive: Arc<dyn DriveSync>) -> Self {
        Self { drive }
    }

    /// Met de côté la version actuelle de `task_id` ; l'interface enregistre ensuite la sienne.
    ///
    /// # Erreurs
    /// Stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<(), AppError> {
        Ok(self.drive.set_aside_current(task_id)?)
    }
}
