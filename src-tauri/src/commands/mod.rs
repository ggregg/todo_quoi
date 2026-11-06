//! Commandes Tauri : ce que l'interface peut appeler avec `invoke('nom_de_commande', {...})`.
//!
//! Chaque commande est **mince** : traduire l'entrée (DTO → types de l'application), appeler un
//! cas d'usage, traduire la sortie. Aucune règle métier ici.
//!
//! Les cas d'usage sont synchrones (ADR-0002) : [`run`] les exécute dans un fil de travail pour
//! ne jamais bloquer l'interface pendant une lecture ou une écriture sur le drive.
//!
//! Regroupement par famille : [`queries`] (lecture), [`editing`], [`status`], [`hierarchy`],
//! [`trash`], [`detail`] (panneau de détail, note), [`windows`] (fenêtres de tâche),
//! [`media`] (stocker, ouvrir, révéler un média).

pub mod detail;
pub mod editing;
pub mod hierarchy;
pub mod media;
pub mod queries;
pub mod status;
pub mod trash;
pub mod windows;

use crate::error::UiError;
use tq_application::AppError;

/// Exécute `job` dans un fil de travail et convertit son erreur pour l'interface.
// [rust] `async fn` : la commande rend la main pendant que le fil de travail s'exécute
//        (≈ une fonction `async` qui `await` un worker). `'static + Send` : le travail peut
//        être envoyé à un autre fil car il ne contient aucune référence temporaire.
pub(crate) async fn run<T, Job>(job: Job) -> Result<T, UiError>
where
    T: Send + 'static,
    Job: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|error| UiError::internal(format!("tâche de fond interrompue : {error}")))?
        .map_err(UiError::from)
}
