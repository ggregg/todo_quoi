//! Contrats des ports : ce que **toute** implémentation doit garantir
//! ([`check_task_repository_contract`], [`check_search_index_contract`],
//! [`check_media_store_contract`]).
//!
//! Chaque adaptateur appelle [`check_task_repository_contract`] dans ses propres tests, avec un
//! dépôt **vide**. Le faux en mémoire passe la même suite : on est sûr que les tests des cas
//! d'usage (faits avec le faux) restent valables avec le vrai stockage.

use crate::TaskBuilder;
use tq_application::ports::{MediaStore, RepositoryError, SearchIndex, TaskRepository};
use tq_domain::search::{MediaKind, NoteDigest, SearchDocument, SearchQuery};
use tq_domain::{BaseId, MediaRef, Task};

/// Exécute toutes les vérifications du contrat sur un dépôt vide.
///
/// # Panique
/// Au premier écart, avec un message qui dit quelle garantie n'est pas tenue.
pub fn check_task_repository_contract(repository: &dyn TaskRepository) {
    starts_empty(repository);
    saved_task_is_loaded_identically(repository);
    saving_again_replaces(repository);
    every_field_survives(repository);
    removed_task_is_gone(repository);
}

fn load(repository: &dyn TaskRepository) -> Vec<Task> {
    let mut tasks = repository
        .load_all()
        .expect("contrat : load_all doit réussir");
    tasks.sort_by(|a, b| a.id().cmp(b.id()));
    tasks
}

fn starts_empty(repository: &dyn TaskRepository) {
    assert!(
        load(repository).is_empty(),
        "contrat : un dépôt neuf est vide"
    );
}

fn saved_task_is_loaded_identically(repository: &dyn TaskRepository) {
    let task = TaskBuilder::new(1, "Première tâche").build();
    repository.save(&task).expect("contrat : save doit réussir");
    assert_eq!(
        load(repository),
        vec![task],
        "contrat : une tâche enregistrée est relue à l'identique"
    );
}

fn saving_again_replaces(repository: &dyn TaskRepository) {
    let renamed = TaskBuilder::new(1, "Première tâche, renommée").build();
    repository
        .save(&renamed)
        .expect("contrat : save doit réussir");
    assert_eq!(
        load(repository),
        vec![renamed],
        "contrat : réenregistrer remplace, sans doublon"
    );
}

fn every_field_survives(repository: &dyn TaskRepository) {
    let rich = TaskBuilder::new(2, "Tâche complète : « accents », émojis 🎉")
        .parent(1)
        .order("a0V")
        .done()
        .deleted_at(5_000)
        .tags(&["release", "client-x", "équipe"])
        .note(
            "# Titre\n\n- [x] fait\n- [ ] à faire\n\n:::radio{label=\"Cible\"}\n- (x) Prod\n:::\n",
        )
        .build();
    repository.save(&rich).expect("contrat : save doit réussir");
    let loaded = load(repository);
    assert_eq!(loaded.len(), 2, "contrat : deux tâches distinctes");
    assert_eq!(
        loaded[1], rich,
        "contrat : tous les champs survivent à l'enregistrement"
    );
}

fn removed_task_is_gone(repository: &dyn TaskRepository) {
    let task = TaskBuilder::new(90, "À effacer").build();
    repository.save(&task).expect("contrat : save doit réussir");
    repository
        .remove(task.id())
        .expect("contrat : remove doit réussir");
    assert!(
        load(repository)
            .iter()
            .all(|loaded| loaded.id() != task.id()),
        "contrat : une tâche effacée n'est plus chargée"
    );
    repository
        .remove(task.id())
        .expect("contrat : effacer une tâche absente n'est pas une erreur");
}

/// Exécute toutes les vérifications du contrat de `SearchIndex` sur un index **vide**.
///
/// # Panique
/// Au premier écart, avec un message qui dit quelle garantie n'est pas tenue.
pub fn check_search_index_contract(index: &dyn SearchIndex) {
    upserted_documents_are_found(index);
    upsert_replaces_and_remove_forgets(index);
    retain_only_keeps_the_given_tasks(index);
}

