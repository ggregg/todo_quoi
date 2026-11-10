//! Alignement et couleur du texte (ADR-0012) : ce que le Markdown standard ne sait pas dire.
//!
//! | Mise en forme | Écriture | Exemple |
//! |---|---|---|
//! | Paragraphe aligné | ligne `{align=…}` à la fin du paragraphe | `Texte\n{align=center}` |
//! | Titre aligné | `{align=…}` en fin de ligne | `## Titre {align=right}` |
//! | Colonne de tableau alignée | alignement GFM de la ligne de séparation | `\| :---: \| ---: \|` |
//! | Texte coloré | `<span data-color="…">` (HTML en ligne, admis par CommonMark) | `<span data-color="red">urgent</span>` |
//!
//! Le texte de l'utilisateur ne peut pas les imiter : `{`, `}`, `<` et `>` y sont échappés
//! (`\{`), et seule la forme **non échappée** est reconnue. Couleurs : une palette fixe de
//! noms (le thème clair ou sombre choisit la teinte), pas de code couleur libre.

/// Alignements reconnus (sans alignement = à gauche, la valeur par défaut).
pub const ALIGNMENTS: [&str; 3] = ["left", "center", "right"];

/// Couleurs de texte reconnues (noms de la palette de l'interface).
pub const TEXT_COLORS: [&str; 6] = ["red", "orange", "green", "blue", "purple", "gray"];

/// Si le source `raw` d'un texte se termine par `{align=…}` **non échappé**, l'alignement.
pub fn trailing_alignment(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim_end();
    ALIGNMENTS.into_iter().find(|align| {
        trimmed
            .strip_suffix(&alignment_suffix(align))
            .is_some_and(|before| !before.ends_with('\\'))
    })
}

/// `{align=center}` : le suffixe écrit pour un alignement.
pub fn alignment_suffix(align: &str) -> String {
    format!("{{align={align}}}")
}

/// Alignement valide lu dans un attribut (`None` : par défaut, ou inconnu).
pub fn valid_alignment(value: Option<&str>) -> Option<&'static str> {
    ALIGNMENTS.into_iter().find(|align| Some(*align) == value)
}

/// Couleur valide lue dans un attribut.
pub fn valid_color(value: Option<&str>) -> Option<&'static str> {
    TEXT_COLORS.into_iter().find(|color| Some(*color) == value)
}

/// `<span data-color="red">` → `red` (couleur de la palette seulement).
pub fn color_span_start(html: &str) -> Option<&'static str> {
    let name = html
        .trim()
        .strip_prefix("<span data-color=\"")?
        .strip_suffix("\">")?;
    valid_color(Some(name))
}

/// Balise ouvrante écrite pour une couleur.
pub fn color_span_open(color: &str) -> String {
    format!("<span data-color=\"{color}\">")
}

/// Balise fermante d'un texte coloré.
pub const COLOR_SPAN_CLOSE: &str = "</span>";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unescaped_known_alignments_are_recognised() {
        let cases = [
            ("{align=center}", Some("center")),
            ("Titre {align=right}  ", Some("right")),
            ("Titre \\{align=right}", None),
            ("{align=justify}", None),
            ("texte", None),
        ];
        for (raw, expected) in cases {
            assert_eq!(trailing_alignment(raw), expected, "{raw}");
        }
    }

    #[test]
    fn only_palette_colors_open_a_colored_span() {
        assert_eq!(color_span_start("<span data-color=\"red\">"), Some("red"));
        assert_eq!(color_span_start("<span data-color=\"#ff0000\">"), None);
        assert_eq!(color_span_start("<span style=\"color:red\">"), None);
        assert_eq!(color_span_open("blue"), "<span data-color=\"blue\">");
    }
}
