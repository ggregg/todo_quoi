//! Tous les faux ports assemblés, avec un raccourci par cas d'usage.
//!
//! C'est une mini « composition root » de test : la même chose que ce que fera `src-tauri`,
//! mais avec des faux au lieu des vrais adaptateurs.

use crate::fakes::{
    InMemoryTaskRepository, RecordingEventPublisher, SequentialIdGenerator, SteppingClock,
};
use std::sync::Arc;
use tq_application::Stamper;
use tq_application::use_cases::{
    close_task::CloseTask, create_task::CreateTask, delete_task::DeleteTask,
    get_close_impact::GetCloseImpact, list_tasks::ListTasks, list_trash::ListTrash,
    move_task::MoveTask, rename_task::RenameTask, reopen_task::ReopenTask,
    restore_task::RestoreTask, set_tags::SetTags,
};
use tq_domain::{DeviceId, Task};

/// Application de test.
pub struct TestApp {
    /// Dépôt en mémoire (consultable directement par les tests).
    pub repository: Arc<InMemoryTaskRepository>,
    /// Événements publiés.
    pub events: Arc<RecordingEventPublisher>,
    ids: Arc<SequentialIdGenerator>,
    stamper: Stamper,
}

impl TestApp {
    /// Application vide.
    pub fn new() -> Self {
        Self::with_tasks(Vec::new())
    }

    /// Application contenant déjà ces tâches.
    pub fn with_tasks(tasks: Vec<Task>) -> Self {
        let device = DeviceId::from_machine_name("appareil-de-test");
        Self {
            repository: Arc::new(InMemoryTaskRepository::with_tasks(tasks)),
            events: Arc::new(RecordingEventPublisher::default()),
            ids: Arc::new(SequentialIdGenerator::default()),
            stamper: Stamper::new(Arc::new(SteppingClock::default()), device),
        }
    }

    /// Cas d'usage « créer ».
    pub fn create_task(&self) -> CreateTask {
        CreateTask::new(
            self.repository.clone(),
            self.ids.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « renommer ».
    pub fn rename_task(&self) -> RenameTask {
        RenameTask::new(
            self.repository.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « impact d'une clôture ».
    pub fn get_close_impact(&self) -> GetCloseImpact {
        GetCloseImpact::new(self.repository.clone())
    }
    /// Cas d'usage « clôturer ».
    pub fn close_task(&self) -> CloseTask {
        CloseTask::new(
            self.repository.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « rouvrir ».
    pub fn reopen_task(&self) -> ReopenTask {
        ReopenTask::new(
            self.repository.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « déplacer ».
    pub fn move_task(&self) -> MoveTask {
        MoveTask::new(
            self.repository.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « étiquettes ».
    pub fn set_tags(&self) -> SetTags {
        SetTags::new(
            self.repository.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « corbeille ».
    pub fn delete_task(&self) -> DeleteTask {
        DeleteTask::new(
            self.repository.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « restaurer ».
    pub fn restore_task(&self) -> RestoreTask {
        RestoreTask::new(
            self.repository.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « lister ».
    pub fn list_tasks(&self) -> ListTasks {
        ListTasks::new(self.repository.clone())
    }
    /// Cas d'usage « lister la corbeille ».
    pub fn list_trash(&self) -> ListTrash {
        ListTrash::new(self.repository.clone())
    }
}

impl Default for TestApp {
    fn default() -> Self {
        Self::new()
    }
}
