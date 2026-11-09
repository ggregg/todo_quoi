//! Joindre (copier, déplacer) et lier un fichier choisi par l'utilisateur.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use std::path::Path;
use std::sync::Arc;
use tq_application::AppError;
use tq_application::ports::{LocalFiles, MediaStore, RepositoryError};
use tq_application::use_cases::attach_file::{AttachFile, AttachMode};
use tq_application::use_cases::check_linked_file::CheckLinkedFile;
use tq_application::use_cases::link_file::LinkFile;
use tq_application::use_cases::store_media::StoreMedia;
use tq_domain::{DeviceId, LinkAvailability, LocalFileKind};
use tq_testing::fakes::{InMemoryLocalFiles, InMemoryMediaStore, InMemoryTaskRepository};
use tq_testing::{TaskBuilder, task_id};

struct Setup {
    files: Arc<InMemoryLocalFiles>,
    media: Arc<InMemoryMediaStore>,
    attach: AttachFile,
}

fn setup() -> Setup {
    let files = Arc::new(InMemoryLocalFiles::default());
    files.add("/docs/Devis.PDF", LocalFileKind::Document, b"devis");
    let media = Arc::new(InMemoryMediaStore::default());
    let tasks = Arc::new(InMemoryTaskRepository::with_tasks([TaskBuilder::new(
        1, "Client",
    )
    .build()]));
    let store = Arc::new(StoreMedia::new(media.clone(), tasks));
    let attach = AttachFile::new(store, files.clone());
    Setup {
        files,
        media,
        attach,
    }
}

fn device(name: &str) -> DeviceId {
    DeviceId::parse(name).unwrap()
}

#[test]
fn copying_keeps_the_original_and_stores_the_file_in_the_base() {
    let Setup {
        files,
        media,
        attach,
    } = setup();
    let attached = attach
        .execute(&task_id(1), Path::new("/docs/Devis.PDF"), AttachMode::Copy)
        .unwrap();
    assert_eq!(attached.name, "Devis.PDF");
    assert_eq!(attached.size, 5);
    assert_eq!(attached.media.extension(), "pdf");
    assert_eq!(media.read(&attached.media).unwrap(), b"devis");
    assert!(files.exists("/docs/Devis.PDF"));
}

#[test]
fn moving_removes_the_original_once_stored() {
    let Setup { files, attach, .. } = setup();
    attach
        .execute(&task_id(1), Path::new("/docs/Devis.PDF"), AttachMode::Move)
        .unwrap();
    assert!(!files.exists("/docs/Devis.PDF"));
}

#[test]
fn a_missing_file_or_a_folder_cannot_be_attached() {
    let Setup { files, attach, .. } = setup();
    files.add("/docs/Projet", LocalFileKind::Directory, b"");
    for path in ["/docs/absent.pdf", "/docs/Projet"] {
        let result = attach.execute(&task_id(1), Path::new(path), AttachMode::Copy);
        assert!(
            matches!(
                result,
                Err(AppError::Repository(RepositoryError::NotFound(_)))
            ),
            "{path}"
        );
    }
}

#[test]
fn a_link_remembers_this_device_and_is_inactive_elsewhere() {
    let files = Arc::new(InMemoryLocalFiles::default());
    files.add("/docs/plan.dwg", LocalFileKind::Document, b"plan");
    let here = device("mac-greg");
    let link = LinkFile::new(files.clone(), here.clone())
        .execute(Path::new("/docs/plan.dwg"))
        .unwrap();
    assert_eq!(link.name, "plan.dwg");
    assert_eq!(link.device, here);

    let check_here = CheckLinkedFile::new(files.clone(), here.clone());
    assert_eq!(
        check_here.execute(Path::new(&link.path), &link.device),
        LinkAvailability::Available
    );
    let check_elsewhere = CheckLinkedFile::new(files.clone(), device("pc-bureau"));
    assert_eq!(
        check_elsewhere.execute(Path::new(&link.path), &link.device),
        LinkAvailability::OnOtherDevice(here.clone())
    );
    files.remove(Path::new("/docs/plan.dwg")).unwrap();
    assert_eq!(
        check_here.execute(Path::new(&link.path), &link.device),
        LinkAvailability::Missing
    );
}
