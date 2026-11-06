//! Fenêtres de tâche (spécification §5.4bis) : une tâche peut s'ouvrir seule dans sa propre
//! fenêtre système. Une fenêtre **par tâche** : la rouvrir la ramène au premier plan.
//!
//! La fenêtre charge la même interface avec `#task=<id>` : l'interface affiche alors le seul
//! panneau de détail de cette tâche.

use super::run;
use crate::composition::AppState;
use crate::dto::parse_task_id;
use crate::error::UiError;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

/// Ouvre (ou ramène au premier plan) la fenêtre de la tâche. `invoke('open_task_window', { taskId })`.
#[tauri::command]
pub async fn open_task_window(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), UiError> {
    let id = parse_task_id(&task_id)?;
    let label = format!("task-{id}");
    if let Some(existing) = app.get_webview_window(&label) {
        return existing
            .set_focus()
            .map_err(|error| UiError::internal(error.to_string()));
    }
    let use_case = state.get_task.clone();
    let lookup_id = id.clone();
    let title = run(move || Ok(use_case.execute(&lookup_id)?.title().to_string())).await?;
    WebviewWindowBuilder::new(
        &app,
        label,
        WebviewUrl::App(format!("index.html#task={id}").into()),
    )
    .title(format!("TodoQuoi — {title}"))
    .inner_size(560.0, 640.0)
    .min_inner_size(380.0, 320.0)
    .build()
    .map(|_| ())
    .map_err(|error| UiError::internal(format!("fenêtre non ouverte : {error}")))
}
