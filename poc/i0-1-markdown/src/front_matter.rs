//! En-tête TOML d'un fichier de tâche, entre deux lignes `+++` (convention Hugo / Zola).
//!
//! ```text
//! +++
//! title = "Préparer la release"
//! done = false
//! +++
//! Corps de la note en Markdown…
//! ```
//!
//! Deux niveaux :
//! - [`split`] / [`join`] : découper / recoller le texte, sans l'interpréter ;
//! - [`TaskHeader`] : lecture typée, avec **conservation des champs inconnus**
//!   (un appareil plus récent peut avoir ajouté des champs qu'on ne connaît pas encore).

use serde::{Deserialize, Serialize};
use toml::value::Datetime;

const OPENING: &str = "+++\n";
const CLOSING: &str = "\n+++\n";

/// Sépare l'en-tête (sans les `+++`, avec son dernier saut de ligne) du corps.
///
/// Renvoie `(None, entrée)` si le fichier ne commence pas par un en-tête complet.
pub fn split(input: &str) -> (Option<&str>, &str) {
    // [rust] `let … else` : si le motif ne correspond pas, on exécute le `else` (qui doit sortir).
    //        Équivalent TS : `const after = strip(input); if (after === undefined) return …;`
    let Some(after_opening) = input.strip_prefix(OPENING) else {
        return (None, input);
    };
    match after_opening.find(CLOSING) {
        // [rust] `&texte[a..b]` = sous-chaîne (≈ `texte.slice(a, b)`), sans copie.
        Some(end) => (
            Some(&after_opening[..end + 1]),
            &after_opening[end + CLOSING.len()..],
        ),
        None => (None, input),
    }
}

/// Recolle un en-tête (tel que renvoyé par [`split`]) et un corps.
pub fn join(header: &str, body: &str) -> String {
    format!("{OPENING}{header}+++\n{body}")
}

/// Lecture typée de l'en-tête. Les champs non déclarés ici atterrissent dans `unknown`
/// et sont réécrits tels quels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskHeader {
    pub schema_version: u32,
    pub id: String,
    // [rust] `#[serde(default)]` : champ absent → valeur par défaut (`None`, `false`, `[]`).
    //        TOML n'a pas de `null` : on omet simplement le champ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub order: String,
    pub title: String,
    pub done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_at: Option<Datetime>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: Datetime,
    pub updated_at: Datetime,
    pub updated_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<Datetime>,
    #[serde(default)]
    pub encrypted: bool,
    /// Tous les autres champs (`view`, champs ajoutés par une version future…).
    // [rust] `flatten` : les clés restantes de l'objet sont rangées dans cette table.
    #[serde(flatten)]
    pub unknown: toml::Table,
}

/// Lit l'en-tête TOML.
// [rust] `Result<T, E>` : soit `Ok(valeur)`, soit `Err(erreur)` — l'erreur fait partie du type.
pub fn parse_header(text: &str) -> Result<TaskHeader, toml::de::Error> {
    toml::from_str(text)
}

/// Écrit l'en-tête TOML.
pub fn render_header(header: &TaskHeader) -> Result<String, toml::ser::Error> {
    toml::to_string(header)
}

// [rust] `#[cfg(test)]` : ce module n'est compilé que pour `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"schema_version = 1
id = "0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e"
order = "a0V"
title = "Préparer la release 1.2"
done = false
tags = ["release", "client-x"]
created_at = 2026-11-05T16:02:11.120Z
updated_at = 2026-11-05T17:25:12.345Z
updated_by = "mac-greg"
couleur = "rouge"

[view]
columns = ["title", "progress"]
sort = [{ field = "manual" }]
"#;

    #[test]
    fn split_and_join_are_inverse() {
        let file = format!("+++\n{SAMPLE}+++\n# Titre\n");
        let (header, body) = split(&file);
        assert_eq!(header, Some(SAMPLE));
        assert_eq!(body, "# Titre\n");
        assert_eq!(join(SAMPLE, body), file);
    }

    #[test]
    fn file_without_header_is_all_body() {
        assert_eq!(split("# Titre\n"), (None, "# Titre\n"));
    }

    #[test]
    fn typed_header_round_trips_and_keeps_unknown_fields() {
        // [rust] `.expect("…")` : plante avec ce message si c'est une erreur. Autorisé dans les tests.
        let header = parse_header(SAMPLE).expect("en-tête valide");
        assert_eq!(header.title, "Préparer la release 1.2");
        assert_eq!(header.parent, None);
        assert!(header.unknown.contains_key("couleur"));
        assert!(header.unknown.contains_key("view"));

        let rendered = render_header(&header).expect("écriture TOML");
        let reparsed = parse_header(&rendered).expect("relecture TOML");
        assert_eq!(reparsed, header, "TOML réécrit :\n{rendered}");
    }

    /// Constat du POC : `toml` réécrit `11.120Z` en `11.12Z` (zéros finaux retirés).
    /// Le texte change, mais la **valeur** (120 ms) est conservée — c'est ce qui compte.
    #[test]
    fn milliseconds_survive_round_trip() {
        let header = parse_header(SAMPLE).expect("en-tête valide");
        let rendered = render_header(&header).expect("écriture TOML");
        let reparsed = parse_header(&rendered).expect("relecture TOML");
        assert_eq!(reparsed.created_at, header.created_at);
        assert_eq!(reparsed.created_at.to_string(), "2026-11-05T16:02:11.12Z");
    }
}
