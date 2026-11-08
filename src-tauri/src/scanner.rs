//! **Scanner** du dossier du drive (adaptateur primaire, architecture §2.4 et §5.2) : un fil qui,
//! toutes les 3 s — ou tout de suite sur demande (retour au premier plan) —, appelle le cas
//! d'usage `SyncWithDrive` avec un délai maximal, et publie l'état pour l'indicateur
//! (événement `sync-status`). Au démarrage puis une fois par jour : `PurgeExpired`.

use crate::composition::{LiveState, SyncUseCases};
use crate::io_timeout::{DRIVE_TIMEOUT, with_timeout};
use serde::Serialize;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tq_application::use_cases::sync_with_drive::SyncStatus;

/// Intervalle entre deux passages.
pub const SCAN_INTERVAL: Duration = Duration::from_secs(3);
/// Intervalle entre deux vidages (corbeille, conflits).
const PURGE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// Nom de l'événement envoyé à l'interface.
pub const SYNC_STATUS_EVENT: &str = "sync-status";

/// État affiché par l'indicateur (miroir de `SyncStatus`, plus « hors ligne »).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusDto {
    /// `'starting' | 'upToDate' | 'downloading' | 'attention' | 'offline'`.
    pub kind: &'static str,
    /// Fichiers en attente de téléchargement.
    pub pending: usize,
    /// Fichiers illisibles.
    pub unreadable: usize,
    /// Conflits réglés depuis la dernière consultation du journal.
    pub conflicts: usize,
    /// Explication (dossier inaccessible…).
    pub message: Option<String>,
}

/// Partagé entre le fil du scanner et les commandes (`State<SyncMonitor>`).
pub struct SyncMonitor {
    status: Mutex<SyncStatusDto>,
    unseen_conflicts: Mutex<usize>,
    wake: Mutex<Sender<()>>,
}

impl SyncMonitor {
    /// Dernier état connu.
    pub fn status(&self) -> SyncStatusDto {
        lock(&self.status).clone()
    }

    /// Demande un passage immédiat (sans attendre les 3 s).
    pub fn wake(&self) {
        let _ = lock(&self.wake).send(());
    }

    /// Le journal des conflits a été consulté : l'indicateur cesse de les signaler.
    pub fn acknowledge_conflicts(&self) {
        *lock(&self.unseen_conflicts) = 0;
    }
}

/// Démarre le scanner ; renvoie l'état partagé à enregistrer avec `manage`.
pub fn start(app: AppHandle) -> Arc<SyncMonitor> {
    let (sender, receiver) = mpsc::channel();
    let monitor = Arc::new(SyncMonitor {
        status: Mutex::new(SyncStatusDto {
            kind: "starting",
            ..SyncStatusDto::default()
        }),
        unseen_conflicts: Mutex::new(0),
        wake: Mutex::new(sender),
    });
    let shared = monitor.clone();
    std::thread::spawn(move || run(&app, &shared, &receiver));
    monitor
}

fn run(app: &AppHandle, monitor: &SyncMonitor, wake: &Receiver<()>) {
    let mut last_purge: Option<Instant> = None;
    loop {
        // L'assemblage courant (l'état est enregistré juste après le démarrage du fil).
        let Some(sync) = current_sync(app) else {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        };
        let sync = &sync;
        if last_purge.is_none_or(|at| at.elapsed() >= PURGE_INTERVAL) {
            let purge = sync.purge_expired.clone();
            let _ = with_timeout(DRIVE_TIMEOUT, move || purge.execute());
            last_purge = Some(Instant::now());
        }
        let status = one_pass(sync, monitor);
        if *lock(&monitor.status) != status {
            *lock(&monitor.status) = status.clone();
            let _ = app.emit(SYNC_STATUS_EVENT, status);
        }
        // Attend 3 s, ou moins si l'interface demande un passage ; l'app fermée → on s'arrête.
        if let Err(mpsc::RecvTimeoutError::Disconnected) = wake.recv_timeout(SCAN_INTERVAL) {
            return;
        }
    }
}

/// Cas d'usage de synchronisation de l'assemblage courant (`None` avant l'enregistrement de l'état).
fn current_sync(app: &AppHandle) -> Option<SyncUseCases> {
    use tauri::Manager;
    app.try_state::<LiveState>()
        .map(|live| live.current().sync.clone())
}

fn one_pass(sync: &SyncUseCases, monitor: &SyncMonitor) -> SyncStatusDto {
    let use_case = sync.sync_with_drive.clone();
    match with_timeout(DRIVE_TIMEOUT, move || use_case.execute()) {
        Err(_) => offline(
            "un dossier de base ne répond pas (disque débranché, drive hors ligne ?)".to_string(),
        ),
        Ok(Err(error)) => offline(error.to_string()),
        Ok(Ok(status)) => describe(status, monitor),
    }
}

fn describe(status: SyncStatus, monitor: &SyncMonitor) -> SyncStatusDto {
    let mut unseen = lock(&monitor.unseen_conflicts);
    let (pending, unreadable) = match status {
        SyncStatus::UpToDate => (0, 0),
        SyncStatus::Downloading { pending } => (pending, 0),
        SyncStatus::Attention {
            unreadable,
            conflicts,
        } => {
            *unseen += conflicts;
            (0, unreadable)
        }
    };
    let kind = if unreadable > 0 || *unseen > 0 {
        "attention"
    } else if pending > 0 {
        "downloading"
    } else {
        "upToDate"
    };
    SyncStatusDto {
        kind,
        pending,
        unreadable,
        conflicts: *unseen,
        message: None,
    }
}

fn offline(message: String) -> SyncStatusDto {
    SyncStatusDto {
        kind: "offline",
        message: Some(message),
        ..SyncStatusDto::default()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
