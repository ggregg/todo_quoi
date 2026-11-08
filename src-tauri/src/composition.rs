//! **Composition root** : le seul endroit qui connaît les implémentations concrètes des ports
//! et qui les branche sur les cas d'usage (ADR-0001, ADR-0003).
//!
//! L'équivalent d'un conteneur d'injection de dépendances, écrit à la main pour rester lisible :
//! on voit d'un coup d'œil quel adaptateur sert à quoi.

use crate::events::TauriEventPublisher;
use crate::io_timeout::{DRIVE_TIMEOUT, with_timeout};
use crate::settings::{BaseSetting, LocalSettings};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tq_adapter_crypto::{Argon2VaultKeys, XChaChaCipher};
use tq_adapter_fs_markdown::{
    FsMarkdownTaskRepository, FsMediaStore, FsRootViewStore, FsVaultStore,
};
use tq_adapter_keyring::OsKeyVault;
use tq_adapter_memory_index::MemorySearchIndex;
use tq_adapter_multi_base::{
    MultiBaseDriveSync, MultiBaseMediaStore, MultiBaseRootViews, MultiBaseTaskRepository,
    MultiBaseVaults, OpenBases,
};
use tq_adapter_note_markdown::MarkdownNoteDigester;
use tq_adapter_system::{HybridClock, UuidV7Generator, machine_device_id};
use tq_application::ports::{
    BaseDirectory, Cipher, Clock, ContentSealer, DriveSync, EventPublisher, IdGenerator, KeyVault,
    MediaStore, RootViewStore, SearchIndex, TaskRepository, VaultKeys, VaultStore,
};
use tq_application::use_cases::{
    close_task::CloseTask, create_task::CreateTask, customize_list_view::CustomizeListView,
    delete_task::DeleteTask, edit_note::EditNote, get_close_impact::GetCloseImpact,
    get_task::GetTask, inherit_list_view::InheritListView, list_tasks::ListTasks,
    list_trash::ListTrash, move_task::MoveTask, read_media::ReadMedia, rename_task::RenameTask,
    reopen_task::ReopenTask, restore_task::RestoreTask, search_tasks::SearchTasks,
    set_tags::SetTags, store_media::StoreMedia,
};
use tq_application::use_cases::{
    create_vault::CreateVault, decrypt_task::DecryptTask, encrypt_task::EncryptTask,
    list_vaults::ListVaults, lock_vault::LockVault,
    restore_remembered_vaults::RestoreRememberedVaults, unlock_vault::UnlockVault,
};
use tq_application::use_cases::{
    keep_local_version::KeepLocalVersion, list_conflicts::ListConflicts,
    purge_expired::PurgeExpired, purge_task::PurgeTask, sync_with_drive::SyncWithDrive,
};
use tq_application::vault::{BaseSealer, OpenVaults};
use tq_application::{IndexingTaskRepository, Stamper};
use tq_domain::BaseId;

/// Variable d'environnement pour choisir le dossier de données (en attendant l'écran de choix
/// du dossier du drive, I5). Exemple : `TODOQUOI_DATA_DIR="$HOME/Library/Mobile Documents/com~apple~CloudDocs/TodoQuoi"`.
pub const DATA_DIR_VARIABLE: &str = "TODOQUOI_DATA_DIR";

/// Tous les cas d'usage, prêts à l'emploi, partagés par les commandes.
// [rust] Chaque cas d'usage est dans un `Arc` : une commande en prend une copie (du pointeur)
//        pour l'exécuter dans un fil de travail, sans copier le cas d'usage lui-même.
pub struct AppState {
    /// Lister les tâches actives.
    pub list_tasks: Arc<ListTasks>,
    /// Corbeille : lister, supprimer définitivement.
    pub trash: TrashUseCases,
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
    /// Synchronisation par le dossier du drive (scanner, vidage, journal des conflits).
    pub sync: SyncUseCases,
    /// Coffres des bases (ADR-0010).
    pub vault: VaultUseCases,
    /// Bases demandées et leur état à l'ouverture (ADR-0009).
    pub opened_bases: Vec<OpenedBase>,
}

