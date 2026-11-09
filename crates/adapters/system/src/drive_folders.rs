//! Dossiers de drive présents sur l'appareil (iCloud, Google Drive, OneDrive, Dropbox), proposés
//! dans l'écran « Dossier de données » (ADR-0008 : pas de sélecteur natif).
//!
//! Emplacements usuels :
//! - macOS : `~/Library/Mobile Documents/com~apple~CloudDocs` (iCloud Drive),
//!   `~/Library/CloudStorage/<Fournisseur>-<compte>` (Google Drive, OneDrive, Dropbox récents),
//!   `~/Dropbox` (ancien client) ;
//! - Windows : variables `OneDrive` / `OneDriveCommercial`, `%USERPROFILE%\iCloudDrive`,
//!   `%USERPROFILE%\Dropbox`, et le lecteur de Google Drive (`G:\Mon Drive`, `G:\My Drive`).
//!
//! La détection ne fait que **regarder** si des dossiers existent : `metadata()` ne rapatrie
//! rien (POC I0-3). Elle est testée avec un faux dossier personnel.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// Service de drive reconnu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveProvider {
    /// iCloud Drive.
    ICloud,
    /// Google Drive.
    GoogleDrive,
    /// OneDrive (personnel ou professionnel).
    OneDrive,
    /// Dropbox.
    Dropbox,
}

/// Un dossier de drive trouvé sur l'appareil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFolder {
    /// Service.
    pub provider: DriveProvider,
    /// Dossier racine du drive (point de départ pour choisir le dossier d'une base).
    pub path: PathBuf,
}

/// Dossiers de drive de l'utilisateur courant, sans doublon, dans un ordre stable.
pub fn detect_drive_folders() -> Vec<DriveFolder> {
    let Some(home) = std::env::home_dir() else {
        return Vec::new();
    };
    detect_in(&home, &|name| std::env::var_os(name))
}

/// Détection à partir d'un dossier personnel et de variables d'environnement données (testable).
pub fn detect_in(home: &Path, env: &dyn Fn(&str) -> Option<OsString>) -> Vec<DriveFolder> {
    let mut found = Vec::new();
    let mut push = |provider, path: PathBuf| {
        if path.is_dir() && !found.iter().any(|known: &DriveFolder| known.path == path) {
            found.push(DriveFolder { provider, path });
        }
    };
    push(
        DriveProvider::ICloud,
        home.join("Library/Mobile Documents/com~apple~CloudDocs"),
    );
    push(DriveProvider::ICloud, home.join("iCloudDrive"));
    for (provider, path) in cloud_storage_folders(&home.join("Library/CloudStorage")) {
        push(provider, path);
    }
    for variable in ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"] {
        if let Some(path) = env(variable) {
            push(DriveProvider::OneDrive, PathBuf::from(path));
        }
    }
    push(DriveProvider::Dropbox, home.join("Dropbox"));
    for root in google_drive_roots(home) {
        push(DriveProvider::GoogleDrive, root);
    }
    found
}

/// `~/Library/CloudStorage/GoogleDrive-x@y/Mon Drive`, `OneDrive-Perso`, `Dropbox`… (macOS).
fn cloud_storage_folders(cloud_storage: &Path) -> Vec<(DriveProvider, PathBuf)> {
    let Ok(entries) = fs::read_dir(cloud_storage) else {
        return Vec::new();
    };
    let mut folders: Vec<(DriveProvider, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if name.starts_with("GoogleDrive") {
                return Some((DriveProvider::GoogleDrive, my_drive_inside(&path)));
            }
            if name.starts_with("OneDrive") {
                return Some((DriveProvider::OneDrive, path));
            }
            name.starts_with("Dropbox")
                .then_some((DriveProvider::Dropbox, path))
        })
        .collect();
    folders.sort_by(|a, b| a.1.cmp(&b.1));
    folders
}

/// Google Drive montre le dossier personnel sous un nom traduit : « Mon Drive », « My Drive »…
fn my_drive_inside(account: &Path) -> PathBuf {
    ["Mon Drive", "My Drive", "Mein Ablage", "Mi unidad"]
        .iter()
        .map(|name| account.join(name))
        .find(|path| path.is_dir())
        .unwrap_or_else(|| account.to_path_buf())
}

/// Google Drive pour Windows : un lecteur (souvent `G:`) ; ancien client : `~/Google Drive`.
fn google_drive_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots = vec![home.join("Google Drive")];
    if cfg!(target_os = "windows") {
        roots.extend(["G:\\Mon Drive", "G:\\My Drive"].iter().map(PathBuf::from));
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Faux dossier personnel, effacé à la fin du test.
    struct FakeHome(PathBuf);

    impl FakeHome {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("tq-drives-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("dossier temporaire");
            Self(path)
        }

        fn mkdir(&self, relative: &str) -> PathBuf {
            let path = self.0.join(relative);
            fs::create_dir_all(&path).expect("dossier");
            path
        }
    }

    impl Drop for FakeHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finds_macos_drives_and_the_translated_google_folder() {
        let home = FakeHome::new("macos");
        let icloud = home.mkdir("Library/Mobile Documents/com~apple~CloudDocs");
        let google = home.mkdir("Library/CloudStorage/GoogleDrive-greg@exemple.fr/Mon Drive");
        let onedrive = home.mkdir("Library/CloudStorage/OneDrive-Personnel");
        let found = detect_in(&home.0, &|_| None);
        assert_eq!(
            found,
            [
                DriveFolder {
                    provider: DriveProvider::ICloud,
                    path: icloud
                },
                DriveFolder {
                    provider: DriveProvider::GoogleDrive,
                    path: google
                },
                DriveFolder {
                    provider: DriveProvider::OneDrive,
                    path: onedrive
                },
            ]
        );
    }

    #[test]
    fn uses_onedrive_variables_without_duplicates() {
        let home = FakeHome::new("windows");
        let onedrive = home.mkdir("OneDrive");
        let dropbox = home.mkdir("Dropbox");
        let env = |name: &str| {
            (name == "OneDrive" || name == "OneDriveConsumer")
                .then(|| onedrive.clone().into_os_string())
        };
        let found = detect_in(&home.0, &env);
        assert_eq!(
            found
                .iter()
                .map(|folder| folder.provider)
                .collect::<Vec<_>>(),
            [DriveProvider::OneDrive, DriveProvider::Dropbox]
        );
        assert_eq!(found[1].path, dropbox);
    }

    #[test]
    fn nothing_is_found_in_an_empty_home() {
        let home = FakeHome::new("empty");
        assert!(detect_in(&home.0, &|_| None).is_empty());
    }
}
