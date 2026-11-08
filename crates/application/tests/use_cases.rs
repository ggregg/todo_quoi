//! Tests des cas d'usage, avec les faux ports du kit de test (aucun disque, aucune horloge réelle).
//! Chaque test se lit comme un scénario de la spécification fonctionnelle.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use tq_application::AppError;
use tq_application::ports::{AppEvent, RepositoryError};
use tq_application::use_cases::close_task::{CloseScope, CloseTaskCommand};
use tq_application::use_cases::create_task::{CreateTaskCommand, Position};
use tq_application::use_cases::move_task::Movement;
use tq_domain::{DomainError, TaskId};
use tq_testing::{TaskBuilder, TestApp, task_id};

/// Titres des tâches actives sous `parent`, dans l'ordre d'affichage.
fn titles_under(app: &TestApp, parent: Option<&TaskId>) -> Vec<String> {
    app.list_tasks()
        .execute()
        .unwrap()
        .tasks
        .into_iter()
        .filter(|task| task.parent() == parent)
        .map(|task| task.title().to_string())
        .collect()
}

fn create(app: &TestApp, title: &str, position: Position) -> TaskId {
    app.create_task()
        .execute(CreateTaskCommand {
            title: title.to_string(),
            position,
        })
        .unwrap()
}

// ----- Création, renommage ------------------------------------------------------------------

#[test]
fn created_tasks_appear_in_order_and_are_announced() {
    let app = TestApp::new();
    let first = create(&app, "Courses", Position::AtEndOf { parent: None });
    create(&app, "Ménage", Position::AtEndOf { parent: None });
    create(
        &app,
        "Pain",
        Position::AtEndOf {
            parent: Some(first.clone()),
        },
    );
    create(
        &app,
        "Lessive",
        Position::After {
            sibling: first.clone(),
        },
    );

    assert_eq!(titles_under(&app, None), ["Courses", "Lessive", "Ménage"]);
    assert_eq!(titles_under(&app, Some(&first)), ["Pain"]);
    assert_eq!(app.events.events().len(), 4);
}

#[test]
fn an_empty_title_is_refused_and_nothing_is_saved() {
    let app = TestApp::new();
    let result = app.create_task().execute(CreateTaskCommand {
        title: "   ".to_string(),
        position: Position::AtEndOf { parent: None },
    });
    assert_eq!(result, Err(AppError::Domain(DomainError::EmptyTitle)));
    assert!(app.repository.snapshot().is_empty());
    assert!(app.events.events().is_empty());
}

#[test]
fn renaming_updates_the_title_and_the_modification_date() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Brouillon").build()]);
    app.rename_task()
        .execute(&task_id(1), "  Version finale ")
        .unwrap();
    let task = app.repository.get(&task_id(1)).unwrap();
    assert_eq!(task.title().as_str(), "Version finale");
    assert!(task.updated_at() > task.created_at());
}

// ----- Clôture / réouverture (R1–R3) ---------------------------------------------------------

fn family() -> TestApp {
    TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Release").build(),
        TaskBuilder::new(2, "Recette").parent(1).order("V").build(),
        TaskBuilder::new(3, "Déploiement")
            .parent(1)
            .order("k")
            .build(),
        TaskBuilder::new(4, "Scénarios").parent(2).build(),
    ])
}

#[test]
fn r1_closing_the_last_open_child_proposes_the_parent_one_level_at_a_time() {
    let app = family();
    let close = |number| {
        app.close_task()
            .execute(CloseTaskCommand {
                task_id: task_id(number),
                scope: CloseScope::TaskOnly,
            })
            .unwrap()
    };
    assert_eq!(close(3).propose_closing_parent, None); // « Recette » encore ouverte
    assert_eq!(close(4).propose_closing_parent, Some(task_id(2)));
    assert_eq!(close(2).propose_closing_parent, Some(task_id(1)));
}