/// Une base demandée dans les réglages, et pourquoi elle n'a pas pu être ouverte le cas échéant.
#[derive(Debug, Clone)]
pub struct OpenedBase {
    /// Réglage de la base (nom, couleur, dossier).
    pub setting: BaseSetting,
    /// `None` : ouverte ; sinon, la raison (dossier inaccessible…).
    pub problem: Option<String>,
}

/// Cas d'usage de la synchronisation, utilisés par le scanner (`scanner.rs`) et les commandes.
#[derive(Clone)]
pub struct SyncUseCases {
    /// Un passage sur le dossier.
    pub sync_with_drive: Arc<SyncWithDrive>,
    /// Vider corbeille et conflits de plus de 30 jours.
    pub purge_expired: Arc<PurgeExpired>,
    /// Journal des conflits.
    pub list_conflicts: Arc<ListConflicts>,
    /// « Garder la mienne ».
    pub keep_local_version: Arc<KeepLocalVersion>,
}

/// Cas d'usage de la corbeille (R6, R8).
pub struct TrashUseCases {
    /// Lister la corbeille.
    pub list: Arc<ListTrash>,
    /// Supprimer définitivement.
    pub purge: Arc<PurgeTask>,
}

/// Cas d'usage du coffre (spec §9, ADR-0010).
pub struct VaultUseCases {
    /// État du coffre de chaque base.
    pub list: Arc<ListVaults>,
    /// Créer le coffre d'une base.
    pub create: Arc<CreateVault>,
    /// Déverrouiller.
    pub unlock: Arc<UnlockVault>,
    /// Verrouiller.
    pub lock: Arc<LockVault>,
    /// Rouvrir les coffres gardés dans le trousseau.
    pub restore: Arc<RestoreRememberedVaults>,
    /// Chiffrer une tâche.
    pub encrypt: Arc<EncryptTask>,
    /// Remettre une tâche en clair.
    pub decrypt: Arc<DecryptTask>,
}

/// Coffres déverrouillés, **communs à tous les assemblages** : ajouter ou détacher une base
/// reconstruit l'assemblage sans refermer les coffres déjà ouverts. Géré par Tauri (`manage`).
pub struct SharedVaults(pub Arc<OpenVaults>);

/// L'assemblage **courant**, remplaçable à chaud : ajouter ou détacher une base reconstruit tout
/// (`build`) et remplace l'ancien, sans redémarrer le processus (ADR-0009). Les commandes en
/// prennent une copie du pointeur au début de chaque appel (`current()`).
// [rust] `RwLock<Arc<T>>` : beaucoup de lecteurs (chaque commande) prennent une copie de l'`Arc`
//        en un instant ; un seul écrivain (le remplacement) l'échange. Une commande déjà en cours
//        finit avec l'ancien assemblage, qui disparaît quand plus personne ne le tient.
pub struct LiveState(std::sync::RwLock<Arc<AppState>>);

impl LiveState {
    /// État de départ.
    pub fn new(state: AppState) -> Self {
        Self(std::sync::RwLock::new(Arc::new(state)))
    }

