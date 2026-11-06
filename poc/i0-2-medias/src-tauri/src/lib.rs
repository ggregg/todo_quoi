//! POC I0-2 — médias dans la webview.
//!
//! Tout le travail (enregistrement audio, compression d'images) est fait **côté page**.
//! Le Rust ne sert qu'à deux choses :
//! - recevoir le rapport de tests et l'écrire dans un fichier (lisible hors de l'app) ;
//! - enregistrer les échantillons audio produits, pour les relire sur l'autre OS.

use std::fs;
use std::path::PathBuf;

/// Dossier des échantillons partagés entre OS (`public/samples` du POC).
/// Chemin connu à la compilation : acceptable pour un POC lancé depuis le dépôt.
fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../public/samples")
}

/// Écrit le rapport JSON de la page dans le dossier temporaire et l'annonce sur la console.
// [rust] `#[tauri::command]` rend la fonction appelable depuis la page : `invoke('report', { json })`.
//        Une erreur `Err(String)` devient une promesse rejetée côté TypeScript.
#[tauri::command]
fn report(json: String) -> Result<String, String> {
    let path = std::env::temp_dir().join("todoquoi-poc-i0-2.json");
    fs::write(&path, json).map_err(|error| error.to_string())?;
    println!("RAPPORT POC I0-2 -> {}", path.display());
    Ok(path.display().to_string())
}

/// Enregistre un échantillon (ex. `micro-macos.m4a`) dans `public/samples`.
#[tauri::command]
fn save_sample(name: String, bytes: Vec<u8>) -> Result<String, String> {
    let is_safe_name = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !name.starts_with('.');
    if !is_safe_name {
        return Err(format!("nom d'échantillon refusé : {name}"));
    }
    let dir = samples_dir();
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join(&name);
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path.display().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![report, save_sample])
        .run(tauri::generate_context!())
        // [rust] `expect` acceptable ici : sans fenêtre, l'application n'a aucune raison de continuer.
        .expect("impossible de démarrer l'application Tauri");
}
