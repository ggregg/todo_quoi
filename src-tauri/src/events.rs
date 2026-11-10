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
    /// Vrai si le changement vient d'un autre appareil (scan du drive).
    pub elsewhere: bool,
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
        // Un seul événement côté interface ; `elsewhere` dit s'il vient d'un autre appareil.
        let (ids, elsewhere) = match event {
            AppEvent::TasksChanged { ids } => (ids, false),
            AppEvent::ChangedElsewhere { ids } => (ids, true),
        };
        let payload = TasksChangedPayload {
            ids: ids.iter().map(ToString::to_string).collect(),
            elsewhere,
        };
        // Le port interdit d'échouer : un événement perdu ne doit pas annuler l'action déjà
        // enregistrée. On le signale seulement sur la console.
        if let Err(error) = self.app.emit(TASKS_CHANGED, payload) {
            eprintln!("événement {TASKS_CHANGED} non transmis : {error}");
        }
    }
}

/// Nom de l'événement « un coffre s'est ouvert ou fermé » (toutes les fenêtres relisent l'état
/// des coffres et les tâches, qui deviennent lisibles ou scellées).
pub const VAULTS_CHANGED: &str = "vaults-changed";

/// Prévient toutes les fenêtres qu'un coffre a changé d'état.
pub fn notify_vaults_changed(app: &AppHandle) {
    if let Err(error) = app.emit(VAULTS_CHANGED, ()) {
        eprintln!("événement {VAULTS_CHANGED} non transmis : {error}");
    }
}
