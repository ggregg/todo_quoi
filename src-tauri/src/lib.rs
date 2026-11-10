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
//! - [`error`] : l'erreur renvoyée à l'interface ;
//! - [`media_protocol`] : le protocole `tq-media://` qui sert images et audio à l'interface ;
//! - [`auto_lock`] : minuteur du verrouillage automatique des coffres ;
//! - [`plain_copies`] : copies déchiffrées des pièces jointes ouvertes (effacées au verrouillage).

pub mod auto_lock;
pub mod commands;
pub mod composition;
pub mod dto;
pub mod error;
pub mod events;
pub mod io_timeout;
pub mod media_protocol;
pub mod plain_copies;
pub mod scanner;
pub mod settings;
pub mod system_open;

/// Assemble l'application au démarrage : cas d'usage (`composition`), puis scanner du drive.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // [rust] `?` dans `setup` : une erreur d'assemblage (dossier de données inutilisable…)
    //        arrête le démarrage avec un message, au lieu de lancer une app à moitié prête.
    // Les coffres ouverts survivent aux rechargements de l'assemblage (ajout d'une base…).
    let vaults = std::sync::Arc::new(tq_application::vault::OpenVaults::new());
    tauri::Manager::manage(app, composition::SharedVaults(vaults.clone()));
    // Copies déchiffrées laissées par une session précédente (arrêt brutal) : effacées.
    plain_copies::clear();
    let auto_lock = composition::auto_lock(app.handle(), vaults)?;
    auto_lock::start(app.handle().clone(), auto_lock.clone());
    tauri::Manager::manage(app, auto_lock::SharedAutoLock(auto_lock));
    let state = composition::build(app.handle())?;
    // Le scanner tourne dans son propre fil pendant toute la vie de l'app ; il relit
    // l'assemblage courant à chaque passage (les bases peuvent changer à chaud).
    let monitor = scanner::start(app.handle().clone());
    // [rust] `manage` rend l'état accessible à toutes les commandes (`State<AppState>`),
    //        un peu comme `app.provide()` en Vue.
    tauri::Manager::manage(app, monitor);
    tauri::Manager::manage(app, composition::LiveState::new(state));
    Ok(())
}

/// Démarre l'application.
// La longueur vient de la **table** des commandes (`generate_handler!` doit toutes les recevoir
// d'un coup) : il n'y a pas de logique à découper.
#[allow(clippy::too_many_lines)]
pub fn run() {
    let result = tauri::Builder::default()
        // Sélecteur de dossier natif (fenêtre Bases) ; seule la permission `dialog:allow-open`
        // est accordée (capabilities/default.json).
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol(media_protocol::SCHEME, media_protocol::handle)
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::views::list_tasks,
            commands::views::customize_list_view,
            commands::views::inherit_list_view,
            commands::queries::list_trash,
            commands::queries::get_close_impact,
            commands::editing::create_task,
            commands::editing::rename_task,
            commands::editing::set_tags,
            commands::links::open_link,
            commands::settings::get_data_location,
            commands::settings::add_base,
            commands::settings::update_base,
            commands::settings::remove_base,
            commands::search::search_tasks,
            commands::sync::get_sync_status,
            commands::sync::sync_now,
            commands::sync::list_conflicts,
            commands::sync::keep_local_version,
            commands::vault::list_vaults,
            commands::vault::create_vault,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::change_vault_password,
            commands::vault::vault_activity,
            commands::vault::get_vault_auto_lock,
            commands::vault::set_vault_auto_lock,
            commands::vault::encrypt_task,
            commands::vault::decrypt_task,
            commands::status::close_task,
            commands::status::reopen_task,
            commands::hierarchy::move_task,
            commands::trash::delete_task,
            commands::trash::restore_task,
            commands::trash::purge_task,
            commands::detail::get_task_detail,
            commands::detail::update_note,
            commands::windows::open_task_window,
            commands::media::store_media,
            commands::media::open_media,
            commands::media::reveal_media,
            commands::files::attach_file,
            commands::files::link_file,
            commands::files::linked_file_status,
            commands::files::open_linked_file,
            commands::files::reveal_linked_file,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("TodoQuoi n'a pas pu démarrer : {error}");
        std::process::exit(1);
    }
}
