//! Commandes du panneau de détail : lire une tâche avec sa note, modifier la note.
//!
//! C'est ici, à la frontière, que la note passe du Markdown (stocké, compris par personne
//! dans l'application) au JSON de l'éditeur TipTap, et inversement (ADR-0007).

use super::run;
use crate::composition::LiveState;
use crate::dto::{TaskDto, parse_task_id};
use crate::error::UiError;
use serde::Serialize;
use tauri::State;
use tq_adapter_note_markdown::{PmNode, doc_to_markdown, markdown_to_doc};
use tq_domain::NoteBody;

/// Une tâche et sa note, au format de l'éditeur.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetailDto {
    /// La tâche.
    pub task: TaskDto,
    /// La note, en document TipTap (`{ "type": "doc", … }`).
    pub note: PmNode,
}

/// Lit une tâche et sa note. `invoke('get_task_detail', { taskId })`.
#[tauri::command]
pub async fn get_task_detail(
    state: State<'_, LiveState>,
    task_id: String,
) -> Result<TaskDetailDto, UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().get_task.clone();
    run(move || {
        let task = use_case.execute(&id)?;
        Ok(TaskDetailDto {
            task: TaskDto::from(&task),
            note: markdown_to_doc(task.note().as_markdown()),
        })
    })
    .await
}

/// Remplace la note par le document de l'éditeur. `invoke('update_note', { taskId, note })`.
#[tauri::command]
pub async fn update_note(
    state: State<'_, LiveState>,
    task_id: String,
    note: PmNode,
) -> Result<(), UiError> {
    let id = parse_task_id(&task_id)?;
    if note.kind != "doc" {
        return Err(UiError::invalid_input(
            "la note doit être un document TipTap (type « doc »)",
        ));
    }
    let markdown = NoteBody::from_markdown(doc_to_markdown(&note));
    let use_case = state.current().edit_note.clone();
    run(move || use_case.execute(&id, markdown)).await
}
