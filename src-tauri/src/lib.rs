//! # Application de bureau TodoQuoi
//!
//! **Place dans l'hexagone** : à l'extérieur. C'est l'**adaptateur primaire** (l'interface Vue
//! appelle l'application par des commandes Tauri) et la **composition root** (le seul endroit
//! qui choisit les implémentations concrètes des ports).
//!
//! - [`composition`] : assemble dépôt fichiers, horloge, identifiants, cas d'usage ;
//! - [`commands`] : une fonction mince par action de l'interface — aucune règle métier ici ;
//! - [`dto`] : les objets échangés avec l'interface (JSON) ;
//! - [`events`] : implémentation du port `EventPublisher` (événements vers l'interface) ;
//! - [`error`] : l'erreur renvoyée à l'interface.

pub mod commands;
pub mod composition;
pub mod dto;
pub mod error;
pub mod events;

/// Démarre l'application.
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            // [rust] `?` dans `setup` : une erreur d'assemblage (dossier de données inutilisable…)
            //        arrête le démarrage avec un message, au lieu de lancer une app à moitié prête.
            let state = composition::build(app.handle())?;
            // [rust] `manage` rend l'état accessible à toutes les commandes (`State<AppState>`),
            //        un peu comme `app.provide()` en Vue.
            tauri::Manager::manage(app, state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::queries::list_tasks,
            commands::queries::list_trash,
            commands::queries::get_close_impact,
            commands::editing::create_task,
            commands::editing::rename_task,
            commands::editing::set_tags,
            commands::status::close_task,
            commands::status::reopen_task,
            commands::hierarchy::move_task,
            commands::trash::delete_task,
            commands::trash::restore_task,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("TodoQuoi n'a pas pu démarrer : {error}");
        std::process::exit(1);
    }
}
