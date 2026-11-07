//! **Composition root** : le seul endroit qui connaît les implémentations concrètes des ports
//! et qui les branche sur les cas d'usage (ADR-0001, ADR-0003).
//!
//! L'équivalent d'un conteneur d'injection de dépendances, écrit à la main pour rester lisible :
//! on voit d'un coup d'œil quel adaptateur sert à quoi.

use crate::events::TauriEventPublisher;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tq_adapter_fs_markdown::{FsMarkdownTaskRepository, FsMediaStore, FsRootViewStore};
use tq_adapter_memory_index::MemorySearchIndex;
use tq_adapter_note_markdown::MarkdownNoteDigester;
use tq_adapter_system::{HybridClock, UuidV7Generator, machine_device_id};
use tq_application::ports::{
    EventPublisher, IdGenerator, MediaStore, RootViewStore, SearchIndex, TaskRepository,
};
use tq_application::use_cases::{
    close_task::CloseTask, create_task::CreateTask, customize_list_view::CustomizeListView,
    delete_task::DeleteTask, edit_note::EditNote, get_close_impact::GetCloseImpact,
    get_task::GetTask, inherit_list_view::InheritListView, list_tasks::ListTasks,
    list_trash::ListTrash, move_task::MoveTask, read_media::ReadMedia, rename_task::RenameTask,
    reopen_task::ReopenTask, restore_task::RestoreTask, search_tasks::SearchTasks,
    set_tags::SetTags, store_media::StoreMedia,
};
use tq_application::{IndexingTaskRepository, Stamper};

/// Variable d'environnement pour choisir le dossier de données (en attendant l'écran de choix
/// du dossier du drive, I5). Exemple : `TODOQUOI_DATA_DIR="$HOME/Library/Mobile Documents/com~apple~CloudDocs/TodoQuoi"`.
pub const DATA_DIR_VARIABLE: &str = "TODOQUOI_DATA_DIR";

/// Tous les cas d'usage, prêts à l'emploi, partagés par les commandes.
// [rust] Chaque cas d'usage est dans un `Arc` : une commande en prend une copie (du pointeur)
//        pour l'exécuter dans un fil de travail, sans copier le cas d'usage lui-même.
pub struct AppState {
    /// Lister les tâches actives.
    pub list_tasks: Arc<ListTasks>,
    /// Lister la corbeille.
    pub list_trash: Arc<ListTrash>,
    /// Impact d'une clôture.
    pub get_close_impact: Arc<GetCloseImpact>,
    /// Créer.
    pub create_task: Arc<CreateTask>,
    /// Renommer.
    pub rename_task: Arc<RenameTask>,
    /// Étiquettes.
    pub set_tags: Arc<SetTags>,
    /// Clôturer.
    pub close_task: Arc<CloseTask>,
    /// Rouvrir.
    pub reopen_task: Arc<ReopenTask>,
    /// Déplacer.
    pub move_task: Arc<MoveTask>,
    /// Mettre à la corbeille.
    pub delete_task: Arc<DeleteTask>,
    /// Restaurer.
    pub restore_task: Arc<RestoreTask>,
    /// Lire une tâche, note comprise.
    pub get_task: Arc<GetTask>,
    /// Modifier la note.
    pub edit_note: Arc<EditNote>,
    /// Stocker un média.
    pub store_media: Arc<StoreMedia>,
    /// Lire un média.
    pub read_media: Arc<ReadMedia>,
    /// Rechercher.
    pub search_tasks: Arc<SearchTasks>,
    /// Personnaliser une liste.
    pub customize_list_view: Arc<CustomizeListView>,
    /// Revenir à la vue héritée.
    pub inherit_list_view: Arc<InheritListView>,
}

/// Assemble l'application.
///
/// # Erreurs
/// Dossier de données introuvable ou inutilisable.
pub fn build(app: &AppHandle) -> Result<AppState, Box<dyn std::error::Error>> {
    let ports = Ports::real(app)?;
    Ok(ports.into_use_cases())
}

/// Implémentations réelles des ports (adaptateurs secondaires).
struct Ports {
    tasks: Arc<dyn TaskRepository>,
    search: Arc<dyn SearchIndex>,
    root_view: Arc<dyn RootViewStore>,
    media: Arc<dyn MediaStore>,
    ids: Arc<dyn IdGenerator>,
    events: Arc<dyn EventPublisher>,
    stamper: Stamper,
}

