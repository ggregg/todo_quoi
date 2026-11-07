//! # Conversion de la note : Markdown étendu ↔ éditeur TipTap
//!
//! **Place dans l'hexagone** : adaptateur (traduction de format), appelé à la frontière par
//! `src-tauri` (ADR-0007). Le domaine stocke la note en Markdown sans la comprendre ; l'éditeur,
//! lui, travaille sur un document JSON ProseMirror. Cette crate passe de l'un à l'autre.
//!
//! Code issu du POC I0-1 (`poc/i0-1-markdown/RESULTATS.md`), validé sans perte par aller-retour
//! sur un corpus, y compris en passant par le schéma TipTap réel.
//!
//! - [`markdown_to_doc`] : Markdown → `{ "type": "doc", … }` ;
//! - [`doc_to_markdown`] : document → Markdown, sous une forme canonique stable ;
//! - [`MarkdownNoteDigester`] : texte à plat et médias, pour la recherche (port `NoteDigester`).
//!
//! Modules : [`pm`] (format JSON), `directives` (blocs maison `:::radio`, `::audio`, `::file`),
//! `parse` et `render` (les deux sens de conversion).

mod digest;
mod directives;
mod parse;
pub mod pm;
mod render;

pub use digest::{MarkdownNoteDigester, digest_markdown};
pub use parse::markdown_to_doc;
pub use pm::{Attrs, PmMark, PmNode};
pub use render::doc_to_markdown;
