//! Composites multi-bases (ADR-0009), avec les faux en mémoire de `tq_testing`.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use tq_adapter_multi_base::{MultiBaseDriveSync, MultiBaseMediaStore, MultiBaseTaskRepository};
use tq_application::ports::{DriveSync, MediaStore, RepositoryError, ScanReport, TaskRepository};
use tq_domain::{BaseId, ChangeStamp, DeviceId, NoteBody, Timestamp};
use tq_testing::contract::check_task_repository_contract;
use tq_testing::fakes::{InMemoryMediaStore, InMemoryTaskRepository, ScriptedDriveSync};
use tq_testing::{TaskBuilder, task_id};

fn base(name: &str) -> BaseId {
    BaseId::parse(name).unwrap()
}

struct TwoBases {
    perso: Arc<InMemoryTaskRepository>,
    pro: Arc<InMemoryTaskRepository>,
    perso_media: Arc<InMemoryMediaStore>,
    pro_media: Arc<InMemoryMediaStore>,
    composite: MultiBaseTaskRepository,
}

fn two_bases() -> TwoBases {
    let perso = Arc::new(InMemoryTaskRepository::default());
    let pro = Arc::new(InMemoryTaskRepository::default());
    let perso_media = Arc::new(InMemoryMediaStore::default());
    let pro_media = Arc::new(InMemoryMediaStore::default());
    let media = Arc::new(MultiBaseMediaStore::new(vec![
        (base("perso"), perso_media.clone() as Arc<dyn MediaStore>),
        (base("pro"), pro_media.clone() as Arc<dyn MediaStore>),
    ]));
    let composite = MultiBaseTaskRepository::new(
        vec![
            (base("perso"), perso.clone() as Arc<dyn TaskRepository>),
            (base("pro"), pro.clone() as Arc<dyn TaskRepository>),
        ],
        media,
    );
    TwoBases {
        perso,
        pro,
        perso_media,
        pro_media,
        composite,
    }
}

#[test]
fn a_single_base_honours_the_repository_contract() {
    let media = Arc::new(MultiBaseMediaStore::new(vec![(
        BaseId::main(),
        Arc::new(InMemoryMediaStore::default()) as Arc<dyn MediaStore>,
    )]));
    let composite = MultiBaseTaskRepository::new(
        vec![(
            BaseId::main(),
            Arc::new(InMemoryTaskRepository::default()) as Arc<dyn TaskRepository>,
        )],
        media,
    );
    check_task_repository_contract(&composite);
}

#[test]
fn tasks_are_read_with_the_base_of_their_folder_and_saved_back_there() {
    let bases = two_bases();
    bases
        .perso
        .save(&TaskBuilder::new(1, "Courses").build())
        .unwrap();
    bases
        .pro
        .save(&TaskBuilder::new(2, "Client").build())
        .unwrap();
    let loaded = bases.composite.load_all().unwrap();
    let client = loaded.iter().find(|task| task.id() == &task_id(2)).unwrap();
    assert_eq!(client.base(), &base("pro"));
    bases.composite.save(client).unwrap();
    assert!(bases.perso.get(&task_id(2)).is_none());
}

#[test]
fn moving_a_task_to_another_base_moves_its_file_and_copies_its_media() {
    let bases = two_bases();
    let image = bases
        .perso_media
        .store(&base("perso"), b"photo", "jpg")
        .unwrap();
    let mut task = TaskBuilder::new(1, "Avec photo").build();
    let stamp = ChangeStamp {
        at: Timestamp::from_millis(5_000),
        by: DeviceId::parse("mac").unwrap(),
    };
    task.replace_note(
        NoteBody::from_markdown(format!("![photo]({})", image.note_link())),
        &stamp,
    )
    .unwrap();
    bases.perso.save(&task).unwrap();

    let loaded = bases.composite.load_all().unwrap().remove(0);
    let moved = loaded.in_base(base("pro"));
    bases.composite.save(&moved).unwrap();

    assert!(
        bases.perso.get(&task_id(1)).is_none(),
        "effacée de la base d'origine"
    );
    assert!(
        bases.pro.get(&task_id(1)).is_some(),
        "écrite dans la base visée"
    );
    assert_eq!(
        bases.pro_media.read(&image).unwrap(),
        b"photo",
        "le média a suivi"
    );
}

#[test]
fn an_interrupted_move_leaves_a_duplicate_settled_by_the_latest_write() {
    let bases = two_bases();
    let old = TaskBuilder::new(1, "Ancienne").build();
    let mut newer = old.clone();
    let stamp = ChangeStamp {
        at: Timestamp::from_millis(9_000),
        by: DeviceId::parse("pc").unwrap(),
    };
    newer
        .rename(tq_domain::Title::parse("Récente").unwrap(), &stamp)
        .unwrap();
    bases.perso.save(&old).unwrap();
    bases.pro.save(&newer).unwrap();
    let loaded = bases.composite.load_all().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(
        (loaded[0].title().as_str(), loaded[0].base()),
        ("Récente", &base("pro"))
    );
}

#[test]
fn scans_are_summed_and_an_unreachable_base_is_named() {
    let perso = Arc::new(ScriptedDriveSync::default());
    let pro = Arc::new(ScriptedDriveSync::default());
    perso.next_scan(Ok(ScanReport {
        changed: vec![task_id(1)],
        pending_downloads: 1,
        ..ScanReport::default()
    }));
    pro.next_scan(Ok(ScanReport {
        changed: vec![task_id(2)],
        pending_downloads: 2,
        ..ScanReport::default()
    }));
    pro.next_scan(Err(RepositoryError::Unavailable("hors ligne".to_string())));
    let sync = MultiBaseDriveSync::new(vec![
        (base("perso"), perso as Arc<dyn DriveSync>),
        (base("pro"), pro as Arc<dyn DriveSync>),
    ]);
    let report = sync.scan().unwrap();
    assert_eq!((report.changed.len(), report.pending_downloads), (2, 3));
    let error = sync.scan().unwrap_err().to_string();
    assert!(
        error.contains("pro"),
        "la base inaccessible est nommée : {error}"
    );
}
