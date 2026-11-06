//! Tests d'aller-retour sur le corpus `tests/corpus/*.md`.
//!
//! Chaque fichier du corpus est écrit sous forme **canonique** : le relire puis le réécrire
//! doit redonner exactement le même texte, et le JSON intermédiaire doit survivre à une
//! sérialisation / désérialisation.

use md_roundtrip::{TaskFile, parse_task_file, render_task_file};
use std::fs;
use std::path::PathBuf;

// [rust] `env!("CARGO_MANIFEST_DIR")` : dossier de la crate, connu à la compilation.
fn corpus_files() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("dossier du corpus lisible")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    files
}

#[test]
fn corpus_is_not_empty() {
    assert!(corpus_files().len() >= 4);
}

#[test]
fn markdown_round_trip_is_exact() {
    for path in corpus_files() {
        let original = fs::read_to_string(&path).expect("fichier lisible");
        let rendered = render_task_file(&parse_task_file(&original));
        assert_eq!(
            rendered,
            original,
            "\n--- aller-retour différent pour {} ---",
            path.display()
        );
    }
}

#[test]
fn json_survives_serialization() {
    for path in corpus_files() {
        let original = fs::read_to_string(&path).expect("fichier lisible");
        let file = parse_task_file(&original);
        let json = serde_json::to_string(&file).expect("sérialisation JSON");
        let back: TaskFile = serde_json::from_str(&json).expect("désérialisation JSON");
        assert_eq!(back, file, "JSON différent pour {}", path.display());
    }
}

#[test]
fn headers_of_the_corpus_are_valid_toml() {
    for path in corpus_files() {
        let original = fs::read_to_string(&path).expect("fichier lisible");
        if let Some(header) = parse_task_file(&original).header {
            let typed = md_roundtrip::front_matter::parse_header(&header);
            assert!(typed.is_ok(), "{} : {:?}", path.display(), typed.err());
        }
    }
}
