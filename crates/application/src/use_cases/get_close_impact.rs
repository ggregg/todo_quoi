//! Avant de clôturer : combien de sous-tâches sont encore ouvertes ? (R2)
//!
//! L'interface appelle ce cas d'usage pour savoir s'il faut demander
//! « Clôturer aussi les N sous-tâches ouvertes ? » avant d'appeler [`super::close_task`].
//! Séparer la **question** (lecture) de l'**action** (écriture) garde chaque cas d'usage simple.

use crate::AppError;
use crate::ports::TaskRepository;
use std::sync::Arc;
use tq_domain::rules::closure::open_descendants;
use tq_domain::{TaskId, TaskTree};

/// Conséquences d'une clôture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseImpact {
    /// Nombre de sous-tâches (toutes profondeurs) encore ouvertes.
    pub open_descendants: usize,
}

/// Cas d'usage « mesurer l'impact d'une clôture » (lecture seule).
pub struct GetCloseImpact {
    tasks: Arc<dyn TaskRepository>,
}

impl GetCloseImpact {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(tasks: Arc<dyn TaskRepository>) -> Self {
        Self { tasks }
    }

    /// Calcule l'impact de la clôture de `task_id`.
    ///
    /// # Erreurs
    /// Tâche introuvable, stockage.
    pub fn execute(&self, task_id: &TaskId) -> Result<CloseImpact, AppError> {
        let tree = TaskTree::new(self.tasks.load_all()?)?;
        tree.get(task_id)?;
        Ok(CloseImpact {
            open_descendants: open_descendants(&tree, task_id).len(),
        })
    }
}
