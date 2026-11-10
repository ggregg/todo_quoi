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

// ----- Médias des notes chiffrées -------------------------------------------------------------

fn open_vault(app: &TestApp, base: &BaseId) {
    app.create_vault()
        .execute(base, &password("mot de passe"), Remember::NotOnThisDevice)
        .unwrap();
}

fn note_of(app: &TestApp, number: u64) -> tq_domain::NoteBody {
    app.get_task()
        .execute(&task_id(number))
        .unwrap()
        .note()
        .clone()
}

fn media_of(app: &TestApp, number: u64) -> Vec<tq_domain::MediaRef> {
    tq_domain::media_refs_in(&note_of(app, number))
}

/// Tâche 1 dont la note cite une image stockée en clair.
fn task_with_image(app: &TestApp) -> tq_domain::MediaRef {
    let image = app
        .store_media()
        .execute(&task_id(1), b"photo du chantier", "jpg")
        .unwrap();
    app.edit_note()
        .execute(
            &task_id(1),
            tq_domain::NoteBody::from_markdown(format!("![photo]({})", image.note_link())),
        )
        .unwrap();
    image
}

#[test]
fn encrypting_a_task_seals_its_media_and_erases_the_plain_copy() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Chantier").build()]);
    let plain = task_with_image(&app);
    open_vault(&app, &main_base());

    app.encrypt_task()
        .execute(&task_id(1), EncryptScope::TaskOnly)
        .unwrap();

    let media = media_of(&app, 1);
    assert_eq!(media.len(), 1);
    let sealed = media[0].clone();
    assert!(sealed.is_sealed());
    assert_eq!(
        app.media.names(),
        std::slice::from_ref(&sealed),
        "plus de copie en clair"
    );
    let (_, bytes) = app.read_media().execute(sealed.as_str()).unwrap();
    assert_eq!(bytes, b"photo du chantier");
    assert_ne!(sealed, plain);
}

#[test]
fn a_plain_media_still_cited_elsewhere_is_kept() {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Chantier").build(),
        TaskBuilder::new(2, "Même photo").build(),
    ]);
    let plain = task_with_image(&app);
    app.edit_note()
        .execute(
            &task_id(2),
            tq_domain::NoteBody::from_markdown(format!("![photo]({})", plain.note_link())),
        )
        .unwrap();
    open_vault(&app, &main_base());
    app.encrypt_task()
        .execute(&task_id(1), EncryptScope::TaskOnly)
        .unwrap();
    assert!(app.media.names().contains(&plain), "la tâche 2 la cite");
}

#[test]
fn media_added_to_an_encrypted_note_are_sealed_and_unreadable_once_locked() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Secret").build()]);
    open_vault(&app, &main_base());
    app.encrypt_task()
        .execute(&task_id(1), EncryptScope::TaskOnly)
        .unwrap();
    let media = app
        .store_media()
        .execute(&task_id(1), b"audio", "webm")
        .unwrap();
    assert!(media.is_sealed());
    assert_eq!(media.extension(), "webm");

    app.lock_vault().execute(&main_base());
    assert_eq!(
        app.read_media().execute(media.as_str()),
        Err(AppError::Vault(VaultError::Locked))
    );
    assert_eq!(
        app.read_media().local_path(media.as_str()),
        Err(AppError::Vault(VaultError::EncryptedMedia))
    );
}

#[test]
fn decrypting_puts_the_media_back_in_clear() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Chantier").build()]);
    let plain = task_with_image(&app);
    open_vault(&app, &main_base());
    let encrypt = app.encrypt_task();
    encrypt
        .execute(&task_id(1), EncryptScope::TaskOnly)
        .unwrap();

    app.decrypt_task().execute(&task_id(1)).unwrap();

    assert_eq!(media_of(&app, 1), std::slice::from_ref(&plain));
    assert_eq!(app.media.names(), [plain]);
}

// ----- Déplacer une tâche chiffrée vers une autre base ----------------------------------------

fn two_bases() -> TestApp {
    let app = TestApp::with_tasks(vec![
        TaskBuilder::new(1, "Chantier").in_base("perso").build(),
        TaskBuilder::new(2, "Pro").in_base("pro").build(),
    ]);
    app.bases.set(vec![base("perso"), base("pro")]);
    app
}

