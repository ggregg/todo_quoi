//! Créer une tâche (raccourcis `Mod+N` : sœur suivante ; `Mod+Maj+N` : sous-tâche).

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, IdGenerator, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::rules::hierarchy::{Placement, placement_after};
use tq_domain::{BaseId, DomainError, NewTask, Task, TaskId, TaskTree, Title};

/// Où placer la nouvelle tâche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    /// À la fin des enfants de `parent` (`None` = fin du premier niveau).
    AtEndOf {
        /// Parent ; `None` = premier niveau.
        parent: Option<TaskId>,
    },
    /// Juste après une tâche existante, au même niveau.
    After {
        /// La tâche après laquelle insérer.
        sibling: TaskId,
    },
    /// À la fin du premier niveau d'une base précise (section du mode « Tout », ADR-0009).
    AtTopOf {
        /// Base visée.
        base: BaseId,
    },
}

/// Commande de création.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskCommand {
    /// Titre saisi.
    pub title: String,
    /// Emplacement.
    pub position: Position,
}

/// Cas d'usage « créer une tâche ».
pub struct CreateTask {
    tasks: Arc<dyn TaskRepository>,
    ids: Arc<dyn IdGenerator>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl CreateTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        ids: Arc<dyn IdGenerator>,
        stamper: Stamper,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            tasks,
            ids,
            stamper,
            events,
        }
    }

    /// Crée la tâche et renvoie son identifiant.
    ///
    /// # Erreurs
    /// Titre invalide, parent ou tâche de référence introuvable / dans la corbeille, stockage.
    pub fn execute(&self, command: CreateTaskCommand) -> Result<TaskId, AppError> {
        let title = Title::parse(&command.title)?;
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let placement = placement_for(session.tree(), &command.position)?;

        let base = base_for(session.tree(), &command.position, placement.parent.as_ref())?;
        let id = self.ids.new_task_id();
        let new = NewTask {
            id: id.clone(),
            parent: placement.parent,
            order: placement.order,
            title,
        };
        session
            .tree_mut()
            .insert(Task::create(new, &self.stamper.stamp()).in_base(base))?;
        session.mark_changed([id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(id)
    }
}

/// Traduit la position demandée en parent + clé d'ordre, en vérifiant que la cible est active.
fn placement_for(
    tree: &tq_domain::TaskTree,
    position: &Position,
) -> Result<Placement, DomainError> {
    match position {
        Position::AtEndOf { parent } => {
            if let Some(parent_id) = parent
                && tree.get(parent_id)?.is_deleted()
            {
                return Err(DomainError::TaskInTrash(parent_id.clone()));
            }
            Ok(Placement {
                parent: parent.clone(),
                order: tree.order_at_end(parent.as_ref()),
                base: None,
            })
        }
        Position::AtTopOf { base } => {
            let last = tree.roots_in(base).last().map(|task| task.id().clone());
            placement_after(tree, None, None, last.as_ref())
        }
        Position::After { sibling } => {
            let sibling_task = tree.get(sibling)?;
            if sibling_task.is_deleted() {
                return Err(DomainError::TaskInTrash(sibling.clone()));
            }
            let parent = tree.effective_parent(sibling_task);
            placement_after(tree, None, parent.as_ref(), Some(sibling))
        }
    }
}

/// Base de la nouvelle tâche : celle de son parent, de sa sœur, celle demandée, sinon la base
/// principale (une seule base ouverte).
fn base_for(
    tree: &TaskTree,
    position: &Position,
    parent: Option<&TaskId>,
) -> Result<BaseId, DomainError> {
    if let Some(parent) = parent {
        return Ok(tree.get(parent)?.base().clone());
    }
    Ok(match position {
        Position::AtTopOf { base } => base.clone(),
        Position::After { sibling } => tree.get(sibling)?.base().clone(),
        Position::AtEndOf { .. } => BaseId::main(),
    })
}
