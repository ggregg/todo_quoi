//! Ouvrir un fichier ou un lien avec l'application par défaut du système, ou montrer un fichier
//! dans le Finder / l'Explorateur. Aucune dépendance : on appelle les commandes standard de
//! chaque système (`open`, `explorer`, `xdg-open`).
//!
//! **Ce module ne vérifie rien** : les appelants lui passent un chemin de média résolu par le
//! cœur, ou un [`tq_domain::ExternalLink`] déjà vérifié (http, https, mailto).

use crate::error::UiError;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

/// Ouvre `target` (chemin de fichier ou lien) avec l'application par défaut.
///
/// # Erreurs
/// Commande du système introuvable.
pub fn open_with_system(target: &OsStr) -> Result<(), UiError> {
    launch(open_command(target))
}

/// Montre le fichier dans le gestionnaire de fichiers.
///
/// # Erreurs
/// Commande du système introuvable.
pub fn reveal_in_file_manager(path: &Path) -> Result<(), UiError> {
    launch(reveal_command(path))
}

fn launch(mut command: Command) -> Result<(), UiError> {
    let mut child = command.spawn().map_err(|error| {
        UiError::internal(format!(
            "impossible de lancer l'application du système : {error}"
        ))
    })?;
    // On attend la fin de `open` / `explorer` dans un fil séparé : un processus jamais attendu
    // resterait « zombie » jusqu'à la fermeture de TodoQuoi.
    std::thread::spawn(move || child.wait());
    Ok(())
}

// [rust] `#[cfg(target_os = "…")]` : seule la version de l'OS en cours est compilée.
//        Aucune dépendance : on appelle les commandes standard de chaque système.

#[cfg(target_os = "macos")]
fn open_command(target: &OsStr) -> Command {
    let mut command = Command::new("open");
    command.arg(target);
    command
}

#[cfg(target_os = "macos")]
fn reveal_command(path: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg("-R").arg(path);
    command
}

#[cfg(target_os = "windows")]
fn open_command(target: &OsStr) -> Command {
    let mut command = Command::new("explorer");
    command.arg(target);
    command
}

#[cfg(target_os = "windows")]
fn reveal_command(path: &Path) -> Command {
    let mut command = Command::new("explorer");
    // `/select,` et le chemin forment UN SEUL argument pour l'Explorateur.
    command.arg(format!("/select,{}", path.display()));
    command
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn open_command(target: &OsStr) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(target);
    command
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn reveal_command(path: &Path) -> Command {
    // Pas de commande standard « révéler » sous Linux : on ouvre le dossier.
    open_command(path.parent().unwrap_or(path).as_os_str())
}
