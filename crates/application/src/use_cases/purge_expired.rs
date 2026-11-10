//! Vider ce qui a expiré : la corbeille (R8, durée réglable, 30 jours par défaut) et les versions
//! écartées par les conflits (`.conflicts/`, 30 jours, spec §11). Appelé au démarrage puis une
//! fois par jour.

use crate::AppError;
use crate::ports::{AppEvent, Clock, DriveSync, EventPublisher, MediaStore, TaskRepository};
use crate::purging::erase_for_good;
use std::sync::Arc;
use tq_domain::TaskTree;
use tq_domain::rules::trash::{CONFLICT_RETENTION, TrashRetention, expired_in_trash};

/// Ce qui a été effacé.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PurgeOutcome {
    /// Tâches effacées de la corbeille.
    pub tasks: usize,
    /// Médias effacés avec elles (cités par aucune autre note de leur base).
    pub media: usize,
    /// Versions de conflit effacées.
    pub conflict_copies: usize,
}

/// Cas d'usage « vider ce qui a expiré ».
pub struct PurgeExpired {
    tasks: Arc<dyn TaskRepository>,
    media: Arc<dyn MediaStore>,
    drive: Arc<dyn DriveSync>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventPublisher>,
    retention: TrashRetention,
}

impl PurgeExpired {
    /// Assemble le cas d'usage avec ses ports ; `retention` : durée de la corbeille.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        media: Arc<dyn MediaStore>,
        drive: Arc<dyn DriveSync>,
        clock: Arc<dyn Clock>,
        events: Arc<dyn EventPublisher>,
        retention: TrashRetention,
    ) -> Self {
        Self {
            tasks,
            media,
            drive,
            clock,
            events,
            retention,
        }
    }

    /// Efface définitivement ce qui est en corbeille depuis plus que la durée réglée, et les
    /// versions écartées depuis plus de 30 jours.
    ///
    /// # Erreurs
    /// Stockage.
    pub fn execute(&self) -> Result<PurgeOutcome, AppError> {
        let now = self.clock.now();
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        let expired = expired_in_trash(&tree, now, self.retention);
        let media = erase_for_good(&tree, &expired, self.tasks.as_ref(), self.media.as_ref())?;
        let conflict_copies = self
            .drive
            .purge_conflict_copies(CONFLICT_RETENTION.limit(now))?;
        if !expired.is_empty() {
            self.events.publish(AppEvent::TasksChanged {
                ids: expired.clone(),
            });
        }
        Ok(PurgeOutcome {
            tasks: expired.len(),
            media,
            conflict_copies,
        })
    }
}
