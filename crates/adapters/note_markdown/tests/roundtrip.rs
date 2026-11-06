//! Aller-retour sur le corpus `tests/corpus/*.md` (repris du POC I0-1) : chaque fichier est sous
//! forme canonique, le relire puis le réécrire doit redonner exactement le même texte.

// Fichier de tests : un `unwrap` / `expect` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use tq_adapter_note_markdown::{PmNode, doc_to_markdown, markdown_to_doc};

fn corpus() -> Vec<(PathBuf, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect()
}

#[test]
fn corpus_is_present() {
    assert!(corpus().len() >= 4);
}

#[test]
fn markdown_round_trip_is_exact() {
    for (path, original) in corpus() {
        let rendered = doc_to_markdown(&markdown_to_doc(&original));
        assert_eq!(
            rendered,
            original,
            "aller-retour différent pour {}",
            path.display()
        );
    }
}

#[test]
fn json_survives_serialization() {
    for (path, original) in corpus() {
        let doc = markdown_to_doc(&original);
        let json = serde_json::to_string(&doc).unwrap();
        let back: PmNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, doc, "JSON différent pour {}", path.display());
    }
}

#[test]
fn an_empty_note_is_an_empty_document_and_back() {
    let doc = markdown_to_doc("");
    assert_eq!(doc.kind, "doc");
    assert_eq!(doc.children().len(), 1, "TipTap exige au moins un bloc");
    assert_eq!(doc_to_markdown(&doc), "");
}

#[test]
fn default_attributes_added_by_tiptap_are_ignored() {
    // Forme renvoyée par `editor.getJSON()` : attributs par défaut à `null`, `target`, `rel`…
    let json = serde_json::json!({ "type": "doc", "content": [
        { "type": "paragraph", "attrs": { "textAlign": null }, "content": [
            { "type": "text", "text": "Voir " },
            { "type": "text", "text": "le site", "marks": [{ "type": "link", "attrs": {
                "href": "https://exemple.fr", "target": "_blank", "rel": "noopener", "class": null, "title": null } }] }
        ]},
        { "type": "codeBlock", "attrs": { "language": null }, "content": [{ "type": "text", "text": "x = 1" }] }
    ]});
    let doc: PmNode = serde_json::from_value(json).unwrap();
    assert_eq!(
        doc_to_markdown(&doc),
        "Voir [le site](https://exemple.fr)\n\n```\nx = 1\n```\n"
    );
}

#[test]
fn empty_paragraphs_between_blocks_are_dropped() {
    // Constat de l'I3 : des paragraphes vides entre un audio et un groupe radio.
    let json = serde_json::json!({ "type": "doc", "content": [
        { "type": "paragraph", "content": [{ "type": "text", "text": "Avant" }] },
        { "type": "paragraph" },
        { "type": "paragraph" },
        { "type": "paragraph", "content": [{ "type": "text", "text": "Après" }] }
    ]});
    let doc: PmNode = serde_json::from_value(json).unwrap();
    assert_eq!(doc_to_markdown(&doc), "Avant\n\nAprès\n");
}

#[test]
fn trailing_empty_paragraphs_from_the_editor_are_dropped() {
    // L'éditeur garde un paragraphe vide après un bloc de code (constat de l'I2).
    let json = serde_json::json!({ "type": "doc", "content": [
        { "type": "codeBlock", "attrs": { "language": "typescript" }, "content": [{ "type": "text", "text": "const x = 1" }] },
        { "type": "paragraph" },
        { "type": "paragraph", "content": [] }
    ]});
    let doc: PmNode = serde_json::from_value(json).unwrap();
    assert_eq!(doc_to_markdown(&doc), "```typescript\nconst x = 1\n```\n");
}
