//! Normalisation du texte pour la recherche : minuscules, accents retirés (spec §5.6).
//!
//! Écrite à la main (table Latin-1 / Latin étendu) plutôt qu'avec une crate Unicode : le besoin
//! est limité au français et aux langues européennes courantes (archi §6).
//!
//! Les ligatures sont développées (`œ` → `oe`, `æ` → `ae`, `ß` → `ss`) : « œufs » se trouve
//! en tapant « oeufs ». Le texte normalisé peut donc être plus long que l'original ;
//! [`fold_with_origins`] garde, pour chaque caractère produit, la position du caractère
//! d'origine — c'est ce qui permet de surligner au bon endroit.

/// Texte normalisé : minuscules, sans accents, ligatures développées.
///
/// ```
/// use tq_domain::search::fold_text;
/// assert_eq!(fold_text("Élève À Noël, œuvre"), "eleve a noel, oeuvre");
/// ```
pub fn fold_text(text: &str) -> String {
    fold_with_origins(text).0
}

/// Texte normalisé et, pour chacun de ses caractères, l'indice (en caractères) du caractère
/// d'origine dont il provient.
pub fn fold_with_origins(text: &str) -> (String, Vec<usize>) {
    let mut folded = String::with_capacity(text.len());
    let mut origins = Vec::with_capacity(text.len());
    for (position, character) in text.chars().enumerate() {
        for produced in fold_char(character).chars() {
            folded.push(produced);
            origins.push(position);
        }
    }
    (folded, origins)
}

/// Un caractère normalisé (une ou deux lettres).
fn fold_char(character: char) -> String {
    // [rust] `to_lowercase` renvoie un itérateur (une majuscule peut donner plusieurs
    //        caractères, ex. « İ ») : on garde le premier, `unwrap_or` couvre le cas vide.
    let lower = character.to_lowercase().next().unwrap_or(character);
    match lower {
        'œ' => "oe".to_string(),
        'æ' => "ae".to_string(),
        'ß' => "ss".to_string(),
        other => strip_accent(other).to_string(),
    }
}

fn strip_accent(character: char) -> char {
    match character {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'ď' | 'đ' => 'd',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => 'i',
        'ł' | 'ľ' => 'l',
        'ñ' | 'ń' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => 'o',
        'ŕ' | 'ř' => 'r',
        'ś' | 'š' | 'ş' => 's',
        'ť' | 'ţ' => 't',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
        'ý' | 'ÿ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        '’' | '‘' => '\'',
        '«' | '»' | '“' | '”' => '"',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{fold_text, fold_with_origins};

    #[test]
    fn lowercases_strips_accents_and_expands_ligatures() {
        let cases = [
            ("Préparer la RELEASE", "preparer la release"),
            ("Ça déçoit où ?", "ca decoit ou ?"),
            ("Œuvre, cœur, Æsir", "oeuvre, coeur, aesir"),
            ("l’été", "l'ete"),
            ("Straße", "strasse"),
            ("日本語 🎙", "日本語 🎙"),
        ];
        for (input, expected) in cases {
            assert_eq!(fold_text(input), expected, "entrée : {input}");
        }
    }

    #[test]
    fn origins_point_back_to_the_original_characters() {
        let (folded, origins) = fold_with_origins("Œuf é");
        assert_eq!(folded, "oeuf e");
        assert_eq!(origins, [0, 0, 1, 2, 3, 4]);
    }
}
