//! # Adaptateur de stockage « fichiers Markdown »
//!
//! **Place dans l'hexagone** : adaptateur secondaire, implémente les ports `TaskRepository` et
//! `MediaStore`.
//!
//! Disposition dans le dossier choisi sur le drive (architecture §4, ADR-0004) :
//! ```text
//! TodoQuoi/
//! ├── manifest.json          version du format, pour les migrations
//! ├── tasks/
//! │   └── <uuid-v7>.md       une tâche : en-tête TOML entre +++ puis la note en Markdown
//! └── media/
//!     └── <sha256>.<ext>     image, audio, pièce jointe (immuable)
//! ```
//!
//! Modules :
//! - [`format`](mod@format) : texte ↔ tâche (aucun accès disque, testable seul) ;
//! - `storage` : lecture / écriture des fichiers (écriture atomique, fichiers illisibles écartés) ;
//! - `drive_sync` : le scan du dossier (changements d'ailleurs, copies de conflit, fichiers à la
//!   demande) — port `DriveSync` ; `conflicts` : le dossier `.conflicts/` ; `file_state` :
//!   empreintes des fichiers et présence locale.

mod conflicts;
mod drive_sync;
mod file_state;
pub mod format;
mod manifest;
mod media;
mod storage;
mod view_format;

pub use drive_sync::FsDriveSync;
pub use manifest::{FsRootViewStore, WORKSPACE_FOLDER, workspace_folder};
pub use media::FsMediaStore;
pub use storage::{FsMarkdownTaskRepository, RejectedFile};
