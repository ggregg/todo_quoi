//! Cas d'usage du coffre (spec §9, ADR-0010), avec les faux du kit de test.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use tq_application::AppError;
use tq_application::use_cases::encrypt_task::EncryptScope;
use tq_application::use_cases::list_vaults::BaseVault;
use tq_application::vault::{Password, Remember, VaultError, VaultState};
use tq_domain::{BaseId, Confidentiality, DomainError};
use tq_testing::{TaskBuilder, TestApp, task_id};

fn password(text: &str) -> Password {
    Password::new(text.to_string()).unwrap()
}

fn main_base() -> BaseId {
    BaseId::main()
}

fn states(app: &TestApp) -> Vec<VaultState> {
    app.list_vaults()
        .execute()
        .unwrap()
        .into_iter()
        .map(|BaseVault { state, .. }| state)
        .collect()
}

#[test]
fn a_base_has_no_vault_until_one_is_created_then_it_is_open() {
    let app = TestApp::new();
    assert_eq!(states(&app), [VaultState::None]);
    app.create_vault()
        .execute(
            &main_base(),
            &password("mot de passe"),
            Remember::NotOnThisDevice,
        )
        .unwrap();
    assert_eq!(states(&app), [VaultState::Unlocked]);
}

#[test]
fn a_vault_is_never_replaced() {
    let app = TestApp::new();
    let create = app.create_vault();
    create
        .execute(
            &main_base(),
            &password("premier mdp"),
            Remember::NotOnThisDevice,
        )
        .unwrap();
    let again = create.execute(
        &main_base(),
        &password("second mdp"),
        Remember::NotOnThisDevice,
    );
    assert_eq!(again, Err(AppError::Vault(VaultError::AlreadyExists)));
}

#[test]
fn lock_then_unlock_with_the_right_password_only() {
    let app = TestApp::new();
    app.create_vault()
        .execute(
            &main_base(),
            &password("mot de passe"),
            Remember::NotOnThisDevice,
        )
        .unwrap();
    app.lock_vault().execute(&main_base());
    assert_eq!(states(&app), [VaultState::Locked]);
    let wrong = app.unlock_vault().execute(
        &main_base(),
        &password("pas le bon"),
        Remember::NotOnThisDevice,
    );
    assert_eq!(wrong, Err(AppError::Vault(VaultError::WrongPassword)));
    app.unlock_vault()
        .execute(
            &main_base(),
            &password("mot de passe"),
            Remember::NotOnThisDevice,
        )
        .unwrap();
    assert_eq!(states(&app), [VaultState::Unlocked]);
}

#[test]
fn unlocking_a_base_without_vault_says_so() {
    let app = TestApp::new();
    let result = app.unlock_vault().execute(
        &main_base(),
        &password("mot de passe"),
        Remember::NotOnThisDevice,
    );
    assert_eq!(result, Err(AppError::Vault(VaultError::NoVault)));
}

#[test]
fn a_remembered_vault_reopens_at_next_launch_and_can_be_forgotten() {
    let app = TestApp::new();
    app.create_vault()
        .execute(
            &main_base(),
            &password("mot de passe"),
            Remember::OnThisDevice,
        )
        .unwrap();
    app.lock_vault().execute(&main_base()); // ≈ quitter l'application
    assert_eq!(app.restore_remembered_vaults().execute(), [main_base()]);
    assert_eq!(states(&app), [VaultState::Unlocked]);

    // Déverrouiller sans « se souvenir » oublie la clé gardée.
    app.lock_vault().execute(&main_base());
    app.unlock_vault()
        .execute(
            &main_base(),
            &password("mot de passe"),
            Remember::NotOnThisDevice,
        )
        .unwrap();
    app.lock_vault().execute(&main_base());
    assert!(app.restore_remembered_vaults().execute().is_empty());
    assert_eq!(states(&app), [VaultState::Locked]);
}

#[test]
fn encrypting_needs_the_base_vault_open() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Secret").build()]);
    let result = app
        .encrypt_task()
        .execute(&task_id(1), EncryptScope::TaskOnly);
    assert_eq!(result, Err(AppError::Vault(VaultError::Locked)));
}

#[test]
fn encrypting_with_subtasks_encrypts_the_plain_active_ones() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Projet").build(),
        TaskBuilder::new(2, "Étape").parent(1).build(),
        TaskBuilder::new(3, "Déjà chiffrée")
            .parent(1)
            .encrypted()
            .build(),
        TaskBuilder::new(4, "Jetée")
            .parent(1)
            .deleted_at(500)
            .build(),
    ]);
    app.create_vault()
        .execute(
            &main_base(),
            &password("mot de passe"),
            Remember::NotOnThisDevice,
        )
        .unwrap();
    let encrypted = app
        .encrypt_task()
        .execute(&task_id(1), EncryptScope::WithSubtasks)
        .unwrap();
    assert_eq!(encrypted, [task_id(1), task_id(2)]);
    let confidentiality = |n| {
        app.repository
            .get(&task_id(n))
            .unwrap()
            .confidentiality()
            .clone()
    };
    assert_eq!(confidentiality(1), Confidentiality::Encrypted);
    assert_eq!(confidentiality(2), Confidentiality::Encrypted);
    assert_eq!(confidentiality(4), Confidentiality::Plain);
}

#[test]
fn decrypting_puts_only_the_task_back_in_clear() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Projet").encrypted().build(),
        TaskBuilder::new(2, "Étape").parent(1).encrypted().build(),
    ]);
    app.decrypt_task().execute(&task_id(1)).unwrap();
    let confidentiality = |n| {
        app.repository
            .get(&task_id(n))
            .unwrap()
            .confidentiality()
            .clone()
    };
    assert_eq!(confidentiality(1), Confidentiality::Plain);
    assert_eq!(confidentiality(2), Confidentiality::Encrypted);
}

#[test]
fn a_sealed_task_can_be_renamed_and_found_by_title_but_its_note_is_locked() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Secret").sealed().build()]);
    app.rename_task()
        .execute(&task_id(1), "Autre titre")
        .unwrap();
    let edit = app.edit_note().execute(
        &task_id(1),
        tq_domain::NoteBody::from_markdown("nouvelle note"),
    );
    assert_eq!(
        edit,
        Err(AppError::Domain(DomainError::TaskSealed(task_id(1))))
    );
    app.list_tasks().execute().unwrap(); // alimente l'index
    assert_eq!(app.index.len(), 1, "cherchable par son titre");
}
