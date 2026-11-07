//! Présentation des listes (spec §5.3) : colonnes, tri, filtres, et leur héritage.
//!
//! - [`ListView`] : la vue d'une liste (types dans `list_view`) ;
//! - [`effective_view`] : quelle vue s'applique à une liste (la sienne, celle d'un ancêtre, ou
//!   la vue racine) ;
//! - [`arrange_children`] : les sous-tâches à afficher, filtrées et triées.
//!
//! La vue d'une tâche est enregistrée dans son fichier (`[view]`), la vue racine dans
//! `manifest.json` : elles sont synchronisées comme le reste. L'état déplié / replié, lui, reste
//! local à l'appareil.

mod arrangement;
mod inheritance;
mod list_view;

#[cfg(test)]
mod tests;

pub use arrangement::{Progress, arrange_children};
pub use inheritance::{ViewSource, effective_view};
pub use list_view::{
    Column, ListView, MediaChoice, SortDirection, SortField, SortKey, StatusChoice, ViewFilter,
};
