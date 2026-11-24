//! Tests du dossier `archive/` (ADR-0015, R13–R18), dans de vrais dossiers temporaires : un
//! « autre appareil » est simulé en écrivant directement les fichiers.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use tq_adapter_fs_markdown::FsMarkdownTaskRepository;
use tq_adapter_fs_markdown::format::render_task_file;
use tq_application::ports::{DriveSync, TaskRepository};
use tq_domain::{ChangeStamp, DeviceId, Task, Timestamp, Title};
use tq_testing::{TaskBuilder, task_id};

/// Dossier temporaire supprimé à la fin du test.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "todoquoi-archive-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, folder: &str, task: &Task) -> PathBuf {
        self.0.join(folder).join(format!("{}.md", task.id()))
    }

    fn conflicts(&self) -> usize {
        fs::read_dir(self.0.join(".conflicts")).map_or(0, |entries| entries.count())
    }

    fn manifest(&self) -> String {
        fs::read_to_string(self.0.join("manifest.json")).unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stamp(millis: i64) -> ChangeStamp {
    ChangeStamp {
        at: Timestamp::from_millis(millis),
        by: DeviceId::parse("mac").unwrap(),
    }
}

fn archived(mut task: Task, millis: i64) -> Task {
    task.archive(&stamp(millis)).unwrap();
    task
}

/// Écrit un fichier « comme un autre appareil » (sans passer par le dépôt).
fn write_elsewhere(path: &Path, task: &Task) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, render_task_file(task, &toml::Table::new()).unwrap()).unwrap();
}

#[test]
fn archiving_moves_the_file_and_restoring_brings_it_back() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let task = TaskBuilder::new(1, "Compte rendu").build();
    repository.save(&task).unwrap();

    let archive = archived(task.clone(), 2_000);
    repository.save(&archive).unwrap();
    assert!(!dir.file("tasks", &task).exists());
    assert!(dir.file("archive", &task).exists());
    assert!(
        fs::read_to_string(dir.file("archive", &task))
            .unwrap()
            .contains("archived_at")
    );
    assert!(repository.load_all().unwrap().is_empty());
    assert_eq!(
        repository.load_archived().unwrap(),
        std::slice::from_ref(&archive)
    );

    let mut restored = archive;
    restored.restore_from_archive(&stamp(3_000)).unwrap();
    repository.save(&restored).unwrap();
    assert!(dir.file("tasks", &task).exists());
    assert!(!dir.file("archive", &task).exists());
    assert_eq!(repository.load_all().unwrap(), [restored]);
}

#[test]
fn the_first_archive_raises_the_schema_version_of_the_base() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    repository
        .save(&TaskBuilder::new(1, "Active").build())
        .unwrap();
    assert!(dir.manifest().contains("\"schema_version\": 1"));
    repository
        .save(&archived(TaskBuilder::new(2, "Archivée").build(), 2_000))
        .unwrap();
    assert!(dir.manifest().contains("\"schema_version\": 3"));
}

#[test]
fn unknown_header_fields_follow_the_task_into_the_archive() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let task = TaskBuilder::new(1, "Avec champ inconnu").build();
    let mut unknown = toml::Table::new();
    unknown.insert("couleur".into(), toml::Value::String("rouge".into()));
    fs::create_dir_all(dir.0.join("tasks")).unwrap();
    let text = render_task_file(&task, &unknown).unwrap();
    fs::write(dir.file("tasks", &task), text).unwrap();

    repository.save(&archived(task.clone(), 2_000)).unwrap();
    let archived_text = fs::read_to_string(dir.file("archive", &task)).unwrap();
    assert!(archived_text.contains("couleur = \"rouge\""));
}

#[test]
fn the_folder_decides_whether_a_task_is_archived() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    // Rangée à la main dans `archive/` sans date, ressortie à la main avec sa date.
    let filed_by_hand = TaskBuilder::new(1, "Rangée à la main").build();
    write_elsewhere(&dir.file("archive", &filed_by_hand), &filed_by_hand);
    let taken_out = archived(TaskBuilder::new(2, "Ressortie à la main").build(), 2_000);
    write_elsewhere(&dir.file("tasks", &taken_out), &taken_out);

    let archives = repository.load_archived().unwrap();
    assert_eq!(archives.len(), 1);
    assert_eq!(archives[0].archived_at(), Some(filed_by_hand.updated_at()));
    let active = repository.load_all().unwrap();
    assert_eq!(active.len(), 1);
    assert!(active[0].is_active());
}

#[test]
fn r18_the_copy_left_behind_by_archiving_elsewhere_is_simply_removed() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let task = TaskBuilder::new(1, "Archivée ailleurs").build();
    repository.save(&task).unwrap();
    // L'autre appareil a archivé : `archive/` est arrivé, l'effacement de `tasks/` pas encore.
    write_elsewhere(&dir.file("archive", &task), &archived(task.clone(), 2_000));

    assert!(repository.load_all().unwrap().is_empty());
    assert!(!dir.file("tasks", &task).exists());
    assert_eq!(dir.conflicts(), 0, "rien à garder : même contenu");
    assert_eq!(repository.load_archived().unwrap().len(), 1);
}

#[test]
fn r18_a_later_edit_wins_over_the_archive_which_is_set_aside() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let task = TaskBuilder::new(1, "Départ").build();
    write_elsewhere(&dir.file("archive", &task), &archived(task.clone(), 2_000));
    let mut edited = task.clone();
    edited
        .rename(Title::parse("Modifiée après").unwrap(), &stamp(3_000))
        .unwrap();
    write_elsewhere(&dir.file("tasks", &task), &edited);

    assert_eq!(repository.load_all().unwrap(), [edited]);
    assert!(!dir.file("archive", &task).exists());
    assert_eq!(dir.conflicts(), 1, "l'archive écartée est gardée");
}

#[test]
fn r18_a_later_archive_wins_over_an_earlier_edit_which_is_set_aside() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let scan = repository.drive_sync();
    let task = TaskBuilder::new(1, "Départ").build();
    let mut edited = task.clone();
    edited
        .rename(Title::parse("Modifiée avant").unwrap(), &stamp(2_000))
        .unwrap();
    write_elsewhere(&dir.file("tasks", &task), &edited);
    scan.scan().unwrap();
    write_elsewhere(&dir.file("archive", &task), &archived(task.clone(), 3_000));

    assert!(repository.load_all().unwrap().is_empty());
    assert_eq!(dir.conflicts(), 1, "la modification écartée est gardée");
    assert!(
        scan.scan().unwrap().changed.is_empty(),
        "notre propre effacement n'est pas un changement venu d'ailleurs"
    );
    assert_eq!(repository.load_archived().unwrap()[0].id(), &task_id(1));
}
