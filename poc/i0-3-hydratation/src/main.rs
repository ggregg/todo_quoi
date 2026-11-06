//! POC I0-3 — fichiers « à la demande » des drives cloud (volet macOS).
//!
//! ```text
//! hydrate etat <fichier>                 état d'un fichier (local / dataless / statut cloud)
//! hydrate scenario <dossier-du-drive>    scénario complet dans <dossier>/TodoQuoi-POC/
//! ```
//!
//! Le scénario ne touche qu'à un fichier qu'il crée lui-même dans `TodoQuoi-POC/` :
//! écriture → attente de l'envoi → éviction → mesures → téléchargement par l'API →
//! nouvelle éviction → simple lecture. Rien n'est supprimé.

#[cfg(target_os = "macos")]
mod macos;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = match (args.get(1).map(String::as_str), args.get(2)) {
        (Some("etat"), Some(path)) => show_state(Path::new(path)),
        (Some("scenario"), Some(dir)) => run_scenario(Path::new(dir)),
        _ => {
            Err("usage : hydrate etat <fichier> | hydrate scenario <dossier-du-drive>".to_string())
        }
    };
    // [rust] `if let Err(e) = …` : on ne traite que le cas d'erreur (≈ `catch`).
    if let Err(error) = result {
        eprintln!("✘ {error}");
        std::process::exit(1);
    }
}

// ----- Détection sans API : drapeau « dataless » du système de fichiers ------------------

/// Drapeau `SF_DATALESS` de `<sys/stat.h>` : le contenu n'est pas sur le disque.
const SF_DATALESS: u32 = 0x4000_0000;

/// Lit taille, date de modification et drapeau « dataless » — **sans** télécharger le fichier
/// (c'est exactement ce que fera le scan périodique).
fn quick_stat(path: &Path) -> Result<(u64, SystemTime, bool), String> {
    let metadata = fs::metadata(path).map_err(|e| format!("stat {} : {e}", path.display()))?;
    let modified = metadata.modified().map_err(|e| e.to_string())?;
    Ok((metadata.len(), modified, is_dataless(&metadata)))
}

#[cfg(target_os = "macos")]
fn is_dataless(metadata: &fs::Metadata) -> bool {
    // [rust] Trait spécifique à macOS qui ajoute `st_flags()` aux métadonnées standard.
    use std::os::macos::fs::MetadataExt;
    metadata.st_flags() & SF_DATALESS != 0
}

#[cfg(not(target_os = "macos"))]
fn is_dataless(_metadata: &fs::Metadata) -> bool {
    false
}

// ----- Commandes --------------------------------------------------------------------------

fn show_state(path: &Path) -> Result<(), String> {
    let (size, modified, dataless) = quick_stat(path)?;
    println!("fichier   : {}", path.display());
    println!(
        "taille    : {size} o · modifié : {} · dataless : {dataless}",
        seconds(modified)
    );
    #[cfg(target_os = "macos")]
    println!("cloud     : {:?}", macos::cloud_state(path)?);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn run_scenario(_dir: &Path) -> Result<(), String> {
    Err("scénario disponible uniquement sur macOS dans ce POC".to_string())
}

#[cfg(target_os = "macos")]
fn run_scenario(drive_dir: &Path) -> Result<(), String> {
    let file = create_test_file(drive_dir)?;
    let content = fs::read(&file).map_err(|e| e.to_string())?;
    step(&format!("fichier de test créé : {}", file.display()));
    report_state(&file)?;

    step("1. attente de l'envoi vers le cloud (max 3 min)");
    let uploaded = wait_until(Duration::from_secs(180), || {
        Ok(macos::cloud_state(&file)?.uploaded == Some(true))
    })?;
    println!("   envoyé : {uploaded:?}");

    step("2. éviction de la copie locale");
    macos::evict(&file)?;
    let evicted = wait_until(Duration::from_secs(60), || Ok(quick_stat(&file)?.2))?;
    println!("   dataless après éviction : {evicted:?}");
    report_state(&file)?;

    step("3. scan : taille et date lisibles sans télécharger ?");
    let (size, modified, still_dataless) = quick_stat(&file)?;
    println!(
        "   taille {size} o (attendu {}) · modifié {} · toujours dataless : {still_dataless}",
        content.len(),
        seconds(modified)
    );

    step("4. téléchargement demandé par l'API (startDownloadingUbiquitousItem)");
    macos::request_download(&file)?;
    let downloaded = wait_until(Duration::from_secs(120), || Ok(!quick_stat(&file)?.2))?;
    println!("   présent localement : {downloaded:?}");
    report_state(&file)?;

    step("5. nouvelle éviction, puis simple lecture du fichier (sans API)");
    macos::evict(&file)?;
    wait_until(Duration::from_secs(60), || Ok(quick_stat(&file)?.2))?;
    let started = Instant::now();
    let read_back = fs::read(&file);
    let elapsed = started.elapsed();
    match read_back {
        Ok(bytes) => println!(
            "   lecture : {} o en {} ms · contenu identique : {}",
            bytes.len(),
            elapsed.as_millis(),
            bytes == content
        ),
        Err(error) => println!(
            "   lecture : ÉCHEC après {} ms : {error}",
            elapsed.as_millis()
        ),
    }
    report_state(&file)?;
    step("terminé (le fichier de test est conservé dans TodoQuoi-POC/)");
    Ok(())
}

// ----- Utilitaires ------------------------------------------------------------------------

/// Écrit ~200 Ko de texte dans `<drive>/TodoQuoi-POC/poc-<horodatage>.md`.
fn create_test_file(drive_dir: &Path) -> Result<PathBuf, String> {
    if !drive_dir.is_dir() {
        return Err(format!("dossier introuvable : {}", drive_dir.display()));
    }
    let dir = drive_dir.join("TodoQuoi-POC");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = seconds(SystemTime::now());
    let file = dir.join(format!("poc-{stamp}.md"));
    let line = "Fichier de test du POC I0-3 de TodoQuoi : peut être supprimé sans risque.\n";
    fs::write(&file, line.repeat(2700)).map_err(|e| e.to_string())?;
    Ok(file)
}

/// Répète `check` toutes les 500 ms jusqu'à `true` ou expiration. Renvoie la durée écoulée,
/// ou `None` si le délai est dépassé.
fn wait_until(
    timeout: Duration,
    mut check: impl FnMut() -> Result<bool, String>,
) -> Result<Option<Duration>, String> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if check()? {
            return Ok(Some(started.elapsed()));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Ok(None)
}

#[cfg(target_os = "macos")]
fn report_state(file: &Path) -> Result<(), String> {
    let (_, _, dataless) = quick_stat(file)?;
    println!(
        "   état : dataless={dataless} · {:?}",
        macos::cloud_state(file)?
    );
    Ok(())
}

fn step(text: &str) {
    println!("\n▶ {text}");
}

fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
