//! Port `LocalFiles` : les fichiers de l'utilisateur, hors des bases (joindre, déplacer, lier).
//!
//! « Programme » : ce que l'OS lancerait au lieu de l'ouvrir dans une application — extension
//! d'exécutable, de script ou d'installeur, ou fichier exécutable sans extension. Un dossier
//! (dont une application macOS `.app`) n'est jamais « ouvert » depuis une note.

use std::fs;
use std::path::Path;
use tq_application::ports::{LocalFiles, RepositoryError};
use tq_domain::LocalFileKind;

/// Accès aux fichiers par la bibliothèque standard.
#[derive(Debug, Default, Clone, Copy)]
pub struct FsLocalFiles;

impl FsLocalFiles {
    /// L'accès aux fichiers de cette machine.
    pub fn new() -> Self {
        Self
    }
}

impl LocalFiles for FsLocalFiles {
    fn kind(&self, path: &Path) -> LocalFileKind {
        let Ok(metadata) = fs::metadata(path) else {
            return LocalFileKind::Missing;
        };
        if metadata.is_dir() {
            LocalFileKind::Directory
        } else if is_program(path, &metadata) {
            LocalFileKind::Program
        } else {
            LocalFileKind::Document
        }
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, RepositoryError> {
        fs::read(path).map_err(|error| failure(path, &error))
    }

    fn remove(&self, path: &Path) -> Result<(), RepositoryError> {
        fs::remove_file(path).map_err(|error| failure(path, &error))
    }
}

fn failure(path: &Path, error: &std::io::Error) -> RepositoryError {
    match error.kind() {
        std::io::ErrorKind::NotFound => RepositoryError::NotFound(path.display().to_string()),
        _ => RepositoryError::Unavailable(format!("{} : {error}", path.display())),
    }
}

/// Extensions de programmes, de scripts et d'installeurs (tous systèmes confondus : une note
/// synchronisée peut venir d'un autre OS).
const PROGRAM_EXTENSIONS: [&str; 22] = [
    "exe", "com", "bat", "cmd", "msi", "ps1", "vbs", "js", "jse", "wsf", "scr", "lnk", "url",
    "reg", "sh", "command", "tool", "zsh", "bash", "pkg", "mpkg", "jar",
];

/// Programme : extension d'exécutable ou de script, ou (Unix) fichier exécutable **sans**
/// extension. Le bit `x` seul ne suffit pas : sur un disque externe ou un partage réseau, tous
/// les fichiers l'ont souvent, documents compris.
fn is_program(path: &Path, metadata: &fs::Metadata) -> bool {
    match path.extension() {
        Some(ext) => PROGRAM_EXTENSIONS.contains(&ext.to_string_lossy().to_lowercase().as_str()),
        None => is_executable(metadata),
    }
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    // [rust] `PermissionsExt` : permissions Unix (`rwx`), disponibles seulement sous Unix.
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_of_what_a_path_designates() {
        let dir = std::env::temp_dir().join(format!("tq-local-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let document = dir.join("devis.pdf");
        fs::write(&document, b"pdf").unwrap();
        let files = FsLocalFiles::new();
        assert_eq!(files.kind(&document), LocalFileKind::Document);
        assert_eq!(files.kind(&dir), LocalFileKind::Directory);
        assert_eq!(files.kind(&dir.join("absent.pdf")), LocalFileKind::Missing);
        assert_eq!(files.read(&document).unwrap(), b"pdf");
        files.remove(&document).unwrap();
        assert_eq!(files.kind(&document), LocalFileKind::Missing);
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn programs_are_known_by_extension_or_by_being_executable_without_one() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("tq-prog-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let cases = [
            ("script.sh", 0o644, LocalFileKind::Program),
            ("Setup.EXE", 0o644, LocalFileKind::Program),
            ("outil", 0o755, LocalFileKind::Program),
            ("notes", 0o644, LocalFileKind::Document),
            // Disque externe : tout est exécutable, un PDF reste un document.
            ("devis.pdf", 0o777, LocalFileKind::Document),
        ];
        for (name, mode, expected) in cases {
            let path = dir.join(name);
            fs::write(&path, b"x").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(FsLocalFiles::new().kind(&path), expected, "{name}");
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
