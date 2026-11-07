//! Cas d'usage : **une action de l'utilisateur = un fichier**.
//!
//! Chaque cas d'usage est une `struct` qui reçoit ses ports à la construction (`new`) et expose
//! une méthode `execute`. Ceux qui modifient suivent le même déroulé (voir `session.rs`) :
//! charger → appliquer une règle du domaine → enregistrer → prévenir l'interface.
//!
//! | Fichier | Action | Règles |
//! |---|---|---|
//! | [`create_task`] | Créer une tâche | — |
//! | [`rename_task`] | Renommer | — |
//! | [`get_close_impact`] | Combien de sous-tâches ouvertes avant de clôturer ? | R2 |
//! | [`close_task`] | Clôturer (± sous-tâches) | R1, R2 |
//! | [`reopen_task`] | Rouvrir | R3 |
//! | [`move_task`] | Indenter, désindenter, monter, descendre, déplacer | R9–R12 |
//! | [`set_tags`] | Remplacer les étiquettes | — |
//! | [`get_task`] | Lire une tâche, note comprise | — |
//! | [`edit_note`] | Modifier la note | — |
//! | [`store_media`] | Stocker un média (≤ 50 Mo) | — |
//! | [`read_media`] | Lire un média | — |
//! | [`delete_task`] | Mettre à la corbeille | R5 |
//! | [`restore_task`] | Restaurer depuis la corbeille | R7 |
//! | [`list_tasks`] | Lister les tâches actives et la mise en forme de chaque liste | §5.3 |
//! | [`customize_list_view`] | Personnaliser la vue d'une liste | §5.3 |
//! | [`inherit_list_view`] | Revenir à la vue héritée | §5.3 |
//! | [`list_trash`] | Lister la corbeille | R6 |
//! | [`search_tasks`] | Rechercher (titre, étiquettes, note, filtres) | — |

pub mod close_task;
pub mod create_task;
pub mod customize_list_view;
pub mod delete_task;
pub mod edit_note;
pub mod get_close_impact;
pub mod get_task;
pub mod inherit_list_view;
pub mod list_tasks;
pub mod list_trash;
pub mod move_task;
pub mod read_media;
pub mod rename_task;
pub mod reopen_task;
pub mod restore_task;
pub mod search_tasks;
pub mod set_tags;
pub mod store_media;
