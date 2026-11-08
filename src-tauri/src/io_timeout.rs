//! Délai maximal pour les entrées / sorties sur le dossier du drive (architecture §5.2).
//!
//! Un dossier de drive peut **bloquer** longtemps (POC I0-3 : plus de 2 min au premier accès à
//! Google Drive). L'opération tourne donc dans son propre fil ; si elle dépasse le délai, on
//! rend la main avec une erreur « dossier inaccessible » et le fil finit seul, plus tard.

use std::sync::mpsc;
use std::time::Duration;

/// Délai habituel d'une opération sur le drive.
pub const DRIVE_TIMEOUT: Duration = Duration::from_secs(15);

/// L'opération n'a pas répondu à temps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedOut;

/// Exécute `job` dans un fil séparé et attend au plus `limit`.
///
/// # Erreurs
/// [`TimedOut`] si `job` n'a pas fini à temps (il continue en arrière-plan, résultat ignoré).
// [rust] `mpsc::channel` : une file « plusieurs producteurs, un consommateur » entre deux fils ;
//        `recv_timeout` attend un message au plus `limit` (≈ `Promise.race` avec un minuteur).
pub fn with_timeout<T, Job>(limit: Duration, job: Job) -> Result<T, TimedOut>
where
    T: Send + 'static,
    Job: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        // Si l'appelant a déjà abandonné, l'envoi échoue : sans importance.
        let _ = sender.send(job());
    });
    receiver.recv_timeout(limit).map_err(|_| TimedOut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_job_returns_its_result() {
        assert_eq!(with_timeout(Duration::from_secs(1), || 42), Ok(42));
    }

    #[test]
    fn a_blocked_job_times_out() {
        let result = with_timeout(Duration::from_millis(20), || {
            std::thread::sleep(Duration::from_millis(500));
        });
        assert_eq!(result, Err(TimedOut));
    }
}
