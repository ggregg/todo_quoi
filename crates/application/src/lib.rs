//! # Application TodoQuoi
//!
//! **Place dans l'hexagone** : autour du domaine. Contient :
//! - les **ports** ([`ports`]) : les interfaces dont l'application a besoin (stocker des tâches,
//!   connaître l'heure, générer un identifiant, prévenir l'interface) — sans savoir comment
//!   elles sont réalisées ;
//! - les **cas d'usage** ([`use_cases`]) : une action de l'utilisateur = un fichier, qui charge
//!   les tâches, applique une règle du domaine, enregistre et prévient l'interface ;
//! - le **coffre** ([`vault`]) : secrets, coffres ouverts, scelleur de chaque base (ADR-0010) ;
//! - [`IndexingTaskRepository`] : tient l'index de recherche à jour à chaque lecture / écriture.
//!
//! **Ce qu'elle ne fait jamais** : ouvrir un fichier, parler à Tauri, contenir une règle métier
//! (celles-ci sont dans `tq_domain`).

mod error;
mod indexing;
mod session;
mod stamper;

pub mod ports;
pub mod use_cases;
pub mod vault;

pub use error::AppError;
pub use indexing::IndexingTaskRepository;
pub use stamper::Stamper;
