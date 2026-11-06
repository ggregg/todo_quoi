//! Point d'entrée du programme : délègue tout à la bibliothèque (`lib.rs`).

// Évite une fenêtre console supplémentaire sous Windows en version publiée. NE PAS RETIRER.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    todoquoi_lib::run();
}
