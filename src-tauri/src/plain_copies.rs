//! **Coquille Tauri** (hors de l'hexagone, détail technique de l'ouverture) : copies
//! **déchiffrées** des pièces jointes chiffrées, le temps de les ouvrir avec
//! l'application du système (Aperçu, Word…) qui ne lit que des fichiers.
//!
//! Elles vont dans un dossier temporaire propre à l'utilisateur (`$TMPDIR` sous macOS,
//! `%TEMP%` sous Windows), lisible par lui seul sous macOS / Linux, et sont **effacées** quand
//! un coffre se verrouille et au lancement suivant. Une note chiffrée ne protège plus une
//! pièce jointe ouverte tant que l'application qui l'affiche en garde une copie : c'est dit
//! dans la spécification §9.

use crate::error::UiError;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const FOLDER: &str = "todoquoi-ouverts";

fn folder() -> PathBuf {
    std::env::temp_dir().join(FOLDER)
}

/// Écrit `bytes` sous `file_name` et renvoie le chemin.
///
/// # Erreurs
/// Dossier temporaire impossible à créer ou à écrire.
pub fn write(file_name: &str, bytes: &[u8]) -> Result<PathBuf, UiError> {
    let folder = folder();
    fs::create_dir_all(&folder).map_err(|error| failure(&error))?;
    restrict_to_user(&folder);
    let path = folder.join(file_name);
    let mut file = fs::File::create(&path).map_err(|error| failure(&error))?;
    restrict_to_user(&path);
    file.write_all(bytes).map_err(|error| failure(&error))?;
    Ok(path)
}

/// Efface toutes les copies déchiffrées (coffre verrouillé, lancement). Rien à effacer : rien.
pub fn clear() {
    let _ = fs::remove_dir_all(folder()); // absent : très bien ; ouvert ailleurs : au prochain coup
}

fn failure(error: &std::io::Error) -> UiError {
    UiError::internal(format!("copie déchiffrée impossible : {error}"))
}

// [rust] Droits Unix « lecture / écriture pour le seul propriétaire » (0700 / 0600). Sous
//        Windows, `%TEMP%` est déjà propre au compte : rien à faire.
#[cfg(unix)]
fn restrict_to_user(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = if path.is_dir() { 0o700 } else { 0o600 };
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn restrict_to_user(_path: &std::path::Path) {}
