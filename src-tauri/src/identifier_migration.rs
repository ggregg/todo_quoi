//! **Coquille Tauri** (hors de l'hexagone) : reprise des dossiers de l'ancien identifiant
//! d'application.
//!
//! L'identifiant (`tauri.conf.json`) nomme les dossiers de l'app : réglages (`settings.json`,
//! `app_config_dir`) et données locales (base « espace-local », `app_local_data_dir`). Il est passé
//! de `fr.todoquoi.app` (déconseillé : `.app` est l'extension des applications macOS) à
//! `fr.todoquoi.desktop`. Au premier lancement sous le nouvel identifiant, chaque ancien dossier
//! est **copié** (l'original reste, en secours) et les chemins de `settings.json` qui pointaient
//! dedans sont réécrits.
//!
//! Le thème et l'état d'affichage de l'arbre (stockage local de la webview) ne sont pas repris :
//! ils reviennent à leur valeur par défaut.

use crate::settings::LocalSettings;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Identifiant d'application des versions précédentes.
pub const PREVIOUS_IDENTIFIER: &str = "fr.todoquoi.app";

/// Reprend les dossiers de l'ancien identifiant, si besoin. Ne bloque jamais le démarrage : un
/// échec est signalé sur la console et l'app part de réglages neufs (l'ancien dossier reste).
pub fn run(app: &AppHandle) {
    let paths = app.path();
    let (Ok(config), Ok(local)) = (paths.app_config_dir(), paths.app_local_data_dir()) else {
        return;
    };
    for folder in [&config, &local] {
        if let Err(error) = copy_previous_folder(folder) {
            eprintln!(
                "TodoQuoi : reprise de {} impossible : {error}",
                folder.display()
            );
        }
    }
    if let Some(previous_local) = previous_of(&local) {
        rewrite_settings_paths(&config, &previous_local, &local);
    }
}

/// Le dossier de l'ancien identifiant, à côté de `current` (même dossier parent).
fn previous_of(current: &Path) -> Option<PathBuf> {
    Some(current.parent()?.join(PREVIOUS_IDENTIFIER))
}

/// Copie l'ancien dossier vers `current` s'il existe et que `current` n'existe pas encore.
/// Copie dans un dossier temporaire puis renommage : une coupure ne laisse pas un dossier à
/// moitié copié (il serait pris pour un dossier déjà repris).
///
/// # Erreurs
/// Lecture ou écriture impossible.
fn copy_previous_folder(current: &Path) -> io::Result<bool> {
    let Some(previous) = previous_of(current) else {
        return Ok(false);
    };
    if current.exists() || !previous.is_dir() {
        return Ok(false);
    }
    let temporary = current.with_extension("reprise");
    let _ = fs::remove_dir_all(&temporary); // reste d'une reprise interrompue
    copy_tree(&previous, &temporary)?;
    fs::rename(&temporary, current)?;
    Ok(true)
}

/// Copie récursive d'un dossier.
fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Dans `settings.json`, les dossiers de base situés dans l'ancien dossier local pointent
/// désormais dans le nouveau (sinon la base « espace-local » resterait dans l'ancien dossier).
fn rewrite_settings_paths(config: &Path, previous_local: &Path, local: &Path) {
    let mut settings = LocalSettings::load(config);
    if !moved_paths(&mut settings, previous_local, local) {
        return;
    }
    if let Err(error) = settings.save(config) {
        eprintln!("TodoQuoi : chemins des bases non mis à jour : {error}");
    }
}

/// Réécrit les chemins qui commencent par `previous` ; vrai si l'un d'eux a changé.
fn moved_paths(settings: &mut LocalSettings, previous: &Path, current: &Path) -> bool {
    let mut changed = false;
    let mut relocate = |path: &mut PathBuf| {
        if let Ok(rest) = path.strip_prefix(previous) {
            *path = current.join(rest);
            changed = true;
        }
    };
    for base in &mut settings.bases {
        relocate(&mut base.path);
    }
    if let Some(dir) = &mut settings.data_dir {
        relocate(dir);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::BaseSetting;

    fn temporary_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("tq-migration-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn the_previous_folder_is_copied_once_and_kept() {
        let root = temporary_root("copy");
        let previous = root.join(PREVIOUS_IDENTIFIER);
        fs::create_dir_all(previous.join("espace-local/tasks")).expect("dossier");
        fs::write(previous.join("settings.json"), "{}").expect("fichier");
        fs::write(previous.join("espace-local/tasks/a.md"), "tâche").expect("fichier");
        let current = root.join("fr.todoquoi.desktop");

        assert!(copy_previous_folder(&current).expect("copie"));
        assert_eq!(
            fs::read_to_string(current.join("espace-local/tasks/a.md")).expect("lu"),
            "tâche"
        );
        assert!(previous.join("settings.json").exists(), "l'original reste");
        assert!(
            !copy_previous_folder(&current).expect("rien"),
            "déjà repris"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_happens_without_a_previous_folder() {
        let root = temporary_root("none");
        let current = root.join("fr.todoquoi.desktop");
        assert!(!copy_previous_folder(&current).expect("rien"));
        assert!(!current.exists());
    }

    #[test]
    fn base_paths_inside_the_previous_folder_are_moved() {
        let previous = PathBuf::from("/data").join(PREVIOUS_IDENTIFIER);
        let current = PathBuf::from("/data/fr.todoquoi.desktop");
        let base = |path: PathBuf| BaseSetting {
            id: "x".into(),
            name: "X".into(),
            color: "#000000".into(),
            path,
        };
        let mut settings = LocalSettings {
            bases: vec![
                base(previous.join("espace-local")),
                base(PathBuf::from("/drive/TodoQuoi")),
            ],
            ..LocalSettings::default()
        };
        assert!(moved_paths(&mut settings, &previous, &current));
        assert_eq!(settings.bases[0].path, current.join("espace-local"));
        assert_eq!(settings.bases[1].path, PathBuf::from("/drive/TodoQuoi"));
    }
}
