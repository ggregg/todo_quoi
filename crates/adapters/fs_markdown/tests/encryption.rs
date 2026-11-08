//! Tâches chiffrées dans de vrais fichiers (ADR-0010), avec le chiffrement simulé du kit de test.

#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tq_adapter_fs_markdown::{FsMarkdownTaskRepository, FsVaultStore};
use tq_application::ports::{RepositoryError, TaskRepository, VaultKeys, VaultStore};
use tq_application::vault::{BaseSealer, OpenVaults, Password, VaultError};
use tq_domain::{BaseId, Confidentiality};
use tq_testing::vault_fakes::{FakeCipher, FakeVaultKeys};
use tq_testing::{TaskBuilder, task_id};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "todoquoi-crypt-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Un dossier de base, son coffre ouvert, et un dépôt qui chiffre avec.
struct Base {
    dir: TempDir,
    vaults: Arc<OpenVaults>,
    key: (tq_domain::KeyId, tq_application::vault::ContentKey),
}

impl Base {
    fn with_open_vault() -> Self {
        let dir = TempDir::new();
        let vaults = Arc::new(OpenVaults::new());
        let password = Password::new("mot de passe".into()).unwrap();
        let (record, key) = FakeVaultKeys::default().create(&password).unwrap();
        vaults.open(BaseId::main(), record.key_id.clone(), key.clone());
        Self {
            dir,
            vaults,
            key: (record.key_id, key),
        }
    }

    fn repository(&self) -> FsMarkdownTaskRepository {
        let sealer = BaseSealer::new(BaseId::main(), self.vaults.clone(), Arc::new(FakeCipher));
        FsMarkdownTaskRepository::open(self.dir.path())
            .unwrap()
            .with_sealer(Arc::new(sealer))
    }

    fn lock(&self) {
        self.vaults.close(&BaseId::main());
    }

    fn unlock(&self) {
        let (key_id, key) = self.key.clone();
        self.vaults.open(BaseId::main(), key_id, key);
    }

    fn file_text(&self) -> String {
        let path = self
            .dir
            .path()
            .join("tasks")
            .join(format!("{}.md", task_id(1)));
        fs::read_to_string(path).unwrap()
    }
}

fn secret_task() -> tq_domain::Task {
    TaskBuilder::new(1, "Code du coffre-fort")
        .tags(&["perso"])
        .note("Le code est 1234")
        .encrypted()
        .build()
}

#[test]
fn only_the_note_of_an_encrypted_task_is_written_encrypted() {
    let base = Base::with_open_vault();
    base.repository().save(&secret_task()).unwrap();
    let text = base.file_text();
    assert!(text.contains("encrypted = true"));
    assert!(text.contains("schema_version = 2"));
    assert!(
        text.contains("title = \"Code du coffre-fort\""),
        "titre en clair"
    );
    assert!(text.contains("\"perso\""), "étiquettes en clair");
    assert!(!text.contains("1234"), "la note ne doit pas être en clair");
}

#[test]
fn with_the_vault_open_an_encrypted_task_reads_back_identical() {
    let base = Base::with_open_vault();
    let task = secret_task();
    base.repository().save(&task).unwrap();
    let loaded = base.repository().load_all().unwrap();
    assert_eq!(loaded, vec![task]);
}

#[test]
fn with_the_vault_locked_everything_but_the_note_reads_back() {
    let base = Base::with_open_vault();
    let task = TaskBuilder::new(1, "Secret")
        .parent(7)
        .tags(&["perso"])
        .note("Le code est 1234")
        .done()
        .encrypted()
        .build();
    base.repository().save(&task).unwrap();
    base.lock();
    let loaded = base.repository().load_all().unwrap().remove(0);
    assert!(loaded.is_sealed());
    assert_eq!(loaded.title().to_string(), "Secret");
    assert_eq!(loaded.tags(), task.tags());
    assert!(loaded.note().as_markdown().is_empty());
    assert_eq!(loaded.parent(), task.parent());
    assert!(loaded.is_done());
}

