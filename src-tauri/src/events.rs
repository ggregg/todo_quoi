//! Implémentation du port `EventPublisher` : transmet les événements à l'interface.
//!
//! Côté TypeScript : `listen('tasks-changed', (event) => …)`.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tq_application::ports::{AppEvent, EventPublisher};

/// Nom de l'événement « des tâches ont changé ».
pub const TASKS_CHANGED: &str = "tasks-changed";

/// Contenu de l'événement `tasks-changed`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TasksChangedPayload {
    /// Identifiants des tâches créées ou modifiées.
    pub ids: Vec<String>,
}

/// Publie les événements de l'application vers toutes les fenêtres.
pub struct TauriEventPublisher {
    app: AppHandle,
}

impl TauriEventPublisher {
    /// Publie via cette instance d'application.
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventPublisher for TauriEventPublisher {
    fn publish(&self, event: AppEvent) {
        let AppEvent::TasksChanged { ids } = event;
        let payload = TasksChangedPayload {
            ids: ids.iter().map(ToString::to_string).collect(),
        };
        // Le port interdit d'échouer : un événement perdu ne doit pas annuler l'action déjà
        // enregistrée. On le signale seulement sur la console.
        if let Err(error) = self.app.emit(TASKS_CHANGED, payload) {
            eprintln!("événement {TASKS_CHANGED} non transmis : {error}");
        }
    }
}
