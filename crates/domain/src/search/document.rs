//! Ce que la recherche sait d'une tâche : une **fiche** préparée une fois, à l'indexation.
//!
//! Le domaine ne lit pas le Markdown de la note (ADR-0007) : son texte « à plat » et ses médias
//! arrivent déjà extraits, sous forme de [`NoteDigest`] (fourni par l'adaptateur `note_markdown`).

use super::fold::fold_text;
use crate::{BaseId, Task, TaskId, Timestamp};

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

/// Contenu cherchable d'une note : texte à plat (paragraphes, tableaux, cases, noms de pièces
/// jointes…) et types de médias présents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoteDigest {
    /// Texte de la note, sans balisage, blocs séparés par des retours à la ligne.
    pub text: String,
    /// Types de médias présents (sans doublon).
    pub media: Vec<MediaKind>,
}

/// Rangement d'une tâche indexée : les archives ne sont cherchées qu'à la demande (bouton
/// « Archives », R15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shelf {
    /// Tâche active.
    Active,
    /// Tâche archivée (lecture seule).
    Archived,
}

impl Shelf {
    /// Rangement de `task`.
    pub fn of(task: &Task) -> Self {
        if task.is_archived() {
            Self::Archived
        } else {
            Self::Active
        }
    }
}

/// Fiche de recherche d'une tâche active ou archivée (jamais de la corbeille).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    /// Tâche décrite.
    pub id: TaskId,
    /// Base de la tâche (recherche limitée aux bases affichées, ADR-0009).
    pub base: BaseId,
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
    /// Active ou archivée.
    pub shelf: Shelf,
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
            base: task.base().clone(),
            parent: task.parent().cloned(),
            folded_title: fold_text(&title),
            title,
            folded_tags: task
                .tags()
                .iter()
                .map(|tag| fold_text(tag.as_str()))
                .collect(),
            done: task.is_done(),
            shelf: Shelf::of(task),
            media: digest.media,
            folded_note: fold_text(&digest.text),
            note_text: digest.text,
            version: task.updated_at(),
        }
    }

    /// Vrai si la fiche décrit `task` telle qu'elle est : même version, même rangement (une
    /// tâche ressortie à la main d'`archive/` garde sa version mais change de rangement).
    pub fn is_current_for(&self, task: &Task) -> bool {
        self.version == task.updated_at() && self.shelf == Shelf::of(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::{id, stamp, tree};

    #[test]
    fn a_document_is_stale_when_the_task_changes_version_or_shelf() {
        let task = tree(&[("1", None)]).get(&id("1")).expect("existe").clone();
        let document = SearchDocument::new(&task, NoteDigest::default());
        assert!(document.is_current_for(&task));
        let mut archived = task.clone();
        archived.archive(&stamp(9)).expect("active");
        assert!(!document.is_current_for(&archived));
        // Fichier ressorti à la main d'`archive/` : même version, autre rangement.
        let mut moved_by_hand = archived.to_record();
        moved_by_hand.updated_at = task.updated_at();
        let archived_document =
            SearchDocument::new(&Task::from_record(moved_by_hand), NoteDigest::default());
        assert!(!archived_document.is_current_for(&task));
    }
}
