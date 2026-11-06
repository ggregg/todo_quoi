//! # Adaptateur de stockage « fichiers Markdown »
//!
//! **Place dans l'hexagone** : adaptateur secondaire, implémente le port `TaskRepository`.
//!
//! Disposition dans le dossier choisi sur le drive (architecture §4, ADR-0004) :
//! ```text
//! TodoQuoi/
//! ├── manifest.json          version du format, pour les migrations
//! └── tasks/
//!     └── <uuid-v7>.md       une tâche : en-tête TOML entre +++ puis la note en Markdown
//! ```
//!
//! Deux modules, deux responsabilités :
//! - [`format`](mod@format) : texte ↔ tâche (aucun accès disque, testable seul) ;
//! - `storage` : lecture / écriture des fichiers (écriture atomique, fichiers illisibles écartés).

pub mod format;
mod manifest;
mod storage;

pub use storage::{FsMarkdownTaskRepository, RejectedFile};
