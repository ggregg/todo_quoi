//! **Couche application**, aide partagée par deux cas d'usage : effacer **définitivement** des tâches et les médias que plus rien ne cite (R8) : partagé par
//! la purge manuelle (`purge_task`) et celle des 30 jours (`purge_expired`).

use crate::AppError;
use crate::ports::{MediaStore, TaskRepository};
use tq_domain::rules::trash::orphaned_media;
use tq_domain::{TaskId, TaskTree};

/// Efface les tâches `ids` puis leurs médias devenus orphelins ; renvoie le nombre de médias
/// effacés. Les médias à effacer sont calculés **avant** (sur l'arbre complet) : après, on ne
/// saurait plus ce que citaient les notes effacées.
///
/// # Erreurs
/// Stockage.
pub(crate) fn erase_for_good(
    tree: &TaskTree,
    ids: &[TaskId],
    tasks: &dyn TaskRepository,
    media: &dyn MediaStore,
) -> Result<usize, AppError> {
    let orphans = orphaned_media(tree, ids);
    for id in ids {
        tasks.remove(id)?;
    }
    for (base, orphan) in &orphans {
        media.remove(base, orphan)?;
    }
    Ok(orphans.len())
}
