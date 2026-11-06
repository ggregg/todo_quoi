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
//! | [`delete_task`] | Mettre à la corbeille | R5 |
//! | [`restore_task`] | Restaurer depuis la corbeille | R7 |
//! | [`list_tasks`] | Lister les tâches actives | — |
//! | [`list_trash`] | Lister la corbeille | R6 |

pub mod close_task;
pub mod create_task;
pub mod delete_task;
pub mod edit_note;
pub mod get_close_impact;
pub mod get_task;
pub mod list_tasks;
pub mod list_trash;
pub mod move_task;
pub mod rename_task;
pub mod reopen_task;
pub mod restore_task;
pub mod set_tags;
