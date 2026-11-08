//! Le dossier `.conflicts/` : les versions **perdantes** d'un conflit (spec §7), gardées 30 jours
//! comme filet de sécurité, puis effacées.
//!
//! Nom d'une version écartée : `<id>.<date>.<appareil>.md`, par exemple
//! `0195f3c2-….20261105T172512Z.mac-greg.md` — la date est celle de la mise à l'écart, l'appareil
//! celui qui avait écrit cette version. Le contenu est le fichier de tâche tel qu'il était.

use crate::format::parse_task_file;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tq_application::ports::{ConflictCopy, RepositoryError};
use tq_domain::{TaskId, Timestamp};

/// Nom du dossier, à la racine de l'espace.
pub(crate) const CONFLICTS_DIR: &str = ".conflicts";

/// Range `text` (une version de la tâche `id`, écrite par `device`) dans `.conflicts/`.
///
/// # Erreurs
/// Dossier impossible à créer ou à écrire.
pub(crate) fn set_aside(
    root: &Path,
    id: &str,
    device: &str,
    text: &str,
) -> std::io::Result<PathBuf> {
    let dir = root.join(CONFLICTS_DIR);
    fs::create_dir_all(&dir)?;
    let device: String = device
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let path = dir.join(format!("{id}.{}.{device}.md", compact(now())));
    fs::write(&path, text)?;
    Ok(path)
}

/// Versions écartées, les plus récentes d'abord.
pub(crate) fn list(root: &Path) -> Result<Vec<ConflictCopy>, RepositoryError> {
    let dir = root.join(CONFLICTS_DIR);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(Vec::new()); // pas encore de conflit
    };
    let mut copies: Vec<ConflictCopy> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| describe(&entry.path()))
        .collect();
    copies.sort_by_key(|copy| std::cmp::Reverse(copy.set_aside_at));
    Ok(copies)
}

/// Efface les versions écartées avant `before` ; renvoie leur nombre.
pub(crate) fn purge(root: &Path, before: Timestamp) -> Result<usize, RepositoryError> {
    let mut removed = 0;
    for copy in list(root)?
        .into_iter()
        .filter(|copy| copy.set_aside_at < before)
    {
        let path = root.join(CONFLICTS_DIR).join(&copy.file_name);
        fs::remove_file(&path).map_err(|error| {
            RepositoryError::Unavailable(format!("{} : {error}", path.display()))
        })?;
        removed += 1;
    }
    Ok(removed)
}

fn describe(path: &Path) -> Option<ConflictCopy> {
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let mut parts = file_name.strip_suffix(".md")?.split('.');
    let task_id = parts.next().and_then(|id| TaskId::parse(id).ok());
    let set_aside_at = parts.next().and_then(expand)?;
    let title = fs::read_to_string(path)
        .ok()
        .and_then(|text| parse_task_file(&text).ok())
        .map(|parsed| parsed.task.title().to_string());
    Some(ConflictCopy {
        task_id,
        title,
        file_name,
        set_aside_at,
    })
}

fn now() -> Timestamp {
    let millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        });
    Timestamp::from_millis(millis)
}

/// `2026-11-05T17:25:12.345Z` → `20261105T172512Z` (sans `:` interdit sous Windows, ni `.`).
fn compact(at: Timestamp) -> String {
    let text = at.to_rfc3339();
    let seconds = text.split('.').next().unwrap_or(&text);
    format!("{}Z", seconds.replace(['-', ':'], ""))
}

/// Inverse de [`compact`].
fn expand(compact: &str) -> Option<Timestamp> {
    let digits = compact.strip_suffix('Z')?;
    let (date, time) = digits.split_once('T')?;
    if date.len() != 8 || time.len() != 6 {
        return None;
    }
    let text = format!(
        "{}-{}-{}T{}:{}:{}.000Z",
        &date[..4],
        &date[4..6],
        &date[6..],
        &time[..2],
        &time[2..4],
        &time[4..]
    );
    Timestamp::parse_rfc3339(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_dates_go_back_and_forth_to_the_second() {
        let at = Timestamp::parse_rfc3339("2026-11-05T17:25:12.345Z").expect("date");
        assert_eq!(compact(at), "20261105T172512Z");
        assert_eq!(
            expand("20261105T172512Z"),
            Timestamp::parse_rfc3339("2026-11-05T17:25:12.000Z").ok()
        );
        assert_eq!(expand("2026-11-05"), None);
    }

    #[test]
    fn copies_are_listed_newest_first_and_purged_by_date() {
        let root = std::env::temp_dir().join(format!("tq-conflicts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join(CONFLICTS_DIR);
        fs::create_dir_all(&dir).expect("dossier");
        let id = "0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e";
        fs::write(dir.join(format!("{id}.20260101T000000Z.pc.md")), "ancienne").expect("écriture");
        let recent = set_aside(&root, id, "mac greg!", "récente").expect("mise à l'écart");
        assert!(
            recent
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(".macgreg.md"))
        );

        let copies = list(&root).expect("liste");
        assert_eq!(copies.len(), 2);
        assert!(copies[0].set_aside_at > copies[1].set_aside_at);
        assert_eq!(copies[1].task_id, TaskId::parse(id).ok());

        let before = Timestamp::parse_rfc3339("2026-06-01T00:00:00.000Z").expect("date");
        assert_eq!(purge(&root, before).expect("purge"), 1);
        assert_eq!(list(&root).expect("liste").len(), 1);
        let _ = fs::remove_dir_all(&root);
    }
}
