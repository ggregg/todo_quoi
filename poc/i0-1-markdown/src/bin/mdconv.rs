//! Petit utilitaire en ligne de commande, utilisé par le script de validation côté TipTap.
//!
//! ```text
//! mdconv to-json < tache.md   > tache.json
//! mdconv to-md   < tache.json > tache.md
//! ```

use std::io::{self, Read, Write};

// [rust] `Box<dyn std::error::Error>` : « n'importe quelle erreur ». Pratique pour un petit
//        outil ; le code de l'app utilisera des types d'erreur précis.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let output = match mode.as_str() {
        "to-json" => serde_json::to_string_pretty(&md_roundtrip::parse_task_file(&input))?,
        "to-md" => md_roundtrip::render_task_file(&serde_json::from_str(&input)?),
        _ => return Err("usage : mdconv to-json|to-md < entrée > sortie".into()),
    };
    io::stdout().write_all(output.as_bytes())?;
    Ok(())
}
