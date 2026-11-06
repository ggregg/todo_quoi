//! Script de compilation : génère le code dont Tauri a besoin (configuration, permissions).

fn main() {
    tauri_build::build();
}
