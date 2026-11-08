//! **DTO** (*Data Transfer Objects*) : la forme JSON des données échangées avec l'interface.
//!
//! Pourquoi ne pas envoyer directement les types du domaine ? Pour que le domaine reste libre
//! d'évoluer sans casser l'interface, et inversement. Ici on ne fait que **traduire**.
//! Les types TypeScript correspondants sont dans `ui/src/domain/task.ts` (même noms de champs).
//!
//! Convention JSON : `camelCase` (`parentId`), comme en TypeScript.

use crate::error::UiError;
use serde::{Deserialize, Serialize};
use tq_application::use_cases::close_task::{CloseOutcome, CloseScope};
use tq_application::use_cases::create_task::Position;
use tq_application::use_cases::delete_task::DeleteOutcome;
use tq_application::use_cases::move_task::{MoveOutcome, Movement};
use tq_application::use_cases::reopen_task::ReopenOutcome;
use tq_domain::rules::trash::RestoreOutcome;
use tq_domain::{Task, TaskId, TaskStatus};

/// Une tâche, telle que l'interface l'affiche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    /// Identifiant.
    pub id: String,
    /// Parent (`null` = premier niveau).
    pub parent_id: Option<String>,
    /// Clé d'ordre (l'interface trie les sœurs par ordre de texte).
    pub order: String,
    /// Titre.
    pub title: String,
    /// Terminée ?
    pub done: bool,
    /// Date de clôture (RFC 3339).
    pub done_at: Option<String>,
    /// Étiquettes.
    pub tags: Vec<String>,
    /// La note contient-elle du texte ? (le contenu arrive en I2)
    pub has_note: bool,
    /// Création (RFC 3339).
    pub created_at: String,
    /// Dernière modification (RFC 3339).
    pub updated_at: String,
    /// Appareil de la dernière modification.
    pub updated_by: String,
    /// Date de mise à la corbeille, le cas échéant.
    pub deleted_at: Option<String>,
    /// Base qui contient la tâche (ADR-0009).
    pub base: String,
}

impl From<&Task> for TaskDto {
    fn from(task: &Task) -> Self {
        let done_at = match task.status() {
            TaskStatus::Open => None,
            TaskStatus::Done { at } => Some(at.to_rfc3339()),
        };
        Self {
            id: task.id().to_string(),
            parent_id: task.parent().map(ToString::to_string),
            order: task.order().to_string(),
            title: task.title().to_string(),
            done: task.is_done(),
            done_at,
            tags: task.tags().iter().map(ToString::to_string).collect(),
            has_note: !task.note().is_blank(),
            created_at: task.created_at().to_rfc3339(),
            updated_at: task.updated_at().to_rfc3339(),
            updated_by: task.updated_by().to_string(),
            deleted_at: task.deleted_at().map(|at| at.to_rfc3339()),
            base: task.base().to_string(),
        }
    }
}

/// Lit un identifiant de base envoyé par l'interface.
///
/// # Erreurs
/// [`UiError`] `invalid-input` si l'identifiant est mal formé.
pub fn parse_base_id(text: &str) -> Result<tq_domain::BaseId, UiError> {
    tq_domain::BaseId::parse(text).map_err(|error| UiError::invalid_input(error.to_string()))
}

/// Lit un identifiant envoyé par l'interface.
///
/// # Erreurs
/// [`UiError`] `invalid-input` si ce n'est pas un UUID.
pub fn parse_task_id(text: &str) -> Result<TaskId, UiError> {
    TaskId::parse(text).map_err(|error| UiError::invalid_input(error.to_string()))
}

fn optional_task_id(text: Option<&str>) -> Result<Option<TaskId>, UiError> {
    text.map(parse_task_id).transpose()
}

fn ids(ids: &[TaskId]) -> Vec<String> {
    ids.iter().map(ToString::to_string).collect()
}

// ----- Entrées ---------------------------------------------------------------------------------

/// Où créer une tâche. JSON : `{ "kind": "atEnd", "parentId": null }` ou
/// `{ "kind": "after", "siblingId": "…" }`.
// [rust] `tag = "kind"` : serde lit le champ `kind` pour savoir quelle variante construire
//        (≈ une union discriminée TypeScript).
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PositionDto {
    /// À la fin des enfants de `parent_id`.
    AtEnd {
        /// Parent (`null` = premier niveau).
        parent_id: Option<String>,
    },
    /// Juste après une tâche.
    After {
        /// Tâche de référence.
        sibling_id: String,
    },
    /// À la fin du premier niveau d'une base (ADR-0009).
    AtTopOf {
        /// Base visée.
        base: String,
    },
}

impl PositionDto {
    /// Traduction vers le type de l'application.
    ///
    /// # Erreurs
    /// Identifiant mal formé.
    pub fn into_position(self) -> Result<Position, UiError> {
        Ok(match self {
            Self::AtEnd { parent_id } => Position::AtEndOf {
                parent: optional_task_id(parent_id.as_deref())?,
            },
            Self::After { sibling_id } => Position::After {
                sibling: parse_task_id(&sibling_id)?,
            },
            Self::AtTopOf { base } => Position::AtTopOf {
                base: parse_base_id(&base)?,
            },
        })
    }
}

/// Réponse de l'utilisateur à « clôturer aussi les sous-tâches ouvertes ? » (R2).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CloseScopeDto {
    /// Seulement la tâche.
    TaskOnly,
    /// Avec ses sous-tâches ouvertes.
    WithOpenDescendants,
}

impl From<CloseScopeDto> for CloseScope {
    fn from(scope: CloseScopeDto) -> Self {
        match scope {
            CloseScopeDto::TaskOnly => Self::TaskOnly,
            CloseScopeDto::WithOpenDescendants => Self::WithOpenDescendants,
        }
    }
}

