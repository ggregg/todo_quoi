//! Règles métier qui portent sur **plusieurs** tâches (l'arbre), numérotées comme dans la
//! spécification fonctionnelle (`docs/01-specification-fonctionnelle.md` §4).
//!
//! - [`closure`] : R1 à R3 — proposer de clôturer / rouvrir les parents ;
//! - [`hierarchy`] : R9 à R12 — indenter, désindenter, monter, descendre, déplacer ;
//! - [`trash`] : R5 et R7 — mettre à la corbeille et restaurer un sous-arbre.
//!
//! Ce sont des **fonctions pures** : elles lisent ou modifient un [`crate::TaskTree`] et
//! renvoient un résultat ; elles ne lisent ni l'heure ni le disque.

pub mod closure;
pub mod hierarchy;
pub mod trash;

#[cfg(test)]
pub(crate) mod test_support;