#[test]
fn r2_close_impact_then_close_with_open_descendants() {
    let app = family();
    let impact = app.get_close_impact().execute(&task_id(1)).unwrap();
    assert_eq!(impact.open_descendants, 3);

    let outcome = app
        .close_task()
        .execute(CloseTaskCommand {
            task_id: task_id(1),
            scope: CloseScope::WithOpenDescendants,
        })
        .unwrap();
    assert_eq!(outcome.closed.len(), 4);
    assert!(app.repository.snapshot().iter().all(|task| task.is_done()));
}

#[test]
fn r3_reopening_a_child_proposes_the_closed_ancestors() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Release").done().build(),
        TaskBuilder::new(2, "Recette").parent(1).done().build(),
        TaskBuilder::new(4, "Scénarios").parent(2).done().build(),
    ]);
    let outcome = app.reopen_task().execute(&task_id(4)).unwrap();
    assert_eq!(outcome.propose_reopening, vec![task_id(2), task_id(1)]);
    assert!(!app.repository.get(&task_id(4)).unwrap().is_done());
}

#[test]
fn closing_twice_is_refused() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Fait").done().build()]);
    let result = app.close_task().execute(CloseTaskCommand {
        task_id: task_id(1),
        scope: CloseScope::TaskOnly,
    });
    assert_eq!(
        result,
        Err(AppError::Domain(DomainError::AlreadyClosed(task_id(1))))
    );
}

// ----- Déplacements (R9–R12) -----------------------------------------------------------------

#[test]
fn r9_indent_then_outdent() {
    let app = family();
    app.move_task()
        .execute(&task_id(3), Movement::Indent)
        .unwrap();
    assert_eq!(
        titles_under(&app, Some(&task_id(2))),
        ["Scénarios", "Déploiement"]
    );
    app.move_task()
        .execute(&task_id(3), Movement::Outdent)
        .unwrap();
    assert_eq!(
        titles_under(&app, Some(&task_id(1))),
        ["Recette", "Déploiement"]
    );
}

#[test]
fn r11_moving_into_its_own_subtree_is_refused() {
    let app = family();
    let result = app.move_task().execute(
        &task_id(1),
        Movement::To {
            parent: Some(task_id(4)),
            after: None,
        },
    );
    assert_eq!(
        result,
        Err(AppError::Domain(DomainError::MoveIntoOwnSubtree {
            task: task_id(1)
        }))
    );
}

#[test]
fn r12_moving_the_last_open_child_away_proposes_closing_the_old_parent() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Parent").build(),
        TaskBuilder::new(2, "Faite")
            .parent(1)
            .order("V")
            .done()
            .build(),
        TaskBuilder::new(3, "Ouverte").parent(1).order("k").build(),
    ]);
    let outcome = app
        .move_task()
        .execute(
            &task_id(3),
            Movement::To {
                parent: None,
                after: Some(task_id(1)),
            },
        )
        .unwrap();
    assert_eq!(outcome.propose_closing, Some(task_id(1)));
}

#[test]
fn r12_moving_an_open_task_under_a_closed_parent_proposes_reopening_it() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Terminée").order("V").done().build(),
        TaskBuilder::new(2, "Ouverte").order("k").build(),
    ]);
    let outcome = app
        .move_task()
        .execute(&task_id(2), Movement::Indent)
        .unwrap();
    assert_eq!(outcome.propose_reopening, vec![task_id(1)]);
}

// ----- Corbeille (R5–R7), étiquettes ------------------------------------------------------------

#[test]
fn r5_r6_r7_delete_list_trash_and_restore() {
    let app = family();
    let deleted = app.delete_task().execute(&task_id(2)).unwrap().deleted;
    assert_eq!(deleted.len(), 2); // « Recette » et « Scénarios »
    assert_eq!(titles_under(&app, Some(&task_id(1))), ["Déploiement"]);

    let trash: Vec<String> = app
        .list_trash()
        .execute()
        .unwrap()
        .iter()
        .map(|t| t.title().to_string())
        .collect();
    assert_eq!(trash, ["Recette"]);

    let restored = app.restore_task().execute(&task_id(2)).unwrap();
    assert_eq!(restored.restored.len(), 2);
    assert_eq!(
        titles_under(&app, Some(&task_id(1))),
        ["Recette", "Déploiement"]
    );
}

