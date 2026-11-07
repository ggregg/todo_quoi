//! Contenu cherchable d'une note : implémentation du port
//! [`NoteDigester`](tq_application::ports::NoteDigester).
//!
//! On relit le Markdown avec [`markdown_to_doc`] (le même convertisseur que pour l'éditeur), puis
//! on parcourt le document : texte de chaque bloc (paragraphes, titres, cases, cellules de
//! tableau, options radio…), plus les textes portés par des attributs (légende d'image, nom de
//! pièce jointe, libellé d'un groupe radio), et les types de médias rencontrés.

use crate::parse::markdown_to_doc;
use crate::pm::PmNode;
use tq_application::ports::NoteDigester;
use tq_domain::NoteBody;
use tq_domain::search::{MediaKind, NoteDigest};

/// Extraction du texte des notes en Markdown étendu.
#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownNoteDigester;

impl NoteDigester for MarkdownNoteDigester {
    fn digest(&self, note: &NoteBody) -> NoteDigest {
        digest_markdown(note.as_markdown())
    }
}

/// Texte à plat et médias d'une note, un bloc par ligne.
///
/// ```
/// use tq_adapter_note_markdown::digest_markdown;
/// let digest = digest_markdown("# Titre\n\n- [x] **Fait**\n");
/// assert_eq!(digest.text, "Titre\nFait");
/// ```
pub fn digest_markdown(markdown: &str) -> NoteDigest {
    let mut digest = NoteDigest::default();
    let mut lines = Vec::new();
    collect(&markdown_to_doc(markdown), &mut lines, &mut digest.media);
    digest.text = lines.join("\n");
    digest
}

/// Parcourt `node` : ajoute ses lignes de texte et ses médias.
fn collect(node: &PmNode, lines: &mut Vec<String>, media: &mut Vec<MediaKind>) {
    if let Some(kind) = media_kind(&node.kind)
        && !media.contains(&kind)
    {
        media.push(kind);
    }
    push_attribute_text(node, lines);
    if is_text_block(node) {
        push_line(lines, node.plain_text());
        return;
    }
    for child in node.children() {
        collect(child, lines, media);
    }
}

fn media_kind(node_kind: &str) -> Option<MediaKind> {
    match node_kind {
        "image" => Some(MediaKind::Image),
        "audio" => Some(MediaKind::Audio),
        "attachment" => Some(MediaKind::File),
        _ => None,
    }
}

/// Textes portés par des attributs : légende (`alt`, `title`), nom de fichier, libellé.
fn push_attribute_text(node: &PmNode, lines: &mut Vec<String>) {
    for key in ["alt", "title", "name", "label"] {
        if let Some(text) = node.attr(key).and_then(|value| value.as_str()) {
            push_line(lines, text.to_string());
        }
    }
}

/// Bloc dont tout le contenu est du texte en ligne : une ligne de résultat.
fn is_text_block(node: &PmNode) -> bool {
    matches!(
        node.kind.as_str(),
        "paragraph" | "heading" | "codeBlock" | "radioOption"
    )
}

fn push_line(lines: &mut Vec<String>, text: String) {
    let text = text.trim();
    if !text.is_empty() {
        lines.push(text.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::digest_markdown;
    use tq_domain::search::MediaKind;

    #[test]
    fn every_kind_of_block_contributes_text() {
        let markdown = "\
Intro *importante*

| Colonne | Valeur |
|---|---|
| prix | 12 € |

- [ ] acheter du lait

:::radio{label=\"Cible\"}
- ( ) Recette
- (x) Production
:::

```
let code = 1;
```
";
        let text = digest_markdown(markdown).text;
        for expected in [
            "Intro importante",
            "Colonne",
            "12 €",
            "acheter du lait",
            "Cible",
            "Production",
            "let code = 1;",
        ] {
            assert!(text.contains(expected), "« {expected} » absent de : {text}");
        }
    }

    #[test]
    fn media_are_detected_and_their_names_are_searchable() {
        let markdown = "\
![schéma réseau](../media/3f9a5e0c2b3f9a5e0c2b3f9a5e0c2b3f9a5e0c2b3f9a5e0c2b3f9a5e0c2b3f9a.jpg)

::file{src=\"../media/81bd0e81bd0e81bd0e81bd0e81bd0e81bd0e81bd0e81bd0e81bd0e81bd0e81bd.pdf\" name=\"devis-2027.pdf\" size=1200}

::audio{src=\"../media/5c11aa5c11aa5c11aa5c11aa5c11aa5c11aa5c11aa5c11aa5c11aa5c11aa5c11.webm\" duration=3.5}
";
        let digest = digest_markdown(markdown);
        assert_eq!(
            digest.media,
            [MediaKind::Image, MediaKind::File, MediaKind::Audio]
        );
        assert!(digest.text.contains("schéma réseau"));
        assert!(digest.text.contains("devis-2027.pdf"));
    }

    #[test]
    fn an_empty_note_has_no_text_nor_media() {
        let digest = digest_markdown("");
        assert!(digest.text.is_empty() && digest.media.is_empty());
    }
}
