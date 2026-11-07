//! Recherche plein texte (spec §5.6) : la partie **métier**, sans index ni stockage.
//!
//! - [`fold_text`] : normalisation (minuscules, sans accents) ;
//! - [`SearchQuery`] : analyse de la saisie (`tag:`, `is:`, `has:`, `in:`, phrases) ;
//! - [`SearchDocument`] : la fiche cherchable d'une tâche, préparée à l'indexation ;
//! - [`search_documents`] : correspondance, ordre des résultats, extraits surlignés.
//!
//! Où sont gardées les fiches et quand elles sont refaites : port `SearchIndex` (application)
//! et adaptateur `memory_index`.

mod document;
mod fold;
mod matching;
mod query;

pub use document::{MediaKind, NoteDigest, SearchDocument};
pub use fold::{fold_text, fold_with_origins};
pub use matching::{Highlight, SearchHit, Snippet, highlights, matches, search_documents};
pub use query::{SearchQuery, StatusFilter};