#[test]
fn tags_are_normalized_and_an_invalid_one_rejects_all() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Tâche").build()]);
    app.set_tags()
        .execute(&task_id(1), &["#Client-X".into(), "release".into()])
        .unwrap();
    let tags: Vec<String> = app
        .repository
        .get(&task_id(1))
        .unwrap()
        .tags()
        .iter()
        .map(|t| t.to_string())
        .collect();
    assert_eq!(tags, ["client-x", "release"]);

    let result = app
        .set_tags()
        .execute(&task_id(1), &["ok".into(), "deux mots".into()]);
    assert!(matches!(
        result,
        Err(AppError::Domain(DomainError::InvalidTag(_)))
    ));
    assert_eq!(app.repository.get(&task_id(1)).unwrap().tags().len(), 2);
}

// ----- Stockage indisponible ------------------------------------------------------------------

#[test]
fn an_unavailable_drive_is_reported_and_nothing_is_announced() {
    let app = family();
    app.repository.make_unavailable();
    let result = app.rename_task().execute(&task_id(1), "Nouveau titre");
    assert!(matches!(
        result,
        Err(AppError::Repository(RepositoryError::Unavailable(_)))
    ));
    assert!(app.events.events().is_empty());
}

#[test]
fn events_list_every_changed_task_once() {
    let app = family();
    app.delete_task().execute(&task_id(2)).unwrap();
    // [rust] On garde la liste dans une variable : `events()` renvoie une copie temporaire,
    //        qui serait détruite avant qu'on lise `ids` dedans.
    let events = app.events.events();
    let [AppEvent::TasksChanged { ids }] = events.as_slice() else {
        panic!("un seul événement attendu");
    };
    assert_eq!(ids, &vec![task_id(2), task_id(4)]);
}

// ----- Note (I2) -----------------------------------------------------------------------------

#[test]
fn the_note_is_read_and_replaced() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Tâche").note("Ancienne note\n").build(),
    ]);
    assert_eq!(
        app.get_task()
            .execute(&task_id(1))
            .unwrap()
            .note()
            .as_markdown(),
        "Ancienne note\n"
    );

    app.edit_note()
        .execute(
            &task_id(1),
            tq_domain::NoteBody::from_markdown("- [ ] Nouvelle\n"),
        )
        .unwrap();
    let task = app.repository.get(&task_id(1)).unwrap();
    assert_eq!(task.note().as_markdown(), "- [ ] Nouvelle\n");
    assert!(task.updated_at() > task.created_at());
}

#[test]
fn saving_an_unchanged_note_writes_nothing() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Tâche").note("Identique\n").build(),
    ]);
    app.edit_note()
        .execute(
            &task_id(1),
            tq_domain::NoteBody::from_markdown("Identique\n"),
        )
        .unwrap();
    assert!(
        app.events.events().is_empty(),
        "aucun événement : rien n'a été écrit"
    );
}

// ----- Médias (I3) ---------------------------------------------------------------------------

#[test]
fn a_media_is_stored_deduplicated_and_read_back() {
    use std::sync::Arc;
    use tq_application::use_cases::{read_media::ReadMedia, store_media::StoreMedia};
    use tq_testing::fakes::InMemoryMediaStore;

    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Avec image").build()]);
    let store = Arc::new(InMemoryMediaStore::default());
    let save = StoreMedia::new(store.clone(), app.repository.clone());
    let first = save
        .execute(&task_id(1), b"contenu de l'image", "JPG")
        .unwrap();
    let again = save
        .execute(&task_id(1), b"contenu de l'image", "jpg")
        .unwrap();
    assert_eq!(first, again, "même contenu = même référence");
    assert_eq!(store.len(), 1);
    assert_eq!(first.extension(), "jpg");

    let (media, bytes) = ReadMedia::new(store).execute(first.as_str()).unwrap();
    assert_eq!(media, first);
    assert_eq!(bytes, b"contenu de l'image");
}

