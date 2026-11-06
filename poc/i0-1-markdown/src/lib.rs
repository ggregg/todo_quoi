//! POC I0-1 — aller-retour « fichier de tâche Markdown ↔ document TipTap (JSON ProseMirror) ».
//!
//! **Code jetable** : cette crate répond à une seule question — *peut-on convertir une note
//! dans les deux sens sans perte ?* Le code de production vivra dans l'adaptateur
//! `fs_markdown` et passera par le modèle du domaine (`Note`, `Block`), pas par le JSON TipTap.
//!
//! Organisation :
//! - [`pm`]           : le format JSON de TipTap / ProseMirror (nœuds et marques) ;
//! - [`front_matter`] : l'en-tête TOML entre deux lignes `+++` ;
//! - [`directives`]   : repérage des blocs maison `:::radio`, `::audio`, `::file` ;
//! - [`parse`]        : Markdown → JSON ;
//! - [`render`]       : JSON → Markdown.

// [rust] `pub mod x;` déclare un module dont le code est dans `src/x.rs`.
//        Équivalent TS : un fichier `x.ts` ré-exporté par un `index.ts`.
pub mod directives;
pub mod front_matter;
pub mod parse;
pub mod pm;
pub mod render;

use serde::{Deserialize, Serialize};

/// Un fichier de tâche découpé en deux : l'en-tête TOML brut (s'il existe) et la note.
///
/// L'en-tête est gardé tel quel (texte) : dans ce POC, seul le corps passe par TipTap.
// [rust] `#[derive(...)]` génère du code à la compilation : ici l'affichage de debug,
//        la copie, la comparaison `==`, et la (dé)sérialisation JSON via serde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskFile {
    pub header: Option<String>,
    pub doc: pm::PmNode,
}

/// Lit un fichier de tâche complet (en-tête + corps Markdown).
pub fn parse_task_file(input: &str) -> TaskFile {
    let (header, body) = front_matter::split(input);
    TaskFile {
        // [rust] `Option::map` transforme la valeur si elle existe (≈ `header?.toString()`).
        header: header.map(str::to_string),
        doc: parse::markdown_to_doc(body),
    }
}

/// Réécrit un fichier de tâche complet à partir de l'en-tête et du document TipTap.
pub fn render_task_file(file: &TaskFile) -> String {
    let body = render::doc_to_markdown(&file.doc);
    // [rust] `match` sur un `Option` : les deux cas doivent être traités (exhaustivité).
    match &file.header {
        Some(header) => front_matter::join(header, &body),
        None => body,
    }
}