#[test]
fn a_sealed_task_renamed_while_locked_keeps_its_note() {
    let base = Base::with_open_vault();
    base.repository().save(&secret_task()).unwrap();
    base.lock();
    let mut sealed = base.repository().load_all().unwrap().remove(0);
    sealed
        .rename(
            tq_domain::Title::parse("Nouveau titre").unwrap(),
            &tq_domain::ChangeStamp {
                at: tq_domain::Timestamp::from_millis(5_000),
                by: tq_domain::DeviceId::parse("test").unwrap(),
            },
        )
        .unwrap();
    base.repository().save(&sealed).unwrap();
    base.unlock();
    let loaded = base.repository().load_all().unwrap().remove(0);
    assert_eq!(loaded.title().to_string(), "Nouveau titre");
    assert_eq!(loaded.note().as_markdown(), "Le code est 1234");
}

#[test]
fn a_sealed_task_closed_while_locked_keeps_its_content() {
    let base = Base::with_open_vault();
    base.repository().save(&secret_task()).unwrap();
    base.lock();
    let mut sealed = base.repository().load_all().unwrap().remove(0);
    sealed
        .close(&tq_domain::ChangeStamp {
            at: tq_domain::Timestamp::from_millis(5_000),
            by: tq_domain::DeviceId::parse("test").unwrap(),
        })
        .unwrap();
    base.repository().save(&sealed).unwrap(); // réécrit le même texte chiffré
    base.unlock();
    let loaded = base.repository().load_all().unwrap().remove(0);
    assert_eq!(loaded.confidentiality(), &Confidentiality::Encrypted);
    assert_eq!(loaded.note().as_markdown(), "Le code est 1234");
    assert!(loaded.is_done());
}

#[test]
fn saving_an_encrypted_task_with_the_vault_locked_is_refused() {
    let base = Base::with_open_vault();
    base.lock();
    let result = base.repository().save(&secret_task());
    assert_eq!(result, Err(RepositoryError::Vault(VaultError::Locked)));
}

#[test]
fn a_ciphertext_copied_into_another_task_does_not_open() {
    let base = Base::with_open_vault();
    base.repository().save(&secret_task()).unwrap();
    let text = base
        .file_text()
        .replace(&task_id(1).to_string(), &task_id(2).to_string());
    let other = base
        .dir
        .path()
        .join("tasks")
        .join(format!("{}.md", task_id(2)));
    fs::remove_file(
        base.dir
            .path()
            .join("tasks")
            .join(format!("{}.md", task_id(1))),
    )
    .unwrap();
    fs::write(other, text).unwrap();
    let loaded = base.repository().load_all().unwrap().remove(0);
    assert!(loaded.is_sealed(), "contexte différent : reste scellée");
}

#[test]
fn the_vault_is_kept_in_the_manifest_and_never_replaced_by_another() {
    let dir = TempDir::new();
    let store = FsVaultStore::open(dir.path()).unwrap();
    assert_eq!(store.load_vault(&BaseId::main()).unwrap(), None);
    let keys = FakeVaultKeys::default();
    let password = Password::new("mot de passe".into()).unwrap();
    let (first, _) = keys.create(&password).unwrap();
    store.save_vault(&BaseId::main(), &first).unwrap();
    assert_eq!(
        store.load_vault(&BaseId::main()).unwrap(),
        Some(first.clone())
    );
    let manifest = fs::read_to_string(dir.path().join("manifest.json")).unwrap();
    assert!(manifest.contains("\"algorithm\": \"argon2id\""));
    let (second, _) = keys.create(&password).unwrap();
    assert_eq!(
        store.save_vault(&BaseId::main(), &second),
        Err(RepositoryError::Vault(VaultError::AlreadyExists))
    );
}