#[test]
fn a_media_over_fifty_megabytes_is_refused() {
    use std::sync::Arc;
    use tq_application::use_cases::store_media::StoreMedia;
    use tq_testing::fakes::InMemoryMediaStore;

    let too_big = vec![0_u8; tq_domain::MAX_MEDIA_BYTES + 1];
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Avec pièce jointe").build()]);
    let result = StoreMedia::new(
        Arc::new(InMemoryMediaStore::default()),
        app.repository.clone(),
    )
    .execute(&task_id(1), &too_big, "pdf");
    assert_eq!(
        result,
        Err(AppError::Domain(DomainError::MediaTooLarge {
            max_megabytes: 50
        }))
    );
}

#[test]
fn a_path_like_media_name_is_refused() {
    use std::sync::Arc;
    use tq_application::use_cases::read_media::ReadMedia;
    use tq_testing::fakes::InMemoryMediaStore;

    let result =
        ReadMedia::new(Arc::new(InMemoryMediaStore::default())).execute("../tasks/secret.md");
    assert!(matches!(
        result,
        Err(AppError::Domain(DomainError::InvalidMediaRef(_)))
    ));
}

// ----- Recherche (spec §5.6) ----------------------------------------------------------------

fn search(app: &TestApp, text: &str) -> Vec<String> {
    app.search_tasks()
        .execute(text, Vec::new())
        .into_iter()
        .map(|hit| hit.title)
        .collect()
}

#[test]
fn the_index_is_filled_by_the_first_listing() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Projet A").build(),
        TaskBuilder::new(2, "Préparer la release")
            .parent(1)
            .note("Rédiger le changelog")
            .build(),
    ]);
    assert!(
        search(&app, "changelog").is_empty(),
        "rien n'est indexé avant le premier chargement"
    );
    app.list_tasks().execute().unwrap();
    let hits = app
        .search_tasks()
        .execute(r#"changelog in:"projet a""#, Vec::new());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, ["Projet A"]);
}

#[test]
fn modifications_are_searchable_immediately() {
    let app = TestApp::new();
    let id = create(&app, "Courses", Position::AtEndOf { parent: None });
    app.rename_task().execute(&id, "Courses du samedi").unwrap();
    assert_eq!(search(&app, "samedi"), ["Courses du samedi"]);
    app.close_task()
        .execute(CloseTaskCommand {
            task_id: id.clone(),
            scope: CloseScope::TaskOnly,
        })
        .unwrap();
    assert_eq!(search(&app, "samedi is:done"), ["Courses du samedi"]);
    assert!(search(&app, "samedi is:open").is_empty());
}

#[test]
fn trashed_tasks_are_not_found() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Ancienne idée").build()]);
    app.list_tasks().execute().unwrap();
    app.delete_task().execute(&task_id(1)).unwrap();
    assert!(search(&app, "idee").is_empty());
}

#[test]
fn unchanged_tasks_are_not_indexed_twice() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "A").build(),
        TaskBuilder::new(2, "B").build(),
    ]);
    app.list_tasks().execute().unwrap();
    app.list_tasks().execute().unwrap();
    assert_eq!(app.index.upsert_count(), 2);
}

// ----- Vues par liste (spec §5.3) -----------------------------------------------------------

use tq_application::use_cases::list_tasks::{ListLayout, ListTarget};
use tq_domain::view::{ListView, SortDirection, SortField, SortKey, StatusChoice, ViewSource};

fn layout_of(app: &TestApp, list: ListTarget) -> ListLayout {
    let listing = app.list_tasks().execute().unwrap();
    listing
        .lists
        .into_iter()
        .find(|layout| layout.list == list)
        .unwrap()
}

fn by_title_descending() -> ListView {
    ListView {
        sort: vec![SortKey {
            field: SortField::Title,
            direction: SortDirection::Descending,
        }],
        ..ListView::default()
    }
}

