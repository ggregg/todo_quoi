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
