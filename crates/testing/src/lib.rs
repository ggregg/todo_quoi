//! # Kit de test de TodoQuoi
//!
//! **Place dans l'hexagone** : à côté des adaptateurs. Ce sont de **faux adaptateurs**, simples
//! et entièrement contrôlables, pour tester les cas d'usage sans disque ni horloge réelle :
//!
//! - [`fakes`] : un faux de chaque port (dépôt en mémoire, horloge qui avance d'1 ms à chaque
//!   lecture, identifiants séquentiels, enregistreur d'événements) ;
//! - [`vault_fakes`] : faux ports du coffre (chiffrement simulé, trousseau, manifeste) ;
//! - [`TestApp`] : tous les faux déjà assemblés, avec un raccourci par cas d'usage ;
//! - [`TaskBuilder`] : fabriquer une tâche de test en une ligne ;
//! - [`contract`] : la suite de tests que **toute** implémentation de `TaskRepository` doit
//!   réussir (principe de substitution de Liskov : le faux et le vrai se comportent pareil).

// Cette crate n'est utilisée que par les tests : un `expect` qui échoue y signale un test en
// échec, avec un message explicite. C'est exactement le comportement voulu.
#![allow(clippy::expect_used)]

pub mod contract;
pub mod fakes;
pub mod vault_fakes;

mod builder;
mod test_app;

pub use builder::{TaskBuilder, task_id};
pub use test_app::TestApp;