#[test]
fn every_list_is_laid_out_with_the_default_view() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "B").build(),
        TaskBuilder::new(2, "A").build(),
        TaskBuilder::new(3, "B.1").parent(1).build(),
    ]);
    let root = layout_of(&app, ListTarget::Base(tq_domain::BaseId::main()));
    assert_eq!(root.view, ListView::default());
    assert_eq!(root.children, [task_id(1), task_id(2)]);
    assert_eq!(
        layout_of(&app, ListTarget::Task(task_id(1))).source,
        ViewSource::Root
    );
}

#[test]
fn a_customized_task_list_is_saved_and_inherited_below() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Projet").build(),
        TaskBuilder::new(2, "Alpha").parent(1).build(),
        TaskBuilder::new(3, "Zulu").parent(1).build(),
        TaskBuilder::new(4, "Lot").parent(3).build(),
    ]);
    app.customize_list_view()
        .execute(&ListTarget::Task(task_id(1)), by_title_descending())
        .unwrap();

    let project = layout_of(&app, ListTarget::Task(task_id(1)));
    assert_eq!(project.children, [task_id(3), task_id(2)]);
    assert_eq!(project.source, ViewSource::Own);
    assert_eq!(
        app.repository.get(&task_id(1)).unwrap().view(),
        Some(&by_title_descending())
    );
    let below = layout_of(&app, ListTarget::Task(task_id(3)));
    assert_eq!(below.source, ViewSource::InheritedFrom(task_id(1)));

    app.inherit_list_view().execute(&task_id(1)).unwrap();
    assert_eq!(
        layout_of(&app, ListTarget::Task(task_id(1))).source,
        ViewSource::Root
    );
}

#[test]
fn the_root_view_is_stored_apart_and_announced() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Fait").done().build(),
        TaskBuilder::new(2, "À faire").build(),
    ]);
    let mut open_only = ListView::default();
    open_only.filter.status = StatusChoice::Open;
    app.customize_list_view()
        .execute(
            &ListTarget::Base(tq_domain::BaseId::main()),
            open_only.clone(),
        )
        .unwrap();
    assert_eq!(
        layout_of(&app, ListTarget::Base(tq_domain::BaseId::main())).children,
        [task_id(2)]
    );
    assert!(
        app.events
            .events()
            .contains(&AppEvent::TasksChanged { ids: Vec::new() })
    );
}

#[test]
fn media_are_listed_for_icons() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Photo")
            .note("![x](../media/a.jpg)")
            .build(),
    ]);
    let listing = app.list_tasks().execute().unwrap();
    assert_eq!(listing.media.get(&task_id(1)).map(Vec::len), Some(1));
}

// ----- Synchronisation par le drive (spec §7, I5) -------------------------------------------

use tq_application::ports::{ConflictCopy, ScanReport};
use tq_application::use_cases::sync_with_drive::SyncStatus;
use tq_domain::Timestamp;

#[test]
fn changes_made_elsewhere_are_announced_and_status_reported() {
    let app = TestApp::new();
    app.drive.next_scan(Ok(ScanReport {
        changed: vec![task_id(1)],
        pending_downloads: 2,
        ..ScanReport::default()
    }));
    app.drive.next_scan(Ok(ScanReport {
        resolved_conflicts: 1,
        ..ScanReport::default()
    }));
    app.drive
        .next_scan(Err(RepositoryError::Unavailable("hors ligne".to_string())));

    assert_eq!(
        app.sync_with_drive().execute().unwrap(),
        SyncStatus::Downloading { pending: 2 }
    );
    assert!(app.events.events().contains(&AppEvent::ChangedElsewhere {
        ids: vec![task_id(1)]
    }));
    assert_eq!(
        app.sync_with_drive().execute().unwrap(),
        SyncStatus::Attention {
            unreadable: 0,
            conflicts: 1
        }
    );
    assert!(app.sync_with_drive().execute().is_err());
    assert_eq!(
        app.sync_with_drive().execute().unwrap(),
        SyncStatus::UpToDate
    );
}

