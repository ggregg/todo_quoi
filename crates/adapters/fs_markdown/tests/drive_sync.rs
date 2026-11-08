//! Tests du scan du dossier (architecture §5.2–5.3), dans de vrais dossiers temporaires : un
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
            "todoquoi-scan-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn tasks(&self) -> PathBuf {
        self.0.join("tasks")
    }

    fn conflicts(&self) -> Vec<String> {
        fs::read_dir(self.0.join(".conflicts"))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Une version de la tâche n° 1, renommée `title` à `millis` par `device`.
fn version(title: &str, millis: i64, device: &str) -> Task {
    let mut task = TaskBuilder::new(1, "Départ").build();
    let stamp = ChangeStamp {
        at: Timestamp::from_millis(millis),
        by: DeviceId::parse(device).unwrap(),
    };
    task.rename(Title::parse(title).unwrap(), &stamp).unwrap();
    task
}

/// Écrit un fichier « comme un autre appareil » (sans passer par le dépôt).
fn write_elsewhere(path: &Path, task: &Task) {
    // Une taille différente garantit une empreinte différente, même dans la même seconde.
    fs::write(path, render_task_file(task, &toml::Table::new()).unwrap()).unwrap();
}

#[test]
fn only_changes_made_elsewhere_are_reported() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let scan = repository.drive_sync();
    repository.save(&version("A", 1_000, "mac")).unwrap();

    assert!(
        scan.scan().unwrap().changed.is_empty(),
        "premier passage : point de départ"
    );
    repository
        .save(&version("B (écrit ici)", 2_000, "mac"))
        .unwrap();
    assert!(
        scan.scan().unwrap().changed.is_empty(),
        "nos écritures ne sont pas des changements"
    );

    write_elsewhere(
        &dir.tasks().join(format!("{}.md", task_id(1))),
        &version("C, venue du PC", 3_000, "pc"),
    );
    assert_eq!(scan.scan().unwrap().changed, [task_id(1)]);
    assert!(scan.scan().unwrap().changed.is_empty());

    fs::remove_file(dir.tasks().join(format!("{}.md", task_id(1)))).unwrap();
    assert_eq!(
        scan.scan().unwrap().changed,
        [task_id(1)],
        "effacée ailleurs"
    );
}

#[test]
fn overwriting_a_version_changed_elsewhere_sets_it_aside() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let scan = repository.drive_sync();
    repository.save(&version("A", 1_000, "mac")).unwrap();
    scan.scan().unwrap();

    write_elsewhere(
        &dir.tasks().join(format!("{}.md", task_id(1))),
        &version("Version du PC", 2_000, "pc"),
    );
    repository
        .save(&version("Version du Mac", 3_000, "mac"))
        .unwrap();

    let set_aside = dir.conflicts();
    assert_eq!(set_aside.len(), 1);
    assert!(set_aside[0].ends_with(".pc.md"));
    let copies = scan.conflict_copies().unwrap();
    assert_eq!(copies[0].title.as_deref(), Some("Version du PC"));
    assert_eq!(
        repository.load_all().unwrap()[0].title().as_str(),
        "Version du Mac"
    );
}

#[test]
fn a_drive_conflict_copy_is_settled_by_the_latest_write() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let scan = repository.drive_sync();
    repository.save(&version("Ancienne", 1_000, "mac")).unwrap();
    scan.scan().unwrap();

    let copy = dir
        .tasks()
        .join(format!("{} (conflicted copy 2026-11-07).md", task_id(1)));
    write_elsewhere(&copy, &version("Plus récente", 2_000, "pc"));
    let report = scan.scan().unwrap();

    assert_eq!(
        (report.resolved_conflicts, report.changed.clone()),
        (1, vec![task_id(1)])
    );
    assert!(!copy.exists(), "la copie a disparu");
    assert_eq!(
        repository.load_all().unwrap()[0].title().as_str(),
        "Plus récente"
    );
    assert_eq!(dir.conflicts().len(), 1, "l'ancienne version est rangée");
}

#[test]
fn an_older_drive_copy_is_set_aside_and_the_task_kept() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let scan = repository.drive_sync();
    repository.save(&version("Gardée", 5_000, "mac")).unwrap();
    write_elsewhere(
        &dir.tasks().join(format!("{} (1).md", task_id(1))),
        &version("Écartée", 1_000, "pc"),
    );

    scan.scan().unwrap();
    assert_eq!(repository.load_all().unwrap()[0].title().as_str(), "Gardée");
    assert!(dir.conflicts()[0].ends_with(".pc.md"));
}

#[test]
fn unreadable_files_are_reported_until_fixed() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(&dir.0).unwrap();
    let scan = repository.drive_sync();
    let broken = dir.tasks().join(format!("{}.md", task_id(2)));
    fs::write(&broken, "pas un fichier de tâche").unwrap();

    assert_eq!(scan.scan().unwrap().unreadable_files, 1);
    assert_eq!(scan.scan().unwrap().unreadable_files, 1, "toujours signalé");
    fs::remove_file(&broken).unwrap();
    assert_eq!(scan.scan().unwrap().unreadable_files, 0);
}
