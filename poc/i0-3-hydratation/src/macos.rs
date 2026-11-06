//! Interrogation et pilotage des fichiers « à la demande » sur macOS (iCloud Drive et
//! drives File Provider : Google Drive, OneDrive, Dropbox), via Foundation.
//!
//! Préfiguration de l'adaptateur `cloud_sync` (port `FileHydrator`) : seul endroit du projet
//! où l'on parle à l'OS dans son langage (Objective-C, via les liaisons `objc2`).

use objc2_foundation::{
    NSArray, NSFileManager, NSNumber, NSString, NSURL, NSURLResourceKey,
    NSURLUbiquitousItemDownloadingStatusKey, NSURLUbiquitousItemIsDownloadingKey,
    NSURLUbiquitousItemIsUploadedKey,
};
use std::path::Path;

/// État d'un fichier vu par le service de synchronisation de macOS.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudState {
    /// Le fichier appartient-il à un dossier synchronisé (iCloud / File Provider) ?
    pub ubiquitous: bool,
    /// `NotDownloaded`, `Downloaded` (pas forcément la dernière version) ou `Current`.
    pub status: Option<String>,
    pub downloading: Option<bool>,
    pub uploaded: Option<bool>,
}

/// Crée un `NSURL` neuf à chaque appel : `NSURL` met les valeurs en cache, et on veut l'état réel.
// [rust] `Retained<T>` : pointeur vers un objet Objective-C dont Rust gère le compteur de références
//        (≈ une référence d'objet ordinaire en TS ; libérée automatiquement).
fn file_url(path: &Path) -> objc2::rc::Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
}

/// Lit l'état de synchronisation d'un fichier.
pub fn cloud_state(path: &Path) -> Result<CloudState, String> {
    let url = file_url(path);
    let ubiquitous = NSFileManager::defaultManager().isUbiquitousItemAtURL(&url);

    // SAFETY: ces constantes sont des `NSString` statiques fournies par Foundation, valides
    // pendant toute la vie du processus ; on ne fait que les lire.
    let (status_key, downloading_key, uploaded_key): (
        &NSURLResourceKey,
        &NSURLResourceKey,
        &NSURLResourceKey,
    ) = unsafe {
        (
            NSURLUbiquitousItemDownloadingStatusKey,
            NSURLUbiquitousItemIsDownloadingKey,
            NSURLUbiquitousItemIsUploadedKey,
        )
    };
    let keys = NSArray::from_slice(&[status_key, downloading_key, uploaded_key]);
    let values = url
        .resourceValuesForKeys_error(&keys)
        .map_err(|error| error.localizedDescription().to_string())?;

    let read_bool = |key: &NSURLResourceKey| {
        values
            .objectForKey(key)
            .and_then(|value| value.downcast::<NSNumber>().ok())
            .map(|number| number.boolValue())
    };
    let status = values
        .objectForKey(status_key)
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|text| short_status(&text.to_string()));

    Ok(CloudState {
        ubiquitous,
        status,
        downloading: read_bool(downloading_key),
        uploaded: read_bool(uploaded_key),
    })
}

/// `NSURLUbiquitousItemDownloadingStatusCurrent` → `Current`.
fn short_status(raw: &str) -> String {
    raw.trim_start_matches("NSURLUbiquitousItemDownloadingStatus")
        .to_string()
}

/// Demande au service de synchronisation de télécharger le fichier (appel non bloquant).
pub fn request_download(path: &Path) -> Result<(), String> {
    NSFileManager::defaultManager()
        .startDownloadingUbiquitousItemAtURL_error(&file_url(path))
        .map_err(|error| error.localizedDescription().to_string())
}

/// Retire la copie locale du fichier (il reste dans le cloud). Sert uniquement au test.
pub fn evict(path: &Path) -> Result<(), String> {
    NSFileManager::defaultManager()
        .evictUbiquitousItemAtURL_error(&file_url(path))
        .map_err(|error| error.localizedDescription().to_string())
}
