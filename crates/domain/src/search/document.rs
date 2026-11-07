//! Ce que la recherche sait d'une tâche : une **fiche** préparée une fois, à l'indexation.
//!
//! Le domaine ne lit pas le Markdown de la note (ADR-0007) : son texte « à plat » et ses médias
//! arrivent déjà extraits, sous forme de [`NoteDigest`] (fourni par l'adaptateur `note_markdown`).

use super::fold::fold_text;
use crate::{Task, TaskId, Timestamp};

/// Type de média présent dans une note (`has:image`, `has:audio`, `has:file`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind {
    /// Image.
    Image,
    /// Message audio.
    Audio,
    /// Pièce jointe.
    File,
}

/// Contenu cherchable d'une note : texte à plat (paragraphes, tableaux, cases, radios, noms de
/// pièces jointes…) et types de médias présents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoteDigest {
    /// Texte de la note, sans balisage, blocs séparés par des retours à la ligne.
    pub text: String,
    /// Types de médias présents (sans doublon).
    pub media: Vec<MediaKind>,
}

/// Fiche de recherche d'une tâche active (hors corbeille).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    /// Tâche décrite.
    pub id: TaskId,
    /// Parent, pour le filtre `in:` et le chemin affiché dans les résultats.
    pub parent: Option<TaskId>,
    /// Titre d'origine.
    pub title: String,
    /// Titre normalisé.
    pub folded_title: String,
    /// Étiquettes normalisées.
    pub folded_tags: Vec<String>,
    /// Statut.
    pub done: bool,
    /// Médias présents dans la note.
    pub media: Vec<MediaKind>,
    /// Texte d'origine de la note (pour les extraits).
    pub note_text: String,
    /// Texte normalisé de la note.
    pub folded_note: String,
    /// Version de la tâche décrite : une fiche n'est refaite que si la tâche a changé.
    pub version: Timestamp,
}

impl SearchDocument {
    /// Prépare la fiche d'une tâche à partir de sa note déjà extraite.
    pub fn new(task: &Task, digest: NoteDigest) -> Self {
        let title = task.title().as_str().to_string();
        Self {
            id: task.id().clone(),
            parent: task.parent().cloned(),
            folded_title: fold_text(&title),
            title,
            folded_tags: task
                .tags()
                .iter()
                .map(|tag| fold_text(tag.as_str()))
                .collect(),
            done: task.is_done(),
            media: digest.media,
            folded_note: fold_text(&digest.text),
            note_text: digest.text,
            version: task.updated_at(),
        }
    }
}
