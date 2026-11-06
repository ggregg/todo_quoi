//! Déplacer une tâche : `Tab` / `Maj+Tab`, `Alt+↑` / `Alt+↓`, glisser-déposer — R9 à R12.

use crate::AppError;
use crate::Stamper;
use crate::ports::{EventPublisher, TaskRepository};
use crate::session::TreeSession;
use std::sync::Arc;
use tq_domain::TaskId;
use tq_domain::rules::closure::{closed_ancestors, is_ready_to_close};
use tq_domain::rules::hierarchy::{
    apply_move, indent, move_down, move_up, outdent, placement_after,
};

/// Type de déplacement demandé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Movement {
    /// `Tab` : sous la tâche au-dessus (R9).
    Indent,
    /// `Maj+Tab` : au niveau du parent (R9).
    Outdent,
    /// `Alt+↑`.
    Up,
    /// `Alt+↓`.
    Down,
    /// Glisser-déposer (R10) : sous `parent` (`None` = premier niveau), juste après `after`
    /// (`None` = en tête).
    To {
        /// Nouveau parent.
        parent: Option<TaskId>,
        /// Sœur après laquelle se placer.
        after: Option<TaskId>,
    },
}

/// Résultat d'un déplacement (R12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveOutcome {
    /// L'ancien parent n'a plus que des sous-tâches terminées : proposer de le clôturer.
    pub propose_closing: Option<TaskId>,
    /// La tâche déplacée est ouverte sous des ancêtres terminés : proposer de les rouvrir.
    pub propose_reopening: Vec<TaskId>,
}

/// Cas d'usage « déplacer ».
pub struct MoveTask {
    tasks: Arc<dyn TaskRepository>,
    stamper: Stamper,
    events: Arc<dyn EventPublisher>,
}

impl MoveTask {
    /// Assemble le cas d'usage avec ses ports.
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        stamper: Stamper,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            tasks,
            stamper,
            events,
        }
    }

    /// Déplace `task_id` selon `movement`.
    ///
    /// # Erreurs
    /// Déplacement impossible (R9, R11, bord de liste), tâche introuvable, stockage.
    pub fn execute(&self, task_id: &TaskId, movement: Movement) -> Result<MoveOutcome, AppError> {
        let mut session = TreeSession::load(self.tasks.as_ref())?;
        let tree = session.tree();
        let old_parent = tree.effective_parent(tree.get(task_id)?);
        let placement = match movement {
            Movement::Indent => indent(tree, task_id)?,
            Movement::Outdent => outdent(tree, task_id)?,
            Movement::Up => move_up(tree, task_id)?,
            Movement::Down => move_down(tree, task_id)?,
            Movement::To { parent, after } => {
                placement_after(tree, Some(task_id), parent.as_ref(), after.as_ref())?
            }
        };
        let parent_changed = placement.parent != old_parent;
        apply_move(
            session.tree_mut(),
            task_id,
            placement,
            &self.stamper.stamp(),
        )?;

        let outcome = self.proposals(
            session.tree(),
            task_id,
            old_parent.filter(|_| parent_changed),
        )?;
        session.mark_changed([task_id.clone()]);
        session.commit(self.tasks.as_ref(), self.events.as_ref())?;
        Ok(outcome)
    }

    /// R12 : propositions sur l'ancien parent (clôture) et les nouveaux ancêtres (réouverture).
    fn proposals(
        &self,
        tree: &tq_domain::TaskTree,
        task_id: &TaskId,
        left_parent: Option<TaskId>,
    ) -> Result<MoveOutcome, AppError> {
        let propose_closing = left_parent.filter(|parent| is_ready_to_close(tree, parent));
        let propose_reopening = if tree.get(task_id)?.is_done() {
            Vec::new()
        } else {
            closed_ancestors(tree, task_id)?
        };
        Ok(MoveOutcome {
            propose_closing,
            propose_reopening,
        })
    }
}