    /// Assemblage courant.
    pub fn current(&self) -> Arc<AppState> {
        self.0
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Remplace l'assemblage (nouvelle liste de bases).
    pub fn replace(&self, state: AppState) {
        *self
            .0
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(state);
    }
}

/// Assemble l'application.
///
/// # Erreurs
/// Dossier de données introuvable ou inutilisable.
pub fn build(app: &AppHandle) -> Result<AppState, Box<dyn std::error::Error>> {
    let location = data_location(app)?;
    let vaults = app
        .try_state::<SharedVaults>()
        .map(|shared| shared.0.clone())
        .ok_or("coffres non initialisés (SharedVaults)")?;
    let (ports, opened_bases) = Ports::real(app, &location, vaults)?;
    let state = ports.into_use_cases(opened_bases);
    // « Se souvenir sur cet appareil » : les coffres gardés dans le trousseau s'ouvrent seuls.
    state.vault.restore.execute();
    Ok(state)
}

/// Implémentations réelles des ports (adaptateurs secondaires).
struct Ports {
    tasks: Arc<dyn TaskRepository>,
    drive: Arc<dyn DriveSync>,
    clock: Arc<dyn Clock>,
    search: Arc<dyn SearchIndex>,
    root_view: Arc<dyn RootViewStore>,
    media: Arc<dyn MediaStore>,
    bases: Arc<dyn BaseDirectory>,
    ids: Arc<dyn IdGenerator>,
    events: Arc<dyn EventPublisher>,
    stamper: Stamper,
    vault: VaultPorts,
}

/// Coffres des bases (manifestes) et coffres ouverts.
struct VaultPorts {
    store: Arc<dyn VaultStore>,
    open: Arc<OpenVaults>,
}

/// Adaptateurs d'une base ouverte (ADR-0009).
struct BaseAdapters {
    tasks: Arc<dyn TaskRepository>,
    drive: Arc<dyn DriveSync>,
    media: Arc<dyn MediaStore>,
    root_view: Arc<dyn RootViewStore>,
    vault: Arc<dyn VaultStore>,
}

impl BaseAdapters {
    /// Ouvre le dossier d'une base, avec délai maximal : un drive bloqué ne bloque pas le
    /// démarrage (la base est alors signalée « inaccessible » et ignorée).
    fn open(path: PathBuf, sealer: Arc<dyn ContentSealer>) -> Result<Self, String> {
        let opened = with_timeout(DRIVE_TIMEOUT, move || -> Result<Self, String> {
            let files = FsMarkdownTaskRepository::open(&path)
                .map_err(|e| e.to_string())?
                .with_sealer(sealer);
            Ok(Self {
                vault: Arc::new(FsVaultStore::open(&path).map_err(|e| e.to_string())?),
                // Le scan partage les empreintes du dépôt : il ignore nos propres écritures.
                drive: Arc::new(files.drive_sync()),
                tasks: Arc::new(files),
                media: Arc::new(FsMediaStore::open(&path).map_err(|e| e.to_string())?),
                root_view: Arc::new(FsRootViewStore::open(&path).map_err(|e| e.to_string())?),
            })
        });
        opened.map_err(|_| "le dossier ne répond pas".to_string())?
    }
}

/// Composites qui présentent toutes les bases ouvertes comme une seule.
struct Composites {
    tasks: Vec<(BaseId, Arc<dyn TaskRepository>)>,
    drives: Vec<(BaseId, Arc<dyn DriveSync>)>,
    media: Vec<(BaseId, Arc<dyn MediaStore>)>,
    views: Vec<(BaseId, Arc<dyn RootViewStore>)>,
    vaults: Vec<(BaseId, Arc<dyn VaultStore>)>,
}

impl Ports {
    /// Étape 1 : ouvrir chaque base et assembler les adaptateurs.
    fn real(
        app: &AppHandle,
        location: &DataLocation,
        vaults: Arc<OpenVaults>,
    ) -> Result<(Self, Vec<OpenedBase>), Box<dyn std::error::Error>> {
        let mut composites = Composites {
            tasks: Vec::new(),
            drives: Vec::new(),
            media: Vec::new(),
            views: Vec::new(),
            vaults: Vec::new(),
        };
        let cipher: Arc<dyn Cipher> = Arc::new(XChaChaCipher::new());
        let mut opened = Vec::new();
        for setting in &location.bases {
            let id = setting.base_id();
            println!(
                "TodoQuoi : base « {} » dans {}",
                setting.name,
                setting.path.display()
            );
            // Chaque base chiffre avec **son** coffre (ADR-0010).
            let sealer = BaseSealer::new(id.clone(), vaults.clone(), cipher.clone());
            let problem = match BaseAdapters::open(setting.path.clone(), Arc::new(sealer)) {
                Ok(adapters) => {
                    composites.vaults.push((id.clone(), adapters.vault));
                    composites.tasks.push((id.clone(), adapters.tasks));
                    composites.drives.push((id.clone(), adapters.drive));
                    composites.media.push((id.clone(), adapters.media));
                    composites.views.push((id.clone(), adapters.root_view));
                    None
                }
                Err(problem) => Some(problem),
            };
            opened.push(OpenedBase {
                setting: setting.clone(),
                problem,
            });
        }
        Ok((Self::assemble(app, composites, vaults), opened))
    }

