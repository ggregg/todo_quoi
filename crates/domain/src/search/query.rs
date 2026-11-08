//! Requête de recherche : ce que l'utilisateur tape, transformé en critères (spec §5.6).
//!
//! Syntaxe : mots, `"phrase exacte"`, `tag:client-x`, `-tag:perso`, `is:open` / `is:done`,
//! `has:audio|image|file`, `in:"Projet A"` (sous-arbre d'une tâche dont le titre contient
//! « Projet A »). Un filtre inconnu ou mal formé est cherché comme un mot : la saisie n'est
//! jamais refusée.

use super::document::MediaKind;
use super::fold::fold_text;
use crate::BaseId;

/// Statut recherché (`is:open`, `is:done`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    /// Tâches ouvertes.
    Open,
    /// Tâches terminées.
    Done,
}

/// Critères d'une recherche. Tous les textes sont **normalisés** ([`fold_text`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchQuery {
    /// Mots et phrases qui doivent tous apparaître (titre, étiquettes ou note).
    pub terms: Vec<String>,
    /// Étiquettes obligatoires (`tag:`).
    pub tags_included: Vec<String>,
    /// Étiquettes interdites (`-tag:`).
    pub tags_excluded: Vec<String>,
    /// Statut demandé (`is:`), aucun = tous.
    pub status: Option<StatusFilter>,
    /// Médias que la note doit contenir (`has:`).
    pub media: Vec<MediaKind>,
    /// Sous-arbre (`in:`) : un ancêtre dont le titre contient ce texte.
    pub within: Option<String>,
    /// Bases où chercher (les bases affichées) ; vide = toutes. Pas saisi : fixé par l'interface.
    pub bases: Vec<BaseId>,
}

/// Un morceau de la saisie. `quoted` : il commençait par un guillemet (phrase exacte).
struct Token {
    text: String,
    quoted: bool,
}

impl SearchQuery {
    /// Analyse la saisie de l'utilisateur.
    ///
    /// ```
    /// use tq_domain::search::{SearchQuery, StatusFilter};
    /// let query = SearchQuery::parse(r#"Release "note de version" tag:Client-X is:open"#);
    /// assert_eq!(query.terms, ["release", "note de version"]);
    /// assert_eq!(query.tags_included, ["client-x"]);
    /// assert_eq!(query.status, Some(StatusFilter::Open));
    /// ```
    pub fn parse(input: &str) -> Self {
        let mut query = Self::default();
        for token in tokenize(input) {
            query.add(token);
        }
        query
    }

    /// Vrai si la requête ne contient aucun critère (rien à chercher).
    pub fn is_empty(&self) -> bool {
        Self {
            bases: Vec::new(),
            ..self.clone()
        } == Self::default()
    }

    /// La même requête, limitée à ces bases (vide = toutes).
    pub fn in_bases(mut self, bases: Vec<BaseId>) -> Self {
        self.bases = bases;
        self
    }

    fn add(&mut self, token: Token) {
        let text = fold_text(&token.text);
        if token.quoted || !self.add_filter(&text) {
            self.terms.push(text);
        }
    }

    /// Reconnaît un filtre `nom:valeur` ; renvoie `false` si ce n'en est pas un.
    fn add_filter(&mut self, text: &str) -> bool {
        let Some((name, value)) = text.split_once(':') else {
            return false;
        };
        if value.is_empty() {
            return false;
        }
        match name {
            "tag" => self.tags_included.push(value.to_string()),
            "-tag" => self.tags_excluded.push(value.to_string()),
            "in" => self.within = Some(value.to_string()),
            "is" => return self.set_status(value),
            "has" => return self.add_media(value),
            _ => return false,
        }
        true
    }

    fn set_status(&mut self, value: &str) -> bool {
        self.status = match value {
            "open" => Some(StatusFilter::Open),
            "done" => Some(StatusFilter::Done),
            _ => return false,
        };
        true
    }

    fn add_media(&mut self, value: &str) -> bool {
        let kind = match value {
            "audio" => MediaKind::Audio,
            "image" => MediaKind::Image,
            "file" => MediaKind::File,
            _ => return false,
        };
        self.media.push(kind);
        true
    }
}

/// Découpe la saisie aux espaces, sauf entre guillemets (`in:"Projet A"` = un seul morceau).
fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut inside_quotes = false;
    for character in input.chars() {
        match character {
            '"' | '“' | '”' | '«' | '»' => {
                quoted |= current.is_empty() && !inside_quotes;
                inside_quotes = !inside_quotes;
            }
            c if c.is_whitespace() && !inside_quotes => {
                push_token(&mut tokens, &mut current, quoted);
                quoted = false;
            }
            c => current.push(c),
        }
    }
    push_token(&mut tokens, &mut current, quoted);
    tokens
}

fn push_token(tokens: &mut Vec<Token>, current: &mut String, quoted: bool) {
    let text = current.trim();
    if !text.is_empty() {
        tokens.push(Token {
            text: text.to_string(),
            quoted,
        });
    }
    current.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_phrases_become_folded_terms() {
        let query = SearchQuery::parse(r#"  Été   "Note DE version"  café "#);
        assert_eq!(query.terms, ["ete", "note de version", "cafe"]);
    }

    #[test]
    fn every_filter_is_recognised() {
        let query = SearchQuery::parse(
            r#"tag:client-x -tag:Perso is:done has:audio has:file in:"Projet A""#,
        );
        assert_eq!(query.tags_included, ["client-x"]);
        assert_eq!(query.tags_excluded, ["perso"]);
        assert_eq!(query.status, Some(StatusFilter::Done));
        assert_eq!(query.media, [MediaKind::Audio, MediaKind::File]);
        assert_eq!(query.within.as_deref(), Some("projet a"));
        assert!(query.terms.is_empty());
    }

    #[test]
    fn unknown_or_incomplete_filters_are_searched_as_words() {
        let query = SearchQuery::parse(r#"is:later has:video tag: url:x "tag:x""#);
        assert_eq!(
            query.terms,
            ["is:later", "has:video", "tag:", "url:x", "tag:x"]
        );
        assert!(query.tags_included.is_empty());
    }

    #[test]
    fn blank_input_is_empty() {
        assert!(SearchQuery::parse("   ").is_empty());
        assert!(!SearchQuery::parse("is:open").is_empty());
    }

    #[test]
    fn an_unclosed_quote_keeps_the_rest_as_a_phrase() {
        assert_eq!(SearchQuery::parse(r#"a "b c"#).terms, ["a", "b c"]);
    }
}
