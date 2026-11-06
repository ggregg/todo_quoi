//! Contrat de `TaskRepository` : ce que **toute** implémentation doit garantir.
//!
//! Chaque adaptateur appelle [`check_task_repository_contract`] dans ses propres tests, avec un
//! dépôt **vide**. Le faux en mémoire passe la même suite : on est sûr que les tests des cas
//! d'usage (faits avec le faux) restent valables avec le vrai stockage.

use crate::TaskBuilder;
use tq_application::ports::TaskRepository;
use tq_domain::Task;

/// Exécute toutes les vérifications du contrat sur un dépôt vide.
///
/// # Panique
/// Au premier écart, avec un message qui dit quelle garantie n'est pas tenue.
pub fn check_task_repository_contract(repository: &dyn TaskRepository) {
    starts_empty(repository);
    saved_task_is_loaded_identically(repository);
    saving_again_replaces(repository);
    every_field_survives(repository);
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