/// Déplacement. JSON : `{ "kind": "indent" }`, … ou
/// `{ "kind": "to", "parentId": …, "afterId": … }`.
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MovementDto {
    /// `Tab`.
    Indent,
    /// `Maj+Tab`.
    Outdent,
    /// `Alt+↑`.
    Up,
    /// `Alt+↓`.
    Down,
    /// Glisser-déposer.
    To {
        /// Nouveau parent.
        parent_id: Option<String>,
        /// Sœur après laquelle se placer (`null` = en tête).
        after_id: Option<String>,
    },
    /// Vers une autre base, à la fin de son premier niveau (ADR-0009).
    ToBase {
        /// Base visée.
        base: String,
    },
}

impl MovementDto {
    /// Traduction vers le type de l'application.
    ///
    /// # Erreurs
    /// Identifiant mal formé.
    pub fn into_movement(self) -> Result<Movement, UiError> {
        Ok(match self {
            Self::Indent => Movement::Indent,
            Self::Outdent => Movement::Outdent,
            Self::Up => Movement::Up,
            Self::Down => Movement::Down,
            Self::To {
                parent_id,
                after_id,
            } => Movement::To {
                parent: optional_task_id(parent_id.as_deref())?,
                after: optional_task_id(after_id.as_deref())?,
            },
            Self::ToBase { base } => Movement::ToBase {
                base: parse_base_id(&base)?,
            },
        })
    }
}

// ----- Sorties ---------------------------------------------------------------------------------

/// Résultat d'une clôture.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseOutcomeDto {
    /// Tâches clôturées.
    pub closed: Vec<String>,
    /// R1 : parent à proposer à la clôture.
    pub propose_closing_parent: Option<String>,
}

impl From<CloseOutcome> for CloseOutcomeDto {
    fn from(outcome: CloseOutcome) -> Self {
        Self {
            closed: ids(&outcome.closed),
            propose_closing_parent: outcome.propose_closing_parent.map(|id| id.to_string()),
        }
    }
}

/// Résultat d'une réouverture.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReopenOutcomeDto {
    /// R3 : ancêtres terminés à proposer à la réouverture.
    pub propose_reopening: Vec<String>,
}

impl From<ReopenOutcome> for ReopenOutcomeDto {
    fn from(outcome: ReopenOutcome) -> Self {
        Self {
            propose_reopening: ids(&outcome.propose_reopening),
        }
    }
}

/// Résultat d'un déplacement.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcomeDto {
    /// R12 : ancien parent à proposer à la clôture.
    pub propose_closing: Option<String>,
    /// R12 : nouveaux ancêtres terminés à proposer à la réouverture.
    pub propose_reopening: Vec<String>,
}

impl From<MoveOutcome> for MoveOutcomeDto {
    fn from(outcome: MoveOutcome) -> Self {
        Self {
            propose_closing: outcome.propose_closing.map(|id| id.to_string()),
            propose_reopening: ids(&outcome.propose_reopening),
        }
    }
}

/// Résultat d'une mise à la corbeille.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOutcomeDto {
    /// Tâches mises à la corbeille.
    pub deleted: Vec<String>,
}

impl From<DeleteOutcome> for DeleteOutcomeDto {
    fn from(outcome: DeleteOutcome) -> Self {
        Self {
            deleted: ids(&outcome.deleted),
        }
    }
}

/// Résultat d'une restauration.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcomeDto {
    /// Tâches restaurées.
    pub restored: Vec<String>,
    /// Rattachée au premier niveau faute de parent (R7).
    pub moved_to_root: bool,
}

impl From<RestoreOutcome> for RestoreOutcomeDto {
    fn from(outcome: RestoreOutcome) -> Self {
        Self {
            restored: ids(&outcome.restored),
            moved_to_root: outcome.moved_to_root,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Contrat JSON avec l'interface : ces formes sont celles attendues par `ui/src/domain/task.ts`.
    use super::*;
    use serde_json::json;

    #[test]
    fn positions_are_read_from_their_json_form() {
        let at_end: PositionDto =
            serde_json::from_value(json!({ "kind": "atEnd", "parentId": null })).unwrap();
        assert!(matches!(
            at_end.into_position(),
            Ok(Position::AtEndOf { parent: None })
        ));
        let after: PositionDto = serde_json::from_value(
            json!({ "kind": "after", "siblingId": "00000000-0000-7000-8000-000000000001" }),
        )
        .unwrap();
        assert!(matches!(after.into_position(), Ok(Position::After { .. })));
    }

    #[test]
    fn movements_and_scopes_are_read_from_their_json_form() {
        let indent: MovementDto = serde_json::from_value(json!({ "kind": "indent" })).unwrap();
        assert!(matches!(indent.into_movement(), Ok(Movement::Indent)));
        let to: MovementDto =
            serde_json::from_value(json!({ "kind": "to", "parentId": null, "afterId": null }))
                .unwrap();
        assert!(matches!(
            to.into_movement(),
            Ok(Movement::To {
                parent: None,
                after: None
            })
        ));
        let scope: CloseScopeDto = serde_json::from_value(json!("withOpenDescendants")).unwrap();
        assert_eq!(CloseScope::from(scope), CloseScope::WithOpenDescendants);
    }

    #[test]
    fn a_malformed_id_is_an_invalid_input() {
        let bad: PositionDto =
            serde_json::from_value(json!({ "kind": "after", "siblingId": "x" })).unwrap();
        assert_eq!(bad.into_position().unwrap_err().code, "invalid-input");
    }
}
