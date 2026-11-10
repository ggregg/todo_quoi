//! Tous les faux ports assemblés, avec un raccourci par cas d'usage.
//!
//! C'est une mini « composition root » de test : la même chose que ce que fera `src-tauri`,
//! mais avec des faux au lieu des vrais adaptateurs. Comme en production, les cas d'usage
//! passent par [`IndexingTaskRepository`] : l'index de recherche suit chaque modification.

use crate::fakes::{
    FixedBases, InMemoryMediaStore, InMemoryRootViewStore, InMemorySearchIndex,
    InMemoryTaskRepository, PlainNoteDigester, RecordingEventPublisher, ScriptedDriveSync,
    SequentialIdGenerator, SteppingClock,
};
use crate::vault_fakes::{FakeCipher, FakeVaultKeys, InMemoryKeyringStore, InMemoryVaultStore};
use std::sync::Arc;
use tq_application::ports::TaskRepository;
use tq_application::use_cases::{
    auto_lock_vaults::AutoLockVaults, change_vault_password::ChangeVaultPassword,
    read_media::ReadMedia, store_media::StoreMedia,
};
use tq_application::use_cases::{
    close_task::CloseTask, create_task::CreateTask, customize_list_view::CustomizeListView,
    delete_task::DeleteTask, edit_note::EditNote, get_close_impact::GetCloseImpact,
    get_task::GetTask, inherit_list_view::InheritListView, keep_local_version::KeepLocalVersion,
    list_conflicts::ListConflicts, list_tasks::ListTasks, list_trash::ListTrash,
    move_task::MoveTask, purge_expired::PurgeExpired, purge_task::PurgeTask,
    rename_task::RenameTask, reopen_task::ReopenTask, restore_task::RestoreTask,
    search_tasks::SearchTasks, set_tags::SetTags, sync_with_drive::SyncWithDrive,
};
use tq_application::use_cases::{
    create_vault::CreateVault, decrypt_task::DecryptTask, encrypt_task::EncryptTask,
    list_vaults::ListVaults, lock_vault::LockVault,
    restore_remembered_vaults::RestoreRememberedVaults, unlock_vault::UnlockVault,
};
use tq_application::vault::{AutoLockDelay, OpenVaults, SealedMedia};
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
    /// Médias en mémoire (consultables directement par les tests).
    pub media: Arc<InMemoryMediaStore>,
    /// Synchronisation simulée.
    pub drive: Arc<ScriptedDriveSync>,
    /// Bases ouvertes (la base principale, sauf si le test en fixe d'autres).
    pub bases: Arc<FixedBases>,
    /// Horloge contrôlée (avance d'une milliseconde à chaque lecture).
    pub clock: Arc<SteppingClock>,
    /// Événements publiés.
    pub events: Arc<RecordingEventPublisher>,
    /// Coffres déverrouillés.
    pub vaults: Arc<OpenVaults>,
    /// Coffres des bases (manifestes).
    pub vault_store: Arc<InMemoryVaultStore>,
    /// Trousseau.
    pub keyring: Arc<InMemoryKeyringStore>,
    vault_keys: Arc<FakeVaultKeys>,
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
            media: Arc::new(InMemoryMediaStore::default()),
            bases: Arc::new(FixedBases::default()),
            clock: clock.clone(),
            events: Arc::new(RecordingEventPublisher::default()),
            ids: Arc::new(SequentialIdGenerator::default()),
            stamper: Stamper::new(clock, device),
            vaults: Arc::new(OpenVaults::new()),
            vault_store: Arc::new(InMemoryVaultStore::default()),
            keyring: Arc::new(InMemoryKeyringStore::default()),
            vault_keys: Arc::new(FakeVaultKeys::default()),
        }
    }

    /// Cas d'usage « supprimer définitivement ».
    pub fn purge_task(&self) -> PurgeTask {
        PurgeTask::new(self.store.clone(), self.media.clone(), self.events.clone())
    }

    /// Cas d'usage « créer le coffre d'une base ».
    pub fn create_vault(&self) -> CreateVault {
        CreateVault::new(
            self.vault_store.clone(),
            self.vault_keys.clone(),
            self.keyring.clone(),
            self.vaults.clone(),
        )
    }

    /// Cas d'usage « déverrouiller ».
    pub fn unlock_vault(&self) -> UnlockVault {
        UnlockVault::new(
            self.vault_store.clone(),
            self.vault_keys.clone(),
            self.keyring.clone(),
            self.vaults.clone(),
        )
    }

    /// Cas d'usage « verrouiller ».
    pub fn lock_vault(&self) -> LockVault {
        LockVault::new(self.vaults.clone())
    }

    /// Cas d'usage « état des coffres ».
    pub fn list_vaults(&self) -> ListVaults {
        ListVaults::new(
            self.bases.clone(),
            self.vault_store.clone(),
            self.vaults.clone(),
        )
    }

    /// Cas d'usage « rouvrir les coffres mémorisés ».
    pub fn restore_remembered_vaults(&self) -> RestoreRememberedVaults {
        RestoreRememberedVaults::new(
            self.bases.clone(),
            self.vault_store.clone(),
            self.keyring.clone(),
            self.vaults.clone(),
        )
    }

    /// Service des médias chiffrés (chiffrement simulé [`FakeCipher`]).
    pub fn sealed_media(&self) -> Arc<SealedMedia> {
        Arc::new(SealedMedia::new(
            self.media.clone(),
            self.vaults.clone(),
            Arc::new(FakeCipher),
        ))
    }

    /// Cas d'usage « chiffrer ».
    pub fn encrypt_task(&self) -> EncryptTask {
        EncryptTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
            self.vaults.clone(),
            self.sealed_media(),
        )
    }

    /// Cas d'usage « déchiffrer ».
    pub fn decrypt_task(&self) -> DecryptTask {
        DecryptTask::new(
            self.store.clone(),
            self.stamper.clone(),
            self.events.clone(),
            self.sealed_media(),
        )
    }

    /// Cas d'usage « stocker un média ».
    pub fn store_media(&self) -> StoreMedia {
        StoreMedia::new(self.media.clone(), self.store.clone(), self.sealed_media())
    }

    /// Cas d'usage « lire un média ».
    pub fn read_media(&self) -> ReadMedia {
        ReadMedia::new(self.media.clone(), self.sealed_media())
    }

    /// Cas d'usage « changer le mot de passe d'un coffre ».
    pub fn change_vault_password(&self) -> ChangeVaultPassword {
        ChangeVaultPassword::new(self.vault_store.clone(), self.vault_keys.clone())
    }

    /// Cas d'usage « verrouillage automatique ».
    pub fn auto_lock_vaults(&self, delay: AutoLockDelay) -> AutoLockVaults {
        AutoLockVaults::new(self.vaults.clone(), self.clock.clone(), delay)
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
            self.vaults.clone(),
            self.sealed_media(),
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
            self.media.clone(),
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
