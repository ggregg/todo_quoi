//! # Domaine de TodoQuoi
//!
//! **Place dans l'hexagone** : le centre. Ce module contient le vocabulaire métier (tâche,
//! étiquette, ordre…) et les règles (clôture, déplacement, corbeille).
//!
//! **Ce qu'il ne fait jamais** : lire ou écrire un fichier, connaître l'heure, générer un
//! identifiant aléatoire, parler à l'interface. Tout cela arrive « de l'extérieur » sous forme
//! de valeurs (un `Timestamp`, un `TaskId`…) fournies par la couche application.
//!
//! Organisation :
//! - types-valeurs validés : [`TaskId`], [`DeviceId`], [`Tag`], [`Title`], [`Timestamp`],
//!   [`OrderKey`], [`NoteBody`], [`MediaRef`] ;
//! - l'entité [`Task`] et son statut [`TaskStatus`] ;
//! - l'ensemble [`TaskTree`] (une tâche + ses sous-tâches = un arbre) ;
//! - les règles métier, dans [`rules`] (R1 à R12 de la spécification).

// [rust] `mod x;` déclare un module privé (fichier `x.rs`) ; `pub use` ré-exporte ses types
//        pour qu'on écrive `tq_domain::Task` au lieu de `tq_domain::task::Task`
//        (≈ un `index.ts` qui fait `export { Task } from './task'`).
mod error;
mod identifiers;
mod media;
mod note;
mod order_key;
mod tag;
mod task;
mod timestamp;
mod title;
mod tree;

pub mod rules;

pub use error::DomainError;
pub use identifiers::{DeviceId, TaskId};
pub use media::{MAX_MEDIA_BYTES, MEDIA_LINK_PREFIX, MediaRef, check_media_size};
pub use note::NoteBody;
pub use order_key::OrderKey;
pub use tag::Tag;
pub use task::{ChangeStamp, NewTask, Task, TaskRecord, TaskStatus};
pub use timestamp::Timestamp;
pub use title::Title;
pub use tree::TaskTree;