    fn assemble(app: &AppHandle, composites: Composites, vaults: Arc<OpenVaults>) -> Self {
        let search: Arc<dyn SearchIndex> = Arc::new(MemorySearchIndex::new());
        let open_ids = composites.tasks.iter().map(|(id, _)| id.clone()).collect();
        let media = Arc::new(MultiBaseMediaStore::new(composites.media));
        let files = MultiBaseTaskRepository::new(composites.tasks, media.clone());
        // Les cas d'usage voient le dépôt **enveloppé** : chaque lecture / écriture met l'index
        // de recherche à jour (décorateur, cf. `tq_application::IndexingTaskRepository`).
        let tasks = IndexingTaskRepository::new(
            Arc::new(files),
            search.clone(),
            Arc::new(MarkdownNoteDigester),
        );
        let clock: Arc<dyn Clock> = Arc::new(HybridClock::new());
        Self {
            tasks: Arc::new(tasks),
            drive: Arc::new(MultiBaseDriveSync::new(composites.drives)),
            clock: clock.clone(),
            search,
            root_view: Arc::new(MultiBaseRootViews::new(composites.views)),
            media,
            bases: Arc::new(OpenBases::new(open_ids)),
            ids: Arc::new(UuidV7Generator::new()),
            events: Arc::new(TauriEventPublisher::new(app.clone())),
            stamper: Stamper::new(clock, machine_device_id()),
            vault: VaultPorts {
                store: Arc::new(MultiBaseVaults::new(composites.vaults)),
                open: vaults,
            },
        }
    }