fn base(name: &str) -> BaseId {
    BaseId::parse(name).unwrap()
}

fn to_pro() -> tq_application::use_cases::move_task::Movement {
    tq_application::use_cases::move_task::Movement::ToBase { base: base("pro") }
}

#[test]
fn an_encrypted_task_cannot_go_to_a_base_whose_vault_is_closed() {
    let app = two_bases();
    open_vault(&app, &base("perso"));
    app.encrypt_task()
        .execute(&task_id(1), EncryptScope::TaskOnly)
        .unwrap();
    assert_eq!(
        app.move_task().execute(&task_id(1), to_pro()),
        Err(AppError::Vault(VaultError::TargetLocked {
            base: base("pro")
        }))
    );
}

#[test]
fn an_encrypted_task_moving_to_another_base_has_its_media_resealed() {
    let app = two_bases();
    let _ = task_with_image(&app);
    open_vault(&app, &base("perso"));
    app.encrypt_task()
        .execute(&task_id(1), EncryptScope::TaskOnly)
        .unwrap();
    let before = media_of(&app, 1);
    open_vault(&app, &base("pro"));

    app.move_task().execute(&task_id(1), to_pro()).unwrap();

    let after = media_of(&app, 1);
    assert_ne!(after, before, "autre coffre, autre nom");
    assert!(after[0].is_sealed());
    app.lock_vault().execute(&base("perso"));
    let (_, bytes) = app.read_media().execute(after[0].as_str()).unwrap();
    assert_eq!(
        bytes, b"photo du chantier",
        "lisible avec la seule clé de « pro »"
    );
}

// ----- Mot de passe, verrouillage automatique -------------------------------------------------

#[test]
fn changing_the_password_needs_the_current_one_and_keeps_the_content_key() {
    let app = TestApp::with_tasks(vec![TaskBuilder::new(1, "Secret").build()]);
    open_vault(&app, &main_base());
    app.encrypt_task()
        .execute(&task_id(1), EncryptScope::TaskOnly)
        .unwrap();
    let change = app.change_vault_password();
    assert_eq!(
        change.execute(
            &main_base(),
            &password("pas le bon"),
            &password("nouveau mdp")
        ),
        Err(AppError::Vault(VaultError::WrongPassword))
    );
    change
        .execute(
            &main_base(),
            &password("mot de passe"),
            &password("nouveau mdp"),
        )
        .unwrap();

    app.lock_vault().execute(&main_base());
    let unlock = app.unlock_vault();
    let old = unlock.execute(
        &main_base(),
        &password("mot de passe"),
        Remember::NotOnThisDevice,
    );
    assert_eq!(old, Err(AppError::Vault(VaultError::WrongPassword)));
    unlock
        .execute(
            &main_base(),
            &password("nouveau mdp"),
            Remember::NotOnThisDevice,
        )
        .unwrap();
    let task = app.get_task().execute(&task_id(1)).unwrap();
    assert_eq!(task.confidentiality(), &Confidentiality::Encrypted, "relue");
}

#[test]
fn vaults_lock_after_the_delay_without_activity_except_remembered_ones() {
    let app = TwoVaults::new();
    let auto_lock = app
        .app
        .auto_lock_vaults(tq_application::vault::AutoLockDelay::AfterMinutes(15));

    app.app.clock.advance_minutes(10);
    auto_lock.record_activity();
    app.app.clock.advance_minutes(10);
    assert!(auto_lock.execute().is_empty(), "activité il y a 10 min");

    app.app.clock.advance_minutes(6);
    assert_eq!(auto_lock.execute(), [base("perso")]);
    assert!(
        app.app.vaults.is_open(&base("pro")),
        "gardé dans le trousseau"
    );
}

/// « perso » ouvert sans mémoire, « pro » gardé dans le trousseau.
struct TwoVaults {
    app: TestApp,
}

impl TwoVaults {
    fn new() -> Self {
        let app = two_bases();
        open_vault(&app, &base("perso"));
        app.create_vault()
            .execute(
                &base("pro"),
                &password("mot de passe"),
                Remember::OnThisDevice,
            )
            .unwrap();
        Self { app }
    }
}