fn digest(text: &str) -> NoteDigest {
    NoteDigest {
        text: text.to_string(),
        media: Vec::new(),
    }
}

fn found(index: &dyn SearchIndex, text: &str) -> usize {
    index.search(&SearchQuery::parse(text), 10).len()
}

fn upserted_documents_are_found(index: &dyn SearchIndex) {
    let task = TaskBuilder::new(1, "Préparer la release").build();
    assert_eq!(
        index.indexed_version(task.id()),
        None,
        "contrat : un index neuf est vide"
    );
    index.upsert(SearchDocument::new(&task, digest("changelog, œufs")));
    assert_eq!(
        index.indexed_version(task.id()),
        Some(task.updated_at()),
        "contrat : la version est retenue"
    );
    assert_eq!(
        found(index, "release changelog"),
        1,
        "contrat : titre et note sont cherchés"
    );
    assert_eq!(
        found(index, "oeufs"),
        1,
        "contrat : sans accents, ligatures développées"
    );
    assert!(
        index.media_of(task.id()).is_empty(),
        "contrat : pas de média dans cette note"
    );
    let with_image = NoteDigest {
        text: String::new(),
        media: vec![MediaKind::Image],
    };
    index.upsert(SearchDocument::new(&task, with_image));
    assert_eq!(
        index.media_of(task.id()),
        [MediaKind::Image],
        "contrat : médias de la fiche"
    );
}

fn upsert_replaces_and_remove_forgets(index: &dyn SearchIndex) {
    let task = TaskBuilder::new(2, "Courses").build();
    index.upsert(SearchDocument::new(&task, digest("lait")));
    index.upsert(SearchDocument::new(&task, digest("pain")));
    assert_eq!(
        found(index, "lait"),
        0,
        "contrat : une fiche refaite remplace l'ancienne"
    );
    index.remove(task.id());
    assert_eq!(
        found(index, "courses"),
        0,
        "contrat : une fiche retirée n'est plus trouvée"
    );
}

fn retain_only_keeps_the_given_tasks(index: &dyn SearchIndex) {
    let kept = TaskBuilder::new(3, "Gardée").build();
    let dropped = TaskBuilder::new(4, "Oubliée").build();
    index.upsert(SearchDocument::new(&kept, digest("")));
    index.upsert(SearchDocument::new(&dropped, digest("")));
    index.retain_only(&[kept.id().clone()]);
    assert_eq!(
        index.indexed_version(dropped.id()),
        None,
        "contrat : retain_only oublie les autres"
    );
    assert_eq!(
        found(index, "gardee"),
        1,
        "contrat : retain_only garde celles demandées"
    );
}

const STORE: &str = "contrat : store doit réussir";
const PUT: &str = "contrat : put doit réussir";
const READ: &str = "contrat : read doit réussir";
const REMOVE: &str = "contrat : remove doit réussir (même absent)";

/// Contrat d'un stockage de médias **vide** : même contenu = même nom, un nom choisi par
/// l'application (média chiffré) n'est jamais réécrit, effacer est définitif et sans erreur si
/// le média est absent.
///
/// # Panique
/// Au premier écart.
pub fn check_media_store_contract(store: &dyn MediaStore) {
    let base = BaseId::main();
    let first = store.store(&base, b"image", "jpg").expect(STORE);
    assert_eq!(
        store.store(&base, b"image", "JPG").expect(STORE),
        first,
        "dédupliqué"
    );
    assert_ne!(store.store(&base, b"autre", "jpg").expect(STORE), first);
    assert_eq!(store.read(&first).expect(READ), b"image");

    let sealed = MediaRef::sealed_from_parts(&"c".repeat(64), "pdf").expect("nom valide");
    store.put(&base, &sealed, b"chiffre-1").expect(PUT);
    store.put(&base, &sealed, b"chiffre-2").expect(PUT);
    assert_eq!(
        store.read(&sealed).expect(READ),
        b"chiffre-1",
        "jamais réécrit"
    );

    store.remove(&base, &sealed).expect(REMOVE);
    store.remove(&base, &sealed).expect(REMOVE);
    assert!(matches!(
        store.read(&sealed),
        Err(RepositoryError::NotFound(_))
    ));
}
