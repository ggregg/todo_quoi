//! De quel type de stockage relève un dossier de base : disque local, disque externe, partage
//! réseau, ou drive cloud (spec §5.1bis). Une base peut être n'importe où : c'est l'outil qui
//! s'adapte — et qui le dit à l'utilisateur (nom affiché, messages adaptés).
//!
//! Classement **par le chemin**, sans accès au disque (un disque débranché se classe aussi) :
//! - dans un dossier de drive détecté, ou dans un emplacement typique d'un client de drive
//!   (`Mobile Documents`, `Library/CloudStorage/<Fournisseur>…`, dossier `OneDrive`, `Dropbox`,
//!   `Google Drive`, `iCloudDrive`) → drive cloud ;
//! - `/Volumes/…` (macOS), `/media/…`, `/mnt/…` (Linux), autre lettre que le disque système
//!   (Windows) → disque externe ; `\\serveur\partage` → partage réseau ;
//! - sinon → disque local.

use crate::drive_folders::{DriveFolder, DriveProvider};
use std::path::Path;

/// Type de stockage d'un dossier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    /// Dossier synchronisé par un client de drive.
    Cloud(DriveProvider),
    /// Disque amovible ou second disque.
    ExternalDisk,
    /// Partage réseau (`\\serveur\partage`).
    NetworkShare,
    /// Disque de la machine.
    LocalDisk,
}

/// Classe `path`. `drives` : dossiers de drive détectés ; `system_drive` : lettre du disque
/// système sous Windows (`C:`), `None` ailleurs.
pub fn classify_folder(
    path: &Path,
    drives: &[DriveFolder],
    system_drive: Option<&str>,
) -> StorageKind {
    if let Some(drive) = drives.iter().find(|drive| path.starts_with(&drive.path)) {
        return StorageKind::Cloud(drive.provider);
    }
    let text = path.to_string_lossy().replace('\\', "/");
    if let Some(provider) = provider_from_path(&text) {
        return StorageKind::Cloud(provider);
    }
    if text.starts_with("//") {
        return StorageKind::NetworkShare;
    }
    if ["/Volumes/", "/media/", "/mnt/"]
        .iter()
        .any(|prefix| text.starts_with(prefix))
    {
        return StorageKind::ExternalDisk;
    }
    match (system_drive, windows_drive_letter(&text)) {
        (Some(system), Some(letter)) if !letter.eq_ignore_ascii_case(system) => {
            StorageKind::ExternalDisk
        }
        _ => StorageKind::LocalDisk,
    }
}

/// Emplacements typiques des clients de drive, même non détectés sur cet appareil.
fn provider_from_path(text: &str) -> Option<DriveProvider> {
    let lower = text.to_lowercase();
    let patterns = [
        ("/mobile documents/", DriveProvider::ICloud),
        ("/icloud drive", DriveProvider::ICloud),
        ("/iclouddrive", DriveProvider::ICloud),
        ("/cloudstorage/googledrive", DriveProvider::GoogleDrive),
        ("/google drive", DriveProvider::GoogleDrive),
        ("/cloudstorage/onedrive", DriveProvider::OneDrive),
        ("/onedrive", DriveProvider::OneDrive),
        ("/cloudstorage/dropbox", DriveProvider::Dropbox),
        ("/dropbox", DriveProvider::Dropbox),
    ];
    patterns
        .iter()
        .find(|(pattern, _)| lower.contains(pattern))
        .map(|(_, provider)| *provider)
}

/// `D:/…` → `D:`.
fn windows_drive_letter(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':').then(|| &text[..2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn kind(path: &str, system: Option<&str>) -> StorageKind {
        let drives = [DriveFolder {
            provider: DriveProvider::Dropbox,
            path: PathBuf::from("/Users/greg/Boulot"),
        }];
        classify_folder(Path::new(path), &drives, system)
    }

    #[test]
    fn cloud_folders_are_recognised_detected_or_not() {
        assert_eq!(
            kind("/Users/greg/Boulot/TodoQuoi", None),
            StorageKind::Cloud(DriveProvider::Dropbox)
        );
        assert_eq!(
            kind(
                "/Users/greg/Library/Mobile Documents/com~apple~CloudDocs/TodoQuoi",
                None
            ),
            StorageKind::Cloud(DriveProvider::ICloud)
        );
        assert_eq!(
            kind(
                "/Users/greg/Library/CloudStorage/GoogleDrive-g@x.fr/Mon Drive/TodoQuoi",
                None
            ),
            StorageKind::Cloud(DriveProvider::GoogleDrive)
        );
        assert_eq!(
            kind("C:\\Users\\greg\\OneDrive\\TodoQuoi", Some("C:")),
            StorageKind::Cloud(DriveProvider::OneDrive)
        );
    }

    #[test]
    fn disks_and_shares_are_told_apart() {
        assert_eq!(
            kind("/Users/greg/Documents/TodoQuoi", None),
            StorageKind::LocalDisk
        );
        assert_eq!(
            kind("/Volumes/Clé USB/TodoQuoi", None),
            StorageKind::ExternalDisk
        );
        assert_eq!(
            kind("/media/greg/disque/TodoQuoi", None),
            StorageKind::ExternalDisk
        );
        assert_eq!(
            kind("C:\\Users\\greg\\TodoQuoi", Some("C:")),
            StorageKind::LocalDisk
        );
        assert_eq!(kind("E:\\TodoQuoi", Some("C:")), StorageKind::ExternalDisk);
        assert_eq!(
            kind("\\\\nas\\partage\\TodoQuoi", Some("C:")),
            StorageKind::NetworkShare
        );
    }
}
