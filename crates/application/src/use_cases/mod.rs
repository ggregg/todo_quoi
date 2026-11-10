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
//! | [`attach_file`] | Joindre un fichier choisi : copier ou déplacer dans la base | — |
//! | [`link_file`] | Lier un fichier (chemin + appareil, rien n'est copié) | — |
//! | [`check_linked_file`] | Que permet un fichier lié ici (ouvrir, révéler, rien) | — |
//! | [`read_media`] | Lire un média | — |
//! | [`delete_task`] | Mettre à la corbeille | R5 |
//! | [`restore_task`] | Restaurer depuis la corbeille | R7 |
//! | [`purge_task`] | Supprimer définitivement depuis la corbeille | R8 |
//! | [`list_tasks`] | Lister les tâches actives et la mise en forme de chaque liste | §5.3 |
//! | [`customize_list_view`] | Personnaliser la vue d'une liste | §5.3 |
//! | [`inherit_list_view`] | Revenir à la vue héritée | §5.3 |
//! | [`list_trash`] | Lister la corbeille | R6 |
//! | [`search_tasks`] | Rechercher (titre, étiquettes, note, filtres) | — |
//! | [`sync_with_drive`] | Passage sur le dossier du drive (changements d'ailleurs, conflits) | §7 |
//! | [`purge_expired`] | Vider corbeille et conflits de plus de 30 jours | R8, §11 |
//! | [`list_conflicts`] | Journal des conflits | §7 |
//! | [`keep_local_version`] | « Garder la mienne » : mettre de côté la version de l'autre appareil | §7 |
//! | [`create_vault`] | Créer le coffre d'une base (premier chiffrement) | §9 |
//! | [`unlock_vault`] | Déverrouiller le coffre d'une base | §9 |
//! | [`lock_vault`] | Verrouiller le coffre d'une base | §9 |
//! | [`auto_lock_vaults`] | Verrouiller les coffres après N minutes sans activité | §9 |
//! | [`change_vault_password`] | Changer le mot de passe d'un coffre | §9 |
//! | [`list_vaults`] | État du coffre de chaque base | §9 |
//! | [`restore_remembered_vaults`] | Au lancement, rouvrir les coffres gardés dans le trousseau | §9 |
//! | [`encrypt_task`] | Chiffrer une tâche (± sous-tâches) | §9 |
//! | [`decrypt_task`] | Remettre une tâche en clair | §9 |

pub mod attach_file;
pub mod auto_lock_vaults;
pub mod change_vault_password;
pub mod check_linked_file;
pub mod close_task;
pub mod create_task;
pub mod create_vault;
pub mod customize_list_view;
pub mod decrypt_task;
pub mod delete_task;
pub mod edit_note;
pub mod encrypt_task;
pub mod get_close_impact;
pub mod get_task;
pub mod inherit_list_view;
pub mod keep_local_version;
pub mod link_file;
pub mod list_conflicts;
pub mod list_tasks;
pub mod list_trash;
pub mod list_vaults;
pub mod lock_vault;
pub mod move_task;
pub mod purge_expired;
pub mod purge_task;
pub mod read_media;
pub mod rename_task;
pub mod reopen_task;
pub mod restore_remembered_vaults;
pub mod restore_task;
pub mod search_tasks;
pub mod set_tags;
pub mod store_media;
pub mod sync_with_drive;
pub mod unlock_vault;