impl Ports {
    /// Étape 1 : choisir et créer les adaptateurs.
    fn real(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = data_dir(app)?;
        println!("TodoQuoi : données dans {}", data_dir.display());
        let search: Arc<dyn SearchIndex> = Arc::new(MemorySearchIndex::new());
        // Les cas d'usage voient le dépôt **enveloppé** : chaque lecture / écriture met l'index
        // de recherche à jour (décorateur, cf. `tq_application::IndexingTaskRepository`).
        let tasks = IndexingTaskRepository::new(
            Arc::new(FsMarkdownTaskRepository::open(&data_dir)?),
            search.clone(),
            Arc::new(MarkdownNoteDigester),
        );
        Ok(Self {
            tasks: Arc::new(tasks),
            search,
            root_view: Arc::new(FsRootViewStore::open(&data_dir)?),
            media: Arc::new(FsMediaStore::open(&data_dir)?),
            ids: Arc::new(UuidV7Generator::new()),
            events: Arc::new(TauriEventPublisher::new(app.clone())),
            stamper: Stamper::new(Arc::new(HybridClock::new()), machine_device_id()),
        })
    }

    /// Étape 2 : brancher chaque cas d'usage sur exactement les ports dont il a besoin.
    fn into_use_cases(self) -> AppState {
        let Self {
            tasks,
            search,
            root_view,
            media,
            ids,
            events,
            stamper,
        } = self;
        // Les cas d'usage de modification ont tous la même signature (dépôt, tampon, événements).
        let edit = || (tasks.clone(), stamper.clone(), events.clone());
        let (list_tasks, customize_list_view) =
            list_view_use_cases(edit(), root_view, search.clone());
        AppState {
            list_tasks,
            list_trash: Arc::new(ListTrash::new(tasks.clone())),
            get_close_impact: Arc::new(GetCloseImpact::new(tasks.clone())),
            create_task: Arc::new(CreateTask::new(
                tasks.clone(),
                ids,
                stamper.clone(),
                events.clone(),
            )),
            rename_task: Arc::new(call(RenameTask::new, edit())),
            set_tags: Arc::new(call(SetTags::new, edit())),
            close_task: Arc::new(call(CloseTask::new, edit())),
            reopen_task: Arc::new(call(ReopenTask::new, edit())),
            move_task: Arc::new(call(MoveTask::new, edit())),
            delete_task: Arc::new(call(DeleteTask::new, edit())),
            restore_task: Arc::new(call(RestoreTask::new, edit())),
            get_task: Arc::new(GetTask::new(tasks.clone())),
            edit_note: Arc::new(call(EditNote::new, edit())),
            store_media: Arc::new(StoreMedia::new(media.clone())),
            read_media: Arc::new(ReadMedia::new(media)),
            search_tasks: Arc::new(SearchTasks::new(search)),
            customize_list_view,
            inherit_list_view: Arc::new(call(InheritListView::new, edit())),
        }
    }
}

/// Les cas d'usage qui ont besoin de la vue racine : lister (mise en forme) et personnaliser.
fn list_view_use_cases(
    (tasks, stamper, events): EditPorts,
    root_view: Arc<dyn RootViewStore>,
    search: Arc<dyn SearchIndex>,
) -> (Arc<ListTasks>, Arc<CustomizeListView>) {
    let list = ListTasks::new(tasks.clone(), root_view.clone(), search);
    let customize = CustomizeListView::new(tasks, root_view, stamper, events);
    (Arc::new(list), Arc::new(customize))
}

/// Ports communs aux cas d'usage de modification.
type EditPorts = (Arc<dyn TaskRepository>, Stamper, Arc<dyn EventPublisher>);

/// Appelle un constructeur de cas d'usage avec les trois ports de modification.
// [rust] `impl FnOnce(A, B, C) -> T` : « n'importe quelle fonction à trois paramètres »,
//        ici `RenameTask::new`, `CloseTask::new`… passées comme des valeurs.
fn call<T>(
    constructor: impl FnOnce(Arc<dyn TaskRepository>, Stamper, Arc<dyn EventPublisher>) -> T,
    (tasks, stamper, events): EditPorts,
) -> T {
    constructor(tasks, stamper, events)
}

/// Dossier de données : `TODOQUOI_DATA_DIR` s'il est défini, sinon un dossier local à l'appareil
/// (`…/fr.todoquoi.app/espace-local`), fourni par Tauri selon l'OS.
fn data_dir(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(dir) = std::env::var_os(DATA_DIR_VARIABLE) {
        return Ok(PathBuf::from(dir));
    }
    Ok(app.path().app_local_data_dir()?.join("espace-local"))
}