    /// Étape 2 : brancher chaque cas d'usage sur exactement les ports dont il a besoin.
    fn into_use_cases(self, opened_bases: Vec<OpenedBase>) -> AppState {
        let Self {
            tasks,
            drive,
            clock,
            search,
            root_view,
            media,
            bases,
            ids,
            events,
            stamper,
            vault,
        } = self;
        // Les cas d'usage de modification ont tous la même signature (dépôt, tampon, événements).
        let edit = || (tasks.clone(), stamper.clone(), events.clone());
        let vault = vault_use_cases(edit(), vault, bases.clone());
        let (list_tasks, customize_list_view) =
            list_view_use_cases(edit(), root_view, search.clone(), bases);
        AppState {
            list_tasks,
            trash: trash_use_cases(tasks.clone(), events.clone()),
            get_close_impact: Arc::new(GetCloseImpact::new(tasks.clone())),
            create_task: create_task_use_case(edit(), ids),
            rename_task: Arc::new(call(RenameTask::new, edit())),
            set_tags: Arc::new(call(SetTags::new, edit())),
            close_task: Arc::new(call(CloseTask::new, edit())),
            reopen_task: Arc::new(call(ReopenTask::new, edit())),
            move_task: Arc::new(call(MoveTask::new, edit())),
            delete_task: Arc::new(call(DeleteTask::new, edit())),
            restore_task: Arc::new(call(RestoreTask::new, edit())),
            get_task: Arc::new(GetTask::new(tasks.clone())),
            edit_note: Arc::new(call(EditNote::new, edit())),
            store_media: Arc::new(StoreMedia::new(media.clone(), tasks.clone())),
            read_media: Arc::new(ReadMedia::new(media)),
            search_tasks: Arc::new(SearchTasks::new(search)),
            customize_list_view,
            sync: sync_use_cases(tasks.clone(), drive, clock, events.clone()),
            inherit_list_view: Arc::new(call(InheritListView::new, edit())),
            vault,
            opened_bases,
        }
    }
}

/// Les cas d'usage qui ont besoin de la vue racine : lister (mise en forme) et personnaliser.
fn list_view_use_cases(
    (tasks, stamper, events): EditPorts,
    root_view: Arc<dyn RootViewStore>,
    search: Arc<dyn SearchIndex>,
    bases: Arc<dyn BaseDirectory>,
) -> (Arc<ListTasks>, Arc<CustomizeListView>) {
    let list = ListTasks::new(tasks.clone(), root_view.clone(), search, bases);
    let customize = CustomizeListView::new(tasks, root_view, stamper, events);
    (Arc::new(list), Arc::new(customize))
}

/// « Créer » : les ports de modification, plus le générateur d'identifiants.
fn create_task_use_case(
    (tasks, stamper, events): EditPorts,
    ids: Arc<dyn IdGenerator>,
) -> Arc<CreateTask> {
    Arc::new(CreateTask::new(tasks, ids, stamper, events))
}

/// Les cas d'usage du coffre : dérivation Argon2id, trousseau de l'OS, coffres ouverts partagés.
fn vault_use_cases(
    (tasks, stamper, events): EditPorts,
    VaultPorts {
        store,
        open: vaults,
    }: VaultPorts,
    bases: Arc<dyn BaseDirectory>,
) -> VaultUseCases {
    let keys: Arc<dyn VaultKeys> = Arc::new(Argon2VaultKeys::new());
    let keyring: Arc<dyn KeyVault> = Arc::new(OsKeyVault::system());
    let create = CreateVault::new(store.clone(), keys.clone(), keyring.clone(), vaults.clone());
    let unlock = UnlockVault::new(store.clone(), keys, keyring.clone(), vaults.clone());
    VaultUseCases {
        list: Arc::new(ListVaults::new(
            bases.clone(),
            store.clone(),
            vaults.clone(),
        )),
        create: Arc::new(create),
        unlock: Arc::new(unlock),
        lock: Arc::new(LockVault::new(vaults.clone())),
        restore: Arc::new(RestoreRememberedVaults::new(
            bases,
            store,
            keyring,
            vaults.clone(),
        )),
        encrypt: Arc::new(EncryptTask::new(
            tasks.clone(),
            stamper.clone(),
            events.clone(),
            vaults,
        )),
        decrypt: Arc::new(DecryptTask::new(tasks, stamper, events)),
    }
}

/// Les cas d'usage de la corbeille.
fn trash_use_cases(
    tasks: Arc<dyn TaskRepository>,
    events: Arc<dyn EventPublisher>,
) -> TrashUseCases {
    TrashUseCases {
        list: Arc::new(ListTrash::new(tasks.clone())),
        purge: Arc::new(PurgeTask::new(tasks, events)),
    }
}

/// Les cas d'usage de la synchronisation par le drive.
fn sync_use_cases(
    tasks: Arc<dyn TaskRepository>,
    drive: Arc<dyn DriveSync>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventPublisher>,
) -> SyncUseCases {
    SyncUseCases {
        sync_with_drive: Arc::new(SyncWithDrive::new(drive.clone(), events.clone())),
        purge_expired: Arc::new(PurgeExpired::new(tasks, drive.clone(), clock, events)),
        list_conflicts: Arc::new(ListConflicts::new(drive.clone())),
        keep_local_version: Arc::new(KeepLocalVersion::new(drive)),
    }
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

/// D'où vient la liste des bases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataLocationKind {
    /// Variable `TODOQUOI_DATA_DIR` (développement, essais) : une seule base, imposée.
    EnvironmentVariable,
    /// Réglages de l'appareil (bases ajoutées, ou à défaut le dossier local).
    Settings,
}

/// Bases à ouvrir pour cette exécution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLocation {
    /// Bases, dans l'ordre du bandeau.
    pub bases: Vec<BaseSetting>,
    /// Origine.
    pub kind: DataLocationKind,
    /// Dossier local à l'appareil (non synchronisé), utilisé tant qu'aucune base n'est ajoutée.
    pub device_folder: PathBuf,
}

/// Bases à ouvrir : `TODOQUOI_DATA_DIR` s'il est défini (une base), sinon les bases des réglages
/// de l'appareil, sinon le dossier local (`…/fr.todoquoi.app/espace-local`).
pub fn data_location(app: &AppHandle) -> Result<DataLocation, Box<dyn std::error::Error>> {
    let device_folder = app.path().app_local_data_dir()?.join("espace-local");
    if let Some(dir) = std::env::var_os(DATA_DIR_VARIABLE) {
        let settings = LocalSettings {
            data_dir: Some(PathBuf::from(dir)),
            ..LocalSettings::default()
        };
        let bases = settings.bases_or(device_folder.clone());
        return Ok(DataLocation {
            bases,
            kind: DataLocationKind::EnvironmentVariable,
            device_folder,
        });
    }
    let settings = LocalSettings::load(&app.path().app_config_dir()?);
    let bases = settings.bases_or(device_folder.clone());
    Ok(DataLocation {
        bases,
        kind: DataLocationKind::Settings,
        device_folder,
    })
}
