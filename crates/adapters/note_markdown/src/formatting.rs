//! Alignement, couleur et taille du texte (ADR-0012, ADR-0013) : ce que le Markdown standard ne
//! sait pas dire.
//!
//! | Mise en forme | Écriture | Exemple |
//! |---|---|---|
//! | Paragraphe aligné | ligne `{align=…}` à la fin du paragraphe | `Texte\n{align=center}` |
//! | Titre aligné | `{align=…}` en fin de ligne | `## Titre {align=right}` |
//! | Colonne de tableau alignée | alignement GFM de la ligne de séparation | `\| :---: \| ---: \|` |
//! | Texte coloré | `<span data-color="…">` (HTML en ligne, admis par CommonMark) | `<span data-color="red">urgent</span>` |
//! | Taille du texte | `<span data-size="…">` | `<span data-size="large">important</span>` |
//!
//! Le texte de l'utilisateur ne peut pas les imiter : `{`, `}`, `<` et `>` y sont échappés
//! (`\{`), et seule la forme **non échappée** est reconnue. Couleurs et tailles : des listes
//! fixes de noms (le thème choisit la teinte, l'interface la taille en `em`), pas de valeur libre.

/// Alignements reconnus (sans alignement = à gauche, la valeur par défaut).
pub const ALIGNMENTS: [&str; 3] = ["left", "center", "right"];

/// Couleurs de texte reconnues (noms de la palette de l'interface).
pub const TEXT_COLORS: [&str; 6] = ["red", "orange", "green", "blue", "purple", "gray"];

/// Tailles de texte reconnues (sans taille = normale, la valeur par défaut).
pub const TEXT_SIZES: [&str; 3] = ["small", "large", "huge"];

/// Si le source `raw` d'un texte se termine par `{align=…}` **non échappé**, l'alignement.
// [rust] `&'static str` : une chaîne écrite dans le programme lui-même (ici, un élément de
//        `ALIGNMENTS`), qui vit toute l'exécution ; on peut la renvoyer sans copie. Seules les
//        valeurs des listes ci-dessus sortent de ce module : un alignement ou une couleur
//        inconnus ne peuvent pas être écrits.
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

/// Taille valide lue dans un attribut.
pub fn valid_size(value: Option<&str>) -> Option<&'static str> {
    TEXT_SIZES.into_iter().find(|size| Some(*size) == value)
}

/// `<span data-color="red">` → `red` (couleur de la palette seulement).
pub fn color_span_start(html: &str) -> Option<&'static str> {
    known_span_value(html, "color", &TEXT_COLORS)
}

/// `<span data-size="large">` → `large` (taille de la liste seulement).
pub fn size_span_start(html: &str) -> Option<&'static str> {
    known_span_value(html, "size", &TEXT_SIZES)
}

/// Valeur de `<span data-<attribute>="…">` si la balise a exactement cette forme et que la
/// valeur est dans `known`.
// [rust] `?` sur un `Option` : renvoie `None` tout de suite si la balise ne correspond pas.
fn known_span_value(html: &str, attribute: &str, known: &[&'static str]) -> Option<&'static str> {
    let value = html
        .trim()
        .strip_prefix("<span data-")?
        .strip_prefix(attribute)?
        .strip_prefix("=\"")?
        .strip_suffix("\">")?;
    known.iter().copied().find(|candidate| *candidate == value)
}

/// Balise ouvrante écrite pour une couleur.
pub fn color_span_open(color: &str) -> String {
    format!("<span data-color=\"{color}\">")
}

/// Balise ouvrante écrite pour une taille.
pub fn size_span_open(size: &str) -> String {
    format!("<span data-size=\"{size}\">")
}

/// Balise fermante d'un texte coloré ou dimensionné.
pub const SPAN_CLOSE: &str = "</span>";

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

    #[test]
    fn only_listed_sizes_open_a_sized_span() {
        let cases = [
            ("<span data-size=\"large\">", Some("large")),
            ("<span data-size=\"huge\">", Some("huge")),
            ("<span data-size=\"18px\">", None),
            ("<span data-size=\"large\" data-color=\"red\">", None),
            ("<span style=\"font-size:2em\">", None),
        ];
        for (html, expected) in cases {
            assert_eq!(size_span_start(html), expected, "{html}");
        }
        assert_eq!(size_span_open("small"), "<span data-size=\"small\">");
    }
}
