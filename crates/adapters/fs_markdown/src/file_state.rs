//! Ce que l'on sait des fichiers de tâche **sans les lire** : empreinte (taille, date de
//! modification) et présence locale (fichier « à la demande » pas encore téléchargé).
//!
//! - L'empreinte sert au scan (architecture §5.2) : un fichier dont l'empreinte a changé a été
//!   modifié — par un autre appareil, si ce n'est pas nous qui venons de l'écrire.
//! - La présence locale est lue dans les métadonnées, avec la bibliothèque standard (ADR-0008) :
//!   drapeau `SF_DATALESS` sous macOS (iCloud, File Provider), attributs `RECALL_ON_*` sous
//!   Windows (Cloud Files). Lire un tel fichier le ferait télécharger **en bloquant** : on le
//!   laisse de côté et on le fait télécharger en tâche de fond.

use std::collections::{HashMap, HashSet};
use std::fs::Metadata;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;

/// Empreinte d'un fichier : suffit à voir qu'il a changé, sans le lire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

impl FileStamp {
    pub(crate) fn of(metadata: &Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }
}

/// Empreintes connues des fichiers de `tasks/`, **partagées** entre l'écriture (le dépôt note
/// ce qu'il vient d'écrire) et le scan (qui repère ce qui a changé d'autre). Le verrou sert aussi
/// à ne jamais écrire pendant un scan.
#[derive(Debug, Clone, Default)]
pub(crate) struct KnownFiles {
    inner: Arc<Mutex<KnownFilesState>>,
}

#[derive(Debug, Default)]
pub(crate) struct KnownFilesState {
    /// Empreintes au dernier passage (ou à notre dernière écriture).
    pub(crate) stamps: HashMap<PathBuf, FileStamp>,
    /// Faux tant que le premier scan (point de départ) n'a pas eu lieu.
    pub(crate) initialized: bool,
    /// Fichiers illisibles connus (indicateur ⚠ tant qu'ils le restent).
    pub(crate) unreadable: HashSet<PathBuf>,
}

impl KnownFiles {
    /// Verrouille l'état (un verrou « empoisonné » est repris : ce n'est qu'un cache).
    pub(crate) fn lock(&self) -> MutexGuard<'_, KnownFilesState> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Vrai si le fichier n'est pas présent localement (il faudrait le télécharger pour le lire).
#[cfg(target_os = "macos")]
pub(crate) fn is_dataless(metadata: &Metadata) -> bool {
    // [rust] `std::os::macos::fs::MetadataExt` : métadonnées propres à macOS (`st_flags`),
    //        disponibles seulement quand on compile pour macOS (cf. `#[cfg]`).
    use std::os::macos::fs::MetadataExt;
    /// `SF_DATALESS` (`<sys/stat.h>`) : contenu absent, fourni à la demande.
    const SF_DATALESS: u32 = 0x4000_0000;
    metadata.st_flags() & SF_DATALESS != 0
}

/// Vrai si le fichier n'est pas présent localement (il faudrait le télécharger pour le lire).
#[cfg(target_os = "windows")]
pub(crate) fn is_dataless(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    /// `FILE_ATTRIBUTE_OFFLINE`, `…_RECALL_ON_OPEN`, `…_RECALL_ON_DATA_ACCESS` (Cloud Files).
    const NOT_LOCAL: u32 = 0x0000_1000 | 0x0004_0000 | 0x0040_0000;
    metadata.file_attributes() & NOT_LOCAL != 0
}

/// Linux et autres : pas de fichiers à la demande gérés.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn is_dataless(_metadata: &Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn an_ordinary_file_is_local_and_its_stamp_follows_its_content() {
        let path = std::env::temp_dir().join(format!("tq-stamp-{}.md", std::process::id()));
        fs::write(&path, "a").expect("écriture");
        let before = FileStamp::of(&fs::metadata(&path).expect("métadonnées"));
        assert!(!is_dataless(&fs::metadata(&path).expect("métadonnées")));
        fs::write(&path, "abc").expect("écriture");
        let after = FileStamp::of(&fs::metadata(&path).expect("métadonnées"));
        assert_ne!(before, after, "la taille a changé");
        let _ = fs::remove_file(&path);
    }
}
