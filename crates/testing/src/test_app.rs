//! Tous les faux ports assemblés, avec un raccourci par cas d'usage.
//!
//! C'est une mini « composition root » de test : la même chose que ce que fera `src-tauri`,
//! mais avec des faux au lieu des vrais adaptateurs. Comme en production, les cas d'usage
//! passent par [`IndexingTaskRepository`] : l'index de recherche suit chaque modification.

use crate::fakes::{
    FixedBases, InMemoryRootViewStore, InMemorySearchIndex, InMemoryTaskRepository,
    PlainNoteDigester, RecordingEventPublisher, ScriptedDriveSync, SequentialIdGenerator,
    SteppingClock,
};
use std::sync::Arc;
use tq_application::ports::TaskRepository;
use tq_application::use_cases::{
    close_task::CloseTask, create_task::CreateTask, customize_list_view::CustomizeListView,
    delete_task::DeleteTask, edit_note::EditNote, get_close_impact::GetCloseImpact,
    get_task::GetTask, inherit_list_view::InheritListView, keep_local_version::KeepLocalVersion,
    list_conflicts::ListConflicts, list_tasks::ListTasks, list_trash::ListTrash,
    move_task::MoveTask, purge_expired::PurgeExpired, rename_task::RenameTask,
    reopen_task::ReopenTask, restore_task::RestoreTask, search_tasks::SearchTasks,
    set_tags::SetTags, sync_with_drive::SyncWithDrive,
};
use tq_application::{IndexingTaskRepository, Stamper};
use tq_domain::{DeviceId, Task};

/// Application de test.
pub struct TestApp {
    /// Dépôt en mémoire (consultable directement par les tests).
    pub repository: Arc<InMemoryTaskRepository>,
    /// Index de recherche en mémoire (consultable directement par les tests).
    pub index: Arc<InMemorySearchIndex>,
    /// Vue racine en mémoire.
    pub root_view: Arc<InMemoryRootViewStore>,
    /// Synchronisation simulée.
    pub drive: Arc<ScriptedDriveSync>,
    /// Bases ouvertes (la base principale, sauf si le test en fixe d'autres).
    pub bases: Arc<FixedBases>,
    /// Horloge contrôlée (avance d'une milliseconde à chaque lecture).
    pub clock: Arc<SteppingClock>,
    /// Événements publiés.
    pub events: Arc<RecordingEventPublisher>,
    /// Ce que voient les cas d'usage : le dépôt, enveloppé par l'indexation.
    store: Arc<dyn TaskRepository>,
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
        let clock = Arc::new(SteppingClock::default());
        let repository = Arc::new(InMemoryTaskRepository::with_tasks(tasks));
        let index = Arc::new(InMemorySearchIndex::default());
        let store = Arc::new(IndexingTaskRepository::new(
            repository.clone(),
            index.clone(),
            Arc::new(PlainNoteDigester),
        ));
        Self {
            repository,
            index,
            store,
            root_view: Arc::new(InMemoryRootViewStore::default()),
            drive: Arc::new(ScriptedDriveSync::default()),
            bases: Arc::new(FixedBases::default()),
            clock: clock.clone(),
            events: Arc::new(RecordingEventPublisher::default()),
            ids: Arc::new(SequentialIdGenerator::default()),
            stamper: Stamper::new(clock, device),
        }
    }

    /// Cas d'usage « créer ».
    pub fn create_task(&self) -> CreateTask {
        CreateTask::new(
            self.store.clone(),
            self.ids.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « renommer ».
    pub fn rename_task(&self) -> RenameTask {
        RenameTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « impact d'une clôture ».
    pub fn get_close_impact(&self) -> GetCloseImpact {
        GetCloseImpact::new(self.store.clone())
    }
    /// Cas d'usage « clôturer ».
    pub fn close_task(&self) -> CloseTask {
        CloseTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « rouvrir ».
    pub fn reopen_task(&self) -> ReopenTask {
        ReopenTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « déplacer ».
    pub fn move_task(&self) -> MoveTask {
        MoveTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « étiquettes ».
    pub fn set_tags(&self) -> SetTags {
        SetTags::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « corbeille ».
    pub fn delete_task(&self) -> DeleteTask {
        DeleteTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « restaurer ».
    pub fn restore_task(&self) -> RestoreTask {
        RestoreTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « lire une tâche ».
    pub fn get_task(&self) -> GetTask {
        GetTask::new(self.store.clone())
    }
    /// Cas d'usage « modifier la note ».
    pub fn edit_note(&self) -> EditNote {
        EditNote::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « lister ».
    pub fn list_tasks(&self) -> ListTasks {
        ListTasks::new(
            self.store.clone(),
            self.root_view.clone(),
            self.index.clone(),
            self.bases.clone(),
        )
    }
    /// Cas d'usage « rechercher ».
    pub fn search_tasks(&self) -> SearchTasks {
        SearchTasks::new(self.index.clone())
    }
    /// Cas d'usage « personnaliser une liste ».
    pub fn customize_list_view(&self) -> CustomizeListView {
        CustomizeListView::new(
            self.store.clone(),
            self.root_view.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « revenir à l'héritage ».
    pub fn inherit_list_view(&self) -> InheritListView {
        InheritListView::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « se synchroniser ».
    pub fn sync_with_drive(&self) -> SyncWithDrive {
        SyncWithDrive::new(self.drive.clone(), self.events.clone())
    }
    /// Cas d'usage « vider ce qui a expiré ».
    pub fn purge_expired(&self) -> PurgeExpired {
        PurgeExpired::new(
            self.store.clone(),
            self.drive.clone(),
            self.clock.clone(),
            self.events.clone(),
        )
    }
    /// Cas d'usage « garder ma version ».
    pub fn keep_local_version(&self) -> KeepLocalVersion {
        KeepLocalVersion::new(self.drive.clone())
    }
    /// Cas d'usage « journal des conflits ».
    pub fn list_conflicts(&self) -> ListConflicts {
        ListConflicts::new(self.drive.clone())
    }
    /// Cas d'usage « lister la corbeille ».
    pub fn list_trash(&self) -> ListTrash {
        ListTrash::new(self.store.clone())
    }
}

impl Default for TestApp {
    fn default() -> Self {
        Self::new()
    }
}