#[test]
fn trash_older_than_thirty_days_and_old_conflict_copies_are_purged() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Vieille").build(),
        TaskBuilder::new(2, "Récente").build(),
    ]);
    app.delete_task().execute(&task_id(1)).unwrap();
    app.drive.add_conflict_copy(ConflictCopy {
        task_id: None,
        title: None,
        file_name: "ancienne.md".to_string(),
        set_aside_at: Timestamp::from_millis(0),
    });
    app.clock.advance_days(31);
    app.delete_task().execute(&task_id(2)).unwrap();

    let outcome = app.purge_expired().execute().unwrap();
    assert_eq!((outcome.tasks, outcome.conflict_copies), (1, 1));
    assert!(
        app.repository.get(&task_id(1)).is_none(),
        "effacée définitivement"
    );
    assert!(
        app.repository.get(&task_id(2)).is_some(),
        "encore dans la corbeille"
    );
    assert_eq!(app.list_trash().execute().unwrap().len(), 1);
}

#[test]
fn keeping_the_local_version_sets_the_other_one_aside() {
    let app = TestApp::new();
    app.keep_local_version().execute(&task_id(4)).unwrap();
    assert_eq!(app.drive.copies()[0].task_id, Some(task_id(4)));
}

// ----- Plusieurs bases (ADR-0009) -----------------------------------------------------------

use tq_application::use_cases::create_task::Position as CreatePosition;
use tq_domain::BaseId;

fn base(name: &str) -> BaseId {
    BaseId::parse(name).unwrap()
}

fn two_bases_app() -> TestApp {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(101, "Courses").in_base("perso").build(),
        TaskBuilder::new(102, "Projet client")
            .in_base("pro")
            .build(),
        TaskBuilder::new(103, "Lot 1")
            .in_base("pro")
            .parent(102)
            .build(),
    ]);
    app.bases.set(vec![base("perso"), base("pro")]);
    app
}

#[test]
fn each_base_has_its_own_top_level_list() {
    let app = two_bases_app();
    let listing = app.list_tasks().execute().unwrap();
    assert_eq!(listing.bases, [base("perso"), base("pro")]);
    assert_eq!(
        layout_of(&app, ListTarget::Base(base("perso"))).children,
        [task_id(101)]
    );
    assert_eq!(
        layout_of(&app, ListTarget::Base(base("pro"))).children,
        [task_id(102)]
    );
}

#[test]
fn a_task_created_at_the_top_of_a_base_lands_in_it() {
    let app = two_bases_app();
    let id = app
        .create_task()
        .execute(CreateTaskCommand {
            title: "Facture".to_string(),
            position: CreatePosition::AtTopOf { base: base("pro") },
        })
        .unwrap();
    assert_eq!(app.repository.get(&id).unwrap().base(), &base("pro"));
    let child = create(
        &app,
        "Sous-tâche",
        Position::AtEndOf {
            parent: Some(task_id(101)),
        },
    );
    assert_eq!(
        app.repository.get(&child).unwrap().base(),
        &base("perso"),
        "comme son parent"
    );
}

#[test]
fn moving_to_another_base_takes_subtasks_along() {
    let app = two_bases_app();
    app.move_task()
        .execute(
            &task_id(102),
            Movement::ToBase {
                base: base("perso"),
            },
        )
        .unwrap();
    assert_eq!(
        app.repository.get(&task_id(102)).unwrap().base(),
        &base("perso")
    );
    assert_eq!(
        app.repository.get(&task_id(103)).unwrap().base(),
        &base("perso")
    );
    assert_eq!(
        layout_of(&app, ListTarget::Base(base("perso"))).children,
        [task_id(101), task_id(102)]
    );
    assert!(
        layout_of(&app, ListTarget::Base(base("pro")))
            .children
            .is_empty()
    );
}

#[test]
fn search_is_limited_to_the_displayed_bases() {
    let app = two_bases_app();
    app.list_tasks().execute().unwrap();
    let in_pro = app.search_tasks().execute("o", vec![base("pro")]);
    assert!(in_pro.iter().all(|hit| hit.base == base("pro")));
    assert_eq!(in_pro.len(), 2);
}
