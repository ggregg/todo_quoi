// Évite une fenêtre console supplémentaire sous Windows en release. NE PAS RETIRER.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    poc_medias_lib::run();
}
