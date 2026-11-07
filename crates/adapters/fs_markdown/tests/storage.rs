//! Tests de l'adaptateur fichiers, dans de vrais dossiers temporaires.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use tq_adapter_fs_markdown::FsMarkdownTaskRepository;
use tq_adapter_fs_markdown::format::{parse_task_file, render_task_file};
use tq_application::ports::{RepositoryError, TaskRepository};
use tq_testing::contract::check_task_repository_contract;
use tq_testing::{TaskBuilder, task_id};

/// Dossier temporaire supprimé automatiquement à la fin du test (sans crate dédiée, ADR-0006).
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "todoquoi-test-{}-{}",
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

// [rust] `Drop` : code exécuté automatiquement quand la valeur disparaît (fin du test, même en
//        cas d'échec) — l'équivalent d'un `finally` attaché à l'objet.
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SAMPLE: &str = "+++
schema_version = 1
id = \"0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e\"
parent = \"0195f3c0-1a2b-7c3d-8e4f-5a6b7c8d9e0f\"
order = \"a0V\"
title = \"Préparer la release 1.2\"
done = false
tags = [\"release\", \"client-x\"]
created_at = 2026-11-05T16:02:11.120Z
updated_at = 2026-11-05T17:25:12.345Z
updated_by = \"mac-greg\"
encrypted = false
couleur = \"rouge\"

[view]
columns = [\"title\", \"progress\"]
sort = [{ field = \"manual\" }]
+++
Checklist de la release :

- [x] Changelog
- [ ] Tag Git
";

// ----- Contrat ---------------------------------------------------------------------------------

#[test]
fn file_repository_honours_the_contract() {
    let dir = TempDir::new();
    check_task_repository_contract(&FsMarkdownTaskRepository::open(dir.path()).unwrap());
}

// ----- Format ----------------------------------------------------------------------------------

#[test]
fn the_documented_format_is_read_and_rewritten_with_unknown_fields() {
    let parsed = parse_task_file(SAMPLE).unwrap();
    assert_eq!(parsed.task.title().as_str(), "Préparer la release 1.2");
    assert_eq!(parsed.task.tags().len(), 2);
    assert_eq!(
        parsed.task.created_at().to_rfc3339(),
        "2026-11-05T16:02:11.120Z"
    );
    assert!(parsed.task.note().as_markdown().starts_with("Checklist"));
    assert!(parsed.unknown.contains_key("couleur"));
    // `[view]` est désormais compris (I4) : il quitte les champs inconnus pour la tâche.
    assert!(!parsed.unknown.contains_key("view"));
    let view = parsed.task.view().expect("vue relue");
    assert_eq!(view.columns, [tq_domain::view::Column::Progress]);

    // Réécriture : on compare des VALEURS (la crate toml reformate le texte, cf. POC I0-1).
    let rewritten = render_task_file(&parsed.task, &parsed.unknown).unwrap();
    assert_eq!(parse_task_file(&rewritten).unwrap(), parsed);
}

#[test]
fn windows_line_endings_are_accepted() {
    let parsed = parse_task_file(&SAMPLE.replace('\n', "\r\n")).unwrap();
    assert_eq!(parsed.task.title().as_str(), "Préparer la release 1.2");
}

#[test]
fn invalid_files_are_explained() {
    let no_header = parse_task_file("# Pas d'en-tête\n").unwrap_err();
    assert_eq!(no_header.to_string(), "en-tête +++ absent ou non refermé");
    let empty_title =
        parse_task_file(&SAMPLE.replace("Préparer la release 1.2", "  ")).unwrap_err();
    assert!(empty_title.to_string().contains("title"), "{empty_title}");
    let newer =
        parse_task_file(&SAMPLE.replace("schema_version = 1", "schema_version = 9")).unwrap_err();
    assert!(newer.to_string().contains("version 9"), "{newer}");
}

// ----- Stockage --------------------------------------------------------------------------------

#[test]
fn opening_creates_the_layout_and_the_manifest() {
    let dir = TempDir::new();
    FsMarkdownTaskRepository::open(dir.path()).unwrap();
    assert!(dir.path().join("tasks").is_dir());
    let manifest = fs::read_to_string(dir.path().join("manifest.json")).unwrap();
    assert!(manifest.contains("\"schema_version\": 1"));
}

#[test]
fn data_from_a_newer_version_is_refused() {
    let dir = TempDir::new();
    fs::write(
        dir.path().join("manifest.json"),
        "{ \"schema_version\": 2 }",
    )
    .unwrap();
    assert!(matches!(
        FsMarkdownTaskRepository::open(dir.path()),
        Err(RepositoryError::NewerSchema {
            found: 2,
            supported: 1
        })
    ));
}

#[test]
fn a_corrupted_file_is_set_aside_without_blocking_the_others() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(dir.path()).unwrap();
    repository
        .save(&TaskBuilder::new(1, "Valide").build())
        .unwrap();
    fs::write(dir.path().join("tasks/cassé.md"), "pas un fichier de tâche").unwrap();

    let tasks = repository.load_all().unwrap();
    assert_eq!(tasks.len(), 1);
    let rejected = repository.rejected_files();
    assert_eq!(rejected.len(), 1);
    assert!(rejected[0].path.ends_with("cassé.md"));
}

#[test]
fn a_drive_conflict_copy_keeps_the_most_recent_version() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(dir.path()).unwrap();
    let task = TaskBuilder::new(1, "Version ancienne").build();
    repository.save(&task).unwrap();
    // Copie de conflit créée par le client du drive, plus récente.
    let newer = parse_task_file(
        &fs::read_to_string(dir.path().join(format!("tasks/{}.md", task_id(1)))).unwrap(),
    )
    .unwrap()
    .task;
    let text = render_task_file(&newer, &toml::Table::new())
        .unwrap()
        .replace("Version ancienne", "Version récente")
        .replace("1970-01-01T00:00:01Z", "1970-01-01T00:00:09Z");
    fs::write(
        dir.path().join(format!("tasks/{} (1).md", task_id(1))),
        text,
    )
    .unwrap();

    let tasks = repository.load_all().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title().as_str(), "Version récente");
    assert_eq!(repository.rejected_files().len(), 1);
}

#[test]
fn unknown_header_fields_survive_a_save() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(dir.path()).unwrap();
    let path = dir
        .path()
        .join("tasks/0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e.md");
    fs::write(&path, SAMPLE).unwrap();

    let mut task = repository.load_all().unwrap().remove(0);
    task.rename(
        tq_domain::Title::parse("Nouveau titre").unwrap(),
        &tq_domain::ChangeStamp {
            at: tq_domain::Timestamp::from_millis(1_793_980_000_000),
            by: tq_domain::DeviceId::parse("pc-bureau").unwrap(),
        },
    )
    .unwrap();
    repository.save(&task).unwrap();

    let reparsed = parse_task_file(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(reparsed.task.title().as_str(), "Nouveau titre");
    assert_eq!(reparsed.unknown["couleur"].as_str(), Some("rouge"));
    assert_eq!(
        reparsed.task.view(),
        task.view(),
        "la vue survit à la réécriture"
    );
    assert!(reparsed.task.view().is_some());
}

#[test]
fn an_unreadable_view_is_kept_as_is_and_the_list_inherits() {
    let text = SAMPLE.replace("[view]", "[view]\nfilter = { status = \"later\" }");
    let parsed = parse_task_file(&text).unwrap();
    assert_eq!(parsed.task.view(), None);
    assert!(
        parsed.unknown["view"].is_table(),
        "vue illisible conservée telle quelle"
    );
}

#[test]
fn no_temporary_file_is_left_behind() {
    let dir = TempDir::new();
    let repository = FsMarkdownTaskRepository::open(dir.path()).unwrap();
    for number in 1..=20 {
        repository
            .save(&TaskBuilder::new(number, "Tâche").build())
            .unwrap();
    }
    let leftovers: Vec<_> = fs::read_dir(dir.path().join("tasks"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".~"))
        .collect();
    assert!(leftovers.is_empty());
    assert_eq!(repository.load_all().unwrap().len(), 20);
}

// ----- Médias (I3) -----------------------------------------------------------------------------

#[test]
fn media_are_named_by_content_deduplicated_and_read_back() {
    use tq_adapter_fs_markdown::FsMediaStore;
    use tq_application::ports::MediaStore;

    let dir = TempDir::new();
    let store = FsMediaStore::open(dir.path()).unwrap();
    let media = store.store(b"abc", "TXT").unwrap();
    assert_eq!(
        media.as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad.txt"
    );
    assert_eq!(store.store(b"abc", "txt").unwrap(), media);
    assert_eq!(
        fs::read_dir(dir.path().join("media")).unwrap().count(),
        1,
        "un seul fichier, sans temporaire"
    );
    assert_eq!(store.read(&media).unwrap(), b"abc");
    assert!(store.local_path(&media).unwrap().exists());
}

#[test]
fn a_missing_media_is_not_found() {
    use tq_adapter_fs_markdown::FsMediaStore;
    use tq_application::ports::MediaStore;

    let dir = TempDir::new();
    let store = FsMediaStore::open(dir.path()).unwrap();
    let absent = tq_domain::MediaRef::parse(&format!("{}.jpg", "0".repeat(64))).unwrap();
    assert!(matches!(
        store.read(&absent),
        Err(RepositoryError::NotFound(_))
    ));
}

// ----- Vue racine (manifest.json) ---------------------------------------------------------------

#[test]
fn the_root_view_is_saved_in_the_manifest_keeping_other_fields() {
    use tq_adapter_fs_markdown::FsRootViewStore;
    use tq_application::ports::RootViewStore;
    use tq_domain::view::{ListView, StatusChoice};

    let dir = TempDir::new();
    let store = FsRootViewStore::open(dir.path()).unwrap();
    assert_eq!(store.load_root_view().unwrap(), None);

    let manifest = dir.path().join("manifest.json");
    let with_extra =
        fs::read_to_string(&manifest)
            .unwrap()
            .replacen('{', "{\n  \"espace\": \"perso\",", 1);
    fs::write(&manifest, with_extra).unwrap();

    let mut view = ListView::default();
    view.filter.status = StatusChoice::Open;
    store.save_root_view(&view).unwrap();
    assert_eq!(store.load_root_view().unwrap(), Some(view));
    assert!(
        fs::read_to_string(&manifest)
            .unwrap()
            .contains("\"espace\": \"perso\"")
    );
}
